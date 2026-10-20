//! The webhook channel: the reference plugin, and the one that needs no
//! account anywhere.
//!
//! A webhook sender POSTs a task to this channel's port with a peer
//! token in the header; the task runs in the peer's own session and the
//! outcome is POSTed back to the callback the delivery named. A seat is
//! POSTed to the same callback as a card, and the answer comes back as a
//! follow-up POST — so the whole round trip is exercisable with `curl`
//! and a test needs no platform, no account, and no network.
//!
//! ## Why this one is the reference
//!
//! Every other channel plugin is this shape with a platform's own
//! protocol in the middle: a long-poll loop instead of a listener, an
//! interactive card instead of a JSON body. If the driver, the policy,
//! and the seat path work here, they work everywhere — and a bug found
//! here is a bug found without a Telegram bot in the way.
//!
//! ## The listener's shape
//!
//! Its own port, away from the browser faces, gated by peer tokens and
//! nothing else: `POST /hook` takes a task, `POST /answer` answers a
//! seat. Both name their peer by token. An unpaired token is refused by
//! name, and a paired one that may only listen is refused for that.

use std::net::Ipv4Addr;
use std::sync::Arc;

use serde_json::json;

use crate::channels::{Channel, Driver, Inbound, Peer, Refusal, Reply, SeatCard};
/// The HTTP reply struct, aliased: this file has two `Reply` names in it
/// — the channel's outcome handle and the reply a socket is written —
/// and pretending one is the other is how a seat ends up POSTed to the
/// browser.
use crate::gui::server::Reply as HttpReply;
use crate::channels::PeerStore;

/// The port this channel listens on unless told otherwise. Away from
/// the browser faces' 3080 and the harness's 8300, so a running fleet
/// never has to guess which surface a port belongs to.
pub const DEFAULT_PORT: u16 = 3081;

/// The header a delivery's peer token rides.
const TOKEN_HEADER: &str = "x-fleet-peer";

/// The webhook channel: a listener and the driver its deliveries reach.
pub struct Webhook {
    port: u16,
    home: std::path::PathBuf,
    driver: Arc<Driver>,
}

impl Webhook {
    /// A webhook channel on `port`, over one home and one driver.
    pub fn new(port: u16, home: std::path::PathBuf, driver: Arc<Driver>) -> Webhook {
        Webhook { port, home, driver }
    }

    /// The port this channel listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The listener's own body, boxed by [`Channel::serve`] so the trait
    /// stays object-safe.
    async fn serve_inner(
        &self,
        sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), String> {
        let listener = match tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, self.port)).await
        {
            Ok(listener) => listener,
            Err(error) => {
                let reason = format!(
                    "the webhook channel cannot bind 127.0.0.1:{port}: {error}",
                    port = self.port
                );
                // The bind is the thing that could fail, so it is the
                // thing the supervisor is told about: a face that said
                // "live" over a socket nothing holds is the defect.
                let _ = ready.send(Err(reason.clone()));
                return Err(reason);
            }
        };
        // Bound: now it is a fact, and the port is the one actually held
        // — which is not `self.port` when that was zero.
        let held = listener
            .local_addr()
            .map(|address| address.port())
            .unwrap_or(self.port);
        let _ = ready.send(Ok(held));
        loop {
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((stream, _from)) => {
                        let home = self.home.clone();
                        let sink = Arc::clone(&sink);
                        let driver = Arc::clone(&self.driver);
                        tokio::spawn(async move {
                            if let Err(error) =
                                serve_connection(stream, &home, &driver, &*sink).await
                            {
                                eprintln!("fleet channels: {error}");
                            }
                        });
                    }
                    Err(error) => eprintln!("fleet channels: a delivery did not arrive: {error}"),
                },
                // The face that hosts the service owns Ctrl-C; a channel
                // that watched the signal itself would swallow it.
                _ = shutdown.changed() => return Ok(()),
            }
        }
    }
}

impl Channel for Webhook {
    fn name(&self) -> String {
        "webhook".to_string()
    }

    fn serve(
        &self,
        sink: Arc<dyn Fn(Inbound) -> Result<String, Refusal> + Send + Sync>,
        ready: tokio::sync::oneshot::Sender<Result<u16, String>>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(self.serve_inner(sink, ready, shutdown))
    }

    /// The outcome goes back as a POST to the callback the delivery
    /// named. A channel whose delivery named no callback has nowhere to
    /// send it, and saying nothing is the honest outcome.
    fn reply(&self, to: &Reply, text: &str) {
        let Reply::Url(url) = to else { return };
        let body = json!({"type": "reply", "text": text}).to_string();
        let _ = ureq::post(url)
            .set("content-type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_string(&body);
    }

    /// A seat is POSTed to the callback as a card naming the tool, the
    /// summary, and the session — the answer comes back through
    /// `/answer` and lands in the same session's seat, so a peer and the
    /// GUI inbox answer one seat between them.
    fn present_seat(&self, to: &Reply, seat: &SeatCard) {
        let Reply::Url(url) = to else { return };
        let body = json!({
            "type": "seat",
            "session": seat.session,
            "tool": seat.tool,
            "summary": seat.summary,
            "answer": "POST /answer with {\"session\":…,\"decision\":\"once\"|\"always\"|\"deny\"}",
        })
        .to_string();
        let _ = ureq::post(url)
            .set("content-type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_string(&body);
    }
}

/// One connection: the head, the peer token, the route, the answer.
async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    home: &std::path::Path,
    driver: &Arc<Driver>,
    sink: &(dyn Fn(Inbound) -> Result<String, crate::channels::Refusal> + Send + Sync),
) -> Result<(), String> {
    use crate::gui::server::{read_body, read_head, write_reply, Request};

    let (head, rest) = read_head(&mut stream).await?;
    let request = Request::parse(&head)?;
    let token = request
        .header(TOKEN_HEADER)
        .or_else(|| request.header("x-fleet-token"))
        .unwrap_or_default()
        .to_string();
    // The peer is resolved from the store before the body is read: an
    // unpaired token is refused without the delivery's contents being
    // looked at twice.
    let store = match PeerStore::load(home, "webhook") {
        Ok(store) => store,
        Err(error) => return write_reply(&mut stream, error_reply(&error.to_string())).await,
    };
    let Some(peer) = store.peer_for_token(&token) else {
        // "Who tried to reach me" is the question the activity log
        // exists to answer, and an unpaired token is the loudest answer
        // there is — the refusal the owner most needs to see. The token
        // itself is never logged: only that one arrived and was not
        // known.
        crate::channels::service::record(
            home,
            crate::channels::service::Activity {
                at: crate::boards::now_stamp(),
                kind: "refusal".to_string(),
                channel: "webhook".to_string(),
                peer: None,
                peer_id: None,
                session: None,
                task: None,
                detail: Some("an unpaired token asked for the hook".to_string()),
            },
        );
        return write_reply(
            &mut stream,
            error_reply("that peer token is not paired; `fleet channels pair-webhook` starts one"),
        )
        .await;
    };
    let peer_id = peer.id.clone();
    let peer_name = peer.name.clone();

    match (request.method.as_str(), request.path.as_str()) {
        ("POST", "/hook") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, error_reply(&message)).await,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(message) => return write_reply(&mut stream, error_reply(&message.to_string())).await,
            };
            let task = parsed
                .get("task")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if task.is_empty() {
                return write_reply(&mut stream, error_reply("a delivery names a task")).await;
            }
            let callback = parsed
                .get("callback")
                .and_then(serde_json::Value::as_str)
                .map(|url| url.trim().to_string())
                .filter(|url| !url.is_empty());
            let session = parsed
                .get("session")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let inbound = Inbound {
                peer: Peer::new("webhook", &peer_id, &peer_name),
                session,
                task,
                reply: match callback {
                    Some(url) => Reply::Url(url),
                    None => Reply::None,
                },
            };
            // The delivery goes to the sink, which is the host's decision
            // about what one means: the driver's answer, with the peer's
            // capability checked in the one place that decides it. A
            // refusal names itself.
            match sink(inbound) {
                Ok(session) => {
                    write_reply(
                        &mut stream,
                        api(200, json!({"type": "accepted", "session": session, "peer": peer_name})),
                    )
                    .await
                }
                Err(refusal) => write_reply(&mut stream, error_reply(&refusal.to_string())).await,
            }
        }
        ("POST", "/answer") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, error_reply(&message)).await,
            };
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(parsed) => parsed,
                Err(message) => return write_reply(&mut stream, error_reply(&message.to_string())).await,
            };
            let session = parsed
                .get("session")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let decision = parsed
                .get("decision")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            match driver.answer(session, decision, &crate::gui::bridge::AnsweredBy::Peer(peer_name.clone())) {
                Ok(()) => {
                    write_reply(
                        &mut stream,
                        api(200, json!({"type": "answered", "decision": decision})),
                    )
                    .await
                }
                Err(message) => {
                    write_reply(&mut stream, error_reply(&message.to_string())).await
                }
            }
        }
        _ => {
            write_reply(
                &mut stream,
                error_reply("this channel answers POST /hook and POST /answer"),
            )
            .await
        }
    }
}

/// A reply the channel's own caller reads.
fn api(status: u16, body: serde_json::Value) -> HttpReply {
    HttpReply {
        status: crate::gui::server::status_line(status),
        content_type: "application/json; charset=utf-8",
        extra: Vec::new(),
        body: format!("{body}\n"),
    }
}

/// A refusal, naming its reason: a sender that gets a bare 403 has
/// nothing to fix.
fn error_reply(message: &str) -> HttpReply {
    HttpReply {
        status: crate::gui::server::status_line(403),
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("{message}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::{Capability, Driver, Policy};
    use crate::gui::bridge::Bridge;
    use std::io::{Read, Write};

    fn temp_home(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-webhook-{tag}-{}-{}",
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

    /// A home with one paired peer, and the token it paired with.
    fn paired_home(tag: &str, capability: Capability) -> (std::path::PathBuf, String) {
        let home = temp_home(tag);
        let mut store = PeerStore::default();
        let token = crate::channels::mint_peer_token().unwrap();
        store.pair("abc", "the CI box", &token, capability, "stamp");
        store.save(&home, "webhook").unwrap();
        (home, token)
    }

    /// A live channel on an OS-picked port, with its driver behind it.
    /// The door comes back too: the test must hold it open for as long
    /// as it drives deliveries, because a dropped receiver is a face
    /// with no agent behind it.
    async fn live(
        home: std::path::PathBuf,
    ) -> (
        u16,
        Arc<Driver>,
        tokio::sync::mpsc::UnboundedReceiver<crate::gui::bridge::Command>,
    ) {
        let (mut bridge, _sources) = Bridge::detached(
            home.clone(),
            "deepseek-chat".to_string(),
            Some("session-test".to_string()),
        );
        let door = bridge.connect();
        let driver = Arc::new(Driver::new(Arc::new(bridge), Policy::new(home.clone())));
        let port = start(home, Arc::clone(&driver)).await;
        (port, driver, door)
    }

    /// Start the channel on port zero and learn the port it took. The
    /// channel binds inside its own runtime, so the test asks the
    /// operating system which free port to take and the channel reports
    /// back through the file the listener wrote.
    async fn start(home: std::path::PathBuf, driver: Arc<Driver>) -> u16 {
        // The channel binds a fixed port, so the test asks the operating
        // system for a free one and hands it over. Two tests racing for
        // the same freed port is real — they run in one process — so a
        // port that will not take is retried with a fresh one rather
        // than failing the run.
        for _ in 0..8 {
            let probe = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .await
                .expect("a free port");
            let port = probe.local_addr().expect("an address").port();
            drop(probe);
            let channel = Webhook::new(port, home.clone(), Arc::clone(&driver));
            let answering = Arc::clone(&driver);
            tokio::spawn(async move {
                let (ready, _arrived) = tokio::sync::oneshot::channel();
                let (_stop, stopped) = tokio::sync::watch::channel(false);
                let _ = channel
                    .serve(
                        Arc::new(move |inbound| answering.deliver(&inbound)),
                        ready,
                        stopped,
                    )
                    .await;
            });
            // The channel is ready when it answers something: a request
            // it refuses is still an answer, and it leaves no malformed
            // line in the log the way a bare connect does.
            let ready = "POST /answer HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\n\r\n{}";
            for _ in 0..50 {
                if try_ask(port, ready).is_some_and(|reply| reply.starts_with("HTTP/1.1")) {
                    return port;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        panic!("the channel would not start on any port");
    }

    /// One request that may not arrive yet: the channel binds on its own
    /// schedule, and a readiness probe must not panic at a port that is
    /// not listening yet.
    fn try_ask(port: u16, raw: &str) -> Option<String> {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .ok()?;
        stream.write_all(raw.as_bytes()).ok()?;
        let mut reply = String::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => reply.push_str(&String::from_utf8_lossy(&chunk[..read])),
                Err(_) => break,
            }
        }
        Some(reply)
    }

    fn ask(port: u16, raw: &str) -> String {
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("the channel is listening");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .expect("a read timeout is settable");
        stream.write_all(raw.as_bytes()).expect("the request is sent");
        let mut reply = String::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => reply.push_str(&String::from_utf8_lossy(&chunk[..read])),
                Err(_) => break,
            }
        }
        reply
    }

    fn hook(port: u16, token: &str, body: &str) -> String {
        ask(
            port,
            &format!(
                "POST /hook HTTP/1.1\r\nHost: 127.0.0.1\r\n{TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_paired_peers_delivery_is_accepted_and_runs_in_its_own_session() {
        let (home, token) = paired_home("accepted", Capability::Task);
        let (port, driver, _door) = live(home.clone()).await;

        let reply = hook(
            port,
            &token,
            &json!({"task": "run the tests", "callback": "http://127.0.0.1:9/callback"}).to_string(),
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let session = reply
            .split("\"session\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("the reply names the session")
            .to_string();
        // The run landed in the peer's session, with the peer's task in
        // its log and the bridge's own session untouched.
        let state = driver.bridge().state(&session);
        assert_eq!(state["running"], true);
        assert_eq!(state["events"][0]["text"], "run the tests");
        let launch = driver.bridge().state("session-test");
        assert!(
            launch["events"].as_array().is_none_or(|events| events.is_empty()),
            "the launch session is untouched: {launch}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_unpaired_token_is_refused_by_name() {
        let (home, _token) = paired_home("stranger", Capability::Task);
        let (port, _driver, _door) = live(home.clone()).await;

        let reply = hook(port, "not-a-real-token", &json!({"task": "do it"}).to_string());
        assert!(reply.starts_with("HTTP/1.1 403"), "{reply}");
        assert!(reply.contains("not paired"), "{reply}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_observe_peer_may_not_hand_a_task() {
        let (home, token) = paired_home("observe", Capability::Observe);
        let (port, driver, _door) = live(home.clone()).await;

        let reply = hook(port, &token, &json!({"task": "do it"}).to_string());
        assert!(reply.starts_with("HTTP/1.1 403"), "{reply}");
        assert!(reply.contains("only listen"), "{reply}");
        // And no session was minted for it.
        assert!(driver
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_delivery_without_a_task_is_refused_before_a_session_exists() {
        let (home, token) = paired_home("no-task", Capability::Task);
        let (port, driver, _door) = live(home.clone()).await;

        let reply = hook(port, &token, &json!({"callback": "http://127.0.0.1:9"}).to_string());
        assert!(reply.starts_with("HTTP/1.1 403"), "{reply}");
        assert!(reply.contains("names a task"), "{reply}");
        assert!(driver
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_revoked_peers_token_stops_working() {
        let (home, token) = paired_home("revoked", Capability::Task);
        let (port, _driver, _door) = live(home.clone()).await;
        assert!(hook(port, &token, &json!({"task": "one"}).to_string()).starts_with("HTTP/1.1 200"));

        let mut store = PeerStore::load(&home, "webhook").unwrap();
        assert!(store.revoke("abc"));
        store.save(&home, "webhook").unwrap();

        let after = hook(port, &token, &json!({"task": "two"}).to_string());
        assert!(after.starts_with("HTTP/1.1 403"), "{after}");
        std::fs::remove_dir_all(&home).ok();
    }
}
