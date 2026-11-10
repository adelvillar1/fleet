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
    let machine = crate::layerfs_tools::read_machine_at(home);
    let endpoint = match machine.as_ref() {
        Ok(m) => m.home.join("worker").join("endpoint.json"),
        Err(reason) => {
            let conventional = home
                .join("layerfs-home")
                .join("worker")
                .join("endpoint.json");
            if !conventional.exists() {
                return ServiceHealth::down("layerfs", format!("not provisioned — {reason}"));
            }
            conventional
        }
    };
    let Ok(text) = std::fs::read_to_string(&endpoint) else {
        return ServiceHealth::down(
            "layerfs",
            format!("the endpoint file is unreadable: {}", endpoint.display()),
        );
    };
    let port = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("port").and_then(|p| p.as_u64()))
        .map(|p| p as u16);
    let Some(port) = port else {
        return ServiceHealth::down("layerfs", "the endpoint file names no port — install again");
    };
    if crate::net::listening("127.0.0.1", port, CONNECT_TIMEOUT) {
        ServiceHealth::up("layerfs")
    } else {
        ServiceHealth::down(
            "layerfs",
            format!(
                "the worker does not answer on port {port} — the substrate is down \
                 (KI-493's shape: a Docker store without the pinned worker)"
            ),
        )
    }
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
            let missing: Vec<&str> = crate::lanes::LANES
                .iter()
                .filter(|lane| !root.join(lane.name).join("VENDOR.json").exists())
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

/// The repair map, one line per service: what the face paints, what
/// the repair note quotes, what the main session attempts. An
/// owner-action wording for what no command can self-repair.
pub fn repair_for(name: &str) -> &'static str {
    match name {
        "layerfs" => "fleet install (re-provisions the worker and re-records the endpoint)",
        "lanes" => "fleet install (rebuilds the lane venvs)",
        "docker" => "start Docker Desktop",
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

/// Where the loop's latest report lives — the server's handle to the
/// bridge's shared slot.
pub type SharedReport = std::sync::Arc<std::sync::Mutex<Option<HealthReport>>>;

/// The loop body the server spawns: one pass per tick, slow tier every
/// other pass, the shared slot updated in place. Returns whether the
/// pass transitioned, so the caller (which sees the beat) can name it.
pub fn probe_once(home: &Path, pass: u64, slot: &SharedReport) -> bool {
    let report = probe(home, pass % SLOW_EVERY_NTH_PASS != 0);
    let transitioned = {
        let guard = slot.lock().expect("health slot");
        match guard.as_ref() {
            Some(previous) => report.transitioned_from(previous),
            None => !report.down().is_empty(),
        }
    };
    *slot.lock().expect("health slot") = Some(report);
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

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-health-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch home");
        dir
    }

    #[test]
    fn a_closed_endpoint_port_reads_down_with_the_repair_named() {
        let home = scratch();
        // Port 1 on loopback is never listening.
        write_endpoint(&home, 1);
        let report = probe(&home, true);
        let layerfs = report
            .services
            .iter()
            .find(|s| s.name == "layerfs")
            .expect("the layerfs line");
        assert!(!layerfs.up, "a refused connect is down");
        assert!(layerfs.note.contains("port 1"));
        assert_eq!(
            layerfs.repair,
            "fleet install (re-provisions the worker and re-records the endpoint)"
        );
    }

    #[test]
    fn a_home_without_a_provisioned_substrate_names_the_absence() {
        let home = scratch();
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
        let home = scratch();
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
        let home = scratch();
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
        let mut a = probe(&scratch(), true);
        let b = probe(&scratch(), true);
        assert!(!a.transitioned_from(&b));
        if let Some(s) = a.services.iter_mut().find(|s| s.name == "layerfs") {
            s.up = !s.up;
        }
        assert!(a.transitioned_from(&b));
        // The stamp alone is not a transition — a fresh pair, only
        // the day changed.
        let mut c = probe(&scratch(), true);
        let d = probe(&scratch(), true);
        c.stamp = "another-day".into();
        assert!(!c.transitioned_from(&d));
    }

    #[test]
    fn the_loop_slot_updates_and_reports_transitions_once() {
        let home = scratch();
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
        assert_eq!(report.services.len(), 2, "pass 1 is a fast pass");
    }
}
