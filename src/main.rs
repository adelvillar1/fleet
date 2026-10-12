//! Fleet: a native launcher for the DeepSeek Harness agent runtime.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fleet::cli::{self, Command};
use fleet::{discovery, doctor, net, nodever, run};

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
        Ok(Command::Status(options)) => status_command(&options),
        Ok(Command::Doctor(options)) => doctor_command(&options),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

/// Reads the user home from the environment, Unix and Windows spellings.
fn user_home() -> Option<PathBuf> {
    discovery::home_dir(
        std::env::var("HOME").ok().as_deref(),
        std::env::var("USERPROFILE").ok().as_deref(),
    )
}

/// Gathers the discovery inputs from the environment and resolves the harness
/// checkout for commands that need one.
fn find_harness(flag: Option<&Path>) -> Result<PathBuf, String> {
    let env_home = std::env::var("FLEET_HOME").ok();
    let installed = user_home().as_deref().map(discovery::installed_home);
    let cwd = std::env::current_dir().unwrap_or_default();
    discovery::resolve_root(flag, env_home.as_deref(), installed, &cwd)
}

fn run_command(options: &cli::RunOptions) -> ExitCode {
    let root = match find_harness(options.home.as_deref()) {
        Ok(root) => root,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let node: String = options
        .node
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| std::env::var("FLEET_NODE").ok())
        .unwrap_or_else(|| "node".to_string());
    match nodever::node_version(&node) {
        Ok(version) if nodever::meets_floor(version) => {}
        Ok(version) => {
            eprintln!(
                "fleet: node v{}.{}.{} is outside the supported range ^{}.{} || >=24; \
                 install a supported Node.js or point --node at one",
                version.0, version.1, version.2, nodever::NODE_FLOOR.0, nodever::NODE_FLOOR.1
            );
            return ExitCode::FAILURE;
        }
        Err(message) => {
            eprintln!("fleet: {message}");
            return ExitCode::FAILURE;
        }
    }
    let profile = options
        .profile
        .clone()
        .unwrap_or_else(|| cli::DEFAULT_PROFILE.to_string());
    let args = run::build_args(&profile, &options.patches, &options.forward);
    println!("fleet: launching the DeepSeek Harness at {}", root.display());
    match run::launch(&root, &node, &args) {
        Ok(code) => ExitCode::from((code & 0xff) as u8),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn status_command(options: &cli::StatusOptions) -> ExitCode {
    if net::listening(&options.host, options.port) {
        println!(
            "fleet: the DeepSeek Harness web UI is listening on http://{}:{}",
            options.host, options.port
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "fleet: nothing is listening on {}:{}",
            options.host, options.port
        );
        ExitCode::FAILURE
    }
}

fn doctor_command(options: &cli::DoctorOptions) -> ExitCode {
    let node = std::env::var("FLEET_NODE").unwrap_or_else(|_| "node".to_string());
    let root = find_harness(options.home.as_deref()).ok();
    let mut checks = vec![doctor::evaluate_node(nodever::node_version(&node))];
    match &root {
        Some(root) => {
            checks.push(doctor::evaluate_harness(Some(root)));
            checks.push(doctor::evaluate_deps(root));
        }
        None => checks.push(doctor::evaluate_harness(None)),
    }
    let dsh_home = discovery::dsh_home(
        std::env::var("DSH_HOME").ok().as_deref(),
        user_home().as_deref(),
    );
    checks.push(match dsh_home {
        Some(home) => doctor::evaluate_profiles(&home),
        None => doctor::Check {
            name: "dsh home",
            ok: false,
            detail: "no user home known; set HOME, USERPROFILE, or DSH_HOME".to_string(),
        },
    });
    let mut failed = false;
    for check in &checks {
        println!("{}", doctor::render(check));
        failed |= !check.ok;
    }
    let web = doctor::evaluate_web(
        net::listening(cli::DEFAULT_HOST, cli::DEFAULT_PORT),
        cli::DEFAULT_HOST,
        cli::DEFAULT_PORT,
    );
    println!("{}", doctor::render(&web));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
