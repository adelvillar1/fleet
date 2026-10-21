//! Nace Perception through sys1 — the lane that holds the credential.
//!
//! Fleet does not read `DREX_API_KEY` and does not run the vendor's HTTP
//! flow itself. sys1 is the provider lane: it loads its own provider env,
//! owns the Nace credential, and exposes `sys1_perceive_document`, which
//! runs the grant → upload → parse → poll sequence and returns the
//! vendor's Markdown with its page/unit counts and truncation disclosure.
//! This module is the fleet side of that call plus the cache: a document
//! attached twice is one billed parse.
//!
//! The lane call carries a longer budget than the judge's classify asks —
//! a document may take minutes — and the vendor's own job budget is the
//! outer bound.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::mcp::{McpServer, ServerSpec, Timeouts};

/// The tool sys1 exposes for a document parse.
pub const TOOL: &str = "sys1_perceive_document";

/// The largest file this path will hand to the lane.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// The one-call budget: a document parse polls the vendor's job, and a
/// slow document is a long call rather than a failure.
pub const CALL_BUDGET: Duration = Duration::from_secs(300);

/// One successful hosted parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub parser: String,
    pub markdown: String,
    pub pages: Option<u64>,
    pub units: Option<u64>,
    pub job_id: String,
    /// `Some((original, retained))` when the vendor returned a preview.
    pub truncated: Option<(u64, u64)>,
}

/// Parse one document through sys1, cache included. `home` is the fleet
/// home; the cache is `<home>/perception-cache`.
pub async fn parse_via_lane(
    path: &Path,
    name: &str,
    home: &Path,
    spec: ServerSpec,
) -> Result<Parsed, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("{} could not be read: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(format!(
            "{} is larger than the {} MB attachment bound",
            path.display(),
            MAX_BYTES / (1024 * 1024)
        ));
    }
    let hash = content_hash(&bytes);
    let cache = home.join("perception-cache");
    if let Some(cached) = read_cache(&cache, &hash) {
        return Ok(cached);
    }
    let timeouts = Timeouts {
        call: CALL_BUDGET,
        ..Timeouts::default()
    };
    let server = McpServer::spawn_with(&spec, timeouts)
        .map_err(|error| format!("the sys1 lane could not start: {error}"))?;
    if let Err(error) = server.initialize().await {
        let _ = server.close().await;
        return Err(format!("the sys1 lane did not initialize: {error}"));
    }
    let call = server
        .call_tool(
            TOOL,
            json!({ "path": path.to_string_lossy(), "name": name }),
        )
        .await;
    let _ = server.close().await;
    let result = call.map_err(|error| format!("the sys1 document tool failed: {error}"))?;
    let text = result.text();
    let value: Value = serde_json::from_str(text.trim())
        .map_err(|error| format!("the sys1 document tool answered with no JSON: {error}"))?;
    if !value.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let code = value
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("refused");
        let reason = value
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("the document tool refused the file");
        return Err(format!(
            "Perception refused the document ({code}): {reason}"
        ));
    }
    let parsed = Parsed {
        parser: value
            .get("parser")
            .and_then(Value::as_str)
            .unwrap_or("nace")
            .to_string(),
        markdown: value
            .get("markdown")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pages: value.get("pages").and_then(Value::as_u64),
        units: value.get("units").and_then(Value::as_u64),
        job_id: value
            .get("job_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        truncated: value
            .get("truncated")
            .and_then(Value::as_object)
            .map(|row| {
                (
                    row.get("original").and_then(Value::as_u64).unwrap_or(0),
                    row.get("retained").and_then(Value::as_u64).unwrap_or(0),
                )
            }),
    };
    write_cache(&cache, &hash, &parsed);
    Ok(parsed)
}

/// FNV-1a over the file's bytes: the cache key.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn cache_path(cache: &Path, hash: &str) -> PathBuf {
    cache.join(format!("{hash}.json"))
}

fn read_cache(cache: &Path, hash: &str) -> Option<Parsed> {
    let text = std::fs::read_to_string(cache_path(cache, hash)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    Some(Parsed {
        parser: text_value(&value, "parser")?,
        markdown: text_value(&value, "markdown")?,
        pages: value.get("pages").and_then(Value::as_u64),
        units: value.get("units").and_then(Value::as_u64),
        job_id: text_value(&value, "job_id")?,
        truncated: value
            .get("truncated")
            .and_then(Value::as_array)
            .and_then(|rows| Some((rows.first()?.as_u64()?, rows.get(1)?.as_u64()?))),
    })
}

fn text_value(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn write_cache(cache: &Path, hash: &str, parsed: &Parsed) {
    let payload = json!({
        "parser": parsed.parser,
        "markdown": parsed.markdown,
        "pages": parsed.pages,
        "units": parsed.units,
        "job_id": parsed.job_id,
        "truncated": parsed.truncated.map(|(original, retained)| json!([original, retained])),
    });
    if std::fs::create_dir_all(cache).is_err() {
        return;
    }
    let path = cache_path(cache, hash);
    let temp = path.with_extension("json.tmp");
    if std::fs::write(&temp, payload.to_string()).is_ok() {
        let _ = std::fs::rename(&temp, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Request {
        path: String,
        body: String,
    }

    /// A scripted Perception for the sys1 lane: grant, upload, parse,
    /// then one finished job. The lane's own Python calls this.
    fn served(
        handler: impl Fn(&Request) -> (u16, String) + Send + 'static,
        requests: usize,
    ) -> (u16, Arc<Mutex<Vec<Request>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
        let port = listener.local_addr().expect("the port").port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let handle = std::thread::spawn(move || {
            for _ in 0..requests {
                let (mut stream, _) = listener.accept().expect("a connection");
                let request = read_request(&mut stream);
                recorded.lock().expect("the requests").push(Request {
                    path: request.0,
                    body: request.1,
                });
                let (status, body) = handler(
                    recorded
                        .lock()
                        .expect("the requests")
                        .last()
                        .expect("a request"),
                );
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (port, seen, handle)
    }

    fn read_request(stream: &mut TcpStream) -> (String, String) {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 2048];
        let head = loop {
            let read = stream.read(&mut chunk).unwrap_or(0);
            if read == 0 {
                break buffer.len().saturating_sub(4);
            }
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                let text = String::from_utf8_lossy(&buffer[..at]).to_string();
                let length = text
                    .lines()
                    .filter_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap_or(0))
                    })
                    .next()
                    .unwrap_or(0);
                if buffer.len() >= at + 4 + length {
                    break at;
                }
            }
        };
        let text = String::from_utf8_lossy(&buffer).to_string();
        let path = text
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("")
            .to_string();
        let body = String::from_utf8_lossy(&buffer[head + 4..]).to_string();
        (path, body)
    }

    fn spec(port: u16) -> ServerSpec {
        let mut spec = crate::lanes::spec_for("sys1").expect("the sys1 lane resolves");
        spec.env.push((
            "SYS1_PERCEPTION_BASE".into(),
            format!("http://127.0.0.1:{port}"),
        ));
        spec.env
            .push(("DREX_API_KEY".into(), "nace_sk_test".into()));
        spec.env
            .push(("SYS1_ENV_FILE".into(), "/nonexistent".into()));
        spec
    }

    fn home() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-perception-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("the home");
        dir
    }

    fn fixture() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("services/documents/fixtures/simple.csv")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_sys1_tool_runs_the_perception_flow() {
        let (port, seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"UPLOAD","token":"tok"}"#.to_string(),
                    );
                }
                if request.path == "/UPLOAD" || request.path == "UPLOAD" {
                    return (
                        200,
                        r#"{"result":{"file":{"file_id":"file1"}}}"#.to_string(),
                    );
                }
                if request.path.contains("/v1/documents/parse") {
                    return (202, r#"{"status":"queued","job_id":"job1"}"#.to_string());
                }
                (200, r#"{"status":"succeeded","result":{"document":{"markdown":"| item | count |\n| --- | --- |\n| alpha | 1 |","page_count":2},"units":2,"disclosures":[{"code":"content_truncated","original_size":1000,"retained_size":65536}]}}"#.to_string())
            },
            4,
        );
        let parsed = parse_via_lane(&fixture(), "simple.csv", &home(), spec(port))
            .await
            .expect("the parse");
        let _ = handle.join();
        assert_eq!(parsed.parser, "nace");
        assert!(parsed.markdown.contains("alpha"), "{parsed:?}");
        assert_eq!(parsed.pages, Some(2));
        assert_eq!(parsed.units, Some(2));
        assert_eq!(parsed.job_id, "job1");
        assert_eq!(parsed.truncated, Some((1000, 65536)));
        let requests = seen.lock().expect("the requests");
        assert_eq!(requests.len(), 4, "{requests:?}");
        assert!(
            requests[2].body.contains("markdown") && requests[2].body.contains("parse_mode"),
            "{}",
            requests[2].body
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_second_attach_is_a_cache_hit() {
        let (port, seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"U","token":"t"}"#.to_string(),
                    );
                }
                if request.path == "/U" || request.path == "U" {
                    return (200, r#"{"file_id":"f"}"#.to_string());
                }
                (200, r#"{"result":{"document":{"markdown":"cached text","page_count":1},"units":1}}"#.to_string())
            },
            3,
        );
        let home = home();
        let first = parse_via_lane(&fixture(), "simple.csv", &home, spec(port))
            .await
            .expect("the parse");
        let second = parse_via_lane(&fixture(), "simple.csv", &home, spec(port))
            .await
            .expect("the cached parse");
        let _ = handle.join();
        assert_eq!(first, second);
        assert_eq!(seen.lock().expect("the requests").len(), 3);
        assert!(read_cache(
            &home.join("perception-cache"),
            &content_hash(&std::fs::read(fixture()).expect("the fixture"))
        )
        .is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_refused_job_names_the_vendors_code() {
        let (port, _seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"U2","token":"t"}"#.to_string(),
                    );
                }
                if request.path == "/U2" || request.path == "U2" {
                    return (200, r#"{"file_id":"f"}"#.to_string());
                }
                if request.path.contains("/parse") {
                    return (202, r#"{"job_id":"job2"}"#.to_string());
                }
                (200, r#"{"status":"failed","error":{"code":"corrupt_file","message":"the pdf is torn"}}"#.to_string())
            },
            4,
        );
        let error = parse_via_lane(&fixture(), "simple.csv", &home(), spec(port))
            .await
            .expect_err("the failure");
        let _ = handle.join();
        assert!(error.contains("corrupt_file"), "{error}");
        assert!(error.contains("job2"), "{error}");
    }

    #[test]
    fn a_lane_without_the_key_answers_no_key() {
        // The lane's tool, called directly with no provider env and no
        // env file: the refusal is the lane's own.
        let python =
            std::env::var("FLEET_DOCUMENTS_PYTHON").unwrap_or_else(|_| "python3".to_string());
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("services/sys1/src");
        let output = std::process::Command::new(python)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin")
            .env("PYTHONPATH", &root)
            .env("SYS1_ENV_FILE", "/nonexistent")
            .arg("-c")
            .arg("import json,sys; from sys1.config import load_config; from sys1.perception import parse_document; print(json.dumps(parse_document(sys.argv[1], load_config())))")
            .arg("services/documents/fixtures/simple.csv")
            .output()
            .expect("the lane tool runs");
        let value: Value = serde_json::from_slice(&output.stdout).expect("the tool's JSON");
        assert_eq!(value["ok"], false);
        assert_eq!(value["code"], "no_key");
    }
}
