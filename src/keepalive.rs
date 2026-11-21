//! Orphaned keep-alive processes: the `tail -f /dev/null` population.
//!
//! Measured 2026-11-21 on the owner's machine: **1,232** `tail -f /dev/null`
//! processes had reparented to launchd (`ppid=1`) over ~55 days, holding
//! 580 MB RSS and 46% of the per-uid process cap (`maxprocperuid` = 2,666),
//! with ~95 MB of RAM free. They are keep-alives whose owner went away —
//! nothing waits on an orphan, so each is pure leak. The spawner was never
//! identified (no literal in the Hermes tree; omh's attach module spawns no
//! processes at all), which is exactly why this belongs in the hygiene sweep:
//! the tool does not need to know who spawned them to bound their population.
//!
//! **This sweep is not restricted to Fleet's own processes**, and the reason is
//! that the rule governing the neighbouring stale-process sweep does not apply
//! here. That sweep kills only what it can prove Fleet spawned, because the
//! alternative is guessing about a process somebody is using. An orphaned
//! keep-alive is a different object: `ppid == 1` means the process that would
//! wait on it is **gone**, so stopping it cannot disrupt anything. Its only
//! effect is to hold a process slot, an address space and a file descriptor
//! forever. The goal is an environment in optimal shape, so the default is to
//! stop them all and report what they were.
//!
//! Two rails keep that safe rather than eager:
//!
//! 1. **The command must match exactly** — `tail -f /dev/null` and nothing
//!    else: not `tail -f /dev/null.bak`, not `/usr/bin/mytail`, not a command
//!    that merely starts the same way.
//! 2. **The parent must be gone** — a keep-alive with a live parent is
//!    somebody's sleep, possibly a supervisor deliberately holding a session
//!    open, and is left alone.
//!
//! Attribution (whether the orphan inherited a `FLEET_`/`HERMES_` namespace) is
//! still read, but only to *explain* the population in the report: the owner
//! deserves to know how much of the litter was the harness's. It is not a
//! permission gate. `FLEET_KEEPALIVE_DRY_RUN=1` turns the sweep into a census.

use std::process::Command;

/// One orphaned keep-alive, as `ps` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    pub pid: u32,
    /// The command as `ps` printed it (trimmed).
    pub command: String,
    /// Whether the process carries a `FLEET_`/`HERMES_` variable. Reported,
    /// never required: an orphan has no waiter, so whose it was does not
    /// change whether it should be stopped.
    pub attributed: bool,
}

/// The exact command a keep-alive runs. Matched on the whole command string so
/// a `tail -f /dev/null` a person typed by hand in a terminal — which would
/// have a parent, and therefore never reach this check — cannot be confused
/// with the orphan class by a substring.
const KEEPALIVE: &str = "tail -f /dev/null";

/// Read `ps -axo pid=,ppid=,command=` output and return the orphaned
/// keep-alives. Pure, so the parse is testable without a process to kill.
///
/// `ps` pads its columns to fixed widths, so the split collapses whitespace
/// runs: a bare split on one space hands the ppid slot an empty field and every
/// line dies at the parse — the stale sweep's own documented first-run bug.
pub(crate) fn parse(text: &str) -> Vec<Orphan> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some((pid_field, rest)) = trimmed.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some((ppid_field, command)) = rest.split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid_field.trim().parse::<u32>() else {
            continue;
        };
        let Ok(ppid) = ppid_field.trim().parse::<u32>() else {
            continue;
        };
        let command = command.trim();
        // An orphan is one whose parent is gone: ppid 1 is launchd (or init).
        // A keep-alive with a live parent is somebody's sleep and not here.
        if ppid == 1 && command == KEEPALIVE {
            out.push(Orphan {
                pid,
                command: command.to_string(),
                attributed: false,
            });
        }
    }
    out
}

/// The environment variables that make a keep-alive ours. `ps -E` prints the
/// environment of processes owned by the caller; a keep-alive spawned by the
/// harness inherits its namespace, and one spawned by anything else does not.
pub(crate) fn attribution_markers(env_line: &str) -> bool {
    env_line.contains("FLEET_") || env_line.contains("HERMES_")
}

/// Ask `ps` for the environment of one process and decide attribution.
///
/// Best-effort by design: a process that died between the two reads, or that
/// `ps` will not show an environment for, is simply reported unattributed.
/// Nothing about the sweep depends on the answer.
fn attributed(pid: u32) -> bool {
    let Ok(output) = Command::new("ps")
        .args(["-p", &pid.to_string(), "-E", "-o", "command="])
        .output()
    else {
        return false;
    };
    attribution_markers(&String::from_utf8_lossy(&output.stdout))
}

/// Every orphaned keep-alive on this machine, with its attribution filled in.
pub fn orphans() -> Vec<Orphan> {
    let Ok(output) = Command::new("ps")
        .args(["-axo", "pid=,ppid=,command="])
        .output()
    else {
        return Vec::new();
    };
    let mut found = parse(&String::from_utf8_lossy(&output.stdout));
    for one in &mut found {
        one.attributed = attributed(one.pid);
    }
    found
}

/// What one sweep did, kept as a value so the report cannot drift from the act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reaped {
    /// Orphans this run stopped.
    pub stopped: usize,
    /// Orphans this run only counted (the census).
    pub named: usize,
    /// How many of the population carried a harness namespace — the part of the
    /// story that says whether the harness is the litterer.
    pub attributed: usize,
}

/// Whether this run is a census rather than a sweep.
pub fn dry_run() -> bool {
    std::env::var("FLEET_KEEPALIVE_DRY_RUN").as_deref() == Ok("1")
}

/// Stop every orphaned keep-alive — or, in a census, count them.
///
/// No attribution gate: see the module doc for why an unwaited-on process is
/// safe to stop and why the environment's health is the goal. Attribution is
/// surfaced in the returned [`Reaped`] so the report can still say whose litter
/// it was.
pub fn reap(found: &[Orphan], census_only: bool) -> Reaped {
    let mut out = Reaped::default();
    for one in found {
        if one.attributed {
            out.attributed += 1;
        }
        if census_only {
            out.named += 1;
            continue;
        }
        if Command::new("kill")
            .arg(one.pid.to_string())
            .output()
            .map(|done| done.status.success())
            .unwrap_or(false)
        {
            out.stopped += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hundred-and-twenty-second trap: `ps` pads its columns, so a parse
    /// that splits on a single whitespace loses every line. This fixture has
    /// the padding a real `ps` prints.
    #[test]
    fn the_parse_reads_the_padded_columns_a_real_ps_prints() {
        // Exactly the three columns the sweep requests: pid=,ppid=,command=.
        // An `etime` column here would be the test inventing a format the code
        // never asks `ps` for — the parse would read the age as the command.
        let text = "\
  135      1 tail -f /dev/null
  416      1 tail -f /dev/null
 9012   9011 tail -f /dev/null
 9013      1 tail -f /some/other/file
";
        let found = parse(text);
        assert_eq!(
            found.iter().map(|one| one.pid).collect::<Vec<_>>(),
            vec![135, 416],
            "only ppid-1 processes running the keep-alive command are orphans"
        );
    }

    /// A keep-alive with a live parent is somebody's sleep, not a leak.
    #[test]
    fn a_keepalive_with_a_live_parent_is_not_an_orphan() {
        let found = parse(" 9012   9011 tail -f /dev/null\n");
        assert!(found.is_empty(), "{found:?}");
    }

    /// The command match is exact: a different file, or a prefixed command,
    /// is not this class. The substring trap is how a sweep kills the wrong
    /// thing.
    #[test]
    fn the_command_match_is_exact_not_a_substring() {
        for line in [
            "  7      1 tail -f /dev/null.bak\n",
            "  8      1 /usr/bin/mytail -f /dev/null\n",
            "  9      1 tail -f /dev/null --retry\n",
        ] {
            assert!(parse(line).is_empty(), "{line}");
        }
        assert_eq!(parse("  7      1 tail -f /dev/null\n").len(), 1);
    }

    /// The policy: an orphan is stopped whoever spawned it, because nothing
    /// waits on it — and attribution is reported, not required.
    #[test]
    fn an_orphan_is_reaped_regardless_of_who_spawned_it() {
        let theirs = Orphan {
            pid: 42,
            command: KEEPALIVE.to_string(),
            attributed: false,
        };
        let census = reap(std::slice::from_ref(&theirs), true);
        assert_eq!(
            census,
            Reaped {
                stopped: 0,
                named: 1,
                attributed: 0
            },
            "a census counts it and stops nothing"
        );

        let ours = Orphan {
            attributed: true,
            ..theirs.clone()
        };
        let both = reap(&[theirs, ours], true);
        assert_eq!(
            both,
            Reaped {
                stopped: 0,
                named: 2,
                attributed: 1
            },
            "attribution explains the litter without gating the sweep"
        );
    }

    /// The marker check reads the harness's namespaces and nothing else, so a
    /// stray PATH entry containing `FLEET` as a word cannot attribute a process.
    #[test]
    fn only_the_harness_namespaces_attribute_a_process() {
        assert!(attribution_markers(
            "TERM=xterm FLEET_HOME=/Users/a/.fleet PATH=/usr/bin"
        ));
        assert!(attribution_markers("HERMES_SESSION_ID=abc"));
        assert!(!attribution_markers(
            "PATH=/usr/bin:/opt/homebrew/bin HOME=/Users/a"
        ));
        assert!(
            !attribution_markers("PS1=fleet> "),
            "a bare word is not a namespace"
        );
    }
}
