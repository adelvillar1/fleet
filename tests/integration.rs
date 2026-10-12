//! Behavior tests for Fleet's parsing, discovery, version floor, probing,
//! doctor checks, and launch-argument construction. Standard library only;
//! no network beyond loopback.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use fleet::cli::{self, Command};
use fleet::{discovery, doctor, net, nodever, run};

fn tempdir(name: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "fleet-tests-{}-{}-{}",
        name,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_harness(name: &str) -> PathBuf {
    let dir = tempdir(name);
    std::fs::create_dir_all(dir.join("apps/cli/src")).unwrap();
    std::fs::write(dir.join("apps/cli/src/bin.ts"), "// harness entry").unwrap();
    dir
}

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn as_run(command: Command) -> cli::RunOptions {
    match command {
        Command::Run(options) => options,
        _ => panic!("expected a run command, got {command:?}"),
    }
}

fn as_status(command: Command) -> cli::StatusOptions {
    match command {
        Command::Status(options) => options,
        _ => panic!("expected a status command, got {command:?}"),
    }
}

#[test]
fn bare_invocation_and_run_are_the_same_command() {
    let bare = as_run(cli::parse(&argv(&[])).unwrap());
    let explicit = as_run(cli::parse(&argv(&["run"])).unwrap());
    assert!(bare.profile.is_none() && bare.forward.is_empty());
    assert!(explicit.profile.is_none() && explicit.forward.is_empty());
}

#[test]
fn run_takes_a_profile_then_forwards_words() {
    let options = as_run(cli::parse(&argv(&["run", "fleet", "--no-open"])).unwrap());
    assert_eq!(options.profile.as_deref(), Some("fleet"));
    assert_eq!(options.forward, argv(&["--no-open"]));
}

#[test]
fn run_accepts_flags_in_either_form_and_stops_at_double_dash() {
    let options = as_run(cli::parse(&argv(&[
        "run",
        "--home",
        "/tmp/h",
        "--profile=web",
        "--patch",
        "/tmp/p.yml",
        "--patch=/tmp/q.yml",
        "--",
        "--port",
        "0",
    ]))
    .unwrap());
    assert_eq!(options.home, Some(PathBuf::from("/tmp/h")));
    assert_eq!(options.profile.as_deref(), Some("web"));
    assert_eq!(
        options.patches,
        vec![PathBuf::from("/tmp/p.yml"), PathBuf::from("/tmp/q.yml")]
    );
    assert_eq!(options.forward, argv(&["--port", "0"]));
}

#[test]
fn run_forwards_unknown_flags_and_rejects_bad_fleet_flags() {
    // Unknown flags pass through to the harness, mirroring `dsh` semantics.
    let options = as_run(cli::parse(&argv(&["run", "--wat"])).unwrap());
    assert!(options.profile.is_none());
    assert_eq!(options.forward, argv(&["--wat"]));
    // Fleet's own flags still fail loudly when malformed.
    assert!(cli::parse(&argv(&["run", "--home"])).is_err());
    assert!(cli::parse(&argv(&["run", "--profile", "a", "--profile", "b"])).is_err());
}

#[test]
fn status_defaults_to_the_web_ui_address() {
    let defaults = as_status(cli::parse(&argv(&["status"])).unwrap());
    assert_eq!(defaults.host, "127.0.0.1");
    assert_eq!(defaults.port, 3080);

    let custom = as_status(
        cli::parse(&argv(&["status", "--host", "0.0.0.0", "--port=4080"])).unwrap(),
    );
    assert_eq!(custom.host, "0.0.0.0");
    assert_eq!(custom.port, 4080);

    assert!(cli::parse(&argv(&["status", "--port", "http"])).is_err());
    assert!(cli::parse(&argv(&["status", "extra"])).is_err());
}

#[test]
fn doctor_takes_only_a_home() {
    match cli::parse(&argv(&["doctor", "--home", "/tmp/h"])).unwrap() {
        Command::Doctor(options) => assert_eq!(options.home, Some(PathBuf::from("/tmp/h"))),
        _ => panic!("expected a doctor command"),
    }
    assert!(cli::parse(&argv(&["doctor", "--wat"])).is_err());
}

#[test]
fn version_and_help_tokens_parse_before_any_command() {
    assert!(matches!(cli::parse(&argv(&["--version"])).unwrap(), Command::Version));
    assert!(matches!(cli::parse(&argv(&["-V"])).unwrap(), Command::Version));
    assert!(matches!(cli::parse(&argv(&["--help"])).unwrap(), Command::Help));
    assert!(cli::parse(&argv(&["deploy"])).is_err());
}

#[test]
fn version_parsing_tolerates_prefixes_and_suffixes() {
    assert_eq!(nodever::parse_version("v22.19.0\n"), Some((22, 19, 0)));
    assert_eq!(nodever::parse_version("24.1.2"), Some((24, 1, 2)));
    assert_eq!(nodever::parse_version("v22.19.0-beta.1"), Some((22, 19, 0)));
    assert_eq!(nodever::parse_version("v24.0.0+build.7"), Some((24, 0, 0)));
    assert_eq!(nodever::parse_version("bogus"), None);
    assert_eq!(nodever::parse_version("v22"), None);
}

#[test]
fn version_floor_matches_the_harness_engines_field() {
    assert!(nodever::meets_floor((22, 19, 0)));
    assert!(nodever::meets_floor((22, 20, 5)));
    assert!(!nodever::meets_floor((22, 18, 3)));
    assert!(!nodever::meets_floor((23, 5, 0)));
    assert!(nodever::meets_floor((24, 0, 0)));
    assert!(nodever::meets_floor((25, 1, 0)));
    assert!(!nodever::meets_floor((21, 9, 9)));
}

#[test]
fn discovery_prefers_flag_then_env_then_installed_then_cwd() {
    let harness = make_harness("discovery");
    let plain = tempdir("discovery-plain");

    assert!(discovery::looks_like_harness(&harness));
    assert!(!discovery::looks_like_harness(&plain));

    let from_flag = discovery::resolve_root(Some(&harness), Some("/nonexistent"), None, &plain);
    assert_eq!(from_flag.unwrap(), harness);

    let env_path = harness.to_str().unwrap().to_string();
    let from_env = discovery::resolve_root(None, Some(&env_path), None, &plain);
    assert_eq!(from_env.unwrap(), harness);

    let from_installed = discovery::resolve_root(None, None, Some(harness.clone()), &plain);
    assert_eq!(from_installed.unwrap(), harness);

    let from_cwd = discovery::resolve_root(None, None, None, &harness);
    assert_eq!(from_cwd.unwrap(), harness);

    let error = discovery::resolve_root(None, None, Some(plain.clone()), &plain).unwrap_err();
    assert!(error.contains("no DeepSeek Harness checkout found"));
    assert!(error.contains(&plain.display().to_string()));
}

#[test]
fn installed_and_home_directories_follow_the_documented_layout() {
    assert_eq!(
        discovery::installed_home(Path::new("/Users/a")),
        PathBuf::from("/Users/a/.fleet/harness")
    );
    assert_eq!(
        discovery::home_dir(Some("/h"), Some("/w")),
        Some(PathBuf::from("/h"))
    );
    assert_eq!(
        discovery::home_dir(None, Some("/w")),
        Some(PathBuf::from("/w"))
    );
    assert_eq!(discovery::home_dir(None, None), None);
    assert_eq!(
        discovery::dsh_home(Some("/d"), Some(Path::new("/h"))),
        Some(PathBuf::from("/d"))
    );
    assert_eq!(
        discovery::dsh_home(None, Some(Path::new("/h"))),
        Some(PathBuf::from("/h/.dsh"))
    );
}

#[test]
fn probing_reports_a_live_listener_and_a_closed_port() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    assert!(net::listening("127.0.0.1", port));
    drop(listener);
    assert!(!net::listening("127.0.0.1", port));
    assert!(!net::listening("no.such.host.invalid", 3080));
}

#[test]
fn doctor_node_check_enforces_the_floor() {
    assert!(doctor::evaluate_node(Ok((22, 19, 0))).ok);
    assert!(doctor::evaluate_node(Ok((22, 20, 1))).ok);
    assert!(!doctor::evaluate_node(Ok((22, 18, 0))).ok);
    assert!(!doctor::evaluate_node(Err("cannot run node: no such file".to_string())).ok);
}

#[test]
fn doctor_harness_check_reports_a_missing_entry() {
    let harness = make_harness("doctor-harness");
    assert!(doctor::evaluate_harness(Some(&harness)).ok);
    let empty = tempdir("doctor-empty");
    let failed = doctor::evaluate_harness(Some(&empty));
    assert!(!failed.ok);
    assert!(failed.detail.contains(discovery::HARNESS_ENTRY));
    assert!(!doctor::evaluate_harness(None).ok);
}

#[test]
fn doctor_deps_check_names_the_install_command() {
    let plain = tempdir("doctor-deps");
    let failed = doctor::evaluate_deps(&plain);
    assert!(!failed.ok);
    assert!(failed.detail.contains("pnpm install"));
}

#[test]
fn doctor_profiles_check_lists_the_profiles() {
    let home = tempdir("doctor-dsh");
    std::fs::create_dir_all(home.join("profiles/web")).unwrap();
    std::fs::write(home.join("profiles/web/cordis.yml"), "").unwrap();
    std::fs::create_dir_all(home.join("profiles/fleet")).unwrap();
    std::fs::write(home.join("profiles/fleet/cordis.yml"), "").unwrap();
    // Directories without cordis.yml are not profiles.
    std::fs::create_dir_all(home.join("profiles/node_modules")).unwrap();
    let ok = doctor::evaluate_profiles(&home);
    assert!(ok.ok);
    assert!(ok.detail.contains("fleet") && ok.detail.contains("web"));
    assert!(!ok.detail.contains("node_modules"));

    let empty = tempdir("doctor-dsh-empty");
    assert!(!doctor::evaluate_profiles(&empty).ok);
}

#[test]
fn doctor_web_check_reports_the_endpoint() {
    assert!(doctor::evaluate_web(true, "127.0.0.1", 3080).ok);
    assert!(!doctor::evaluate_web(false, "127.0.0.1", 3080).ok);
}

#[test]
fn check_rendering_marks_failures() {
    let ok = doctor::render(&doctor::Check {
        name: "node",
        ok: true,
        detail: "v22.19.0".to_string(),
    });
    assert!(ok.starts_with(" ok ") && ok.contains("node"));
    let failed = doctor::render(&doctor::Check {
        name: "node",
        ok: false,
        detail: "missing".to_string(),
    });
    assert!(failed.starts_with("FAIL"));
}

#[test]
fn launch_arguments_put_patches_before_the_profile() {
    let args = run::build_args(
        "fleet",
        &[PathBuf::from("/tmp/a.yml")],
        &["--no-open".to_string()],
    );
    assert_eq!(args, argv(&["--patch", "/tmp/a.yml", "fleet", "--no-open"]));
    assert_eq!(run::build_args("web", &[], &[]), argv(&["web"]));
}
