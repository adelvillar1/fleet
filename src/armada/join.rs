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
use serde_json::{json, Value};

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

/// The shared TOOL REGISTRY's git remote (armada §4's third sync-ready
/// artifact, and L4's last deferred class). `mcp.json` holds server
/// DEFINITIONS only — a program, its args, and the env NAMES a server needs,
/// with no field a credential could live in — so the file is safe to share.
/// What is NOT shared is enablement and the probe history: whether *this*
/// instance runs a server, and when it last answered, are local judgements.
/// A new instance therefore inherits the fleet's tools switched ON in
/// principle and off in fact, and its owner turns on what this machine
/// should run.
pub const MCP_REMOTE_ENV: &str = "FLEET_MCP_REMOTE";

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
    /// The shared tool registry's git remote, when one is configured.
    pub mcp_remote: Option<String>,
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
    /// Server definitions the instance inherited from the shared registry.
    pub servers_inherited: usize,
    /// Of those, how many this instance already had (a restart).
    pub servers_added: usize,
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
    let mcp_remote = read(MCP_REMOTE_ENV);
    Ok(JoinSpec {
        corpus_remote,
        skills_remote,
        mcp_remote,
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

    // 3c. The shared TOOL REGISTRY (armada §4's third artifact; L4's last
    //     deferred class). The file is a document of server DEFINITIONS —
    //     program, args, env names, no value — so it travels safely. The
    //     merge is deliberately asymmetric: a definition this instance lacks
    //     is ADDED disabled (the owner decides what runs here), and a
    //     definition the instance already has keeps ITS enablement and probe
    //     history, because those are local judgements about this machine.
    let (servers_inherited, servers_added) = match joining.mcp_remote.as_deref() {
        Some(remote) if !remote.trim().is_empty() => inherit_servers(home, remote)?,
        _ => (0, 0),
    };

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
        servers_inherited,
        servers_added,
        registered_at: instance.registered_at,
    })
}

/// Pull the shared tool registry and merge its DEFINITIONS into this home's
/// `mcp.json`. Returns `(definitions the shared registry carries, how many
/// this instance did not have)`.
///
/// Three rules, each load-bearing:
///
/// 1. **Definitions travel; enablement does not.** A newly inherited server
///    lands `enabled: false`. Silently switching on a server an operator has
///    never seen would let a shared file change what an instance runs — the
///    confused deputy at the registry layer.
/// 2. **A local entry wins on the local fields.** If this instance already
///    knows a server, its `enabled` and `last_probe` survive untouched: they
///    describe this machine, not the fleet.
/// 3. **Values never travel, and nothing here could leak one.** The schema
///    has no field for a credential — only `env_names` — so the clone is
///    safe to keep on a volume and the merge needs no redaction pass.
pub fn inherit_servers(home: &Path, remote: &str) -> Result<(usize, usize), String> {
    let scratch = std::env::temp_dir().join(format!(
        "fleet-mcp-inherit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|span| span.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let clone = std::process::Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg("--depth")
        .arg("1")
        .arg(remote)
        .arg(&scratch)
        .output()
        .map_err(|error| format!("the tool registry clone could not run: {error}"))?;
    if !clone.status.success() {
        let said = String::from_utf8_lossy(&clone.stderr).trim().to_string();
        let _ = std::fs::remove_dir_all(&scratch);
        return Err(format!(
            "the shared tool registry could not be cloned from `{remote}`: {said} — an \
             instance that silently kept only its own tools would look configured while \
             missing the fleet's, so this is refused rather than booted past"
        ));
    }
    // The shared file may sit at the repo root or under a `mcp/` directory.
    let shared_path = [scratch.join("mcp.json"), scratch.join("mcp").join("mcp.json")]
        .into_iter()
        .find(|path| path.exists());
    let Some(shared_path) = shared_path else {
        let _ = std::fs::remove_dir_all(&scratch);
        return Err(format!(
            "the shared tool registry at `{remote}` carries no mcp.json — the repo must \
             hold the registry document"
        ));
    };
    let shared: Value = serde_json::from_str(
        &std::fs::read_to_string(&shared_path)
            .map_err(|error| format!("the shared registry is unreadable: {error}"))?,
    )
    .map_err(|error| format!("the shared registry is not valid JSON: {error}"))?;
    let shared_servers = shared
        .get("servers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let inherited = shared_servers.len();

    let local_path = crate::mcp_registry::servers_path(home);
    let mut local: Value = match std::fs::read_to_string(&local_path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| format!("{} is not valid JSON: {error}", local_path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(error) => return Err(format!("{} is unreadable: {error}", local_path.display())),
    };
    if !local.is_object() {
        local = json!({});
    }
    let mut added = 0usize;
    let mut changed = false;
    {
        let object = local
            .as_object_mut()
            .expect("checked as an object immediately above");
        let servers = object
            .entry("servers")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| format!("{}'s `servers` is not an object", local_path.display()))?;
        for (name, definition) in shared_servers {
            if servers.contains_key(&name) {
                // Rule 2: a known server keeps this machine's enablement and
                // probe history. Only the DEFINITION fields are refreshed, so
                // an upstream fix to args or program reaches the instance.
                if let (Some(local_entry), Some(shared_entry)) = (
                    servers.get_mut(&name).and_then(Value::as_object_mut),
                    definition.as_object(),
                ) {
                    for field in ["program", "args", "env_names"] {
                        if let Some(value) = shared_entry.get(field) {
                            // Only a real difference counts as a change: an
                            // unchanged refresh must not rewrite the file on
                            // every boot.
                            if local_entry.get(field) != Some(value) {
                                local_entry.insert(field.to_string(), value.clone());
                                changed = true;
                            }
                        }
                    }
                }
                continue;
            }
            // Rule 1: a server this instance has never seen arrives OFF.
            let mut entry = definition.as_object().cloned().unwrap_or_default();
            entry.insert("enabled".to_string(), Value::Bool(false));
            entry.insert(
                "source".to_string(),
                json!({"harness": "fleet-shared", "imported": crate::calib::today()}),
            );
            servers.insert(name, Value::Object(entry));
            added += 1;
            changed = true;
        }
    }
    // Write when anything actually changed: a pure field refresh must
    // persist too, which the first version of this got wrong — it gated the
    // write on `added > 0`, so an upstream definition fix was computed and
    // discarded. The test that caught it is why this reads as it does.
    if changed || !local_path.exists() {
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("the registry home: {error}"))?;
        }
        let text = serde_json::to_string_pretty(&local)
            .map_err(|error| format!("the registry could not be written: {error}"))?;
        std::fs::write(&local_path, text)
            .map_err(|error| format!("{} could not be written: {error}", local_path.display()))?;
    }
    let _ = std::fs::remove_dir_all(&scratch);
    Ok((inherited, added))
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
    // Tools are reported as definitions, never as "enabled": an inherited
    // server arrives off, and saying otherwise would promise a capability
    // this instance has not turned on.
    let tools = if joined.servers_inherited == 0 {
        "tools=0 inherited".to_string()
    } else if joined.servers_added > 0 {
        format!(
            "tools: {} inherited, {} new (added disabled)",
            joined.servers_inherited, joined.servers_added
        )
    } else {
        format!(
            "tools: {} inherited, all already known",
            joined.servers_inherited
        )
    };
    format!(
        "armada join: sid={} install={} corpus_cloned={} {} {} registered_at={} — un-calibrated \
         until it settles dispatches",
        joined.sid,
        joined.install_id,
        joined.corpus_cloned,
        skills,
        tools,
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
            mcp_remote: None,
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
            mcp_remote: None,
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
            servers_inherited: 0,
            servers_added: 0,
            registered_at: 1,
        };
        let said = report(&joined);
        assert!(
            said.contains("runs on its own skills, not the fleet's"),
            "the absence is named: {said}"
        );
    }

    /// A bare repo holding one mcp.json, ready to clone.
    fn registry_remote(root: &Path, name: &str, body: &str) -> PathBuf {
        let work = root.join(format!("{name}-work"));
        let bare = root.join(format!("{name}.git"));
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("mcp.json"), body).unwrap();
        let run = |args: &[&str], cwd: &Path| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .expect("git runs")
        };
        run(&["init", "--bare", "-q", bare.to_str().unwrap()], root);
        run(&["init", "-q"], &work);
        run(&["add", "."], &work);
        run(
            &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "seed"],
            &work,
        );
        run(&["remote", "add", "origin", bare.to_str().unwrap()], &work);
        run(&["push", "-q", "origin", "HEAD:master"], &work);
        bare
    }

    #[test]
    fn an_inherited_server_definition_arrives_disabled() {
        let root = scratch("mcp-inherit");
        let remote = registry_remote(
            &root,
            "shared",
            r#"{"servers":{"cloudflare":{"program":"npx","args":["-y","mcp-remote@latest"],
                "env_names":[],"enabled":true}}}"#,
        );
        let home = scratch("mcp-home");
        let (inherited, added) = inherit_servers(&home, remote.to_str().unwrap()).expect("merged");
        assert_eq!(inherited, 1, "the shared registry carries one server");
        assert_eq!(added, 1, "this instance had none");
        let servers = crate::mcp_registry::read_servers(&home).expect("the merged registry parses");
        let (name, entry) = servers.first().expect("the server landed");
        assert_eq!(name, "cloudflare");
        // Rule 1: a server this instance has never seen arrives OFF, even
        // though the shared file said enabled:true.
        assert!(
            !entry.enabled,
            "an inherited definition must not switch itself on"
        );
        assert_eq!(entry.program, "npx");
        assert_eq!(entry.args, vec!["-y", "mcp-remote@latest"]);
        assert_eq!(
            entry.source.as_ref().map(|source| source.harness.as_str()),
            Some("fleet-shared"),
            "the provenance names the shared registry"
        );
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_local_servers_enablement_and_probe_survive_the_merge() {
        let root = scratch("mcp-local");
        let remote = registry_remote(
            &root,
            "shared2",
            r#"{"servers":{"cloudflare":{"program":"npx","args":["-y","v2"],
                "env_names":[],"enabled":false}}}"#,
        );
        let home = scratch("mcp-local-home");
        // This instance already runs the server, with a probe on record.
        std::fs::write(
            crate::mcp_registry::servers_path(&home),
            r#"{"servers":{"cloudflare":{"program":"npx","args":["-y","v1"],
                "env_names":[],"enabled":true,
                "last_probe":{"at":"2026-11-19T00:00Z","ok":true,"reason":"",
                  "server_name":"cloudflare-api","server_version":"1","tools":4}}}}"#,
        )
        .unwrap();
        let (inherited, added) =
            inherit_servers(&home, remote.to_str().unwrap()).expect("merged");
        assert_eq!((inherited, added), (1, 0), "nothing new to add");
        let (_, entry) = crate::mcp_registry::read_servers(&home)
            .expect("parses")
            .into_iter()
            .next()
            .expect("still there");
        // Rule 2: enablement and probe are THIS machine's judgements.
        assert!(entry.enabled, "a local enablement survived");
        assert!(entry.last_probe.is_some(), "the probe history survived");
        // ...and the definition fields refreshed from upstream.
        assert_eq!(entry.args, vec!["-y", "v2"], "the definition updated");
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_shared_registry_with_no_document_is_refused_by_name() {
        let root = scratch("mcp-empty");
        let remote = registry_remote(&root, "empty", "{}");
        // No mcp.json: the repo holds something else entirely.
        let work = root.join("empty-work");
        std::fs::remove_file(work.join("mcp.json")).unwrap();
        std::fs::write(work.join("README.md"), "not a registry").unwrap();
        let run = |args: &[&str], cwd: &Path| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .expect("git runs")
        };
        run(&["add", "-A"], &work);
        run(
            &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "swap"],
            &work,
        );
        run(&["push", "-q", "origin", "HEAD:master"], &work);
        let home = scratch("mcp-empty-home");
        let refused = inherit_servers(&home, remote.to_str().unwrap()).expect_err("refused");
        assert!(refused.contains("carries no mcp.json"), "{refused}");
        assert!(
            refused.contains("must hold the registry document"),
            "the refusal says what to do: {refused}"
        );
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_heartbeat_for_an_unknown_sid_is_refused() {
        let home = scratch("unknown");
        let refused = beat(&home, "ghost", 1).expect_err("refused");
        assert!(refused.contains("ghost"), "{refused}");
        std::fs::remove_dir_all(&home).ok();
    }
}
