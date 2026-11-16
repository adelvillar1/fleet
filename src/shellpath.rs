//! The owner's shell environment, for a process that was not started by
//! one.
//!
//! A dock-launched app inherits launchd's environment, where `PATH` is
//! `/usr/bin:/bin:/usr/sbin:/sbin`. Everything the owner installed lives
//! elsewhere — `docker` in `/usr/local/bin`, `npx` in `~/.local/bin`, a
//! modern `python3` in `/opt/homebrew/bin` — so the app's own probes see a
//! machine with no tools on it: the runtime card called a healthy kiri
//! daemon dead because `docker info` could not be spawned, three MCP
//! servers failed to start with ENOENT, and the blend1 lane refused a
//! 3.9.6 interpreter that no terminal would have given it.
//!
//! The fix is to adopt the login shell's `PATH` once, before anything
//! spawns. That is the environment the owner's own terminal has: the
//! machine the app is meant to see. It is not a value the app hands out —
//! the binaries still have to exist. A read that fails, hangs or says
//! nothing leaves the inherited `PATH` untouched, because a narrow `PATH`
//! is a degradation while an app that will not start is not.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The markers the shell's output is cut between. A profile may print
/// anything it likes; only what sits inside these is the `PATH`.
const BEGIN: &str = "__FLEET_PATH_BEGIN__";
const END: &str = "__FLEET_PATH_END__";

/// How long the shell may take. A login profile is normally instant; one
/// that stalls (a network profile, a hung version manager) must not hold
/// the window.
const READ_BUDGET: Duration = Duration::from_secs(3);

/// The `PATH` a login shell for `shell` reports.
///
/// The interactive shell is tried first, because that is what a terminal
/// opens (it sources `.zshrc` as well as the login files); the plain login
/// shell is the fallback. `None` when the shell cannot be spawned, says
/// nothing between the sentinels, or outlives [`READ_BUDGET`].
pub fn login_path(shell: &Path) -> Option<String> {
    login_path_with(shell, "-lic").or_else(|| login_path_with(shell, "-lc"))
}

fn login_path_with(shell: &Path, flags: &str) -> Option<String> {
    let mut child = Command::new(shell)
        .arg(flags)
        .arg(format!("printf '{BEGIN}%s{END}' \"$PATH\""))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + READ_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            // A stalled profile: kill it and leave the caller's PATH alone.
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }

    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    parse_login_path(&out)
}

/// The `PATH` between the sentinels, from output that may carry a
/// profile's chatter before and after it.
fn parse_login_path(output: &str) -> Option<String> {
    let start = output.find(BEGIN)? + BEGIN.len();
    let end = start + output[start..].find(END)?;
    let path = output[start..end].trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// The login `PATH`'s entries first — the owner's own ordering is the
/// authoritative one — then whatever the inherited `PATH` adds. Empty
/// entries (a profile can leave `::`) and repeats are dropped.
fn merge(current: Option<&OsStr>, login: &str) -> OsString {
    let inherited: Vec<PathBuf> = match current {
        Some(path) => std::env::split_paths(path).collect(),
        None => Vec::new(),
    };
    let mut entries: Vec<OsString> = Vec::new();
    for entry in std::env::split_paths(OsStr::new(login)).chain(inherited) {
        if entry.as_os_str().is_empty() || entries.iter().any(|seen| seen == &entry) {
            continue;
        }
        entries.push(entry.into_os_string());
    }
    std::env::join_paths(entries).unwrap_or_else(|_| OsString::from(login))
}

/// Adopt the login shell's `PATH` into this process, returning what was
/// set. `None` — the environment left untouched — when the shell cannot be
/// read; see the module docs for why that is the safe direction.
pub fn adopt_login_path() -> Option<OsString> {
    let shell = shell_path();
    let login = login_path(&shell)?;
    let current = std::env::var_os("PATH");
    let merged = merge(current.as_deref(), &login);
    // SAFETY-adjacent note for readers: set_var is safe here because the
    // shell calls this once, before any thread spawns a child.
    std::env::set_var("PATH", &merged);
    Some(merged)
}

/// The shell the owner runs: `$SHELL`, then the platform's default.
fn shell_path() -> PathBuf {
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_is_cut_between_the_sentinels_past_profile_chatter() {
        let output =
            format!("welcome to your shell\n{BEGIN}/opt/homebrew/bin:/usr/bin{END}\nbye\n");
        assert_eq!(
            parse_login_path(&output).as_deref(),
            Some("/opt/homebrew/bin:/usr/bin")
        );
    }

    #[test]
    fn an_output_without_the_sentinels_is_no_path() {
        assert!(parse_login_path("welcome\n").is_none());
        assert!(parse_login_path(&format!("{BEGIN}{END}")).is_none());
        assert!(parse_login_path("").is_none());
    }

    #[test]
    fn the_login_entries_come_first_and_repeats_drop() {
        let merged = merge(
            Some(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin")),
            "/opt/homebrew/bin:/usr/bin:/Users/owner/.local/bin",
        );
        let entries: Vec<_> = std::env::split_paths(&merged).collect();
        assert_eq!(
            entries,
            vec![
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/Users/owner/.local/bin"),
                PathBuf::from("/bin"),
                PathBuf::from("/usr/sbin"),
                PathBuf::from("/sbin"),
            ]
        );
    }

    #[test]
    fn empty_entries_never_join_the_path() {
        let merged = merge(Some(OsStr::new("")), "/opt/homebrew/bin::/usr/bin");
        assert_eq!(merged, OsString::from("/opt/homebrew/bin:/usr/bin"));
    }

    #[test]
    fn a_real_shell_reports_its_path() {
        let shell = Path::new("/bin/sh");
        let path = login_path(shell).expect("a POSIX shell answers");
        assert!(!path.is_empty());
        assert!(
            path.split(':').any(|entry| entry == "/bin"),
            "the PATH carries /bin: {path}"
        );
    }

    #[test]
    fn a_shell_that_does_not_exist_says_nothing() {
        assert!(login_path(Path::new("/nonexistent/fleet-shell")).is_none());
    }
}
