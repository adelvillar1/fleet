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
    /// Show the calibration stream: one day's rows, grouped by op.
    Calib(CalibOptions),
    /// The skills surface: the live skills, the drafts awaiting the
    /// slice-12 gate, and the usage-scan curation.
    Skills(SkillsOptions),
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

/// Options for `fleet calib`.
#[derive(Debug, PartialEq, Eq)]
pub struct CalibOptions {
    /// A past day to show, `YYYY-MM-DD`; absent is today (UTC).
    pub day: Option<String>,
}

/// Options for `fleet skills`. The bare command lists the live skills;
/// each flag narrows to one of the slice's other three surfaces.
#[derive(Debug, PartialEq, Eq)]
pub struct SkillsOptions {
    /// List the drafts awaiting the slice-12 gate instead of the live
    /// skills.
    pub proposed: bool,
    /// Run the usage-scan curation first, filing any retire/merge
    /// proposals it finds.
    pub curate: bool,
    /// Print one proposal's draft verbatim.
    pub show: Option<String>,
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
  fleet calib [--day <date>]    the calibration stream: one day's rows by op
  fleet skills [--proposed] [--curate] [--show <name>]
                                the loaded skills; --proposed lists the drafts
                                awaiting the gate, --curate files usage-scan
                                proposals, --show prints one draft
  fleet doctor                   check credentials, home, API, shell
  fleet version                  print the version
  fleet help                     print this help

options for run and chat:
  --model <name>      model to use (default deepseek-chat)
  --max-turns <n>     tool-loop turn cap for run (default 25)
  --yes, -y           approve every tool call without prompting
  --session <id>      resume a saved session (see fleet sessions)

options for calib:
  --day <YYYY-MM-DD>  a past day's rows (default: today, UTC)

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
        "calib" => parse_calib(rest),
        "skills" => parse_skills(rest),
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

/// `fleet calib`: one optional `--day`, and nothing else. A day that is
/// not `YYYY-MM-DD` is a usage error rather than a silently wrong read.
fn parse_calib(args: &[String]) -> Result<Command, String> {
    let mut day: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--day" => day = Some(take_value(args, &mut index, arg)?),
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown calib option `{arg}`"));
            }
            _ => return Err(format!("`{arg}` takes no value here; calib reads a day with --day")),
        }
        index += 1;
    }
    if let Some(day) = &day {
        let numeric = day.len() == 10 && day.chars().enumerate().all(|(position, character)| {
            if position == 4 || position == 7 {
                character == '-'
            } else {
                character.is_ascii_digit()
            }
        });
        if !numeric {
            return Err(format!("`{day}` is not a day; use --day YYYY-MM-DD"));
        }
        if !valid_calendar_day(day) {
            return Err(format!("`{day}` is not a real day; use --day YYYY-MM-DD"));
        }
    }
    Ok(Command::Calib(CalibOptions { day }))
}

/// `fleet skills`: the bare command lists the live skills; `--proposed`
/// lists the drafts awaiting the gate; `--curate` runs the usage-scan
/// curation and files what it finds; `--show <name>` prints one draft
/// verbatim. The flags compose in that order, so `--curate --proposed`
/// curates and then shows what is waiting.
fn parse_skills(args: &[String]) -> Result<Command, String> {
    let mut proposed = false;
    let mut curate = false;
    let mut show: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--proposed" => proposed = true,
            "--curate" => curate = true,
            "--show" => show = Some(take_value(args, &mut index, arg)?),
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown skills option `{arg}`"));
            }
            _ => return Err(format!("`{arg}` takes no value here; see `fleet help`")),
        }
        index += 1;
    }
    Ok(Command::Skills(SkillsOptions { proposed, curate, show }))
}

/// A real calendar check on an already-`YYYY-MM-DD`-shaped string: the
/// month is 1-12 and the day is within that month's own length (leap
/// years included). Without it, `--day 2026-13-45` reads as "Fleet did
/// not run" rather than "that day does not exist" — the shape is numeric,
/// but the day is not real.
fn valid_calendar_day(day: &str) -> bool {
    let year: i32 = day.get(0..4).and_then(|part| part.parse().ok()).unwrap_or(0);
    let month: u32 = day.get(5..7).and_then(|part| part.parse().ok()).unwrap_or(0);
    let day_of_month: u32 = day.get(8..10).and_then(|part| part.parse().ok()).unwrap_or(0);
    let longest = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=longest).contains(&day_of_month)
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
    fn calib_parses_an_optional_day_and_rejects_junk() {
        assert_eq!(
            parse(&args(&["calib"])).unwrap(),
            Command::Calib(CalibOptions { day: None })
        );
        assert_eq!(
            parse(&args(&["calib", "--day", "2026-10-01"])).unwrap(),
            Command::Calib(CalibOptions { day: Some("2026-10-01".to_string()) })
        );
        assert!(parse(&args(&["calib", "--day", "yesterday"])).is_err());
        assert!(parse(&args(&["calib", "--day", "2026-1-1"])).is_err());
        // A well-shaped but unreal day is a usage error, not a silent
        // "Fleet did not run" read.
        assert!(parse(&args(&["calib", "--day", "2026-13-45"])).is_err());
        assert!(parse(&args(&["calib", "--day", "2026-00-10"])).is_err());
        assert!(parse(&args(&["calib", "--day", "2026-02-30"])).is_err());
        assert!(parse(&args(&["calib", "--day", "2026-02-29"])).is_err()); // 2026 is not a leap year
        assert_eq!(
            parse(&args(&["calib", "--day", "2024-02-29"])).unwrap(),
            Command::Calib(CalibOptions { day: Some("2024-02-29".to_string()) })
        ); // 2024 is a leap year
        assert_eq!(
            parse(&args(&["calib", "--day", "2026-12-31"])).unwrap(),
            Command::Calib(CalibOptions { day: Some("2026-12-31".to_string()) })
        );
        assert!(parse(&args(&["calib", "--day"])).is_err());
        assert!(parse(&args(&["calib", "--json"])).is_err());
        assert!(parse(&args(&["calib", "today"])).is_err());
    }

    #[test]
    fn missing_values_fail_with_the_flag_named() {
        let error = parse(&args(&["run", "--model"])).unwrap_err();
        assert!(error.contains("--model"));
    }

    #[test]
    fn skills_parses_its_flags_and_rejects_junk() {
        assert_eq!(
            parse(&args(&["skills"])).unwrap(),
            Command::Skills(SkillsOptions { proposed: false, curate: false, show: None })
        );
        assert_eq!(
            parse(&args(&["skills", "--proposed"])).unwrap(),
            Command::Skills(SkillsOptions { proposed: true, curate: false, show: None })
        );
        assert_eq!(
            parse(&args(&["skills", "--curate", "--proposed"])).unwrap(),
            Command::Skills(SkillsOptions { proposed: true, curate: true, show: None })
        );
        assert_eq!(
            parse(&args(&["skills", "--show", "bash-recovery"])).unwrap(),
            Command::Skills(SkillsOptions {
                proposed: false,
                curate: false,
                show: Some("bash-recovery".to_string()),
            })
        );
        assert!(parse(&args(&["skills", "--json"])).is_err());
        assert!(parse(&args(&["skills", "extra"])).is_err());
        assert!(parse(&args(&["skills", "--show"])).is_err());
        assert_eq!(parse(&args(&["skills", "--help"])).unwrap(), Command::Help);
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
