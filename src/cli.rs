//! Command-line grammar for the `fleet` binary.

use std::path::PathBuf;

/// Profile booted when `run` names none; mirrors a bare `dsh web`.
pub const DEFAULT_PROFILE: &str = "web";
/// Host the `status` command probes by default; the web UI binds loopback only.
pub const DEFAULT_HOST: &str = "127.0.0.1";
/// Port the `status` command probes by default; the web UI's shipped default.
pub const DEFAULT_PORT: u16 = 3080;

/// A parsed `fleet` invocation.
#[derive(Debug)]
pub enum Command {
    /// `fleet [run]`: launch the harness.
    Run(RunOptions),
    /// `fleet status`: probe the harness web UI's port.
    Status(StatusOptions),
    /// `fleet doctor`: probe node, the checkout, and the dsh home.
    Doctor(DoctorOptions),
    /// `fleet --version`.
    Version,
    /// `fleet --help`.
    Help,
}

/// Options of the `run` command.
#[derive(Debug)]
pub struct RunOptions {
    /// Harness checkout to launch, from `--home`.
    pub home: Option<PathBuf>,
    /// Node binary to launch with, from `--node`.
    pub node: Option<PathBuf>,
    /// Profile under `$DSH_HOME/profiles` to boot, from `--profile` or the first positional.
    pub profile: Option<String>,
    /// Extra patch overlays, from repeatable `--patch`.
    pub patches: Vec<PathBuf>,
    /// Arguments after `--` or the profile, forwarded verbatim to the harness.
    pub forward: Vec<String>,
}

/// Options of the `status` command.
#[derive(Debug)]
pub struct StatusOptions {
    /// Host to probe.
    pub host: String,
    /// Port to probe.
    pub port: u16,
}

/// Options of the `doctor` command.
#[derive(Debug)]
pub struct DoctorOptions {
    /// Overrides `FLEET_HOME` discovery for the harness checkout.
    pub home: Option<PathBuf>,
}

/// The `fleet --help` text.
pub const HELP: &str = "\
Fleet - native launcher for the DeepSeek Harness agent runtime

Usage:
  fleet [run] [profile] [options] [-- <harness arguments>...]
  fleet status [--host <host>] [--port <port>]
  fleet doctor [--home <path>]
  fleet -V | fleet --version
  fleet -h | fleet --help

Commands:
  run      Launch the harness (default command; default profile: web)
  status   Report whether the harness web UI is listening
  doctor   Probe node, the harness checkout, and the dsh home

Options:
  --home <path>     Harness checkout to launch (default: FLEET_HOME, then
                    ~/.fleet/harness, then the current directory)
  --node <path>     Node binary to launch with (default: FLEET_NODE, then node)
  --profile <name>  Profile under $DSH_HOME/profiles to boot
  --patch <path>    Extra patch-list overlay applied after the profile layer
                    (repeatable)
  --host <host>     status: host to probe (default 127.0.0.1)
  --port <port>     status: port to probe (default 3080)

Fleet's own flags are recognized anywhere before `--`. Unknown flags, `--`
arguments, and words past the profile are forwarded verbatim to the harness.
";

/// Parses `args` (argv without the program name) into a [`Command`].
pub fn parse(args: &[String]) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        None => Ok(Command::Run(RunOptions {
            home: None,
            node: None,
            profile: None,
            patches: Vec::new(),
            forward: Vec::new(),
        })),
        Some("-h" | "--help") => Ok(Command::Help),
        Some("-V" | "--version") => Ok(Command::Version),
        Some("run") => parse_run(&args[1..]).map(Command::Run),
        Some("status") => parse_status(&args[1..]).map(Command::Status),
        Some("doctor") => parse_doctor(&args[1..]).map(Command::Doctor),
        Some(other) => Err(format!(
            "fleet: unknown command {other:?}; expected run, status, doctor, --help, or --version"
        )),
    }
}

/// Reads the value of `flag` at `args[*index]`, from `--flag=value` or the next token.
fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    let raw = &args[*index];
    if let Some(rest) = raw.strip_prefix(&format!("{flag}=")) {
        return Ok(rest.to_string());
    }
    match args.get(*index + 1) {
        Some(value) => {
            *index += 1;
            Ok(value.clone())
        }
        None => Err(format!("fleet: {flag} needs a value")),
    }
}

fn parse_run(args: &[String]) -> Result<RunOptions, String> {
    let mut options = RunOptions {
        home: None,
        node: None,
        profile: None,
        patches: Vec::new(),
        forward: Vec::new(),
    };
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--" {
            options.forward.extend(args[index + 1..].iter().cloned());
            break;
        }
        if arg == "--home" || arg.starts_with("--home=") {
            options.home = Some(PathBuf::from(take_value(args, &mut index, "--home")?));
        } else if arg == "--node" || arg.starts_with("--node=") {
            options.node = Some(PathBuf::from(take_value(args, &mut index, "--node")?));
        } else if arg == "--profile" || arg.starts_with("--profile=") {
            if options.profile.is_some() {
                return Err("fleet: --profile given more than once".to_string());
            }
            options.profile = Some(take_value(args, &mut index, "--profile")?);
        } else if arg == "--patch" || arg.starts_with("--patch=") {
            options
                .patches
                .push(PathBuf::from(take_value(args, &mut index, "--patch")?));
        } else if arg.starts_with('-') && arg != "-" {
            // Unknown flags belong to the harness, mirroring the launcher's
            // pass-through semantics (`dsh` collects app flags the same way).
            options.forward.push(arg.to_string());
        } else if options.profile.is_none() {
            options.profile = Some(arg.to_string());
        } else {
            options.forward.push(arg.to_string());
        }
        index += 1;
    }
    Ok(options)
}

fn parse_status(args: &[String]) -> Result<StatusOptions, String> {
    let mut host = DEFAULT_HOST.to_string();
    let mut port = DEFAULT_PORT;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--host" || arg.starts_with("--host=") {
            host = take_value(args, &mut index, "--host")?;
        } else if arg == "--port" || arg.starts_with("--port=") {
            let raw = take_value(args, &mut index, "--port")?;
            port = raw
                .parse()
                .map_err(|_| format!("fleet: --port needs a number, got {raw:?}"))?;
        } else {
            return Err(format!("fleet: unknown status option {arg:?}; see `fleet --help`"));
        }
        index += 1;
    }
    Ok(StatusOptions { host, port })
}

fn parse_doctor(args: &[String]) -> Result<DoctorOptions, String> {
    let mut home = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--home" || arg.starts_with("--home=") {
            home = Some(PathBuf::from(take_value(args, &mut index, "--home")?));
        } else {
            return Err(format!("fleet: unknown doctor option {arg:?}; see `fleet --help`"));
        }
        index += 1;
    }
    Ok(DoctorOptions { home })
}
