//! Fleet: a native Rust agent based on DeepSeek Harness.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use fleet::agent::{
    Agent, AgentError, AgentOptions, ApproveAll, Approver, ConsoleApprover, DEFAULT_MAX_TURNS,
    RunReport, StopReason,
};
use fleet::api::{Client, DEFAULT_BASE_URL};
use fleet::cli::{self, ChatOptions, Command, RunOptions};
use fleet::doctor;
use fleet::home::{self, resolve_setting};
use fleet::net;
use fleet::session::{self, Session};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("fleet {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Run(options)) => run_command(&options),
        Ok(Command::Chat(options)) => chat_command(&options),
        Ok(Command::Sessions) => sessions_command(),
        Ok(Command::Doctor) => doctor_command(),
        Err(message) => {
            eprintln!("fleet: {message}");
            eprint!("{}", cli::HELP);
            ExitCode::from(2)
        }
    }
}

/// The resolved client inputs: API key and base URL, environment first with
/// `~/.fleet/.env` and the DeepSeek Harness credential store as fallbacks.
struct Credentials {
    key: String,
    base_url: String,
}

/// Resolve credentials, printing the failed credentials check on the way out.
fn credentials() -> Result<Credentials, ExitCode> {
    let dotenv = read_dotenv();
    let Some(key) = setting("DEEPSEEK_API_KEY", &dotenv) else {
        eprintln!("{}", doctor::render(&doctor::evaluate_credentials(None)));
        return Err(ExitCode::from(2));
    };
    let base_url = setting("DEEPSEEK_BASE_URL", &dotenv).unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    Ok(Credentials { key, base_url })
}

/// Resolve a setting: the environment and `~/.fleet/.env` first
/// (`resolve_setting`), then the DeepSeek Harness credential store's `refs`
/// block, so one stored key serves both harnesses.
fn setting(name: &str, dotenv: &std::collections::HashMap<String, String>) -> Option<String> {
    resolve_setting(name, dotenv).or_else(|| {
        let user_home = std::env::var("HOME").ok();
        let path = home::harness_credentials_path(
            std::env::var("DSH_HOME").ok().as_deref(),
            user_home.as_deref(),
        )?;
        let text = std::fs::read_to_string(path).ok()?;
        home::harness_refs(&text, name)
    })
}

/// The `~/.fleet/.env` map, empty when no home is known.
fn read_dotenv() -> std::collections::HashMap<String, String> {
    fleet_home().ok().as_deref().map(home::read_dotenv).unwrap_or_default()
}

/// The fleet home, failing loud when no home is known.
fn fleet_home() -> Result<std::path::PathBuf, String> {
    let user_home = std::env::var("HOME").ok();
    home::fleet_home(std::env::var("FLEET_HOME").ok().as_deref(), user_home.as_deref())
}

fn run_command(options: &RunOptions) -> ExitCode {
    let credentials = match credentials() {
        Ok(credentials) => credentials,
        Err(code) => return code,
    };
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let mut session = match open_session(&home, options.session.as_deref(), &options.model) {
        Ok(session) => session,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    eprintln!("fleet: session {} (model {})", session.id(), options.model);
    match drive(
        &credentials,
        options.model.clone(),
        options.max_turns,
        options.yes,
        &mut session,
        &options.task,
    ) {
        Ok(report) => match report.stop_reason {
            StopReason::Completed => ExitCode::SUCCESS,
            StopReason::MaxTurns => {
                eprintln!(
                    "fleet: stopped at the {}-turn cap; resume with --session {}",
                    report.turns,
                    session.id()
                );
                ExitCode::from(3)
            }
        },
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

fn chat_command(options: &ChatOptions) -> ExitCode {
    let credentials = match credentials() {
        Ok(credentials) => credentials,
        Err(code) => return code,
    };
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let mut session = match open_session(&home, options.session.as_deref(), &options.model) {
        Ok(session) => session,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    println!("fleet chat — model {}, session {} (/exit to quit)", options.model, session.id());
    let stdin = std::io::stdin();
    loop {
        print!("you> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "/exit" || trimmed == "/quit" {
            break;
        }
        let outcome = drive(
            &credentials,
            options.model.clone(),
            DEFAULT_MAX_TURNS,
            options.yes,
            &mut session,
            trimmed,
        );
        match outcome {
            Ok(_) => {}
            // A failed log write makes the session unsafe to continue.
            Err(AgentError::Io(error)) => {
                eprintln!("fleet: session log write failed: {error}");
                return ExitCode::FAILURE;
            }
            // A failed request leaves the chat usable.
            Err(error) => eprintln!("fleet: {error}"),
        }
    }
    ExitCode::SUCCESS
}

/// Build the agent and run `task`, printing the final answer to stdout.
fn drive(
    credentials: &Credentials,
    model: String,
    max_turns: usize,
    yes: bool,
    session: &mut Session,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(credentials.base_url.clone(), credentials.key.clone());
    let options = AgentOptions { model, max_turns };
    let approver: Box<dyn Approver> =
        if yes { Box::new(ApproveAll) } else { Box::new(ConsoleApprover::new()) };
    let mut progress = std::io::stderr().lock();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress)?;
    let report = agent.run(task)?;
    if let Some(answer) = &report.final_message {
        println!("\n{answer}\n");
    }
    Ok(report)
}

/// Open `--session <id>` or create a new session, failing loud on missing
/// logs.
fn open_session(home: &std::path::Path, id: Option<&str>, model: &str) -> Result<Session, String> {
    match id {
        None => {
            Session::create(home, model)
                .map_err(|error| format!("could not create the session: {error}"))
        }
        Some(id) => {
            let path = session::session_path(home, id);
            if !path.exists() {
                return Err(format!("no session `{id}` under {}", home::sessions_dir(home).display()));
            }
            Session::open(&path).map_err(|error| format!("could not open session `{id}`: {error}"))
        }
    }
}

fn sessions_command() -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let summaries = match session::list(&home) {
        Ok(summaries) => summaries,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    if summaries.is_empty() {
        println!("no sessions yet — run `fleet run \"<task>\"` to start one");
        return ExitCode::SUCCESS;
    }
    println!("{:44} {:20} {:16} state", "id", "created", "model");
    for summary in summaries {
        println!(
            "{:44} {:20} {:16} {}",
            summary.id,
            summary.created,
            summary.model,
            if summary.ended { "ended" } else { "open" }
        );
    }
    ExitCode::SUCCESS
}

fn doctor_command() -> ExitCode {
    let home = fleet_home().ok();
    let dotenv = read_dotenv();
    let key = setting("DEEPSEEK_API_KEY", &dotenv);
    let base_url =
        setting("DEEPSEEK_BASE_URL", &dotenv).unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let mut failed = false;
    let credentials_check = doctor::evaluate_credentials(key.as_deref());
    failed |= !credentials_check.ok;
    println!("{}", doctor::render(&credentials_check));

    let home_outcome = match &home {
        Some(home) => home::ensure_sessions_dir(home)
            .map(|_| ())
            .map_err(|error| error.to_string()),
        None => Err("no home directory known; set FLEET_HOME or HOME".to_string()),
    };
    let home_check = doctor::evaluate_home(home.as_deref(), home_outcome);
    failed |= !home_check.ok;
    println!("{}", doctor::render(&home_check));

    let endpoint = doctor::api_endpoint(&base_url);
    let reachable = endpoint
        .map(|(host, port)| net::listening(host, port, std::time::Duration::from_millis(1500)))
        .unwrap_or(false);
    let api_check = doctor::evaluate_api(endpoint, reachable);
    failed |= !api_check.ok;
    println!("{}", doctor::render(&api_check));

    let shell_check = doctor::evaluate_shell(shell_probe());
    failed |= !shell_check.ok;
    println!("{}", doctor::render(&shell_check));

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Probe the platform shell without changing anything.
fn shell_probe() -> Result<&'static str, String> {
    #[cfg(unix)]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("true")
            .status()
            .map_err(|error| format!("sh: {error}"))?
            .success()
            .then_some("sh")
            .ok_or_else(|| "sh is present but exits non-zero".to_string())
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg("ver")
            .status()
            .map_err(|error| format!("cmd: {error}"))?
            .success()
            .then_some("cmd")
            .ok_or_else(|| "cmd is present but exits non-zero".to_string())
    }
}
