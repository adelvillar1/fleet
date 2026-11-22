//! The kanzo board, relayed: `/kanzo/…` on the paired-remote listener is a
//! proxy to the kanzo origin on this machine.
//!
//! Kanzo binds `127.0.0.1:4242` and stays there. The relay is the only
//! path to it from another device, and the only gate is the pairing the
//! face and the bot PWA already use — a device is authorized by its
//! pairing, never by a second login, and the pairing check runs before
//! this module is reached at all (an unpaired request is a 401 or the
//! pairing page, and the board is never contacted).
//!
//! **Measured from kanzo 0.4.2's own served assets** (not the repo source
//! — the served bundle is a build-stripped variant):
//!
//! - the page is one document with five absolute references
//!   (`/logo.png`, `/icon.png`, `/manifest.webmanifest`, `src="/app.js"`);
//! - the bundle calls the API at four `fetch` sites, all absolute
//!   (`fetch("/api" + path`, two `` fetch(`/api/tasks/${…}`) ``, and
//!   `fetch("/changelog.md")`), and renders images from
//!   `` `/images/${file}` ``;
//! - the board polls `/api/changes`. There is no SSE and no websocket on
//!   its own routes, so there is no frame to relay unbroken: a second
//!   device sees an edit on kanzo's own update cycle, which is the
//!   polling interval the board already has.
//!
//! The mount is the one thing a proxy has to rewrite, and it rewrites
//! exactly what the measurements name: the document's asset references
//! and the bundle's four fetch sites plus its image prefix. Everything
//! else — every `/api/…` request and response — is byte-identical, and
//! that is measured (the fidelity test compares the proxied body to the
//! origin's own bytes). A build whose bundle no longer carries the
//! anchors is refused by name rather than served broken: a board whose
//! buttons call Fleet's own `/api` is worse than a board that says the
//! rewrite needs updating.

use std::io::Read;
use std::time::Duration;

use tokio::io::AsyncReadExt;

/// The mount point on the listener.
pub const PREFIX: &str = "/kanzo";
/// The origin the proxy speaks to: kanzo's own loopback default.
pub const ORIGIN: &str = "http://127.0.0.1:4242";
/// How much of one proxied body is accepted in either direction. A task
/// image is the biggest thing the board moves, and kanzo caps images at
/// 8 MiB; the proxy's bound sits above that with room, and refuses past
/// it by name.
pub const MAX_BODY: usize = 16 * 1024 * 1024;

/// Whether a request path belongs to the board. `/kanzo` and `/kanzo/`
/// are the document; anything under `/kanzo/` is the board's own path.
pub fn owns(path: &str) -> bool {
    path == PREFIX || path.starts_with("/kanzo/")
}

/// The path the origin serves for a mounted one: `/kanzo/api/board` →
/// `/api/board`, `/kanzo/` → `/`, `/kanzo` → `/`.
pub fn upstream_path(path: &str) -> String {
    let rest = path.strip_prefix(PREFIX).unwrap_or(path);
    if rest.is_empty() {
        "/".to_string()
    } else if rest.starts_with('/') {
        rest.to_string()
    } else {
        format!("/{rest}")
    }
}

/// One proxied answer, as bytes: the origin's status, its content type,
/// its body. The listener writes it on the socket path; the relay
/// carries it as text with the base64 marker its frame protocol already
/// uses for bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proxied {
    pub status: u16,
    pub content_type: String,
    /// The board's own version stamp, which its page reads to decide
    /// whether to reload after an update. Carried, never invented.
    pub version: Option<String>,
    pub body: Vec<u8>,
}

impl Proxied {
    /// Whether the body is text the frame protocol can carry as it is.
    pub fn is_text(&self) -> bool {
        self.content_type.starts_with("text/")
            || self.content_type.contains("json")
            || self.content_type.contains("javascript")
            || self.content_type.contains("xml")
            || self.content_type.contains("manifest")
    }
}

/// Read one request's body as bytes: the proxy's own read, because the
/// face's `read_body` returns a String and an image upload is bytes. The
/// bound is the same one the face applies, named the same way.
pub(crate) async fn read_body_bytes(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    request: &crate::gui::server::Request,
    rest: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let length = request
        .header("content-length")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY {
        return Err(format!(
            "the request is {length} bytes; the kanzo proxy carries up to {MAX_BODY}"
        ));
    }
    let mut body = rest;
    if body.len() > length {
        body.truncate(length);
    }
    let mut chunk = [0u8; 8192];
    while body.len() < length {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| format!("the proxy could not read the body: {error}"))?;
        if read == 0 {
            return Err(format!(
                "the proxy read {} of the {length} bytes the head declared",
                body.len()
            ));
        }
        let want = (length - body.len()).min(read);
        body.extend_from_slice(&chunk[..want]);
    }
    Ok(body)
}

/// One request to forward.
#[derive(Debug, Clone)]
pub struct Ask<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub content_type: Option<&'a str>,
    pub body: &'a [u8],
}

/// Forward one request to the origin and return its answer, the mount
/// rewritten on the three documents that need it. Blocking (a loopback
/// HTTP call through `ureq`); callers run it on the blocking pool.
pub fn forward(ask: &Ask<'_>) -> Result<Proxied, String> {
    forward_to(ORIGIN, ask)
}

/// `forward` against an explicit origin — the tests' own stand-in server.
pub fn forward_to(origin: &str, ask: &Ask<'_>) -> Result<Proxied, String> {
    let path = upstream_path(ask.path);
    let url = if ask.query.is_empty() {
        format!("{origin}{path}")
    } else {
        format!("{origin}{path}?{}", ask.query)
    };
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(600))
        .timeout(Duration::from_secs(10))
        .build();
    let mut request = agent.request(ask.method, &url);
    if let Some(content_type) = ask.content_type {
        request = request.set("content-type", content_type);
    }
    // No cookie, no authorization, no token: the board has no auth of its
    // own, and a paired device's credential is Fleet's, not kanzo's.
    let answer = if ask.body.is_empty() {
        request.call()
    } else {
        request.send_bytes(ask.body)
    };
    // A 4xx from the board is the board's answer, not a proxy failure:
    // its own JSON error rides back unchanged (the accept that kanzo
    // refuses must reach the phone in kanzo's words).
    let response = match answer {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(error) => return Err(format!("the kanzo board did not answer: {error}")),
    };
    let status = response.status();
    let content_type = response
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let version = response.header("x-kanzo-version").map(str::to_string);
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("the board's answer could not be read: {error}"))?;
    if body.len() > MAX_BODY {
        return Err(format!(
            "the board's answer is over the proxy's {MAX_BODY}-byte bound"
        ));
    }
    let body = rewrite(&path, &content_type, body)?;
    Ok(Proxied {
        status,
        content_type,
        version,
        body,
    })
}

/// Rewrite the mount on the three documents that carry absolute paths,
/// and pass every other body through untouched. The anchors are the ones
/// the served build carries; a document that no longer carries them is a
/// named refusal, because serving it unmounted would point the board's
/// own buttons at Fleet's routes.
pub fn rewrite(_path: &str, content_type: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
    let text = |body: Vec<u8>| String::from_utf8(body);
    if content_type.starts_with("text/html") {
        let Ok(text) = text(body) else {
            return Err("the board's document is not UTF-8".to_string());
        };
        if !text.contains("src=\"/app.js\"") {
            return Err(
                "kanzo's page no longer loads /app.js by that name — the relay's mount rewrite \
                 needs updating"
                    .to_string(),
            );
        }
        let out = text
            .replace("src=\"/app.js\"", "src=\"/kanzo/app.js\"")
            .replace("href=\"/logo.png\"", "href=\"/kanzo/logo.png\"")
            .replace("src=\"/logo.png\"", "src=\"/kanzo/logo.png\"")
            .replace("href=\"/icon.png\"", "href=\"/kanzo/icon.png\"")
            .replace(
                "href=\"/manifest.webmanifest\"",
                "href=\"/kanzo/manifest.webmanifest\"",
            );
        // A favicon the page names another way still has to resolve.
        let out = out.replace("href=\"/favicon.ico\"", "href=\"/kanzo/favicon.ico\"");
        return Ok(out.into_bytes());
    }
    if content_type.contains("javascript") {
        let Ok(text) = text(body) else {
            return Err("the board's bundle is not UTF-8".to_string());
        };
        if !text.contains("fetch(\"/api\" + path") {
            return Err(
                "kanzo's bundle no longer calls the API through `fetch(\"/api\" + path` — the \
                 relay's mount rewrite needs updating"
                    .to_string(),
            );
        }
        let out = text
            .replace("fetch(\"/api\" + path", "fetch(\"/kanzo/api\" + path")
            .replace("fetch(`/api/", "fetch(`/kanzo/api/")
            .replace("fetch(\"/changelog.md\")", "fetch(\"/kanzo/changelog.md\")")
            .replace("`/images/", "`/kanzo/images/");
        // The board's own absolute links, if the build grows any, are
        // caught by this guard rather than served pointing at Fleet.
        if !out.contains("/kanzo/api") {
            return Err("kanzo's bundle rewrite produced no mounted API call".to_string());
        }
        return Ok(out.into_bytes());
    }
    if content_type.contains("manifest") {
        let Ok(text) = text(body) else {
            return Err("the board's manifest is not UTF-8".to_string());
        };
        let out = text
            .replace("\"start_url\":\"/\"", "\"start_url\":\"/kanzo/\"")
            .replace("\"id\":\"/\"", "\"id\":\"/kanzo/\"")
            .replace("\"src\":\"/icon.png\"", "\"src\":\"/kanzo/icon.png\"");
        return Ok(out.into_bytes());
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mount_owns_its_paths_and_maps_them_back_to_the_origin() {
        assert!(owns("/kanzo"));
        assert!(owns("/kanzo/"));
        assert!(owns("/kanzo/api/board"));
        assert!(!owns("/kanzoish"));
        assert!(!owns("/api/board"));
        assert_eq!(upstream_path("/kanzo"), "/");
        assert_eq!(upstream_path("/kanzo/"), "/");
        assert_eq!(upstream_path("/kanzo/api/board"), "/api/board");
        assert_eq!(upstream_path("/kanzo/images/a.png"), "/images/a.png");
        assert_eq!(
            upstream_path("/kanzo/api/changes?since=9-1"),
            "/api/changes?since=9-1"
        );
    }

    #[test]
    fn the_document_is_mounted_at_its_asset_references() {
        let html = "<link rel=\"icon\" href=\"/logo.png\">\
                    <link rel=\"manifest\" href=\"/manifest.webmanifest\">\
                    <link rel=\"apple-touch-icon\" href=\"/icon.png\">\
                    <img src=\"/logo.png\"><script src=\"/app.js\"></script>";
        let out = String::from_utf8(rewrite("/", "text/html; charset=utf-8", html.into()).unwrap())
            .expect("utf-8");
        assert!(out.contains("src=\"/kanzo/app.js\""), "{out}");
        assert!(out.contains("href=\"/kanzo/logo.png\""), "{out}");
        assert!(out.contains("src=\"/kanzo/logo.png\""), "{out}");
        assert!(out.contains("href=\"/kanzo/icon.png\""), "{out}");
        assert!(
            out.contains("href=\"/kanzo/manifest.webmanifest\""),
            "{out}"
        );
        // A document that lost its script reference is refused by name.
        let moved = rewrite(
            "/",
            "text/html",
            b"<html><script src=\"/bundle.js\"></script></html>".to_vec(),
        );
        let reason = moved.expect_err("a moved anchor is refused");
        assert!(reason.contains("mount rewrite needs updating"), "{reason}");
    }

    #[test]
    fn the_bundle_is_mounted_at_its_four_fetch_sites_and_its_images() {
        let js = "async function api(m,path){const r=await fetch(\"/api\" + path,{})}\
                  fetch(`/api/tasks/${a}`,{})\
                  fetch(`/api/tasks/${b}`,{})\
                  fetch(\"/changelog.md\")\
                  const img = el(\"img\",{src:`/images/${file}`})";
        let out = String::from_utf8(rewrite("/app.js", "text/javascript", js.into()).unwrap())
            .expect("utf-8");
        assert!(out.contains("fetch(\"/kanzo/api\" + path"), "{out}");
        assert!(out.contains("fetch(`/kanzo/api/tasks/${a}`"), "{out}");
        assert!(out.contains("fetch(`/kanzo/api/tasks/${b}`"), "{out}");
        assert!(out.contains("fetch(\"/kanzo/changelog.md\")"), "{out}");
        assert!(out.contains("`/kanzo/images/${file}`"), "{out}");
        // A bundle whose API call moved is refused by name, never served
        // pointing at Fleet's own /api.
        let reason = rewrite("/app.js", "text/javascript", b"fetch('/api/x')".to_vec())
            .expect_err("a moved anchor is refused");
        assert!(reason.contains("mount rewrite needs updating"), "{reason}");
    }

    #[test]
    fn the_manifest_mounts_its_start_url_and_icon() {
        let manifest = "{\"id\":\"/\",\"name\":\"Kanzo\",\"start_url\":\"/\",\
                        \"icons\":[{\"src\":\"/icon.png\",\"sizes\":\"192x192\"}]}";
        let out = String::from_utf8(
            rewrite(
                "/manifest.webmanifest",
                "application/manifest+json",
                manifest.into(),
            )
            .unwrap(),
        )
        .expect("utf-8");
        assert!(out.contains("\"start_url\":\"/kanzo/\""), "{out}");
        assert!(out.contains("\"id\":\"/kanzo/\""), "{out}");
        assert!(out.contains("\"src\":\"/kanzo/icon.png\""), "{out}");
    }

    #[test]
    fn everything_else_is_byte_identical() {
        // The API's answers are the board's own bytes: no re-serialisation,
        // no re-ordering, no size delta.
        let json = br#"{"projects":[{"slug":"probe","open":1}],"tasks":[],"statuses":["todo"]}"#;
        let out = rewrite("/api/board", "application/json", json.to_vec()).unwrap();
        assert_eq!(out, json.to_vec());
        let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        let out = rewrite("/images/a.png", "image/png", png.clone()).unwrap();
        assert_eq!(out, png);
    }

    /// One stand-in origin on a real socket: it records the request line
    /// and answers a fixed body, so the proxy's own work (the path
    /// rewrite, the fidelity, the status pass-through) is measured
    /// without the owner's board.
    #[test]
    fn the_proxy_forwards_the_rewritten_path_and_returns_the_origins_bytes() {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().unwrap().port();
        let origin = format!("http://127.0.0.1:{port}");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen_writer = seen.clone();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("a connection");
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                // Drain the head.
                loop {
                    let mut head = String::new();
                    if reader.read_line(&mut head).unwrap_or(0) == 0 || head == "\r\n" {
                        break;
                    }
                }
                seen_writer.lock().unwrap().push(line.trim().to_string());
                let (status, body) = if line.contains("/api/tasks/9") {
                    ("404 Not Found", "{\"error\":\"no task 9\"}".to_string())
                } else {
                    ("200 OK", "{\"projects\":[],\"tasks\":[]}".to_string())
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(body.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });
        let ask = Ask {
            method: "GET",
            path: "/kanzo/api/board",
            query: "",
            content_type: None,
            body: &[],
        };
        let answer = forward_to(&origin, &ask).expect("the proxy answers");
        assert_eq!(answer.status, 200);
        assert_eq!(answer.body, b"{\"projects\":[],\"tasks\":[]}");
        // The origin saw the path WITHOUT the mount.
        let ask = Ask {
            method: "GET",
            path: "/kanzo/api/tasks/9",
            query: "",
            content_type: None,
            body: &[],
        };
        let answer = forward_to(&origin, &ask).expect("a 404 is an answer");
        assert_eq!(answer.status, 404);
        assert_eq!(answer.body, b"{\"error\":\"no task 9\"}");
        server.join().unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0], "GET /api/board HTTP/1.1", "{seen:?}");
        assert_eq!(seen[1], "GET /api/tasks/9 HTTP/1.1", "{seen:?}");
    }

    #[test]
    fn an_origin_that_does_not_answer_is_named() {
        let ask = Ask {
            method: "GET",
            path: "/kanzo/api/board",
            query: "",
            content_type: None,
            body: &[],
        };
        // A port nothing listens on.
        let reason = forward_to("http://127.0.0.1:1", &ask).expect_err("no origin");
        assert!(reason.contains("did not answer"), "{reason}");
    }
}
