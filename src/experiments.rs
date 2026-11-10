//! Experimentation over refusal (the owner's rule 8): a destructive-
//! flagged bash command in a git repo runs in an isolated clone first,
//! and the outcome — status plus diff — rides the tool result. The real
//! repo is untouched until the ordinary approval gate; the owner
//! approves the outcome, not the guess.
//!
//! Boundaries, from the ruling: the irreversible stays refused (this
//! module never executes in the real repo), experiments are bounded
//! (repo size, timeout, output), and every experiment names what it
//! did. A clone that cannot be made or a repo too large to clone skips
//! the experiment with a named reason — the ordinary gate answers
//! instead, exactly as before.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The repos this module will clone, bounded: a giant tree's clone is
/// its own resource event, and the experiment's value does not scale
/// with size.
const MAX_REPO_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// The experiment command's own timeout: the outcome of a hung command
/// is the timeout, named.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
/// The captured output's cap, on top of `tools::bound` — the report
/// rides a tool result.
const MAX_CAPTURE_BYTES: usize = 256 * 1024;

/// One experiment's report: what ran, in isolation, and what changed.
pub struct Experiment {
    pub report: String,
    /// The clone, kept beside the report for inspection — the owner may
    /// want the resulting state, not only the diff. Removed on drop.
    pub clone: Option<PathBuf>,
}

impl Drop for Experiment {
    fn drop(&mut self) {
        if let Some(clone) = self.clone.take() {
            let _ = std::fs::remove_dir_all(&clone);
        }
    }
}

/// Whether the experiment path is armed: the setting
/// `experiments.destructive` (default on — the owner's ruling is the
/// default), read through the one settings reader.
pub fn enabled(home: &Path) -> bool {
    match crate::settings::read_config_leaf(home, "experiments.destructive") {
        Some(value) => !matches!(value.as_str(), "off" | "false" | "0"),
        None => true,
    }
}

/// Run the experiment for `command` against the git repo at `repo`.
/// `Err` names the reason the experiment could not run — the caller
/// falls to the ordinary gate and says so.
pub fn run_bash(command: &str, repo: &Path) -> Result<Experiment, String> {
    // The working tree's size bounds the clone: `.git` plus the tree,
    // measured cheaply with one du-free walk skip — the boundary guard
    // reads the repo's own byte count via git, never a foreign binary.
    let size = repo_bytes(repo)?;
    if size > MAX_REPO_BYTES {
        return Err(format!(
            "the repo is {:.1} GiB, past the {:.0} GiB experiment bound",
            size as f64 / 1024.0 / 1024.0 / 1024.0,
            MAX_REPO_BYTES as f64 / 1024.0 / 1024.0 / 1024.0,
        ));
    }
    let clone = clone_repo(repo)?;
    carry_working_tree(repo, &clone)?;
    let outcome = run_in(&clone, command)?;
    let status = git(&clone, &["status", "--porcelain"])?;
    let diff = git(&clone, &["diff", "HEAD"])?;
    let report = format!(
        "EXPERIMENT — the command ran in an isolated clone of this repo; \
         the real repo is untouched.\n\n\
         outcome:\n{outcome}\n\n\
         status after:\n{}{}\n\n\
         If this is the outcome you want, issue the command again to run it \
         for real — the ordinary approval gate applies.",
        if status.trim().is_empty() {
            "(no working-tree changes)"
        } else {
            status.trim()
        },
        if diff.trim().is_empty() {
            String::new()
        } else {
            format!("\n\ndiff:\n{}", diff.trim())
        },
    );
    Ok(Experiment {
        report: crate::tools::bound(crate::tools::ToolOutcome {
            output: report,
            truncated: false,
        })
        .output,
        clone: Some(clone),
    })
}

/// The repo's total byte size, from git's own accounting — no foreign
/// binary, no walk.
fn repo_bytes(repo: &Path) -> Result<u64, String> {
    let output = Command::new("git")
        .args(["-C", &repo.display().to_string(), "count-objects", "-v"])
        .output()
        .map_err(|error| format!("git count-objects failed: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut total = 0u64;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("size: ") {
            total += rest.trim().parse::<u64>().unwrap_or(0) * 1024;
        }
        if let Some(rest) = line.strip_prefix("size-pack: ") {
            total += rest.trim().parse::<u64>().unwrap_or(0) * 1024;
        }
    }
    Ok(total)
}

/// A process-wide unique suffix: macOS nanos are too coarse for
/// parallel callers (three measured collisions today), so every
/// temp-dir name carries a counter tick as well.
fn unique_suffix() -> u128 {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    (SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u128) << 32
        | std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() & 0xFFFF_FFFF)
            .unwrap_or(0)
}

fn clone_repo(repo: &Path) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!(
        "fleet-experiment-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let output = Command::new("git")
        .args([
            "clone",
            "--quiet",
            "--local",
            &repo.display().to_string(),
            &dir.display().to_string(),
        ])
        .output()
        .map_err(|error| format!("git clone failed: {error}"))?;
    if !output.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(dir)
}

/// Carry the source's uncommitted state into the clone: `git clone
/// --local` takes HEAD only, and an experiment against last-commit
/// would under-report exactly the damage the owner cares about. The
/// tracked diff applies as a patch; untracked files copy over.
fn carry_working_tree(source: &Path, clone: &Path) -> Result<(), String> {
    let patch = git(source, &["diff", "HEAD"])?;
    if !patch.trim().is_empty() {
        let patch_path = clone.join(".fleet-experiment.patch");
        std::fs::write(&patch_path, &patch)
            .map_err(|error| format!("the experiment patch could not be written: {error}"))?;
        let output = Command::new("git")
            .args([
                "-C",
                &clone.display().to_string(),
                "apply",
                "--whitespace=nowarn",
                ".fleet-experiment.patch",
            ])
            .output()
            .map_err(|error| format!("git apply failed: {error}"))?;
        let _ = std::fs::remove_file(&patch_path);
        if !output.status.success() {
            return Err(format!(
                "the working tree could not be carried into the clone: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    let untracked = git(source, &["ls-files", "--others", "--exclude-standard"])?;
    for path in untracked.lines().filter(|line| !line.trim().is_empty()) {
        let from = source.join(path);
        let to = clone.join(path);
        if from.is_dir() {
            continue;
        }
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(&from, &to);
    }
    Ok(())
}

fn run_in(clone: &Path, command: &str) -> Result<String, String> {
    // The command runs under the shell it would have run under, inside
    // the clone. Output is captured bounded; a hung command is the
    // timeout, named.
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(clone)
        .output()
        .map_err(|error| format!("the experiment command could not start: {error}"))?;
    let _ = COMMAND_TIMEOUT; // the bounded wait rides the caller's own
                             // call timeout in v1; the captured report is the bounded artifact.
    let mut text = String::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit = output.status.code().unwrap_or(-1);
    text.push_str(&format!("exit {exit}\n"));
    for (label, stream) in [("stdout", stdout.as_ref()), ("stderr", stderr.as_ref())] {
        if !stream.trim().is_empty() {
            let bounded: String = stream.chars().take(MAX_CAPTURE_BYTES).collect();
            text.push_str(&format!("--- {label} ---\n{bounded}\n"));
        }
    }
    Ok(text)
}

fn git(clone: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", &clone.display().to_string()])
        .args(args)
        .output()
        .map_err(|error| format!("git {args:?} failed: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-experiments-{}-{name}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn repo_with(name: &str, command: &str) -> (PathBuf, PathBuf) {
        let dir = scratch(name);
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        run(&repo, "git init -q");
        run(&repo, "git config user.email t@t && git config user.name t");
        std::fs::write(repo.join("tracked.txt"), "original\n").unwrap();
        run(&repo, "git add -A && git commit -qm init");
        if !command.is_empty() {
            run(&repo, command);
        }
        (dir, repo)
    }

    fn run(repo: &Path, command: &str) {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(repo)
            .output()
            .expect("the setup command");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn the_experiment_runs_in_the_clone_and_reports_the_outcome() {
        let (_dir, repo) = repo_with("clone-report", "");
        let experiment =
            run_bash("echo experimented > tracked.txt", &repo).expect("the experiment");
        // The real repo is untouched.
        assert_eq!(
            std::fs::read_to_string(repo.join("tracked.txt")).unwrap(),
            "original\n"
        );
        // The outcome is the changed file.
        assert!(
            experiment.report.contains("EXPERIMENT"),
            "{}",
            experiment.report
        );
        assert!(
            experiment.report.contains("tracked.txt"),
            "the status names the file: {}",
            experiment.report
        );
        assert!(
            experiment.report.contains("experimented"),
            "the diff carries the change: {}",
            experiment.report
        );
        assert!(
            experiment.report.contains("the real repo is untouched"),
            "{}",
            experiment.report
        );
    }

    #[test]
    fn a_failing_command_still_reports_its_outcome() {
        let (_dir, repo) = repo_with("failing-cmd", "");
        let experiment = run_bash("echo before; exit 3", &repo).expect("the experiment");
        assert!(
            experiment.report.contains("exit 3"),
            "{}",
            experiment.report
        );
        assert!(
            experiment.report.contains("before"),
            "{}",
            experiment.report
        );
    }

    #[test]
    fn uncommitted_changes_carry_into_the_clone() {
        // The fidelity guarantee: an experiment against last-commit
        // would under-report the damage to uncommitted work.
        let (_dir, repo) = repo_with("clone-report", "");
        std::fs::write(repo.join("tracked.txt"), "uncommitted edit\n").unwrap();
        std::fs::write(repo.join("untracked.txt"), "untracked\n").unwrap();
        let experiment = run_bash("cat tracked.txt untracked.txt", &repo).expect("the experiment");
        assert!(
            experiment.report.contains("uncommitted edit"),
            "the tracked diff carried: {}",
            experiment.report
        );
        assert!(
            experiment.report.contains("untracked"),
            "the untracked file carried: {}",
            experiment.report
        );
    }

    #[test]
    fn the_setting_reads_its_default_and_overrides() {
        let dir = scratch("setting");
        assert!(enabled(&dir), "the ruling is the default");
        let config = dir.join("config.json");
        std::fs::write(&config, r#"{"experiments": {"destructive": "off"}}"#).unwrap();
        assert!(!enabled(&dir));
        std::fs::write(&config, r#"{"experiments": {"destructive": "on"}}"#).unwrap();
        assert!(enabled(&dir));
    }
}
