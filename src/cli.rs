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
    /// The memories surface: the durable facts as cards, the ranked
    /// assembly for one instruction, the cards filtered by shape, the
    /// consolidation cycle, or the archive of forgotten facts.
    Memories(MemoriesOptions),
    /// The scheduled jobs: the bare command lists them, each subcommand
    /// mutates the store by name.
    Cron(CronOptions),
    /// The self-maintenance surface: the bare command lists every
    /// proposal in both stores; `apply` runs one through the gate and
    /// `undo` reverts an applied one.
    Improve(ImproveOptions),
    /// Write one fact into the memory corpus: the CLI form of the
    /// `remember` tool, structured flags over the same field names.
    Remember(RememberOptions),
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

/// Options for `fleet memories`. The bare command cards the live facts;
/// each flag narrows to one of the slice's other surfaces. `--recall`
/// and `--shape` do not combine — one prints an assembly, the other
/// filters cards — and `--archive` is its own surface over the facts
/// the cycle has forgotten.
#[derive(Debug, PartialEq, Eq)]
pub struct MemoriesOptions {
    /// Print the ranked assembly for this instruction — what would
    /// enter a turn's context — instead of the cards. A dry run.
    pub recall: Option<String>,
    /// Filter the cards to facts carrying this structure shape.
    pub shape: Option<String>,
    /// Run the consolidation cycle first, then print the cards: one
    /// pass on demand, with what it did named.
    pub consolidate: bool,
    /// List the facts the cycle's forget action archived, instead of
    /// the live corpus.
    pub archive: bool,
}

/// Options for `fleet remember`. The flags are the frontmatter's own
/// field names, so the CLI and the model-visible tool validate through
/// one parser — the CLI is the same write, typed by hand.
#[derive(Debug, PartialEq, Eq)]
pub struct RememberOptions {
    pub name: String,
    pub description: String,
    /// Optional; an empty value means the writer's default.
    pub fact_type: String,
    /// Optional; comma-separated structure shapes.
    pub shape: String,
    /// Required: `confirmed-by` or `inferred`.
    pub basis: String,
    /// Optional; an empty value means `durable`.
    pub tier: String,
    /// Optional for a durable fact, required for a working one.
    pub valid_until: String,
    pub body: String,
}

/// Options for `fleet cron`. The bare command lists the jobs as cards;
/// each subcommand mutates the persistent store by name, and every
/// mutation validates through the same parser the scheduler matches
/// against.
#[derive(Debug, PartialEq, Eq)]
pub struct CronOptions {
    pub action: CronAction,
}

/// What `fleet cron` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum CronAction {
    /// List the jobs.
    List,
    /// Create one from structured flags.
    Add {
        name: String,
        schedule: String,
        task: String,
        auto_approve: bool,
    },
    /// Turn a job back on.
    Enable { name: String },
    /// Turn a job off without removing it.
    Disable { name: String },
    /// Remove a job from the store.
    Remove { name: String },
}

/// Options for `fleet improve`. The bare command lists the proposals;
/// each subcommand acts on one by name.
#[derive(Debug, PartialEq, Eq)]
pub struct ImproveOptions {
    pub action: ImproveAction,
}

/// What `fleet improve` was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum ImproveAction {
    /// List every proposal in both stores, with its target, basis,
    /// status, and last gate result.
    List,
    /// Run one proposal through the gate: validate, ask for approval
    /// when the change is destructive, apply, gate, record.
    Apply {
        name: String,
        /// Approval in the current turn, for a destructive change or a
        /// confirmed-by memory target.
        approve: bool,
    },
    /// Revert one applied change: structurally for a repo-side apply,
    /// byte-for-byte from the pre-image for a home-side one.
    Undo { name: String },
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
  fleet memories [--recall \"<instruction>\"] [--shape <name>] [--consolidate] [--archive]
                                the durable facts as cards; --recall prints the
                                ranked assembly for one instruction, --shape
                                filters the cards by structure shape,
                                --consolidate runs the consolidation cycle,
                                --archive lists what the cycle forgot
  fleet cron                     list the scheduled jobs: schedule, next fire,
                                last run and outcome
  fleet cron add --name <name> --schedule \"<expr>\" --task \"<goal>\" [--auto-approve]
                                schedule a task; the expression is five fields,
                                minute hour day-of-month month day-of-week,
                                and --auto-approve lets its tool calls run
                                unattended
  fleet cron enable --name <name>
  fleet cron disable --name <name>
  fleet cron remove --name <name>
  fleet improve                list the proposals awaiting the gate: both
                                stores, with target, basis, status, and
                                the last gate result
  fleet improve apply <name> [--approve]
                                run one proposal through the gate — fmt,
                                clippy, build, test — and record the
                                verdict; --approve confirms a destructive
                                change in this turn
  fleet improve undo <name>    revert an applied change: structurally for
                                a repo-side apply, byte-for-byte from the
                                pre-image for a home-side one
  fleet remember --name <name> --description \"<fact>\" --basis confirmed-by|inferred
                                [--type <kind>] [--shape a,b] [--tier working|durable]
                                [--valid-until <date>] [--body \"<text>\"]
                                write one fact into the memory corpus — the CLI
                                form of the remember tool; basis is required, and
                                valid-until is required for a working-tier fact
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

options for cron:
  --name <name>       the job to act on, for every subcommand
  --schedule <expr>   the five-field cron expression, for add
  --task <goal>       the self-contained goal, for add
  --auto-approve      declare the job unattended: its tool calls run
                      without anyone there to approve them (add)

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
        "memories" => parse_memories(rest),
        "cron" => parse_cron(rest),
        "improve" => parse_improve(rest),
        "remember" => parse_remember(rest),
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

/// `fleet memories`: the bare command cards the live facts;
/// `--recall "<instruction>"` prints the ranked assembly for one
/// instruction (a dry run — printing records no hit); `--shape <name>`
/// filters the cards by structure shape; `--consolidate` runs the
/// consolidation cycle and then cards the corpus; `--archive` lists
/// what the cycle forgot. `--recall` and `--shape` do not combine: one
/// prints an assembly, the other filters cards, and a caller who
/// wants both is asking two questions. `--archive` is its own surface
/// and combines with nothing.
fn parse_memories(args: &[String]) -> Result<Command, String> {
    let mut recall: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut consolidate = false;
    let mut archive = false;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--recall" => {
                let value = take_value(args, &mut index, arg)?;
                if value.trim().is_empty() {
                    return Err("--recall needs an instruction".to_string());
                }
                recall = Some(value);
            }
            "--shape" => shape = Some(take_value(args, &mut index, arg)?),
            "--consolidate" => consolidate = true,
            "--archive" => archive = true,
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown memories option `{arg}`"));
            }
            _ => return Err(format!("`{arg}` takes no value here; see `fleet help`")),
        }
        index += 1;
    }
    if recall.is_some() && shape.is_some() {
        return Err(
            "--recall prints the assembly and --shape filters the cards; they do not combine"
                .to_string(),
        );
    }
    if archive && (recall.is_some() || shape.is_some() || consolidate) {
        return Err(
            "--archive lists what the cycle forgot; it is its own surface and combines with nothing"
                .to_string(),
        );
    }
    if consolidate && recall.is_some() {
        return Err(
            "--consolidate runs the cycle and then cards the corpus; --recall prints an assembly"
                .to_string(),
        );
    }
    Ok(Command::Memories(MemoriesOptions { recall, shape, consolidate, archive }))
}

/// `fleet cron`: the bare command lists the jobs; `add` creates one from
/// structured flags; `enable`, `disable`, and `remove` mutate by name.
/// Every subcommand addresses its job with `--name`, one grammar for the
/// whole surface, and a missing flag is a named usage error rather than a
/// silently wrong mutation.
fn parse_cron(args: &[String]) -> Result<Command, String> {
    let Some(subcommand) = args.first() else {
        return Ok(Command::Cron(CronOptions { action: CronAction::List }));
    };
    if *subcommand == "--help" || *subcommand == "-h" {
        return Ok(Command::Help);
    }
    let rest = &args[1..];
    match subcommand.as_str() {
        "add" => {
            let mut name: Option<String> = None;
            let mut schedule: Option<String> = None;
            let mut task: Option<String> = None;
            let mut auto_approve = false;
            let mut index = 0;
            while index < rest.len() {
                let arg = rest[index].as_str();
                match arg {
                    "--help" | "-h" => return Ok(Command::Help),
                    "--name" => name = Some(take_value(rest, &mut index, arg)?),
                    "--schedule" => schedule = Some(take_value(rest, &mut index, arg)?),
                    "--task" => task = Some(take_value(rest, &mut index, arg)?),
                    "--auto-approve" => auto_approve = true,
                    _ if arg.starts_with('-') && arg != "-" => {
                        return Err(format!("unknown cron option `{arg}`"));
                    }
                    _ => {
                        return Err(format!(
                            "`{arg}` takes no value here; see `fleet help`"
                        ));
                    }
                }
                index += 1;
            }
            let (Some(name), Some(schedule), Some(task)) = (name, schedule, task) else {
                return Err(
                    "cron add needs --name, --schedule, and --task (see `fleet help`)".to_string(),
                );
            };
            Ok(Command::Cron(CronOptions {
                action: CronAction::Add { name, schedule, task, auto_approve },
            }))
        }
        "enable" | "disable" | "remove" => {
            let mut name: Option<String> = None;
            let mut index = 0;
            while index < rest.len() {
                let arg = rest[index].as_str();
                match arg {
                    "--help" | "-h" => return Ok(Command::Help),
                    "--name" => name = Some(take_value(rest, &mut index, arg)?),
                    _ if arg.starts_with('-') && arg != "-" => {
                        return Err(format!("unknown cron option `{arg}`"));
                    }
                    _ => {
                        return Err(format!(
                            "`{arg}` takes no value here; the job is named with --name"
                        ));
                    }
                }
                index += 1;
            }
            let Some(name) = name else {
                return Err(format!("cron {subcommand} needs --name <name> (see `fleet help`)"));
            };
            let action = match subcommand.as_str() {
                "enable" => CronAction::Enable { name },
                "disable" => CronAction::Disable { name },
                _ => CronAction::Remove { name },
            };
            Ok(Command::Cron(CronOptions { action }))
        }
        other => Err(format!("unknown cron subcommand `{other}` \u{2014} try `fleet help`")),
    }
}

/// `fleet improve`: the bare command lists every proposal in both
/// stores; `apply <name>` runs one through the gate, with `--approve`
/// confirming a destructive change in this turn; `undo <name>` reverts
/// an applied one. The proposal is named positionally — one grammar
/// for the whole surface — and a missing name is a named usage error.
fn parse_improve(args: &[String]) -> Result<Command, String> {
    let Some(subcommand) = args.first() else {
        return Ok(Command::Improve(ImproveOptions { action: ImproveAction::List }));
    };
    if *subcommand == "--help" || *subcommand == "-h" {
        return Ok(Command::Help);
    }
    let rest = &args[1..];
    let verb = subcommand.as_str();
    let mut name: Option<String> = None;
    let mut approve = false;
    let mut index = 0;
    while index < rest.len() {
        let arg = rest[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--approve" if verb == "apply" => approve = true,
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown improve option `{arg}`"));
            }
            _ if name.is_none() => name = Some(arg.to_string()),
            _ => {
                return Err(format!(
                    "`{arg}` takes no value here; the proposal is named once (see `fleet help`)"
                ));
            }
        }
        index += 1;
    }
    let Some(name) = name else {
        return Err(format!("fleet improve {verb} needs a proposal name (see `fleet help`)"));
    };
    let action = match verb {
        "apply" => ImproveAction::Apply { name, approve },
        "undo" => ImproveAction::Undo { name },
        other => {
            return Err(format!(
                "unknown improve subcommand `{other}` \u{2014} try `fleet help`"
            ))
        }
    };
    Ok(Command::Improve(ImproveOptions { action }))
}

/// `fleet remember`: the CLI form of the write tool. `--name`,
/// `--description`, and `--basis` are required flags — a usage error
/// names the missing one — and every value is validated by the same
/// parser the model-visible tool uses, so the two surfaces cannot
/// drift.
fn parse_remember(args: &[String]) -> Result<Command, String> {
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut fact_type: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut basis: Option<String> = None;
    let mut tier: Option<String> = None;
    let mut valid_until: Option<String> = None;
    let mut body: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--help" | "-h" => return Ok(Command::Help),
            "--name" => name = Some(take_value(args, &mut index, arg)?),
            "--description" => description = Some(take_value(args, &mut index, arg)?),
            "--type" => fact_type = Some(take_value(args, &mut index, arg)?),
            "--shape" => shape = Some(take_value(args, &mut index, arg)?),
            "--basis" => basis = Some(take_value(args, &mut index, arg)?),
            "--tier" => tier = Some(take_value(args, &mut index, arg)?),
            "--valid-until" => valid_until = Some(take_value(args, &mut index, arg)?),
            "--body" => body = Some(take_value(args, &mut index, arg)?),
            _ if arg.starts_with('-') && arg != "-" => {
                return Err(format!("unknown remember option `{arg}`"));
            }
            _ => return Err(format!("`{arg}` takes no value here; see `fleet help`")),
        }
        index += 1;
    }
    let (Some(name), Some(description), Some(basis)) = (name, description, basis) else {
        return Err(
            "remember needs --name, --description, and --basis (see `fleet help`)".to_string(),
        );
    };
    Ok(Command::Remember(RememberOptions {
        name,
        description,
        fact_type: fact_type.unwrap_or_default(),
        shape: shape.unwrap_or_default(),
        basis,
        tier: tier.unwrap_or_default(),
        valid_until: valid_until.unwrap_or_default(),
        body: body.unwrap_or_default(),
    }))
}

/// A real calendar check on an already-`YYYY-MM-DD`-shaped string: the
/// month is 1-12 and the day is within that month's own length (leap
/// years included). Without it, `--day 2026-13-45` reads as "Fleet did
/// not run" rather than "that day does not exist" — the shape is numeric,
/// but the day is not real.
pub(crate) fn valid_calendar_day(day: &str) -> bool {
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
    fn memories_parses_its_flags_and_rejects_junk() {
        assert_eq!(
            parse(&args(&["memories"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: None,
                shape: None,
                consolidate: false,
                archive: false
            })
        );
        assert_eq!(
            parse(&args(&["memories", "--recall", "the deploy is failing"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: Some("the deploy is failing".to_string()),
                shape: None,
                consolidate: false,
                archive: false
            })
        );
        assert_eq!(
            parse(&args(&["memories", "--shape", "deploy"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: None,
                shape: Some("deploy".to_string()),
                consolidate: false,
                archive: false
            })
        );
        // The cycle and the archive are the part-2 surfaces: the cycle
        // composes with the shape filter, the archive with nothing.
        assert_eq!(
            parse(&args(&["memories", "--consolidate"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: None,
                shape: None,
                consolidate: true,
                archive: false
            })
        );
        assert_eq!(
            parse(&args(&["memories", "--consolidate", "--shape", "deploy"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: None,
                shape: Some("deploy".to_string()),
                consolidate: true,
                archive: false
            })
        );
        assert_eq!(
            parse(&args(&["memories", "--archive"])).unwrap(),
            Command::Memories(MemoriesOptions {
                recall: None,
                shape: None,
                consolidate: false,
                archive: true
            })
        );
        // The two flags answer two different questions and do not
        // combine.
        assert!(parse(&args(&["memories", "--recall", "x", "--shape", "deploy"])).is_err());
        // An empty instruction is a usage error, not a silent empty
        // recall.
        assert!(parse(&args(&["memories", "--recall", ""])).is_err());
        assert!(parse(&args(&["memories", "--recall"])).is_err());
        assert!(parse(&args(&["memories", "--json"])).is_err());
        assert!(parse(&args(&["memories", "extra"])).is_err());
        // The archive is its own surface; the cycle does not print an
        // assembly.
        assert!(parse(&args(&["memories", "--archive", "--recall", "x"])).is_err());
        assert!(parse(&args(&["memories", "--archive", "--consolidate"])).is_err());
        assert!(parse(&args(&["memories", "--consolidate", "--recall", "x"])).is_err());
        assert_eq!(parse(&args(&["memories", "--help"])).unwrap(), Command::Help);
    }

    #[test]
    fn remember_parses_its_structured_flags() {
        let command = parse(&args(&[
            "remember",
            "--name", "the-gate-is-local",
            "--description", "the gate is local and its exit code is read",
            "--basis", "confirmed-by",
        ]))
        .unwrap();
        assert_eq!(
            command,
            Command::Remember(RememberOptions {
                name: "the-gate-is-local".to_string(),
                description: "the gate is local and its exit code is read".to_string(),
                fact_type: String::new(),
                shape: String::new(),
                basis: "confirmed-by".to_string(),
                tier: String::new(),
                valid_until: String::new(),
                body: String::new(),
            })
        );
        // Every flag parses in any position.
        let command = parse(&args(&[
            "remember",
            "--tier", "working",
            "--description", "a session-scoped note",
            "--name", "a-note",
            "--valid-until", "2026-10-20",
            "--basis", "inferred",
            "--type", "project",
            "--shape", "deploy, build",
            "--body", "The full content.",
        ]))
        .unwrap();
        let Command::Remember(options) = command else { panic!("not a remember") };
        assert_eq!(options.name, "a-note");
        assert_eq!(options.basis, "inferred");
        assert_eq!(options.tier, "working");
        assert_eq!(options.valid_until, "2026-10-20");
        assert_eq!(options.shape, "deploy, build");
        assert_eq!(options.body, "The full content.");
        // The three required flags are named when missing.
        assert!(parse(&args(&["remember"])).is_err());
        assert!(parse(&args(&["remember", "--name", "x"])).is_err());
        assert!(parse(&args(&["remember", "--name", "x", "--description", "d"])).is_err());
        let error = parse(&args(&["remember", "--name", "x", "--description", "d"])).unwrap_err();
        assert!(error.contains("--basis"), "{error}");
        assert!(parse(&args(&["remember", "--name"])).is_err());
        assert!(parse(&args(&["remember", "--name", "x", "--description", "d", "--basis", "b", "--json"])).is_err());
        assert_eq!(
            parse(&args(&["remember", "--name", "x", "--description", "d", "--basis", "b", "--help"]))
                .unwrap(),
            Command::Help
        );
    }

    #[test]
    fn cron_parses_the_bare_list_and_every_subcommand() {
        assert_eq!(
            parse(&args(&["cron"])).unwrap(),
            Command::Cron(CronOptions { action: CronAction::List })
        );
        assert_eq!(
            parse(&args(&[
                "cron", "add", "--name", "daily-digest", "--schedule", "0 7 * * *",
                "--task", "Summarize the day.", "--auto-approve",
            ]))
            .unwrap(),
            Command::Cron(CronOptions {
                action: CronAction::Add {
                    name: "daily-digest".to_string(),
                    schedule: "0 7 * * *".to_string(),
                    task: "Summarize the day.".to_string(),
                    auto_approve: true,
                }
            })
        );
        // `--auto-approve` is optional and defaults off.
        let command = parse(&args(&[
            "cron", "add", "--name", "n", "--schedule", "* * * * *", "--task", "t",
        ]))
        .unwrap();
        let Command::Cron(options) = command else { panic!("not a cron") };
        let CronAction::Add { auto_approve, .. } = options.action else { panic!("not an add") };
        assert!(!auto_approve);
        // The mutations address the job by name, in any flag order.
        assert_eq!(
            parse(&args(&["cron", "enable", "--name", "daily-digest"])).unwrap(),
            Command::Cron(CronOptions { action: CronAction::Enable { name: "daily-digest".to_string() } })
        );
        assert_eq!(
            parse(&args(&["cron", "disable", "--name", "daily-digest"])).unwrap(),
            Command::Cron(CronOptions { action: CronAction::Disable { name: "daily-digest".to_string() } })
        );
        assert_eq!(
            parse(&args(&["cron", "remove", "--name", "daily-digest"])).unwrap(),
            Command::Cron(CronOptions { action: CronAction::Remove { name: "daily-digest".to_string() } })
        );
        // The three required add flags are named when missing.
        assert!(parse(&args(&["cron", "add"])).is_err());
        assert!(parse(&args(&["cron", "add", "--name", "n"])).is_err());
        let error = parse(&args(&["cron", "add", "--name", "n", "--schedule", "* * * * *"])).unwrap_err();
        assert!(error.contains("--task"), "{error}");
        // A mutation with no name is a usage error, not a no-op.
        let error = parse(&args(&["cron", "enable"])).unwrap_err();
        assert!(error.contains("--name"), "{error}");
        // Junk is refused on every subcommand.
        assert!(parse(&args(&["cron", "frobnicate"])).is_err());
        assert!(parse(&args(&["cron", "add", "--name", "n", "--schedule", "s", "--task", "t", "--json"])).is_err());
        assert!(parse(&args(&["cron", "enable", "daily-digest"])).is_err());
        assert!(parse(&args(&["cron", "--name", "n"])).is_err());
        assert_eq!(parse(&args(&["cron", "--help"])).unwrap(), Command::Help);
        assert_eq!(parse(&args(&["cron", "add", "--help"])).unwrap(), Command::Help);
    }

    #[test]
    fn improve_parses_the_bare_list_apply_and_undo() {
        assert_eq!(
            parse(&args(&["improve"])).unwrap(),
            Command::Improve(ImproveOptions { action: ImproveAction::List })
        );
        assert_eq!(
            parse(&args(&["improve", "apply", "fix-the-ranker"])).unwrap(),
            Command::Improve(ImproveOptions {
                action: ImproveAction::Apply { name: "fix-the-ranker".to_string(), approve: false }
            })
        );
        // `--approve` is apply's own flag and defaults off.
        assert_eq!(
            parse(&args(&["improve", "apply", "retire-demo", "--approve"])).unwrap(),
            Command::Improve(ImproveOptions {
                action: ImproveAction::Apply { name: "retire-demo".to_string(), approve: true }
            })
        );
        assert_eq!(
            parse(&args(&["improve", "undo", "fix-the-ranker"])).unwrap(),
            Command::Improve(ImproveOptions {
                action: ImproveAction::Undo { name: "fix-the-ranker".to_string() }
            })
        );
        // The proposal is named once; a second word or an unknown
        // subcommand is a usage error, and `--approve` belongs to apply.
        assert!(parse(&args(&["improve", "apply"])).is_err());
        assert!(parse(&args(&["improve", "undo"])).is_err());
        assert!(parse(&args(&["improve", "apply", "a", "b"])).is_err());
        assert!(parse(&args(&["improve", "frobnicate", "a"])).is_err());
        assert!(parse(&args(&["improve", "undo", "a", "--approve"])).is_err());
        assert_eq!(parse(&args(&["improve", "--help"])).unwrap(), Command::Help);
        assert_eq!(parse(&args(&["improve", "apply", "--help"])).unwrap(), Command::Help);
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
