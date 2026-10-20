//! The browser face's own usage counters: what the owner actually
//! accepts, which surfaces they open, and which actions they use.
//!
//! The store is `<FLEET_HOME>/ui.json`, owned by this module alone — a
//! GUI-owned file, not a fourth section of `settings.rs`, because it is
//! feedback state rather than configuration. `settings.rs` carries the
//! one knob that switches the adaptation off (`ui.adapt`).
//!
//! ## What is stored, and what is not
//!
//! Counters and timestamps only: `{shown, accepted, dismissed, last}`
//! for suggestions, `{opens, last}` for panels, `{uses, last}` for
//! actions. No task text, no conversation text, no file contents, no
//! credential names or values. An id is a short label the caller owns —
//! an offer's name, a panel's address, a related pair — and it is
//! refused when it carries a control character or grows past
//! [`MAX_ID_CHARS`], so the file cannot become a log by accident.
//!
//! ## The write discipline
//!
//! Atomic temp-and-rename, the recall index's own rule, so a reader
//! never sees a half-written file. The file is capped: past
//! [`MAX_BYTES`] the oldest entries are dropped until it fits, and
//! past [`MAX_ENTRIES`] per section the least recently touched is
//! dropped on insertion. Malformed JSON, a foreign version, or an
//! unreadable file all read as empty — the counters are a cache of
//! preference, and losing them costs a static order, never an error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The file this module owns beside the home's other state. The leading
/// dot keeps it out of a casual listing; it is not a dotfile because it
/// lives outside the repository.
pub const FILE: &str = "ui.json";

/// The schema version. A file from a newer writer is discarded whole.
pub const VERSION: u32 = 1;

/// The most one id may carry. An offer, a panel address, or a related
/// pair is a label; anything longer is not one.
pub const MAX_ID_CHARS: usize = 120;

/// How many entries one section holds before the least recently touched
/// is dropped. Three sections at this cap stay far under the byte cap.
pub const MAX_ENTRIES: usize = 400;

/// The file's ceiling. The counter file is preference state, and a
/// store that grows without one would be a log.
pub const MAX_BYTES: usize = 256 * 1024;

/// One suggestion's counters: how often it was shown, accepted, and
/// dismissed, and when it was last seen.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Suggestion {
    #[serde(default)]
    pub shown: u64,
    #[serde(default)]
    pub accepted: u64,
    #[serde(default)]
    pub dismissed: u64,
    #[serde(default)]
    pub last: u64,
}

/// One panel's counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Panel {
    #[serde(default)]
    pub opens: u64,
    #[serde(default)]
    pub last: u64,
}

/// One action's counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    #[serde(default)]
    pub uses: u64,
    #[serde(default)]
    pub last: u64,
}

/// The whole store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub suggestions: BTreeMap<String, Suggestion>,
    #[serde(default)]
    pub panels: BTreeMap<String, Panel>,
    #[serde(default)]
    pub actions: BTreeMap<String, Action>,
}

/// The store and the home it writes to.
pub struct UsageStore {
    home: PathBuf,
    usage: Mutex<Usage>,
}

impl UsageStore {
    /// Read the store. Never fails: a missing, malformed, or
    /// foreign-version file is an empty store.
    pub fn new(home: PathBuf) -> UsageStore {
        let mut usage = read(&home);
        // A fresh (or discarded) store still writes the current
        // version: the version is the reader's contract, not a fact
        // the counters carry.
        usage.version = VERSION;
        UsageStore {
            home,
            usage: Mutex::new(usage),
        }
    }

    /// The document the browser reads: the counters and whether the
    /// adaptation is on. `adapt` is the settings knob's current value,
    /// passed in so the one reader of configuration stays settings.rs.
    pub fn document(&self, adapt: bool) -> Value {
        let usage = lock(&self.usage);
        json!({
            "type": "usage",
            "adapt": adapt,
            "suggestions": usage.suggestions,
            "panels": usage.panels,
            "actions": usage.actions,
        })
    }

    /// Record one event. The section and event are a closed set: a
    /// suggestion can be shown, accepted, or dismissed; a panel opens;
    /// an action is used. An id is validated for shape, never for
    /// meaning — the store cannot tell a fact name from a credential
    /// name, so the callers only pass their own labels and the length
    /// and control-character rules keep the file from becoming a log.
    pub fn record(&self, section: &str, id: &str, event: &str) -> Result<(), String> {
        if id.trim().is_empty() {
            return Err("a usage event needs an id".to_string());
        }
        if id.chars().count() > MAX_ID_CHARS {
            return Err(format!(
                "a usage id is at most {MAX_ID_CHARS} characters; this one is {}",
                id.chars().count()
            ));
        }
        if id.chars().any(char::is_control) {
            return Err("a usage id carries no control characters".to_string());
        }
        let now = unix_now();
        {
            let mut usage = lock(&self.usage);
            match (section, event) {
                ("suggestions", "shown") => {
                    bump_suggestion(&mut usage, id, now, |row| &mut row.shown)
                }
                ("suggestions", "accepted") => {
                    bump_suggestion(&mut usage, id, now, |row| &mut row.accepted)
                }
                ("suggestions", "dismissed") => {
                    bump_suggestion(&mut usage, id, now, |row| &mut row.dismissed)
                }
                ("panels", "opens") => {
                    let row = usage.panels.entry(id.to_string()).or_default();
                    row.opens += 1;
                    row.last = now;
                    trim::<Panel>(&mut usage.panels, |row| row.last);
                }
                ("actions", "uses") => {
                    let row = usage.actions.entry(id.to_string()).or_default();
                    row.uses += 1;
                    row.last = now;
                    trim::<Action>(&mut usage.actions, |row| row.last);
                }
                (section, event) => {
                    return Err(format!(
                        "`{event}` is not an event a `{section}` records; \
                         suggestions take shown/accepted/dismissed, panels take opens, actions take uses"
                    ));
                }
            }
        }
        let snapshot = lock(&self.usage).clone();
        write(&self.home, &snapshot);
        Ok(())
    }
}

fn bump_suggestion(
    usage: &mut Usage,
    id: &str,
    now: u64,
    field: impl FnOnce(&mut Suggestion) -> &mut u64,
) {
    let row = usage.suggestions.entry(id.to_string()).or_default();
    *field(row) += 1;
    row.last = now;
    trim::<Suggestion>(&mut usage.suggestions, |row| row.last);
}

/// Drop the least recently touched entry when a section is over its
/// cap. The just-touched entry has the newest stamp, so it is never the
/// one dropped unless every entry shares the same second — in which
/// case dropping the first is still bounded and honest.
fn trim<T>(section: &mut BTreeMap<String, T>, last: impl Fn(&T) -> u64) {
    while section.len() > MAX_ENTRIES {
        let Some(oldest) = section
            .iter()
            .min_by_key(|(_, row)| last(row))
            .map(|(id, _)| id.clone())
        else {
            return;
        };
        section.remove(&oldest);
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn read(home: &Path) -> Usage {
    let Ok(text) = std::fs::read_to_string(home.join(FILE)) else {
        return Usage::default();
    };
    let Ok(usage) = serde_json::from_str::<Usage>(&text) else {
        return Usage::default();
    };
    if usage.version != VERSION {
        return Usage::default();
    }
    usage
}

/// Write through a unique temp and a rename. A file over the byte cap
/// has its oldest entries dropped until it fits, and a write that still
/// fails is silent: the counters are preference state, and the next
/// record re-attempts.
fn write(home: &Path, usage: &Usage) {
    let mut usage = usage.clone();
    loop {
        let Ok(text) = serde_json::to_string(&usage) else {
            return;
        };
        if text.len() <= MAX_BYTES {
            let path = home.join(FILE);
            static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut temp = path.clone().into_os_string();
            temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
            let temp = PathBuf::from(temp);
            if std::fs::write(&temp, text).is_ok() {
                let _ = std::fs::rename(&temp, &path);
            }
            return;
        }
        // Over the cap: drop the oldest across every section, then try
        // the serialization again. A store with nothing left is empty,
        // which is smaller than the cap and ends the loop.
        let oldest_suggestion = usage
            .suggestions
            .iter()
            .min_by_key(|(_, row)| row.last)
            .map(|(id, _)| id.clone());
        let oldest_panel = usage
            .panels
            .iter()
            .min_by_key(|(_, row)| row.last)
            .map(|(id, _)| id.clone());
        let oldest_action = usage
            .actions
            .iter()
            .min_by_key(|(_, row)| row.last)
            .map(|(id, _)| id.clone());
        let candidates = [
            oldest_suggestion.map(|id| (0, id)),
            oldest_panel.map(|id| (1, id)),
            oldest_action.map(|id| (2, id)),
        ];
        let mut removed = false;
        for (section, id) in candidates.into_iter().flatten() {
            match section {
                0 if usage.suggestions.contains_key(&id) => {
                    usage.suggestions.remove(&id);
                    removed = true;
                }
                1 if usage.panels.contains_key(&id) => {
                    usage.panels.remove(&id);
                    removed = true;
                }
                2 if usage.actions.contains_key(&id) => {
                    usage.actions.remove(&id);
                    removed = true;
                }
                _ => {}
            }
            if removed {
                break;
            }
        }
        if !removed {
            return;
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Whether the adaptation is on, read through `settings.rs` so the
/// owner's knob has one reader. Absent is on: the default is the whole
/// point of the feature, and turning it off is an explicit choice.
pub fn adapt(home: &Path) -> bool {
    crate::settings::states(home)
        .into_iter()
        .find(|state| state.key == "ui.adapt")
        .and_then(|state| state.value)
        .map(|value| value != "false")
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-usage-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        home
    }

    #[test]
    fn counters_survive_a_reload_and_carry_no_text() {
        let home = home();
        let store = UsageStore::new(home.clone());
        store
            .record("suggestions", "offer:plan", "shown")
            .expect("shown");
        store
            .record("suggestions", "offer:plan", "shown")
            .expect("shown");
        store
            .record("suggestions", "offer:plan", "accepted")
            .expect("accepted");
        store.record("panels", "memory", "opens").expect("opens");
        store
            .record("actions", "process-review", "uses")
            .expect("uses");
        let read = UsageStore::new(home.clone()).document(true);
        assert_eq!(read["suggestions"]["offer:plan"]["shown"], 2);
        assert_eq!(read["suggestions"]["offer:plan"]["accepted"], 1);
        assert_eq!(read["panels"]["memory"]["opens"], 1);
        assert_eq!(read["actions"]["process-review"]["uses"], 1);
        assert_eq!(read["adapt"], true);
        let raw = std::fs::read_to_string(home.join(FILE)).expect("the file");
        assert!(!raw.contains("prompt"));
        assert!(!raw.contains("text"));
        // No temp is left beside it.
        let temps = std::fs::read_dir(&home)
            .expect("the home")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temps, 0);
    }

    #[test]
    fn a_bad_section_event_or_id_is_refused_without_a_write() {
        let home = home();
        let store = UsageStore::new(home.clone());
        assert!(store.record("suggestions", "x", "opens").is_err());
        assert!(store.record("planets", "x", "shown").is_err());
        assert!(store.record("suggestions", "", "shown").is_err());
        assert!(store.record("suggestions", "a\nb", "shown").is_err());
        assert!(store
            .record("suggestions", &"x".repeat(MAX_ID_CHARS + 1), "shown")
            .is_err());
        assert!(!home.join(FILE).exists(), "nothing was written");
    }

    #[test]
    fn a_malformed_or_foreign_file_reads_as_empty() {
        let home = home();
        std::fs::write(home.join(FILE), "not json").expect("the bad file");
        let store = UsageStore::new(home.clone());
        assert!(store.document(true)["suggestions"]
            .as_object()
            .expect("an object")
            .is_empty());
        std::fs::write(
            home.join(FILE),
            json!({"version": VERSION + 1, "suggestions": {"x": {"shown": 3}}}).to_string(),
        )
        .expect("the future file");
        let store = UsageStore::new(home);
        assert!(store.document(true)["suggestions"]
            .as_object()
            .expect("an object")
            .is_empty());
    }

    #[test]
    fn the_knob_is_on_without_a_config_and_off_when_written() {
        let home = home();
        assert!(adapt(&home), "absent is on");
        std::fs::write(
            home.join("config.json"),
            json!({"ui": {"adapt": false}}).to_string(),
        )
        .expect("the config");
        assert!(!adapt(&home), "the owner's off is honored");
        std::fs::write(
            home.join("config.json"),
            json!({"ui": {"adapt": true}}).to_string(),
        )
        .expect("the config");
        assert!(adapt(&home));
    }

    #[test]
    fn a_section_drops_its_least_recently_touched_entry_at_the_cap() {
        let home = home();
        let store = UsageStore::new(home);
        for index in 0..=(MAX_ENTRIES) {
            store
                .record("panels", &format!("panel-{index}"), "opens")
                .expect("an open");
        }
        let read = store.document(true);
        let panels = read["panels"].as_object().expect("panels");
        assert_eq!(panels.len(), MAX_ENTRIES);
        // The first entry is the one the cap dropped; the newest stands.
        assert!(panels.get("panel-0").is_none());
        assert!(panels.get(&format!("panel-{MAX_ENTRIES}")).is_some());
    }
}
