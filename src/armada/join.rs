//! The instance boot: `fleet armada join` (the container's entry point,
//! 2026-11-19).
//!
//! An instance is a container running this binary. Armada deploys it; it
//! then has to become a *usable* member of the fleet with no human at a
//! terminal — no prompts, no interactive install. Everything it needs
//! arrives as environment (the platform's own mechanism) and everything it
//! writes lands under `FLEET_HOME`, which the platform mounts as one named
//! volume.
//!
//! The order is the contract, and it is the responsible-citizen shape (D3):
//!
//! 1. **Read and validate the environment.** A missing variable is a NAMED
//!    refusal that exits non-zero *before any write* — a half-configured
//!    instance that starts anyway is worse than one that refuses, because
//!    its roster row would look healthy.
//! 2. **Bootstrap the corpus** from the shared remote. The fresh-install
//!    guard already in `deploy::bootstrap_corpus` refuses when `.git`
//!    exists, and that refusal is what makes a RESTARTED container
//!    idempotent: the second boot keeps the corpus it already has instead
//!    of re-cloning over local work.
//! 3. **Mint-or-read the install-id.** `identity::install` is idempotent by
//!    the same discipline: it reads the persisted id and only mints when
//!    there is none, so a restart keeps its identity.
//! 4. **Register on the roster**, which earns a place and nothing more: the
//!    instance is reported un-calibrated until it settles dispatches (D49).
//! 5. **Heartbeat on a timer**, which is what earns liveness (D44). An
//!    unknown sid cannot heartbeat, so a boot that skipped the join has no
//!    way to appear live.

use std::path::{Path, PathBuf};

use crate::armada::deploy;
use crate::armada::roster;
use crate::identity;

/// The variable naming the shared corpus's git remote. Required: an
/// instance with no corpus would recall from an empty home, which reads as
/// "this fleet knows nothing" rather than as a misconfiguration.
pub const CORPUS_REMOTE_ENV: &str = "FLEET_CORPUS_REMOTE";

/// The variable carrying the fleet's sid, so the roster row is addressable
/// before the first heartbeat.
pub const FLEET_SID_ENV: &str = "FLEET_SID";

/// How long to wait between heartbeats, in seconds. Defaults to 60 — well
/// inside the roster's liveness window, so one dropped beat does not make a
/// healthy instance look dead.
pub const HEARTBEAT_SECS_ENV: &str = "FLEET_HEARTBEAT_SECS";

/// The shared SKILLS library's git remote. Optional but load-bearing: the
/// owner's ruling (2026-11-19) is that an instance must be "as smart as the
/// other ones" out of the gate, and the canonical skills library is where
/// that intelligence lives — 460+ skills, its own git repo, 3287 tracked
/// files. An instance that boots without it is a capable agent with none of
/// the house's procedural knowledge, which is the difference between a new
/// machine joining the fleet and a new machine merely existing.
pub const SKILLS_REMOTE_ENV: &str = "FLEET_SKILLS_REMOTE";

/// What the boot found in its environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinSpec {
    /// The shared corpus's git remote.
    pub corpus_remote: String,
    /// The shared skills library's git remote, when one is configured. `None`
    /// is an explicit "this fleet shares facts only": the boot then reports
    /// that the instance is running on its own skills rather than pretending
    /// it inherited the house's.
    pub skills_remote: Option<String>,
    /// This install's sid on the roster.
    pub sid: String,
    /// The heartbeat interval.
    pub heartbeat_secs: u64,
}

/// What a join did, for the log and the console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinReport {
    /// This install's id.
    pub install_id: String,
    /// This install's sid.
    pub sid: String,
    /// Whether the corpus was cloned by this boot (false on a restart).
    pub corpus_cloned: bool,
    /// Whether the skills library was cloned by this boot (false on a
    /// restart, or when no skills remote is configured).
    pub skills_cloned: bool,
    /// How many skills the instance can load after the boot — the number the
    /// "as smart as the others" claim is measured by.
    pub skills_loaded: usize,
    /// The roster row's registered-at stamp.
    pub registered_at: u64,
}

/// Read the environment into a [`JoinSpec`], refusing by name on anything
/// missing. This runs FIRST and writes nothing: every refusal below leaves
/// the home exactly as it found it.
pub fn read_env(lookup: &dyn Fn(&str) -> Option<String>) -> Result<JoinSpec, String> {
    let read = |name: &str| {
        lookup(name)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let corpus_remote = read(CORPUS_REMOTE_ENV).ok_or_else(|| {
        format!(
            "refused: {CORPUS_REMOTE_ENV} is unset — an instance inherits the fleet's corpus \
             from a git remote, and one with no remote would start empty and look misconfigured \
             rather than joined"
        )
    })?;
    // The sid may legitimately be absent on a first boot: the container's
    // own name is the natural default, and the roster row is addressable
    // once it exists. Refusing here would make every deploy require a
    // second variable for no gain.
    let sid = read(FLEET_SID_ENV).unwrap_or_else(|| {
        let host = std::env::var("HOSTNAME").unwrap_or_default();
        let host = host.trim();
        if host.is_empty() {
            "instance".to_string()
        } else {
            host.to_string()
        }
    });
    let heartbeat_secs = match read(HEARTBEAT_SECS_ENV) {
        Some(raw) => raw.parse::<u64>().map_err(|_| {
            format!("refused: {HEARTBEAT_SECS_ENV}=`{raw}` is not a whole number of seconds")
        })?,
        None => 60,
    };
    if heartbeat_secs == 0 {
        return Err(format!(
            "refused: {HEARTBEAT_SECS_ENV}=0 would never beat, and liveness is earned by \
             heartbeats (D44)"
        ));
    }
    // The skills remote is optional but its ABSENCE is reported, not hidden:
    // an instance with no skills library is less capable than its siblings,
    // and the boot line says so.
    let skills_remote = read(SKILLS_REMOTE_ENV);
    Ok(JoinSpec {
        corpus_remote,
        skills_remote,
        sid,
        heartbeat_secs,
    })
}

/// The environment as the boot reads it: the real process environment.
pub fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Boot this install as a fleet instance. Idempotent by construction: a
/// second call over a home that already joined keeps the corpus and the
/// install-id and simply re-registers and resumes beating.
pub fn join(home: &Path, joining: &JoinSpec, now_secs: u64) -> Result<JoinReport, String> {
    // 2. The corpus first: the install-id and everything the run recalls
    //    come from it, and a failed clone must stop the boot before a
    //    roster row promises a corpus that is not there.
    let corpus_cloned = match deploy::bootstrap_corpus(home, &joining.corpus_remote) {
        Ok(()) => true,
        Err(refusal) if refusal.contains("already exists") => {
            // A restart: the corpus is this instance's own now. Keep it.
            false
        }
        Err(refusal) => return Err(refusal),
    };
    // 3. The identity: read-or-mint, never re-mint (D7).
    let install_id = identity::install(home).map_err(|error| format!("the install-id: {error}"))?;
    // 3b. The SKILLS library (the owner's ruling, 2026-11-19). The canonical
    //     library is its own git repo, so an instance inherits the house's
    //     procedural knowledge the same way it inherits the facts — by
    //     cloning. The destination is the path fleet already resolves
    //     (`$FLEET_SKILLS_DIR`, else `<user home>/.agents/skills`), so no
    //     second discovery rule is invented for containers.
    let skills_cloned = match joining.skills_remote.as_deref() {
        Some(remote) if !remote.trim().is_empty() => {
            let dest = skills_dir(home);
            if dest.join(".git").exists() {
                // A restart: the library is this instance's own now, and
                // re-cloning would discard any local authoring.
                false
            } else {
                clone_skills(remote, &dest)?;
                true
            }
        }
        _ => false,
    };
    let skills_loaded = crate::skills::load(&skills_dir(home)).len();

    // 4. The roster row. Registration earns a place, never autonomy (D49):
    //    `roster::register` reports the instance un-calibrated until it
    //    settles (D44's grade reads the journal).
    let version = env!("CARGO_PKG_VERSION").to_string();
    let capacity_kib = 0;
    let instance = roster::register(
        home,
        install_id.as_str(),
        &joining.sid,
        &version,
        capacity_kib,
        now_secs,
    )?;
    Ok(JoinReport {
        install_id: install_id.as_str().to_string(),
        sid: joining.sid.clone(),
        corpus_cloned,
        skills_cloned,
        skills_loaded,
        registered_at: instance.registered_at,
    })
}

/// Where the skills library lives for this home: `$FLEET_SKILLS_DIR` when the
/// operator names one, else the same path `skills::resolve_skills_dir` reads —
/// `<user home>/.agents/skills`. One rule, so a container and a laptop agree
/// about where a skill comes from.
pub fn skills_dir(home: &Path) -> PathBuf {
    if let Ok(named) = std::env::var("FLEET_SKILLS_DIR") {
        let named = named.trim();
        if !named.is_empty() {
            return PathBuf::from(named);
        }
    }
    let user_home = std::env::var("HOME").unwrap_or_else(|_| {
        home.to_string_lossy().into_owned()
    });
    PathBuf::from(user_home).join(".agents").join("skills")
}

/// Clone the shared skills library. Refused by name on failure, and a
/// partial clone is removed rather than left as a half-library the loader
/// would silently read.
fn clone_skills(remote: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("the skills home: {error}"))?;
    }
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg(remote)
        .arg(dest)
        .output()
        .map_err(|error| format!("the skills clone could not run: {error}"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = std::fs::remove_dir_all(dest);
        return Err(format!(
            "the skills library could not be cloned from `{remote}`: {said} — an instance \
             with no skills is less capable than its siblings, so this is refused rather \
             than booted past"
        ));
    }
    Ok(())
}

/// Beat once. Split from the loop so a test drives one beat without a
/// thread, and so the loop itself has no logic to get wrong.
pub fn beat(home: &Path, sid: &str, now_secs: u64) -> Result<(), String> {
    roster::heartbeat(home, sid, now_secs)
}

/// The boot's log line, printed by the CLI so the operator's `docker logs`
/// answers "is it joined, and did it re-clone?" without a second command.
pub fn report(joined: &JoinReport) -> String {
    let skills = if joined.skills_cloned {
        format!("skills_cloned=true ({} loaded)", joined.skills_loaded)
    } else if joined.skills_loaded > 0 {
        format!("skills_cloned=false ({} already local)", joined.skills_loaded)
    } else {
        "skills=0 — this instance runs on its own skills, not the fleet's".to_string()
    };
    format!(
        "armada join: sid={} install={} corpus_cloned={} {} registered_at={} — un-calibrated \
         until it settles dispatches",
        joined.sid,
        joined.install_id,
        joined.corpus_cloned,
        skills,
        joined.registered_at
    )
}

/// The home a container's boot writes to, from `FLEET_HOME` (which the image
/// sets to `/data`) or the ordinary resolution. Kept here so the CLI and a
/// test agree on one answer.
pub fn home_from_env(lookup: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let home = lookup("FLEET_HOME")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "refused: FLEET_HOME is unset — an instance's whole state is one mounted volume, \
             and a boot that guessed would scatter it"
                .to_string()
        })?;
    Ok(PathBuf::from(home))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fleet-join-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name: &str| {
            owned
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn a_missing_corpus_remote_is_refused_by_name() {
        let refused = read_env(&env_of(&[])).expect_err("refused");
        assert!(refused.contains("FLEET_CORPUS_REMOTE"), "{refused}");
        assert!(
            refused.contains("look misconfigured rather than joined"),
            "the refusal names the consequence: {refused}"
        );
    }

    #[test]
    fn the_sid_defaults_to_the_host_and_heartbeat_defaults_to_sixty() {
        let spec = read_env(&env_of(&[(CORPUS_REMOTE_ENV, "git@example:fleet.git")])).unwrap();
        assert_eq!(spec.corpus_remote, "git@example:fleet.git");
        assert!(!spec.sid.is_empty(), "the sid falls back to the host name");
        assert_eq!(spec.heartbeat_secs, 60);
    }

    #[test]
    fn a_zero_heartbeat_is_refused() {
        let refused = read_env(&env_of(&[
            (CORPUS_REMOTE_ENV, "git@example:fleet.git"),
            (HEARTBEAT_SECS_ENV, "0"),
        ]))
        .expect_err("refused");
        assert!(
            refused.contains("liveness is earned by heartbeats"),
            "{refused}"
        );
    }

    #[test]
    fn a_non_numeric_heartbeat_is_refused_by_name() {
        let refused = read_env(&env_of(&[
            (CORPUS_REMOTE_ENV, "git@example:fleet.git"),
            (HEARTBEAT_SECS_ENV, "soon"),
        ]))
        .expect_err("refused");
        assert!(refused.contains("not a whole number of seconds"), "{refused}");
    }

    #[test]
    fn an_unset_home_is_refused() {
        let refused = home_from_env(&env_of(&[])).expect_err("refused");
        assert!(refused.contains("FLEET_HOME"), "{refused}");
    }

    #[test]
    fn a_join_registers_the_instance_and_a_second_join_keeps_its_identity() {
        let home = scratch("twice");
        // A local corpus remote: a bare repo with one commit, so the clone
        // is a real git clone rather than a stub.
        let remote = home.join("remote.git");
        let seed = home.join("seed");
        std::fs::create_dir_all(&seed).unwrap();
        let run = |args: &[&str], cwd: &Path| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .expect("git runs")
        };
        run(&["init", "--bare", "-q", remote.to_str().unwrap()], &home);
        run(&["init", "-q"], &seed);
        std::fs::write(seed.join("a-fact.md"), "---\nname: a-fact\n---\n\nx\n").unwrap();
        run(&["add", "."], &seed);
        run(
            &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "seed"],
            &seed,
        );
        run(&["remote", "add", "origin", remote.to_str().unwrap()], &seed);
        run(&["push", "-q", "origin", "HEAD:master"], &seed);

        let instance = scratch("instance");
        let spec = JoinSpec {
            corpus_remote: remote.to_str().unwrap().to_string(),
            skills_remote: None,
            sid: "box-1".to_string(),
            heartbeat_secs: 60,
        };
        let first = join(&instance, &spec, 100).expect("joined");
        assert!(first.corpus_cloned, "the first boot clones the corpus");
        assert_eq!(first.sid, "box-1");
        let roster_after = roster::load(&instance);
        assert!(roster_after.instances.contains_key("box-1"));
        assert!(
            !roster_after.instances["box-1"].live(100),
            "un-calibrated, and not live yet"
        );

        // The restart: same home, second boot.
        let second = join(&instance, &spec, 200).expect("rejoined");
        assert!(!second.corpus_cloned, "a restart keeps the corpus it has");
        assert_eq!(
            second.install_id, first.install_id,
            "the install-id is never re-minted"
        );
        // And it can beat, which is what earns liveness.
        beat(&instance, "box-1", 260).expect("beats");
        assert!(roster::load(&instance).instances["box-1"].live(260));
        std::fs::remove_dir_all(&home).ok();
        std::fs::remove_dir_all(&instance).ok();
    }

    #[test]
    fn a_join_clones_the_shared_skills_library_and_counts_what_it_got() {
        let home = scratch("skills-home");
        // A corpus remote (the join needs one) and a skills remote (the point
        // of this test), both real bare repos with real commits.
        let seed = |name: &str, file: &str, body: &str| -> PathBuf {
            let work = home.join(format!("{name}-seed"));
            std::fs::create_dir_all(&work).unwrap();
            let bare = home.join(format!("{name}.git"));
            let run = |args: &[&str], cwd: &Path| {
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(cwd)
                    .output()
                    .expect("git runs")
            };
            run(&["init", "--bare", "-q", bare.to_str().unwrap()], &home);
            run(&["init", "-q"], &work);
            std::fs::write(work.join(file), body).unwrap();
            run(&["add", "."], &work);
            run(
                &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "seed"],
                &work,
            );
            run(&["remote", "add", "origin", bare.to_str().unwrap()], &work);
            run(&["push", "-q", "origin", "HEAD:master"], &work);
            bare
        };
        let corpus = seed("corpus", "a-fact.md", "---\nname: a-fact\n---\n\nx\n");
        // A skill is a DIRECTORY with a SKILL.md — the loader's own shape.
        let skills_work = home.join("skills-seed");
        std::fs::create_dir_all(skills_work.join("a-skill")).unwrap();
        std::fs::write(
            skills_work.join("a-skill").join("SKILL.md"),
            "---\nname: a-skill\ndescription: a skill the fleet shares\n---\n\nbody\n",
        )
        .unwrap();
        let skills_bare = home.join("skills.git");
        let run = |args: &[&str], cwd: &Path| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .expect("git runs")
        };
        run(&["init", "--bare", "-q", skills_bare.to_str().unwrap()], &home);
        run(&["init", "-q"], &skills_work);
        run(&["add", "."], &skills_work);
        run(
            &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "seed"],
            &skills_work,
        );
        run(&["remote", "add", "origin", skills_bare.to_str().unwrap()], &skills_work);
        run(&["push", "-q", "origin", "HEAD:master"], &skills_work);

        let instance = scratch("skills-instance");
        let dest = instance.join("skills-canonical");
        // SAFETY: this test owns the variable for its duration and names a
        // path unique to its own scratch home.
        unsafe { std::env::set_var("FLEET_SKILLS_DIR", &dest) };
        let spec = JoinSpec {
            corpus_remote: corpus.to_str().unwrap().to_string(),
            skills_remote: Some(skills_bare.to_str().unwrap().to_string()),
            sid: "box-smart".to_string(),
            heartbeat_secs: 60,
        };
        let joined = join(&instance, &spec, 500).expect("joined");
        unsafe { std::env::remove_var("FLEET_SKILLS_DIR") };

        assert!(joined.skills_cloned, "the boot cloned the shared library");
        assert_eq!(joined.skills_loaded, 1, "the instance can load the skill: {}", report(&joined));
        assert!(
            report(&joined).contains("skills_cloned=true"),
            "the boot line says the instance is as smart as its siblings: {}",
            report(&joined)
        );
        assert!(dest.join("a-skill").join("SKILL.md").exists());

        // A restart must NOT re-clone: the library is local authoring now.
        unsafe { std::env::set_var("FLEET_SKILLS_DIR", &dest) };
        let again = join(&instance, &spec, 600).expect("rejoined");
        unsafe { std::env::remove_var("FLEET_SKILLS_DIR") };
        assert!(!again.skills_cloned, "a restart keeps its library");
        assert_eq!(again.skills_loaded, 1, "and still loads it");

        std::fs::remove_dir_all(&home).ok();
        std::fs::remove_dir_all(&instance).ok();
    }

    #[test]
    fn a_boot_with_no_skills_remote_says_so_rather_than_implying_it_inherited() {
        let joined = JoinReport {
            install_id: "abc".to_string(),
            sid: "box-alone".to_string(),
            corpus_cloned: true,
            skills_cloned: false,
            skills_loaded: 0,
            registered_at: 1,
        };
        let said = report(&joined);
        assert!(
            said.contains("runs on its own skills, not the fleet's"),
            "the absence is named: {said}"
        );
    }

    #[test]
    fn a_heartbeat_for_an_unknown_sid_is_refused() {
        let home = scratch("unknown");
        let refused = beat(&home, "ghost", 1).expect_err("refused");
        assert!(refused.contains("ghost"), "{refused}");
        std::fs::remove_dir_all(&home).ok();
    }
}
