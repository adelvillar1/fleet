//! The channel service: a transport plugin carrying work from outside.
//!
//! A channel is not a tool lane. The five vendored lanes in
//! [`crate::lanes`] are stateless children speaking stdio JSON-RPC that
//! contribute *tools* to a run; a channel is a long-lived connection with
//! a peer identity, a pairing, and inbound events that *start* runs. Same
//! architectural principle — everything is a plugin — different
//! lifecycle, so this is a sibling registry rather than an entry in that
//! one.
//!
//! ## The shape
//!
//! ```text
//!   telegram plugin ─┐
//!   webhook plugin ──┼──► Driver ──► Bridge (one session per peer) ──► agent
//!   discord plugin ──┘      │
//!                           └── the seat, raised once, presented on the
//!                               channel and in the GUI inbox alike
//! ```
//!
//! The driver is where the two halves meet. An inbound delivery names a
//! peer, and the peer's capability is checked in exactly one place — the
//! resolver below, the discipline [`crate::approvals::resolve`] holds for
//! a run's mode — because a second wiring site for "may this peer act"
//! is a second answer to the same question. A paired peer's task runs as
//! an ordinary run in that peer's own session log: the same agent, the
//! same log format, the same seat. Nothing here is a second path to the
//! core.
//!
//! ## The seat on a channel
//!
//! A seat is raised once, in the session it belongs to, and presented
//! everywhere that session has a surface: the GUI inbox the bridge
//! already serves, and the channel the delivery arrived on. Answering it
//! anywhere takes it — the bridge guarantees one answer — so a phone and
//! a Telegram chat cannot both approve the same call. What a channel
//! cannot do is retract a notification it has already sent, and this
//! module does not pretend otherwise: the seat's *state* is single, and
//! the channel's copy of the news is a copy.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod service;
pub mod telegram;
pub mod webhook;

pub use service::{ChannelReport, ChannelService, Config as ServiceConfig, Phase, Status};

use crate::gui::bridge::Bridge;

/// The channels this build ships.
///
/// The registry is a list of names rather than a table of the plugins
/// that happen to be running, because the question "who can reach this
/// face" has to be answerable by a process that is not hosting the
/// service: `fleet channels` and the GUI's access read both walk the
/// stores on disk, and a channel that is stopped still has paired peers
/// who are still paired. A name here is a promise that a plugin with
/// that name reads and writes `<channel>-peers.json`.
pub const CHANNELS: &[&str] = &["webhook", "telegram"];

/// Who is on the far end of a channel: the channel's name, the peer's own
/// id on it, and the name the peer paired under. A peer is identified by
/// the channel and the id, because the same human on Telegram and on a
/// webhook is two peers with one owner — and the name rides along
/// because every surface that shows a peer shows the name, not the
/// digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Peer {
    pub channel: String,
    pub id: String,
    pub name: String,
}

impl Peer {
    pub fn new(channel: &str, id: &str, name: &str) -> Peer {
        Peer {
            channel: channel.to_string(),
            id: id.to_string(),
            name: name.to_string(),
        }
    }

    /// The key this peer's session mapping is filed under.
    fn key(&self) -> String {
        format!("{}:{id}", self.channel, id = self.id)
    }
}

/// What a peer may do. The distinction that matters: a peer that can
/// hand Fleet a task is a peer that can spend the owner's tokens and
/// write to the owner's tree, and a peer that can only be told things is
/// not. There is no third level — a peer either acts or listens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// May hand Fleet a task and receive the outcome.
    Task,
    /// May only be notified. The default for a peer that has paired but
    /// not been granted the acting capability.
    Observe,
}

/// Why a delivery was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The peer is not paired at all.
    Unknown,
    /// The peer is paired but may only listen.
    ObserveOnly,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Unknown => write!(out, "that peer is not paired"),
            Refusal::ObserveOnly => {
                write!(
                    out,
                    "that peer may only listen; it cannot hand fleet a task"
                )
            }
        }
    }
}

/// One paired peer, as the store keeps it. The token is not here — only
/// its SHA-256 is, the same rule the device store holds: a stolen peers
/// file is a roster of names and digests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedPeer {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub capability: Capability,
    pub created_at: String,
    /// Where outcomes and seats go when a delivery names no callback of
    /// its own. A webhook client may name the callback per delivery — the
    /// body's own field wins — and this is the default it falls back to.
    /// It is a URL, not a credential, so it may be re-read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback: Option<String>,
}

/// The paired peers of one channel.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PeerStore {
    #[serde(default)]
    pub peers: Vec<PairedPeer>,
}

impl PeerStore {
    /// The store's path: `<home>/channels/<channel>-peers.json`.
    pub fn path(home: &Path, channel: &str) -> PathBuf {
        home.join("channels").join(format!("{channel}-peers.json"))
    }

    /// Read the store. A home with no channel directory yet is an empty
    /// store, not an error: the service is opt-in, and a machine that
    /// has never configured a channel has nothing to read.
    pub fn load(home: &Path, channel: &str) -> std::io::Result<PeerStore> {
        let path = Self::path(home, channel);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} is not readable: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(PeerStore::default()),
            Err(error) => Err(error),
        }
    }

    /// Write the store back, privately: it names who may act.
    pub fn save(&self, home: &Path, channel: &str) -> std::io::Result<()> {
        let path = Self::path(home, channel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
        })?;
        {
            let mut file = std::fs::File::create(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            }
            use std::io::Write;
            file.write_all(text.as_bytes())?;
            file.flush()?;
        }
        std::fs::rename(&temp, &path)
    }

    /// Change a paired peer's capability. Returns false when no peer has
    /// that id, so a surface says "no such peer" rather than writing a
    /// roster entry that is not there.
    pub fn set_capability(&mut self, id: &str, capability: Capability) -> bool {
        match self.peers.iter_mut().find(|peer| peer.id == id) {
            Some(peer) => {
                peer.capability = capability;
                true
            }
            None => false,
        }
    }

    /// Pair a peer: the token is hashed here and never stored. The id is
    /// the digest's prefix, which is unique without a second random draw.
    pub fn pair(&mut self, id: &str, name: &str, token: &str, capability: Capability, stamp: &str) {
        self.pair_with(id, name, token, capability, stamp, None);
    }

    /// The same pairing, with the peer's default callback. The plain
    /// `pair` is this with none, so the store has one write path and the
    /// tests that do not care about callbacks keep their shape.
    #[allow(clippy::too_many_arguments)]
    pub fn pair_with(
        &mut self,
        id: &str,
        name: &str,
        token: &str,
        capability: Capability,
        stamp: &str,
        callback: Option<&str>,
    ) {
        let token_hash = crate::remote::pairing::hash_hex(token.as_bytes());
        self.peers.retain(|peer| peer.id != id);
        self.peers.push(PairedPeer {
            id: id.to_string(),
            name: name.to_string(),
            token_hash,
            capability,
            created_at: stamp.to_string(),
            callback: callback.map(str::to_string),
        });
    }

    /// The peer a presented token belongs to, comparing the digest of
    /// what arrived against every stored one in constant time.
    pub fn peer_for_token(&self, token: &str) -> Option<&PairedPeer> {
        let arrived = crate::remote::pairing::hash_hex(token.as_bytes());
        self.peers.iter().find(|peer| {
            crate::remote::pairing::digest_eq(peer.token_hash.as_bytes(), arrived.as_bytes())
        })
    }

    /// Forget a peer by id. True when one was actually removed.
    pub fn revoke(&mut self, id: &str) -> bool {
        let before = self.peers.len();
        self.peers.retain(|peer| peer.id != id);
        self.peers.len() != before
    }
}

/// The one place a peer's capability is decided. The store is the
/// source of truth; this reads it and answers, so a hand-edit between a
/// delivery and the next is respected exactly as the remembered
/// approval rules are.
#[derive(Debug, Default)]
pub struct Policy {
    home: Option<PathBuf>,
}

impl Policy {
    /// A policy over one home.
    pub fn new(home: PathBuf) -> Policy {
        Policy { home: Some(home) }
    }

    /// What this peer may do, or None when it is not paired at all.
    pub fn capability(&self, peer: &Peer) -> Option<Capability> {
        let home = self.home.as_ref()?;
        let store = PeerStore::load(home, &peer.channel).ok()?;
        store
            .peers
            .iter()
            .find(|paired| paired.id == peer.id)
            .map(|paired| paired.capability)
    }

    /// The check a delivery passes before it is allowed to act: paired,
    /// and allowed to hand tasks. An unpaired peer is refused by name,
    /// because a channel that silently drops a delivery is a channel
    /// nobody can debug.
    pub fn authorize(&self, peer: &Peer) -> Result<Capability, Refusal> {
        match self.capability(peer) {
            Some(Capability::Task) => Ok(Capability::Task),
            Some(Capability::Observe) => Err(Refusal::ObserveOnly),
            None => Err(Refusal::Unknown),
        }
    }
}

/// The peer a mapping key names, or `None` when the key is not a key
/// this build would have written. The key is `channel:id`, which is the
/// same string [`Peer::key`] builds; parsing it back is safe because a
/// channel name is one of [`CHANNELS`] and a peer id carries no colon.
fn peer_of(key: &str) -> Option<Peer> {
    let (channel, id) = key.split_once(':')?;
    if !CHANNELS.contains(&channel) || id.is_empty() {
        return None;
    }
    Some(Peer::new(channel, id, ""))
}

/// Whether the log at `id` is this peer's own conversation: it exists,
/// it is readable, and its channel record names this channel and this
/// peer's id. The name is not checked — a peer may be renamed and the
/// conversation is still theirs, and two peers may share a name.
fn log_carries(home: &Path, id: &str, peer: &Peer) -> bool {
    let path = crate::session::session_path(home, id);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    text.lines().any(|line| {
        matches!(
            serde_json::from_str::<crate::session::Record>(line.trim()),
            Ok(crate::session::Record::Channel { channel, peer_id, .. })
                if channel == peer.channel && peer_id == peer.id
        )
    })
}

/// A peer that tried to reach the service and is not paired.
///
/// The refusal path records it here so the owner can let it in from the
/// app: hunting a numeric chat id out of a log to type into a terminal
/// is the step that makes a channel feel like a config file rather than
/// a surface. Only a channel whose peer identifies itself before pairing
/// can record one — Telegram's chat id does, and the webhook's unknown
/// token does not, which is why the webhook's attempt is an activity
/// line rather than a peer to approve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPeer {
    /// The peer's id on its channel.
    pub id: String,
    /// The name it announced, if it announced one.
    pub name: String,
    /// How many deliveries it has attempted. A peer that keeps trying is
    /// one the owner probably means to let in.
    pub attempts: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// How many pending peers one channel remembers. A stranger that floods
/// the channel must not grow a file without bound.
pub const PENDING_CAP: usize = 20;

/// The unpaired peers of one channel, newest first.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PendingStore {
    #[serde(default)]
    pub peers: Vec<PendingPeer>,
}

impl PendingStore {
    /// The store's path: `<home>/channels/<channel>-pending.json`.
    pub fn path(home: &Path, channel: &str) -> PathBuf {
        home.join("channels")
            .join(format!("{channel}-pending.json"))
    }

    /// Read it. A home with nothing pending has an empty store.
    pub fn load(home: &Path, channel: &str) -> PendingStore {
        std::fs::read_to_string(Self::path(home, channel))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Write it, privately and atomically: it names who tried.
    pub fn save(&self, home: &Path, channel: &str) -> std::io::Result<()> {
        let path = Self::path(home, channel);
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

    /// Record one attempt: upsert by id, count it, and keep the store
    /// bounded. The newest is first, so the cap drops the stalest.
    pub fn note(&mut self, id: &str, name: &str, stamp: &str) {
        match self.peers.iter_mut().find(|peer| peer.id == id) {
            Some(peer) => {
                peer.attempts += 1;
                peer.last_seen = stamp.to_string();
                if !name.trim().is_empty() && peer.name.trim().is_empty() {
                    peer.name = name.trim().to_string();
                }
            }
            None => self.peers.insert(
                0,
                PendingPeer {
                    id: id.to_string(),
                    name: name.trim().to_string(),
                    attempts: 1,
                    first_seen: stamp.to_string(),
                    last_seen: stamp.to_string(),
                },
            ),
        }
        self.peers.truncate(PENDING_CAP);
    }

    /// Forget one, for a surface that refused to let it in.
    pub fn forget(&mut self, id: &str) -> bool {
        let before = self.peers.len();
        self.peers.retain(|peer| peer.id != id);
        self.peers.len() != before
    }
}

/// One pairing's outcome: the peer's id and the token, which is the only
/// copy — the store keeps the digest, so a surface shows this once and
/// never reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub channel: String,
    pub id: String,
    pub name: String,
    pub capability: Capability,
    /// The token, exactly once. `None` for a channel whose credential is
    /// the platform's own (Telegram's chat id), which has nothing to
    /// reveal.
    pub token: Option<String>,
    /// The callback stored with the peer, when one was named. Not a
    /// secret: the surface says where the outcomes will go.
    pub callback: Option<String>,
}

/// Pair a peer server-side: mint the token when the channel needs one,
/// store only its digest, and hand the token back to be shown once.
///
/// This is the one place a peer is created, so the CLI and the app
/// cannot disagree about what pairing means — and the token's only exit
/// is this return value.
pub fn pair(
    home: &Path,
    channel: &str,
    id: &str,
    name: &str,
    capability: Capability,
    callback: Option<&str>,
) -> Result<Pairing, String> {
    if !CHANNELS.contains(&channel) {
        return Err(format!("`{channel}` is not a channel this build ships"));
    }
    // The callback is a URL the service will POST to, so it is checked
    // here rather than at the first delivery: a typo paired today would
    // otherwise surface as a failed reply nobody sees. Telegram's reply
    // goes to the chat id, so a callback there is refused by name
    // instead of stored and ignored.
    let callback = callback.map(str::trim).filter(|url| !url.is_empty());
    if let Some(url) = callback {
        if channel != "webhook" {
            return Err(format!(
                "`{channel}` replies to its own handle; a callback belongs to a webhook peer"
            ));
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("a callback must be an http(s) URL".to_string());
        }
    }
    let name = name.trim();
    let name = if name.is_empty() { id } else { name };
    let stamp = crate::session::iso_utc_stamp(std::time::SystemTime::now())
        .unwrap_or_else(|_| String::from("unknown"));
    let mut store = PeerStore::load(home, channel).map_err(|error| error.to_string())?;
    // A webhook peer's credential is a token Fleet mints and reveals
    // once, and its id is the digest's prefix — unique without a second
    // random draw, and not the token itself. A Telegram peer's id is the
    // chat id the platform vouches for, so the id is the caller's and
    // there is nothing to reveal; the store still wants a digest slot,
    // so a token is minted and discarded because nothing presents it.
    let asked = id.to_string();
    let (id, credential, token) = if channel == "webhook" {
        let token = mint_peer_token()?;
        let id = crate::remote::pairing::hash_hex(token.as_bytes())[..12].to_string();
        (id, token.clone(), Some(token))
    } else {
        (asked.clone(), mint_peer_token()?, None)
    };
    store.pair_with(&id, name, &credential, capability, &stamp, callback);
    store
        .save(home, channel)
        .map_err(|error| error.to_string())?;
    // A peer that is now paired is no longer pending. The pending entry
    // is filed under the id the caller asked about — the chat id — which
    // is the minted id for every channel but the webhook.
    let mut pending = PendingStore::load(home, channel);
    let cleared = pending.forget(&asked) || pending.forget(&id);
    if cleared {
        let _ = pending.save(home, channel);
    }
    Ok(Pairing {
        channel: channel.to_string(),
        id,
        name: name.to_string(),
        capability,
        token,
        callback: callback.map(str::to_string),
    })
}

/// Change a paired peer's capability. Returns whether the peer exists —
/// a surface says "no such peer" rather than writing a roster entry that
/// is not there.
pub fn set_capability(
    home: &Path,
    channel: &str,
    id: &str,
    capability: Capability,
) -> Result<(), String> {
    let mut store = PeerStore::load(home, channel).map_err(|error| error.to_string())?;
    if !store.set_capability(id, capability) {
        return Err(format!("no paired {channel} peer named `{id}`"));
    }
    store.save(home, channel).map_err(|error| error.to_string())
}

/// The peer→session mapping on disk: `<home>/channels/sessions.json`.
///
/// In memory the mapping is a cache; on disk it is the truth, because a
/// restart that forgot it would start every peer a fresh conversation
/// and the archive would hold one conversation per restart per peer.
/// Every entry is validated when it is read back — the log has to exist
/// and carry that peer's own channel record — so a mapping that has
/// gone stale is dropped rather than believed.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionMap {
    #[serde(default)]
    peers: std::collections::HashMap<String, String>,
}

impl SessionMap {
    /// The map's path.
    pub fn path(home: &Path) -> PathBuf {
        home.join("channels").join("sessions.json")
    }

    /// Read the map. A home with no channel directory is an empty map,
    /// and an unreadable one is an empty map too: the mapping is a
    /// convenience over a truth the archive already holds, so losing it
    /// costs a peer its continuity and nothing else.
    pub fn load(home: &Path) -> SessionMap {
        match std::fs::read_to_string(Self::path(home)) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => SessionMap::default(),
        }
    }

    /// Write the map back, privately and atomically.
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
            let mut file = std::fs::File::create(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            }
            use std::io::Write;
            file.write_all(text.as_bytes())?;
            file.flush()?;
        }
        std::fs::rename(&temp, &path)
    }
}

/// Where an outcome goes: the channel's own handle for the peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// A URL the outcome is POSTed to.
    Url(String),
    /// Nowhere: the delivery asked for nothing back.
    None,
}

/// One inbound delivery, mapped by a channel plugin from whatever the
/// platform's own message looks like.
#[derive(Debug, Clone)]
pub struct Inbound {
    pub peer: Peer,
    /// The session this delivery belongs to. None means the peer's own
    /// session — the one conversation this peer has with Fleet — which
    /// the driver creates on first sight.
    pub session: Option<String>,
    pub task: String,
    pub reply: Reply,
}

/// The card a seat presents on a channel: what is waiting, and where
/// the answer goes.
#[derive(Debug, Clone)]
pub struct SeatCard {
    pub session: String,
    pub tool: String,
    pub summary: String,
}

/// A channel plugin. One implementation per platform; the driver and the
/// policy are shared by all of them.
pub trait Channel: Send + Sync {
    /// The channel's name, which is also its store's filename and the
    /// first half of a peer's key.
    fn name(&self) -> String;

    /// Serve inbound deliveries until the shutdown fires, handing each to
    /// `sink`. The plugin owns its own connection, its own reconnect,
    /// and its own authentication — the driver never sees a credential.
    ///
    /// `ready` is how "live" becomes a fact rather than a hope: the
    /// plugin sends `Ok(port)` exactly once, after the thing that could
    /// fail has succeeded — a bind for a listener, the first answered
    /// poll for a long-poller — or `Err(reason)` when it could not come
    /// up at all. A supervisor that cannot tell those apart says "live"
    /// about a socket nothing is listening on, which is the defect this
    /// signature exists to make impossible.
    ///
    /// Spawned onto the caller's runtime: a channel is a task of the
    /// face that hosts it, exactly as the consolidator and the scheduler
    /// are, and building a second runtime inside a tokio context is a
    /// panic rather than a design. The boxed future is what keeps the
    /// trait object-safe — a plugin is chosen at run time, so the trait
    /// has to be usable as `dyn Channel`.
    fn serve(
        &self,
        sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>;

    /// Send an outcome back to the peer.
    fn reply(&self, to: &Reply, text: &str);

    /// Present a seat on the channel. The answer comes back through the
    /// plugin's own inbound path and lands in the same session's seat.
    fn present_seat(&self, to: &Reply, seat: &SeatCard);
}

/// The driver: one inbound delivery becomes one run in the peer's own
/// session, with the outcome sent back and the seat presented on the
/// channel it arrived on.
pub struct Driver {
    bridge: Arc<Bridge>,
    policy: Policy,
    /// Which session each peer's conversation lives in and where its
    /// outcomes go, filed under the peer's key. The disk map is the
    /// truth and this is its cache: a restart that started every peer a
    /// fresh conversation would leave the archive holding one
    /// conversation per restart per peer, which is a history nobody
    /// asked for.
    sessions: Mutex<HashMap<String, (String, Reply)>>,
    /// Where each peer's outcomes go, filed under the peer's key. The
    /// delivery named the callback; this remembers it so a seat raised
    /// three turns later still knows.
    replies: Mutex<HashMap<String, Reply>>,
}

impl Driver {
    /// A driver over one bridge and one policy. The peer→session mapping
    /// is read back from the home and every entry is validated before it
    /// is believed: the log has to still exist and still carry that
    /// peer's own channel record, because a mapping that has gone stale
    /// would hand a peer somebody else's conversation.
    pub fn new(bridge: Arc<Bridge>, policy: Policy) -> Driver {
        let home = bridge.home_path();
        let mut sessions = HashMap::new();
        for (key, id) in SessionMap::load(&home).peers {
            if let Some(peer) = peer_of(&key) {
                if log_carries(&home, &id, &peer) {
                    sessions.insert(key, (id, Reply::None));
                }
            }
        }
        Driver {
            bridge,
            policy,
            sessions: Mutex::new(sessions),
            replies: Mutex::new(HashMap::new()),
        }
    }

    /// Remember a peer's session on disk as well as in memory, so the
    /// next process resumes this conversation.
    fn remember(&self, key: &str, id: &str) {
        let home = self.bridge.home_path();
        let mut map = SessionMap::load(&home);
        map.peers.insert(key.to_string(), id.to_string());
        // A disk that will not take the mapping is a continuity lost, not
        // a delivery refused: the run is already in flight and the peer
        // is served either way.
        if let Err(error) = map.save(&home) {
            eprintln!("fleet channels: the session mapping did not save: {error}");
        }
    }

    /// The bridge this driver runs through.
    pub fn bridge(&self) -> &Arc<Bridge> {
        &self.bridge
    }

    /// The session a peer's conversation lives in, created on first
    /// sight. A peer that names a session explicitly keeps it: the
    /// platform may thread its own conversations, and Fleet follows.
    pub fn session_for(&self, peer: &Peer, asked: Option<&str>) -> Result<String, String> {
        if let Some(asked) = asked {
            if crate::gui::bridge::session_id_is_shape(asked) {
                return Ok(asked.to_string());
            }
            return Err(format!("`{asked}` is not a session id"));
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some((session, _)) = sessions.get(&peer.key()) {
            return Ok(session.clone());
        }
        // A fresh log for this peer's first task: the same convention
        // every other surface creates one with.
        let home = self.bridge.home_path();
        let mut session = crate::session::Session::create(&home, &self.bridge.model())
            .map_err(|error| format!("could not create the session: {error}"))?;
        // The log says which channel this conversation arrived on and
        // who the peer is: an inbox that cannot name them is a list of
        // stamps, and the owner cannot tell a reply here from a message
        // into the void.
        if let Err(error) = session.record(&crate::session::Record::Channel {
            channel: peer.channel.clone(),
            peer_id: peer.id.clone(),
            peer: peer.name.clone(),
        }) {
            return Err(format!("could not record the channel: {error}"));
        }
        let id = session.id().to_string();
        let reply = self
            .replies
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(&peer.key())
            .cloned()
            .unwrap_or(Reply::None);
        sessions.insert(peer.key(), (id.clone(), reply));
        drop(sessions);
        self.remember(&peer.key(), &id);
        Ok(id)
    }

    /// Watch the bridge's stream for this driver's sessions and present
    /// what happens on the channel: a seat when one is raised, the
    /// outcome when the run closes.
    ///
    /// One task, subscribed to the same stream the browser watches, so a
    /// seat raised in a peer's session is presented on the channel the
    /// peer arrived on and in the GUI inbox alike — raised once, seen
    /// everywhere. The events name their session, which is what makes
    /// the filtering exact rather than a guess about whose turn it was.
    pub async fn watch(&self, channels: &[Arc<dyn Channel>]) {
        let mut events = self.bridge.subscribe();
        loop {
            match events.recv().await {
                Ok(event) => {
                    let Some(session) = event.get("session").and_then(Value::as_str) else {
                        continue;
                    };
                    // The key names the channel and the peer, so the
                    // plugin that carries the reply is the one the
                    // conversation arrived on — not the only one there
                    // is. A face hosting two channels routes each peer's
                    // seats to the transport that can actually deliver
                    // them.
                    let Some((key, reply)) = self
                        .sessions
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .iter()
                        .find(|(_, (owned, _))| owned == session)
                        .map(|(key, (_, reply))| (key.clone(), reply.clone()))
                    else {
                        continue;
                    };
                    let Some(channel) = peer_of(&key).and_then(|peer| {
                        channels
                            .iter()
                            .find(|channel| channel.name() == peer.channel)
                            .cloned()
                    }) else {
                        continue;
                    };
                    let announce = peer_of(&key);
                    match event.get("type").and_then(Value::as_str) {
                        Some("seat") => {
                            let card = SeatCard {
                                session: session.to_string(),
                                tool: event
                                    .get("tool")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string(),
                                summary: event
                                    .get("summary")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string(),
                            };
                            channel.present_seat(&reply, &card);
                            if let Some(peer) = &announce {
                                crate::channels::service::note_seat(
                                    &self.bridge.home_path(),
                                    &peer.channel,
                                    &peer.id,
                                    &card,
                                );
                            }
                        }
                        Some("finished") | Some("failed") => {
                            // The outcome is the last thing the session
                            // said: the log is the record, and a reply
                            // that paraphrases it is a second answer to
                            // what the run produced.
                            if let Some(text) = self.last_words(session) {
                                channel.reply(&reply, &text);
                                if let Some(peer) = &announce {
                                    crate::channels::service::note_reply(
                                        &self.bridge.home_path(),
                                        &peer.channel,
                                        &peer.id,
                                        &reply,
                                        &text,
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    /// The session's last assistant message, which is what a run closed
    /// with. Read off the log rather than accumulated from the stream:
    /// the log is the record of what was said, and a lagged stream must
    /// not produce a reply missing its ending.
    fn last_words(&self, session: &str) -> Option<String> {
        let home = self.bridge.home_path();
        let session = session.to_string();
        let path = crate::session::session_path(&home, &session);
        let records = crate::session::replay(&path).ok()?;
        records.iter().rev().find_map(|record| match record {
            crate::session::Record::Assistant { content, .. } => content.clone(),
            _ => None,
        })
    }

    /// One inbound delivery, end to end: the peer is authorized, the
    /// task runs in the peer's session, and the outcome goes back. The
    /// run itself is asynchronous — the driver hands it to the bridge's
    /// agent thread exactly as the composer does — so a delivery is
    /// accepted immediately and its outcome arrives on the session's
    /// stream, which the channel watches for the reply.
    pub fn deliver(&self, inbound: &Inbound) -> Result<String, Refusal> {
        if let Err(refusal) = self.policy.authorize(&inbound.peer) {
            // An unpaired peer is remembered as pending, in the one place
            // the decision is made: a surface can then let it in without
            // the owner hunting a numeric id out of a log. A peer that is
            // paired but may only listen is not pending — it is already
            // in, and its capability is the thing to edit.
            if refusal == Refusal::Unknown {
                let home = self.bridge.home_path();
                let stamp = crate::session::iso_utc_stamp(std::time::SystemTime::now())
                    .unwrap_or_else(|_| String::from("unknown"));
                let mut pending = PendingStore::load(&home, &inbound.peer.channel);
                pending.note(&inbound.peer.id, &inbound.peer.name, &stamp);
                let _ = pending.save(&home, &inbound.peer.channel);
            }
            return Err(refusal);
        }
        // Where this peer's outcomes go, remembered BEFORE the session
        // mapping is built: a mapping that carries no reply handle is a
        // conversation nothing can be posted back to, and the seat the
        // run raises three turns later would have nowhere to go.
        self.replies
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .insert(inbound.peer.key(), inbound.reply.clone());
        let session = self
            .session_for(&inbound.peer, inbound.session.as_deref())
            .map_err(|_| Refusal::Unknown)?;
        // The peer's message lands in the session's log first, exactly as
        // the composer's does, and the run is accepted or refused by the
        // same door.
        self.bridge
            .run(&session, &inbound.task)
            .map_err(|_| Refusal::Unknown)?;
        Ok(session)
    }

    /// Answer a seat on a peer's behalf: the same door the browser's
    /// approve button uses, so an answer here and an answer there are
    /// the same answer.
    pub fn answer(
        &self,
        session: &str,
        decision: &str,
        by: &crate::gui::bridge::AnsweredBy,
    ) -> Result<(), String> {
        self.bridge
            .answer(session, decision, by)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Test-only: where this peer's outcomes go. It pins the webhook's
    /// stored-callback default through the delivery path rather than
    /// only through the pure helper.
    #[cfg(test)]
    pub fn reply_to(&self, peer: &Peer) -> Option<Reply> {
        self.replies
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(&peer.key())
            .cloned()
    }
}

/// The Telegram bot token, read the way every credential is: the vault
/// over the dotenv over the environment, resolved once at start and
/// never carried in a payload. A machine with no token has no Telegram
/// channel — not a stopped one, an unconfigured one.
pub fn telegram_token(home: &Path) -> Option<String> {
    let dotenv = crate::home::read_dotenv_with_process_vault(home);
    crate::home::resolve_setting("FLEET_TELEGRAM_BOT_TOKEN", &dotenv)
        .or_else(|| std::env::var("FLEET_TELEGRAM_BOT_TOKEN").ok())
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

/// Mint a channel peer's token: 32 random bytes, base64url, the same
/// shape and entropy as a device token.
pub fn mint_peer_token() -> Result<String, String> {
    crate::remote::pairing::mint_token().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-channels-{tag}-{}-{}",
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

    #[test]
    fn a_pairing_stores_the_callback_and_refuses_a_bad_one() {
        let home = temp_home("callback");
        let pairing = pair(
            &home,
            "webhook",
            "",
            "the CI box",
            Capability::Task,
            Some("http://127.0.0.1:9/callback"),
        )
        .expect("paired");
        assert_eq!(
            pairing.callback.as_deref(),
            Some("http://127.0.0.1:9/callback")
        );
        // The store holds it, so a delivery that names none can still
        // answer — the value the CLI used to print and throw away.
        let store = PeerStore::load(&home, "webhook").unwrap();
        assert_eq!(
            store.peers[0].callback.as_deref(),
            Some("http://127.0.0.1:9/callback")
        );

        // A callback the service could not POST to is refused at the
        // pairing rather than at the first failed reply.
        assert!(pair(
            &home,
            "webhook",
            "",
            "x",
            Capability::Task,
            Some("file:///etc/passwd")
        )
        .is_err());
        // Telegram replies to its chat id; a callback is not its shape.
        assert!(pair(
            &home,
            "telegram",
            "42",
            "x",
            Capability::Task,
            Some("http://127.0.0.1:9/callback")
        )
        .is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_paired_peers_token_is_stored_as_a_digest_only() {
        let home = temp_home("store");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair(
            "abc",
            "the CI box",
            &token,
            Capability::Task,
            "2026-10-18T00:00:00Z",
        );
        store.save(&home, "webhook").unwrap();

        let on_disk = std::fs::read_to_string(PeerStore::path(&home, "webhook")).unwrap();
        assert!(!on_disk.contains(&token), "the token is on disk: {on_disk}");
        assert!(on_disk.contains("token_hash"), "{on_disk}");

        let reloaded = PeerStore::load(&home, "webhook").unwrap();
        assert_eq!(reloaded.peers.len(), 1);
        assert!(reloaded.peer_for_token(&token).is_some());
        assert!(reloaded.peer_for_token("not-the-token").is_none());
        assert_eq!(reloaded.peers[0].capability, Capability::Task);

        // Revoking forgets it and the token stops working.
        let mut store = reloaded;
        assert!(store.revoke("abc"));
        assert!(!store.revoke("abc"));
        assert!(store.peer_for_token(&token).is_none());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_policy_answers_from_the_store_and_refuses_by_name() {
        let home = temp_home("policy");
        let mut store = PeerStore::default();
        let actor = mint_peer_token().unwrap();
        let watcher = mint_peer_token().unwrap();
        store.pair("actor", "the actor", &actor, Capability::Task, "stamp");
        store.pair(
            "watcher",
            "the watcher",
            &watcher,
            Capability::Observe,
            "stamp",
        );
        store.save(&home, "webhook").unwrap();

        let policy = Policy::new(home.clone());
        let actor_peer = Peer::new("webhook", "actor", "the actor");
        let watcher_peer = Peer::new("webhook", "watcher", "the watcher");
        let stranger = Peer::new("webhook", "stranger", "a stranger");
        let other_channel = Peer::new("telegram", "actor", "the actor");

        assert_eq!(policy.authorize(&actor_peer), Ok(Capability::Task));
        assert_eq!(policy.authorize(&watcher_peer), Err(Refusal::ObserveOnly));
        assert_eq!(policy.authorize(&stranger), Err(Refusal::Unknown));
        // The same id on another channel is a different peer: a peer is
        // identified by the pair, not by the human.
        assert_eq!(policy.authorize(&other_channel), Err(Refusal::Unknown));

        // The refusal names itself.
        assert!(Refusal::Unknown.to_string().contains("not paired"));
        assert!(Refusal::ObserveOnly.to_string().contains("only listen"));
        std::fs::remove_dir_all(&home).ok();
    }

    /// A channel that records what it was shown, for the watcher's test:
    /// the seat cards presented and the replies sent, in order.
    struct Spy {
        shown: Mutex<Vec<(String, String)>>,
    }

    impl Spy {
        fn new() -> Arc<Spy> {
            Arc::new(Spy {
                shown: Mutex::new(Vec::new()),
            })
        }
    }

    impl Channel for Spy {
        /// The spy stands in for the webhook: the watcher routes a peer's
        /// seats to the transport its conversation arrived on, so a
        /// stand-in has to answer to that transport's name.
        fn name(&self) -> String {
            "webhook".to_string()
        }

        fn serve(
            &self,
            _sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
            ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
            _shutdown: tokio::sync::watch::Receiver<bool>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        {
            let _ = ready.send(Ok(0));
            Box::pin(async { Ok(()) })
        }

        fn reply(&self, to: &Reply, text: &str) {
            if let Reply::Url(url) = to {
                self.shown
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .push((url.clone(), text.to_string()));
            }
        }

        fn present_seat(&self, to: &Reply, seat: &SeatCard) {
            if let Reply::Url(url) = to {
                self.shown
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .push((url.clone(), format!("seat:{}:{}", seat.tool, seat.summary)));
            }
        }
    }

    /// The watcher presents a seat on the channel the delivery arrived
    /// on, and posts the outcome when the run closes — the two things
    /// that make a channel a surface rather than a fire-and-forget.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_watcher_presents_the_seat_and_posts_the_outcome() {
        let home = temp_home("watch");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair("abc", "the CI box", &token, Capability::Task, "stamp");
        store.save(&home, "webhook").unwrap();

        let (mut bridge, sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let door = bridge.connect();
        let driver = Arc::new(Driver::new(Arc::new(bridge), Policy::new(home.clone())));
        let spy = Spy::new();

        // A delivery that names a callback: the seat and the outcome both
        // have somewhere to go.
        let inbound = Inbound {
            peer: Peer::new("webhook", "abc", "the CI box"),
            session: None,
            task: "run the tests".to_string(),
            reply: Reply::Url("http://127.0.0.1:9/callback".to_string()),
        };
        let session = driver.deliver(&inbound).expect("accepted");

        // The watcher, subscribed to the same stream the browser watches.
        let watching = Arc::clone(&driver);
        let channel: Arc<dyn Channel> = Arc::clone(&spy) as Arc<dyn Channel>;
        tokio::spawn(async move { watching.watch(&[channel]).await });
        // The watcher subscribes on its own schedule; a broadcast does
        // not replay, so the seat waits until it is listening.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // The thread names the session it drives before the run's events
        // start; the test stands in for the thread, because a seat
        // belongs to the session being driven.
        driver.bridge().drive(&session);

        // A seat is raised in the peer's session, exactly as the agent
        // thread would raise it.
        let (responder, _reply) = std::sync::mpsc::channel();
        sources
            .seat
            .send(crate::agent::ApprovalRequest {
                call: crate::api::ToolCall {
                    id: "c1".to_string(),
                    kind: "function".to_string(),
                    function: crate::api::FunctionCall {
                        name: "bash".to_string(),
                        arguments: r#"{"command":"cargo test"}"#.to_string(),
                    },
                },
                summary: "run cargo test".to_string(),
                responder,
            })
            .expect("the seat arrives");
        // The fan-out is a task, so the seat needs a moment to land.
        for _ in 0..50 {
            if !spy
                .shown
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let shown = spy
            .shown
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(shown.len(), 1, "the seat was presented: {shown:?}");
        assert_eq!(shown[0].0, "http://127.0.0.1:9/callback");
        assert_eq!(shown[0].1, "seat:bash:run cargo test");

        // The run closes with an assistant message in the log, and the
        // outcome is posted back.
        let path = crate::session::session_path(&home, &session);
        let mut log = crate::session::Session::open(&path).expect("the log opens");
        log.record(&crate::session::Record::Assistant {
            content: Some("all 1238 tests pass".to_string()),
            tool_calls: Vec::new(),
            persona: None,
        })
        .expect("the record lands");
        drop(log);
        let _ = sources.ui.send(crate::agent::UiEvent::Finished {
            stop_reason: "completed".to_string(),
        });
        for _ in 0..50 {
            if spy
                .shown
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .len()
                > 1
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let shown = spy
            .shown
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(shown.len(), 2, "the outcome was posted: {shown:?}");
        assert_eq!(shown[1].1, "all 1238 tests pass");
        drop(door);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_home_with_no_channel_directory_is_an_empty_store() {
        let home = temp_home("empty");
        let store = PeerStore::load(&home, "webhook").unwrap();
        assert!(store.peers.is_empty());
        let policy = Policy::new(home);
        assert_eq!(
            policy.authorize(&Peer::new("webhook", "anyone", "anyone")),
            Err(Refusal::Unknown)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_session_mapping_follows_a_named_session_and_mints_a_fresh_one() {
        let home = temp_home("sessions");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let peer = Peer::new("webhook", "abc", "the CI box");

        // A named session is followed when its shape is an id's.
        let asked = "20261018T101010Z-abcdef";
        assert_eq!(
            driver.session_for(&peer, Some(asked)).unwrap(),
            asked.to_string()
        );
        // A name that is not an id is refused for its shape, before
        // anything is created.
        assert!(driver.session_for(&peer, Some("../escape")).is_err());
        // No name: a fresh session, and the same one the second time.
        let first = driver.session_for(&peer, None).unwrap();
        let second = driver.session_for(&peer, None).unwrap();
        assert_eq!(first, second, "the peer's session is stable");
        assert_ne!(first, "session-test", "it is not the bridge's own");
        std::fs::remove_dir_all(&home).ok();
    }

    /// A peer that is not paired is remembered as pending, counted, and
    /// bounded — so a surface can let it in without the owner hunting a
    /// numeric id out of a log. A peer that is paired but may only listen
    /// is not pending: it is already in, and its capability is the thing
    /// to edit.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_refused_peer_is_remembered_as_pending_and_not_once_paired() {
        let home = temp_home("pending");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let stranger = Peer::new("telegram", "4242", "Alejandro");
        let inbound = Inbound {
            peer: stranger.clone(),
            session: None,
            task: "hello".to_string(),
            reply: Reply::None,
        };
        assert_eq!(driver.deliver(&inbound), Err(Refusal::Unknown));
        assert_eq!(driver.deliver(&inbound), Err(Refusal::Unknown));
        let pending = PendingStore::load(&home, "telegram");
        assert_eq!(pending.peers.len(), 1, "one peer, counted twice");
        assert_eq!(pending.peers[0].id, "4242");
        assert_eq!(pending.peers[0].name, "Alejandro");
        assert_eq!(pending.peers[0].attempts, 2);

        // Letting it in clears the pending entry and mints the pairing.
        let pairing = pair(
            &home,
            "telegram",
            "4242",
            "Alejandro",
            Capability::Observe,
            None,
        )
        .expect("paired");
        assert_eq!(pairing.id, "4242");
        assert_eq!(pairing.capability, Capability::Observe);
        assert!(pairing.token.is_none(), "the chat id is the credential");
        assert!(
            PendingStore::load(&home, "telegram").peers.is_empty(),
            "a paired peer is no longer pending"
        );

        // An observing peer is refused a task and is NOT recorded again:
        // the refusal names the capability, not the admission.
        assert_eq!(driver.deliver(&inbound), Err(Refusal::ObserveOnly));
        assert!(PendingStore::load(&home, "telegram").peers.is_empty());

        // The store is bounded, and the newest is what survives.
        let mut pending = PendingStore::default();
        for index in 0..(PENDING_CAP + 10) {
            pending.note(&format!("id{index}"), "a stranger", "stamp");
        }
        assert_eq!(pending.peers.len(), PENDING_CAP);
        assert_eq!(pending.peers[0].id, format!("id{}", PENDING_CAP + 9));
        pending.save(&home, "telegram").expect("the pending store");
        std::fs::remove_dir_all(&home).ok();
    }

    /// The pairing door is the only place a peer is created: a webhook
    /// peer's token is minted here, revealed once, and stored only as its
    /// digest — and the same call is what the app and the CLI use.
    #[test]
    fn the_pairing_door_reveals_the_token_once_and_stores_a_digest() {
        let home = temp_home("pairing");
        let pairing =
            pair(&home, "webhook", "", "the CI box", Capability::Task, None).expect("paired");
        let token = pairing.token.clone().expect("a webhook token");
        assert_eq!(token.len(), 43, "32 bytes, base64url");
        assert_eq!(
            pairing.id,
            crate::remote::pairing::hash_hex(token.as_bytes())[..12].to_string(),
            "the id is the digest's prefix, not a second random draw"
        );
        let store = PeerStore::load(&home, "webhook").expect("the store");
        assert_eq!(store.peers.len(), 1);
        assert_eq!(
            store.peers[0].token_hash,
            crate::remote::pairing::hash_hex(token.as_bytes()),
            "the store holds the digest"
        );
        // The token is nowhere on disk but its digest.
        for entry in std::fs::read_dir(home.join("channels")).expect("the channel dir") {
            let text = std::fs::read_to_string(entry.expect("an entry").path()).unwrap_or_default();
            assert!(
                !text.contains(&token),
                "the token is not written: the reveal is the only copy"
            );
        }
        // A channel this build does not ship is refused by name.
        assert!(pair(&home, "slack", "x", "y", Capability::Task, None).is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    /// `Observe` becomes real: a paired peer's capability is editable, and
    /// the policy answers with the new one — a demoted peer keeps its
    /// conversation and loses the door a task comes through.
    #[test]
    fn a_paired_peers_capability_is_editable_and_the_policy_follows() {
        let home = temp_home("capability");
        let pairing =
            pair(&home, "webhook", "", "the actor", Capability::Task, None).expect("paired");
        let policy = Policy::new(home.clone());
        let peer = Peer::new("webhook", &pairing.id, "the actor");
        assert_eq!(policy.authorize(&peer), Ok(Capability::Task));
        set_capability(&home, "webhook", &pairing.id, Capability::Observe).expect("demoted");
        assert_eq!(policy.authorize(&peer), Err(Refusal::ObserveOnly));
        assert_eq!(policy.capability(&peer), Some(Capability::Observe));
        // A peer that is not there is refused by name rather than
        // invented.
        assert!(set_capability(&home, "webhook", "nobody", Capability::Task).is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    /// The mapping survives a restart, and a stale one is dropped rather
    /// than believed: a driver built over the same home resumes the
    /// peer's conversation, and one whose log was deleted — or whose
    /// channel record names a different peer — starts fresh.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_peers_conversation_survives_a_restart_and_a_stale_one_is_dropped() {
        let home = temp_home("continuity");
        let peer = Peer::new("webhook", "abc", "the CI box");
        let first = {
            let (mut bridge, _sources) = Bridge::detached(
                home.clone(),
                "deepseek-flash".to_string(),
                Some("session-test".to_string()),
            );
            let _door = bridge.connect();
            let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
            driver.session_for(&peer, None).unwrap()
        };
        // The map is on disk, and it is private.
        let map = SessionMap::path(&home);
        assert!(map.exists(), "the mapping is written");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&map).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "the mapping is private: {mode:o}");
        }

        // A second driver over the same home reads the same conversation.
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let resumed = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        assert_eq!(
            resumed.session_for(&peer, None).unwrap(),
            first,
            "the peer's conversation survives the restart"
        );

        // A mapping whose log is gone is dropped: the peer gets a fresh
        // conversation rather than a run that lands in a file nothing
        // will ever read.
        std::fs::remove_file(crate::session::session_path(&home, &first)).ok();
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let orphaned = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        assert_ne!(
            orphaned.session_for(&peer, None).unwrap(),
            first,
            "a log that is gone is not a conversation"
        );

        // A mapping that points at another peer's log is dropped too: the
        // channel record names the peer's id, and a name is not unique.
        let other = Peer::new("webhook", "someone-else", "the CI box");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let swapped = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let mine = swapped.session_for(&other, None).unwrap();
        std::fs::write(
            SessionMap::path(&home),
            serde_json::to_string_pretty(&serde_json::json!({
                "peers": { "webhook:abc": mine }
            }))
            .unwrap(),
        )
        .unwrap();
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let cross = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        assert_ne!(
            cross.session_for(&peer, None).unwrap(),
            mine,
            "another peer's conversation is not handed over"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_unpaired_peers_delivery_is_refused_before_a_session_exists() {
        let home = temp_home("refused");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let inbound = Inbound {
            peer: Peer::new("webhook", "stranger", "a stranger"),
            session: None,
            task: "do the thing".to_string(),
            reply: Reply::None,
        };
        assert_eq!(driver.deliver(&inbound), Err(Refusal::Unknown));
        // No session was minted for a stranger.
        assert!(driver
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_paired_peers_delivery_runs_in_its_own_session() {
        let home = temp_home("deliver");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair("abc", "the CI box", &token, Capability::Task, "stamp");
        store.save(&home, "webhook").unwrap();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let peer = Peer::new("webhook", "abc", "the CI box");
        let inbound = Inbound {
            peer: peer.clone(),
            session: None,
            task: "run the tests".to_string(),
            reply: Reply::Url("http://127.0.0.1:9/callback".to_string()),
        };
        let session = driver.deliver(&inbound).expect("the delivery is accepted");
        // The run landed in the peer's own session, not the bridge's.
        assert_ne!(session, "session-test");
        let state = driver.bridge().state(&session);
        assert_eq!(state["running"], true, "the run is in flight in it");
        assert_eq!(state["events"][0]["type"], "message");
        assert_eq!(state["events"][0]["role"], "owner");
        assert_eq!(state["events"][0]["text"], "run the tests");
        // And the bridge's own session is untouched by the delivery.
        let launch = driver.bridge().state("session-test");
        assert!(
            launch["events"]
                .as_array()
                .is_none_or(|events| events.is_empty()),
            "the launch session kept its own log: {launch}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_observe_peer_may_not_hand_a_task() {
        let home = temp_home("observe");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair(
            "watcher",
            "the watcher",
            &token,
            Capability::Observe,
            "stamp",
        );
        store.save(&home, "webhook").unwrap();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-flash".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let inbound = Inbound {
            peer: Peer::new("webhook", "watcher", "the watcher"),
            session: None,
            task: "do the thing".to_string(),
            reply: Reply::None,
        };
        assert_eq!(driver.deliver(&inbound), Err(Refusal::ObserveOnly));
        std::fs::remove_dir_all(&home).ok();
    }
}
