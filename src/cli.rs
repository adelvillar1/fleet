//! Command-line grammar: parse argv into commands. The parser owns Fleet's
//! own flags; there is nothing to forward anywhere.

use crate::api::DEFAULT_MODEL;

/// Everything `fleet` answers to.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Run one task to completion.
    Run(RunOptions),
    /// Interactive multi-turn session.
    Chat(ChatOptions),
    /// List saved sessions.
    Sessions,
    /// Probe the four vendored lanes' health.
    Lanes,
    /// List the routing roster in declared chain order with per-provider state.
    Providers,
    /// Environment and connectivity checks.
    Doctor,
    /// Print the version.
    Version,
    /// Print help.
    Help,
}

/// Options for `fleet run`.
#[derive(Debug, PartialEq, Eq)]
pub struct RunOptions {
    /// The task text: all non-flag words, joined.
    pub task: String,
    pub model: String,
    pub max_turns: usize,
    /// Approve every tool call without prompting.
    pub yes: bool,
    /// Session id to resume, when given.
    pub session: Option<String>,
}

/// Options for `fleet chat`.
#[derive(Debug, PartialEq, Eq)]
pub struct ChatOptions {
    pub model: String,
    pub yes: bool,
    pub session: Option<String>,
}

/// Turn cap when `--max-turns` is absent.
pub const DEFAULT_MAX_TURNS: usize = 25;

/// The help text; also the usage error body.
pub const HELP: &str = "\
fleet — a native Rust agent based on DeepSeek Harness

usage:
  fleet run \"<task>\" [options]   run one task to completion
  fleet chat [options]           interactive multi-turn session
  fleet sessions                 list saved sessions
  fleet lanes                    probe the four vendored lanes' health
  fleet providers                list the routing roster and per-provider state
  fleet doctor                   check credentials, home, API, shell
  fleet version                  print the version
  fleet help                     print this help

options for run and chat:
  --model <name>      model to use (default deepseek-chat)
  --max-turns <n>     tool-loop turn cap for run (default 25)
  --yes, -y           approve every tool call without prompting
  --session <id>      resume a saved session (see fleet sessions)

environment:
  DEEPSEEK_API_KEY    required; also read from ~/.fleet/.env and the
                      DeepSeek Harness credential store (~/.dsh/.credentials.yaml)
  DEEPSEEK_BASE_URL   optional API base (default https://api.deepseek.com)
  FLEET_HOME          optional home (default ~/.fleet)

exit codes: 0 success, 1 runtime failure, 2 usage or configuration,
3 turn cap reached
";

/// Parse the argument list (without the program name). `Err` carries the
/// user-facing complaint.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(command) = args.first() else {
        return Err("no command given".to_string());
    };
    let rest = &args[1..];
    match command.as_str() {
        "run" => parse_run(rest),
        "chat" => parse_chat(rest),
        "sessions" => no_flags(rest, Command::Sessions),
        "lanes" => no_flags(rest, Command::Lanes),
        "providers" => no_flags(rest, Command::Providers),
        "doctor" => no_flags(rest, Command::Doctor),
        "version" | "--version" | "-V" => no_flags(rest, Command::Version),
        "help" | "--help" | "-h" => Ok(Command::Help),
        other => Err(format!("unknown command `{other}` — try `fleet help`")),
    }
}

fn parse_run(args: &[String]) -> Result<Command, String> {
    let mut words: Vec<&str> = Vec::new();
    let mut model = DEFAULT_MODEL.to_string();
    let mut max_turns = DEFAULT_MAX_TURNS;
    let mut yes = false;
    let mut session: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--yes" | "-y" => yes = true,
            "--model" => model = take_value(args, &mut index, arg)?,
            "--max-turns" => {
                let value = take_value(args, &mut index, arg)?;
                max_turns = value.parse().map_err(|_| format!("`{value}` is not a turn count"))?;
            }
            "--session" => session = Some(take_value(args, &mut index, arg)?),
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown run option `{arg}`"));
            }
            _ => words.push(arg),
        }
        index += 1;
    }
    if words.is_empty() {
        return Err("run needs a task: fleet run \"<task>\"".to_string());
    }
    Ok(Command::Run(RunOptions {
        task: words.join(" "),
        model,
        max_turns,
        yes,
        session,
    }))
}

fn parse_chat(args: &[String]) -> Result<Command, String> {
    let mut model = DEFAULT_MODEL.to_string();
    let mut yes = false;
    let mut session: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--yes" | "-y" => yes = true,
            "--model" => model = take_value(args, &mut index, arg)?,
            "--session" => session = Some(take_value(args, &mut index, arg)?),
            _ => return Err(format!("unknown chat option `{arg}`")),
        }
        index += 1;
    }
    Ok(Command::Chat(ChatOptions { model, yes, session }))
}

/// Commands that take no options at all; a stray flag is a usage error
/// rather than a silently ignored one.
fn no_flags(args: &[String], command: Command) -> Result<Command, String> {
    match args.first() {
        None => Ok(command),
        Some(arg) => Err(format!("`{arg}` takes no options here")),
    }
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn run_takes_a_task_and_defaults() {
        let command = parse(&args(&["run", "fix", "the", "bug"])).unwrap();
        let Command::Run(options) = command else { panic!("not a run") };
        assert_eq!(options.task, "fix the bug");
        assert_eq!(options.model, "deepseek-chat");
        assert_eq!(options.max_turns, DEFAULT_MAX_TURNS);
        assert!(!options.yes);
        assert_eq!(options.session, None);
    }

    #[test]
    fn run_parses_all_flags_in_any_position() {
        let command =
            parse(&args(&["run", "--yes", "do it", "--model", "m2", "--max-turns", "3", "--session", "abc"]))
                .unwrap();
        let Command::Run(options) = command else { panic!("not a run") };
        assert_eq!(options.task, "do it");
        assert_eq!(options.model, "m2");
        assert_eq!(options.max_turns, 3);
        assert!(options.yes);
        assert_eq!(options.session.as_deref(), Some("abc"));
    }

    #[test]
    fn run_rejects_unknown_flags_and_bad_counts() {
        let error = parse(&args(&["run", "--profile", "web", "x"])).unwrap_err();
        assert!(error.contains("--profile"));
        let error = parse(&args(&["run", "--max-turns", "soon", "x"])).unwrap_err();
        assert!(error.contains("not a turn count"));
    }

    #[test]
    fn run_needs_a_task() {
        assert!(parse(&args(&["run"])).is_err());
        assert!(parse(&args(&["run", "--yes"])).is_err());
    }

    #[test]
    fn chat_parses_flags() {
        let command = parse(&args(&["chat", "-y", "--model", "m"])).unwrap();
        assert_eq!(command, Command::Chat(ChatOptions {
            model: "m".to_string(),
            yes: true,
            session: None,
        }));
    }

    #[test]
    fn chat_rejects_stray_words() {
        assert!(parse(&args(&["chat", "hello"])).is_err());
    }

    #[test]
    fn bare_commands_take_no_options() {
        assert_eq!(parse(&args(&["sessions"])).unwrap(), Command::Sessions);
        assert_eq!(parse(&args(&["lanes"])).unwrap(), Command::Lanes);
        assert_eq!(parse(&args(&["doctor"])).unwrap(), Command::Doctor);
        assert_eq!(parse(&args(&["version"])).unwrap(), Command::Version);
        assert_eq!(parse(&args(&["--help"])).unwrap(), Command::Help);
        assert!(parse(&args(&["sessions", "--all"])).is_err());
        assert!(parse(&args(&["lanes", "--json"])).is_err());
    }

    #[test]
    fn missing_values_fail_with_the_flag_named() {
        let error = parse(&args(&["run", "--model"])).unwrap_err();
        assert!(error.contains("--model"));
    }

    #[test]
    fn unknown_commands_fail() {
        let error = parse(&args(&["frobnicate"])).unwrap_err();
        assert!(error.contains("frobnicate"));
    }

    #[test]
    fn no_command_is_a_usage_error() {
        assert!(parse(&[]).is_err());
    }
}
