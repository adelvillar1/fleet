//! Fleet: a native Rust agent based on DeepSeek Harness.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::process::ExitCode;

use fleet::agent::{
    Agent, AgentError, AgentOptions, Approver, ConsoleApprover, FileAccess, RunReport, StopReason,
    DEFAULT_MAX_TURNS,
};
use fleet::api::{Client, DEFAULT_BASE_URL};
use fleet::cli::{self, ChatOptions, Command, CronAction, ImproveAction, RunOptions, VaultAction};
use fleet::doctor;
use fleet::home;
use fleet::net;
use fleet::route;
use fleet::session::{self, Session};

/// Load the home's `fleet.env` into the process environment, skipping
/// credential-shaped names and never overriding a variable already set.
/// One helper, so a scripted export and the GUI's write have the same
/// precedence.
fn load_fleet_env() {
    let env = std::env::var("FLEET_HOME").ok();
    let user = std::env::var("HOME").ok();
    let Ok(home) = home::fleet_home(env.as_deref(), user.as_deref()) else {
        return;
    };
    for (name, value) in home::read_fleet_env(&home) {
        if name.ends_with("_KEY") || name.ends_with("_TOKEN") || name.ends_with("_SECRET") {
            continue;
        }
        if std::env::var_os(&name).is_some() {
            continue;
        }
        std::env::set_var(&name, value);
    }
}

fn main() -> ExitCode {
    // KI-128: restore the default SIGPIPE disposition. Rust sets it to
    // ignore, which turns a closed stdout pipe into an `io::Error` that
    // every `println!` unwraps — so `fleet sessions | head -5` killed
    // the process with "failed printing to stdout: Broken pipe". A CLI
    // piped into a pager is the ordinary case, not a crash.
    #[cfg(unix)]
    unsafe {
        unsafe extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        // SIGPIPE is 13 and SIG_DFL is 0 on every unix.
        signal(13, 0);
    }
    // The GUI-owned `fleet.env` is the settings surface's non-secret
    // counterpart file. Loading it before anything reads `std::env` is
    // what makes a write there take effect at the next launch; the
    // process environment always wins, and a credential-shaped name is
    // never loaded (secrets belong in `.env` or the vault).
    load_fleet_env();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // No subcommand is the TUI, Fleet's daily-driver surface. KI-023: it
    // gets no runtime at all — the multi-thread runtime `#[tokio::main]`
    // used to build for every launch parked ten worker threads for the
    // TUI's whole life while the agent ran on its own dedicated thread.
    // Now the TUI path builds nothing and the headless paths build what
    // they need.
    if args.is_empty() {
        return tui_command();
    }
    // The commands that await: each builds its own current-thread runtime
    // here, for the life of the one call. A current-thread runtime is
    // enough for a client that drives one run at a time, and it does not
    // park a pool nobody asked for.
    let needs_runtime = matches!(
        cli::parse(&args),
        Ok(Command::Run(_))
            | Ok(Command::Chat(_))
            | Ok(Command::Gui(_))
            | Ok(Command::Compact { .. })
            | Ok(Command::Lanes)
            | Ok(Command::Judge(_))
            | Ok(Command::Delegate(_))
            | Ok(Command::Eval(_))
            | Ok(Command::Explain(_))
            | Ok(Command::Mcp(_))
            | Ok(Command::Tools(_))
            | Ok(Command::Lifecycle(_))
    );
    if needs_runtime {
        return match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(dispatch(&args)),
            Err(error) => {
                eprintln!("fleet: the runtime could not start: {error}");
                ExitCode::FAILURE
            }
        };
    }
    futures_block_on_free(&args)
}

/// The sync commands' dispatch: everything that never awaits. Its own
/// function so the runtime check above stays a list of names and the
/// parse happens once per path.
fn futures_block_on_free(args: &[String]) -> ExitCode {
    dispatch_sync(args)
}

async fn dispatch(args: &[String]) -> ExitCode {
    match cli::parse(args) {
        Ok(Command::Run(options)) => run_command(&options).await,
        Ok(Command::Chat(options)) => chat_command(&options).await,
        Ok(Command::Gui(options)) => gui_command(&options).await,
        Ok(Command::Compact { session, force }) => compact_command(session.as_deref(), force).await,
        Ok(Command::Lanes) => lanes_command().await,
        Ok(Command::Mcp(options)) => mcp_command(&options).await,
        Ok(Command::Tools(options)) => tools_command(&options).await,
        Ok(Command::Lifecycle(options)) => lifecycle_command(&options),
        Ok(Command::Judge(options)) => judge_command(&options).await,
        Ok(Command::Delegate(options)) => delegate_command(&options).await,
        Ok(Command::Eval(options)) => eval_command(&options).await,
        Ok(Command::Explain(options)) => explain_command(&options).await,
        // The sync half owns every other command; the runtime check in
        // `main` is what keeps them apart.
        _ => dispatch_sync(args),
    }
}

/// The sync half, mirroring `dispatch`: every command that never awaits,
/// so the runtime question is answered by which half a command is in
/// rather than by a second parse.
fn dispatch_sync(args: &[String]) -> ExitCode {
    match cli::parse(args) {
        Ok(Command::Help) => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("fleet {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::SessionsView { recall, view }) => {
            sessions_command(recall.as_deref(), view.as_deref())
        }
        // The remote surface's own commands: none of them awaits, so they
        // live in the sync half with the rest.
        Ok(Command::Pair) => pair_command(),
        Ok(Command::Devices { revoke }) => devices_command(revoke.as_deref()),
        Ok(Command::Channels(options)) => channels_command(&options),
        Ok(Command::Settings { set }) => settings_command(
            set.as_ref()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        ),
        Ok(Command::Providers) => providers_command(),
        Ok(Command::Calib(options)) => calib_command(&options),
        Ok(Command::Roles) => roles_command(),
        Ok(Command::Convention(options)) => convention_command(&options),
        Ok(Command::Graph(options)) => graph_command(&options),
        Ok(Command::Plan(options)) => plan_command(&options),
        Ok(Command::Skills(options)) => skills_command(&options),
        Ok(Command::Memories(options)) => memories_command(&options),
        Ok(Command::Cron(options)) => cron_command(&options),
        Ok(Command::Improve(options)) => improve_command(&options),
        Ok(Command::Vault(options)) => vault_command(&options),
        Ok(Command::Remember(options)) => remember_command(&options),
        Ok(Command::Doctor(options)) => doctor_command(&options),
        Ok(Command::Install(options)) => install_command(&options),
        Err(message) => {
            eprintln!("fleet: {message}");
            eprint!("{}", cli::HELP);
            ExitCode::from(2)
        }
        // The awaiting half is dispatch's; reaching it here means the
        // runtime check above misrouted, which is a bug rather than a
        // fallback to invent.
        _ => {
            eprintln!("fleet: internal: an async command reached the sync dispatch");
            ExitCode::FAILURE
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
        model: fleet::route::AUTO_MODEL.to_string(),
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fleet: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Resolve a setting: the environment, then the vault, then
/// `~/.fleet/.env` — Fleet's own home, and nothing else (the owner's
/// 2026-10-16 ruling: no other harness's config is ever read).
///
/// The vault sits between the two because that is the migration order
/// the owner wants: an encrypted entry is preferred over a plaintext
/// line, so migrating a home stops the plaintext copy from being the one
/// that answers. A passphrase is required to read it, and a home with no
/// vault — or a passphrase that does not open it — falls through to the
/// plaintext leg rather than failing the run, because a run that cannot
/// start is worse than a run reading the key the owner already had
/// there.
fn setting(name: &str, dotenv: &std::collections::HashMap<String, String>) -> Option<String> {
    let from_env = std::env::var(name).ok().filter(|value| !value.is_empty());
    if from_env.is_some() {
        return from_env;
    }
    vault_get(name).or_else(|| dotenv.get(name).filter(|value| !value.is_empty()).cloned())
}

/// One entry out of the vault, when there is a vault and a passphrase
/// that opens it. A missing or unopenable vault is not an error here:
/// the caller falls through to the plaintext leg.
fn vault_get(name: &str) -> Option<String> {
    let home = fleet_home().ok()?;
    let path = home.join(fleet::vault::VAULT_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let vault = fleet::vault::Vault::from_json(&text).ok()?;
    let passphrase = home::vault_passphrase()?;
    vault.get(&passphrase, name).ok()
}

/// The `~/.fleet/.env` map with the vault laid over it, empty when no
/// home is known. Every key the run resolves — the router's roster keys
/// as much as the base URL — reads this map, so a sealed name answers
/// everywhere a plaintext name would have.
fn read_dotenv() -> std::collections::HashMap<String, String> {
    fleet_home()
        .ok()
        .as_deref()
        .map(home::read_dotenv_with_process_vault)
        .unwrap_or_default()
}

/// The fleet home, failing loud when no home is known.
fn fleet_home() -> Result<std::path::PathBuf, String> {
    let user_home = std::env::var("HOME").ok();
    home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        user_home.as_deref(),
    )
}

async fn run_command(options: &RunOptions) -> ExitCode {
    let user_home = std::env::var("HOME").ok();
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let dotenv = read_dotenv();
    // KI-045: the pre-flight, named. Slice 1 required `DEEPSEEK_API_KEY`
    // and exited 2 with the doctor's credentials render; routing
    // replaced that with per-candidate key resolution, so a machine
    // with no usable candidate at all now learns it from the first
    // failed run instead of before it. The pre-flight names the missing
    // key env vars when no candidate is usable — a keyless machine is
    // the layer a routed run cannot cross, and the run says so before
    // it starts rather than after.
    match route::preflight(&home, &dotenv) {
        Ok(()) => {}
        Err(missing) => {
            eprintln!(
                "fleet: no routing candidate is usable — the missing key env vars are: {}",
                missing.join(", ")
            );
            eprintln!(
                "fleet: set one of them (in the environment or <FLEET_HOME>/.env), or run `fleet doctor`"
            );
            return ExitCode::from(2);
        }
    }
    // `--model` is a pin, resolved by name against the roster before the
    // route is judged: a `provider/model` pair as written, a bare id to
    // the one provider that serves it, and a refusal naming every
    // spelling otherwise. No `--model` is the router's own choice.
    let pin = match options.model.as_deref() {
        Some(id) => match route::resolve_model_pin(&home, id) {
            Ok(pin) => Some(pin),
            Err(message) => {
                eprintln!("fleet: {message}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };
    // The route is prepared before the session exists: the session's
    // first record and the first line the owner reads name the route
    // that will actually serve, not the CLI's requested default — and
    // a failed route no longer leaves an empty session behind.
    let prepared = match route::prepare_with_pin(
        &home,
        &dotenv,
        &options.task,
        &HashMap::new(),
        pin.as_deref(),
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::from(2);
        }
    };
    let mut session =
        match open_session(&home, options.session.as_deref(), &prepared.decision.model) {
            Ok(session) => session,
            Err(message) => {
                eprintln!("fleet: {message}");
                return ExitCode::from(2);
            }
        };
    eprintln!("fleet: session {}", session.id());
    eprintln!(
        "route: {} ({}) · model {}",
        prepared.decision.provider,
        prepared.decision.how.label(),
        prepared.decision.model
    );
    let skills = load_skills(user_home.as_deref(), &home);
    let outcome = drive(
        &prepared,
        prepared.decision.model.clone(),
        options.max_turns,
        options.yes,
        options.compact_now,
        Some(home.join("checkpoints")),
        false,
        &mut session,
        &options.task,
        skills,
        &home,
    )
    .await;
    report_walk(&prepared);
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
    // `--model` is the same pin `fleet run` takes. A chat's log opens
    // before its first message exists, so the start record names the
    // pinned pair's model when there is one and the router's `auto`
    // otherwise; every end record names what actually served.
    let pin = match options.model.as_deref() {
        Some(id) => match route::resolve_model_pin(&home, id) {
            Ok(pin) => Some(pin),
            Err(message) => {
                eprintln!("fleet: {message}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };
    let start_model = pin
        .as_deref()
        .and_then(|pin| pin.split_once('/'))
        .map(|(_, model)| model.to_string())
        .unwrap_or_else(|| route::AUTO_MODEL.to_string());
    let mut session = match open_session(&home, options.session.as_deref(), &start_model) {
        Ok(session) => session,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    println!("fleet chat — session {} (/exit to quit)", session.id());
    // The scheduled consolidation cycle: one pass now, then one every
    // hour for the life of the session. The live system's episodic
    // store went dormant because its consolidation was manual — the
    // timer is the fix. The stream is the cycle's own, so the cycle
    // runs even before routing has produced one.
    let user_home = std::env::var("HOME").ok();
    let calib_root = fleet::calib::resolve_root(&home, user_home.as_deref());
    let memories_root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
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
    let skills = load_skills(user_home.as_deref(), &home);
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
            match route::prepare_with_pin(&home, &dotenv, trimmed, &HashMap::new(), pin.as_deref())
                .await
            {
                Ok(fresh) => {
                    eprintln!(
                        "route: {} ({}) · model {}",
                        fresh.decision.provider,
                        fresh.decision.how.label(),
                        fresh.decision.model
                    );
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
            prepared.decision.model.clone(),
            DEFAULT_MAX_TURNS,
            options.yes,
            false,
            Some(home.join("checkpoints")),
            false,
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
        report_walk(prepared);
        prepared.record_route(&judge_task);
    }
    // The session is over: draft its lessons into proposals, best-effort.
    draft_lessons(&home, session.path());
    ExitCode::SUCCESS
}

/// `fleet channels`: the channel service's surface. The bare command
/// lists the configured channels and their paired peers as cards; a
/// pairing mints a peer token printed once, exactly as a device's is;
/// a revoke forgets one. The store holds digests, so this surface names
/// who may act and never what would let them.
fn channels_command(options: &cli::ChannelsOptions) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    if let Some((name, callback, capability)) = &options.pair_webhook {
        // One pairing door for the app and the terminal: the store's
        // digest and the once-only token are minted in the same place,
        // so the two clients cannot disagree about what pairing means.
        let pairing = match fleet::channels::pair(
            &home,
            "webhook",
            // The webhook mints its own id from the token.
            "",
            name,
            *capability,
            callback.as_deref(),
        ) {
            Ok(pairing) => pairing,
            Err(error) => {
                eprintln!("fleet: channels: {error}");
                return ExitCode::FAILURE;
            }
        };
        let id = pairing.id.clone();
        // The token is the one copy; the store holds its digest.
        let token = pairing.token.clone().unwrap_or_default();
        // The port the process will actually bind: the configured one,
        // not the compiled-in default a flag used to name.
        let port = fleet::channels::ServiceConfig::from_settings(&home, None).webhook_port;
        println!("fleet channels — one webhook peer paired");
        println!("  peer:  {name}  ({id})");
        println!(
            "  may:   {}",
            match capability {
                fleet::channels::Capability::Task => "hand fleet a task and receive the outcome",
                fleet::channels::Capability::Observe =>
                    "be notified only — it cannot hand fleet a task",
            }
        );
        println!("  token: {token}");
        println!("  hook:  POST http://127.0.0.1:{port}/hook");
        println!("         with the header x-fleet-peer: {token}");
        match &pairing.callback {
            Some(callback) => println!(
                "  callback: {callback} — outcomes and seats go here when a delivery names none"
            ),
            None => println!(
                "  callback: none stored — each delivery names its own callback, or its outcomes go nowhere"
            ),
        }
        println!("  the token is printed once and stored as its digest only");
        return ExitCode::SUCCESS;
    }
    if let Some((chat_id, name, capability)) = &options.allow_telegram {
        // The peer is paired by the chat id, with no token to mint: the
        // chat id IS the credential on this platform, and the bot only
        // answers chats it has been told to answer.
        let name = name
            .clone()
            .unwrap_or_else(|| format!("telegram {chat_id}"));
        if let Err(error) =
            fleet::channels::pair(&home, "telegram", chat_id, &name, *capability, None)
        {
            eprintln!("fleet: channels: {error}");
            return ExitCode::FAILURE;
        }
        println!("fleet channels — telegram peer {name} ({chat_id})");
        println!(
            "  may:   {}",
            match capability {
                fleet::channels::Capability::Task => "hand fleet a task and receive the outcome",
                fleet::channels::Capability::Observe =>
                    "be notified only — it cannot hand fleet a task",
            }
        );
        println!("  the bot answers this chat's messages and posts outcomes back to it");
        return ExitCode::SUCCESS;
    }
    if let Some(id) = &options.revoke_webhook {
        let mut store = match fleet::channels::PeerStore::load(&home, "webhook") {
            Ok(store) => store,
            Err(error) => {
                eprintln!("fleet: channels: {error}");
                return ExitCode::FAILURE;
            }
        };
        if store.revoke(id) {
            if let Err(error) = store.save(&home, "webhook") {
                eprintln!("fleet: channels: {error}");
                return ExitCode::FAILURE;
            }
            println!("fleet channels — `{id}` is forgotten; its token no longer works");
            return ExitCode::SUCCESS;
        }
        eprintln!("fleet: channels: no webhook peer named `{id}`; `fleet channels` lists them");
        return ExitCode::FAILURE;
    }
    // The bare command: what is configured, and who may act.
    if let Some((channel, peer, capability)) = &options.capability {
        match fleet::channels::set_capability(&home, channel, peer, *capability) {
            Ok(()) => {
                let word = match capability {
                    fleet::channels::Capability::Task => "may act",
                    fleet::channels::Capability::Observe => "may only listen",
                };
                println!("fleet channels — {peer} on {channel} now {word}");
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("fleet: channels: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if options.pending {
        println!("fleet channels — peers that tried and are not paired");
        let mut any = false;
        for channel in fleet::channels::CHANNELS {
            for peer in fleet::channels::PendingStore::load(&home, channel).peers {
                any = true;
                let name = if peer.name.is_empty() {
                    "(no name announced)".to_string()
                } else {
                    peer.name.clone()
                };
                println!(
                    "  {channel}  {id}  {name} · {attempts} attempt{s} · last {last}",
                    id = peer.id,
                    attempts = peer.attempts,
                    s = if peer.attempts == 1 { "" } else { "s" },
                    last = peer.last_seen,
                );
            }
        }
        if !any {
            println!("  nothing is waiting");
        }
        return ExitCode::SUCCESS;
    }
    println!("fleet channels — the channel service");
    // The service's own state first, from the one document every surface
    // reads: whether anything is running, whether the heartbeat is still
    // beating, and what each channel's phase is. Without this the list
    // answered "who is paired" and left "is it running" to a guess.
    match fleet::channels::Status::load(&home) {
        Some(status) => {
            let stale = status.is_stale(std::time::SystemTime::now());
            println!(
                "  service  {pid} · started {started} · {state}",
                pid = status.pid,
                started = status.started,
                state = if stale {
                    "stale — that process is no longer reporting"
                } else {
                    "live"
                }
            );
            for report in &status.channels {
                let port = match report.port {
                    Some(port) => format!(" on 127.0.0.1:{port}"),
                    None => String::new(),
                };
                let error = match &report.error {
                    Some(error) => format!(" · {error}"),
                    None => String::new(),
                };
                // A stale document's phases are the last thing a dead
                // process knew, so they are labelled as such rather than
                // read as a live channel's state.
                let last = if stale { " (last report)" } else { "" };
                println!(
                    "    {name}: {phase}{port}{error}{last}",
                    name = report.name,
                    phase = report.phase.label()
                );
            }
        }
        None => println!("  service  no channel service has run in this home"),
    }
    // Every channel the build ships, not the webhook alone: a Telegram
    // peer the owner paired is as much a way into the service, and a list
    // that named one channel read as if the other had none.
    // The port the process will bind, from the same setting the service
    // reads — not the compiled-in default, which is what a machine that
    // configured another port does not use.
    let configured_port = fleet::channels::ServiceConfig::from_settings(&home, None).webhook_port;
    for channel in fleet::channels::CHANNELS {
        let store = fleet::channels::PeerStore::load(&home, channel).unwrap_or_default();
        let mut line = format!("  {channel}  {} paired", store.peers.len());
        if *channel == "webhook" {
            line.push_str(&format!(" · port {configured_port}"));
        }
        println!("{line}");
        for peer in &store.peers {
            let capability = match peer.capability {
                fleet::channels::Capability::Task => "may act",
                fleet::channels::Capability::Observe => "may only listen",
            };
            println!(
                "    {id}  {name} · {capability}",
                id = peer.id,
                name = peer.name
            );
        }
        if store.peers.is_empty() {
            match *channel {
                "webhook" => {
                    println!("    `fleet channels pair-webhook <name> --callback <url>` pairs one")
                }
                "telegram" => println!("    `fleet channels allow-telegram <chat-id>` pairs one"),
                _ => {}
            }
        }
    }
    println!(
        "  `channels.enabled` in the settings, or `fleet gui --channels`, serves the face these peers reach"
    );
    ExitCode::SUCCESS
}

/// `fleet pair`: mint a pairing code and print it with the address a
/// phone opens. The window lands in the home, so this command needs no
/// running face — the listener reads the window when a code arrives.
fn pair_command() -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    // The address first, so a machine with nothing to print mints no
    // window: a live code nobody was told about is worse than no code.
    let remote = fleet::remote::RemoteOptions::default();
    let address = match fleet::remote::resolve_address(&remote) {
        Ok(found) => found,
        Err(_) => {
            eprintln!(
                "fleet: pair: no address to print — pass the address to `fleet gui --remote --remote-host <address>` and pair from there"
            );
            return ExitCode::FAILURE;
        }
    };
    let (_window, code) = match fleet::remote::pairing::PairingWindow::mint(&home) {
        Ok(minted) => minted,
        Err(error) => {
            eprintln!("fleet: pair: {error}");
            return ExitCode::FAILURE;
        }
    };
    let port = fleet::gui::DEFAULT_PORT;
    let kind = match address.kind {
        fleet::remote::AddressKind::Detected => "this machine's address",
        fleet::remote::AddressKind::Named => "the address you named",
        fleet::remote::AddressKind::Tailnet => "your tailnet address",
    };
    println!("fleet pair — one pairing code, single use");
    println!("  code:  {}", fleet::remote::pairing::display_code(&code));
    println!(
        "  open:  http://{}:{port}/pair",
        format_address(address.address)
    );
    println!("  from:  {kind}");
    // The QR: the same URL with the code in the query, so a scan lands
    // on the form with the code already filled.
    let url = fleet::remote::pairing_url(address.address, port, &code);
    match fleet::remote::qr::encode(&url) {
        Ok(code_matrix) => {
            println!();
            print!("{}", code_matrix.to_ansi());
        }
        Err(error) => println!("  (no QR: {error}) — type the code instead"),
    }
    println!(
        "  the code lives {} minutes and burns on use; `fleet devices` lists what paired",
        fleet::remote::pairing::WINDOW_TTL_MINUTES
    );
    ExitCode::SUCCESS
}

/// An address as a URL authority: IPv6 bracketed, IPv4 bare.
fn format_address(address: std::net::IpAddr) -> String {
    match address {
        std::net::IpAddr::V6(address) => format!("[{address}]"),
        std::net::IpAddr::V4(address) => address.to_string(),
    }
}

/// `fleet devices`: the paired devices as cards, or one revoked by id.
/// The store holds hashes, so this surface names devices and never a
/// credential — a reader learns who is paired, not what would let them
/// in.
fn devices_command(revoke: Option<&str>) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let mut store = match fleet::remote::pairing::DeviceStore::load(&home) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("fleet: devices: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(id) = revoke {
        if store.revoke(id) {
            if let Err(error) = store.save(&home) {
                eprintln!("fleet: devices: {error}");
                return ExitCode::FAILURE;
            }
            println!("fleet devices — `{id}` is forgotten; its token no longer works");
            return ExitCode::SUCCESS;
        }
        eprintln!("fleet: devices: no paired device named `{id}`; `fleet devices` lists them");
        return ExitCode::FAILURE;
    }
    if store.devices.is_empty() {
        println!("fleet devices — nothing is paired");
        println!("  `fleet pair` starts a pairing code, and `fleet gui --remote` serves the face it pairs to");
        return ExitCode::SUCCESS;
    }
    println!("fleet devices — {} paired", store.devices.len());
    for device in &store.devices {
        println!("  {id}  {name}", id = device.id, name = device.name);
        println!(
            "    paired {created} · last seen {seen}",
            created = device.created_at,
            seen = device.last_seen_at
        );
    }
    println!("  `fleet devices --revoke <id>` forgets one");
    ExitCode::SUCCESS
}

/// `fleet gui`: the browser face. The parser has already checked the
/// host and the port, so the one failure that reaches here is a bind the
/// parser could not see coming — a port already taken — which is named
/// and exits nonzero rather than falling back to another port the owner
/// did not ask for.
async fn gui_command(options: &cli::GuiOptions) -> ExitCode {
    let options = fleet::gui::GuiOptions {
        host: options.host.clone(),
        port: options.port,
        no_open: options.no_open,
        remote: fleet::remote::RemoteOptions {
            enabled: options.remote,
            host: options.remote_host.clone(),
        },
        channels: options.channels,
        relay: options.relay.clone(),
    };
    match fleet::gui::serve(options).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fleet: {message}");
            ExitCode::FAILURE
        }
    }
}

/// After a run: what the walk actually did. The attempts in order —
/// served, failed with its class, or skipped naming the env var it
/// wanted — so "why did this candidate serve" is answerable without
/// opening the stream. The route row carries the same facts for the
/// calibration stream; this is the owner-facing half.
fn report_walk(prepared: &route::Prepared) {
    let report = prepared.router.walk_report();
    if report.attempts.is_empty() {
        return;
    }
    let key_env = |provider: &str| {
        report
            .unkeyed
            .iter()
            .find(|(skipped, _)| skipped == provider)
            .map(|(_, env)| env.clone())
    };
    let walked = report
        .attempts
        .iter()
        .map(|attempt| match attempt.outcome {
            "served" => format!("{} served", attempt.provider),
            "skipped" => match key_env(&attempt.provider) {
                Some(env) => format!("{} skipped (no {})", attempt.provider, env),
                None => format!("{} skipped", attempt.provider),
            },
            _ => format!(
                "{} failed ({})",
                attempt.provider,
                attempt.class.unwrap_or("unknown")
            ),
        })
        .collect::<Vec<_>>()
        .join(" → ");
    eprintln!("walk: {walked}");
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
    let skills_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    if let Some(canonical) =
        fleet::skills::resolve_skills_dir(skills_dir.as_deref(), user_home.as_deref())
    {
        let filed =
            fleet::skills::draft_session(&canonical, home, session_path, user_home.as_deref());
        if !filed.is_empty() {
            eprintln!(
                "fleet: skills: drafted {} proposal(s) awaiting the gate in {}",
                filed.len(),
                fleet::skills::proposals_dir(home).display()
            );
        }
    }
}

/// The canonical library's skills, for the surfaces that declare them:
/// `$FLEET_SKILLS_DIR` wins, else `<user home>/.agents/skills`. When
/// neither is known there are no skills, and one line says so.
fn load_skills(user_home: Option<&str>, home: &std::path::Path) -> Vec<fleet::skills::Skill> {
    let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    match fleet::skills::resolve_skills_dir(override_dir.as_deref(), user_home) {
        // The declared list: the library with the owner's retired
        // filtered out — the skills page is where a retirement happens,
        // and the library is never written by it (KI-215).
        Some(dir) => {
            fleet::skills::note_retired(home);
            fleet::skills::load_declared(&dir, home)
        }
        None => {
            eprintln!(
                "fleet: skills: no skills directory resolvable \u{2014} set FLEET_SKILLS_DIR or HOME"
            );
            Vec::new()
        }
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
/// One lifecycle calibration row, on the stream the beats use. The row
/// is the audit trail for "why is this disabled"; the config write is
/// the fact, the row is the reason.
fn lifecycle_row(home: &std::path::Path, name: &str, action: &str, reason: &str) {
    let user_home = std::env::var("HOME").ok();
    let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(home, user_home.as_deref()));
    fleet::calib::record_lifecycle(&stream, "tools", name, action, reason);
}

/// `fleet tools disable <name> [--force]`: the manual verb. A protected
/// tool refuses without `--force` and names the protection.
fn tools_disable(home: &std::path::Path, name: &str, force: bool) -> ExitCode {
    if fleet::lifecycle::PROTECTED_TOOLS.contains(&name) && !force {
        eprintln!(
            "fleet: `{name}` is protected \u{2014} the working core the sweep never touches. \
             `--force` disables it anyway."
        );
        return ExitCode::FAILURE;
    }
    let known = fleet::tools::known_tool_names().contains(&name)
        || name.starts_with(fleet::tools::MCP_NAMESPACE);
    if !known {
        eprintln!("fleet: no tool named `{name}` is declared by this harness");
        return ExitCode::FAILURE;
    }
    let reason = if force {
        "forced by the owner".to_string()
    } else {
        "disabled by the owner".to_string()
    };
    match fleet::settings::set_tool_disabled(home, name, true) {
        Ok(()) => {
            lifecycle_row(home, name, "disable", &reason);
            println!("fleet: disabled `{name}` \u{2014} `fleet tools enable {name}` restores it");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `fleet tools enable <name>`: the one-command reverse.
fn tools_enable(home: &std::path::Path, name: &str) -> ExitCode {
    match fleet::settings::set_tool_disabled(home, name, false) {
        Ok(()) => {
            lifecycle_row(home, name, "enable", "enabled by the owner");
            println!("fleet: enabled `{name}` \u{2014} the next run declares it");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

struct Sem1Embedder {
    spec: fleet::mcp::ServerSpec,
}

impl fleet::tool_coverage::TurnEmbedder for Sem1Embedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        // The trait is synchronous and the lane is async. `block_in_place`
        // would do it on a multi-thread runtime, but every headless
        // command here runs on a current-thread one (KI-023), so the
        // work gets its own thread and its own runtime instead: the
        // embedder is called a handful of times per pass and the thread
        // costs less than a pool parked for the process's life.
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| format!("the embedder's runtime could not start: {error}"))?;
                runtime.block_on(async {
                    let timeouts = fleet::mcp::Timeouts::default();
                    let server = fleet::mcp::McpServer::spawn_with(&self.spec, timeouts)
                        .map_err(|error| format!("the sem1 lane could not start: {error}"))?;
                    let joined = async {
                        server.initialize().await?;
                        server.list_tools().await
                    }
                    .await
                    .map_err(|error| format!("the sem1 lane could not initialize: {error}"))?;
                    // The server's own name for the tool, verbatim: the
                    // roster strips the lane prefix for the model's
                    // eyes, and the wire takes the prefixed form.
                    let tool = joined
                        .iter()
                        .find(|tool| tool.name.ends_with("embed"))
                        .map(|tool| tool.name.clone())
                        .ok_or_else(|| "the sem1 lane lists no embed tool".to_string())?;
                    let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
                    for batch in texts.chunks(48) {
                        let items: Vec<String> = batch.to_vec();
                        let call = server
                            .call_tool(&tool, serde_json::json!({ "items": items }))
                            .await
                            .map_err(|error| format!("the sem1 lane failed: {error}"))?;
                        let parsed = fleet::tool_router::parse_vectors(&call.text())
                            .filter(|parsed| parsed.len() == batch.len())
                            .ok_or_else(|| {
                                "the sem1 lane answered with the wrong vector shape".to_string()
                            })?;
                        vectors.extend(parsed);
                    }
                    let _ = server.close().await;
                    Ok(vectors)
                })
            });
            handle
                .join()
                .unwrap_or_else(|_| Err("the embedder's thread panicked".to_string()))
        })
    }
}

/// `fleet tools coverage [--sessions N]`: owner intent over the archive,
/// through the sem1 lane, cached by message hash.
async fn tools_coverage(home: &std::path::Path, sessions: usize) -> ExitCode {
    let cards: Vec<(String, String)> = fleet::tools::ALL_TOOLS
        .iter()
        .map(|tool| {
            let spec = tool.spec();
            (spec.name().to_string(), spec.description().to_string())
        })
        .collect();
    let disabled = fleet::settings::tools_disabled(home);
    let spec = match fleet::lanes::spec_for("sem1") {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("fleet: the sem1 lane cannot launch: {error}");
            return ExitCode::FAILURE;
        }
    };
    let embedder = Sem1Embedder { spec };
    match fleet::tool_coverage::coverage(home, sessions, &embedder, &cards, &disabled) {
        Ok(report) => {
            print!("{}", fleet::tool_coverage::render(&report, sessions));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("fleet: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `fleet tools sweep [--apply]`: the gate computed over the archive,
/// dry-run by default. Applies at most `max_disable_per_sweep`, never a
/// protected, trial, or already-disabled tool, and records every write.
fn tools_sweep(home: &std::path::Path, apply: bool) -> ExitCode {
    let thresholds = fleet::settings::lifecycle_thresholds(home);
    let scan = fleet::tool_advisor::scan(home, thresholds.window);
    let metrics = fleet::tool_advisor::lifecycle_metrics(&scan);
    let skill_metrics = skill_metrics_from(home, thresholds.window);
    let user_home = std::env::var("HOME").ok();
    let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(home, user_home.as_deref()));
    let plan = fleet::lifecycle::sweep(
        home,
        &thresholds,
        &metrics,
        &skill_metrics,
        apply,
        Some(&stream),
    );
    // The dry run renders per kind; the applied run also writes the stamp.
    if apply {
        let _ = fleet::settings::record_sweep(home, &fleet::boards::now_stamp());
    }
    for (kind, result) in &plan {
        println!(
            "fleet lifecycle sweep ({}) \u{2014} {kind}: {} disabled, {} skipped",
            if apply { "apply" } else { "dry-run" },
            result.applied.len(),
            result.skipped.len()
        );
        for candidate in &result.applied {
            println!(
                "  disable  {}  \u{2014} {}",
                candidate.name, candidate.reason
            );
        }
        for (name, reason) in &result.skipped {
            println!("  skip     {name}  \u{2014} {reason}");
        }
    }
    for (kind, result) in &plan {
        for candidate in &result.applied {
            lifecycle_row(
                home,
                &candidate.name,
                "disable",
                &format!("{kind}: {}", candidate.reason),
            );
        }
    }
    ExitCode::SUCCESS
}

/// The skills' lifecycle metrics from the scan: declared runs and uses.
/// A skill's first-seen date is the run window's start; the exact
/// per-skill record arrives with the Skills records' own denominator.
fn skill_metrics_from(
    home: &std::path::Path,
    window: usize,
) -> Vec<fleet::lifecycle::SkillMetrics> {
    fleet::tool_advisor::scan_skills(home, window)
        .into_iter()
        .map(|skill| fleet::lifecycle::SkillMetrics {
            name: skill.name,
            declared_runs: skill.declared_runs,
            uses: skill.uses,
            first_seen: skill.first_seen,
        })
        .collect()
}

/// `fleet tools`: the eight built-ins, the usage scan over the session
/// archive, and the advisor over that scan. The scan reads files and
/// spends nothing; the advice is a rule over its counts, so the whole
/// surface answers without a lane or a model.
/// `fleet lifecycle promote`: the archive's repeated work, as
/// suggestions. It prints and exits; nothing is created, and the action
/// line is the exact command the owner would run.
fn lifecycle_command(options: &cli::LifecycleOptions) -> ExitCode {
    let cli::LifecycleAction::Promote {
        sessions,
        min_sessions,
        similarity_milli,
    } = options.action;
    let home = match fleet_home() {
        Ok(home) => home,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    let similarity = f64::from(similarity_milli) / 1000.0;
    let found = fleet::lifecycle::promote(&home, sessions, min_sessions, similarity, sessions);
    println!(
        "fleet lifecycle promote \u{2014} {} cluster(s) over the last {sessions} session(s), similarity {similarity:.2}, at least {min_sessions} each",
        found.len()
    );
    if found.is_empty() {
        println!(
            "\nno repeated work found. That is an answer: nothing in this archive repeats often enough to be worth a new tool, skill, loop, or workflow yet."
        );
        return ExitCode::SUCCESS;
    }
    println!("\n  suggestions are text, never actions: read one, then run its line if you agree\n");
    for promotion in &found {
        // The pattern is the cluster's own task, which can be a whole
        // briefing: one line here, and the action below is what the
        // owner actually acts on.
        let short: String = promotion.pattern.chars().take(110).collect();
        let short = if short.chars().count() < promotion.pattern.chars().count() {
            format!("{short}\u{2026}")
        } else {
            short
        };
        println!(
            "  {:<9} {} ({} session(s))",
            promotion.kind,
            short.replace('\n', " "),
            promotion.sessions
        );
        println!("            {}", promotion.action);
    }
    ExitCode::SUCCESS
}

async fn tools_command(options: &cli::ToolsOptions) -> ExitCode {
    if matches!(options.action, cli::ToolsAction::List) {
        let home = fleet_home().ok();
        let disabled: Vec<String> = home
            .as_deref()
            .map(fleet::settings::tools_disabled)
            .unwrap_or_default();
        // The whole native surface, in the order a run declares it: the
        // built-ins, then the tools that ride beside them. A list of the
        // eleven built-ins alone was the answer when that was the whole
        // surface, and it stopped being the whole surface.
        for spec in fleet::agent::Agent::base_declarations(&[]) {
            let builtin = fleet::tools::ALL_TOOLS
                .iter()
                .any(|tool| tool.name() == spec.name());
            let mark = if disabled.iter().any(|name| name == spec.name()) {
                "  [disabled]"
            } else if builtin {
                ""
            } else {
                "  [beside]"
            };
            println!("{}{}  {}", spec.name(), mark, spec.description());
        }
        println!(
            "\ndeferred past {} tools or {} definition tokens; `tool_search` names the rest",
            fleet::agent::MAX_VISIBLE_TOOLS,
            fleet::agent::MAX_DEFINITION_TOKENS
        );
        return ExitCode::SUCCESS;
    }
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    if let cli::ToolsAction::Audit { session } = &options.action {
        let path = match session {
            Some(id) => fleet::home::sessions_dir(&home).join(format!("{id}.jsonl")),
            None => match fleet::session::list(&home)
                .ok()
                .and_then(|mut list| list.pop())
            {
                Some(summary) => summary.path,
                None => {
                    eprintln!(
                        "fleet: no sessions in {}",
                        fleet::home::sessions_dir(&home).display()
                    );
                    return ExitCode::from(2);
                }
            },
        };
        if !path.exists() {
            eprintln!("fleet: no session log at {}", path.display());
            return ExitCode::from(2);
        }
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        return match fleet::tool_audit::audit(&path) {
            Ok(audit) => {
                print!("{}", fleet::tool_audit::render(&name, &audit));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        };
    }
    // The lifecycle verbs: disable, enable, sweep, coverage. Each
    // writes config through the one writer and leaves a row behind;
    // coverage reads the archive through the sem1 lane.
    match &options.action {
        cli::ToolsAction::Disable { name, force } => {
            return tools_disable(&home, name, *force);
        }
        cli::ToolsAction::Enable { name } => {
            return tools_enable(&home, name);
        }
        cli::ToolsAction::Sweep { apply } => {
            return tools_sweep(&home, *apply);
        }
        cli::ToolsAction::Coverage { sessions } => {
            return tools_coverage(&home, *sessions).await;
        }
        _ => {}
    }
    let sessions = match &options.action {
        cli::ToolsAction::Scan { sessions } | cli::ToolsAction::Advise { sessions } => *sessions,
        cli::ToolsAction::List | cli::ToolsAction::Audit { .. } => unreachable!("handled above"),
        cli::ToolsAction::Disable { .. }
        | cli::ToolsAction::Enable { .. }
        | cli::ToolsAction::Sweep { .. }
        | cli::ToolsAction::Coverage { .. } => unreachable!("handled above"),
    };
    let scan = fleet::tool_advisor::scan(&home, sessions);
    match &options.action {
        cli::ToolsAction::Advise { .. } => {
            let advice = fleet::tool_advisor::advise(&scan);
            print!("{}", fleet::tool_advisor::render_advice(&advice, &scan));
        }
        _ => print!("{}", fleet::tool_advisor::render_scan(&scan)),
    }
    ExitCode::SUCCESS
}

fn skills_command(options: &cli::SkillsOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let user_home = std::env::var("HOME").ok();
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
    if options.duplicates {
        // The collapse report is its own surface: deterministic,
        // report-only, and never a writer — the curation proposals
        // stay the only path that changes the library.
        let skills = load_skills(user_home.as_deref(), &home);
        let pairs = fleet::skills::near_duplicates(&skills, fleet::skills::DUPLICATE_SIMILARITY);
        print!("{}", fleet::skills::render_duplicates(skills.len(), &pairs));
        return ExitCode::SUCCESS;
    }
    if options.curate {
        let config = route::skills_config(&home);
        let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
        match fleet::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref()) {
            Some(canonical) => {
                let filed = fleet::skills::curate(
                    &canonical,
                    &home,
                    config.retire_after_sessions,
                    config.merge_similarity,
                );
                print!("{}", fleet::skills::render_curation(&filed));
            }
            None => {
                eprintln!(
                    "fleet: skills: no skills directory resolvable \u{2014} set FLEET_SKILLS_DIR or HOME"
                );
            }
        }
    }
    if options.proposed {
        print!(
            "{}",
            fleet::skills::render_proposals(&fleet::skills::load_proposals(&home))
        );
    } else if !options.curate {
        let config = route::skills_config(&home);
        let skills = load_skills(user_home.as_deref(), &home);
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
            fleet::calib::Stream::open(fleet::calib::resolve_root(&home, user_home.as_deref()))
        });
        let result = fleet::memory::consolidate(&root, &fleet::calib::today(), stream.as_ref());
        println!("{}", fleet::memory::render_consolidation(&result));
    }
    // The corpus is loaded after the cycle, so the cards show what the
    // pass left behind — the new summary in, the archived facts out.
    let memory = fleet::memory::Memory::load(&root);
    if let Some(instruction) = &options.recall {
        print!("{}", fleet::memory::render_recall(&memory, instruction));
    } else {
        print!(
            "{}",
            fleet::memory::render(&memory, options.shape.as_deref())
        );
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
    // The two list fields ride as arrays, split on commas the way
    // `--shape` is: the writer validates the count and the per-item
    // bound, so a malformed list is its named refusal rather than a
    // silent truncation here.
    for (flag, value) in [("phrases", &options.phrases), ("missing", &options.missing)] {
        let items: Vec<serde_json::Value> = value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| serde_json::Value::String(item.to_string()))
            .collect();
        if !items.is_empty() {
            object.insert(flag.to_string(), serde_json::Value::Array(items));
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
/// `fleet mcp health`: probe every configured server — spawn,
/// initialize, list its tools, report, tear down — and write each
/// outcome into the registry's `last_probe`, so the cards carry it until
/// the next probe. The exit code doubles as the fleet's: success only
/// when every probe came back ok. A server whose env name resolves to
/// nothing probes as failed with that reason — the named refusal, the
/// probe's way.
async fn mcp_health_command(home: &std::path::Path) -> ExitCode {
    let servers = match fleet::mcp_registry::read_servers(home) {
        Ok(servers) => servers,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::from(2);
        }
    };
    if servers.is_empty() {
        print!("{}", fleet::mcp_registry::render(&servers));
        return ExitCode::SUCCESS;
    }
    let dotenv: std::collections::BTreeMap<String, String> =
        home::read_dotenv_with_process_vault(home)
            .into_iter()
            .collect();
    let mut probed: Vec<(String, fleet::mcp_registry::ServerEntry)> = Vec::new();
    for (name, mut entry) in servers {
        match fleet::mcp_registry::probe_server(home, &name, &dotenv).await {
            Ok(probe) => {
                if let Err(error) = fleet::settings::record_server_probe(home, &name, probe.clone())
                {
                    eprintln!("fleet: {error}");
                    return ExitCode::from(2);
                }
                let word = if probe.ok { "ok" } else { "FAILED" };
                println!(
                    "fleet: mcp: probed `{name}` — {word}: {} v{} · {} tool{}",
                    probe.server_name,
                    probe.server_version,
                    probe.tools,
                    if probe.tools == 1 { "" } else { "s" },
                );
                entry.last_probe = Some(probe);
                probed.push((name, entry));
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                return ExitCode::from(2);
            }
        }
    }
    print!("{}", fleet::mcp_registry::render(&probed));
    if probed
        .iter()
        .all(|(_, entry)| entry.last_probe.as_ref().is_some_and(|probe| probe.ok))
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `fleet mcp`: the MCP server registry's surface. The bare command
/// lists the configured servers as cards; the mutations address one
/// server by name and go through the registry's own validation, so a
/// refused write never lands half of itself. The exit code doubles as
/// the list's health: success with zero servers too — an empty registry
/// is a state, not an error.
async fn mcp_command(options: &cli::McpOptions) -> ExitCode {
    use fleet::cli::McpAction;
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    match &options.action {
        McpAction::List => match fleet::mcp_registry::read_servers(&home) {
            Ok(servers) => {
                print!("{}", fleet::mcp_registry::render(&servers));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        },
        McpAction::Remove { name } => match fleet::settings::remove_server(&home, name) {
            Ok(removed) => {
                println!("fleet: mcp: removed `{removed}`");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        },
        McpAction::Scan => {
            let report = fleet::mcp_registry::scan(&home);
            print!("{}", fleet::mcp_registry::render_scan(&report));
            ExitCode::SUCCESS
        }
        McpAction::Import { name } => {
            // The scan is the import's source of truth: it runs now, the
            // row is found in its answer, and a name discovered under
            // more than one harness is refused until the ask says which.
            let report = fleet::mcp_registry::scan(&home);
            let today = fleet::boards::now_stamp();
            let today = &today[..today.len().min(10)];
            let mut found: Vec<&fleet::mcp_registry::ScanRow> =
                report.rows.iter().filter(|row| row.name == *name).collect();
            if let Some((harness, _)) = name.split_once(':') {
                found.retain(|row| row.harness == harness);
            }
            match found.as_slice() {
                [one] => match fleet::mcp_registry::import(&home, one, today) {
                    Ok(imported) => {
                        println!("fleet: mcp: imported `{imported}` from {}", one.harness);
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("fleet: {error}");
                        ExitCode::FAILURE
                    }
                },
                [] => {
                    eprintln!(
                        "fleet: `{name}` is not in the scan's answer; run `fleet mcp scan` to see what this machine carries"
                    );
                    ExitCode::FAILURE
                }
                many => {
                    let harnesses: Vec<&str> = many.iter().map(|row| row.harness).collect();
                    eprintln!(
                        "fleet: `{name}` is discovered under more than one harness ({}); import one with `fleet mcp import <harness>:{name}`",
                        harnesses.join(", ")
                    );
                    ExitCode::FAILURE
                }
            }
        }
        McpAction::Health => mcp_health_command(&home).await,
        McpAction::Enable { name, enabled } => {
            let word = if *enabled { "enabled" } else { "disabled" };
            match fleet::settings::set_server_enabled(&home, name, *enabled) {
                Ok(set) => {
                    println!("fleet: mcp: {word} `{set}` — the next run reads the registry");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

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
        CronAction::Add {
            name,
            schedule,
            task,
            tools,
            auto_approve,
        } => {
            match fleet::cron::Store::open(&home).and_then(|mut store| {
                let job = store.add(name, schedule, task, *auto_approve)?;
                if !tools.is_empty() {
                    // The allowlist rides the declaration: a job approved
                    // to reach a lane names the tool, and enabling it is
                    // the approval (KI-256).
                    return store.set_declaration(name, &job.task, tools);
                }
                Ok(job)
            }) {
                Ok(job) => {
                    let next = fleet::cron::Schedule::parse(&job.schedule)
                        .ok()
                        .and_then(|schedule| schedule.next_after(now))
                        .map(|minute| format!(", next fire {}", minute.stamp()))
                        .unwrap_or_default();
                    println!(
                        "fleet: cron: added `{}` \u{2014} {}{next}",
                        job.name, job.schedule
                    );
                    print!(
                        "{}",
                        fleet::cron::render(&home, std::slice::from_ref(&job), now)
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        CronAction::Enable { name } => mutate(&home, name, now, "enabled", true),
        CronAction::Disable { name } => mutate(&home, name, now, "disabled", false),
        CronAction::Remove { name } => {
            match fleet::cron::Store::open(&home).and_then(|mut store| {
                let job = store.remove(name)?;
                Ok(job)
            }) {
                Ok(job) => {
                    println!(
                        "fleet: cron: removed `{}` \u{2014} {}",
                        job.name, job.schedule
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        CronAction::Seed => match fleet::cron::seed_missing(&home) {
            Ok(report) => {
                for name in &report.added {
                    println!("fleet: cron: seeded `{name}`");
                }
                for name in &report.reconciled {
                    println!("fleet: cron: reconciled `{name}`'s task string to the manifest");
                }
                if report.added.is_empty() && report.reconciled.is_empty() {
                    println!("fleet: cron: the store matches the manifest");
                }
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
            println!(
                "fleet: cron: {verb} `{}` \u{2014} {}",
                job.name, job.schedule
            );
            print!(
                "{}",
                fleet::cron::render(home, std::slice::from_ref(&job), now)
            );
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
/// The vault's commands. Every one names its entry and never its value,
/// except `get`, which is the one deliberate exception — a secret has to
/// be able to reach a pipe.
fn vault_command(options: &cli::VaultOptions) -> ExitCode {
    match &options.action {
        VaultAction::List => {
            let Ok(home) = fleet_home() else {
                eprintln!("fleet: vault: no home is known");
                return ExitCode::FAILURE;
            };
            let path = home.join(fleet::vault::VAULT_FILE);
            let Ok(text) = std::fs::read_to_string(&path) else {
                println!(
                    "fleet: vault: no vault at {} — run `fleet vault init`",
                    path.display()
                );
                return ExitCode::SUCCESS;
            };
            let vault = match fleet::vault::Vault::from_json(&text) {
                Ok(vault) => vault,
                Err(reason) => {
                    eprintln!("fleet: vault: {reason}");
                    return ExitCode::FAILURE;
                }
            };
            if vault.is_empty() {
                println!("fleet: vault: the vault holds no entries");
                return ExitCode::SUCCESS;
            }
            // Names, never values — the one rule this surface holds.
            for name in vault.names() {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        VaultAction::Set { name, value } => {
            let Ok(home) = fleet_home() else {
                eprintln!("fleet: vault: no home is known");
                return ExitCode::FAILURE;
            };
            let Some(passphrase) = home::vault_passphrase() else {
                eprintln!(
                    "fleet: vault: FLEET_VAULT_PASSPHRASE is not set — the vault is keyed by it"
                );
                return ExitCode::FAILURE;
            };
            let path = home.join(fleet::vault::VAULT_FILE);
            let mut vault = match std::fs::read_to_string(&path) {
                Ok(text) => fleet::vault::Vault::from_json(&text)
                    .unwrap_or_else(|_| fleet::vault::Vault::init()),
                Err(_) => fleet::vault::Vault::init(),
            };
            vault.set(&passphrase, name, value);
            match std::fs::write(&path, vault.to_json()) {
                Ok(()) => {
                    println!("fleet: vault: sealed `{name}`");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: vault: could not write {}: {error}", path.display());
                    ExitCode::FAILURE
                }
            }
        }
        VaultAction::Get { name } => {
            let Ok(home) = fleet_home() else {
                eprintln!("fleet: vault: no home is known");
                return ExitCode::FAILURE;
            };
            let Some(passphrase) = home::vault_passphrase() else {
                eprintln!("fleet: vault: FLEET_VAULT_PASSPHRASE is not set");
                return ExitCode::FAILURE;
            };
            let path = home.join(fleet::vault::VAULT_FILE);
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => {
                    eprintln!("fleet: vault: no vault at {}", path.display());
                    return ExitCode::FAILURE;
                }
            };
            match fleet::vault::Vault::from_json(&text)
                .and_then(|vault| vault.get(&passphrase, name))
            {
                // The one command that prints a secret, because a secret
                // has to be able to reach a pipe.
                Ok(secret) => {
                    println!("{secret}");
                    ExitCode::SUCCESS
                }
                Err(reason) => {
                    eprintln!("fleet: vault: {reason}");
                    ExitCode::FAILURE
                }
            }
        }
        VaultAction::Remove { name } => {
            let Ok(home) = fleet_home() else {
                eprintln!("fleet: vault: no home is known");
                return ExitCode::FAILURE;
            };
            let path = home.join(fleet::vault::VAULT_FILE);
            let Ok(text) = std::fs::read_to_string(&path) else {
                eprintln!("fleet: vault: no vault at {}", path.display());
                return ExitCode::FAILURE;
            };
            let mut vault = match fleet::vault::Vault::from_json(&text) {
                Ok(vault) => vault,
                Err(reason) => {
                    eprintln!("fleet: vault: {reason}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(reason) = vault.remove(name) {
                eprintln!("fleet: vault: {reason}");
                return ExitCode::FAILURE;
            }
            if let Err(error) = std::fs::write(&path, vault.to_json()) {
                eprintln!("fleet: vault: could not write {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
            println!("fleet: vault: dropped `{name}`");
            ExitCode::SUCCESS
        }
        VaultAction::Init => {
            let Ok(home) = fleet_home() else {
                eprintln!("fleet: vault: no home is known");
                return ExitCode::FAILURE;
            };
            let Some(passphrase) = home::vault_passphrase() else {
                eprintln!(
                    "fleet: vault: FLEET_VAULT_PASSPHRASE is not set — it is the vault's key, so set it once and keep it"
                );
                return ExitCode::FAILURE;
            };
            let path = home.join(fleet::vault::VAULT_FILE);
            let mut vault = fleet::vault::Vault::init();
            // Migrate whatever the plaintext .env already holds: the
            // point of the vault is that the plaintext copy stops being
            // the one that answers.
            let dotenv = home::read_dotenv(&home);
            let mut migrated = 0;
            for (name, value) in &dotenv {
                if name.is_empty() || value.is_empty() {
                    continue;
                }
                vault.set(&passphrase, name, value);
                migrated += 1;
            }
            match std::fs::write(&path, vault.to_json()) {
                Ok(()) => {
                    println!(
                        "fleet: vault: created {} with {migrated} entr{} migrated from .env",
                        path.display(),
                        if migrated == 1 { "y" } else { "ies" }
                    );
                    println!(
                        "fleet: vault: the plaintext .env still holds its lines — remove the ones you have sealed"
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: vault: could not write {}: {error}", path.display());
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn improve_command(options: &cli::ImproveOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    match &options.action {
        ImproveAction::List => {
            print!(
                "{}",
                fleet::improve::render(&fleet::improve::load_all(&home))
            );
            ExitCode::SUCCESS
        }
        ImproveAction::Lessons => {
            print!(
                "{}",
                fleet::improve::render_lessons(&fleet::improve::load_lessons(&home))
            );
            ExitCode::SUCCESS
        }
        ImproveAction::Apply { name, approve } => {
            let repo = std::env::current_dir().unwrap_or_default();
            // KI-099: the gate runs against the working directory as the
            // repository root, so an apply from outside the intended repo
            // would gate and commit against the wrong tree with nothing
            // saying which. The adopted root is named before the verdict,
            // one line, beside the outcome that follows.
            println!(
                "fleet: improve: gating and committing against {}",
                repo.display()
            );
            let user_home = std::env::var("HOME").ok();
            let stream =
                fleet::calib::Stream::open(fleet::calib::resolve_root(&home, user_home.as_deref()));
            let gate = fleet::improve::Gate::local(&repo);
            // The eval-corpus gate probe for a source target: it runs
            // `fleet eval` as a subprocess so the gate grades the
            // freshly-built binary. The probe fails closed — it refuses
            // unless the current executable resolves under the gate
            // root's own target/ directory, so a source self-edit is
            // never waved through by an eval step that did not run, or
            // by a stale installed binary's green.
            let eval = Some(fleet::eval::subprocess_probe(&repo));
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            let canonical = match fleet::skills::resolve_skills_dir(
                skills_override.as_deref(),
                user_home.as_deref(),
            ) {
                Some(dir) => dir,
                None => {
                    eprintln!(
                        "fleet: improve: no skills directory resolvable \u{2014} set FLEET_SKILLS_DIR or HOME"
                    );
                    return ExitCode::FAILURE;
                }
            };
            let ctx = fleet::improve::Apply {
                home: &home,
                repo: &repo,
                gate: &gate,
                approved: *approve,
                stream: Some(&stream),
                eval,
                skills_dir: &canonical,
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
            let user_home = std::env::var("HOME").ok();
            let skills_override = std::env::var("FLEET_SKILLS_DIR").ok();
            let canonical =
                fleet::skills::resolve_skills_dir(skills_override.as_deref(), user_home.as_deref());
            match canonical
                .as_deref()
                .map(|dir| fleet::improve::undo(name, &home, &repo, dir))
            {
                Some(Ok(summary)) => {
                    println!("fleet: improve: {summary}");
                    ExitCode::SUCCESS
                }
                Some(Err(error)) => {
                    eprintln!("fleet: improve: {error}");
                    ExitCode::FAILURE
                }
                None => {
                    eprintln!(
                        "fleet: improve: no skills directory resolvable \u{2014} set FLEET_SKILLS_DIR or HOME"
                    );
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
    let day = options.day.clone().unwrap_or_else(fleet::calib::today);
    if options.calibration {
        // The calibration surface: the per-provider judged quality, with
        // the fit rule's readiness and the ride-back's gate state read
        // from the owner's config so the surface says what the gate is
        // rather than leaving it to be inferred.
        let gate = fleet::route::calibration_config(&home).informs_failover;
        print!("{}", fleet::calib::render_calibration(&root, &day, gate));
        return ExitCode::SUCCESS;
    }
    if options.delegation {
        // The delegation calibration surface: the per-(provider ×
        // brief-shape) judged quality — the delegation beat's own cells,
        // read from the day's `delegation_judged` rows, with the three
        // rates kept apart and the fit rule's readiness. The same gate,
        // read the same way.
        let gate = fleet::route::calibration_config(&home).informs_failover;
        print!("{}", fleet::delegate::render_calibration(&root, &day, gate));
        return ExitCode::SUCCESS;
    }
    print!("{}", fleet::calib::render(&root, &day));
    ExitCode::SUCCESS
}

/// `fleet roles`: the named roles a delegation can run under, as
/// cards — each role's description, what it narrows, and its file. The
/// `subagent` tool's `role` argument names one; an unknown name is a
/// named refusal, never a silent default.
fn roles_command() -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    print!("{}", fleet::roles::render(&fleet::roles::load_all(&home)));
    ExitCode::SUCCESS
}

/// `fleet convention <concept>`: what this project already calls a
/// concept, answered from the working tree's own vocabulary.
///
/// The exit code is success on every answer — an established term, an
/// established-as-written term, a named variant, and the honest miss
/// are all answers, and only a usage error (a missing concept, which
/// the parser refuses before this runs) exits nonzero. A lookup that
/// exits nonzero on a miss would train its caller to read the miss as
/// a failure, which is exactly the distrust the honest state exists to
/// avoid.
fn convention_command(options: &cli::ConventionOptions) -> ExitCode {
    let root = std::env::current_dir().unwrap_or_default();
    let answer =
        fleet::tools::convention_result(&root, &serde_json::json!({ "concept": options.concept }));
    print!("{answer}");
    ExitCode::SUCCESS
}

/// `fleet graph [--file <path>]`: the declared dependency graph as
/// cards, read from the home's `graph.json` or `--file`'s.
///
/// Every failure is named and exits nonzero: an absent graph file, an
/// unreadable one, and a malformed one are three different facts, and
/// rendering an empty graph for any of them would show the owner
/// "nothing is blocking anything" when the truth is that the surface
/// could not read the declaration. The cards are the plan's own bar —
/// status dots, readiness bars, named edges, the critical path, and
/// what is holding each node.
fn graph_command(options: &cli::GraphOptions) -> ExitCode {
    // A plan document reads the plan's own runtime file: the plan is the
    // contract, so its graph is the one beside it — and that is what makes
    // a completed plan's graph traceable after the fact rather than
    // overwritten by the next run's.
    if let Some(plan) = &options.plan {
        let path = std::path::PathBuf::from(plan);
        if options.file.is_some() {
            eprintln!("fleet: `--file` and a plan are two graphs; name one");
            return ExitCode::from(2);
        }
        return match fleet::plan_graph::PlanGraph::load(&path) {
            Ok(plan_graph) => {
                print!("{}", fleet::graph::render(&plan_graph.graph));
                // The criteria roll-up rides the same output: the plan's
                // own acceptance table, read off the graph.
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                print!("{}", render_criteria(&plan_graph, &text));
                ExitCode::SUCCESS
            }
            Err(reason) => {
                eprintln!("fleet: {reason}");
                ExitCode::from(2)
            }
        };
    }
    let path = match &options.file {
        Some(file) => std::path::PathBuf::from(file),
        None => {
            let Some(home) = fleet_home().ok() else {
                eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
                return ExitCode::from(2);
            };
            fleet::graph::path_in(&home)
        }
    };
    match fleet::graph::Graph::load(&path) {
        Ok(graph) => {
            print!("{}", fleet::graph::render(&graph));
            ExitCode::SUCCESS
        }
        Err(reason) => {
            eprintln!("fleet: {reason}");
            ExitCode::from(2)
        }
    }
}

/// The plan's acceptance criteria as the graph reports them: each
/// criterion, the nodes serving it, and whether every one has verified.
///
/// This is the join that makes the plan's own table the progress surface.
/// The plan already owns "is X done" — `docs/plans/README.md` says so —
/// and this reads it off the graph rather than off a human's memory.
fn render_criteria(plan_graph: &fleet::plan_graph::PlanGraph, plan_text: &str) -> String {
    let mut out = String::new();
    let criteria = plan_graph.criteria();
    if criteria.is_empty() {
        return out;
    }
    out.push_str("\nAcceptance criteria, off the graph\n\n");
    for criterion in &criteria {
        let marker = if criterion.met { "met" } else { "unmet" };
        out.push_str(&format!(
            "  {marker:<5} {name}  \u{b7} served by {nodes}\n",
            name = criterion.name,
            nodes = criterion.nodes.join(", ")
        ));
    }
    // A criterion the plan declares but no node serves is named as
    // unserved rather than left to read as unmet: a criterion nobody is
    // working on is a plan that says one thing and executes another. The
    // list comes from the document's own convention, so a criterion
    // written some other way is simply not checked.
    let declared = fleet::plan_graph::criteria_in(plan_text);
    let unserved = plan_graph.unserved(&declared);
    if !unserved.is_empty() {
        out.push_str(&format!(
            "\n  unserved {}  \u{b7} no node declares it\n",
            unserved.join(", ")
        ));
    }
    out
}

/// `fleet plan <plan.md | slug>`: the plan's own surface.
///
/// It renders the graph the plan generated and the acceptance criteria
/// read off it — the plan already owns "is X done" (`docs/plans/README.md`
/// says so), and this reads it off the graph rather than off a human's
/// memory.
///
/// It also generates the graph when an approved plan has none yet, or
/// when the plan's declaration has drifted from the one the runtime file
/// was built from. Drift is the normal case — the repo's rule is "update
/// the plan's acceptance criteria first, then build" — so regenerating
/// here is self-healing rather than surprising, and every regeneration
/// says it happened.
fn plan_command(options: &cli::PlanOptions) -> ExitCode {
    let path = resolve_plan(&options.plan);
    let meta = match fleet::plan_graph::read_meta(&path) {
        Ok(meta) => meta,
        Err(reason) => {
            eprintln!("fleet: {reason}");
            return ExitCode::from(2);
        }
    };
    // Only an approved plan executes, so only an approved plan has a graph
    // to read or generate.
    if !meta.status.generates_a_graph() {
        eprintln!(
            "fleet: {}: a `{}` plan does not generate a graph — only an approved one does",
            path.display(),
            meta.status.as_str()
        );
        return ExitCode::from(2);
    }
    let mut plan_graph = match fleet::plan_graph::PlanGraph::load(&path) {
        Ok(loaded) => loaded,
        Err(_) => match fleet::plan_graph::PlanGraph::generate(&path) {
            Ok(generated) => generated,
            Err(reason) => {
                eprintln!("fleet: {reason}");
                return ExitCode::from(2);
            }
        },
    };
    // Drift: the declaration changed since the runtime file was built, so
    // regenerate rather than read a graph that mismaps nodes onto
    // criteria that moved.
    if plan_graph.drift(&path).is_some() {
        match fleet::plan_graph::PlanGraph::generate(&path) {
            Ok(fresh) => plan_graph = fresh,
            Err(reason) => {
                eprintln!("fleet: {reason}");
                return ExitCode::from(2);
            }
        }
    }
    // P6 C5: demote a node whose work did not integrate, before the
    // render, so the surface shows the graph as it now stands.
    if let Some((node, reason)) = &options.demote {
        match plan_graph.graph.demote(node, reason) {
            Ok(()) => {
                if let Err(reason) = plan_graph.save(&path) {
                    eprintln!("fleet: {reason}");
                    return ExitCode::from(2);
                }
                eprintln!("fleet: `{node}` demoted: {reason}");
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // P7: the registry of other plans, so this plan's cross-plan edges
    // resolve rather than reading as typos, a cross-plan cycle is refused
    // rather than deadlocking silently, and a dependency that can never
    // clear is named.
    let plans_dir = path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    let registry = fleet::plan_graph::PlanRegistry::scan_for(&plans_dir, &meta.slug);

    if let Some(cycle) = fleet::plan_graph::cross_plan_cycle(&plan_graph.graph, &registry) {
        eprintln!(
            "fleet: {}: the cross-plan edges close a cycle: {}",
            path.display(),
            cycle.join(" \u{2192} ")
        );
        return ExitCode::from(2);
    }
    let unresolvable = fleet::plan_graph::unresolvable_edges(&plan_graph.graph, &registry);
    let mut cross_plan: Vec<String> = Vec::new();
    for node in plan_graph.graph.nodes() {
        if let Some(unlock) = plan_graph.graph.unlock_with(&node.id, Some(&registry)) {
            for named in unlock.cross_plan {
                cross_plan.push(format!("`{}` \u{b7} {named}", node.id));
            }
        }
    }

    let text = std::fs::read_to_string(&path).unwrap_or_default();
    print!(
        "{}",
        fleet::graph::render_with(&plan_graph.graph, Some(&registry))
    );
    print!("{}", render_criteria(&plan_graph, &text));
    if !cross_plan.is_empty() {
        println!("\nCross-plan dependencies\n");
        for line in &cross_plan {
            println!("  {line}");
        }
    }
    if !unresolvable.is_empty() {
        println!("\nUnresolvable cross-plan dependencies\n");
        for line in &unresolvable {
            println!("  {line}");
        }
    }
    ExitCode::SUCCESS
}

/// Resolve a plan argument: a path that exists is used as given, and
/// anything else is resolved as a slug under `docs/plans/` — the plan
/// format's own naming, so `fleet plan gui-revamp` finds the document.
fn resolve_plan(arg: &str) -> std::path::PathBuf {
    let direct = std::path::PathBuf::from(arg);
    if direct.is_file() {
        return direct;
    }
    let root = std::env::current_dir().unwrap_or_default();
    let plans = root.join("docs").join("plans");
    if let Ok(entries) = std::fs::read_dir(&plans) {
        let mut matches: Vec<std::path::PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("md")
                    && path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| stem.ends_with(&format!("-{arg}")))
            })
            .collect();
        matches.sort();
        if let Some(found) = matches.into_iter().next() {
            return found;
        }
    }
    direct
}

/// `fleet judge`: the day's route rows, judged against their outcome
/// evidence through the vendored sys1 lane — or the owner's disposition
/// on one judged row.
///
/// The batch is batch-only by construction: it spends at most one
/// bounded lane ask per unjudged route row, lands a redacted
/// `route_judged` row per ask, and refuses by name on every failure. The
/// exit code is success when the batch answered (including a dry run,
/// which answers by rehearsing, and a batch whose every route row was
/// already judged, which answers by confirming) and failure when it did
/// not: a batch that stood in front of unjudged route rows and judged
/// none of them — the lane unavailable, the budget spent, the day empty
/// — exits nonzero, because the consuming-lanes rule is that such a leg
/// is NO ANSWER and a silent success would ship it as one. A disposition
/// exits 0 when it landed and 1 when its key named no judged row.
async fn judge_command(options: &cli::JudgeOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let user_home = std::env::var("HOME").ok();
    let root = fleet::calib::resolve_root(&home, user_home.as_deref());
    let day = options.day.clone().unwrap_or_else(fleet::calib::today);
    if let Some(disposition) = &options.disposition {
        return match fleet::judge::disposition(&root, &day, &disposition.key, &disposition.word) {
            Ok(row) => {
                print!("{}", fleet::judge::disposition_card(&row, &day));
                ExitCode::SUCCESS
            }
            Err(reason) => {
                print!("{}", fleet::judge::disposition_refusal_card(&day, &reason));
                ExitCode::FAILURE
            }
        };
    }
    let outcome = fleet::judge::judge(fleet::judge::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: options.dry_run,
        lane: fleet::judge::LaneSource::Vendored,
    })
    .await;
    print!("{}", outcome.card);
    if outcome.batch_answered() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `fleet delegate`: the day's delegation rows, judged against their
/// outcome evidence — the child's own session log — through the
/// vendored sys1 lane, or the owner's disposition on one judged
/// delegation row.
///
/// The ported discipline verbatim, the plan's D2: the batch spends at
/// most one bounded ask per unjudged delegation row under a named daily
/// budget, lands a redacted `delegation_judged` row per ask, and refuses
/// by name on every failure. The exit code is the plan's contractual
/// rule: a batch that obtained no judgment — every row refused, an empty
/// day, an over-budget day — exits 1; a dry run, a judging batch, and an
/// all-already-judged batch exit 0, so a twice-daily cron is not a false
/// alarm.
async fn delegate_command(options: &cli::DelegateOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let user_home = std::env::var("HOME").ok();
    let root = fleet::calib::resolve_root(&home, user_home.as_deref());
    let day = options.day.clone().unwrap_or_else(fleet::calib::today);
    if let Some(disposition) = &options.disposition {
        return match fleet::delegate::disposition(&root, &day, &disposition.key, &disposition.word)
        {
            Ok(row) => {
                print!("{}", fleet::delegate::disposition_card(&row, &day));
                ExitCode::SUCCESS
            }
            Err(reason) => {
                print!(
                    "{}",
                    fleet::delegate::disposition_refusal_card(&day, &reason)
                );
                ExitCode::FAILURE
            }
        };
    }
    // The owner's control over a running session: the surface writes
    // the file beside the session's log, and the turn loop reads it
    // before its next model call. A control naming no session is a
    // named refusal — the control rides beside a log that exists.
    if let Some(control) = &options.control {
        let (session, steer) = match control {
            cli::DelegateControl::Steer { session, text } => (session.clone(), Some(text.clone())),
            cli::DelegateControl::Stop { session } => (session.clone(), None),
        };
        return match fleet::delegate::write_control(&home, &session, steer.as_deref()) {
            Ok(path) => {
                print!(
                    "{}",
                    fleet::delegate::control_card(&session, steer.as_deref(), &path)
                );
                ExitCode::SUCCESS
            }
            Err(reason) => {
                print!("{}", fleet::delegate::control_refusal_card(&reason));
                ExitCode::FAILURE
            }
        };
    }
    let outcome = fleet::delegate::judge(fleet::delegate::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: options.dry_run,
        lane: fleet::delegate::LaneSource::Vendored,
    })
    .await;
    print!("{}", outcome.card);
    // The ride-back made visible: what the day's calibration would
    // move for a brief shape, read pure from the stream — a judgment
    // spends, a calibration does not — beside the graded rows, because
    // the cells were built for a consumer and this is it.
    let gate = fleet::route::calibration_config(&home).informs_failover;
    let candidate_ids: Vec<String> = fleet::route::load_roster(&home)
        .map(|roster| {
            roster
                .candidates
                .iter()
                .map(|candidate| candidate.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let candidates: Vec<&str> = candidate_ids.iter().map(String::as_str).collect();
    print!(
        "{}",
        fleet::delegate::render_ride_back_day(&root, &day, &candidates, gate)
    );
    if outcome.batch_answered() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Build the routed agent and run `task`, printing the final answer to
/// stdout. The route line and the decision row are the caller's: `run`
/// prints once per task, `chat` once per session.
///
/// The parameters are the CLI's own flags carried one-to-one into the
/// agent's options — grouping them would rename, not reduce, what the
/// two commands hand the run.
#[allow(clippy::too_many_arguments)]
async fn drive(
    prepared: &route::Prepared,
    model: String,
    max_turns: usize,
    yes: bool,
    compact_now: bool,
    checkpoints: Option<std::path::PathBuf>,
    plan_mode: bool,
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
    // KI-249's measurement: the load is the embed path's whole cost on a
    // run, so its wall clock and the fact count it produced land as one
    // redacted row — the tail these rows accumulate is what the embed
    // bounds should be set from, not a guess.
    let load_started = std::time::Instant::now();
    let memory = tokio::task::spawn_blocking(|| {
        let root = fleet::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
        std::sync::Arc::new(fleet::memory::Memory::load(&root))
    })
    .await
    .ok();
    if let Some(memory) = memory.as_ref() {
        let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(
            home,
            std::env::var("HOME").ok().as_deref(),
        ));
        fleet::calib::record_embed_timing(
            &stream,
            "corpus-load",
            load_started.elapsed().as_millis(),
            Some(memory.fact_count()),
        );
        // KI-249's other half: one row per embed the load's index build
        // made \u{2014} `EMBED_TIMEOUT` bounds one of these calls, so the
        // bound's size is the measurement's to set. An empty build adds
        // nothing.
        for ms in &memory.build_embeds_ms {
            fleet::calib::record_embed_timing(&stream, "embed", *ms, None);
        }
    }
    // The registry's enabled servers seed the run's lane specs, so the
    // run pre-joins them before its first request (D2). A name the env
    // chain cannot resolve is a named refusal before anything spawns.
    let dotenv: std::collections::BTreeMap<String, String> =
        home::read_dotenv_with_process_vault(home)
            .into_iter()
            .collect();
    let mut seed = AgentOptions::default();
    let lane_specs = match seed.seed_user_servers(home, &dotenv) {
        Ok(absent) => {
            // The named absences (the 2026-10-26 plan): a server whose
            // env resolves to nothing costs the run that server's tools,
            // named here at startup the way the skills' skipped lines
            // are — never the run itself.
            for (server, reason) in absent {
                eprintln!(
                    "fleet: server `{server}`: {reason} — its tools are not declared this run"
                );
            }
            seed.lane_specs
        }
        Err(error) => return Err(AgentError::Registry(error)),
    };
    let options = AgentOptions {
        goal: None,
        model,
        max_turns,
        identity: crate::home::read_soul(home),
        ui_tx: None,
        lane_specs,
        calib: Some(prepared.calib.clone()),
        skills,
        // The parent's own run carries no role: a role is a
        // delegation's specialization, and the overlay rides a child's
        // prompt, never the owner's session.
        role_overlay: None,
        memory,
        scope: None,
        // The self-maintenance tool rides every surface that knows its
        // home: a session can file a proposal, and only the owner's
        // `fleet improve apply` runs one.
        improve_home: Some(home.to_path_buf()),
        route_home: None,
        gui_prompt: None,
        compact_now,
        checkpoints,
        // The headless CLI has no per-session folder: it works in the
        // directory the owner launched it from.
        file_access: FileAccess::Unrestricted,
        plan_mode,
    };
    // Every non-interactive mode rides the Gate, never `ApproveAll`:
    // the always-ask class (computer use) sits ABOVE the modes, and an
    // approver without that check would let `--yes` waive the owner's
    // ruling on the owner's behalf. Only `Ask` prompts, because only
    // `Ask` has a user to ask.
    let mode = fleet::approvals::resolve(
        yes.then_some(true),
        fleet::approvals::config_word(home).as_deref(),
    );
    let approver: Box<dyn Approver> = match mode {
        // The default: every call asks. The owner's house rule stands.
        fleet::approvals::Mode::Ask => Box::new(ConsoleApprover::new()),
        // Everything runs, the destructive class included — the
        // explicit `--yes` or a config that asked for it. The
        // always-ask class still asks: that is the whole point of it
        // sitting above the modes.
        fleet::approvals::Mode::Yes => {
            Box::new(fleet::approvals::Gate::new(fleet::approvals::Mode::Yes))
        }
        // Everything but the destructive class runs; a call in that
        // class still asks, and its denial names the rule that kept
        // it there.
        fleet::approvals::Mode::YesGuarded => Box::new(fleet::approvals::Gate::new(
            fleet::approvals::Mode::YesGuarded,
        )),
    };
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
        None => Session::create(home, model)
            .map_err(|error| format!("could not create the session: {error}")),
        Some(id) => {
            let path = session::session_path(home, id);
            if !path.exists() {
                return Err(format!(
                    "no session `{id}` under {}",
                    home::sessions_dir(home).display()
                ));
            }
            Session::open(&path).map_err(|error| format!("could not open session `{id}`: {error}"))
        }
    }
}

/// `fleet compact [<session>]`: fold one session's stored context on
/// demand. The fold is the same summarize-and-continue the budget fires
/// on its own — one request through the run's own routed client, no
/// separate provider and no separate model — so a compacted session
/// resumes as the compacted conversation it is rather than as the raw
/// records.
///
/// A session already under the budget is a named no-op, not a second
/// summary: the honest answer to "compact this" when there is nothing
/// to fold is that there is nothing to fold. The log stays append-only
/// either way — the fold is a `compaction` record replay honors, never
/// a rewrite of what came before.
async fn compact_command(session_id: Option<&str>, force: bool) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    let sessions_dir = fleet::home::sessions_dir(&home);
    let summaries = match session::list(&home) {
        Ok(summaries) => summaries,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    if summaries.is_empty() {
        eprintln!("fleet: no sessions yet — run `fleet run \"<task>\"` to start one");
        return ExitCode::FAILURE;
    }
    // A bare command takes the most recent session — the list is oldest
    // first, so that is its last element — and a positional names one,
    // with an unknown name refused rather than silently falling back to
    // the newest.
    let summary = match session_id {
        Some(id) => match summaries.iter().find(|summary| summary.id == id) {
            Some(summary) => summary,
            None => {
                eprintln!("fleet: no session named `{id}`; `fleet sessions` lists them");
                return ExitCode::FAILURE;
            }
        },
        None => summaries.last().expect("the list is non-empty"),
    };
    let path = sessions_dir.join(format!("{}.jsonl", summary.id));
    let records = match session::replay(&path) {
        Ok(records) => records,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    let conversation = session::conversation(&records);
    let measured = fleet::compaction::estimate_tokens(&conversation.messages);
    let budget = fleet::compaction::COMPACTION_TOKEN_BUDGET;
    // Already compacted, or already under the budget: nothing to fold.
    if summary.compacted {
        println!(
            "`{}` is already compacted \u{2014} the log carries a fold, and a second one \
             would summarize a summary",
            summary.id
        );
        return ExitCode::SUCCESS;
    }
    if measured < budget && !force {
        println!(
            "`{}` is at ~{measured} tokens against the {budget} budget \u{2014} nothing to \
             fold, so nothing was folded (--force folds it anyway)",
            summary.id
        );
        return ExitCode::SUCCESS;
    }
    let tail_start = fleet::compaction::tail_start(&conversation.messages);
    if tail_start == 0 {
        println!(
            "`{}` is all verbatim tail \u{2014} there is no earlier region to replace",
            summary.id
        );
        return ExitCode::SUCCESS;
    }

    // The routed client, the same path every other request takes. The
    // compaction request is judged like any other: it is a real call
    // with a real cost, and pretending it is free would put a lie in
    // the decision row.
    let passphrase = home::vault_passphrase();
    let dotenv = fleet::home::read_dotenv_with_vault(&home, passphrase.as_deref());
    let prepared =
        match fleet::route::prepare(&home, &dotenv, "compact a session", &HashMap::new()).await {
            Ok(prepared) => prepared,
            Err(error) => {
                eprintln!("fleet: the route could not be prepared: {error}");
                return ExitCode::FAILURE;
            }
        };
    let client = fleet::api::Client::routed(prepared.router.clone());
    let first_turn = conversation.turns.first().copied().unwrap_or(1);
    let last_turn = conversation.turns[..tail_start]
        .iter()
        .zip(&conversation.messages[..tail_start])
        .filter(|(_, message)| message.role == fleet::api::Role::Assistant)
        .map(|(turn, _)| *turn)
        .next_back()
        .unwrap_or(first_turn);
    let turns = fleet::compaction::turns_in_range(first_turn, last_turn);

    let mut request_messages = Vec::with_capacity(tail_start + 2);
    request_messages.push(fleet::api::Message::text(
        fleet::api::Role::System,
        fleet::agent::SYSTEM_PROMPT,
    ));
    request_messages.extend(conversation.messages[..tail_start].iter().cloned());
    request_messages.push(fleet::api::Message::text(
        fleet::api::Role::User,
        fleet::compaction::COMPACTION_INSTRUCTION,
    ));
    let request = fleet::api::ChatRequest {
        model: &prepared.decision.model,
        messages: &request_messages,
        tools: Vec::new(),
    };
    let summary_text = match client.chat_stream(&request, |_| {}).await {
        Ok(response) => response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default(),
        Err(error) => {
            // The fold is auxiliary: its failure costs the session its
            // checkpoint and nothing else. Named, never fatal.
            eprintln!("fleet: the compaction call failed: {error}; the log is unchanged");
            return ExitCode::FAILURE;
        }
    };
    if summary_text.trim().is_empty() {
        eprintln!("fleet: the compaction call returned nothing; the log is unchanged");
        return ExitCode::FAILURE;
    }
    let tokens_after = fleet::compaction::estimate_tokens(&[fleet::api::Message::text(
        fleet::api::Role::User,
        fleet::compaction::summary_frame(first_turn, last_turn, &summary_text),
    )]);

    // The record, appended: the log stays append-only, and replay folds
    // at the record exactly as it folds at a live run's.
    let mut log = match session::Session::open(&path) {
        Ok(log) => log,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = log.record(&session::Record::Compaction {
        summary: summary_text,
        first_turn,
        last_turn,
        tokens_before: measured,
        tokens_after,
    }) {
        eprintln!("fleet: {error}");
        return ExitCode::FAILURE;
    }
    // The row, so the fold's arithmetic is auditable rather than
    // assumed: what it replaced, what it cost, and what it saved.
    let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(
        &home,
        std::env::var("HOME").ok().as_deref(),
    ));
    stream.record(fleet::calib::Row::Consolidate(
        fleet::calib::ConsolidateRow {
            ts: fleet::calib::stamp(),
            action: "compact".to_string(),
            fact: summary.id.clone(),
            reason: format!(
                "{turns} turn{} \u{2192} 1 summary, ~{measured} \u{2192} ~{tokens_after} tokens",
                if turns == 1 { "" } else { "s" }
            ),
        },
    ));
    println!(
        "`{}` compacted \u{2014} {turns} turn{} \u{2192} 1 summary (~{measured} \u{2192} \
         ~{tokens_after} tokens)",
        summary.id,
        if turns == 1 { "" } else { "s" }
    );
    ExitCode::SUCCESS
}

/// `fleet sessions` — three surfaces over one archive. Bare, it lists
/// the sessions as cards. `--recall "<instruction>"` ranks the archive
/// through the six-channel ranker, where a session IS a trajectory and
/// the direction channel reads the record itself. A positional opens
/// one session's log as cards.
fn sessions_command(recall: Option<&str>, view: Option<&str>) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    if let Some(instruction) = recall {
        // The ranker, pointed at the archive. The embedder resolves
        // the same way the run's does, and an absent embedder is the
        // keyword-only degradation — never a blocked surface.
        let embedder = fleet::memory::probe(&fleet::memory::embedding_endpoint()).embedder();
        let sessions_dir = fleet::home::sessions_dir(&home);
        let mut index = fleet::session_recall::SessionIndex::load(&sessions_dir);
        let hits = index.recall(&sessions_dir, embedder, instruction);
        // The archive's own feedback loop: a recalled session bumps
        // its count store — never the append-only log — and lands one
        // `recall_hit` row naming the session id, the same row shape
        // the facts corpus lands, through the stream's own path. The
        // stream's drop at the return drains the queue, the pattern
        // the compact command already runs.
        let stream = fleet::calib::Stream::open(fleet::calib::resolve_root(
            &home,
            std::env::var("HOME").ok().as_deref(),
        ));
        for hit in &hits {
            fleet::session_recall::record_hit(
                &sessions_dir,
                &hit.id,
                &hit.outcome,
                hit.score,
                Some(&stream),
            );
        }
        print!("{}", fleet::session_recall::render_hits(&hits, instruction));
        return ExitCode::SUCCESS;
    }
    let summaries = match session::list(&home) {
        Ok(summaries) => summaries,
        Err(error) => {
            eprintln!("fleet: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(id) = view {
        let Some(summary) = summaries.iter().find(|summary| summary.id == id) else {
            eprintln!("fleet: no session named `{id}`; `fleet sessions` lists them");
            return ExitCode::FAILURE;
        };
        let path = fleet::home::sessions_dir(&home).join(format!("{id}.jsonl"));
        match session::replay(&path) {
            Ok(records) => {
                print!(
                    "{}",
                    fleet::session_recall::render_detail(&summary.id, &summary.created, &records)
                );
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    print!("{}", fleet::session_recall::render_list(&summaries));
    ExitCode::SUCCESS
}

/// `fleet settings`: every addressable knob as cards, or one written
/// through the same writer the TUI pane calls. A write that the file's
/// own validation refuses names its reason and changes nothing — the
/// CLI is not a way around the rules the loader enforces.
fn settings_command(set: Option<(&str, &str)>) -> ExitCode {
    let home = match fleet_home() {
        Ok(home) => home,
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::from(2);
        }
    };
    if let Some((key, value)) = set {
        // The list-shaped surfaces arrive as a tagged key: the writer's
        // scalar path handles the plain ones, and the two list actions
        // are dispatched here rather than inventing a second command
        // tree the TUI would have to mirror.
        if let Some(entry) = key.strip_prefix("provider.add:") {
            return match fleet::settings::set_provider(&home, entry) {
                Ok(id) => {
                    println!("provider `{id}` written \u{2014} `fleet providers` shows the roster");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        if let Some(id) = key.strip_prefix("provider.remove:") {
            let force = value == "true";
            return match fleet::settings::remove_provider(&home, id, force) {
                Ok(()) => {
                    println!("provider `{id}`'s override removed \u{2014} it reverts to the compiled-in default");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        if let Some(name) = key.strip_prefix("workload.set:") {
            let candidates: Vec<String> = value
                .split(',')
                .map(|candidate| candidate.trim().to_string())
                .filter(|candidate| !candidate.is_empty())
                .collect();
            return match fleet::settings::set_workload(&home, name, &candidates) {
                Ok(()) => {
                    println!(
                        "workload `{name}` written \u{2014} {} candidate(s)",
                        candidates.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        if let Some(name) = key.strip_prefix("workload.remove:") {
            let force = value == "true";
            return match fleet::settings::remove_workload(&home, name, force) {
                Ok(()) => {
                    println!("workload `{name}`'s preference removed \u{2014} it reverts to the compiled-in table");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("fleet: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        return match fleet::settings::set(&home, key, value) {
            Ok(()) => {
                println!("`{key}` written \u{2014} `fleet settings` shows the resolved value");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fleet: {error}");
                ExitCode::FAILURE
            }
        };
    }
    print!("{}", fleet::settings::render(&home));
    ExitCode::SUCCESS
}

/// `fleet lanes`: probe every vendored lane through the MCP client and
/// render the health cards. No credentials, no home, no network beyond
/// the lane children themselves — the lanes are local services. The exit
/// code is the doctor's convention: success only when every lane is ok.
async fn lanes_command() -> ExitCode {
    let health = fleet::lanes::health_all().await;
    print!("{}", fleet::lanes::render(&health));
    if health
        .iter()
        .all(|lane| lane.state == fleet::lanes::LaneState::Ok)
    {
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
    if resolved
        .iter()
        .any(|entry| route::provider_state(entry) == route::ProviderState::Ready)
    {
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

    // The delegated-workspaces layer: the one opt-in knob beside the
    // CLI it drives. Off is the default and reads ok; a knob turned on
    // over a machine that cannot answer is a broken layer, named here
    // rather than at the first delegation that tries to mount.
    let layerfs_check = doctor::evaluate_layerfs(
        fleet::layerfs_tools::gate(),
        &fleet::layerfs_tools::project_name(),
    );
    failed |= !layerfs_check.ok;
    println!("{}", doctor::render(&layerfs_check));

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
    // KI-134: the roster check's failure sets the gate's verdict. An
    // invalid `<FLEET_HOME>/config.json` printed a FAIL line and the
    // process still exited 0 — a broken roster is a broken layer, and
    // the exit code is what a caller reads.
    failed |= !roster_check.ok;
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
    // the deep MCP health probe. This line names which of the five is
    // unavailable and why, the way the plan's layer list asks.
    for lane in fleet::lanes::LANES.iter() {
        let resolved = match fleet::lanes::resolve_home(lane) {
            fleet::lanes::LaneHome::Resolved { home, .. } => Ok(home),
            fleet::lanes::LaneHome::Unavailable { reason } => Err(reason),
        };
        let check = doctor::evaluate_mcp_core(
            lane.name,
            resolved.as_deref().map_err(|reason| reason.clone()),
        );
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

    // The web-leg line: whether the live-source tool's credential is
    // present, and whether blend1 — the lane the leg answers through —
    // resolves. Informational, never a check: an unkeyed web leg is the
    // lane's own designed fail-open rather than a broken machine, so
    // the line names the state and never touches the exit code. The
    // key rides as a presence — its text is never read — and the
    // deeper truth, the lane's own doctor report, is what `fleet
    // lanes` renders.
    let tinyfish_key = setting("TINYFISH_API_KEY", &dotenv);
    let blend1_resolves = fleet::lanes::LANES
        .iter()
        .find(|lane| lane.name == "blend1")
        .is_some_and(|lane| {
            matches!(
                fleet::lanes::resolve_home(lane),
                fleet::lanes::LaneHome::Resolved { .. }
            )
        });
    let web_check = doctor::evaluate_web_leg(tinyfish_key.as_deref(), blend1_resolves);
    println!("{}", doctor::render(&web_check));

    // The approval mode, named where a run that stopped asking can be
    // understood without inferring it. The doctor carries no `--yes`,
    // so this is the mode a plain `fleet run` resolves to — the config
    // word's own answer, with the flag absent.
    let mode = fleet::approvals::resolve(
        None,
        fleet::approvals::config_word(home.as_deref().unwrap_or(std::path::Path::new(".")))
            .as_deref(),
    );
    println!("{}", doctor::render_approvals(mode));

    // The desktop's line: whether the cua driver resolves. Read-only
    // and never fatal — a machine without one simply has no desktop
    // tool, and the line says so rather than letting the declaration
    // imply a capability that is not there.
    match fleet::agent::cua_driver_spec() {
        Ok(spec) => println!(
            "{}",
            doctor::render_desktop(true, &format!("the cua driver answers at {}", spec.program))
        ),
        Err(error) => println!("{}", doctor::render_desktop(false, &error)),
    }

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
    // KI-161: the repo-side cron manifest, seeded into a home that lacks
    // its jobs. An existing job's schedule and enabled flag are never
    // rewritten, so an owner's edits win and a re-run is a no-op there;
    // a seeded job's task string reconciles to the manifest's current
    // wording (KI-277), and a fresh home gets the schedule the owner
    // built by hand, declared where it cannot be lost.
    match fleet::cron::seed_missing(&home) {
        Ok(report) if !report.added.is_empty() || !report.reconciled.is_empty() => {
            println!(
                "fleet: install: cron: seeded {} manifest job(s), reconciled {} task string(s)",
                report.added.len(),
                report.reconciled.len()
            );
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("fleet: install: cron: {error}");
            return ExitCode::FAILURE;
        }
    }
    print!(
        "{}",
        fleet::install::render_head(&home, &layout, (config_step, &config_path), options.lanes)
    );
    // The lanes bootstrap, with each tool's own lines streaming to the
    // operator as they arrive — a multi-minute `uv pip install` cannot
    // look frozen, and a tool that passes its bound is killed rather
    // than hanging the install.
    let lanes = if options.lanes {
        let mut results: Vec<fleet::install::LaneResult> = Vec::new();
        let mut progress = std::io::stdout();
        for lane in fleet::install::LANES.iter() {
            let outcome = fleet::install::bootstrap_lane(&home, lane, None, &mut progress);
            results.push((lane, outcome));
        }
        Some(results)
    } else {
        None
    };
    print!("{}", fleet::install::render_tail(&home, lanes.as_deref()));
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
    // improve gate's cwd convention, KI-099's family). KI-106: the
    // adopted path is NAMED in the diagnosis output, so a `--diagnose`
    // run from the wrong directory says which ledger it read rather
    // than leaving the operator to infer it.
    let named = ledger.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join("docs")
            .join("known-issues.md")
    });
    if ledger.is_none() {
        println!("fleet: doctor: the ledger read is {}", named.display());
    }
    print!("{}", doctor::render_diagnosis(&diagnosis, &named));
    match ledger {
        Some(_) => {
            let entry = doctor::LedgerEntry {
                severity: "defect",
                diagnosis,
            };
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

/// `fleet explain`: one verdict, explained. Eval-only and batch-only by
/// construction — the command spends at most one bounded lane ask, lands
/// a redacted row beside the verdict it explains, and writes the
/// recoverable prose under the home. The exit code is success when the
/// beat answered (including a dry run, which answers by rehearsing) and
/// failure when it refused: an unknown selector, a drifted artifact, an
/// over-budget day, or a lane that cannot answer. A refusal is a named
/// outcome on the card, never a silent absence and never a fabricated
/// explanation.
async fn explain_command(options: &cli::ExplainOptions) -> ExitCode {
    let Some(home) = fleet_home().ok() else {
        eprintln!("fleet: no home directory known; set FLEET_HOME or HOME");
        return ExitCode::from(2);
    };
    let selector = match (&options.proposal, &options.eval) {
        (Some(name), _) => fleet::explain::Selector::Proposal(name.clone()),
        (None, Some(scenario)) => fleet::explain::Selector::Scenario(scenario.clone()),
        (None, None) => {
            eprintln!("fleet: explain: needs a selector: --proposal <name> or --eval <scenario>");
            return ExitCode::from(2);
        }
    };
    let user_home = std::env::var("HOME").ok();
    let root = fleet::calib::resolve_root(&home, user_home.as_deref());
    let outcome = fleet::explain::explain(fleet::explain::ExplainRequest {
        root: &root,
        home: &home,
        selector,
        dry_run: options.dry_run,
        lane: fleet::explain::LaneSource::Vendored,
    })
    .await;
    print!("{}", outcome.card);
    if outcome.refusal.is_some() {
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
