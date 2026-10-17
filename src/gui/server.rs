//! The loopback HTTP surface behind `fleet gui`.
//!
//! Hand-rolled HTTP/1.1 over tokio's `TcpListener`, because the crate
//! contract is one static Rust binary: a framework would buy this
//! surface nothing it does not already do by hand — four GET routes,
//! one SSE stream, three actions, and a token on every one of them.
//!
//! ## The security shape
//!
//! - **Loopback only.** The listener binds a loopback address, and a
//!   `--host` that is not one is refused by name before a socket
//!   exists. This face is the owner's machine and nothing else.
//! - **One token per launch**, minted from the operating system's
//!   entropy and printed with the URL. Every route requires it — from
//!   the `x-fleet-token` header, the `token` query parameter, or the
//!   `fleet_gui` cookie the page's first reply sets — so there is no
//!   anonymous page, no anonymous asset, no anonymous stream, and no
//!   anonymous action. A port scan sees 401s and learns nothing else.
//! - **No credentials in any payload.** The page, the assets, the
//!   stream, and the action replies carry names and presence, never
//!   values: the masking rule doctor holds is the rule here.
//!
//! ## The stream and the actions
//!
//! `/events` is the only long-lived connection. It answers a hello that
//! tells the page where it is, then a heartbeat on an interval for as
//! long as the browser listens. Everything the face shows — a run's
//! turns, a route's decision, a memory's recall, a tool's approval seat
//! — rides this same stream as one more event shape, so G1's seam is
//! the wire the rest of the face speaks on. The actions are the other
//! direction: `POST /api/run` hands the browser's task to the agent
//! thread, and `POST /api/approve` answers the seat that thread is
//! waiting on. A page that has just loaded reads `GET /api/state` for
//! the replay, then subscribes for what comes next; the two agree
//! because they are the same log.
//!
//! The read routes beyond `/api/state` are the panels': `GET
//! /api/router` is the router board's own reads plus the
//! route-decision cache's counts, read off the async worker, `GET
//! /api/delegation` is the live run's delegation tree off the session
//! logs beside the dependency graph's own file, `GET /api/memory` is
//! the corpus and the day's recall hits, and `GET /api/skills` is the
//! library with its skips, its proposals, and its curation candidates.
//! A panel's document is the board's facts, not a second opinion about
//! them, and it never carries a credential.
//!
//! The write actions are the panels' own, and each rides the
//! substrate's path rather than a file write of this surface's: `POST
//! /api/fact` edits a fact in place through memory's own rewriter, and
//! `POST /api/skill` approves a skills proposal through improve's own
//! apply — the same gate `fleet improve apply` runs, so the reply waits
//! on it. Neither can reach a credential, and neither writes anything
//! the substrate would refuse.

use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::gui::bridge::Bridge;

/// The page, its stylesheet, and its script, embedded in the binary:
/// there is no asset directory to ship beside it and nothing served
/// from disk.
const INDEX: &str = include_str!("assets/index.html");
const STYLES: &str = include_str!("assets/app.css");
const SCRIPT: &str = include_str!("assets/app.js");

/// The cookie the page's first reply sets, so the requests the open
/// page makes afterwards need no token in a URL.
const COOKIE_NAME: &str = "fleet_gui";

/// How often the stream says the server is still there. The page counts
/// the beats, so a silent stream is a dead server the owner can see.
const HEARTBEAT: Duration = Duration::from_secs(5);

/// The largest request head read from one connection. A browser's head
/// is a few hundred bytes; the cap is what stops a peer that never
/// sends the terminator from holding a task forever.
const MAX_HEAD_BYTES: usize = 16 * 1024;

/// The largest request body this surface reads. A task is a paragraph
/// and a decision is a word; the cap is what stops a peer that declares
/// a length and never sends it from holding a task forever.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// The token's length in bytes before hex encoding: 16 bytes is 128
/// bits, which is where a guess stops being a threat model.
const TOKEN_BYTES: usize = 16;

/// One launch's face: the listener, the token, the address the owner
/// was given, and the bridge to the agent thread the actions speak to.
/// A `Server` is deliberately not cloneable — one launch, one token,
/// one listener — and the per-connection tasks carry what they need by
/// value instead.
pub struct Server {
    token: String,
    base_url: String,
    heartbeat: Duration,
    listener: tokio::net::TcpListener,
    bridge: Arc<Bridge>,
}

impl Server {
    /// Binds the loopback listener and mints the launch's token. `port`
    /// zero asks the operating system for a free one, which is what a
    /// second concurrent launch and the tests need.
    pub async fn bind(host: &str, port: u16, bridge: Arc<Bridge>) -> Result<(Server, u16), String> {
        let address = loopback_address(host)?;
        let listener = tokio::net::TcpListener::bind((address, port))
            .await
            .map_err(|e| format!("fleet gui cannot bind {address}:{port}: {e}"))?;
        let bound = listener
            .local_addr()
            .map_err(|e| format!("fleet gui cannot read its own address: {e}"))?;
        let base_url = format!("http://{bound}");
        let token = mint_token()?;
        let server = Server {
            token,
            base_url,
            heartbeat: HEARTBEAT,
            listener,
            bridge,
        };
        Ok((server, bound.port()))
    }

    /// The address the owner opens, without the token: the token rides
    /// the query of the printed URL and nowhere else.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// This launch's token. Printed once, to the terminal, with the URL.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Serves until the owner stops the process. Ctrl-C ends it
    /// cleanly, and every connection is a task of its own, so one slow
    /// stream never holds the listener.
    pub async fn serve(&self) -> Result<(), String> {
        loop {
            tokio::select! {
                accepted = self.listener.accept() => match accepted {
                    Ok((stream, _from)) => {
                        let token = self.token.clone();
                        let base_url = self.base_url.clone();
                        let heartbeat = self.heartbeat;
                        let bridge = Arc::clone(&self.bridge);
                        tokio::spawn(async move {
                            if let Err(e) =
                                serve_connection(stream, &token, &base_url, heartbeat, &bridge).await
                            {
                                eprintln!("fleet gui: {e}");
                            }
                        });
                    }
                    // A connection that failed to arrive is not a reason
                    // to stop serving the ones that do.
                    Err(e) => eprintln!("fleet gui: a connection did not arrive: {e}"),
                },
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }
}

/// One connection, start to finish: read the head, gate on the token,
/// route, answer, and drop the socket. Every reply says
/// `Connection: close` and means it — a browser reconnects, and a
/// scanner learns nothing from a held-open port.
async fn serve_connection(
    mut stream: tokio::net::TcpStream,
    token: &str,
    base_url: &str,
    heartbeat: Duration,
    bridge: &Arc<Bridge>,
) -> Result<(), String> {
    let (head, rest) = read_head(&mut stream).await?;
    let request = Request::parse(&head)?;
    if request.presented_token() != Some(token) {
        return write_reply(&mut stream, unauthorized()).await;
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => write_reply(&mut stream, page(token)).await,
        ("GET", "/app.css") => {
            write_reply(&mut stream, asset("text/css; charset=utf-8", STYLES)).await
        }
        ("GET", "/app.js") => {
            write_reply(&mut stream, asset("text/javascript; charset=utf-8", SCRIPT)).await
        }
        ("GET", "/events") => {
            stream_events(&mut stream, base_url, heartbeat, bridge.subscribe()).await
        }
        ("GET", "/api/state") => write_reply(&mut stream, api(200, bridge.state())).await,
        ("GET", "/api/router") => write_reply(&mut stream, api(200, bridge.router().await)).await,
        ("GET", "/api/delegation") => {
            write_reply(&mut stream, api(200, bridge.delegation().await)).await
        }
        ("GET", "/api/memory") => write_reply(&mut stream, api(200, bridge.memory().await)).await,
        ("GET", "/api/skills") => write_reply(&mut stream, api(200, bridge.skills().await)).await,
        // The pane's two reads. Both are contained to the working
        // directory by the bridge, and both refuse a credential file by
        // name — the browser can ask for a file now, which is exactly
        // where one would leak.
        ("GET", "/api/tree") => {
            let asked = decode_query(&request.query, "path");
            write_reply(&mut stream, api(200, bridge.tree(&asked).await)).await
        }
        ("GET", "/api/file") => {
            let asked = decode_query(&request.query, "path");
            write_reply(&mut stream, api(200, bridge.file(&asked).await)).await
        }
        ("POST", "/api/run") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let task = match task_of(&body) {
                Ok(task) => task,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.run(&task) {
                Ok(()) => {
                    write_reply(
                        &mut stream,
                        api(200, json!({"type": "run", "task": task, "accepted": true})),
                    )
                    .await
                }
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/approve") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let decision = match decision_of(&body) {
                Ok(decision) => decision,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            match bridge.answer(&decision) {
                Ok(answered) => write_reply(&mut stream, api(200, answered)).await,
                Err(crate::gui::bridge::AnswerError::NoSeat) => {
                    write_reply(
                        &mut stream,
                        api(409, json!({"error": "no approval is waiting"})),
                    )
                    .await
                }
                Err(crate::gui::bridge::AnswerError::Unknown(word)) => {
                    write_reply(&mut stream, api(400, json!({"error": word}))).await
                }
            }
        }
        ("POST", "/api/fact") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            // The edit rides the substrate's own rewriter, so the body is
            // the fact-edit's own arguments object rather than a shape
            // this surface invented.
            match bridge.fact_edit(&body).await {
                Ok(edited) => write_reply(&mut stream, api(200, edited)).await,
                Err(message) => write_reply(&mut stream, api(400, json!({"error": message}))).await,
            }
        }
        ("POST", "/api/skill") => {
            let body = match read_body(&mut stream, &request, rest).await {
                Ok(body) => body,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            let name = match proposal_of(&body) {
                Ok(name) => name,
                Err(message) => return write_reply(&mut stream, bad_request(&message)).await,
            };
            // The gate is the real one, so the reply waits on it: an
            // approve that answered before the gate ran would be an
            // approve of nothing.
            match bridge.skill_apply(&name).await {
                Ok(applied) => write_reply(&mut stream, api(200, applied)).await,
                Err(message) => write_reply(&mut stream, api(409, json!({"error": message}))).await,
            }
        }
        ("GET", _) => write_reply(&mut stream, not_found(&request.path)).await,
        _ => write_reply(&mut stream, method_not_allowed(&request.method)).await,
    }
}

/// Reads one request head and whatever arrived past it: bytes up to the
/// `\r\n\r\n` that ends them, capped so a peer that never sends it
/// cannot hold a task forever. The second value is the body bytes that
/// rode the same read — a browser sends a short POST in one packet, so
/// discarding them would lose the task the owner just typed.
async fn read_head(stream: &mut tokio::net::TcpStream) -> Result<(String, Vec<u8>), String> {
    let mut head: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let end = loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("fleet gui could not read the request: {e}"))?;
        if read == 0 {
            break None;
        }
        head.extend_from_slice(&chunk[..read]);
        if let Some(end) = head_end(&head) {
            break Some(end);
        }
        if head.len() > MAX_HEAD_BYTES {
            return Err("fleet gui refused an oversized request head".to_string());
        }
    };
    let (head, rest) = match end {
        Some(end) => (
            String::from_utf8(head[..end].to_vec())
                .map_err(|_| "fleet gui refused a non-UTF-8 request head".to_string())?,
            head[end..].to_vec(),
        ),
        None => (
            String::from_utf8(head)
                .map_err(|_| "fleet gui refused a non-UTF-8 request head".to_string())?,
            Vec::new(),
        ),
    };
    Ok((head, rest))
}

/// Reads one request body: the `Content-Length` bytes the head declared,
/// starting with the bytes that rode past the head's terminator. A body
/// larger than the cap is refused rather than read, and a body that
/// ends early is a malformed request — both are the caller's 400.
async fn read_body(
    stream: &mut tokio::net::TcpStream,
    request: &Request,
    rest: Vec<u8>,
) -> Result<String, String> {
    let length = request
        .header("content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(format!(
            "fleet gui refused a body of {length} bytes; the cap is {MAX_BODY_BYTES}"
        ));
    }
    let mut body = rest;
    if body.len() > length {
        body.truncate(length);
    }
    let mut chunk = [0u8; 4096];
    while body.len() < length {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("fleet gui could not read the body: {e}"))?;
        if read == 0 {
            return Err(format!(
                "fleet gui read {} of the {length} bytes the head declared",
                body.len()
            ));
        }
        body.extend_from_slice(&chunk[..read]);
        body.truncate(length.min(body.len()));
    }
    String::from_utf8(body).map_err(|_| "fleet gui refused a non-UTF-8 body".to_string())
}

/// The task a run's body carries. Anything that is not a JSON object
/// with a `task` string is a malformed request, and saying so is the
/// whole reply.
fn task_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the run's body: {e}"))?;
    parsed
        .get("task")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "a run's body is a JSON object with a task string".to_string())
}

/// The decision an approval's body carries, in the seat's four words.
fn decision_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the answer's body: {e}"))?;
    parsed
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "an answer's body is a JSON object with a decision string".to_string())
}

/// The proposal an approve's body carries.
fn proposal_of(body: &str) -> Result<String, String> {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("fleet gui could not read the approve's body: {e}"))?;
    parsed
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "an approve's body is a JSON object with a name string".to_string())
}

/// Where a head ends: just past the first `\r\n\r\n`.
fn head_end(head: &[u8]) -> Option<usize> {
    head.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// A request head, parsed to what this surface routes on: the method,
/// the path, the query, and the headers a token can arrive in.
struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
}

impl Request {
    /// Parses the head. A malformed line is an error the caller turns
    /// into a closed connection: this surface's inputs are a browser's,
    /// and there is nothing here worth a 400 page.
    fn parse(head: &str) -> Result<Request, String> {
        let mut lines = head.split("\r\n");
        let request_line = lines.next().unwrap_or_default();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let target = parts.next().unwrap_or_default().to_string();
        if method.is_empty() || target.is_empty() {
            return Err(format!(
                "fleet gui refused a malformed request line: {request_line:?}"
            ));
        }
        let (path, query) = match target.split_once('?') {
            Some((path, query)) => (path.to_string(), query.to_string()),
            None => (target, String::new()),
        };
        let mut headers = Vec::new();
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        Ok(Request {
            method,
            path,
            query,
            headers,
        })
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }

    /// The token this request carries, from the three places a browser
    /// can put one: the header a fetch would send, the query the page
    /// load and the EventSource send, and the cookie the open page
    /// sends once the first reply has set it.
    fn presented_token(&self) -> Option<&str> {
        if let Some(value) = self.header("x-fleet-token") {
            return Some(value);
        }
        if let Some(value) = query_value(&self.query, "token") {
            return Some(value);
        }
        let cookie = self.header("cookie")?;
        cookie
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE_NAME)
            .map(|(_, value)| value)
    }
}

/// One query parameter's value, percent-decoded. The pane's paths are
/// the exotic case the token never was: a file path carries spaces,
/// hashes and non-ASCII, and the browser encodes all of them before they
/// reach the wire, so they have to come back off it.
fn decode_query(query: &str, name: &str) -> String {
    let raw = query_value(query, name).unwrap_or_default();
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).ok();
            if let Some(value) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(value);
                at += 3;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// One query parameter's value. The token is hex, so nothing needs
/// decoding; anything exotic is the caller's problem, not this
/// surface's.
fn query_value<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

/// One reply, assembled as bytes before it touches the socket: the
/// status line, the headers this surface always sends, and the body.
struct Reply {
    status: &'static str,
    content_type: &'static str,
    extra: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn bytes(&self) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status,
            self.content_type,
            self.body.len()
        );
        for (name, value) in &self.extra {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(self.body.as_bytes());
        bytes
    }
}

/// The page, with the cookie that lets the rest of the page's requests
/// ride the gate without a token in a URL. HttpOnly keeps the page's
/// own scripts from reading the capability back, and SameSite keeps
/// another site's requests from sending it.
fn page(token: &str) -> Reply {
    Reply {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        extra: vec![("Set-Cookie".to_string(), cookie(token))],
        body: INDEX.to_string(),
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

fn cookie(token: &str) -> String {
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict")
}

/// The gate's refusal. It names the rule and never the token: a caller
/// without one learns that a token is needed, and nothing more.
fn unauthorized() -> Reply {
    Reply {
        status: "401 Unauthorized",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: "fleet gui serves this machine only, and this route needs the launch token — the URL fleet printed carries it.\n"
            .to_string(),
    }
}

fn not_found(path: &str) -> Reply {
    Reply {
        status: "404 Not Found",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("fleet gui has no route for {path}.\n"),
    }
}

/// An action's answer, as JSON: the state document, the run's receipt,
/// or the answer's outcome. One shape for every reply the browser
/// parses, so the page reads one content type forever.
fn api(status: u16, body: serde_json::Value) -> Reply {
    Reply {
        status: status_line(status),
        content_type: "application/json; charset=utf-8",
        extra: Vec::new(),
        body: format!("{body}\n"),
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

/// The status line this surface writes. The set is small and fixed —
/// five lines — and naming them is cheaper than a status-code crate for
/// a surface that answers nothing else.
fn status_line(status: u16) -> &'static str {
    match status {
        200 => "200 OK",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        409 => "409 Conflict",
        _ => "500 Internal Server Error",
    }
}

fn method_not_allowed(method: &str) -> Reply {
    Reply {
        status: "405 Method Not Allowed",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!(
            "fleet gui answers GET on its pages and POST on its actions; {method} is neither.\n"
        ),
    }
}

async fn write_reply(stream: &mut tokio::net::TcpStream, reply: Reply) -> Result<(), String> {
    stream
        .write_all(&reply.bytes())
        .await
        .map_err(|e| format!("fleet gui could not write the reply: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("fleet gui could not flush the reply: {e}"))
}

/// The stream: the head, a hello that tells the page where it is, then
/// a beat on an interval and every event the bridge broadcasts, for as
/// long as the browser listens. A write that fails is the browser being
/// gone — the stream ends with it, quietly, because that is the normal
/// way a tab closes. A browser too slow to keep up is told how many
/// events it missed rather than being left to wonder, and a closed
/// channel is the agent thread being gone, which ends the stream.
async fn stream_events(
    stream: &mut tokio::net::TcpStream,
    base_url: &str,
    heartbeat: Duration,
    mut events: tokio::sync::broadcast::Receiver<serde_json::Value>,
) -> Result<(), String> {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| format!("fleet gui could not open the stream: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("fleet gui could not open the stream: {e}"))?;
    write_event(stream, &json!({"type": "hello", "gui": base_url})).await?;
    let mut ticks = tokio::time::interval(heartbeat);
    // The interval's first tick completes at once; skipping it puts the
    // first heartbeat a full interval out, which is what a beat means.
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

/// One server-sent event: the `data:` line and the blank line that ends
/// it. The payloads are JSON so the page parses one shape forever.
async fn write_event(
    stream: &mut tokio::net::TcpStream,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let event = format!("data: {payload}\n\n");
    stream
        .write_all(event.as_bytes())
        .await
        .map_err(|e| format!("fleet gui could not write a beat: {e}"))
}

/// Resolves `--host` to a loopback address or refuses it by name. The
/// surface never leaves this machine, so a routable address, a
/// wildcard, or a name that is not `localhost` is a configuration
/// error, and naming the refused host is the whole error. The parser
/// shares this so the refusal is a usage error at the prompt, not a
/// run that starts and dies.
pub(crate) fn loopback_address(host: &str) -> Result<IpAddr, String> {
    if host == "localhost" {
        return Ok(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
    match host.parse::<IpAddr>() {
        Ok(address) if address.is_loopback() => Ok(address),
        _ => Err(format!(
            "fleet gui serves loopback only; refusing --host {host}"
        )),
    }
}

/// Reads the operating system's entropy for this launch's token. A
/// failure here is loud and fatal: a predictable token is worse than no
/// server at all, and there is no weak fallback to fall back to.
fn mint_token() -> Result<String, String> {
    let mut entropy = std::fs::File::open("/dev/urandom")
        .map_err(|e| format!("fleet gui cannot read /dev/urandom: {e}"))?;
    let mut bytes = [0u8; TOKEN_BYTES];
    entropy
        .read_exact(&mut bytes)
        .map_err(|e| format!("fleet gui cannot read /dev/urandom: {e}"))?;
    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        token.push_str(&format!("{byte:02x}"));
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A live server on an OS-picked port, with a heartbeat short enough
    /// that a test does not wait five seconds for a beat. The server
    /// runs on its own task; the client below blocks on the test's
    /// thread, which is why these tests ask for a multi-thread runtime.
    /// The bridge behind it is detached — no agent thread — so the
    /// routes answer exactly what the bridge holds and nothing the
    /// network would decide.
    /// A fresh temp home, empty. Each call makes its own directory: a name
    /// shared per process would let one test's `fresh_home` wipe another
    /// test's home while its server is answering, and a home whose
    /// config.json just vanished sends the board's read looking for the
    /// machine's own calibration log instead of an empty tree.
    fn fresh_home() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let home =
            std::env::temp_dir().join(format!("fleet-gui-server-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        home
    }

    /// The home a router-panel test reads: a fresh home whose config
    /// names its own calibration root, so the board's route rows come
    /// from an empty tree rather than the machine the test runs on. The
    /// file's only field is the log root, so the roster is the
    /// compiled-in defaults — exactly what a fresh install reads.
    fn router_home() -> std::path::PathBuf {
        let home = fresh_home();
        std::fs::write(
            home.join("config.json"),
            format!(
                r#"{{"calib":{{"log_root":"{}"}}}}"#,
                home.join("logs").display()
            ),
        )
        .expect("a config naming the temp calibration root");
        home
    }

    /// The home a delegation-panel test reads: the bridge's own session
    /// id has a log, it delegated to one child, and the home declares a
    /// graph with one node in each state the code names. A fresh home
    /// would render a panel with nothing in it, which is the one case
    /// the panel is not for.
    fn delegation_home() -> std::path::PathBuf {
        let home = fresh_home();
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        let log = |id: &str, kind: Option<&str>, parent: Option<&str>, stop: Option<&str>| {
            let mut start = serde_json::json!({
                "type": "start",
                "v": crate::session::SESSION_FORMAT_VERSION,
                "model": "deepseek-chat",
                "created": "2026-10-16T09:00:00Z",
            });
            if let Some(kind) = kind {
                start["kind"] = serde_json::json!(kind);
            }
            if let Some(parent) = parent {
                start["parent"] = serde_json::json!(parent);
            }
            let mut lines = vec![
                start.to_string(),
                serde_json::json!({"type": "user", "content": "revamp the delegation panel"})
                    .to_string(),
            ];
            if let Some(stop) = stop {
                lines.push(
                    serde_json::json!({"type": "end", "stop_reason": stop, "turns": 2}).to_string(),
                );
            }
            std::fs::write(
                sessions.join(format!("{id}.jsonl")),
                format!("{}\n", lines.join("\n")),
            )
            .expect("a session log");
        };
        // The bridge's own session, and the child it delegated to.
        log("session-test", None, None, None);
        log(
            "20261016T090100Z-child",
            Some(crate::session::SUBAGENT_KIND),
            Some("session-test"),
            Some("completed"),
        );
        let node = |id: &str, cost: u64, prereqs: &[&str], paths: &[&str], status: &str| {
            serde_json::json!({
                "id": id,
                "cost": cost,
                "prereqs": prereqs,
                "paths": paths,
                "capabilities": { "tools": ["read"], "skills": [], "workflows": [], "loops": [] },
                "approach": "the approach",
                "status": status,
            })
        };
        std::fs::write(
            crate::graph::path_in(&home),
            serde_json::to_string_pretty(&serde_json::json!({
                "nodes": [
                    node("probe", 2, &[], &[], "verified"),
                    node("plan", 3, &["probe"], &[], "pending"),
                    // Held: `run` is in flight on this node's admitted
                    // path, so the resource condition is the one that
                    // fails and the panel must say so rather than
                    // reading the node as ready.
                    node("audit", 1, &[], &["src/boards.rs"], "pending"),
                    node("run", 1, &[], &["src/boards.rs"], "running"),
                ],
                "revisions": [],
            }))
            .expect("the graph json"),
        )
        .expect("the declared graph");
        home
    }

    /// The home a memory-panel test reads: a corpus it built itself and a
    /// calibration root it owns, so the facts and the hits are the test's
    /// own rather than the machine's. The corpus is handed to the bridge
    /// through [`Bridge::point_at_memories`], the seam that keeps the
    /// production rule — the agent's own resolve off the working
    /// directory — out of the test's way.
    fn memory_home() -> (std::path::PathBuf, std::path::PathBuf) {
        let home = router_home();
        let corpus = fresh_home();
        let day = crate::calib::today();
        std::fs::write(
            corpus.join("a-fact.md"),
            format!(
                "---\nname: a-fact\ndescription: \"the corpus is one fact per file\"\ntype: project\ndate: {day}\nbasis: inferred\ntier: durable\nshape: []\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body.\n"
            ),
        )
        .expect("a fact file");
        let logs = home
            .join("logs")
            .join(&day[..4])
            .join(&day[5..7])
            .join(&day[8..10]);
        std::fs::create_dir_all(&logs).expect("the day's log directory");
        std::fs::write(
            logs.join("events.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::json!({
                    "op": crate::calib::OP_RECALL_HIT,
                    "ts": format!("{day}T09:00:00Z"),
                    "fact": "a-fact",
                    "shape": "docs",
                    "score": 0.82,
                }),
                serde_json::json!({
                    "op": crate::calib::OP_TOOL_SCREEN,
                    "ts": format!("{day}T09:00:01Z"),
                    "tool": "read",
                    "verdict": "allow",
                }),
            ),
        )
        .expect("the day's stream");
        (home, corpus)
    }

    /// A live server over `home`, whose memory panel reads `corpus`.
    async fn launch_over(home: std::path::PathBuf, corpus: std::path::PathBuf) -> (String, u16) {
        let (mut bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        bridge.point_at_memories(corpus);
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .unwrap();
        server.heartbeat = Duration::from_millis(20);
        let token = server.token().to_string();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (token, port)
    }

    async fn launch() -> (String, u16) {
        launch_at(fresh_home()).await
    }

    /// A live server on an OS-picked port, with a heartbeat short enough
    /// that a test does not wait five seconds for a beat. The server
    /// runs on its own task; the client below blocks on the test's
    /// thread, which is why these tests ask for a multi-thread runtime.
    /// The bridge behind it is detached — no agent thread — so the
    /// routes answer exactly what the bridge holds and nothing the
    /// network would decide.
    async fn launch_at(home: std::path::PathBuf) -> (String, u16) {
        let (bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        let (mut server, port) = Server::bind("127.0.0.1", 0, Arc::new(bridge))
            .await
            .unwrap();
        server.heartbeat = Duration::from_millis(20);
        let token = server.token().to_string();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (token, port)
    }

    /// A POST with a JSON body and the token in the query, which is how
    /// the browser's fetches arrive.
    fn post(port: u16, path: &str, token: &str, body: &str) -> String {
        ask(
            port,
            &format!(
                "POST {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{body}",
                body.len()
            ),
            None,
        )
    }

    /// Sends one request and reads the reply, stopping when `needle`
    /// arrives or the reply ends. `None` reads to the end, which is how
    /// a normal reply arrives: the server closes every connection it
    /// answers.
    fn ask(port: u16, raw: &str, needle: Option<&str>) -> String {
        let mut stream =
            std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server is listening");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("a read timeout is settable");
        stream
            .write_all(raw.as_bytes())
            .expect("the request is sent");
        let mut reply = String::new();
        let mut chunk = [0u8; 4096];
        loop {
            if needle.is_some_and(|needle| reply.contains(needle)) {
                break;
            }
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => reply.push_str(&String::from_utf8_lossy(&chunk[..read])),
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(e) => panic!("the reply could not be read: {e}"),
            }
        }
        reply
    }

    /// A GET with the token in the query, which is how the browser's
    /// page load and its EventSource arrive.
    fn get(port: u16, path: &str, token: &str) -> String {
        ask(
            port,
            &format!("GET {path}?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn every_route_needs_the_launch_token() {
        let (token, port) = launch().await;
        let anonymous = ask(port, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", None);
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let wrong = ask(
            port,
            "GET /?token=deadbeef HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(wrong.starts_with("HTTP/1.1 401"), "{wrong}");
        let right = get(port, "/", &token);
        assert!(right.starts_with("HTTP/1.1 200"), "{right}");
        assert!(right.contains("<title>fleet</title>"), "{right}");
        // The refusal names the rule and never the token.
        assert!(!anonymous.contains(&token), "{anonymous}");
        assert!(anonymous.contains("launch token"), "{anonymous}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_token_arrives_in_a_header_a_query_or_a_cookie() {
        let (token, port) = launch().await;
        let by_header = ask(
            port,
            &format!("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nx-fleet-token: {token}\r\n\r\n"),
            None,
        );
        assert!(by_header.starts_with("HTTP/1.1 200"), "{by_header}");
        let page = get(port, "/", &token);
        assert!(page.contains(&format!("fleet_gui={token}")), "{page}");
        let by_cookie = ask(
            port,
            &format!("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: fleet_gui={token}\r\n\r\n"),
            None,
        );
        assert!(by_cookie.starts_with("HTTP/1.1 200"), "{by_cookie}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_assets_ride_behind_the_token() {
        let (token, port) = launch().await;
        for path in ["/app.css", "/app.js"] {
            let anonymous = ask(
                port,
                &format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
                None,
            );
            assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
            let served = get(port, path, &token);
            assert!(served.starts_with("HTTP/1.1 200"), "{served}");
        }
        let css = get(port, "/app.css", &token);
        // The real stylesheet, and the visual language intact: the plan's
        // colour chain has three tiers and nothing skips one, so the face
        // is not shipped with a bare token standing in for the alias it
        // should be reading.
        for tier in ["--dsw-static-", "--dsw-alias-", "--card-"] {
            assert!(css.contains(tier), "{css}");
        }
        let js = get(port, "/app.js", &token);
        assert!(js.contains("EventSource"), "{js}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_unknown_path_is_a_404_and_a_post_is_a_405() {
        let (token, port) = launch().await;
        let missing = get(port, "/nope", &token);
        assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
        assert!(missing.contains("/nope"), "{missing}");
        let posted = ask(
            port,
            &format!(
                "POST /?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n"
            ),
            None,
        );
        assert!(posted.starts_with("HTTP/1.1 405"), "{posted}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_stream_says_hello_and_then_keeps_a_beat() {
        let (token, port) = launch().await;
        let reply = ask(
            port,
            &format!("GET /events?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            Some("heartbeat"),
        );
        assert!(reply.contains("Content-Type: text/event-stream"), "{reply}");
        assert!(reply.contains("\"type\":\"hello\""), "{reply}");
        assert!(
            reply.contains(&format!("\"gui\":\"http://127.0.0.1:{port}\"")),
            "{reply}"
        );
        assert!(reply.contains("\"type\":\"heartbeat\""), "{reply}");
        assert!(reply.matches("data: ").count() >= 2, "{reply}");
        // The stream holds the connection open: it never says close in
        // the body, and the head's Connection line is the server's.
        let anonymous = ask(
            port,
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
    }

    /// A bridge with no agent thread behind it, for the tests that need
    /// a server to exist at all.
    fn bridge() -> Arc<Bridge> {
        let home = std::env::temp_dir().join(format!("fleet-gui-bind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("a temp home");
        let (bridge, _sources) = Bridge::detached(
            home,
            "deepseek-chat".to_string(),
            "session-test".to_string(),
        );
        Arc::new(bridge)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_host_that_is_not_loopback_is_refused_by_name() {
        for host in ["0.0.0.0", "192.168.1.5", "example.com"] {
            let refused = Server::bind(host, 0, bridge()).await;
            let error = refused
                .err()
                .unwrap_or_else(|| panic!("{host} must be refused"));
            assert!(error.contains(host), "{error}");
            assert!(error.contains("loopback"), "{error}");
        }
    }

    /// A loopback host binds and mints a token no guess reaches. The
    /// must-bind set is 127.0.0.1 and `localhost`, and the URL names the
    /// address the listener holds rather than the name the owner typed,
    /// so it works on a machine that resolves neither.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_loopback_host_binds_and_mints_a_token_no_guess_reaches() {
        for host in ["127.0.0.1", "localhost"] {
            let (server, port) = Server::bind(host, 0, bridge()).await.expect(host);
            assert!(port > 0, "{host} bound no port");
            assert!(
                server.base_url().starts_with("http://127.0.0."),
                "{}",
                server.base_url()
            );
            let token = server.token();
            assert_eq!(token.len(), TOKEN_BYTES * 2, "{token}");
            assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "{token}");
        }
        // A loopback address the machine has not assigned to its
        // interface is the operating system's answer, not this
        // surface's: Linux routes all of 127.0.0.0/8 to loopback while
        // macOS assigns .1 alone, so the address either binds or fails
        // with itself named — never a quiet move to another port.
        if let Err(message) = Server::bind("127.0.0.2", 0, bridge()).await {
            assert!(message.contains("127.0.0.2"), "{message}");
            assert!(message.contains("cannot bind"), "{message}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_state_document_is_the_replay_the_stream_agrees_with() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let state = get(port, "/api/state", &token);
        assert!(state.starts_with("HTTP/1.1 200"), "{state}");
        assert!(state.contains("application/json"), "{state}");
        assert!(state.contains("\"session\":\"session-test\""), "{state}");
        assert!(state.contains("\"model\":\"deepseek-chat\""), "{state}");
        assert!(state.contains("\"budget\""), "{state}");
        assert!(state.contains("\"running\":false"), "{state}");
        assert!(state.contains("\"events\":[]"), "{state}");
        assert!(state.contains("\"approval\":null"), "{state}");
        assert!(state.contains("\"rules\":[]"), "{state}");
    }

    /// The router panel's document is the board's own reads: the declared
    /// chain, the workload preference table, and the route-decision
    /// cache's counts, read off the async worker. A fresh home renders
    /// rather than refuses — a day with no rows is a ledger that says so.
    /// And nothing in it carries a credential.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_router_document_is_the_boards_own_read() {
        let (token, port) = launch_at(router_home()).await;
        let anonymous = ask(
            port,
            "GET /api/router HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let reply = get(port, "/api/router", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "router");
        // The declared chain, in the preference table's own order: each
        // workload's first candidate is what a route selects for it, and
        // the rest are what its failover walks.
        let tiers = document["tiers"].as_array().expect("the tier chains");
        assert!(!tiers.is_empty(), "{document}");
        assert!(
            tiers.iter().all(|tier| tier["candidates"]
                .as_array()
                .is_some_and(|candidates| !candidates.is_empty())),
            "{document}"
        );
        // The lane that would judge, and no pin: nothing is pinned on a
        // fresh install.
        assert_eq!(document["judge"]["lane"], "sys1");
        assert_eq!(document["pin"], serde_json::Value::Null);
        // An empty day is a ledger with nothing in it, not a refusal.
        assert_eq!(document["totals"], serde_json::Value::Null);
        assert!(
            document["recent"]
                .as_array()
                .is_some_and(|rows| rows.is_empty()),
            "{document}"
        );
        assert!(
            document["refused"]
                .as_array()
                .is_some_and(|refused| refused.is_empty()),
            "{document}"
        );
        // The cache is empty, and the panel says so rather than
        // presenting another lane's verdicts as this one's.
        assert_eq!(document["cache"]["entries"], 0);
        assert_eq!(document["cache"]["keyed"], false);
        assert!(
            document["cache"]["workloads"]
                .as_array()
                .is_some_and(|workloads| workloads.is_empty()),
            "{document}"
        );
        // Names and presence, never values.
        assert!(!body.contains("api_key"), "{body}");
        assert!(!body.contains("DEEPSEEK_API_KEY"), "{body}");
    }

    /// The delegation panel's document is the board's own read: the live
    /// run's tree off the session logs' `start` records, and the declared
    /// graph with each node's three unlock conditions carried separately.
    /// A home whose graph has declared nothing says so in `refused`
    /// rather than rendering an empty graph — an empty graph reads as
    /// "nothing is blocking anything", which is a different claim.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_delegation_document_is_the_boards_own_read() {
        let (token, port) = launch_at(delegation_home()).await;
        let anonymous = ask(
            port,
            "GET /api/delegation HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let reply = get(port, "/api/delegation", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "delegation");
        // The cap is the code's own, read off the subagents module.
        assert_eq!(document["depth_cap"], crate::subagents::SUBAGENT_MAX_DEPTH);
        // The live tree: the bridge's own session at the root, the child
        // it delegated to beneath it, and the child's stop.
        let live = &document["live"];
        assert_eq!(live["id"], "session-test");
        assert_eq!(live["status"], "running");
        assert_eq!(live["goal"], "revamp the delegation panel");
        let children = live["children"].as_array().expect("the children");
        assert_eq!(children.len(), 1, "{document}");
        assert_eq!(children[0]["kind"], crate::session::SUBAGENT_KIND);
        assert_eq!(children[0]["status"], "stopped");
        assert_eq!(children[0]["stop_reason"], "completed");
        assert_eq!(children[0]["depth"], 1);
        // The dependency graph, with the three conditions per node.
        let graph = &document["graph"];
        let nodes = graph["nodes"].as_array().expect("the nodes");
        assert_eq!(nodes.len(), 4, "{document}");
        // `plan`'s only prerequisite is verified, so a verified prereq
        // carries no cost into the score: all three conditions hold and
        // the node reads `ready`. The panel reports the three marks, not
        // one word derived from them.
        let plan = &nodes[1];
        assert_eq!(plan["state"], "ready");
        assert_eq!(plan["score"], 0);
        let unlock = &plan["unlock"];
        assert_eq!(unlock["score_zero"], true);
        assert_eq!(unlock["resource_free"], true);
        assert_eq!(unlock["deps_verified"], true);
        assert_eq!(unlock["unlocked"], true);
        // `audit` admits a path `run` is in flight on, so the resource
        // condition is the one that fails — score zero, deps verified,
        // and the holder named. A one-word summary would read this node
        // as ready, which is the difference the panel exists to show.
        let audit = &nodes[2];
        assert_eq!(audit["state"], "held");
        assert_eq!(audit["unlock"]["score_zero"], true);
        assert_eq!(audit["unlock"]["deps_verified"], true);
        assert_eq!(audit["unlock"]["resource_free"], false);
        assert_eq!(audit["unlock"]["unlocked"], false);
        assert_eq!(audit["unlock"]["held"][0]["holder"], "run");
        assert_eq!(audit["unlock"]["held"][0]["path"], "src/boards.rs");
        // The verified node's own closure is empty.
        assert_eq!(nodes[0]["state"], "verified");
        assert_eq!(nodes[0]["score"], 0);
        assert!(!body.contains("api_key"), "{body}");
    }

    /// The memory panel's document is the corpus and the day's recall
    /// hits: the facts with their bodies whole, because the panel's edit
    /// form needs the fact as it stands on disk, and the hits off the
    /// calibration stream's own rows — the screening rows between them do
    /// not ride along. Nothing in it carries a credential.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_memory_document_is_the_corpus_and_the_days_hits() {
        let (home, corpus) = memory_home();
        let (token, port) = launch_over(home, corpus.clone()).await;
        let anonymous = ask(
            port,
            "GET /api/memory HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let reply = get(port, "/api/memory", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "memory");
        // The corpus root is named, so the panel can say which one it is
        // showing.
        assert_eq!(document["root"], corpus.display().to_string());
        let facts = document["facts"].as_array().expect("the facts");
        assert_eq!(facts.len(), 1, "{document}");
        assert_eq!(facts[0]["name"], "a-fact");
        assert_eq!(facts[0]["body"], "The body.");
        assert_eq!(facts[0]["tier"], "durable");
        // The day's recall rows, oldest first — and the screening row
        // that sat between them is not one.
        let hits = document["hits"].as_array().expect("the hits");
        assert_eq!(hits.len(), 1, "{document}");
        assert_eq!(hits[0]["fact"], "a-fact");
        assert_eq!(hits[0]["shape"], "docs");
        assert!(
            document["refused"]
                .as_array()
                .is_some_and(|refused| refused.is_empty()),
            "{document}"
        );
        assert!(!body.contains("api_key"), "{body}");
        let _ = std::fs::remove_dir_all(&corpus);
    }

    /// A fact edits through the substrate's own rewriter: the named
    /// frontmatter lines are re-rendered and every other byte survives,
    /// so the panel's edit is the corpus's own write path rather than a
    /// second one. An unknown fact is a refusal that names it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_fact_edits_through_the_substrates_own_rewriter() {
        let (home, corpus) = memory_home();
        let (token, port) = launch_over(home, corpus.clone()).await;
        let reply = post(
            port,
            "/api/fact",
            &token,
            r#"{"name":"a-fact","description":"the corpus is one fact per file, rewritten","shape":"docs"}"#,
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["edited"], true);
        assert_eq!(
            document["fact"]["description"],
            "the corpus is one fact per file, rewritten"
        );
        assert_eq!(document["fact"]["shape"][0], "docs");
        // The body survived the rewrite: the rewriter owns one line and
        // preserves every other byte.
        assert_eq!(document["fact"]["body"], "The body.");
        let on_disk = std::fs::read_to_string(corpus.join("a-fact.md")).expect("the fact file");
        assert!(
            on_disk.contains("description: \"the corpus is one fact per file, rewritten\""),
            "{on_disk}"
        );
        assert!(on_disk.contains("shape: [docs]"), "{on_disk}");
        assert!(on_disk.contains("The body."), "{on_disk}");
        // A fact that is not there is refused by name, and the reason is
        // the part the owner needs.
        let unknown = post(
            port,
            "/api/fact",
            &token,
            r#"{"name":"no-such-fact","description":"a fact that is not there"}"#,
        );
        assert!(unknown.starts_with("HTTP/1.1 400"), "{unknown}");
        assert!(unknown.contains("no fact named"), "{unknown}");
        let _ = std::fs::remove_dir_all(&corpus);
    }

    /// The skills panel's document is the library with its skips, the
    /// proposals awaiting a decision, and the curation candidates. The
    /// curation card names KI-215 rather than offering a retire the
    /// substrate refuses to draft.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_skills_document_is_the_library_its_skips_and_its_proposals() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/skills HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let reply = get(port, "/api/skills", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.contains("application/json"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert_eq!(document["type"], "skills");
        // The library, its skips, its proposals, and its merge candidates
        // are all carried, each as its own list.
        assert!(document["skills"].is_array(), "{document}");
        assert!(document["skips"].is_array(), "{document}");
        assert!(document["proposals"].is_array(), "{document}");
        assert!(document["merges"].is_array(), "{document}");
        assert!(document["sessions"].is_number(), "{document}");
        assert!(document["unused"].is_number(), "{document}");
        assert!(
            document["curation_note"]
                .as_str()
                .is_some_and(|note| note.contains("KI-215")),
            "{document}"
        );
        // The canonical library is named, so the panel can say which one
        // it is showing.
        assert!(document["canonical"].is_string(), "{document}");
        assert!(!body.contains("api_key"), "{body}");
    }

    /// The approve route reaches the substrate's own apply and reports
    /// what it says. A proposal that is not awaiting a decision is
    /// refused by the apply itself, in its own words — and a proposal
    /// whose target is not a skill is refused before the gate, because
    /// this is the skills panel's approve and a source edit's corpus step
    /// is a CLI step, not a button.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_approve_route_reaches_the_substrates_own_apply() {
        let (token, port) = launch().await;
        // Nothing is awaiting a decision on a fresh home, so the apply
        // refuses it in its own words rather than the route inventing one.
        let missing = post(port, "/api/skill", &token, r#"{"name":"no-such-proposal"}"#);
        assert!(missing.starts_with("HTTP/1.1 409"), "{missing}");
        assert!(
            missing.contains("no proposal named") || missing.contains("awaiting a decision"),
            "{missing}"
        );
        // A body with no name is a malformed request, not an apply.
        let malformed = post(
            port,
            "/api/skill",
            &token,
            r#"{"proposal":"no-such-proposal"}"#,
        );
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    /// The pane's two routes, behind the launch token like every other.
    /// The tree reads the working directory this face was launched from,
    /// the file read carries the text, and both refuse what they must:
    /// a path that escapes the root, and a credential file by name.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_pane_routes_read_the_tree_and_the_file_and_refuse_the_rest() {
        let (token, port) = launch().await;
        let anonymous = ask(
            port,
            "GET /api/tree?path=src HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            None,
        );
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");

        let tree = ask(
            port,
            &format!("GET /api/tree?path=src&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(tree.starts_with("HTTP/1.1 200"), "{tree}");
        let document: serde_json::Value =
            serde_json::from_str(tree.split("\r\n\r\n").nth(1).expect("a body past the head"))
                .expect("a JSON document");
        assert_eq!(document["type"], "tree");
        let entries = document["entries"].as_array().expect("the entries");
        assert!(
            entries
                .iter()
                .any(|entry| entry["name"] == "gui" && entry["kind"] == "dir"),
            "{document}"
        );

        // The file read carries the text and names its language.
        let file = ask(
            port,
            &format!(
                "GET /api/file?path=src%2Fgui%2Fassets%2Fapp.css&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(file.starts_with("HTTP/1.1 200"), "{file}");
        let document: serde_json::Value =
            serde_json::from_str(file.split("\r\n\r\n").nth(1).expect("a body past the head"))
                .expect("a JSON document");
        assert_eq!(document["type"], "file");
        assert_eq!(document["language"], "css");
        assert!(
            document["text"]
                .as_str()
                .is_some_and(|text| text.contains("--dsw-alias-page")),
            "{document}"
        );

        // A path that escapes the root, and a credential file, are both
        // refused by name — the browser can ask for a file now, which is
        // exactly where one would leak.
        let escaped = ask(
            port,
            &format!(
                "GET /api/file?path=..%2F..%2Fetc%2Fpasswd&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        let document: serde_json::Value = serde_json::from_str(
            escaped
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body past the head"),
        )
        .expect("a JSON document");
        assert_eq!(document["refused"][0], "outside the working directory");
        let dotenv = ask(
            port,
            &format!("GET /api/file?path=.env&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(
            dotenv.contains("a credential file is never served"),
            "{dotenv}"
        );
        // A path with a space rides the query percent-encoded, and the
        // decoder has to bring it back.
        let spaced = ask(
            port,
            &format!(
                "GET /api/file?path=docs%2Fplans%2F2026-10-16-gui-revamp.md&token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"
            ),
            None,
        );
        assert!(spaced.starts_with("HTTP/1.1 200"), "{spaced}");
    }

    /// A home with no logs and no declared graph names both, rather than
    /// drawing an empty tree and an empty graph.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_home_with_nothing_delegated_names_it_rather_than_drawing_an_empty_graph() {
        let (token, port) = launch().await;
        let reply = get(port, "/api/delegation", &token);
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let body = reply
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body past the head");
        let document: serde_json::Value = serde_json::from_str(body).expect("a JSON document");
        assert!(document["live"].is_null(), "{document}");
        assert!(document["graph"].is_null(), "{document}");
        let refused = document["refused"].as_array().expect("the refusals");
        assert!(
            refused.iter().any(|reason| reason
                .as_str()
                .is_some_and(|reason| reason.contains("holds no declared graph"))),
            "{document}"
        );
        assert!(
            refused.iter().any(|reason| reason
                .as_str()
                .is_some_and(|reason| reason.contains("has no readable session log"))),
            "{document}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_task_crosses_the_run_route_and_lands_in_the_log() {
        let (token, port) = launch().await;
        let anonymous = post(port, "/api/run", "wrong", r#"{"task":"say hello"}"#);
        assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
        let refused = post(port, "/api/run", &token, r#"{"nope":"say hello"}"#);
        assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");
        let empty = post(port, "/api/run", &token, r#"{"task":"   "}"#);
        assert!(empty.starts_with("HTTP/1.1 409"), "{empty}");
        // The detached bridge has no agent thread behind it, so the run
        // is refused — but the owner's message is already in the log,
        // which is the honest order the browser shows.
        let accepted = post(port, "/api/run", &token, r#"{"task":"say hello"}"#);
        assert!(accepted.starts_with("HTTP/1.1 409"), "{accepted}");
        let state = get(port, "/api/state", &token);
        assert!(state.contains("\"type\":\"message\""), "{state}");
        assert!(state.contains("\"text\":\"say hello\""), "{state}");
        assert!(state.contains("\"type\":\"failed\""), "{state}");
    }

    /// The seat round-trips: the bridge stands a seat, the browser
    /// answers it, and the answer crosses. A browser that answers when
    /// nothing is waiting gets a conflict, and a word that is not a
    /// decision is refused with the seat still standing.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_answer_crosses_the_approve_route_and_a_bad_one_does_not() {
        let (token, port) = launch().await;
        let conflict = post(port, "/api/approve", &token, r#"{"decision":"once"}"#);
        assert!(conflict.starts_with("HTTP/1.1 409"), "{conflict}");
        let malformed = post(port, "/api/approve", &token, r#"{"word":"once"}"#);
        assert!(malformed.starts_with("HTTP/1.1 400"), "{malformed}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn no_payload_carries_a_credential() {
        let (token, port) = launch().await;
        let page = get(port, "/", &token);
        let css = get(port, "/app.css", &token);
        let js = get(port, "/app.js", &token);
        let stream = ask(
            port,
            &format!("GET /events?token={token} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            Some("heartbeat"),
        );
        for payload in [&page, &css, &js, &stream, INDEX, STYLES, SCRIPT] {
            for secret in ["sk-", "DEEPSEEK_API_KEY", "Authorization", "Bearer"] {
                assert!(
                    !payload.contains(secret),
                    "a payload names {secret}: {payload}"
                );
            }
        }
        // The token rides the Set-Cookie header and never the body: the
        // page's own markup holds no capability.
        let (head, body) = page.split_once("\r\n\r\n").expect("a head and a body");
        assert!(head.contains(&format!("fleet_gui={token}")), "{head}");
        assert!(!body.contains(&token), "{body}");
    }
}
