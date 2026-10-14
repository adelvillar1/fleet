//! Fleet: a native Rust agent based on DeepSeek Harness.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::ExitCode;

use fleet::agent::{
    Agent, AgentError, AgentOptions, ApproveAll, Approver, ConsoleApprover, DEFAULT_MAX_TURNS,
    RunReport, StopReason,
};
use fleet::api::{Client, DEFAULT_BASE_URL};
use fleet::cli::{self, ChatOptions, Command, CronAction, ImproveAction, RunOptions};
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
        Ok(Command::Cron(options)) => cron_command(&options),
        Ok(Command::Improve(options)) => improve_command(&options),
        Ok(Command::Remember(options)) => remember_command(&options),
        Ok(Command::Doctor(options)) => doctor_command(&options),
        Ok(Command::Install(options)) => install_command(&options),
        Ok(Command::Eval(options)) => eval_command(&options).await,
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
        &home,
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
    // The scheduled consolidation cycle: one pass now, then one every
    // hour for the life of the session. The live system's episodic
    // store went dormant because its consolidation was manual — the
    // timer is the fix. The stream is the cycle's own, so the cycle
    // runs even before routing has produced one.
    let user_home = std::env::var("HOME").ok();
    let calib_root =
        fleet::calib::resolve_root(&home, user_home.as_deref());
    let memories_root =
        fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
    fleet::memory::spawn_consolidator(memories_root, fleet::calib::Stream::open(&calib_root));
    // The scheduled jobs: the catch-up pass once at startup, then one
    // tick per second for the life of the session. The scheduler reads
    // the job store fresh every tick, so `fleet cron add` from another
    // shell is picked up within a second, and each fire is an ordinary
    // headless run in its own session log. Its named lines ride stderr.
    fleet::cron::Scheduler::spawn(home.clone(), None);
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
            &home,
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
/// one instruction (`--recall`), the cards filtered by structure
/// shape (`--shape`), the consolidation cycle on demand
/// (`--consolidate`), or the archive of forgotten facts (`--archive`).
/// A read surface over the repo's own git-tracked files, like
/// `fleet calib` and `fleet skills`: the exit code is always success,
/// and a corpus with no facts says so rather than failing.
///
/// The corpus is the nearest `memories/` at or above the working
/// directory — the repo root in everyday use, the fork's
/// `resolveMemoryRoot` rule. The vector path is probed at load and the
/// surface's header names which backend serves; `--recall` is a dry run
/// that writes nothing, because the turn path that records hits is the
/// agent's.
///
/// `--consolidate` is the one writing flag: it runs one pass of the
/// cycle — the same pass the scheduled thread runs hourly in a chat or
/// the TUI — and prints what it did before the cards, with every
/// action's row in the calibration stream.
fn memories_command(options: &cli::MemoriesOptions) -> ExitCode {
    let root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
    if options.archive {
        print!("{}", fleet::memory::render_archive(&root));
        return ExitCode::SUCCESS;
    }
    if options.consolidate {
        // The cycle needs a stream to record into, so this is the one
        // memories surface that resolves the home. A root that cannot
        // resolve leaves the pass row-less, never failed.
        let stream = fleet_home().ok().map(|home| {
            let user_home = std::env::var("HOME").ok();
            fleet::calib::Stream::open(fleet::calib::resolve_root(
                &home,
                user_home.as_deref(),
            ))
        });
        let result = fleet::memory::consolidate(
            &root,
            &fleet::calib::today(),
            stream.as_ref(),
        );
        print!("{}\n", fleet::memory::render_consolidation(&result));
    }
    // The corpus is loaded after the cycle, so the cards show what the
    // pass left behind — the new summary in, the archived facts out.
    let memory = fleet::memory::Memory::load(&root);
    if let Some(instruction) = &options.recall {
        print!("{}", fleet::memory::render_recall(&memory, instruction));
    } else {
        print!("{}", fleet::memory::render(&memory, options.shape.as_deref()));
    }
    ExitCode::SUCCESS
}

/// `fleet remember`: write one fact into the corpus, in the house card
/// style. The CLI form of the `remember` tool — the user typing the
/// write is the approval, so there is no gate here — and the same
/// validator, so the two surfaces cannot drift. A typed refusal (a
/// taken name, a missing basis, a working fact with no `valid-until`)
/// prints its reason and exits 1.
fn remember_command(options: &cli::RememberOptions) -> ExitCode {
    let root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
    let memory = fleet::memory::Memory::load(&root);
    // The flags map onto the frontmatter's own field names, and an
    // absent optional flag is left out of the object entirely so the
    // writer's defaults apply.
    let mut arguments = serde_json::json!({
        "name": options.name,
        "description": options.description,
        "basis": options.basis,
    });
    let object = arguments.as_object_mut().expect("a fresh object");
    for (flag, value) in [
        ("type", &options.fact_type),
        ("shape", &options.shape),
        ("tier", &options.tier),
        ("valid-until", &options.valid_until),
        ("body", &options.body),
    ] {
        if !value.is_empty() {
            object.insert(flag.to_string(), serde_json::Value::String(value.clone()));
        }
    }
    match fleet::memory::Remember::parse(&arguments).and_then(|request| memory.remember(&request)) {
        Ok(fact) => {
            print!("{}", fleet::memory::render_one(&fact));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `fleet cron`: the scheduled jobs as cards (the bare command), or one
/// mutation of the persistent store (`add`, `enable`, `disable`,
/// `remove`). The listing is a read surface over Fleet's own file, like
/// `fleet calib` and `fleet skills`: the exit code is always success, and
/// a store with no jobs says so rather than failing. A mutation prints
/// the job's card beside the named line for what changed, and a typed
/// refusal (a taken name, a schedule that does not parse, a name that is
/// not in the store) prints its reason and exits 1.
fn cron_command(options: &cli::CronOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let now = fleet::cron::Minute::now();
    match &options.action {
        CronAction::List => match fleet::cron::Store::open(&home) {
            Ok(store) => {
                print!("{}", fleet::cron::render(&home, store.jobs(), now));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        },
        CronAction::Add { name, schedule, task, auto_approve } => {
            match fleet::cron::Store::open(&home).and_then(|mut store| {
                let job = store.add(name, schedule, task, *auto_approve)?;
                Ok(job)
            }) {
                Ok(job) => {
                    let next = fleet::cron::Schedule::parse(&job.schedule)
                        .ok()
                        .and_then(|schedule| schedule.next_after(now))
                        .map(|minute| format!(", next fire {}", minute.stamp()))
                        .unwrap_or_default();
                    println!("fleet: cron: added `{}` \u{2014} {}{next}", job.name, job.schedule);
                    print!("{}", fleet::cron::render(&home, std::slice::from_ref(&job), now));
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        CronAction::Enable { name } => {
            mutate(&home, name, now, "enabled", true)
        }
        CronAction::Disable { name } => {
            mutate(&home, name, now, "disabled", false)
        }
        CronAction::Remove { name } => match fleet::cron::Store::open(&home).and_then(|mut store| {
            let job = store.remove(name)?;
            Ok(job)
        }) {
            Ok(job) => {
                println!("fleet: cron: removed `{}` \u{2014} {}", job.name, job.schedule);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

/// One enable/disable mutation: the same store round-trip, the named
/// line, and the job's card.
fn mutate(
    home: &std::path::Path,
    name: &str,
    now: fleet::cron::Minute,
    verb: &str,
    enabled: bool,
) -> ExitCode {
    match fleet::cron::Store::open(home).and_then(|mut store| store.set_enabled(name, enabled)) {
        Ok(job) => {
            println!("fleet: cron: {verb} `{}` \u{2014} {}", job.name, job.schedule);
            print!("{}", fleet::cron::render(home, std::slice::from_ref(&job), now));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `fleet improve`: the self-maintenance surface. The bare command
/// lists every proposal in both stores as cards. `apply` runs one
/// proposal through the gate — the four steps, in order, in the
/// working directory as the repository root — and records the verdict;
/// a destructive change (or a confirmed-by memory target) waits for
/// `--approve`, the approval in the current turn. `undo` reverts an
/// applied change: structurally for a repo-side apply, byte-for-byte
/// from the pre-image for a home-side one.
///
/// The listing is a read surface like `fleet calib` and `fleet skills`
/// — always exit 0. `apply` and `undo` exit 0 only when the change
/// landed or reverted; a refusal, a wait, and a failed revert each
/// exit 1 with the reason named.
fn improve_command(options: &cli::ImproveOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    match &options.action {
        ImproveAction::List => {
            print!("{}", fleet::improve::render(&fleet::improve::load_all(&home)));
            ExitCode::SUCCESS
        }
        ImproveAction::Apply { name, approve } => {
            let repo = std::env::current_dir().unwrap_or_default();
            let user_home = std::env::var("HOME").ok();
            let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(
                &home,
                user_home.as_deref(),
            ));
            let gate = fleet::improve::Gate::local(&repo);
            // The eval-corpus gate probe for a source target: it runs
            // `fleet eval` as a subprocess so the gate grades the
            // freshly-built binary. The probe fails closed — it refuses
            // unless the current executable resolves under the gate
            // root's own target/ directory, so a source self-edit is
            // never waved through by an eval step that did not run, or
            // by a stale installed binary's green.
            let eval = Some(fleet::eval::subprocess_probe(&repo));
            let ctx = fleet::improve::Apply {
                home: &home,
                repo: &repo,
                gate: &gate,
                approved: *approve,
                stream: Some(&stream),
                eval,
            };
            let verdict = fleet::improve::apply(name, &ctx);
            match &verdict {
                fleet::improve::Verdict::Applied { .. } => {
                    println!("fleet: improve: {name} {}", verdict.line());
                    ExitCode::SUCCESS
                }
                _ => {
                    eprintln!("fleet: improve: {name} {}", verdict.line());
                    ExitCode::FAILURE
                }
            }
        }
        ImproveAction::Undo { name } => {
            let repo = std::env::current_dir().unwrap_or_default();
            match fleet::improve::undo(name, &home, &repo) {
                Ok(summary) => {
                    println!("fleet: improve: {summary}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: improve: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
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
    home: &std::path::Path,
) -> Result<RunReport, AgentError> {
    let client = Client::routed(prepared.router.clone());
    // The corpus is loaded per run, off the async worker: the files can
    // change between runs, and a fact an earlier run remembered is on
    // disk by the time this one assembles. The load is the turn path's
    // whole cost — one probe round trip plus the cached index — and a
    // load that cannot complete leaves the run memory-less rather than
    // failing it.
    let memory = tokio::task::spawn_blocking(|| {
        let root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
        std::sync::Arc::new(fleet::memory::Memory::load(&root))
    })
    .await
    .ok();
    let options = AgentOptions {
        model,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: Some(prepared.calib.clone()),
        skills,
        memory,
        scope: None,
        // The self-maintenance tool rides every surface that knows its
        // home: a session can file a proposal, and only the owner's
        // `fleet improve apply` runs one.
        improve_home: Some(home.to_path_buf()),
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
        // A delegated child's log is an ordinary session with a marker:
        // the listing names the kind beside the state, and a compacted
        // session is marked the same way — its replay is a compacted
        // shape, which a reader deserves to know before opening it.
        let kind = summary.kind.as_deref().unwrap_or("session");
        let markers = if summary.compacted { format!("{kind} compacted") } else { kind.to_string() };
        println!(
            "{:44} {:20} {:16} {} {}",
            summary.id,
            summary.created,
            summary.model,
            if summary.ended { "ended" } else { "open" },
            markers
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

fn doctor_command(options: &cli::DoctorOptions) -> ExitCode {
    // The diagnosis path: classify one session's failure and draft an
    // evidence-cited recap, filing (or naming) its ledger row.
    if let Some(session_id) = &options.diagnose {
        return diagnose_command(session_id, options.ledger.as_deref());
    }
    // The action-items read-back: the ledger's open rows with their
    // next actions, as cards. Read-only, always exit 0 — the ledger is
    // an artifact, not a gate.
    if options.open {
        return open_command(options.ledger.as_deref());
    }

    doctor_checks()
}

/// The doctor's boot checks: credentials, home, API reachability,
/// shell, the roster layer, the ledger layer, one named line per MCP
/// core, and the two informational lines. `fleet doctor` runs these
/// directly; `fleet install` ends on this same function — the gate is
/// the doctor's own checks, the install adds none, and the install's
/// exit code is this function's.
fn doctor_checks() -> ExitCode {
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

    // The roster layer: how many routing candidates can serve. Named
    // beside the credentials line — a keyless machine is the layer a
    // routed run cannot cross, and `doctor` says so rather than leaving
    // it to the first failed run.
    let roster_check = match &home {
        Some(home) => match route::load_roster(home) {
            Ok(roster) => {
                let resolved = route::resolve(&roster.candidates, &dotenv);
                let total = resolved.len();
                let ready = resolved
                    .iter()
                    .filter(|entry| route::provider_state(entry) == route::ProviderState::Ready)
                    .count();
                doctor::evaluate_roster(ready, total)
            }
            Err(error) => doctor::Check {
                name: "roster",
                ok: false,
                detail: format!("the roster did not load: {error}"),
            },
        },
        None => doctor::evaluate_roster(0, 0),
    };
    println!("{}", doctor::render(&roster_check));

    // The ledger layer: the calibration stream's root and whether it is
    // writable. The audit spine every self-improvement loop writes to,
    // so an unwritable one is a broken layer `doctor` names and fails on.
    let user_home = std::env::var("HOME").ok();
    let ledger_check = match &home {
        Some(home) => {
            let root = fleet::calib::resolve_root(home, user_home.as_deref());
            let writable = std::fs::create_dir_all(&root).is_ok();
            let check = doctor::evaluate_ledger(Some(&root), writable);
            failed |= !check.ok;
            check
        }
        None => {
            let check = doctor::evaluate_ledger(None, false);
            failed |= !check.ok;
            check
        }
    };
    println!("{}", doctor::render(&ledger_check));

    // The MCP cores, one named line each: whether each vendored lane's
    // home resolves. Named, not failed — a lane that does not resolve
    // degrades a run rather than blocking boot, and `fleet lanes` owns
    // the deep MCP health probe. This line names which of the four is
    // unavailable and why, the way the plan's layer list asks.
    for lane in fleet::lanes::LANES.iter() {
        let resolved = match fleet::lanes::resolve_home(lane) {
            fleet::lanes::LaneHome::Resolved { home, .. } => Ok(home),
            fleet::lanes::LaneHome::Unavailable { reason } => Err(reason),
        };
        let check = doctor::evaluate_mcp_core(lane.name, resolved.as_deref().map_err(|reason| reason.clone()));
        println!("{}", doctor::render(&check));
    }

    // The calibration-stream line: informational, never a check, and it
    // never changes the exit code. It names the root the stream actually
    // resolves to and whether today's file exists — the cheapest guard
    // against dev-decisions moving its LOG_DIR literal and silently
    // re-routing Fleet's rows.
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
    let probed = fleet::memory::probe(&fleet::memory::embedding_endpoint());
    let recall_check = doctor::evaluate_recall(probed.vector());
    println!("{}", doctor::render(&recall_check));

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `fleet install`: the home layout, the config scaffold, the optional
/// heavy-lane venvs, and the doctor gate.
///
/// Idempotent throughout: a second run changes nothing and names what
/// exists, and an existing `config.json` is never rewritten — an
/// owner's hand-edited config is theirs. The bare command names the
/// lane step as available rather than building it, so a clean-machine
/// install never blocks on a network; `--lanes` runs the per-lane
/// bootstrap, where a lane that cannot be built is skipped with a
/// named line and the install carries on. The command ends on the
/// doctor's own checks — the install adds no new check — and its exit
/// code is the doctor's: green 0, a named failure nonzero.
fn install_command(options: &cli::InstallOptions) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let mut layout: Vec<(&'static str, fleet::install::LayoutStep)> = Vec::new();
    for name in fleet::install::LAYOUT_DIRS.iter().copied() {
        match fleet::install::layout_step(&home, name) {
            Ok(step) => layout.push((name, step)),
            Err(error) => {
                eprintln!("fleet: install: {name}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let (config_step, config_path) = match fleet::install::config_step(&home) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("fleet: install: {}: {error}", fleet::install::CONFIG_FILE);
            return ExitCode::FAILURE;
        }
    };
    let lanes = if options.lanes {
        Some(
            fleet::install::LANES
                .iter()
                .map(|lane| (lane, fleet::install::bootstrap_lane(&home, lane, None)))
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    print!(
        "{}",
        fleet::install::render(&home, &layout, (config_step, &config_path), lanes.as_deref())
    );
    // The gate: the doctor's own checks, unchanged. The install's exit
    // code is the doctor's.
    doctor_checks()
}

/// `fleet doctor --diagnose <session>`: replay one session's log, classify
/// what is deterministically there, and draft the evidence-first recap.
/// The recap cites the ledger rows that already hold this failure's
/// signature — or names the row a filing would add. The filing writes
/// only where `--ledger <path>` points it; without it the command names
/// the repo's own `docs/known-issues.md` as the file it would write and
/// writes nothing, so the human-owned ledger stays human-owned until a
/// path is named.
fn diagnose_command(session_id: &str, ledger: Option<&str>) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let path = session::session_path(&home, session_id);
    if !path.exists() {
        eprintln!(
            "fleet: no session `{session_id}` under {}",
            home::sessions_dir(&home).display()
        );
        return ExitCode::from(2);
    }
    let diagnosis = match doctor::diagnose_session(&path) {
        Ok(Some(diagnosis)) => diagnosis,
        // An orderly completion is not a failure: nothing is drafted,
        // nothing is filed, and the surface says so rather than
        // inventing a "failure" out of a healthy run.
        Ok(None) => {
            println!("fleet: doctor: no failure diagnosed \u{2014} the run completed");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The ledger target: the file `--ledger` names, or the repo's own
    // `docs/known-issues.md` resolved from the working directory (the
    // improve gate's cwd convention, KI-099's family). It is named either
    // way; only an explicit `--ledger` writes.
    let named = ledger
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("docs")
                .join("known-issues.md")
        });
    print!("{}", doctor::render_diagnosis(&diagnosis, &named));
    match ledger {
        Some(_) => {
            let entry = doctor::LedgerEntry { severity: "defect", diagnosis };
            match doctor::file_ledger_row(&named, &entry) {
                Ok(filed) => {
                    let id = match filed {
                        doctor::Filed::Appended { id } | doctor::Filed::Updated { id } => id,
                    };
                    eprintln!("fleet: doctor: filed {id} in {}", named.display());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: doctor: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            eprintln!(
                "fleet: doctor: would file to {} \u{2014} name it with --ledger <path> to write",
                named.display()
            );
            ExitCode::SUCCESS
        }
    }
}

/// `fleet doctor --open`: the action-items read-back the plan's
/// Diagnosis clause asks for — an unimplemented action item stays
/// visible until it is closed. It reads the ledger and surfaces its
/// open rows with their next-action cells as cards. The bare form
/// names the repo's own `docs/known-issues.md` resolved from the
/// working directory (the improve gate's cwd convention, KI-099's
/// family) and writes nothing; `--ledger <path>` points the read at a
/// file. Either way it is read-only — the human-owned ledger stays
/// human-owned.
fn open_command(ledger: Option<&str>) -> ExitCode {
    let named = ledger.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("docs")
            .join("known-issues.md")
    });
    print!("{}", doctor::render_open_items(&named));
    ExitCode::SUCCESS
}

/// `fleet eval`: run the fixed corpus headless against the mock, print
/// house cards, and land one `eval_corpus` row; `--last` reads the most
/// recent run back without running. A read surface when `--last`, a run
/// otherwise — the exit code is success when the corpus is green (or the
/// last run is only read) and failure on a regression, so a scheduler or
/// a gate can read it.
async fn eval_command(options: &cli::EvalOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let user_home = std::env::var("HOME").ok();
    let root = fleet::calib::resolve_root(&home, user_home.as_deref());
    if options.last {
        print!("{}", fleet::eval::render_last(&root));
        return ExitCode::SUCCESS;
    }
    let Some(mock_bin) = fleet::eval::mock_server_binary() else {
        eprintln!(
            "fleet: eval: the mock-server binary was not found beside the fleet executable; the corpus cannot run"
        );
        return ExitCode::FAILURE;
    };
    let report = fleet::eval::run(&root, &mock_bin).await;
    print!("{}", fleet::eval::render(&report));
    if report.green() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
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
