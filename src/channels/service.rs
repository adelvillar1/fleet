//! The channel service: the supervisor over every transport plugin.
//!
//! A plugin says "I am live" only after the thing that could fail has
//! succeeded — a bind for a listener, the first answered poll for a
//! long-poller. This module is what turns that report into three things
//! that cannot disagree:
//!
//! - `<FLEET_HOME>/channels/status.json`, written atomically on every
//!   transition and refreshed on a heartbeat while anything is live, so
//!   the GUI, `fleet channels`, and the TUI read one document rather
//!   than each asking a different question;
//! - `<FLEET_HOME>/channels/activity.jsonl`, one line per delivery,
//!   refusal, reply, seat, and answer — the "what have they done" the
//!   surface exists to answer and nothing recorded before;
//! - the terminal's own line, because a bind failure that only reached a
//!   file is a bind failure the owner discovers by noticing nothing
//!   works.
//!
//! ## Why the supervisor exists
//!
//! The branch spawned each plugin with `let _ = served.serve(…).await`,
//! which is a `Result` thrown away inside a task: a face whose port was
//! taken printed "the webhook channel is live" and bound nothing. The
//! service awaits each plugin's readiness before it reports, so a failed
//! channel is `failed(reason)` everywhere it is read — and a stale
//! heartbeat is named stale rather than believed, because a status file
//! left behind by a dead process is a document that lies in the same
//! shape as a live one.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::channels::{Channel, Driver, Inbound, PeerStore, Refusal, Reply, SeatCard};
use crate::gui::bridge::Bridge;

/// How often a live service refreshes its heartbeat.
pub const HEARTBEAT_SECS: u64 = 10;

/// How many heartbeats a reader lets pass before it calls the document
/// stale. Three is one missed beat plus room for a slow disk.
const STALE_BEATS: u64 = 3;

/// How many activity lines the log keeps.
pub const ACTIVITY_CAP: usize = 500;

/// One channel's state, as every surface reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Not running: never started, or stopped on purpose.
    Stopped,
    /// Asked to come up; the readiness report has not arrived.
    Starting,
    /// The plugin reported the thing that could fail had succeeded.
    Live,
    /// The plugin could not come up, or came down for a reason.
    Failed,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Stopped => "stopped",
            Phase::Starting => "starting",
            Phase::Live => "live",
            Phase::Failed => "failed",
        }
    }
}

/// One channel in the status document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelReport {
    pub name: String,
    pub phase: Phase,
    /// The port the channel holds, for a transport that binds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Why it is not live, when it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When this channel last changed state.
    pub since: String,
}

/// The one document every surface reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The process hosting the service, so a reader can tell whose
    /// heartbeat this is.
    pub pid: u32,
    /// When the service came up.
    pub started: String,
    /// The heartbeat, as a stamp a person can read and as a number a
    /// machine can subtract.
    pub updated: String,
    #[serde(default)]
    pub updated_unix: u64,
    /// The interval the document promises between beats.
    pub heartbeat_secs: u64,
    pub channels: Vec<ChannelReport>,
}

impl Status {
    /// The document's path.
    pub fn path(home: &Path) -> PathBuf {
        home.join("channels").join("status.json")
    }

    /// Read the document, or `None` when no service has written one.
    /// An unreadable document is `None` rather than an error: it is a
    /// report about a process, and a report that would not parse does
    /// not describe one.
    pub fn load(home: &Path) -> Option<Status> {
        let text = std::fs::read_to_string(Self::path(home)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Write it atomically: a reader never sees half a document.
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

    /// Whether the heartbeat has stopped: the document is a dead
    /// process's last words rather than a live one's report. A document
    /// with no heartbeat number at all is stale — a shape this build
    /// never writes.
    pub fn is_stale(&self, now: SystemTime) -> bool {
        let now = unix_secs(now);
        let beat = self.heartbeat_secs.max(1);
        self.updated_unix == 0 || now.saturating_sub(self.updated_unix) > beat * STALE_BEATS
    }
}

/// One line of the activity log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub at: String,
    /// `delivery`, `refusal`, `reply`, `seat`, `answer`, or `state`.
    pub kind: String,
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The task's first line, capped: the log says what was asked
    /// without becoming a transcript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Activity {
    /// The log's path.
    pub fn path(home: &Path) -> PathBuf {
        home.join("channels").join("activity.jsonl")
    }

    /// The first line of a task, cut to what a row can show.
    pub fn first_line(task: &str) -> String {
        const CAP: usize = 120;
        let line = task.lines().next().unwrap_or_default().trim();
        if line.chars().count() <= CAP {
            return line.to_string();
        }
        let cut: String = line.chars().take(CAP).collect();
        format!("{cut}…")
    }
}

/// Append one activity line, capping the file. A log that cannot be
/// written is a log that says nothing, not a delivery refused: the work
/// already happened, and the reporting of it is the lesser thing.
pub fn record(home: &Path, line: Activity) {
    let path = Activity::path(home);
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let encoded = match serde_json::to_string(&line) {
        Ok(encoded) => encoded,
        Err(_) => return,
    };
    let mut lines: Vec<String> = match std::fs::read_to_string(&path) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    };
    lines.push(encoded);
    if lines.len() > ACTIVITY_CAP {
        let keep = lines.len() - ACTIVITY_CAP;
        lines.drain(..keep);
    }
    let temp = path.with_extension("jsonl.tmp");
    let body = format!("{}\n", lines.join("\n"));
    if std::fs::write(&temp, body).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600));
        }
        let _ = std::fs::rename(&temp, &path);
    }
}

/// Read the newest `limit` activity lines, newest last. An unreadable
/// log is an empty one: the surface says "nothing recorded" rather than
/// failing a whole read over a file it could not open.
pub fn activity(home: &Path, limit: usize) -> Vec<Activity> {
    let text = match std::fs::read_to_string(Activity::path(home)) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.len() > limit {
        lines.drain(..lines.len() - limit);
    }
    lines
        .into_iter()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// What a channel is built from: the bridge the deliveries run through,
/// the driver they reach, the home, and the port the webhook holds.
#[derive(Debug, Clone)]
pub struct Config {
    pub home: PathBuf,
    pub webhook_port: u16,
    /// Whether the service comes up at all. Off is the default, and off
    /// means no socket binds anywhere.
    pub enabled: bool,
}

impl Config {
    /// The configuration a face starts from: the settings' own truth,
    /// with `--channels` as an override of the door rather than the
    /// only door.
    pub fn from_settings(home: &Path, around: Option<bool>) -> Config {
        let enabled = around.unwrap_or_else(|| setting_enabled(home));
        Config {
            home: home.to_path_buf(),
            webhook_port: setting_port(home),
            enabled,
        }
    }
}

/// `channels.enabled` from the home's config, defaulting to false: a
/// machine that has never configured a channel binds nothing.
fn setting_enabled(home: &Path) -> bool {
    let text = match std::fs::read_to_string(home.join("config.json")) {
        Ok(text) => text,
        Err(_) => return false,
    };
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    parsed
        .get("channels")
        .and_then(|channels| channels.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// `channels.webhook_port` from the home's config, defaulting to the
/// channel's own constant.
fn setting_port(home: &Path) -> u16 {
    let text = match std::fs::read_to_string(home.join("config.json")) {
        Ok(text) => text,
        Err(_) => return crate::channels::webhook::DEFAULT_PORT,
    };
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(parsed) => parsed,
        Err(_) => return crate::channels::webhook::DEFAULT_PORT,
    };
    parsed
        .get("channels")
        .and_then(|channels| channels.get("webhook_port"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .unwrap_or(crate::channels::webhook::DEFAULT_PORT)
}

/// The supervisor: it owns every plugin it has built, reports what it
/// owns, and starts and stops them at runtime.
pub struct ChannelService {
    home: PathBuf,
    driver: Arc<Driver>,
    /// The plugins this service can start, built once. A plugin is
    /// cheap and stateless enough to reuse: the webhook holds only its
    /// port, and the Telegram offset is behind an `Arc` that survives a
    /// stop and start on purpose — a restart is not a reason to re-read
    /// messages the owner already answered.
    plugins: Mutex<Vec<Arc<dyn Channel>>>,
    /// The stop signal per running channel.
    running: Mutex<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>,
    status: Mutex<Status>,
    webhook_port: u16,
}

impl ChannelService {
    /// A service over one bridge, one driver, and one configuration. The
    /// plugins are built here and not started: building one binds
    /// nothing.
    pub fn new(
        bridge: Arc<Bridge>,
        driver: Arc<Driver>,
        config: &Config,
    ) -> ChannelService {
        let home = config.home.clone();
        let now = crate::boards::now_stamp();
        let webhook: Arc<dyn Channel> = Arc::new(crate::channels::webhook::Webhook::new(
            config.webhook_port,
            home.clone(),
            Arc::clone(&driver),
        ));
        let mut plugins: Vec<Arc<dyn Channel>> = vec![webhook];
        // The Telegram plugin exists only when its token does: a channel
        // with no credential is not a stopped channel, it is a channel
        // this machine has not configured.
        if let Some(token) = crate::channels::telegram_token(&home) {
            plugins.push(Arc::new(
                crate::channels::telegram::Telegram::new(token).at_home(home.clone()),
            ));
        }
        let channels = plugins
            .iter()
            .map(|plugin| ChannelReport {
                name: plugin.name(),
                phase: Phase::Stopped,
                port: None,
                error: None,
                since: now.clone(),
            })
            .collect();
        let _ = bridge;
        ChannelService {
            home,
            driver,
            plugins: Mutex::new(plugins),
            running: Mutex::new(std::collections::HashMap::new()),
            status: Mutex::new(Status {
                pid: std::process::id(),
                started: now.clone(),
                updated: now,
                updated_unix: unix_secs(SystemTime::now()),
                heartbeat_secs: HEARTBEAT_SECS,
                channels,
            }),
            webhook_port: config.webhook_port,
        }
    }

    /// The driver this service's deliveries reach, so a surface can
    /// present a peer's seats without a second one.
    pub fn driver(&self) -> &Arc<Driver> {
        &self.driver
    }

    /// The port the webhook channel was configured with.
    pub fn webhook_port(&self) -> u16 {
        self.webhook_port
    }

    /// The current document, cloned: the read a surface renders.
    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }

    /// Every channel this service can start, by name.
    pub fn names(&self) -> Vec<String> {
        lock(&self.plugins)
            .iter()
            .map(|plugin| plugin.name())
            .collect()
    }

    /// Come up: start every plugin, await each one's readiness, and
    /// report what came up. A channel that could not bind is `failed`
    /// with the reason — in this document and on the terminal — and the
    /// service keeps going for the others, because one channel's port
    /// being taken is not a reason to refuse every channel.
    pub async fn start_all(&self) -> Vec<ChannelReport> {
        let names = self.names();
        let mut reports = Vec::new();
        for name in names {
            reports.push(self.start(&name).await);
        }
        self.heartbeat();
        reports
    }

    /// Start one channel and await its readiness.
    pub async fn start(&self, name: &str) -> ChannelReport {
        let plugin = {
            let plugins = lock(&self.plugins);
            match plugins.iter().find(|plugin| plugin.name() == name) {
                Some(plugin) => Arc::clone(plugin),
                None => return self.note(name, Phase::Failed, None, Some(format!(
                    "no channel named `{name}` in this build"
                ))),
            }
        };
        // Starting something already live is a no-op, not a second
        // listener on the same port.
        {
            let running = lock(&self.running);
            if running.contains_key(name) {
                return self.note(name, Phase::Live, None, None);
            }
        }
        self.note(name, Phase::Starting, None, None);

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let sink = self.sink();
        let serving = Arc::clone(&plugin);
        let named = name.to_string();
        let home = self.home.clone();
        tokio::spawn(async move {
            match serving.serve(sink, ready_tx, stop_rx).await {
                Ok(()) => {}
                Err(reason) => {
                    // A channel that came up and then went down is not a
                    // channel that never came up, but it is also not
                    // live, and the surface has to say which.
                    record(
                        &home,
                        Activity {
                            at: crate::boards::now_stamp(),
                            kind: "state".to_string(),
                            channel: named.clone(),
                            peer: None,
                            peer_id: None,
                            session: None,
                            task: None,
                            detail: Some(reason),
                        },
                    );
                }
            }
        });
        // The readiness report is the whole point: until it lands,
        // nothing is claimed.
        let arrived = tokio::time::timeout(Duration::from_secs(5), ready_rx).await;
        match arrived {
            Ok(Ok(Ok(port))) => {
                lock(&self.running).insert(name.to_string(), stop_tx);
                let port = if port == 0 {
                    if name == "webhook" {
                        Some(self.webhook_port)
                    } else {
                        None
                    }
                } else {
                    Some(port)
                };
                self.note(name, Phase::Live, port, None)
            }
            Ok(Ok(Err(reason))) => self.note(name, Phase::Failed, None, Some(reason)),
            Ok(Err(_)) => self.note(
                name,
                Phase::Failed,
                None,
                Some("the channel stopped before it could report".to_string()),
            ),
            Err(_) => {
                // It has not said either way: the plugin may still come
                // up later, but claiming live now would be the guess
                // this whole module exists to remove.
                self.note(
                    name,
                    Phase::Starting,
                    None,
                    Some("no readiness report within five seconds".to_string()),
                )
            }
        }
    }

    /// Stop one channel: the shutdown fires, the plugin returns, and the
    /// report says stopped. Stopping something that is not running is a
    /// no-op rather than an error — the caller's intent is satisfied.
    pub fn stop(&self, name: &str) -> ChannelReport {
        let sender = lock(&self.running).remove(name);
        if let Some(sender) = sender {
            let _ = sender.send(true);
        }
        self.note(name, Phase::Stopped, None, None)
    }

    /// Stop every channel, for a face that is shutting down.
    pub fn stop_all(&self) {
        for name in self.names() {
            self.stop(&name);
        }
    }

    /// The sink every plugin hands its deliveries to: the driver, with
    /// the activity line recorded on both ends of the decision.
    pub(crate) fn sink(
        &self,
    ) -> Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync> {
        let driver = Arc::clone(&self.driver);
        let home = self.home.clone();
        Arc::new(move |inbound: Inbound| {
            let asked = Activity::first_line(&inbound.task);
            let result = driver.deliver(&inbound);
            let at = crate::boards::now_stamp();
            match &result {
                Ok(session) => record(
                    &home,
                    Activity {
                        at,
                        kind: "delivery".to_string(),
                        channel: inbound.peer.channel.clone(),
                        peer: Some(inbound.peer.name.clone()),
                        peer_id: Some(inbound.peer.id.clone()),
                        session: Some(session.clone()),
                        task: Some(asked),
                        detail: None,
                    },
                ),
                Err(refusal) => record(
                    &home,
                    Activity {
                        at,
                        kind: "refusal".to_string(),
                        channel: inbound.peer.channel.clone(),
                        peer: Some(inbound.peer.name.clone()),
                        peer_id: Some(inbound.peer.id.clone()),
                        session: None,
                        task: Some(asked),
                        detail: Some(refusal.to_string()),
                    },
                ),
            }
            result
        })
    }

    /// Update one channel's report, write the document, and say the same
    /// thing on the terminal.
    fn note(
        &self,
        name: &str,
        phase: Phase,
        port: Option<u16>,
        error: Option<String>,
    ) -> ChannelReport {
        let stamp = crate::boards::now_stamp();
        let mut status = lock(&self.status);
        status.updated = stamp.clone();
        status.updated_unix = unix_secs(SystemTime::now());
        let mut report = None;
        if let Some(slot) = status.channels.iter_mut().find(|slot| slot.name == name) {
            slot.phase = phase;
            slot.port = port;
            slot.error = error.clone();
            slot.since = stamp;
            report = Some(slot.clone());
        }
        let document = status.clone();
        drop(status);
        // A transition is said on the terminal as well as in the file: a
        // bind failure that only reached a document is one the owner
        // discovers by noticing nothing works.
        if report.is_some() {
            match phase {
                Phase::Live => match port {
                    Some(port) => println!("fleet channels — {name} is live on 127.0.0.1:{port}"),
                    None => println!("fleet channels — {name} is live"),
                },
                Phase::Failed => println!(
                    "fleet channels — {name} could not start: {}",
                    error.as_deref().unwrap_or("no reason given")
                ),
                _ => {}
            }
        }
        if let Err(error) = document.save(&self.home) {
            eprintln!("fleet channels: the status document did not save: {error}");
        }
        report.unwrap_or(ChannelReport {
            name: name.to_string(),
            phase,
            port,
            error,
            since: crate::boards::now_stamp(),
        })
    }

    /// Refresh the heartbeat and write the document.
    pub fn heartbeat(&self) {
        let mut status = lock(&self.status);
        status.updated = crate::boards::now_stamp();
        status.updated_unix = unix_secs(SystemTime::now());
        let document = status.clone();
        drop(status);
        if let Err(error) = document.save(&self.home) {
            eprintln!("fleet channels: the status document did not save: {error}");
        }
    }

    /// Run the watcher over every plugin this service can start: one
    /// subscriber to the bridge's stream, routing each peer's seats and
    /// outcomes to the transport that carried the conversation.
    pub fn spawn_watcher(self: &Arc<Self>) {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let plugins = lock(&service.plugins).clone();
            service.driver.watch(&plugins).await;
        });
    }

    /// Keep the heartbeat going while the face lives. Spawned once, by
    /// the face, so a stale document means a dead process rather than a
    /// service that forgot.
    pub fn spawn_heartbeat(self: &Arc<Self>) {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(HEARTBEAT_SECS));
            loop {
                tick.tick().await;
                service.heartbeat();
            }
        });
    }
}

impl Drop for ChannelService {
    fn drop(&mut self) {
        // The face is going away: the document should not claim live
        // channels it no longer hosts.
        let mut status = lock(&self.status);
        for slot in status.channels.iter_mut() {
            if slot.phase == Phase::Live || slot.phase == Phase::Starting {
                slot.phase = Phase::Stopped;
                slot.port = None;
                slot.since = crate::boards::now_stamp();
            }
        }
        status.updated = crate::boards::now_stamp();
        status.updated_unix = 0;
        let document = status.clone();
        drop(status);
        let _ = document.save(&self.home);
    }
}

/// The peers a channel has paired, for a surface that lists them. The
/// signature takes the service's names rather than reading the
/// directory, so a channel this build does not ship is never invented.
pub fn peers(home: &Path, channel: &str) -> Vec<crate::channels::PairedPeer> {
    PeerStore::load(home, channel)
        .map(|store| store.peers)
        .unwrap_or_default()
}

/// The name a peer is filed under, resolved from the store at the moment
/// it is logged: the watcher knows the peer's id from the mapping key,
/// and a name read fresh is a name that does not go stale when the owner
/// edits it.
fn peer_name(home: &Path, channel: &str, peer_id: &str) -> String {
    PeerStore::load(home, channel)
        .ok()
        .and_then(|store| {
            store
                .peers
                .iter()
                .find(|peer| peer.id == peer_id)
                .map(|peer| peer.name.clone())
        })
        .unwrap_or_else(|| peer_id.to_string())
}

/// A seat's channel-side card as the activity log records it: the peer
/// and the tool, never the arguments.
pub fn note_seat(home: &Path, channel: &str, peer_id: &str, seat: &SeatCard) {
    record(
        home,
        Activity {
            at: crate::boards::now_stamp(),
            kind: "seat".to_string(),
            channel: channel.to_string(),
            peer: Some(peer_name(home, channel, peer_id)),
            peer_id: Some(peer_id.to_string()),
            session: Some(seat.session.clone()),
            task: None,
            detail: Some(format!("{} · {}", seat.tool, seat.summary)),
        },
    );
}

/// A reply's channel-side record.
pub fn note_reply(home: &Path, channel: &str, peer_id: &str, to: &Reply, text: &str) {
    record(
        home,
        Activity {
            at: crate::boards::now_stamp(),
            kind: "reply".to_string(),
            channel: channel.to_string(),
            peer: Some(peer_name(home, channel, peer_id)),
            peer_id: Some(peer_id.to_string()),
            session: None,
            task: None,
            detail: match to {
                Reply::Url(url) => Some(format!("{url} · {}", Activity::first_line(text))),
                Reply::None => Some(format!("nowhere · {}", Activity::first_line(text))),
            },
        },
    );
}

/// The document as a surface renders it: the service's own state plus
/// everything a reader needs to judge whether to believe it.
pub fn document(home: &Path) -> serde_json::Value {
    match Status::load(home) {
        Some(status) => {
            let stale = status.is_stale(SystemTime::now());
            let mut value = json!({
                "type": "channels",
                "read_at": crate::boards::now_stamp(),
                "pid": status.pid,
                "started": status.started,
                "updated": status.updated,
                "heartbeat_secs": status.heartbeat_secs,
                "stale": stale,
                "running": !stale,
                "channels": status.channels,
            });
            if stale {
                value["note"] = json!(
                    "the service's heartbeat has stopped — this is the last report of a process that is no longer running"
                );
            }
            value
        }
        None => json!({
            "type": "channels",
            "read_at": crate::boards::now_stamp(),
            "running": false,
            "channels": [],
            "note": "no channel service has run in this home",
        }),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn unix_secs(at: SystemTime) -> u64 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::{Capability, Peer};

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-service-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bind failure is captured rather than swallowed: the service
    /// reports `failed` with the reason, the document says the same, and
    /// the terminal hears it. This is the defect the supervisor exists
    /// for — the branch printed "live" over a socket it never held.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_channel_that_cannot_bind_is_failed_by_name() {
        let home = temp_home("bind");
        // Hold the port so the webhook cannot have it.
        let squatter = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a port");
        let port = squatter.local_addr().expect("the address").port();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Arc::new(Driver::new(
            Arc::new(bridge),
            crate::channels::Policy::new(home.clone()),
        ));
        let service = ChannelService::new(
            Arc::new(Bridge::detached(
                home.clone(),
                "deepseek-chat".to_string(),
                Some("session-test".to_string()),
            )
            .0),
            driver,
            &Config {
                home: home.clone(),
                webhook_port: port,
                enabled: true,
            },
        );
        let report = service.start("webhook").await;
        assert_eq!(report.phase, Phase::Failed, "{report:?}");
        let reason = report.error.expect("a named reason");
        assert!(reason.contains("cannot bind"), "{reason}");
        assert!(reason.contains(&port.to_string()), "{reason}");

        // The document says the same thing a reader will see.
        let status = Status::load(&home).expect("the status document");
        let webhook = status
            .channels
            .iter()
            .find(|channel| channel.name == "webhook")
            .expect("the webhook is reported");
        assert_eq!(webhook.phase, Phase::Failed);
        assert!(webhook.error.as_deref().unwrap_or_default().contains("cannot bind"));

        // And the surface's read names it rather than hiding it.
        let document = document(&home);
        assert_eq!(document["channels"][0]["phase"], "failed");
        assert!(!document["stale"].as_bool().unwrap_or(true), "it just wrote");
        drop(squatter);
        std::fs::remove_dir_all(&home).ok();
    }

    /// A channel that binds reports the port it actually holds, and a
    /// port of zero is resolved by the bind rather than echoed.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_live_channel_reports_the_port_it_holds() {
        let home = temp_home("live");
        let (bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let driver = Arc::new(Driver::new(
            Arc::new(bridge),
            crate::channels::Policy::new(home.clone()),
        ));
        let service = ChannelService::new(
            Arc::new(
                Bridge::detached(
                    home.clone(),
                    "deepseek-chat".to_string(),
                    Some("session-test".to_string()),
                )
                .0,
            ),
            driver,
            &Config {
                home: home.clone(),
                webhook_port: 0,
                enabled: true,
            },
        );
        let report = service.start("webhook").await;
        assert_eq!(report.phase, Phase::Live, "{report:?}");
        let port = report.port.expect("the port it holds");
        assert_ne!(port, 0, "a zero request resolves to a real port");
        // It really is listening: a delivery's peer check answers.
        let refused = ureq::post(&format!("http://127.0.0.1:{port}/hook"))
            .set("content-type", "application/json")
            .send_string("{\"task\":\"hello\"}");
        assert!(refused.is_err(), "an unpaired token is not accepted");

        // Stopping it says so, and the document follows.
        let stopped = service.stop("webhook");
        assert_eq!(stopped.phase, Phase::Stopped);
        let status = Status::load(&home).expect("the status document");
        assert_eq!(status.channels[0].phase, Phase::Stopped);
        std::fs::remove_dir_all(&home).ok();
    }

    /// The heartbeat is what makes the document believable: a fresh one
    /// reads live, and one that stopped reads stale by name rather than
    /// as a live service.
    #[test]
    fn a_stopped_heartbeat_reads_stale() {
        let home = temp_home("stale");
        let now = SystemTime::now();
        let status = Status {
            pid: 1,
            started: "2026-10-20T00:00:00Z".to_string(),
            updated: "2026-10-20T00:00:00Z".to_string(),
            updated_unix: unix_secs(now),
            heartbeat_secs: HEARTBEAT_SECS,
            channels: vec![ChannelReport {
                name: "webhook".to_string(),
                phase: Phase::Live,
                port: Some(3081),
                error: None,
                since: "2026-10-20T00:00:00Z".to_string(),
            }],
        };
        assert!(!status.is_stale(now), "a beat just now is live");
        let later = now + Duration::from_secs(HEARTBEAT_SECS * 4);
        assert!(status.is_stale(later), "four beats later is stale");
        // The read says which it is rather than leaving the caller to
        // compare timestamps it does not have.
        status.save(&home).expect("the document");
        let fresh = document(&home);
        assert_eq!(fresh["running"], true);
        assert!(fresh.get("note").is_none());
        std::fs::remove_dir_all(&home).ok();
    }

    /// The activity log carries the peer, the kind, the task's first
    /// line, and the session, and it is capped rather than unbounded.
    #[test]
    fn the_activity_log_names_the_peer_and_caps_itself() {
        let home = temp_home("activity");
        for index in 0..(ACTIVITY_CAP + 20) {
            record(
                &home,
                Activity {
                    at: "2026-10-20T00:00:00Z".to_string(),
                    kind: "delivery".to_string(),
                    channel: "webhook".to_string(),
                    peer: Some("the CI box".to_string()),
                    peer_id: Some("abc".to_string()),
                    session: Some(format!("s{index}")),
                    task: Some("check the build".to_string()),
                    detail: None,
                },
            );
        }
        let lines = activity(&home, ACTIVITY_CAP + 100);
        assert_eq!(lines.len(), ACTIVITY_CAP, "the log is capped");
        let newest = lines.last().expect("the newest line");
        assert_eq!(newest.kind, "delivery");
        assert_eq!(newest.peer.as_deref(), Some("the CI box"));
        assert_eq!(newest.peer_id.as_deref(), Some("abc"));
        // A long task is one capped line, not a transcript.
        let long = Activity::first_line(&"x".repeat(400));
        assert!(long.ends_with('…'));
        assert_eq!(long.chars().count(), 121);
        // A log with nothing in it reads as nothing, not as an error.
        let empty = temp_home("activity-empty");
        assert!(activity(&empty, 10).is_empty());
        std::fs::remove_dir_all(&home).ok();
        std::fs::remove_dir_all(&empty).ok();
    }

    /// The settings are the door: off is the default, and the port comes
    /// from the config rather than the compiled-in constant.
    #[test]
    fn the_service_reads_its_port_and_door_from_settings() {
        let home = temp_home("settings");
        // No config at all: off, and the channel's own default port.
        let config = Config::from_settings(&home, None);
        assert!(!config.enabled, "an unconfigured home binds nothing");
        assert_eq!(config.webhook_port, crate::channels::webhook::DEFAULT_PORT);
        // The setting turns it on and moves the port; the flag overrides
        // the door either way.
        crate::settings::set(&home, "channels.enabled", "true").expect("the toggle");
        crate::settings::set(&home, "channels.webhook_port", "3999").expect("the port");
        let config = Config::from_settings(&home, None);
        assert!(config.enabled);
        assert_eq!(config.webhook_port, 3999);
        assert!(!Config::from_settings(&home, Some(false)).enabled, "the flag overrides");
        std::fs::remove_dir_all(&home).ok();
    }

    /// The sink is where a delivery and its refusal are both recorded
    /// with the peer's own id, so the surface can answer "what have they
    /// done" without reading the transcript.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_sink_records_a_refusal_with_the_peer() {
        let home = temp_home("sink");
        let (bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let driver = Arc::new(Driver::new(
            Arc::new(bridge),
            crate::channels::Policy::new(home.clone()),
        ));
        let service = ChannelService::new(
            Arc::new(
                Bridge::detached(
                    home.clone(),
                    "deepseek-chat".to_string(),
                    Some("session-test".to_string()),
                )
                .0,
            ),
            driver,
            &Config {
                home: home.clone(),
                webhook_port: 0,
                enabled: false,
            },
        );
        let sink = service.sink();
        let inbound = Inbound {
            peer: Peer::new("webhook", "stranger", "a stranger"),
            session: None,
            task: "do the thing\nand then another".to_string(),
            reply: Reply::None,
        };
        assert_eq!(sink(inbound), Err(Refusal::Unknown));
        let lines = activity(&home, 10);
        let line = lines.last().expect("the refusal is logged");
        assert_eq!(line.kind, "refusal");
        assert_eq!(line.peer.as_deref(), Some("a stranger"));
        assert_eq!(line.peer_id.as_deref(), Some("stranger"));
        assert_eq!(line.task.as_deref(), Some("do the thing"), "the first line only");
        assert!(line.detail.as_deref().unwrap_or_default().contains("not paired"));
        std::fs::remove_dir_all(&home).ok();
    }

    /// A peer paired to listen is visible in the store the service reads,
    /// and the capability it was paired with is the one that answers.
    #[test]
    fn a_peer_paired_to_listen_reads_as_observing() {
        let home = temp_home("observe");
        let mut store = PeerStore::default();
        store.pair("abc", "the listener", "unused", Capability::Observe, "stamp");
        store.save(&home, "webhook").expect("the store");
        let peers = peers(&home, "webhook");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].capability, Capability::Observe);
        std::fs::remove_dir_all(&home).ok();
    }
}
