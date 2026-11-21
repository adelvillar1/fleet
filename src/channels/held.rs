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
    /// id is minted here so the caller does not have to.
    pub fn file(&mut self, inbound: &Inbound, at: &str) -> String {
        let seq = HOLD_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let id = format!("h{seq:04}{nanos:09}");
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
}
