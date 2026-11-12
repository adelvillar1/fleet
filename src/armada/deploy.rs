//! Deployment (armada L9.1, D51/D52/D57): fleet deploying fleet — fully
//! self-contained, no foreign control-plane service. A local spawn lays out a
//! new home directly; a VPS is reached over SSH and dropped the same
//! self-contained binary (the release raw-binary channel). The target mints a
//! fresh install-id (L1), clones the corpus remote (inheriting the mind, not
//! the identity — D29), and registers with the flagship (L8). A fresh install
//! is trusted only after it passes doctor and settles runs — never by arrival
//! (D57). Credentials resolve out-of-band (env/vault), never the corpus.

use std::path::{Path, PathBuf};

use serde_json::json;

/// A provision target: a local home, or a VPS reached over SSH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local {
        home: PathBuf,
    },
    Vps {
        host: String,
        user: String,
        home: String,
    },
}

/// What a provision produced. A fresh install is always un-calibrated until it
/// settles (D49) — `calibrated` is false by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionReport {
    pub install_id: String,
    pub corpus_cloned: bool,
    pub calibrated: bool,
}

/// Bootstrap the corpus on a fresh install (D52): clone the remote into
/// `home/memories`. Identity is NOT in the corpus — the fresh install-id is
/// minted separately (L1), so two installs share the mind but not the self.
/// An existing work tree is refused by name (bootstrap is for a fresh install;
/// the steady state is L4's sync).
pub fn bootstrap_corpus(home: &Path, remote: &str) -> Result<(), String> {
    if remote.trim().is_empty() {
        return Err("a corpus bootstrap needs a remote".to_string());
    }
    let root = crate::corpus::corpus_root(home);
    if root.join(".git").exists() {
        return Err(format!(
            "the corpus at {} already exists — bootstrap is for a fresh install",
            root.display()
        ));
    }
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("the corpus home: {e}"))?;
    }
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg(remote)
        .arg(&root)
        .output()
        .map_err(|e| format!("the corpus clone could not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "the corpus clone was refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    // Record the remote so the steady-state L4 sync rides it.
    let head = git_head(&root).ok();
    let state = crate::corpus::SyncState {
        remote: Some(remote.to_string()),
        corpus_ref: "master".to_string(),
        last_revision: head,
        settles: Vec::new(),
    };
    state
        .save(home)
        .map_err(|e| format!("the corpus state: {e}"))?;
    Ok(())
}

fn git_head(dir: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|e| format!("git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err("the corpus head could not be read".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Provision a local install (D51): lay out the home, mint a fresh install-id,
/// and bootstrap the corpus when a remote is named. The binary is the
/// self-contained payload (hard rule 12) — the caller drops it; this lays out
/// the home it runs in. Registration with the flagship is a separate gated act
/// (`register_fresh`), never implicit in arrival.
pub fn provision_local(home: &Path, remote: Option<&str>) -> Result<ProvisionReport, String> {
    for dir in crate::install::LAYOUT_DIRS {
        std::fs::create_dir_all(home.join(dir)).map_err(|e| format!("the layout `{dir}`: {e}"))?;
    }
    let install_id = crate::identity::install(home)?;
    let corpus_cloned = match remote {
        Some(remote) if !remote.trim().is_empty() => {
            bootstrap_corpus(home, remote)?;
            true
        }
        _ => false,
    };
    Ok(ProvisionReport {
        install_id: install_id.as_str().to_string(),
        corpus_cloned,
        calibrated: false,
    })
}

/// The SSH command sequence to provision a VPS (D51): drop the self-contained
/// binary from the release raw-binary channel, run install, bootstrap the
/// corpus, register. Credentials resolve out-of-band — the SSH agent / env
/// carries them, never the corpus or these commands. The sequence is pure
/// (testable construction); the caller executes it.
pub fn vps_commands(
    host: &str,
    user: &str,
    home: &str,
    payload_url: &str,
    remote: &str,
) -> Vec<String> {
    let bin = format!("{home}/bin/fleet");
    vec![
        format!("ssh {user}@{host} 'mkdir -p {home}/bin'"),
        format!("ssh {user}@{host} 'curl -fsSL {payload_url} -o {bin} && chmod +x {bin}'"),
        format!("ssh {user}@{host} '{bin} install'"),
        format!("ssh {user}@{host} '{bin} memories --sync --remote {remote}'"),
    ]
}

/// Register a fresh install with the flagship roster (D44). The install is
/// reported un-calibrated until it settles (D49) — registration earns a place
/// on the roster, never autonomy.
pub fn register_fresh(
    flagship_home: &Path,
    install_id: &str,
    sid: &str,
    version: &str,
    capacity_kib: u64,
    now_secs: u64,
) -> Result<crate::armada::roster::Instance, String> {
    crate::armada::roster::register(
        flagship_home,
        install_id,
        sid,
        version,
        capacity_kib,
        now_secs,
    )
}

/// The trust grade (D57): a fresh install is trusted only after it passes
/// doctor AND settles runs — graded by measurement, never by arrival. A fresh
/// install with no settles is untrusted even if doctor passes.
pub fn trusted(home: &Path, sid: &str, doctor_passed: bool) -> bool {
    doctor_passed && crate::armada::control::grade(home, sid).calibrated
}

/// The provision decision as the control plane meters it: a governance action
/// (D18/D19), so it clears step-up before it runs. The CLI's deploy verb admits
/// through the guard; this names the class for the frame.
pub fn provision_class() -> crate::security::ActionClass {
    crate::security::ActionClass::Governance
}

/// A provision journal event (D10): the durable record of a deployment, graded
/// against the fresh install's later settles.
pub fn journal_provision(home: &Path, install_id: &str, target: &str, corpus_cloned: bool) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "provision",
            Some(crate::identity::OWNER),
            Some(install_id),
            json!({"install": install_id, "target": target, "corpus_cloned": corpus_cloned}),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-deploy-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bare git remote carrying one fact — the mind a fresh install inherits.
    fn remote_with_fact(tag: &str) -> PathBuf {
        let remote = scratch(tag);
        let work = remote.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&work)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "--quiet", "-b", "master"]);
        run(&["config", "user.email", "test@fleet.local"]);
        run(&["config", "user.name", "fleet test"]);
        std::fs::write(
            work.join("a-fact.md"),
            "---\nname: a-fact\n---\nthe inherited mind\n",
        )
        .unwrap();
        run(&["add", "a-fact.md"]);
        run(&["commit", "--quiet", "-m", "a fact"]);
        // A bare clone to act as the remote.
        let bare = remote.join("remote.git");
        let out = std::process::Command::new("git")
            .arg("clone")
            .arg("--quiet")
            .arg("--bare")
            .arg(&work)
            .arg(&bare)
            .output()
            .unwrap();
        assert!(out.status.success());
        bare
    }

    /// C0: a local provision mints a fresh install-id and clones the corpus
    /// remote — inheriting the facts, not the identity.
    #[test]
    fn provision_inherits_the_mind_not_the_identity() {
        let remote = remote_with_fact("inherit");
        let home = scratch("local");
        let report = provision_local(&home, Some(remote.to_str().unwrap())).unwrap();
        assert!(report.corpus_cloned);
        assert!(!report.calibrated, "a fresh install is un-calibrated");
        assert_eq!(report.install_id.len(), 32, "a fresh 32-hex install-id");
        // The fact is inherited.
        assert!(crate::corpus::corpus_root(&home).join("a-fact.md").exists());
        // The identity is the fresh install's, minted locally — not in the corpus.
        assert_eq!(
            crate::identity::install(&home).unwrap().as_str(),
            report.install_id
        );
        // The remote is recorded so the steady-state sync rides it.
        assert_eq!(
            crate::corpus::SyncState::load(&home).remote.as_deref(),
            Some(remote.to_str().unwrap())
        );
        // The steady-state L4 sync now works on the cloned work tree.
        assert!(crate::corpus::sync(&home, None).is_ok());
    }

    /// C0: bootstrap refuses an existing work tree (it is for a fresh install).
    #[test]
    fn bootstrap_refuses_an_existing_corpus() {
        let remote = remote_with_fact("existing");
        let home = scratch("existing-home");
        bootstrap_corpus(&home, remote.to_str().unwrap()).unwrap();
        let err = bootstrap_corpus(&home, remote.to_str().unwrap()).unwrap_err();
        assert!(err.contains("already exists"), "{err}");
    }

    /// C1: a fresh install registers with the flagship roster and is reported
    /// un-calibrated until it settles.
    #[test]
    fn a_fresh_install_registers_un_calibrated() {
        let flagship = scratch("flagship");
        let instance = register_fresh(
            &flagship,
            "install-new",
            "sid-new",
            "0.6.38",
            1_000_000,
            1000,
        )
        .unwrap();
        assert_eq!(instance.sid, "sid-new");
        assert!(!instance.live(1000), "no heartbeat yet");
        // Un-calibrated until it settles.
        assert!(!crate::armada::control::grade(&flagship, "sid-new").calibrated);
        assert!(
            !trusted(&flagship, "sid-new", true),
            "doctor alone does not trust"
        );
    }

    /// C6: trust is earned by doctor AND settles — never by arrival.
    #[test]
    fn trust_requires_doctor_and_settles() {
        let home = scratch("trust");
        // Fresh: untrusted even with doctor.
        assert!(!trusted(&home, "sid-a", true));
        assert!(!trusted(&home, "sid-a", false));
        // Settle a dispatch: doctor + settles now trusts.
        let mut journal = crate::journal::Journal::open(&home).unwrap();
        journal
            .append(
                "dispatch",
                Some("owner"),
                Some("sid-a"),
                json!({"sid": "sid-a", "outcome": "finished"}),
            )
            .unwrap();
        assert!(trusted(&home, "sid-a", true));
        assert!(!trusted(&home, "sid-a", false), "doctor still required");
    }

    /// C0/C6: the VPS command sequence drops the self-contained payload, runs
    /// install, and syncs the corpus — credentials out-of-band (no secret in
    /// the commands).
    #[test]
    fn vps_commands_drop_the_payload_and_install() {
        let cmds = vps_commands(
            "vps.example",
            "fleet",
            "/opt/fleet",
            "https://example/fleet",
            "git@host:mind.git",
        );
        assert!(cmds
            .iter()
            .any(|c| c.contains("curl -fsSL https://example/fleet")));
        assert!(cmds
            .iter()
            .any(|c| c.contains("/opt/fleet/bin/fleet install")));
        assert!(cmds
            .iter()
            .any(|c| c.contains("memories --sync --remote git@host:mind.git")));
        // No credential material rides the commands (the SSH agent carries it).
        for c in &cmds {
            assert!(!c.contains("password"), "{c}");
        }
        assert_eq!(provision_class(), crate::security::ActionClass::Governance);
    }

    /// C0: a local provision without a remote lays out the home and mints the
    /// id, cloning nothing.
    #[test]
    fn provision_without_a_remote_lays_out_the_home() {
        let home = scratch("no-remote");
        let report = provision_local(&home, None).unwrap();
        assert!(!report.corpus_cloned);
        assert_eq!(report.install_id.len(), 32);
        assert!(home.join("sessions").exists());
        assert!(home.join("memories").exists());
    }
}
