//! The deferral class: inbound deliveries held for the owner's decision.
//!
//! A delivery from a peer configured to hold does not run and does not
//! vanish: it is filed here, in full, so the owner can accept it — and
//! the accept runs it through the same driver door an auto-run would
//! have taken — or decline it with a reason. A hold carries everything
//! the delivery had: the channel, the peer, the task as it arrived, and
//! the reply handle, because an accepted hold must be able to answer the
//! peer exactly as the auto-run would have.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Inbound, Peer, Reply};

/// One held delivery, everything a later accept needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    /// The hold's own id, unique within the store: the inbox key's tail.
    pub id: String,
    pub channel: String,
    pub peer_id: String,
    /// The peer's name as it paired, which the card shows.
    pub peer: String,
    /// The session the delivery named, if it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The task in full: the card shows it inline, and the accept runs it.
    pub task: String,
    /// Where the outcome would have gone, if the delivery named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback: Option<String>,
    /// When the delivery was held.
    pub at: String,
}

impl Hold {
    /// The delivery this hold would run, reconstructed: the same shape
    /// the plugin handed the sink, so an accept and an auto-run are the
    /// same delivery.
    pub fn to_inbound(&self) -> Inbound {
        Inbound {
            peer: Peer::new(&self.channel, &self.peer_id, &self.peer),
            session: self.session.clone(),
            task: self.task.clone(),
            reply: match &self.callback {
                Some(url) => Reply::Url(url.clone()),
                None => Reply::None,
            },
        }
    }
}

/// How many holds one home remembers. A peer that floods the channel
/// must not grow the file without bound; the newest is kept.
pub const HOLD_CAP: usize = 50;

/// The held deliveries of every channel, one file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HoldStore {
    #[serde(default)]
    pub holds: Vec<Hold>,
}

/// A monotonically rising tail for hold ids, so two holds filed in the
/// same microsecond still name themselves apart.
static HOLD_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The one lock every read-modify-write of the hold store holds. The
/// sink files holds from one spawned task per webhook connection while
/// the inbox's accept and decline each load, mutate and save in their
/// own blocking task: unlocked, two accepts can both take the same key
/// and run the same inbound twice, an accept's stale write-back can
/// erase a hold a concurrent connection just filed, and an accept and a
/// decline can both report success over one hold. Every mutation below
/// — `file_locked`, `take_locked` — is one locked load → mutate → save
/// step, so a second take of the same id fails rather than succeeds.
static HOLD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn hold_lock() -> std::sync::MutexGuard<'static, ()> {
    HOLD_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

impl HoldStore {
    /// The store's path: `<home>/channels/held.json`.
    pub fn path(home: &Path) -> PathBuf {
        home.join("channels").join("held.json")
    }

    /// Read the store. A home that has never held a delivery is an empty
    /// store, not an error. A file that does not parse is refused, not
    /// silently emptied: a hold lost to a bad read is a delivery the
    /// owner can neither accept nor decline, and the caller says so.
    pub fn load(home: &Path) -> std::io::Result<HoldStore> {
        let path = Self::path(home);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} is not readable: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HoldStore::default()),
            Err(error) => Err(error),
        }
    }

    /// Write the store, privately and atomically: it names who asked to
    /// reach the owner, and what they asked.
    pub fn save(&self, home: &Path) -> std::io::Result<()> {
        let path = Self::path(home);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
        })?;
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(text.as_bytes())?;
            file.flush()?;
        }
        std::fs::rename(&temp, &path)
    }

    /// File one delivery, newest first, and keep the store bounded. The
    /// id is minted here so the caller does not have to: the running
    /// sequence names two holds apart within the same nanosecond, and
    /// the FULL epoch nanos (the review's note 5) keep the tail from
    /// repeating every second or across a restart.
    pub fn file(&mut self, inbound: &Inbound, at: &str) -> String {
        let seq = HOLD_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let id = format!("h{seq:04}{nanos:019}");
        self.holds.insert(
            0,
            Hold {
                id: id.clone(),
                channel: inbound.peer.channel.clone(),
                peer_id: inbound.peer.id.clone(),
                peer: inbound.peer.name.clone(),
                session: inbound.session.clone(),
                task: inbound.task.clone(),
                callback: match &inbound.reply {
                    Reply::Url(url) => Some(url.clone()),
                    Reply::None => None,
                },
                at: at.to_string(),
            },
        );
        self.holds.truncate(HOLD_CAP);
        id
    }

    /// Take one hold out by its id: removed for the accept or decline
    /// that names it, gone either way. Returns None when no hold is
    /// keyed so — a stale key is a refusal, never a guess.
    pub fn take(&mut self, id: &str) -> Option<Hold> {
        let at = self.holds.iter().position(|hold| hold.id == id)?;
        Some(self.holds.remove(at))
    }

    /// File one delivery through the process-wide lock: the locked unit
    /// is load → file → save, so a delivery filed while an accept is
    /// anywhere in its own locked step is read fresh by that step and
    /// survives it, and two filings never race one write-back.
    pub fn file_locked(home: &Path, inbound: &Inbound, at: &str) -> std::io::Result<String> {
        let _guard = hold_lock();
        let mut store = Self::load(home)?;
        let id = store.file(inbound, at);
        store.save(home)?;
        Ok(id)
    }

    /// Take one hold out through the process-wide lock, persisting the
    /// take before the caller does anything with it. A second decision
    /// of the same key therefore finds the store already without it —
    /// a refusal, never a second run — and a hold a connection files
    /// meanwhile is never erased by a stale write-back, because there
    /// is no stale write-back: the take is the write-back. Returns
    /// Ok(None) when no hold is keyed so; an error leaves the store as
    /// it was, the hold still in it.
    pub fn take_locked(home: &Path, id: &str) -> Result<Option<Hold>, std::io::Error> {
        let _guard = hold_lock();
        let mut store = Self::load(home)?;
        let hold = store.take(id);
        if hold.is_some() {
            store.save(home)?;
        }
        Ok(hold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-held-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn inbound(task: &str, callback: Option<&str>) -> Inbound {
        Inbound {
            peer: Peer::new("webhook", "abc", "the CI box"),
            session: None,
            task: task.to_string(),
            reply: match callback {
                Some(url) => Reply::Url(url.to_string()),
                None => Reply::None,
            },
        }
    }

    /// The hold carries the delivery in full, and the reconstruction is
    /// the same Inbound the sink was handed — the precondition for an
    /// accept to be the same delivery the auto-run would have run.
    #[test]
    fn a_hold_carries_the_delivery_in_full() {
        let home = temp_home("carry");
        let mut store = HoldStore::default();
        let id = store.file(
            &inbound("run the tests", Some("http://127.0.0.1:9/cb")),
            "at",
        );
        assert!(id.starts_with('h'));
        store.save(&home).unwrap();
        let reloaded = HoldStore::load(&home).unwrap();
        assert_eq!(reloaded.holds.len(), 1);
        let hold = &reloaded.holds[0];
        assert_eq!(hold.id, id);
        assert_eq!(hold.channel, "webhook");
        assert_eq!(hold.peer_id, "abc");
        assert_eq!(hold.peer, "the CI box");
        assert_eq!(hold.task, "run the tests");
        assert_eq!(hold.callback.as_deref(), Some("http://127.0.0.1:9/cb"));
        let back = hold.to_inbound();
        assert_eq!(back.peer.id, "abc");
        assert_eq!(back.task, "run the tests");
        assert_eq!(back.reply, Reply::Url("http://127.0.0.1:9/cb".to_string()));
        std::fs::remove_dir_all(&home).ok();
    }

    /// A hold with no reply reconstructs as Reply::None, and the take
    /// removes exactly the hold it names.
    #[test]
    fn a_take_removes_the_hold_it_names_and_no_other() {
        let home = temp_home("take");
        let mut store = HoldStore::default();
        let first = store.file(&inbound("one", None), "at");
        let second = store.file(&inbound("two", None), "at");
        assert_eq!(store.holds.len(), 2);
        let held = store.take(&first).expect("the first hold");
        assert_eq!(held.task, "one");
        assert_eq!(held.to_inbound().reply, Reply::None);
        assert_eq!(store.holds.len(), 1);
        assert_eq!(store.holds[0].id, second);
        assert!(store.take(&first).is_none(), "already taken");
        std::fs::remove_dir_all(&home).ok();
    }

    /// The store is bounded: a flooding peer cannot grow the file
    /// without limit, and what the cap drops is the stalest hold.
    #[test]
    fn the_hold_store_is_capped() {
        let home = temp_home("cap");
        let mut store = HoldStore::default();
        for n in 0..(HOLD_CAP + 5) {
            store.file(&inbound(&format!("task-{n}"), None), "at");
        }
        assert_eq!(store.holds.len(), HOLD_CAP);
        assert!(
            !store.holds.iter().any(|h| h.task == "task-0"),
            "the stalest hold is what the cap drops"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    // ---- the locked store (the adversarial review's FIX 1) ----

    /// The id's tail is the FULL epoch nanos (the review's note 5): a
    /// sub-second tail repeats every second and across restarts, so two
    /// holds minted a second apart — or by two processes — could name
    /// themselves the same.
    #[test]
    fn a_hold_id_carries_the_full_epoch_nanos() {
        let mut store = HoldStore::default();
        let id = store.file(&inbound("one", None), "at");
        assert!(id.starts_with('h'));
        // h + the four seq digits, then the nanos.
        let nanos: u128 = id[5..].parse().expect("the id's numeric tail");
        assert!(
            nanos > 1_000_000_000_000_000_000,
            "the id's tail is a sub-second number that repeats every second: {id}"
        );
    }

    /// A second take of the same id FAILS: the take was persisted by the
    /// first, so the store is already without it. Two successes here is
    /// the double-run.
    #[test]
    fn a_second_take_of_the_same_id_finds_nothing() {
        let home = temp_home("double-take");
        let mut store = HoldStore::default();
        let id = store.file(&inbound("run once", None), "at");
        store.save(&home).unwrap();
        let first = HoldStore::take_locked(&home, &id)
            .unwrap()
            .expect("the first take");
        assert_eq!(first.task, "run once");
        let second = HoldStore::take_locked(&home, &id).unwrap();
        assert!(
            second.is_none(),
            "the second take of the same id succeeded again: {second:?}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    /// A hold the sink files while an accept is inside its run survives
    /// the accept: the accept's take was persisted before the run, its
    /// re-file reads fresh, and there is no stale write-back to erase
    /// the filing with. The reverse ordering — the filing first, the
    /// accept's take after — is the same lock reading fresh.
    #[test]
    fn a_file_that_lands_during_an_accept_survives_it() {
        let home = temp_home("clobber");
        let mut store = HoldStore::default();
        let id = store.file(&inbound("the accept's delivery", None), "at");
        store.save(&home).unwrap();
        // The accept: take, then the run (the sleep here), then the
        // re-file a refused accept does — each step its own locked unit.
        let accept_home = home.clone();
        let accept = std::thread::spawn(move || {
            let hold = HoldStore::take_locked(&accept_home, &id)
                .unwrap()
                .expect("the accept's hold");
            // The run, in flight while the connection files its own.
            std::thread::sleep(std::time::Duration::from_millis(60));
            HoldStore::file_locked(&accept_home, &hold.to_inbound(), &hold.at).unwrap()
        });
        // Give the accept its take, then file as the webhook's other
        // connection does.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let filed = HoldStore::file_locked(&home, &inbound("filed mid-accept", None), "at")
            .expect("the filing");
        let refiled = accept.join().unwrap();
        let store = HoldStore::load(&home).unwrap();
        let ids: Vec<&str> = store.holds.iter().map(|hold| hold.id.as_str()).collect();
        assert!(
            ids.contains(&filed.as_str()),
            "the hold the sink filed during the accept was clobbered: {ids:?}"
        );
        assert!(ids.contains(&refiled.as_str()));
        assert_eq!(store.holds.len(), 2);
        std::fs::remove_dir_all(&home).ok();
    }
}
