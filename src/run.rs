//! Launching the harness through its documented source-launch vector.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::discovery::HARNESS_ENTRY;

/// Builds the harness argv: patch overlays first (launcher flags precede the
/// profile positional), then the profile, then forwarded app arguments.
pub fn build_args(profile: &str, patches: &[PathBuf], forward: &[String]) -> Vec<String> {
    let mut args = Vec::new();
    for patch in patches {
        args.push("--patch".to_string());
        args.push(patch.display().to_string());
    }
    args.push(profile.to_string());
    args.extend(forward.iter().cloned());
    args
}

/// Spawns `node --import tsx/esm <entry> <args>` in the checkout with
/// inherited stdio and waits for it. Ctrl+C reaches the child through the
/// terminal's process group, exactly as it does for a directly launched
/// `dsh`; the child's exit code is Fleet's.
pub fn launch(root: &Path, node: &str, args: &[String]) -> Result<i32, String> {
    let status = Command::new(node)
        .arg("--import")
        .arg("tsx/esm")
        .arg(root.join(HARNESS_ENTRY))
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("fleet: cannot launch {node}: {error}"))?;
    Ok(status.code().unwrap_or(130))
}
