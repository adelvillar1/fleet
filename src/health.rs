//! The health probes: what is down, since when, and the repair.
//!
//! The failsafe's first half (the failsafe-restore plan's phase A): a
//! loop that knows a service is down and says so, instead of the
//! substrate's death surfacing only when a delegation needs it
//! (KI-493). One pass builds a [`HealthReport`]; every
//! [`ServiceHealth`] names the service, its state, the cause when
//! down, and the repair map's verb — the word the face paints and the
//! main session acts on. The probes are deterministic and cheap — a
//! connect, a file's presence, one process spawn at the slow tier's
//! cadence — so a pass costs milliseconds and never spawns an MCP
//! server; a lane's full health probe stays where it was, on demand.

use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::time::Duration;

/// How often the server's loop probes the fast tier.
pub const FAST_INTERVAL: Duration = Duration::from_secs(30);
/// The slow tier rides every other pass (docker's daemon spawn is a
/// process launch, not a connect).
pub const SLOW_EVERY_NTH_PASS: u64 = 2;

/// The loopback connect timeout for a fast probe.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(750);

/// One service's verdict, in the report's own vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ServiceHealth {
    pub name: &'static str,
    pub up: bool,
    /// The cause when down — the note the face paints and the repair
    /// note quotes. Empty when up.
    #[serde(default)]
    pub note: String,
    /// The repair map's verb: what brings this service back.
    pub repair: &'static str,
}

impl ServiceHealth {
    fn up(name: &'static str) -> Self {
        Self {
            name,
            up: true,
            note: String::new(),
            repair: repair_for(name),
        }
    }
    fn down(name: &'static str, note: impl Into<String>) -> Self {
        Self {
            name,
            up: false,
            note: note.into(),
            repair: repair_for(name),
        }
    }
}

/// One pass over the probe set. `fast_only` skips the slow tier's
/// process spawn (docker) — the loop alternates so a tick never pays
/// for both.
pub fn probe(home: &Path, fast_only: bool) -> HealthReport {
    let mut services = vec![layerfs_probe(home), api_probe(home)];
    if !fast_only {
        services.push(docker_probe());
        services.push(lanes_probe());
        // The auto-update channel's supply: only where a checkout
        // resolves — an installed machine without the repo has
        // nothing to cut, and names nothing.
        if let Some(release) = release_probe() {
            services.push(release);
        }
    }
    HealthReport {
        services,
        stamp: crate::calib::today(),
    }
}

/// The layerfs substrate: a connect to the worker's recorded port.
/// The machine read is the first seam (KI-335's run-scoped home);
/// its absence falls back to the conventional endpoint path, and
/// only then reads as not provisioned — named with the machine's own
/// reason, not guessed.
fn layerfs_probe(home: &Path) -> ServiceHealth {
    match recorded_endpoint_port(home) {
        Ok(port) => layerfs_probe_with(port, worker_container_port()),
        Err(health) => health,
    }
}

/// The endpoint record's port, or the early-down verdict the probe
/// carries when the record itself is the problem: not provisioned,
/// unreadable, or naming no port.
fn recorded_endpoint_port(home: &Path) -> Result<u16, ServiceHealth> {
    let machine = crate::layerfs_tools::read_machine_at(home);
    let endpoint = match machine.as_ref() {
        Ok(m) => m.home.join("worker").join("endpoint.json"),
        Err(reason) => {
            let conventional = home
                .join("layerfs-home")
                .join("worker")
                .join("endpoint.json");
            if !conventional.exists() {
                return Err(ServiceHealth::down(
                    "layerfs",
                    format!("not provisioned — {reason}"),
                ));
            }
            conventional
        }
    };
    let Ok(text) = std::fs::read_to_string(&endpoint) else {
        return Err(ServiceHealth::down(
            "layerfs",
            format!("the endpoint file is unreadable: {}", endpoint.display()),
        ));
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("port").and_then(|p| p.as_u64()))
        .map(|p| p as u16)
        .ok_or_else(|| {
            ServiceHealth::down("layerfs", "the endpoint file names no port — install again")
        })
}

/// The stale-record repair (the owner's 2026-11-14 ruling: the light
/// path first). When the recorded port is refused and the running
/// container answers on a different one, rewrite the endpoint record's
/// port to the container's actual and report reconciled — seconds, no
/// re-provision. `false` when the shape does not apply (the record
/// answers, the container is gone, or the container agrees with the
/// record) and the caller should run the full provision leg.
pub(crate) fn reconcile_stale_record(home: &Path) -> bool {
    reconcile_stale_record_with(home, worker_container_port())
}

/// The decision with the container's port injected, so the branches are
/// testable without Docker.
fn reconcile_stale_record_with(home: &Path, actual: Option<u16>) -> bool {
    let recorded = match recorded_endpoint_port(home) {
        Ok(port) => port,
        Err(_) => return false,
    };
    let Some(actual) = actual else {
        return false;
    };
    if actual == recorded {
        return false;
    }
    let machine = crate::layerfs_tools::read_machine_at(home);
    let endpoint = match machine.as_ref() {
        Ok(m) => m.home.join("worker").join("endpoint.json"),
        Err(_) => home
            .join("layerfs-home")
            .join("worker")
            .join("endpoint.json"),
    };
    let Some(mut document) = std::fs::read_to_string(&endpoint)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.as_object().cloned())
    else {
        return false;
    };
    document.insert("port".to_string(), serde_json::json!(actual));
    let Some(text) = serde_json::to_string_pretty(&document).ok() else {
        return false;
    };
    std::fs::write(&endpoint, text).is_ok()
}

fn layerfs_probe_with(port: u16, container: Option<u16>) -> ServiceHealth {
    if crate::net::listening("127.0.0.1", port, CONNECT_TIMEOUT) {
        ServiceHealth::up("layerfs")
    } else {
        let note = match container {
            // The worker runs, the record does not follow it: a machine
            // restart reassigns Docker's published port, and the endpoint
            // file cannot rewrite itself. The diagnosis names both ports
            // — "the substrate is down" would be a lie about a worker
            // that is up (the owner's 2026-11-13 report: the worker ran,
            // the card said down, and the honest words are what make the
            // repair obviously the fix).
            Some(actual) if actual != port => stale_record_note(port, actual),
            _ => format!(
                "the worker does not answer on port {port} — the substrate is down \
                 (KI-493's shape: a Docker store without the pinned worker)"
            ),
        };
        ServiceHealth::down("layerfs", note)
    }
}

/// The host port the running worker container publishes, when it runs:
/// `docker port layerfs-worker 7070` answers `127.0.0.1:<host>` while the
/// container is up and nothing when it is not. Read only on the refused
/// path of the probe — never when the record answers.
fn worker_container_port() -> Option<u16> {
    let output = std::process::Command::new("docker")
        .args(["port", "layerfs-worker", "7070"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next()?.trim().to_string();
    let port = first.rsplit(':').next()?;
    port.parse::<u16>().ok()
}

fn stale_record_note(recorded: u16, actual: u16) -> String {
    format!(
        "the worker answers on port {actual}, not the {recorded} the endpoint record names — \
         the record is stale (the machine restarted and Docker reassigned the port)"
    )
}

/// The API half: the walk cannot serve without a key. Resolution is
/// the standing chain — the process environment, then the home's
/// `.env` — never a third place.
fn api_probe(home: &Path) -> ServiceHealth {
    let from_env = std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty());
    if from_env.is_some() {
        return ServiceHealth::up("api");
    }
    let from_home = crate::home::read_dotenv(home)
        .get("DEEPSEEK_API_KEY")
        .cloned()
        .filter(|k| !k.trim().is_empty());
    match from_home {
        Some(_) => ServiceHealth::up("api"),
        None => ServiceHealth::down(
            "api",
            "no DEEPSEEK_API_KEY in the environment or the home's .env",
        ),
    }
}

/// The slow tier: the six vendored cores' presence — one file stat
/// per lane, never a spawn. A missing core is a named degradation
/// (hard rule 12), and the repair is the install that rebuilds it.
fn lanes_probe() -> ServiceHealth {
    match crate::lanes::services_root() {
        Err(error) => ServiceHealth::down(
            "lanes",
            format!("the services root is unavailable: {error}"),
        ),
        Ok(root) => {
            // The presence marker is the lane's MCP entry — the file the
            // launch spec actually runs — not a manifest stamp: the
            // newer lanes (browser) carry no VENDOR.json, and a marker
            // the runtime does not read names healthy cores down.
            let missing: Vec<&str> = crate::lanes::LANES
                .iter()
                .filter(|lane| {
                    !root
                        .join(lane.name)
                        .join("service")
                        .join("mcp_server.py")
                        .exists()
                })
                .map(|lane| lane.name)
                .collect();
            if missing.is_empty() {
                ServiceHealth::up("lanes")
            } else {
                ServiceHealth::down(
                    "lanes",
                    format!("the vendored cores are missing: {}", missing.join(", ")),
                )
            }
        }
    }
}

/// The slow tier: docker's daemon answers at all. The LayerFS worker
/// is a container; a dead daemon is the substrate's root cause.
fn docker_probe() -> ServiceHealth {
    let ok = std::process::Command::new("docker")
        .arg("info")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        ServiceHealth::up("docker")
    } else {
        ServiceHealth::down(
            "docker",
            "docker's daemon is down — the LayerFS worker cannot run",
        )
    }
}

/// The release feed — GitHub's, pinned (the owner's 2026-11-09 ruling:
/// updates come from GitHub, never locally; no override, the same URL
/// the updater polls and `scripts/release.sh` reads back).
pub const RELEASE_FEED: &str =
    "https://github.com/adelvillar1/fleet/releases/latest/download/latest.json";

/// The auto-update channel's supply (the 2026-11-10 lesson: 0.6.22
/// and 0.6.23 were installed locally and never cut, so no installed
/// app could have updated to them). A build whose version has no
/// release tag, or whose published feed names an older version, will
/// never reach an installed app — the updater reads GitHub's feed and
/// nothing else. `None` when no checkout resolves.
fn release_probe() -> Option<ServiceHealth> {
    let services = crate::lanes::services_root().ok()?;
    let root = services.parent()?;
    if !root.join(".git").exists() {
        return None;
    }
    let version = env!("CARGO_PKG_VERSION");
    let tag = format!("v{version}");
    let listed = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["tag", "-l", &tag])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let tagged = String::from_utf8_lossy(&listed.stdout).trim() == tag;
    // The feed half speaks only once the tag half has: an untagged
    // build is the whole finding, and the read costs a round trip.
    let feed = if tagged { read_feed_version() } else { None };
    Some(release_verdict(version, tagged, feed.as_deref()))
}

/// The published feed's version, read anonymously the way an
/// installed app would. A read that fails says nothing — the tag
/// half's verdict stands alone.
fn read_feed_version() -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let body = agent.get(RELEASE_FEED).call().ok()?.into_string().ok()?;
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

/// The verdict from the two halves: an untagged build is uncut; a
/// tagged build whose published feed names an older version is
/// half-cut (the tag exists, the assets never published — KI-503's
/// shape); anything else is serving. Pure, so the shapes are pinned
/// without a network or a repository.
pub fn release_verdict(version: &str, tagged: bool, feed: Option<&str>) -> ServiceHealth {
    if !tagged {
        return ServiceHealth::down(
            "release",
            format!(
                "v{version} is built but has no release tag — the auto-update \
                 channel will never serve this build"
            ),
        );
    }
    match feed {
        Some(published) if version_lt(published, version) => ServiceHealth::down(
            "release",
            format!(
                "the release feed names {published} while this build is {version} — \
                 the tag exists but the assets never published"
            ),
        ),
        _ => ServiceHealth::up("release"),
    }
}

/// `a < b` over dotted numeric versions. An unparseable side compares
/// as not-less: a feed the reader cannot parse is not a finding.
fn version_lt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Option<Vec<u64>> {
        v.trim_start_matches('v')
            .split('.')
            .map(|part| part.parse::<u64>().ok())
            .collect()
    };
    match (parse(a), parse(b)) {
        (Some(left), Some(right)) => left < right,
        _ => false,
    }
}

/// The repair map, one line per service: what the face paints, what
/// the repair note quotes, what the main session attempts. An
/// owner-action wording for what no command can self-repair.
pub fn repair_for(name: &str) -> &'static str {
    match name {
        "layerfs" => "fleet install (re-provisions the worker and re-records the endpoint)",
        "lanes" => "fleet install (rebuilds the lane venvs)",
        "docker" => "start Docker Desktop",
        "release" => {
            "scripts/release.sh tag v<version>, then scripts/release.sh assets v<version> \
             — the cut is the delivery; a local install is not"
        }
        "api" => "set DEEPSEEK_API_KEY in the environment or ~/.fleet/.env",
        _ => "fleet doctor names the repair",
    }
}

/// One pass's verdict, serde-shaped for `/api/state`'s `services`
/// block and comparable across passes for down→up and up→down
/// transitions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HealthReport {
    pub services: Vec<ServiceHealth>,
    /// The pass's day stamp (`calib::today()`), so a reader can tell
    /// a fresh pass from a stale one.
    pub stamp: String,
}

impl HealthReport {
    /// This pass carried forward beside the previous one: every service the
    /// previous report held that this one did not probe keeps its last known
    /// verdict, and this pass's fresh lines win where both hold a name.
    ///
    /// KI-513: the tiers alternate, so a fast pass carries only layerfs and api
    /// while a slow pass carries those plus docker, lanes and release — and the
    /// card replaced the slot whole each time, so a down slow-tier service was
    /// invisible on every fast tick. The verdicts were true; only the display
    /// was a function of the tick. Merging makes the card's line set stable
    /// across ticks without pretending a stale verdict is fresh: the carried
    /// line keeps its own note, and [`HealthReport::carried`] names which ones
    /// this pass did not re-probe.
    pub fn merged_over(self, previous: &HealthReport) -> HealthReport {
        let probed: std::collections::HashSet<&str> =
            self.services.iter().map(|service| service.name).collect();
        let mut services = self.services;
        for service in &previous.services {
            if !probed.contains(service.name) {
                services.push(service.clone());
            }
        }
        HealthReport {
            services,
            stamp: self.stamp,
        }
    }

    /// The names this report holds WITHOUT having probed them this pass — the
    /// carried-forward lines, so a reader can tell a fresh verdict from a kept
    /// one. Empty for a slow pass, which probes everything.
    pub fn carried(&self, probed_now: &[&'static str]) -> Vec<&'static str> {
        self.services
            .iter()
            .map(|service| service.name)
            .filter(|name| !probed_now.contains(name))
            .collect()
    }

    /// Every down service, in report order — the card's lines and the
    /// repair note's causes.
    pub fn down(&self) -> Vec<&ServiceHealth> {
        self.services.iter().filter(|s| !s.up).collect()
    }
    /// Whether this pass differs from the last in any shared
    /// service's up-ness — compared by name, so the tier alternation
    /// itself (a slow pass carries two lines a fast pass omits) is
    /// never read as a transition.
    pub fn transitioned_from(&self, previous: &HealthReport) -> bool {
        let before: std::collections::HashMap<&str, bool> =
            previous.services.iter().map(|s| (s.name, s.up)).collect();
        self.services
            .iter()
            .any(|s| before.get(s.name).map(|up| *up != s.up).unwrap_or(false))
    }
}

/// The repair note's sentence (the failsafe plan's C4): every down
/// service, its cause, and its repair verb — the word the main session
/// acts on. The closing line carries the ruling: never refuse the work
/// over a down service — degrade and repair instead.
pub fn repair_note(report: &HealthReport) -> String {
    let lines: Vec<String> = report
        .down()
        .iter()
        .map(|s| format!("- {} — {}; repair: {}", s.name, s.note, s.repair))
        .collect();
    format!(
        "fleet's health probes name these services down:\n{}\nRepair what is          repairable (the commands above run through your own bash tool and the          standing approval gate) when the current task allows — and never refuse          the task over a down service: degrade and repair instead.",
        lines.join("\n")
    )
}

/// The run-time half of the probe (the failsafe plan's C4): what THIS
/// run depends on, only when it was provisioned. An unprovisioned
/// substrate is not a down service — nothing opted into it — so it
/// names nothing; a provisioned one whose worker does not answer is
/// the note. The loop's full report stays the face's vocabulary; the
/// run path's is narrower by design.
pub fn repair_note_for_run(home: &Path) -> Option<String> {
    if crate::layerfs_tools::read_machine_at(home).is_err() {
        return None;
    }
    let report = probe(home, true);
    if report.down().is_empty() {
        None
    } else {
        Some(repair_note(&report))
    }
}

/// Where the loop's latest report lives — the server's handle to the
/// bridge's shared slot.
pub type SharedReport = std::sync::Arc<std::sync::Mutex<Option<HealthReport>>>;

/// The loop body the server spawns: one pass per tick, slow tier every
/// other pass, the shared slot updated in place. Returns whether the
/// pass transitioned, so the caller (which sees the beat) can name it.
pub fn probe_once(home: &Path, pass: u64, slot: &SharedReport) -> bool {
    let fresh = probe(home, !pass.is_multiple_of(SLOW_EVERY_NTH_PASS));
    let mut guard = slot.lock().expect("health slot");
    // The card's line set must not be a function of the tick (KI-513): a fast
    // pass keeps the slow tier's last verdicts beside its own fresh ones. The
    // transition is judged on the MERGED report, so a carried verdict that has
    // not changed cannot read as a change, and one that did change — a slow
    // pass finding docker down — is announced on that pass.
    let report = match guard.as_ref() {
        Some(previous) => fresh.merged_over(previous),
        None => fresh,
    };
    let transitioned = match guard.as_ref() {
        Some(previous) => report.transitioned_from(previous),
        None => !report.down().is_empty(),
    };
    *guard = Some(report);
    transitioned
}

/// A scratch endpoint for tests: the file shape the worker records.
#[cfg(test)]
pub(crate) fn write_endpoint(home: &Path, port: u16) -> PathBuf {
    let dir = home.join("layerfs-home").join("worker");
    std::fs::create_dir_all(&dir).expect("endpoint dir");
    let endpoint = dir.join("endpoint.json");
    std::fs::write(
        &endpoint,
        format!(
            r#"{{"home":"{}/layerfs-home","port":{port}}}"#,
            home.display()
        ),
    )
    .expect("endpoint write");
    endpoint
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        // The label is part of the path: tests run in parallel in one
        // process, and a shared path lets one test's fixture-delete
        // race another's probe (the release gate's find — a green
        // working tree had hidden it twice by scheduling luck).
        let dir = std::env::temp_dir().join(format!("fleet-health-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch home");
        dir
    }

    /// The auto-update channel's supply, pinned without a network or a
    /// repository: an untagged build is uncut, a tagged build behind
    /// its own feed is half-cut, and everything else is serving.
    #[test]
    fn an_uncut_or_half_cut_build_names_itself_and_the_cut() {
        // Built, never tagged: the 0.6.22 shape.
        let uncut = release_verdict("0.6.24", false, None);
        assert!(!uncut.up);
        assert!(uncut
            .note
            .contains("v0.6.24 is built but has no release tag"));
        assert!(uncut.note.contains("auto-update"));
        assert!(uncut.repair.contains("scripts/release.sh tag"));
        // Tagged, but the feed still names the previous cut: the
        // assets never published (KI-503's shape).
        let half = release_verdict("0.6.24", true, Some("0.6.23"));
        assert!(!half.up);
        assert!(half
            .note
            .contains("the release feed names 0.6.23 while this build is 0.6.24"));
        // Tagged and the feed names this build: serving.
        assert!(release_verdict("0.6.24", true, Some("0.6.24")).up);
        // A feed ahead of this build is an update waiting, not a fault.
        assert!(release_verdict("0.6.24", true, Some("0.6.25")).up);
        // A feed that could not be read says nothing: the tag half
        // stands alone.
        assert!(release_verdict("0.6.24", true, None).up);
        // An unparseable feed is not a finding.
        assert!(release_verdict("0.6.24", true, Some("not-a-version")).up);
    }

    #[test]
    fn versions_compare_numerically_not_lexically() {
        assert!(
            version_lt("0.6.9", "0.6.21"),
            "9 < 21, not the string order"
        );
        assert!(version_lt("0.6.23", "0.7.0"));
        assert!(version_lt("v0.6.23", "0.6.24"), "a leading v is tolerated");
        assert!(!version_lt("0.6.24", "0.6.24"));
        assert!(!version_lt("0.6.25", "0.6.24"));
    }

    #[test]
    fn a_closed_endpoint_port_reads_down_with_the_repair_named() {
        let home = scratch("closed-port");
        // Port 1 on loopback is never listening, and no container is
        // running: the KI-493 shape, named whole.
        write_endpoint(&home, 1);
        let layerfs = layerfs_probe_with(1, None);
        assert!(!layerfs.up, "a refused connect is down");
        assert!(layerfs.note.contains("port 1"), "{}", layerfs.note);
        assert!(layerfs.note.contains("KI-493"), "{}", layerfs.note);
        assert_eq!(
            layerfs.repair,
            "fleet install (re-provisions the worker and re-records the endpoint)"
        );
    }

    /// The owner's 2026-11-13 report: the worker was up and running on
    /// Docker, the card said down — the machine restart reassigned
    /// Docker's published port and the endpoint record cannot follow.
    /// The diagnosis names both ports and stays a repair away from true.
    /// The light repair (the owner's 2026-11-14 ruling): a stale record
    /// reconciles by rewriting the endpoint's port to the container's
    /// actual — no provision leg. A record the container agrees with is
    /// not stale, and no container means no reconciliation.
    #[test]
    fn the_light_repair_rewrites_the_port_and_reconciles() {
        let home = scratch("light-repair");
        write_endpoint(&home, 1);
        assert!(reconcile_stale_record_with(&home, Some(57888)));
        let endpoint = home
            .join("layerfs-home")
            .join("worker")
            .join("endpoint.json");
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&endpoint).unwrap()).unwrap();
        assert_eq!(
            document["port"], 57888,
            "the record names the container's actual"
        );
        // The container agreeing with the record is not stale: no write.
        write_endpoint(&home, 1);
        assert!(!reconcile_stale_record_with(&home, Some(1)));
        // No container: no reconciliation either.
        assert!(!reconcile_stale_record_with(&home, None));
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&endpoint).unwrap()).unwrap();
        assert_eq!(document["port"], 1, "the record stands");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_stale_record_names_the_running_worker() {
        let home = scratch("stale-record");
        write_endpoint(&home, 1);
        let layerfs = layerfs_probe_with(1, Some(57888));
        assert!(!layerfs.up, "the record's port is refused: down");
        assert!(layerfs.note.contains("port 57888"), "{}", layerfs.note);
        assert!(
            layerfs.note.contains("the 1 the endpoint record names"),
            "{}",
            layerfs.note
        );
        assert!(layerfs.note.contains("stale"), "{}", layerfs.note);
        assert_eq!(layerfs.repair, repair_for("layerfs"));
    }

    #[test]
    fn a_record_matching_a_dead_container_stays_the_ki493_shape() {
        let home = scratch("agreed-dead");
        write_endpoint(&home, 1);
        // The container publishes the same port the record names — the
        // connect should have worked and did not: the transient shape,
        // still the KI-493 words.
        let layerfs = layerfs_probe_with(1, Some(1));
        assert!(layerfs.note.contains("KI-493"), "{}", layerfs.note);
        assert!(!layerfs.note.contains("stale"), "{}", layerfs.note);
    }

    #[test]
    fn a_home_without_a_provisioned_substrate_names_the_absence() {
        let home = scratch("absent-substrate");
        let report = probe(&home, true);
        let layerfs = report
            .services
            .iter()
            .find(|s| s.name == "layerfs")
            .expect("the layerfs line");
        assert!(!layerfs.up);
        assert!(layerfs.note.contains("not provisioned"));
    }

    #[test]
    fn the_api_line_reads_the_standing_chain() {
        let home = scratch("api-chain");
        let report = probe(&home, true);
        let api = report
            .services
            .iter()
            .find(|s| s.name == "api")
            .expect("the api line");
        // Whatever this machine's environment carries, the line's
        // verdict must match a direct read of the same chain.
        let expected = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .is_some()
            || crate::home::read_dotenv(&home)
                .get("DEEPSEEK_API_KEY")
                .map(|k| !k.trim().is_empty())
                .unwrap_or(false);
        assert_eq!(api.up, expected);
    }

    #[test]
    fn fast_only_skips_the_slow_tier_and_the_full_pass_carries_it() {
        let home = scratch("fast-tier");
        let fast = probe(&home, true);
        assert!(
            fast.services.iter().all(|s| s.name != "docker"),
            "the fast tier carries no docker line"
        );
        let full = probe(&home, false);
        assert!(full.services.iter().any(|s| s.name == "docker"));
    }

    #[test]
    fn a_transition_is_any_change_in_upness() {
        let mut a = probe(&scratch("full-tier"), true);
        let b = probe(&scratch("transition-a"), true);
        assert!(!a.transitioned_from(&b));
        if let Some(s) = a.services.iter_mut().find(|s| s.name == "layerfs") {
            s.up = !s.up;
        }
        assert!(a.transitioned_from(&b));
        // The stamp alone is not a transition — a fresh pair, only
        // the day changed.
        let mut c = probe(&scratch("transition-b"), true);
        let d = probe(&scratch("transition-c"), true);
        c.stamp = "another-day".into();
        assert!(!c.transitioned_from(&d));
    }

    /// KI-513: the tiers alternate, so the slot used to hold a two-line view on
    /// every fast pass and a five-line view on every slow one — a down
    /// slow-tier service invisible on every even tick. The merge keeps the line
    /// set stable and still lets a real change announce itself.
    #[test]
    fn a_fast_pass_carries_the_slow_tiers_verdicts_beside_its_own() {
        fn up(name: &'static str) -> ServiceHealth {
            ServiceHealth {
                name,
                up: true,
                note: String::new(),
                repair: "none",
            }
        }
        let slow = HealthReport {
            services: vec![
                up("layerfs"),
                up("api"),
                ServiceHealth::down("docker", "not answering"),
                up("lanes"),
                up("release"),
            ],
            stamp: "2026-11-11".to_string(),
        };
        // A fast pass probes only its own tier.
        let fast = HealthReport {
            services: vec![up("layerfs"), up("api")],
            stamp: "2026-11-11".to_string(),
        };
        let merged = fast.clone().merged_over(&slow);
        let names: Vec<&str> = merged.services.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            vec!["layerfs", "api", "docker", "lanes", "release"],
            "the card always shows the full set: {names:?}"
        );
        // The down verdict survives the fast pass, which is the defect.
        assert_eq!(merged.down().len(), 1);
        assert_eq!(merged.down()[0].name, "docker");
        // And the carried lines say they were not probed this pass.
        let carried = merged.carried(&["layerfs", "api"]);
        assert_eq!(carried, vec!["docker", "lanes", "release"]);
        // A carried, unchanged verdict is NOT a transition.
        assert!(!merged.transitioned_from(&slow));
        // A real change is: the slow pass finds the down service up again.
        let mut healed = slow.clone();
        healed.services[2] = up("docker");
        assert!(healed.transitioned_from(&slow));
        // The fresh line wins where both reports hold the same name.
        let mut fresh_docker = fast.clone();
        fresh_docker
            .services
            .push(ServiceHealth::down("docker", "still down"));
        let merged_fresh = fresh_docker.merged_over(&slow);
        assert_eq!(
            merged_fresh
                .services
                .iter()
                .filter(|s| s.name == "docker")
                .count(),
            1,
            "a name appears once, with this pass's verdict"
        );
        assert_eq!(
            merged_fresh
                .services
                .iter()
                .find(|s| s.name == "docker")
                .map(|s| s.note.clone()),
            Some("still down".to_string())
        );
    }

    #[test]
    fn the_loop_slot_updates_and_reports_transitions_once() {
        let home = scratch("transition-d");
        write_endpoint(&home, 1);
        let slot: SharedReport = std::sync::Arc::new(std::sync::Mutex::new(None));
        let first = probe_once(&home, 0, &slot);
        assert!(first, "the first pass over a down service is a transition");
        let second = probe_once(&home, 1, &slot);
        assert!(
            !second,
            "the same verdict again is not a transition — and the \
             tier alternation alone is not one either"
        );
        let guard = slot.lock().expect("health slot");
        let report = guard.as_ref().expect("the slot holds the latest pass");
        // Pass 1 is a fast pass, so it PROBED two services — but the slot keeps
        // the slow tier's verdicts beside them, because the card's line set must
        // not be a function of the tick (KI-513). Before the merge this asserted
        // 2, which was the defect stated as a contract.
        assert_eq!(
            report.carried(&["layerfs", "api"]).len(),
            report.services.len() - 2,
            "the fast pass probed two and carried the rest: {:?}",
            report.services
        );
        assert!(
            report.services.len() >= 2,
            "the card shows at least the fresh tier: {:?}",
            report.services
        );
    }
}
