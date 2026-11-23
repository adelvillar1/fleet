//! Shared plumbing for the Node harnesses the gate shells out to.
//!
//! KI-593: the face harness's ~290 fresh vm contexts provoked a V8
//! mark-compact segfault on older builds (ClearStaleLeftTrimmedPointerVisitor
//! over internal frames, ~1/10 full runs on v25.2.1; v26 measured clean), so
//! the harnesses require Node 26 or newer and refuse an older runtime by name.
//! This resolver finds a 26+ binary without trusting PATH order — on this
//! machine the default `node` is older than the Homebrew one.

use std::process::Command;

/// A Node 26+ binary for a harness: `FLEET_NODE` first, then Homebrew's
/// `node@26`, then `node26`, then whatever `node` is first on PATH.
///
/// `Ok(None)` means no node at all (the caller skips, as the gate always has
/// for a missing toolchain); `Err` names every runtime found when none is 26+,
/// because a gate must not report green about a harness it never ran.
pub fn node() -> Result<Option<String>, String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var("FLEET_NODE") {
        if !explicit.trim().is_empty() {
            candidates.push(explicit);
        }
    }
    candidates.push("/opt/homebrew/opt/node@26/bin/node".to_string());
    candidates.push("node26".to_string());
    candidates.push("node".to_string());
    let mut found: Vec<String> = Vec::new();
    for candidate in candidates {
        let Ok(output) = Command::new(&candidate).arg("--version").output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let major = text
            .trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|one| one.parse::<u32>().ok());
        if major.is_some_and(|major| major >= 26) {
            return Ok(Some(candidate));
        }
        if !found.iter().any(|one| one.ends_with(&text)) {
            found.push(format!("{candidate} ({text})"));
        }
    }
    if found.is_empty() {
        Ok(None)
    } else {
        Err(format!(
            "the harness requires Node 26 or newer (KI-593: older V8 builds segfault in \
             mark-compact GC over the harness's vm contexts). Found: {}. Install node@26 \
             (`brew install node@26`) or point FLEET_NODE at a 26+ binary.",
            found.join(", ")
        ))
    }
}
