//! The remote listener: the face a paired device reaches.
//!
//! This is a second HTTP surface beside the loopback one, and the
//! difference between them is the whole design:
//!
//! - **It binds an address another machine can reach.** The loopback
//!   face refuses a non-loopback `--host` by name; this one exists to
//!   serve one. The owner opts in with `fleet gui --remote`, and the
//!   address is detected or named.
//! - **Every request carries a device token**, compared against the
//!   SHA-256s in `~/.fleet/devices.json` — not the loopback face's
//!   per-launch token, which never leaves the terminal it was printed
//!   to. A request with no valid token gets the pairing page or a 401,
//!   and nothing else.
//! - **The routes are default-deny.** A paired phone may read state,
//!   watch the stream, drive a run, and answer a seat. Every other route
//!   of the loopback face — settings writes, fact edits, skill
//!   approvals, the file and tree pane — answers 403 *by name*, so a
//!   route added to the loopback face later stays closed here until it
//!   is deliberately added here too.
//!
//! The page, its stylesheet, and its script are the loopback face's own,
//! embedded in the binary and served here unchanged: the phone sees the
//! same app, and the pairing flow hands it the same cookie the loopback
//! face would have set.

use std::net::IpAddr;
use std::sync::Arc;

use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use crate::gui::bridge::Bridge;
use crate::gui::server::{api, decode_query, read_body, read_head, write_reply, Reply, Request};
use crate::remote::pairing::{DeviceStore, PairingWindow};
use crate::remote::RemoteAddress;

/// The cookie a paired device's browser keeps. Its own name, so a phone
/// that has also visited the loopback face on the same machine carries
/// two capabilities rather than one that might be the wrong one.
const DEVICE_COOKIE: &str = "fleet_device";

/// The heartbeat this face's stream beats at. The page counts the beats
/// the same way it does on loopback, so a silent stream is a dead face.
const HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(5);

/// What a paired device may ask for. **Default deny**: a route the
/// loopback face has that this list does not is a route a phone cannot
/// reach, and adding one here is a decision rather than an oversight.
/// The stream and the state ride the same document the page already
/// reduces, so a phone and the desktop see the same conversation.
const ALLOWED_GET: &[&str] = &[
    "/events",
    "/api/state",
    "/api/sessions",
    "/api/session",
    "/api/checkpoints",
    "/api/diff",
];
const ALLOWED_POST: &[&str] = &["/api/run", "/api/plan", "/api/approve", "/api/resume"];

/// The remote face: the listener, the address it was given, and the
/// bridge to the agent thread its actions speak to. Deliberately not
/// cloneable — one launch, one address, one listener — and the
/// per-connection tasks carry what they need by value.
pub struct Listener {
    address: IpAddr,
    port: u16,
    home: std::path::PathBuf,
    listener: TcpListener,
    bridge: Arc<Bridge>,
    /// The device session this face paired with, and the secret that
    /// proves it: what a relay is joined with, so a relay can only ever
    /// be joined by a device that paired.
    session: String,
    secret: String,
}

impl Listener {
    /// Bind the remote listener. `port` zero asks the operating system
    /// for a free one, which is what a second launch and the tests need.
    pub async fn bind(
        remote: &RemoteAddress,
        port: u16,
        home: std::path::PathBuf,
        bridge: Arc<Bridge>,
    ) -> Result<(Listener, u16), String> {
        let listener = TcpListener::bind((remote.address, port))
            .await
            .map_err(|error| format!("fleet remote cannot bind {}: {error}", remote.address))?;
        let bound = listener
            .local_addr()
            .map_err(|error| format!("fleet remote cannot read its own address: {error}"))?;
        let server = Listener {
            address: remote.address,
            port: bound.port(),
            home,
            listener,
            bridge,
            session: mint_session()?,
            secret: mint_secret()?,
        };
        Ok((server, bound.port()))
    }

    /// The address this face serves, as the pairing block names it.
    pub fn address(&self) -> IpAddr {
        self.address
    }

    /// The port this face serves.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The device session a relay is joined with.
    pub fn device_session(&self) -> String {
        self.session.clone()
    }

    /// The secret that proves it.
    pub fn device_secret(&self) -> String {
        self.secret.clone()
    }

    /// The base URL a phone opens, without any credential in it.
    pub fn base_url(&self) -> String {
        match self.address {
            IpAddr::V6(address) => format!("http://[{address}]:{}", self.port),
            IpAddr::V4(address) => format!("http://{address}:{}", self.port),
        }
    }

    /// Serve until the owner stops the process. Every connection is a
    /// task of its own, so one slow stream never holds the listener, and
    /// Ctrl-C ends it cleanly.
    pub async fn serve(&self) -> Result<(), String> {
        loop {
            tokio::select! {
                accepted = self.listener.accept() => match accepted {
                    Ok((stream, _from)) => {
                        let home = self.home.clone();
                        let bridge = Arc::clone(&self.bridge);
                        tokio::spawn(async move {
                            if let Err(error) =
                                serve_connection(stream, &home, &bridge).await
                            {
                                eprintln!("fleet remote: {error}");
                            }
                        });
                    }
                    // A connection that failed to arrive is not a reason
                    // to stop serving the ones that do.
                    Err(error) => eprintln!("fleet remote: a connection did not arrive: {error}"),
                },
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }
}

/// One connection, start to finish: read the head, decide whether it is
/// the pairing flow or a gated request, answer, and drop the socket.
async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    home: &std::path::Path,
    bridge: &Arc<Bridge>,
) -> Result<(), String> {
    let (head, rest) = read_head(&mut stream).await?;
    let request = Request::parse(&head)?;

    // The pairing flow is the one unauthenticated pair of routes, and it
    // is unauthenticated by design: a phone that has not paired yet has
    // no token to present. The page leaks nothing — it is a form — and
    // the redemption is gated by the window's own attempt cap.
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") | ("GET", "/pair") => {
            return write_reply(&mut stream, pairing_page()).await;
        }
        ("POST", "/api/pair") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            return write_reply(&mut stream, redeem(&body, home)).await;
        }
        _ => {}
    }

    // Everything else needs a device token. A browser with no cookie
    // asking for a page is a phone that has not paired: it gets the
    // pairing page rather than a 401 it cannot act on. An API route with
    // no token gets the 401.
    let token = device_token(&request).unwrap_or_default();
    let device = match DeviceStore::load(home) {
        Ok(store) => store.device_for_token(token).cloned(),
        Err(error) => {
            eprintln!("fleet remote: the device store could not be read: {error}");
            None
        }
    };
    let Some(device) = device else {
        if request.method == "GET" && is_page(&request.path) {
            return write_reply(&mut stream, pairing_page()).await;
        }
        return write_reply(&mut stream, unauthorized()).await;
    };

    // Paired, and the route is now checked against the allowlist. A 403
    // names the route and the rule, because "forbidden" with no reason
    // is a surface that lies about what it checked.
    if !allowed(&request) {
        return write_reply(&mut stream, forbidden(&request.method, &request.path)).await;
    }

    let answer = route_with(&request, rest, &mut stream, bridge, &device.name).await;
    // The device was heard from: a display stamp, written off the
    // request's path so a slow disk never holds the answer.
    let home = home.to_path_buf();
    let id = device.id.clone();
    tokio::task::spawn_blocking(move || DeviceStore::touch(&home, &id));
    write_reply(&mut stream, answer).await
}

/// The relay's handler: this face's own router, answering the frames the
/// relay forwards. It exists so the relay is a transport rather than a
/// second implementation of what a request means.
pub struct RelayHandler {
    bridge: Arc<Bridge>,
    device_name: String,
}

impl RelayHandler {
    /// A handler answering as the named paired device.
    pub fn new(bridge: Arc<Bridge>, device_name: String) -> RelayHandler {
        RelayHandler { bridge, device_name }
    }
}

impl crate::remote::relay::Handler for RelayHandler {
    fn answer(
        &self,
        frame: &crate::remote::relay::Frame,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::remote::relay::Answer> + Send + '_>>
    {
        let frame = frame.clone();
        Box::pin(async move {
            let reply = answer_frame(&frame, &self.bridge, &self.device_name).await;
            crate::remote::relay::Answer {
                id: frame.id,
                status: status_of(&reply),
                headers: vec![(
                    "content-type".to_string(),
                    reply.content_type.to_string(),
                )],
                body: reply.body,
            }
        })
    }
}

/// The numeric status a reply's line carries.
fn status_of(reply: &Reply) -> u16 {
    reply
        .status
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(500)
}

/// Answers one frame the relay forwarded, through the same routing the
/// socket path uses — there is one implementation of what a request
/// means, and the relay is not a second one.
///
/// The stream is the one route a frame cannot carry: a frame is a whole
/// reply and a stream is a sequence of them. So `/events` over the relay
/// answers with the session's current tail and a `relay` marker, and the
/// page polls instead of subscribing — which is what a phone does well
/// and a held-open connection through a relay does badly.
async fn answer_frame(
    frame: &crate::remote::relay::Frame,
    bridge: &Arc<Bridge>,
    device_name: &str,
) -> Reply {
    // Re-parse the frame's head the way a socket's head is parsed: one
    // parser, one shape.
    let head = format!(
        "{} {} HTTP/1.1\r\n{}\r\n",
        frame.method,
        frame.path,
        frame
            .headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join("\r\n")
    );
    let Ok(request) = Request::parse(head.trim_end()) else {
        return bad_request("the relay forwarded a request this face could not read");
    };
    if request.path == "/events" {
        return relay_stream(&request, bridge);
    }
    // The body rides the frame, so the routes that read one read the
    // frame's body rather than the socket's.
    let mut cursor = std::io::Cursor::new(frame.body.clone().into_bytes());
    let rest = cursor.get_ref().to_vec();
    route_with(&request, rest, &mut cursor, bridge, device_name).await
}

/// The stream's stand-in behind a relay: the session's current tail, with
/// the marker that tells the page to poll.
fn relay_stream(request: &Request, bridge: &Arc<Bridge>) -> Reply {
    let session = asked_session(bridge, request);
    let state = bridge.state(&session);
    api(
        200,
        json!({
            "type": "relay-stream",
            "session": session,
            "relay": true,
            "state": state,
            "note": "this face is reached through a relay; the page polls instead of subscribing",
        }),
    )
}

/// The session a request addresses: the one its query names, or the
/// launch session when it names none. The same rule the loopback face
/// reads, because a phone that names nothing should get the same
/// conversation the desktop's own face opens.
fn asked_session(bridge: &Bridge, request: &Request) -> String {
    let asked = crate::gui::server::decode_query(&request.query, "session");
    let asked = asked.trim();
    if asked.is_empty() {
        bridge.launch()
    } else {
        asked.to_string()
    }
}

/// The device token a request presents, from the three places a browser
/// can carry it: the header, the query, or this face's own cookie. Its
/// own extraction rather than the loopback face's `presented_token`,
/// because the cookie names differ on purpose — a phone that has also
/// visited the loopback face carries two capabilities, and each face
/// must read only its own.
fn device_token(request: &Request) -> Option<&str> {
    if let Some(value) = request.header("x-fleet-token") {
        return Some(value);
    }
    if let Some(value) = crate::gui::server::query_value(&request.query, "token") {
        return Some(value);
    }
    let cookie = request.header("cookie")?;
    cookie
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == DEVICE_COOKIE)
        .map(|(_, value)| value)
}

/// Whether a GET path is the page or one of its assets, which is what a
/// browser asks for when it has not paired yet.
fn is_page(path: &str) -> bool {
    matches!(path, "/" | "/app.css" | "/app.js")
}

/// The allowlist check, method-aware. A path may be readable and not
/// writable, which is the difference between `/api/state` and
/// `/api/settings` here.
fn allowed(request: &Request) -> bool {
    match request.method.as_str() {
        "GET" => ALLOWED_GET.contains(&request.path.as_str()),
        "POST" => ALLOWED_POST.contains(&request.path.as_str()),
        _ => false,
    }
}

/// One allowlisted request, answered against the same bridge the
/// loopback face answers against — so a phone and the desktop are two
/// clients of one conversation, not two conversations.
async fn route_with(
    request: &Request,
    rest: Vec<u8>,
    stream: &mut (impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin),
    bridge: &Arc<Bridge>,
    // The name the paired device gave itself, which is who an approval
    // from this connection is answered by.
    device_name: &str,
) -> Reply {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => page(),
        ("GET", "/app.css") => asset("text/css; charset=utf-8", crate::gui::server::STYLES),
        ("GET", "/app.js") => {
            asset("text/javascript; charset=utf-8", crate::gui::server::SCRIPT)
        }
        ("GET", "/events") => {
            // The stream is written straight to the socket rather than
            // assembled as a reply, so it answers its own head and then
            // beats until the browser is gone. A relay's frame never
            // reaches here: the relay path answers `/events` with the
            // tail before the router sees it, because a frame is a whole
            // reply and a stream is not.
            let _ = stream_events(stream, bridge.subscribe()).await;
            Reply {
                status: "200 OK",
                content_type: "text/event-stream",
                extra: Vec::new(),
                body: String::new(),
            }
        }
        ("GET", "/api/state") => {
            let session = asked_session(bridge, &request);
            api(200, bridge.state(&session))
        }
        ("GET", "/api/sessions") => {
            let session = asked_session(bridge, &request);
            api(200, bridge.sessions(&session).await)
        }
        ("GET", "/api/session") => {
            let asked = decode_query(&request.query, "id");
            api(200, bridge.replay(&asked).await)
        }
        ("GET", "/api/checkpoints") => {
            let session = asked_session(bridge, &request);
            api(200, bridge.checkpoints(&session).await)
        }
        ("GET", "/api/diff") => {
            let asked = decode_query(&request.query, "turn");
            let turn: u64 = asked.trim().parse().unwrap_or(0);
            let session = asked_session(bridge, &request);
            api(200, bridge.diff(&session, turn).await)
        }
        ("POST", "/api/run") => {
            let body = match read_body(stream, request, rest).await {
                Ok(body) => body,
                Err(message) => return bad_request(&message),
            };
            let session = asked_session(bridge, &request);
            match task_of(&body) {
                Ok(task) => match bridge.run(&session, &task) {
                    Ok(()) => api(
                        200,
                        json!({"type": "run", "task": task, "accepted": true, "from": "device"}),
                    ),
                    Err(message) => api(409, json!({"error": message})),
                },
                Err(message) => bad_request(&message),
            }
        }
        ("POST", "/api/plan") => {
            let body = match read_body(stream, request, rest).await {
                Ok(body) => body,
                Err(message) => return bad_request(&message),
            };
            let session = asked_session(bridge, &request);
            match task_of(&body) {
                Ok(task) => match bridge.plan(&session, &task) {
                    Ok(()) => api(200, json!({"type": "plan", "task": task})),
                    Err(message) => api(409, json!({"error": message})),
                },
                Err(message) => bad_request(&message),
            }
        }
        ("POST", "/api/approve") => {
            let body = match read_body(stream, request, rest).await {
                Ok(body) => body,
                Err(message) => return bad_request(&message),
            };
            let session = asked_session(bridge, &request);
            match decision_of(&body) {
                Ok(decision) => {
                    match bridge.answer(
                        &session,
                        &decision,
                        &crate::gui::bridge::AnsweredBy::Device(device_name.to_string()),
                    ) {
                    Ok(answered) => api(200, answered),
                    Err(crate::gui::bridge::AnswerError::NoSeat) => {
                        api(409, json!({"error": "no approval is waiting"}))
                    }
                    Err(crate::gui::bridge::AnswerError::Unknown(word)) => {
                        api(400, json!({"error": word}))
                    }
                    }
                }
                Err(message) => bad_request(&message),
            }
        }
        ("POST", "/api/resume") => {
            let body = match read_body(stream, request, rest).await {
                Ok(body) => body,
                Err(message) => return bad_request(&message),
            };
            match session_of(&body) {
                Ok(id) => match bridge.resume(&id).await {
                    Ok(resumed) => api(200, resumed),
                    Err(message) => api(409, json!({"error": message})),
                },
                Err(message) => bad_request(&message),
            }
        }
        _ => not_found(&request.path),
    }
}

/// The stream: the head, a hello that tells the page where it is, then
/// a beat on an interval and every event the bridge broadcasts. The
/// loopback face's stream, one surface over: a phone and the desktop
/// watch the same run because they subscribe to the same channel.
async fn stream_events(
    stream: &mut (impl tokio::io::AsyncWrite + Unpin),
    mut events: tokio::sync::broadcast::Receiver<serde_json::Value>,
) -> Result<(), String> {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|error| format!("fleet remote could not open the stream: {error}"))?;
    stream
        .flush()
        .await
        .map_err(|error| format!("fleet remote could not open the stream: {error}"))?;
    write_event(stream, &json!({"type": "hello", "remote": true})).await?;
    let mut ticks = tokio::time::interval(HEARTBEAT);
    ticks.tick().await;
    loop {
        tokio::select! {
            received = events.recv() => match received {
                Ok(payload) => {
                    if write_event(stream, &payload).await.is_err() {
                        return Ok(());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    let missed = json!({"type": "lagged", "skipped": skipped});
                    if write_event(stream, &missed).await.is_err() {
                        return Ok(());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            _ = ticks.tick() => {
                let beat = json!({"type": "heartbeat"});
                if write_event(stream, &beat).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

async fn write_event(
    stream: &mut (impl tokio::io::AsyncWrite + Unpin),
    payload: &serde_json::Value,
) -> Result<(), String> {
    let event = format!("data: {payload}\n\n");
    stream
        .write_all(event.as_bytes())
        .await
        .map_err(|error| format!("fleet remote could not write a beat: {error}"))
}

/// Redeem a pairing code. The window is read from the home — the one
/// `fleet pair` or `fleet gui --remote` wrote — and the code is spent
/// against it. A success writes the device record and hands the browser
/// the token as an HttpOnly cookie, so the requests that follow carry
/// it exactly as the loopback face's do.
fn redeem(body: &str, home: &std::path::Path) -> Reply {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(parsed) => parsed,
        Err(message) => return bad_request(&message.to_string()),
    };
    let code = parsed
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let name = parsed
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("a paired device")
        .trim();
    let name = if name.is_empty() {
        "a paired device"
    } else {
        name
    };
    let Some(mut window) = PairingWindow::load_live(home) else {
        return api(
            409,
            json!({"error": "no pairing code is waiting; `fleet pair` starts one"}),
        );
    };
    let token = match window.redeem(home, code) {
        Ok(token) => token,
        Err(crate::remote::pairing::PairingError::WrongCode { attempts_left }) => {
            return api(
                403,
                json!({
                    "error": "that code did not match",
                    "attempts_left": attempts_left,
                }),
            );
        }
        Err(crate::remote::pairing::PairingError::Expired) => {
            return api(
                409,
                json!({"error": "that pairing code expired; `fleet pair` starts a new one"}),
            );
        }
        Err(crate::remote::pairing::PairingError::AlreadyUsed) => {
            return api(
                409,
                json!({"error": "that pairing code was already used; `fleet pair` starts a new one"}),
            );
        }
    };
    let stamp = crate::session::iso_utc_stamp(std::time::SystemTime::now())
        .unwrap_or_else(|_| String::from("unknown"));
    let mut store = match DeviceStore::load(home) {
        Ok(store) => store,
        Err(error) => return api(500, json!({"error": error.to_string()})),
    };
    let device = store.add(name, &token, &stamp);
    let id = device.id.clone();
    if let Err(error) = store.save(home) {
        return api(500, json!({"error": error.to_string()}));
    }
    // The token rides the cookie from here on. HttpOnly keeps the page's
    // own scripts from reading the capability back, and SameSite keeps
    // another site's requests from sending it.
    Reply {
        status: "200 OK",
        content_type: "application/json; charset=utf-8",
        extra: vec![(
            "Set-Cookie".to_string(),
            format!("{DEVICE_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict"),
        )],
        body: format!("{}\n", json!({"type": "paired", "device": id, "name": name})),
    }
}

/// The pairing page: a form, inline, because a phone that has not paired
/// has no cookie to fetch an asset with. It posts the code to
/// `/api/pair` and reloads on success.
fn pairing_page() -> Reply {
    Reply {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        extra: Vec::new(),
        body: PAIR_PAGE.to_string(),
    }
}

/// The page and its assets, the loopback face's own, served unchanged.
fn page() -> Reply {
    Reply {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        extra: Vec::new(),
        body: crate::gui::server::INDEX.to_string(),
    }
}

fn asset(content_type: &'static str, body: &'static str) -> Reply {
    Reply {
        status: "200 OK",
        content_type,
        extra: Vec::new(),
        body: body.to_string(),
    }
}

/// A relay session id: 16 random bytes, hex, which is where a guess
/// stops being a threat model.
fn mint_session() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    std::io::Read::read_exact(&mut std::fs::File::open("/dev/urandom").map_err(|error| {
        format!("fleet remote cannot read /dev/urandom: {error}")
    })?, &mut bytes)
    .map_err(|error| format!("fleet remote cannot read /dev/urandom: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// A relay secret: 32 random bytes, base64url, the same shape and entropy
/// as a device token.
fn mint_secret() -> Result<String, String> {
    crate::remote::pairing::mint_token().map_err(|error| error.to_string())
}

/// The gate's refusal, for an API route with no valid device token. It
/// names the rule and never the token.
fn unauthorized() -> Reply {
    Reply {
        status: "401 Unauthorized",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: "fleet's remote face serves paired devices only — pair this browser with the code `fleet pair` printed.\n"
            .to_string(),
    }
}

/// The allowlist's refusal. It names the route, because a paired device
/// that asked for something specific deserves to know that specific
/// thing is closed rather than that something unnamed is.
fn forbidden(method: &str, path: &str) -> Reply {
    Reply {
        status: "403 Forbidden",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!(
            "fleet's remote face answers {method} {path} only on this machine — a paired device may drive a session, and that is all it may do.\n"
        ),
    }
}

fn bad_request(message: &str) -> Reply {
    Reply {
        status: "400 Bad Request",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("{message}\n"),
    }
}

fn not_found(path: &str) -> Reply {
    Reply {
        status: "404 Not Found",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("fleet's remote face has no route for {path}.\n"),
    }
}

/// The task a run's body carries.
fn task_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("fleet remote could not read the run's body: {error}"))?;
    parsed
        .get("task")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a run's body is a JSON object with a task string".to_string())
}

/// The session id a resume's body carries.
fn session_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("fleet remote could not read the resume's body: {error}"))?;
    parsed
        .get("session")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a resume's body is a JSON object with a session string".to_string())
}

/// The decision an approval's body carries, in the seat's four words.
fn decision_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("fleet remote could not read the answer's body: {error}"))?;
    parsed
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "an answer's body is a JSON object with a decision string".to_string())
}

/// The pairing page's markup. Inline styles because the cookie that
/// would fetch the stylesheet is the thing being earned here; the form
/// posts the code and the token comes back as a cookie, after which the
/// app is the app.
const PAIR_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>fleet — pair this device</title>
<style>
  :root { color-scheme: dark; }
  body {
    margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: #14161a; color: #e8eaed;
    font: 16px/1.5 -apple-system, "SF Pro Text", "Segoe UI", system-ui, sans-serif;
    padding: 24px; box-sizing: border-box;
  }
  form {
    width: min(420px, 100%); background: #1b1e24; border: 1px solid #2a2e37;
    border-radius: 14px; padding: 28px 24px; box-sizing: border-box;
  }
  h1 { font-size: 19px; margin: 0 0 6px; }
  p { color: #9aa0a8; font-size: 14px; margin: 0 0 20px; }
  label { display: block; font-size: 13px; color: #9aa0a8; margin: 0 0 6px; }
  input {
    width: 100%; box-sizing: border-box; padding: 12px 14px; font-size: 17px;
    letter-spacing: 0.08em; text-transform: uppercase;
    background: #14161a; color: #e8eaed;
    border: 1px solid #2a2e37; border-radius: 9px;
  }
  input:focus { outline: 2px solid #4f8cc9; outline-offset: 1px; }
  button {
    margin-top: 18px; width: 100%; padding: 13px; font-size: 16px; font-weight: 600;
    color: #fff; background: #3d7ab8; border: 0; border-radius: 9px; cursor: pointer;
  }
  button:active { background: #35689c; }
  #note { margin-top: 14px; font-size: 14px; min-height: 20px; }
  #note[data-state="error"] { color: #e5534b; }
  #note[data-state="ok"] { color: #57ab5a; }
  code {
    background: #14161a; border: 1px solid #2a2e37; border-radius: 5px;
    padding: 1px 6px; font-size: 13px;
  }
</style>
</head>
<body>
<form id="pair" method="post" action="/api/pair">
  <h1>Pair this device</h1>
  <p>Type the pairing code from the terminal running <code>fleet gui --remote</code> (or <code>fleet pair</code>).</p>
  <label for="code">Pairing code</label>
  <input id="code" name="code" type="text" autocomplete="one-time-code" inputmode="text"
         autocapitalize="characters" spellcheck="false" placeholder="XXXX-XXXX-XXXX" required>
  <label for="name" style="margin-top:16px">What should this device be called?</label>
  <input id="name" name="name" type="text" placeholder="Alejandro's iPhone" style="letter-spacing:normal;text-transform:none">
  <button type="submit">Pair</button>
  <div id="note" role="status"></div>
</form>
<script>
  // The form posts JSON rather than navigating, so a wrong code answers
  // in place with its attempts left instead of a dead page.
  var form = document.getElementById('pair');
  var note = document.getElementById('note');
  // A scan arrives with the code in the query — the QR the terminal
  // printed carries it — so the form opens already filled and one tap
  // pairs. A phone that typed the URL by hand has an empty form.
  var scanned = new URLSearchParams(window.location.search).get('c');
  if (scanned) form.code.value = scanned.toUpperCase();
  form.addEventListener('submit', function (event) {
    event.preventDefault();
    note.removeAttribute('data-state');
    note.textContent = 'pairing…';
    fetch('/api/pair', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ code: form.code.value, name: form.name.value })
    }).then(function (reply) {
      return reply.json().then(function (body) { return { ok: reply.ok, body: body }; });
    }).then(function (result) {
      if (result.ok) {
        note.setAttribute('data-state', 'ok');
        note.textContent = 'paired — opening fleet';
        // The cookie is set; the app is one navigation away.
        window.location.href = '/';
      } else {
        note.setAttribute('data-state', 'error');
        note.textContent = result.body.error || 'that code did not match';
      }
    }).catch(function () {
      note.setAttribute('data-state', 'error');
      note.textContent = 'could not reach fleet';
    });
  });
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::bridge::Bridge;
    use std::io::{Read, Write};
    use std::time::Duration;

    /// A temp home, unique per call: a pairing window and a device store
    /// are per-test state, and a shared path would let one test's
    /// redemption burn another's window.
    fn fresh_home() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home = std::env::temp_dir().join(format!(
            "fleet-remote-listener-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        home
    }

    /// One request, one reply, read to the end — every reply this face
    /// writes closes its connection.
    fn ask(port: u16, raw: &str) -> String {
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("the listener is listening");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
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

    fn get(port: u16, path: &str, token: &str) -> String {
        ask(
            port,
            &format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: {DEVICE_COOKIE}={token}\r\n\r\n"),
        )
    }

    /// A live listener over a detached bridge, with a minted pairing
    /// window in the home. Returns the port, the home, and the code the
    /// window holds — the same code the terminal would have printed.
    async fn live(home: std::path::PathBuf) -> (u16, String) {
        let (_window, code) = PairingWindow::mint(&home).expect("a window");
        let (bridge, _) = Bridge::detached(home.clone(), "deepseek-chat".to_string(), "s".into());
        let remote = RemoteAddress {
            address: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            kind: crate::remote::AddressKind::Detected,
        };
        let (listener, port) = Listener::bind(&remote, 0, home, Arc::new(bridge))
            .await
            .expect("bind");
        tokio::spawn(async move {
            let _ = listener.serve().await;
        });
        (port, code)
    }

    /// Redeem the window and return the token the reply's cookie carries.
    fn pair(port: u16, code: &str) -> (String, String) {
        let body = serde_json::json!({"code": code, "name": "the test phone"}).to_string();
        let reply = ask(
            port,
            &format!(
                "POST /api/pair HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
        );
        let cookie = reply
            .lines()
            .find_map(|line| {
                line.strip_prefix("Set-Cookie: ")
                    .and_then(|value| value.strip_prefix(&format!("{DEVICE_COOKIE}=")))
                    .map(|value| value.split(';').next().unwrap_or(value).to_string())
            })
            .unwrap_or_default();
        (reply, cookie)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_pairing_code_redeems_once_and_its_token_works() {
        let home = fresh_home();
        let (port, code) = live(home.clone()).await;

        // Before pairing: an API route with no token is refused, and the
        // refusal names the way in rather than the token.
        let refused = ask(
            port,
            "GET /api/state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(refused.starts_with("HTTP/1.1 401"), "{refused}");
        assert!(refused.contains("fleet pair"), "{refused}");

        // The code redeems: a device is written, and the token rides a
        // cookie from here on.
        let (redeemed, token) = pair(port, &code);
        assert!(redeemed.starts_with("HTTP/1.1 200"), "{redeemed}");
        assert_eq!(token.len(), 43, "the token is 43 characters: {redeemed}");
        let store = DeviceStore::load(&home).unwrap();
        assert_eq!(store.devices.len(), 1, "{store:?}");
        assert_eq!(store.devices[0].name, "the test phone");

        // The token works on an allowlisted route.
        let state = get(port, "/api/state", &token);
        assert!(state.starts_with("HTTP/1.1 200"), "{state}");
        assert!(state.contains("\"session\""), "{state}");

        // A second redemption of the same code has no window left: the
        // code paired exactly one device.
        let again = pair(port, &code);
        assert!(again.0.starts_with("HTTP/1.1 409"), "{}", again.0);
        let store = DeviceStore::load(&home).unwrap();
        assert_eq!(store.devices.len(), 1, "a second pairing wrote a device");

        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_paired_device_drives_the_session_and_nothing_else() {
        let home = fresh_home();
        let (port, code) = live(home.clone()).await;
        let (_redeemed, token) = pair(port, &code);

        // What a paired device may do.
        for path in ["/api/state", "/api/sessions"] {
            let reply = get(port, path, &token);
            assert!(reply.starts_with("HTTP/1.1 200"), "{path}: {reply}");
        }

        // What it may not: the loopback face's own routes, refused by
        // name so a later addition there stays closed here.
        for (method, path) in [
            ("GET", "/api/settings"),
            ("GET", "/api/tree"),
            ("GET", "/api/memory"),
            ("POST", "/api/fact"),
            ("POST", "/api/skill"),
            ("POST", "/api/settings"),
        ] {
            let raw = if method == "GET" {
                format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: {DEVICE_COOKIE}={token}\r\n\r\n")
            } else {
                format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nCookie: {DEVICE_COOKIE}={token}\r\n\r\n{{}}")
            };
            let reply = ask(port, &raw);
            assert!(reply.starts_with("HTTP/1.1 403"), "{method} {path}: {reply}");
            assert!(reply.contains(path), "the refusal names the route: {reply}");
        }

        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_revoked_device_stops_working() {
        let home = fresh_home();
        let (port, code) = live(home.clone()).await;
        let (_redeemed, token) = pair(port, &code);
        assert!(get(port, "/api/state", &token).starts_with("HTTP/1.1 200"));

        let mut store = DeviceStore::load(&home).unwrap();
        let id = store.devices[0].id.clone();
        assert!(store.revoke(&id));
        store.save(&home).unwrap();

        let after = get(port, "/api/state", &token);
        assert!(after.starts_with("HTTP/1.1 401"), "{after}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_pairing_page_needs_no_token_and_a_wrong_code_is_counted() {
        let home = fresh_home();
        let (port, _code) = live(home.clone()).await;

        // The page itself is the one unauthenticated route, and it is a
        // form: nothing to leak.
        let page = ask(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        assert!(page.contains("/api/pair"), "{page}");

        // A wrong code is counted, and the count is reported.
        let body = serde_json::json!({"code": "WRONGCODEXYZ", "name": "x"}).to_string();
        let wrong = ask(
            port,
            &format!(
                "POST /api/pair HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
        );
        assert!(wrong.starts_with("HTTP/1.1 403"), "{wrong}");
        assert!(wrong.contains("attempts_left"), "{wrong}");

        // And no device was written by the wrong code.
        let store = DeviceStore::load(&home).unwrap();
        assert!(store.devices.is_empty(), "{store:?}");
        std::fs::remove_dir_all(&home).ok();
    }
}
