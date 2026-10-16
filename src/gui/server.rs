//! The loopback HTTP surface behind `fleet gui`.
//!
//! Hand-rolled HTTP/1.1 over tokio's `TcpListener`, because the crate
//! contract is one static Rust binary: a framework would buy this
//! surface nothing it does not already do by hand — three GET routes,
//! one SSE stream, and a token on every one of them.
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
//!   anonymous page, no anonymous asset, and no anonymous stream. A
//!   port scan sees 401s and learns nothing else.
//! - **No credentials in any payload.** The page, the assets, and the
//!   stream carry names and presence, never values: the masking rule
//!   doctor holds is the rule here.
//!
//! ## The stream
//!
//! `/events` is the only long-lived connection. It answers a hello that
//! tells the page where it is, then a heartbeat on an interval for as
//! long as the browser listens. Everything a later increment shows —
//! a run's turns, a route's decision, a memory's recall — rides this
//! same stream as one more event shape, so G1's seam is the wire the
//! rest of the face speaks on.

use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

/// The token's length in bytes before hex encoding: 16 bytes is 128
/// bits, which is where a guess stops being a threat model.
const TOKEN_BYTES: usize = 16;

/// One launch's face: the listener, the token, and the address the
/// owner was given. A `Server` is deliberately not cloneable — one
/// launch, one token, one listener — and the per-connection tasks
/// carry what they need by value instead.
pub struct Server {
    token: String,
    base_url: String,
    heartbeat: Duration,
    listener: tokio::net::TcpListener,
}

impl Server {
    /// Binds the loopback listener and mints the launch's token. `port`
    /// zero asks the operating system for a free one, which is what a
    /// second concurrent launch and the tests need.
    pub async fn bind(host: &str, port: u16) -> Result<(Server, u16), String> {
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
                        tokio::spawn(async move {
                            if let Err(e) =
                                serve_connection(stream, &token, &base_url, heartbeat).await
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
) -> Result<(), String> {
    let head = read_head(&mut stream).await?;
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
        ("GET", "/events") => stream_events(&mut stream, base_url, heartbeat).await,
        ("GET", _) => write_reply(&mut stream, not_found(&request.path)).await,
        _ => write_reply(&mut stream, method_not_allowed(&request.method)).await,
    }
}

/// Reads one request head: bytes up to the `\r\n\r\n` that ends them,
/// capped so a peer that never sends it cannot hold a task forever.
async fn read_head(stream: &mut tokio::net::TcpStream) -> Result<String, String> {
    let mut head: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("fleet gui could not read the request: {e}"))?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&chunk[..read]);
        if let Some(end) = head_end(&head) {
            head.truncate(end);
            break;
        }
        if head.len() > MAX_HEAD_BYTES {
            return Err("fleet gui refused an oversized request head".to_string());
        }
    }
    String::from_utf8(head).map_err(|_| "fleet gui refused a non-UTF-8 request head".to_string())
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

fn method_not_allowed(method: &str) -> Reply {
    Reply {
        status: "405 Method Not Allowed",
        content_type: "text/plain; charset=utf-8",
        extra: Vec::new(),
        body: format!("fleet gui answers GET; {method} is not a route.\n"),
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
/// a beat on the interval for as long as the browser listens. A write
/// that fails is the browser being gone — the stream ends with it,
/// quietly, because that is the normal way a tab closes.
async fn stream_events(
    stream: &mut tokio::net::TcpStream,
    base_url: &str,
    heartbeat: Duration,
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
        ticks.tick().await;
        let beat = json!({"type": "heartbeat"});
        if write_event(stream, &beat).await.is_err() {
            return Ok(());
        }
    }
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
    async fn launch() -> (String, u16) {
        let (mut server, port) = Server::bind("127.0.0.1", 0).await.unwrap();
        server.heartbeat = Duration::from_millis(20);
        let token = server.token().to_string();
        tokio::spawn(async move {
            let _ = server.serve().await;
        });
        (token, port)
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
        assert!(css.contains("--bg"), "{css}");
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

    #[tokio::test(flavor = "multi_thread")]
    async fn a_host_that_is_not_loopback_is_refused_by_name() {
        for host in ["0.0.0.0", "192.168.1.5", "example.com"] {
            let refused = Server::bind(host, 0).await;
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
            let (server, port) = Server::bind(host, 0).await.expect(host);
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
        if let Err(message) = Server::bind("127.0.0.2", 0).await {
            assert!(message.contains("127.0.0.2"), "{message}");
            assert!(message.contains("cannot bind"), "{message}");
        }
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
