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

pub mod webhook;

use crate::gui::bridge::Bridge;

/// Who is on the far end of a channel: the channel's name and the peer's
/// own id on it. A peer is identified by the pair, because the same
/// human on Telegram and on a webhook is two peers with one owner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Peer {
    pub channel: String,
    pub id: String,
}

impl Peer {
    pub fn new(channel: &str, id: &str) -> Peer {
        Peer {
            channel: channel.to_string(),
            id: id.to_string(),
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
                write!(out, "that peer may only listen; it cannot hand fleet a task")
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
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(PeerStore::default())
            }
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
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
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

    /// Pair a peer: the token is hashed here and never stored. The id is
    /// the digest's prefix, which is unique without a second random draw.
    pub fn pair(&mut self, id: &str, name: &str, token: &str, capability: Capability, stamp: &str) {
        let token_hash = crate::remote::pairing::hash_hex(token.as_bytes());
        self.peers.retain(|peer| peer.id != id);
        self.peers.push(PairedPeer {
            id: id.to_string(),
            name: name.to_string(),
            token_hash,
            capability,
            created_at: stamp.to_string(),
        });
    }

    /// The peer a presented token belongs to, comparing the digest of
    /// what arrived against every stored one in constant time.
    pub fn peer_for_token(&self, token: &str) -> Option<&PairedPeer> {
        let arrived = crate::remote::pairing::hash_hex(token.as_bytes());
        self.peers
            .iter()
            .find(|peer| crate::remote::pairing::digest_eq(peer.token_hash.as_bytes(), arrived.as_bytes()))
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

/// Where an outcome goes: the channel's own handle for the peer.
#[derive(Debug, Clone)]
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

    /// Serve inbound deliveries until the process stops, handing each to
    /// `sink`. The plugin owns its own connection, its own reconnect,
    /// and its own authentication — the driver never sees a credential.
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
    /// outcomes go, filed under the peer's key. In memory because the
    /// mapping is derived: a peer's session is found by scanning the
    /// archive for the channel record, and a peer with no history gets a
    /// fresh one.
    sessions: Mutex<HashMap<String, (String, Reply)>>,
    /// Where each peer's outcomes go, filed under the peer's key. The
    /// delivery named the callback; this remembers it so a seat raised
    /// three turns later still knows.
    replies: Mutex<HashMap<String, Reply>>,
}

impl Driver {
    /// A driver over one bridge and one policy.
    pub fn new(bridge: Arc<Bridge>, policy: Policy) -> Driver {
        Driver {
            bridge,
            policy,
            sessions: Mutex::new(HashMap::new()),
            replies: Mutex::new(HashMap::new()),
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
        let mut sessions = self.sessions.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some((session, _)) = sessions.get(&peer.key()) {
            return Ok(session.clone());
        }
        // A fresh log for this peer's first task: the same convention
        // every other surface creates one with.
        let home = self.bridge.home();
        let session = crate::session::Session::create(&home, &self.bridge.model())
            .map_err(|error| format!("could not create the session: {error}"))?;
        let id = session.id().to_string();
        let reply = self.replies.lock().unwrap_or_else(|poison| poison.into_inner())
            .get(&peer.key())
            .cloned()
            .unwrap_or(Reply::None);
        sessions.insert(peer.key(), (id.clone(), reply));
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
    pub async fn watch(&self, channel: Arc<dyn Channel>) {
        let mut events = self.bridge.subscribe();
        loop {
            match events.recv().await {
                Ok(event) => {
                    let Some(session) = event.get("session").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some((_, reply)) = self
                        .sessions
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .iter()
                        .find(|(_, (owned, _))| owned == session)
                        .map(|(_, pair)| pair.clone())
                    else {
                        continue;
                    };
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
                        }
                        Some("finished") | Some("failed") => {
                            // The outcome is the last thing the session
                            // said: the log is the record, and a reply
                            // that paraphrases it is a second answer to
                            // what the run produced.
                            if let Some(text) = self.last_words(session) {
                                channel.reply(&reply, &text);
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
        let home = self.bridge.home();
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
        self.policy.authorize(&inbound.peer)?;
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
    pub fn answer(&self, session: &str, decision: &str) -> Result<(), String> {
        self.bridge
            .answer(session, decision)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
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
    fn a_paired_peers_token_is_stored_as_a_digest_only() {
        let home = temp_home("store");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair("abc", "the CI box", &token, Capability::Task, "2026-10-18T00:00:00Z");
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
        store.pair("watcher", "the watcher", &watcher, Capability::Observe, "stamp");
        store.save(&home, "webhook").unwrap();

        let policy = Policy::new(home.clone());
        let actor_peer = Peer::new("webhook", "actor");
        let watcher_peer = Peer::new("webhook", "watcher");
        let stranger = Peer::new("webhook", "stranger");
        let other_channel = Peer::new("telegram", "actor");

        assert_eq!(policy.authorize(&actor_peer), Ok(Capability::Task));
        assert_eq!(
            policy.authorize(&watcher_peer),
            Err(Refusal::ObserveOnly)
        );
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
        fn name(&self) -> String {
            "spy".to_string()
        }

        fn serve(
            &self,
            _sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        {
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
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let door = bridge.connect();
        let driver = Arc::new(Driver::new(Arc::new(bridge), Policy::new(home.clone())));
        let spy = Spy::new();

        // A delivery that names a callback: the seat and the outcome both
        // have somewhere to go.
        let inbound = Inbound {
            peer: Peer::new("webhook", "abc"),
            session: None,
            task: "run the tests".to_string(),
            reply: Reply::Url("http://127.0.0.1:9/callback".to_string()),
        };
        let session = driver.deliver(&inbound).expect("accepted");

        // The watcher, subscribed to the same stream the browser watches.
        let watching = Arc::clone(&driver);
        let channel: Arc<dyn Channel> = Arc::clone(&spy) as Arc<dyn Channel>;
        tokio::spawn(async move { watching.watch(channel).await });
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
            policy.authorize(&Peer::new("webhook", "anyone")),
            Err(Refusal::Unknown)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_session_mapping_follows_a_named_session_and_mints_a_fresh_one() {
        let home = temp_home("sessions");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let peer = Peer::new("webhook", "abc");

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

    #[tokio::test(flavor = "multi_thread")]
    async fn an_unpaired_peers_delivery_is_refused_before_a_session_exists() {
        let home = temp_home("refused");
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let inbound = Inbound {
            peer: Peer::new("webhook", "stranger"),
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
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let peer = Peer::new("webhook", "abc");
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
            launch["events"].as_array().is_none_or(|events| events.is_empty()),
            "the launch session kept its own log: {launch}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_observe_peer_may_not_hand_a_task() {
        let home = temp_home("observe");
        let mut store = PeerStore::default();
        let token = mint_peer_token().unwrap();
        store.pair("watcher", "the watcher", &token, Capability::Observe, "stamp");
        store.save(&home, "webhook").unwrap();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let _door = bridge.connect();
        let driver = Driver::new(Arc::new(bridge), Policy::new(home.clone()));
        let inbound = Inbound {
            peer: Peer::new("webhook", "watcher"),
            session: None,
            task: "do the thing".to_string(),
            reply: Reply::None,
        };
        assert_eq!(driver.deliver(&inbound), Err(Refusal::ObserveOnly));
        std::fs::remove_dir_all(&home).ok();
    }
}
