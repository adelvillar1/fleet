//! Registration + roster + heartbeat (armada L8.1, D44). An instance
//! registers on join with its install-id (L1), sid, version, and capacity;
//! the flagship roster holds it at `<home>/roster.json`. Liveness is EARNED
//! by heartbeats, not assumed — a sid with no recent heartbeat is reported
//! not-live (the cross-cutting obligation's L8 row).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A sid is live only within this window of its last heartbeat. The same
/// shape the relay broker uses (L6): liveness is a measured freshness, never
/// a flag set at registration.
pub const HEARTBEAT_WINDOW_SECS: u64 = 90;

/// One registered instance. The install-id is L1's stable identity; the sid
/// is its relay session; capacity is the contract's reading the instance
/// admits against (the flagship cannot overwhelm a small VPS — it admits for
/// itself, D49).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub install_id: String,
    pub sid: String,
    pub version: String,
    /// The instance's declared run budget, in KiB (the contract's reading).
    pub capacity_kib: u64,
    /// Epoch seconds of the last heartbeat; 0 until the first.
    pub last_heartbeat: u64,
    pub registered_at: u64,
}

impl Instance {
    /// Liveness earned by a recent heartbeat (D44). A fresh registration
    /// with no heartbeat is not live until it beats.
    pub fn live(&self, now_secs: u64) -> bool {
        self.last_heartbeat > 0
            && now_secs.saturating_sub(self.last_heartbeat) <= HEARTBEAT_WINDOW_SECS
    }
}

/// The flagship roster: instances keyed by sid.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roster {
    pub instances: BTreeMap<String, Instance>,
}

/// The roster's path: `<home>/roster.json`.
pub fn path(home: &Path) -> PathBuf {
    home.join("roster.json")
}

/// Read the roster; an absent file is an empty roster (a single-instance
/// fleet behaves as today, C8).
pub fn load(home: &Path) -> Roster {
    std::fs::read_to_string(path(home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Write the roster atomically (a temp beside it then a rename), so a reader
/// never sees a half-written roster.
pub fn save(home: &Path, roster: &Roster) -> Result<(), String> {
    let path = path(home);
    let text = serde_json::to_string_pretty(roster).map_err(|e| format!("the roster: {e}"))?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| format!("the roster write: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("the roster rename: {e}"))?;
    Ok(())
}

/// Register (or re-register) an instance on join. The install-id, sid,
/// version and capacity ride the join; a re-registration refreshes them but
/// keeps the registration instant. Liveness still requires a heartbeat.
pub fn register(
    home: &Path,
    install_id: &str,
    sid: &str,
    version: &str,
    capacity_kib: u64,
    now_secs: u64,
) -> Result<Instance, String> {
    if sid.trim().is_empty() {
        return Err("a registration needs a sid".to_string());
    }
    let mut roster = load(home);
    let registered_at = roster
        .instances
        .get(sid)
        .map(|prior| prior.registered_at)
        .unwrap_or(now_secs);
    let instance = Instance {
        install_id: install_id.to_string(),
        sid: sid.to_string(),
        version: version.to_string(),
        capacity_kib,
        last_heartbeat: roster
            .instances
            .get(sid)
            .map(|prior| prior.last_heartbeat)
            .unwrap_or(0),
        registered_at,
    };
    roster.instances.insert(sid.to_string(), instance.clone());
    save(home, &roster)?;
    Ok(instance)
}

/// Record a heartbeat for a sid, earning its liveness. An unknown sid is
/// refused by name — a heartbeat cannot register an instance (that is the
/// join's act, gated separately).
pub fn heartbeat(home: &Path, sid: &str, now_secs: u64) -> Result<(), String> {
    let mut roster = load(home);
    let instance = roster
        .instances
        .get_mut(sid)
        .ok_or_else(|| format!("the sid `{sid}` is not on the roster"))?;
    instance.last_heartbeat = now_secs;
    save(home, &roster)
}

/// The roster's live instances at `now_secs` — liveness earned, not assumed.
pub fn live_instances(home: &Path, now_secs: u64) -> Vec<Instance> {
    load(home)
        .instances
        .into_values()
        .filter(|instance| instance.live(now_secs))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-roster-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// C0: registration holds the instance; a heartbeat earns liveness; a
    /// sid with no recent heartbeat is reported not-live.
    #[test]
    fn liveness_is_earned_by_heartbeats() {
        let home = scratch("liveness");
        let instance = register(&home, "install-a", "sid-a", "0.6.37", 2_000_000, 1000).unwrap();
        // Registered but no heartbeat: not live.
        assert!(!instance.live(1000), "a fresh registration is not live");
        assert!(live_instances(&home, 1000).is_empty());

        // A heartbeat earns liveness.
        heartbeat(&home, "sid-a", 1000).unwrap();
        assert!(load(&home).instances["sid-a"].live(1000));
        assert_eq!(live_instances(&home, 1050).len(), 1);

        // Past the window, liveness lapses — earned, not assumed.
        assert!(!load(&home).instances["sid-a"].live(1000 + HEARTBEAT_WINDOW_SECS + 1));
        assert!(live_instances(&home, 1000 + HEARTBEAT_WINDOW_SECS + 1).is_empty());
    }

    #[test]
    fn a_heartbeat_cannot_register_an_unknown_sid() {
        let home = scratch("unknown");
        let err = heartbeat(&home, "ghost", 1000).unwrap_err();
        assert!(err.contains("not on the roster"), "{err}");
    }

    #[test]
    fn re_registration_refreshes_but_keeps_the_registration_instant() {
        let home = scratch("reregister");
        register(&home, "install-a", "sid-a", "0.6.36", 1_000_000, 1000).unwrap();
        heartbeat(&home, "sid-a", 1000).unwrap();
        let again = register(&home, "install-a", "sid-a", "0.6.37", 2_000_000, 2000).unwrap();
        assert_eq!(again.version, "0.6.37");
        assert_eq!(again.capacity_kib, 2_000_000);
        assert_eq!(again.registered_at, 1000, "the first join's instant stands");
        assert_eq!(again.last_heartbeat, 1000, "the earned heartbeat stands");
    }

    #[test]
    fn an_empty_roster_is_an_ordinary_fleet() {
        let home = scratch("empty");
        assert!(load(&home).instances.is_empty());
        assert!(!crate::armada::is_flagship(&home));
        register(&home, "install-a", "sid-a", "0.6.37", 1, 1000).unwrap();
        assert!(crate::armada::is_flagship(&home));
    }
}
