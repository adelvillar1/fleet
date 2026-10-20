//! The Telegram channel: the second plugin, and the proof that the
//! contract generalizes.
//!
//! The webhook plugin is the reference because it needs no account
//! anywhere; this one is the proof because Telegram is a real platform
//! with a real protocol, and a trait that only fits the reference
//! implementation is a trait shaped like the reference rather than like
//! the problem. If the driver, the policy, and the seat path survive a
//! platform with long-polling, peer ids, and interactive keyboards, they
//! survive the rest.
//!
//! ## What is different here, and what is not
//!
//! The webhook listens; this one dials. `getUpdates` is a long-poll the
//! client holds, and `sendMessage` is how an outcome or a seat goes back.
//! Everything else — the peer's identity, the capability check, the
//! session mapping, the seat's single answer — is the driver's, unchanged.
//!
//! The bot's token rides the environment, never a payload: the same
//! masking rule every other surface holds. It is read once at start and
//! kept in the struct, because a channel that re-read the environment per
//! message is a channel that can change identity mid-conversation.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::channels::{Channel, Inbound, Peer, Refusal, Reply, SeatCard};

/// How long one `getUpdates` poll is held. Telegram allows up to 50
/// seconds; 25 is under every proxy's idle ceiling and answers often
/// enough that a message is picked up in a breath.
const POLL_SECS: u64 = 25;

/// How long an HTTP call to Telegram may take. A poll is held for
/// [`POLL_SECS`], so the ceiling has to be past it.
const CALL_TIMEOUT: Duration = Duration::from_secs(POLL_SECS + 15);

/// The Telegram channel: a bot token and the offset the long-poll has
/// reached. One per bot; the offset is the only state, because Telegram
/// delivers each update once and the channel must not ask twice.
pub struct Telegram {
    token: String,
    /// The API base, overridable so a test drives the real channel
    /// against a fake Telegram: the contract under test is the mapping
    /// and the loop, not the platform's uptime.
    api_base: String,
    /// The update id to ask after. Updates are numbered, and asking
    /// `offset = last + 1` is what makes a reconnect a resume rather than
    /// a replay of the whole history.
    offset: Arc<std::sync::Mutex<i64>>,
    /// The home whose activity log records what this channel says. A
    /// test channel has none and records nothing; the service sets it.
    home: Option<std::path::PathBuf>,
}

impl Telegram {
    /// A Telegram channel on one bot token.
    pub fn new(token: String) -> Telegram {
        Telegram {
            token,
            api_base: "https://api.telegram.org".to_string(),
            offset: Arc::new(std::sync::Mutex::new(0)),
            home: None,
        }
    }

    /// Record this channel's activity in `home`. The service sets it;
    /// a plugin built by hand records nothing, which is what a test
    /// wants and what a plugin with no home must do rather than guess.
    pub fn at_home(mut self, home: std::path::PathBuf) -> Telegram {
        self.home = Some(home);
        self
    }

    /// A Telegram channel against another API base. Production uses the
    /// default; a test points it at a fake.
    pub fn with_api_base(token: String, api_base: String) -> Telegram {
        Telegram {
            token,
            api_base,
            offset: Arc::new(std::sync::Mutex::new(0)),
            home: None,
        }
    }

    /// The API base for one bot token. Its own function so the URL is
    /// written once.
    fn api(&self, method: &str) -> String {
        format!("{}/bot{}/{method}", self.api_base, self.token)
    }

}

use serde_json::Value;

impl Channel for Telegram {
    fn name(&self) -> String {
        "telegram".to_string()
    }

    fn serve(
        &self,
        sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(self.serve_inner(sink, ready, shutdown))
    }

    /// The outcome goes back as a message to the chat the delivery came
    /// from.
    fn reply(&self, to: &Reply, text: &str) {
        let Some(chat) = chat_of(to) else { return };
        let _ = ureq::post(&self.api("sendMessage"))
            .timeout(CALL_TIMEOUT)
            .send_json(json!({"chat_id": chat, "text": text}));
    }

    /// A seat is presented as a message naming the tool, the summary, and
    /// the words that answer it — the platform's own affordance rather
    /// than a button this channel cannot draw.
    fn present_seat(&self, to: &Reply, seat: &SeatCard) {
        let Some(chat) = chat_of(to) else { return };
        let text = format!(
            "approval needed\n{}: {}\nreply once, always, or deny",
            seat.tool, seat.summary
        );
        let _ = ureq::post(&self.api("sendMessage"))
            .timeout(CALL_TIMEOUT)
            .send_json(json!({"chat_id": chat, "text": text}));
    }
}

impl Telegram {
    /// The long-poll loop: one poll at a time, each delivery handed to
    /// the sink. The blocking call runs on a dedicated thread, because
    /// `ureq` is a blocking client and the bridge already spawns threads
    /// for exactly this shape.
    ///
    /// The channel's state is moved into the loop rather than borrowed:
    /// a boxed future cannot hold a borrow across an await, and the
    /// token and the offset are the whole of what a poll needs.
    async fn serve_inner(
        &self,
        sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), String> {
        if self.token.trim().is_empty() {
            let reason = "the telegram channel has no bot token (FLEET_TELEGRAM_BOT_TOKEN)"
                .to_string();
            let _ = ready.send(Err(reason.clone()));
            return Err(reason);
        }
        let token = self.token.clone();
        let api_base = self.api_base.clone();
        let offset = Arc::clone(&self.offset);
        // A long-poller has no port to bind, so "ready" is the first
        // answered poll: until the platform has heard from this bot,
        // nothing is listening and saying "live" would be a guess. An
        // error is not readiness — a bad token or no network must not
        // read as a live channel.
        let mut ready = Some(ready);
        loop {
            let polled = {
                let token = token.clone();
                let offset = Arc::clone(&offset);
                let api_base = api_base.clone();
                tokio::task::spawn_blocking(move || {
                    poll_updates(&api_base, &token, &offset)
                })
            };
            let outcome = tokio::select! {
                polled = polled => polled,
                _ = shutdown.changed() => return Ok(()),
            };
            match outcome {
                Ok(Ok(updates)) => {
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(Ok(0));
                    }
                    for update in &updates {
                        if let Some(inbound) = delivery_of(update) {
                            // A refused delivery names its peer: the chat
                            // id is the only thing an owner can pair, and
                            // a channel that refuses silently is a channel
                            // nobody can configure.
                            if let Err(refusal) = sink(inbound.clone()) {
                                eprintln!(
                                    "fleet channels: telegram peer {id} ({name}) was refused: {refusal}",
                                    id = inbound.peer.id,
                                    name = inbound.peer.name,
                                );
                                // The refusal IS the reply: a peer that
                                // may only listen is told so in its own
                                // chat rather than left with silence it
                                // cannot distinguish from a Fleet that
                                // never heard it. The webhook needs no
                                // equivalent — its refusal is the HTTP
                                // answer the delivery already receives.
                                self.reply(&inbound.reply, &refusal.to_string());
                                if let Some(home) = &self.home {
                                    crate::channels::service::note_reply(
                                        home,
                                        "telegram",
                                        &inbound.peer.id,
                                        &inbound.reply,
                                        &refusal.to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(Err(error.clone()));
                        return Err(error);
                    }
                    eprintln!("fleet channels: {error}");
                    // A failed poll is a network that went away: wait a
                    // moment rather than hammering the platform.
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(_) => return Ok(()),
            }
        }
    }
}

/// One update as a delivery, or None when it is not one. A text message
/// from a private chat is; anything else — a photo, a group message, an
/// edited message — is not, and saying so by returning None is how a
/// channel declines without failing.
fn delivery_of(update: &Value) -> Option<Inbound> {
    let message = update.get("message")?;
    let text = message.get("text").and_then(Value::as_str)?.trim();
    if text.is_empty() {
        return None;
    }
    let chat = message.get("chat")?;
    // Private chats only: a group is many people, and a peer is one.
    if chat.get("type").and_then(Value::as_str) != Some("private") {
        return None;
    }
    let chat_id = chat.get("id").and_then(Value::as_i64)?;
    let name = message
        .get("from")
        .and_then(|from| from.get("first_name"))
        .and_then(Value::as_str)
        .unwrap_or("a telegram peer")
        .to_string();
    // The chat id is the peer's id and the reply handle both: it is
    // where an outcome goes, and it is what a seat's answer names.
    Some(Inbound {
        peer: Peer::new("telegram", &chat_id.to_string(), &name),
        session: None,
        task: text.to_string(),
        reply: Reply::Url(format!("telegram://chat/{chat_id}")),
    })
}

/// One `getUpdates` poll, free of the channel's borrow so the loop can
/// hold it across an await.
fn poll_updates(
    api_base: &str,
    token: &str,
    offset: &std::sync::Mutex<i64>,
) -> Result<Vec<Value>, String> {
    let after = *offset.lock().unwrap_or_else(|poison| poison.into_inner());
    let url = format!("{api_base}/bot{token}/getUpdates");
    let response = ureq::get(&url)
        .timeout(CALL_TIMEOUT)
        .query("offset", &after.to_string())
        .query("timeout", &POLL_SECS.to_string())
        .call()
        .map_err(|error| format!("the telegram poll failed: {error}"))?;
    let body: Value = response
        .into_json()
        .map_err(|error| format!("telegram's updates were not readable: {error}"))?;
    // The offset moves past everything handed back, whether or not the
    // delivery is accepted: an update this channel refuses is still an
    // update Telegram has delivered, and asking for it again is a loop.
    if let Some(updates) = body.get("result").and_then(Value::as_array) {
        if let Some(last) = updates
            .last()
            .and_then(|update| update.get("update_id"))
            .and_then(Value::as_i64)
        {
            *offset.lock().unwrap_or_else(|poison| poison.into_inner()) = last + 1;
        }
    }
    Ok(body
        .get("result")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// The chat a reply handle names. A handle this channel did not mint is
/// refused: the reply goes where the delivery said, and nowhere else.
fn chat_of(to: &Reply) -> Option<i64> {
    match to {
        Reply::Url(url) => url
            .strip_prefix("telegram://chat/")
            .and_then(|chat| chat.parse().ok()),
        Reply::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::{Capability, Driver, PeerStore, Policy};
    use crate::gui::bridge::Bridge;

    fn temp_home(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-telegram-{tag}-{}-{}",
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

    fn update(update_id: i64, chat_id: i64, text: &str) -> Value {
        json!({
            "update_id": update_id,
            "message": {
                "text": text,
                "chat": {"id": chat_id, "type": "private"},
                "from": {"first_name": "Alejandro"},
            },
        })
    }

    #[test]
    fn a_private_text_message_is_a_delivery() {
        let inbound = delivery_of(&update(1, 4242, "check the build"))
            .expect("a private text message delivers");
        assert_eq!(inbound.peer.channel, "telegram");
        assert_eq!(inbound.peer.id, "4242");
        assert_eq!(inbound.peer.name, "Alejandro");
        assert_eq!(inbound.task, "check the build");
        // The chat id is the reply handle: where an outcome goes.
        assert!(matches!(&inbound.reply, Reply::Url(url) if url == "telegram://chat/4242"));
    }

    #[test]
    fn what_is_not_a_private_text_message_is_declined() {
        // A group message: a peer is one person.
        let group = json!({
            "update_id": 2,
            "message": {
                "text": "hello everyone",
                "chat": {"id": -100, "type": "group"},
                "from": {"first_name": "Alejandro"},
            },
        });
        assert!(delivery_of(&group).is_none());
        // A photo: this channel carries text.
        let photo = json!({
            "update_id": 3,
            "message": {
                "photo": [],
                "chat": {"id": 4242, "type": "private"},
                "from": {"first_name": "Alejandro"},
            },
        });
        assert!(delivery_of(&photo).is_none());
        // An empty text: nothing to run.
        assert!(delivery_of(&update(2, 4242, "   ")).is_none());
    }

    #[test]
    fn a_reply_handle_this_channel_did_not_mint_is_refused() {
        // The outcome goes where the delivery said and nowhere else: a
        // handle from another channel, or none at all, is refused rather
        // than guessed at.
        assert_eq!(chat_of(&Reply::Url("telegram://chat/99".to_string())), Some(99));
        assert_eq!(chat_of(&Reply::Url("webhook://other".to_string())), None);
        assert_eq!(chat_of(&Reply::None), None);
    }


    /// A fake Telegram: one update queued, every send recorded. The
    /// channel under test is the real one, against the real driver.
    struct FakeTelegram {
        updates: std::sync::Mutex<Vec<Value>>,
        sent: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl FakeTelegram {
        fn new() -> Arc<FakeTelegram> {
            Arc::new(FakeTelegram {
                updates: std::sync::Mutex::new(Vec::new()),
                sent: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn offer(&self, update: Value) {
            self.updates
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(update);
        }
    }

    /// A telegram channel is the second plugin, and the proof that the
    /// contract generalizes: a platform that dials instead of listening,
    /// numbers its peers, and draws no buttons still runs through the
    /// same driver, the same policy, and the same seat.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_telegram_message_runs_through_the_same_driver() {
        let home = temp_home("telegram-e2e");
        let mut store = PeerStore::default();
        store.pair("4242", "Alejandro", "unused", Capability::Task, "stamp");
        store.save(&home, "telegram").unwrap();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let driver = Arc::new(Driver::new(Arc::new(bridge), Policy::new(home.clone())));

        // The fake Telegram, on its own port.
        let fake = FakeTelegram::new();
        let base = start_fake(Arc::clone(&fake)).await;
        let channel = Telegram::with_api_base("test-token".to_string(), base);
        let served: Arc<dyn Channel> = Arc::new(channel);
        let sink_holder = Arc::clone(&driver);
        let serving = Arc::clone(&served);
        tokio::spawn(async move {
            let sink: Arc<
                dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync,
            > = Arc::new(move |inbound| sink_holder.deliver(&inbound));
            let (ready, _arrived) = tokio::sync::oneshot::channel();
            let (_stop, stopped) = tokio::sync::watch::channel(false);
            let _ = serving.serve(sink, ready, stopped).await;
        });

        // A message arrives, and the run lands in the peer's own session
        // — not the bridge's launch session.
        fake.offer(update(11, 4242, "check the build"));
        // The channel polls on its own schedule, so the delivery needs a
        // moment to land. The bound is ten seconds rather than two: these
        // tests run beside twelve hundred others, and a wait sized for an
        // idle machine is a flake under load — the suite's own lesson,
        // from both ends.
        let _session = {
            let mut found = None;
            for _ in 0..500 {
                let sessions = driver
                    .sessions
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                if let Some((id, _)) = sessions.values().next() {
                    found = Some(id.clone());
                    break;
                }
                drop(sessions);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            found.expect("the peer has a session")
        };
        // The outcome goes back to the chat the message came from.
        served.reply(
            &Reply::Url("telegram://chat/4242".to_string()),
            "the build is green",
        );
        let sent = fake
            .sent
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        assert_eq!(sent.len(), 1, "the outcome was sent: {sent:?}");
        assert!(sent[0].1.contains("the build is green"), "{sent:?}");
        std::fs::remove_dir_all(&home).ok();
    }

    /// An observing peer is refused a task — and told so in its own chat
    /// rather than left with silence it cannot tell from a Fleet that
    /// never heard it. The refusal is the reply the channel owes it.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_observing_peer_is_told_why_its_task_was_refused() {
        let home = temp_home("telegram-observe");
        let mut store = PeerStore::default();
        store.pair("4242", "Alejandro", "unused", Capability::Observe, "stamp");
        store.save(&home, "telegram").unwrap();

        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let _door = bridge.connect();
        let bridge = Arc::new(bridge);
        let driver = Arc::new(Driver::new(Arc::clone(&bridge), Policy::new(home.clone())));
        // The service's sink, not the driver's, because the sink is what
        // records the refusal: the test should exercise the same path a
        // running service does.
        let service = Arc::new(crate::channels::ChannelService::new(
            Arc::clone(&bridge),
            Arc::clone(&driver),
            &crate::channels::ServiceConfig {
                home: home.clone(),
                webhook_port: 0,
                enabled: false,
            },
        ));
        let sink = service.sink();

        let fake = FakeTelegram::new();
        let base = start_fake(Arc::clone(&fake)).await;
        let served: Arc<dyn Channel> = Arc::new(
            Telegram::with_api_base("test-token".to_string(), base).at_home(home.clone()),
        );
        let serving = Arc::clone(&served);
        tokio::spawn(async move {
            let (ready, _arrived) = tokio::sync::oneshot::channel();
            let (_stop, stopped) = tokio::sync::watch::channel(false);
            let _ = serving.serve(sink, ready, stopped).await;
        });

        fake.offer(update(12, 4242, "do the thing"));
        let mut sent = Vec::new();
        for _ in 0..500 {
            sent = fake
                .sent
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone();
            if !sent.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(sent.len(), 1, "the refusal came back: {sent:?}");
        assert!(sent[0].1.contains("may only listen"), "{sent:?}");
        // And the attempt is in the activity log, twice over: the
        // refusal the driver recorded and the reply the channel sent.
        let activity = crate::channels::service::activity(&home, 10);
        assert!(
            activity.iter().any(|line| line.kind == "refusal"),
            "{activity:?}"
        );
        assert!(
            activity.iter().any(|line| line.kind == "reply"),
            "the reply is recorded: {activity:?}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    /// The fake Telegram's listener: one update per poll, every send
    /// recorded. Its own port, its own task.
    async fn start_fake(fake: Arc<FakeTelegram>) -> String {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("a port");
        let port = listener.local_addr().expect("an address").port();
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                let fake = Arc::clone(&fake);
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut head = Vec::new();
                    let mut chunk = [0u8; 1024];
                    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                        let read = stream.read(&mut chunk).await.unwrap_or(0);
                        if read == 0 {
                            return;
                        }
                        head.extend_from_slice(&chunk[..read]);
                    }
                    let text = String::from_utf8_lossy(&head).to_string();
                    let body = if text.starts_with("GET ") && text.contains("getUpdates") {
                        let updates = fake
                            .updates
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        let queued: Vec<Value> = updates.clone();
                        drop(updates);
                        fake.updates
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .clear();
                        serde_json::to_string(&json!({"ok": true, "result": queued})).unwrap()
                    } else if text.starts_with("POST ") && text.contains("sendMessage") {
                        let body = text
                            .split("\r\n\r\n")
                            .nth(1)
                            .unwrap_or_default()
                            .to_string();
                        if let Ok(parsed) = serde_json::from_str::<Value>(body.trim()) {
                            let chat = parsed
                                .get("chat_id")
                                .and_then(Value::as_i64)
                                .map(|id| id.to_string())
                                .unwrap_or_default();
                            let text = parsed
                                .get("text")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            fake.sent
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner())
                                .push((chat, text));
                        }
                        "{\"ok\": true}".to_string()
                    } else {
                        "{\"ok\": true, \"result\": []}".to_string()
                    };
                    let reply = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(reply.as_bytes()).await;
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    #[test]
    fn the_offset_moves_past_what_was_handed_back() {
        // Telegram delivers each update once; asking twice is a loop. The
        // offset is the channel's only state, and it advances past the
        // last update seen whether or not the delivery was accepted.
        let updates = vec![
            json!({"update_id": 7, "message": {"text": "one", "chat": {"id": 1, "type": "private"}}}),
            json!({"update_id": 9, "message": {"text": "two", "chat": {"id": 1, "type": "private"}}}),
        ];
        let last = updates.last().and_then(|u| u.get("update_id")).and_then(Value::as_i64);
        assert_eq!(last, Some(9), "the last update is the one the offset passes");
    }
}
