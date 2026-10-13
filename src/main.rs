//! Fleet: a native Rust agent based on DeepSeek Harness.

use std::collections::HashMap;
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
use fleet::route;
use fleet::session::{self, Session};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // No subcommand is the TUI, Fleet's daily-driver surface; every
    // subcommand path below is the headless one, unchanged.
    if args.is_empty() {
        return tui_command();
    }
    match cli::parse(&args) {
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("fleet {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Run(options)) => run_command(&options).await,
        Ok(Command::Chat(options)) => chat_command(&options).await,
        Ok(Command::Sessions) => sessions_command(),
        Ok(Command::Lanes) => lanes_command().await,
        Ok(Command::Providers) => providers_command(),
        Ok(Command::Calib(options)) => calib_command(&options),
        Ok(Command::Skills(options)) => skills_command(&options),
        Ok(Command::Memories(options)) => memories_command(&options),
        Ok(Command::Doctor) => doctor_command(),
        Err(message) => {
            eprintln!("fleet: {message}");
            eprint!("{}", cli::HELP);
            ExitCode::from(2)
        }
    }
}

/// `fleet` with no subcommand: the TUI on the default model and a fresh
/// session. Credentials are resolved per run by the router — each roster
/// candidate names its own key env var — so no single key is required up
/// front, and the doctor stays the place that reports a missing one.
fn tui_command() -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    match fleet::tui::run(fleet::tui::Config {
        home,
        model: fleet::api::DEFAULT_MODEL.to_string(),
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fleet: {message}");
            ExitCode::FAILURE
        }
    }
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

async fn run_command(options: &RunOptions) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let dotenv = read_dotenv();
    let mut session = match open_session(&home, options.session.as_deref(), &options.model) {
        Ok(session) => session,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    eprintln!("fleet: session {} (model {})", session.id(), options.model);
    let prepared = match route::prepare(&home, &dotenv, &options.task, &HashMap::new()).await {
        Ok(prepared) => prepared,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::from(2);
        }
    };
    eprintln!("route: {} ({})", prepared.decision.provider, prepared.decision.how.label());
    let skills = fleet::skills::load(&home);
    let outcome = drive(
        &prepared,
        options.model.clone(),
        options.max_turns,
        options.yes,
        &mut session,
        &options.task,
        skills,
    )
    .await;
    prepared.record_route(&options.task);
    // The self-authoring half: after the session, the harness drafts the
    // skill changes its outcomes justify. Best-effort by construction —
    // drafting is the harness noticing, never the run's outcome.
    draft_lessons(&home, session.path());
    match outcome {
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

async fn chat_command(options: &ChatOptions) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let dotenv = read_dotenv();
    let mut session = match open_session(&home, options.session.as_deref(), &options.model) {
        Ok(session) => session,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    println!("fleet chat — model {}, session {} (/exit to quit)", options.model, session.id());
    // The skills are loaded once per chat session and declared on every
    // run in it: the files can change between runs, and the per-run
    // `skills` record is what pins what each run actually saw.
    let skills = fleet::skills::load(&home);
    let stdin = std::io::stdin();
    // The route is chosen once, on the first task, and the committed router
    // is reused for the rest of the session — one sys1 judgment, not one
    // per message.
    let mut prepared: Option<route::Prepared> = None;
    let mut judge_task = String::new();
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
        if prepared.is_none() {
            match route::prepare(&home, &dotenv, trimmed, &HashMap::new()).await {
                Ok(fresh) => {
                    eprintln!("route: {} ({})", fresh.decision.provider, fresh.decision.how.label());
                    judge_task = trimmed.to_string();
                    prepared = Some(fresh);
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    return ExitCode::from(2);
                }
            }
        }
        let prepared = prepared.as_ref().expect("prepared on the first task");
        let outcome = drive(
            prepared,
            options.model.clone(),
            DEFAULT_MAX_TURNS,
            options.yes,
            &mut session,
            trimmed,
            skills.clone(),
        )
        .await;
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
    if let Some(prepared) = &prepared {
        prepared.record_route(&judge_task);
    }
    // The session is over: draft its lessons into proposals, best-effort.
    draft_lessons(&home, session.path());
    ExitCode::SUCCESS
}

/// The self-authoring half, after a session ends: read the session's log
/// plus the day's calibration rows, draft the lessons the evidence
/// justifies, and file them as proposals awaiting the slice-12 gate.
///
/// Nothing is applied here and nothing is applied silently — the only
/// writer in this path is `skills/proposals/`, and a proposal enters the
/// live tree only when a gate says so. Best-effort throughout: an
/// unreadable log or an unwritable proposals directory drafts nothing
/// and fails nothing.
fn draft_lessons(home: &std::path::Path, session_path: &std::path::Path) {
    let user_home = std::env::var("HOME").ok();
    let filed =
        fleet::skills::draft_session(home, session_path, user_home.as_deref());
    if !filed.is_empty() {
        eprintln!(
            "fleet: skills: drafted {} proposal(s) awaiting the gate in {}",
            filed.len(),
            fleet::skills::proposals_dir(home).display()
        );
    }
}

/// `fleet skills`: the loaded skills as cards, with each one's usage
/// count from the log scan and its file's mtime. `--proposed` lists the
/// drafts awaiting the gate instead; `--curate` runs the usage-scan
/// curation first; `--show <name>` prints one draft verbatim.
///
/// The exit code is always success: this is a read surface over Fleet's
/// own files, like `fleet calib` — a home with no skills says so rather
/// than failing. The two failures are a `--show` naming a draft that
/// does not exist and a `--show` name that is not a bare name (a path
/// separator or a parent component addresses a file, not a draft).
fn skills_command(options: &cli::SkillsOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    if let Some(name) = &options.show {
        return match fleet::skills::read_proposal(&home, name) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if options.curate {
        let config = route::skills_config(&home);
        let filed = fleet::skills::curate(&home, config.retire_after_sessions, config.merge_similarity);
        print!("{}", fleet::skills::render_curation(&filed));
    }
    if options.proposed {
        print!("{}", fleet::skills::render_proposals(&fleet::skills::load_proposals(&home)));
    } else if !options.curate {
        let config = route::skills_config(&home);
        let skills = fleet::skills::load(&home);
        let usage = fleet::skills::scan_usage(&home, config.retire_after_sessions);
        print!("{}", fleet::skills::render(&skills, &usage));
    }
    ExitCode::SUCCESS
}

/// `fleet memories`: the durable facts as cards, the ranked assembly for
/// one instruction (`--recall`), or the cards filtered by structure
/// shape (`--shape`). A read surface over the repo's own git-tracked
/// files, like `fleet calib` and `fleet skills`: the exit code is always
/// success, and a corpus with no facts says so rather than failing.
///
/// The corpus is the nearest `memories/` at or above the working
/// directory — the repo root in everyday use, the fork's
/// `resolveMemoryRoot` rule. The vector path is probed at load and the
/// surface's header names which backend serves; `--recall` is a dry run
/// that writes nothing, because the turn path that records hits is
/// part 2's.
fn memories_command(options: &cli::MemoriesOptions) -> ExitCode {
    let root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
    let memory = fleet::memory::Memory::load(&root);
    if let Some(instruction) = &options.recall {
        print!("{}", fleet::memory::render_recall(&memory, instruction));
    } else {
        print!("{}", fleet::memory::render(&memory, options.shape.as_deref()));
    }
    ExitCode::SUCCESS
}

/// `fleet calib`: one day of the calibration stream as cards — the rows
/// grouped by op with their counts, the writer's counters, and the last
/// few rows. No network, no lane spawn, no writer of its own: this is a
/// read of the audit trail, and a day with no rows says so rather than
/// failing. The exit code is always success; the stream is an artifact,
/// not a gate.
fn calib_command(options: &cli::CalibOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let root = fleet::calib::resolve_root(&home, std::env::var("HOME").ok().as_deref());
    let day = options
        .day
        .clone()
        .unwrap_or_else(fleet::calib::today);
    print!("{}", fleet::calib::render(&root, &day));
    ExitCode::SUCCESS
}

/// Build the routed agent and run `task`, printing the final answer to
/// stdout. The route line and the decision row are the caller's: `run`
/// prints once per task, `chat` once per session.
async fn drive(
    prepared: &route::Prepared,
    model: String,
    max_turns: usize,
    yes: bool,
    session: &mut Session,
    task: &str,
    skills: Vec<fleet::skills::Skill>,
) -> Result<RunReport, AgentError> {
    let client = Client::routed(prepared.router.clone());
    let options = AgentOptions {
        model,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: Some(prepared.calib.clone()),
        skills,
    };
    let approver: Box<dyn Approver> =
        if yes { Box::new(ApproveAll) } else { Box::new(ConsoleApprover::new()) };
    let mut progress = std::io::stderr().lock();
    let mut agent = Agent::new(&client, options, session, approver, &mut progress)?;
    let report = agent.run(task).await?;
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

/// `fleet lanes`: probe every vendored lane through the MCP client and
/// render the health cards. No credentials, no home, no network beyond
/// the lane children themselves — the lanes are local services. The exit
/// code is the doctor's convention: success only when every lane is ok.
async fn lanes_command() -> ExitCode {
    let health = fleet::lanes::health_all().await;
    print!("{}", fleet::lanes::render(&health));
    if health.iter().all(|lane| lane.state == fleet::lanes::LaneState::Ok) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `fleet providers`: list the routing roster in declared chain order with
/// per-provider state (configured/unconfigured, key present/absent — never
/// the key) and the head marked, as cards in `fleet lanes`' style. No
/// network and no child spawn: key presence is an environment read. The
/// exit code doubles as "is routing possible here" — success when at least
/// one candidate is configured.
fn providers_command() -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let dotenv = read_dotenv();
    let roster = match route::load_roster(&home) {
        Ok(roster) => roster,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::from(2);
        }
    };
    let resolved = route::resolve(&roster.candidates, &dotenv);
    print!("{}", route::render(&resolved));
    if resolved.iter().any(|entry| route::provider_state(entry) == route::ProviderState::Ready) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
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

    // The calibration-stream line: informational, never a check, and it
    // never changes the exit code. It names the root the stream actually
    // resolves to and whether today's file exists — the cheapest guard
    // against dev-decisions moving its LOG_DIR literal and silently
    // re-routing Fleet's rows.
    let user_home = std::env::var("HOME").ok();
    let calib_line = match &home {
        Some(home) => {
            let root = fleet::calib::resolve_root(home, user_home.as_deref());
            let today_file = fleet::calib::day_path(&root, &fleet::calib::today());
            doctor::render_calib(Some(&root), today_file.exists())
        }
        None => doctor::render_calib(None, false),
    };
    println!("{calib_line}");

    // The recall-path line: which retrieval path is live. The probe is
    // one real embedding round trip against the loopback llama-server —
    // a `/health` poll would prove nothing about the embeddings route —
    // and the check names what it found either way. It passes in both
    // states and never touches the exit code: keyword-only recall is a
    // working configuration, and the failure this line exists to fix is
    // the live system's silent degradation, not a missing local model.
    let probed = fleet::memory::probe(fleet::memory::DEFAULT_EMBEDDING_ENDPOINT);
    let recall_check = doctor::evaluate_recall(probed.vector());
    println!("{}", doctor::render(&recall_check));

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
