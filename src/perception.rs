//! Nace Perception: the hosted parser for the documents local extraction
//! cannot read.
//!
//! The flow is the vendor's: ask for an upload grant, post the file to the
//! grant's URL, create a Parse job over the workspace file, and poll the
//! job until it is terminal. The API is server-side only (the `/v1`
//! routes carry no CORS header), the key never reaches the browser, and
//! every failure is a sentence carrying the vendor's own code.
//!
//! The result is cached by content hash: attaching the same document
//! twice makes one billed call, and the cache is the reason a retry after
//! a UI reload costs nothing.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// The vendor's API host. `NACE_BASE_URL` overrides it for a test or a
/// proxy.
pub const DEFAULT_BASE: &str = "https://console.nace.ai";

/// The largest file this path will upload. The vendor's own page bound is
/// 500 pages; this is the byte bound of one attachment.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

const GRANT_TIMEOUT: Duration = Duration::from_secs(30);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const PARSE_TIMEOUT: Duration = Duration::from_secs(90);
const STATUS_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a job may run before the attachment is refused by name.
const JOB_BUDGET: Duration = Duration::from_secs(300);
const MAX_BODY: u64 = 4 * 1024 * 1024;

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

/// The key this machine has, in the order fleet reads settings.
pub fn key(dotenv: &HashMap<String, String>) -> Option<String> {
    pick_key(|name| crate::home::resolve_setting(name, dotenv))
}

fn pick_key(lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
    ["NACE_API_KEY", "DREX_API_KEY"]
        .iter()
        .find_map(|name| lookup(name))
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

/// The API base: `NACE_BASE_URL` when set, else the vendor's host.
pub fn base(dotenv: &HashMap<String, String>) -> String {
    crate::home::resolve_setting("NACE_BASE_URL", dotenv)
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE.to_string())
}

pub struct Client {
    base: String,
    key: String,
}

impl Client {
    pub fn new(base: String, key: String) -> Client {
        Client { base, key }
    }

    /// Read one document through the vendor, cache included. `home` is
    /// the fleet home; the cache is `<home>/perception-cache`.
    pub fn parse(&self, path: &Path, name: &str, home: &Path) -> Result<Parsed, String> {
        self.parse_within(path, name, home, JOB_BUDGET)
    }

    /// The same read with the job budget named, for a test and for a
    /// caller that wants a shorter wait.
    pub fn parse_within(
        &self,
        path: &Path,
        name: &str,
        home: &Path,
        budget: Duration,
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
        let grant = self.upload_grant(name)?;
        let file_id = self.upload(&grant, name, &bytes)?;
        let start = self.start_parse(&grant.workspace_id, &file_id)?;
        let (result, job_id) = match start {
            ParseStart::Done(result) => {
                let id = start_job_id(&result);
                (result, id)
            }
            ParseStart::Queued(job_id) => {
                let result = self.wait(&job_id, budget)?;
                (result, job_id)
            }
        };
        let parsed = finish(result, job_id)?;
        write_cache(&cache, &hash, &parsed);
        Ok(parsed)
    }

    fn upload_grant(&self, name: &str) -> Result<Grant, String> {
        let url = format!("{}/v1/documents/upload-grants", self.base);
        let response = self
            .agent(GRANT_TIMEOUT)
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.key))
            .send_json(json!({ "path": name }))
            .map_err(|error| refusal(error, "an upload grant"))?;
        let body = read_body(response)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| format!("the upload grant is not JSON: {error}"))?;
        let grant = value.get("result").unwrap_or(&value);
        let workspace_id = text(grant, "workspace_id")
            .ok_or_else(|| "the upload grant names no workspace".to_string())?;
        let upload_url = text(grant, "upload_url")
            .ok_or_else(|| "the upload grant names no upload URL".to_string())?;
        let token =
            text(grant, "token").ok_or_else(|| "the upload grant carries no token".to_string())?;
        Ok(Grant {
            workspace_id,
            upload_url,
            token,
        })
    }

    fn upload(&self, grant: &Grant, name: &str, bytes: &[u8]) -> Result<String, String> {
        let boundary = format!("fleet-{}", &content_hash(bytes)[..16]);
        let mut body: Vec<u8> = Vec::new();
        let metadata = json!({ "path": name }).to_string();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"; type=\"application/json\"\r\n\r\n{metadata}\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let upload_url = if grant.upload_url.starts_with("http") {
            grant.upload_url.clone()
        } else {
            format!("{}/{}", self.base, grant.upload_url.trim_start_matches('/'))
        };
        let response = self
            .agent(UPLOAD_TIMEOUT)
            .post(&upload_url)
            .set("X-Upload-Token", &grant.token)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .send_bytes(&body)
            .map_err(|error| refusal(error, "the upload"))?;
        let reply = read_body(response)?;
        let value: Value = serde_json::from_str(&reply)
            .map_err(|error| format!("the upload reply is not JSON: {error}"))?;
        let result = value.get("result").unwrap_or(&value);
        for pointer in ["file/file_id", "file_id"] {
            if let Some(found) = result
                .pointer(&format!("/{pointer}"))
                .and_then(Value::as_str)
            {
                return Ok(found.to_string());
            }
        }
        Err("the upload reply names no file id".to_string())
    }

    fn start_parse(&self, workspace_id: &str, file_id: &str) -> Result<ParseStart, String> {
        let url = format!("{}/v1/documents/parse?wait_seconds=60", self.base);
        let response = self
            .agent(PARSE_TIMEOUT)
            .post(&url)
            .set("Authorization", &format!("Bearer {}", self.key))
            .send_json(json!({
                "source": {
                    "type": "workspace_file",
                    "workspace_id": workspace_id,
                    "file_id": file_id,
                },
                "output": { "formats": ["markdown"] },
                "parse_mode": "low",
            }));
        let response = match response {
            Ok(response) => response,
            Err(error) => return Err(refusal(error, "the parse job")),
        };
        let status = response.status();
        let body = read_body(response)?;
        if status == 200 {
            let value: Value = serde_json::from_str(&body)
                .map_err(|error| format!("the parse reply is not JSON: {error}"))?;
            return Ok(ParseStart::Done(
                value.get("result").cloned().unwrap_or(value),
            ));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| format!("the parse reply is not JSON: {error}"))?;
        let job_id =
            text(&value, "job_id").ok_or_else(|| "the parse reply names no job id".to_string())?;
        Ok(ParseStart::Queued(job_id))
    }

    fn wait(&self, job_id: &str, budget: Duration) -> Result<Value, String> {
        let deadline = Instant::now() + budget;
        let mut pause = Duration::from_secs(1);
        loop {
            let url = format!("{}/v1/documents/jobs/{job_id}", self.base);
            let response = self
                .agent(STATUS_TIMEOUT)
                .get(&url)
                .set("Authorization", &format!("Bearer {}", self.key))
                .call()
                .map_err(|error| refusal(error, "the job"))?;
            let body = read_body(response)?;
            let value: Value = serde_json::from_str(&body)
                .map_err(|error| format!("the job reply is not JSON: {error}"))?;
            let status = text(&value, "status").unwrap_or_default();
            match status.as_str() {
                "succeeded" => {
                    let result = value
                        .get("result")
                        .cloned()
                        .ok_or_else(|| format!("job {job_id} succeeded with no result"))?;
                    if let Some(state) = text(&value, "result_state") {
                        if state == "expired" {
                            return Err(format!("job {job_id} succeeded but its result expired"));
                        }
                    }
                    return Ok(result);
                }
                "failed" | "cancelled" => {
                    let code = value
                        .pointer("/error/code")
                        .and_then(Value::as_str)
                        .unwrap_or(status.as_str());
                    let message = value
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("the job did not finish");
                    return Err(format!("job {job_id} {status} ({code}): {message}"));
                }
                _ => {}
            }
            if Instant::now() + pause > deadline {
                return Err(format!(
                    "job {job_id} did not finish within {}s; it may still complete in the vendor's console",
                    budget.as_secs()
                ));
            }
            std::thread::sleep(pause);
            pause = (pause * 2).min(Duration::from_secs(10));
        }
    }

    fn agent(&self, timeout: Duration) -> ureq::Agent {
        ureq::AgentBuilder::new().timeout(timeout).build()
    }
}

#[derive(Debug, Clone)]
struct Grant {
    workspace_id: String,
    upload_url: String,
    token: String,
}

enum ParseStart {
    Done(Value),
    Queued(String),
}

/// The job id a result carries, when the vendor includes it.
fn start_job_id(result: &Value) -> String {
    text(result, "job_id").unwrap_or_default()
}

fn finish(result: Value, job_id: String) -> Result<Parsed, String> {
    let result = result.get("result").cloned().unwrap_or(result);
    let document = result.get("document").cloned().unwrap_or(Value::Null);
    let markdown = document
        .get("markdown")
        .and_then(Value::as_str)
        .or_else(|| document.get("text").and_then(Value::as_str))
        .or_else(|| result.get("markdown").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    if markdown.trim().is_empty() {
        return Err("the parse returned no text".to_string());
    }
    let pages = document.get("page_count").and_then(Value::as_u64);
    let units = result.get("units").and_then(Value::as_u64);
    let truncated = result
        .get("disclosures")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("code").and_then(Value::as_str) == Some("content_truncated"))
                .map(|row| {
                    (
                        row.get("original_size")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        row.get("retained_size")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                    )
                })
        });
    let job_id = if job_id.is_empty() {
        text(&result, "job_id").unwrap_or_default()
    } else {
        job_id
    };
    Ok(Parsed {
        parser: "nace".to_string(),
        markdown,
        pages,
        units,
        job_id,
        truncated,
    })
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn read_body(response: ureq::Response) -> Result<String, String> {
    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_BODY + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("the perception reply could not be read: {error}"))?;
    if body.len() as u64 > MAX_BODY {
        return Err("the perception reply was larger than its bound".to_string());
    }
    Ok(body)
}

fn refusal(error: ureq::Error, what: &str) -> String {
    match error {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let vendor = value
                .pointer("/error/code")
                .or_else(|| value.get("code"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let message = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or(body.trim());
            if vendor.is_empty() {
                format!("the perception service refused {what} ({code}): {message}")
            } else {
                format!("the perception service refused {what} ({code} {vendor}): {message}")
            }
        }
        ureq::Error::Transport(error) => format!(
            "the perception service did not answer {what}: {error} (check the network and NACE_BASE_URL)"
        ),
    }
}

/// FNV-1a over the file's bytes: the cache key and the idempotency seed.
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
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Request {
        method: String,
        path: String,
        body: String,
    }

    /// A one-connection-per-request scripted server. `reply` receives the
    /// request and returns `(status, body)`; every request is recorded so
    /// a test can assert the calls the client made.
    fn served(
        handler: impl Fn(&Request) -> (u16, String) + Send + 'static,
        requests: usize,
    ) -> (u16, Arc<Mutex<Vec<Request>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
        let port = listener.local_addr().expect("the port").port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        let handle = std::thread::spawn(move || {
            let handler = Some(handler);
            for _ in 0..requests {
                let (mut stream, _) = listener.accept().expect("a connection");
                let request = read_request(&mut stream);
                recorded.lock().expect("the requests").push(Request {
                    method: request.0,
                    path: request.1,
                    body: request.2,
                });
                let (status, body) = handler.as_ref().expect("the handler")(
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

    fn read_request(stream: &mut TcpStream) -> (String, String, String) {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 2048];
        let (head, _length) = loop {
            let read = stream.read(&mut chunk).unwrap_or(0);
            if read == 0 {
                break (buffer.len(), 0);
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
                    break (at, length);
                }
            }
        };
        let text = String::from_utf8_lossy(&buffer).to_string();
        let mut lines = text.lines();
        let first = lines.next().unwrap_or("").to_string();
        let mut parts = first.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let body = String::from_utf8_lossy(&buffer[head + 4..]).to_string();
        (method, path, body)
    }

    fn client(port: u16) -> Client {
        Client::new(
            format!("http://127.0.0.1:{port}"),
            "nace_sk_test".to_string(),
        )
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

    #[test]
    fn the_flow_grants_uploads_parses_and_polls() {
        let (port, seen, handle) = served(
            |request| {
                if request.path.starts_with("/v1/documents/upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws1","upload_url":"UPLOAD","token":"tok"}"#.to_string(),
                    );
                }
                if request.path == "UPLOAD" || request.path == "/UPLOAD" {
                    return (
                        200,
                        r#"{"result":{"file":{"file_id":"file1"}}}"#.to_string(),
                    );
                }
                if request.path.starts_with("/v1/documents/parse") {
                    return (202, r#"{"status":"queued","job_id":"job1"}"#.to_string());
                }
                if request.path == "/v1/documents/jobs/job1" {
                    return (200, r#"{"status":"succeeded","result":{"document":{"markdown":"| item | count |\n| --- | --- |\n| alpha | 1 |","page_count":1},"units":1,"disclosures":[]}}"#.to_string());
                }
                (
                    404,
                    r#"{"error":{"code":"not_found","message":"no"}}"#.to_string(),
                )
            },
            4,
        );
        // The grant's upload URL is relative in this script; the client
        // posts it as given, so the server sees UPLOAD.
        let mut client = client(port);
        client.base = format!("http://127.0.0.1:{port}");
        let parsed = client
            .parse_within(&fixture(), "simple.csv", &home(), Duration::from_secs(10))
            .expect("the parse");
        let _ = handle.join();
        assert_eq!(parsed.parser, "nace");
        assert!(parsed.markdown.contains("alpha"), "{parsed:?}");
        assert_eq!(parsed.pages, Some(1));
        assert_eq!(parsed.units, Some(1));
        let requests = seen.lock().expect("the requests");
        assert_eq!(requests.len(), 4, "{requests:?}");
        assert_eq!(requests[0].method, "POST");
        assert!(requests[0].path.contains("upload-grants"));
        assert!(
            requests[1].body.contains("name=\"file\""),
            "{:?}",
            requests[1].body
        );
        assert!(requests[1].body.contains("simple.csv"));
        assert!(requests[2].path.contains("/v1/documents/parse"));
        assert!(
            requests[2].body.contains("\"formats\":[\"markdown\"]"),
            "{}",
            requests[2].body
        );
        assert_eq!(requests[3].path, "/v1/documents/jobs/job1");
    }

    #[test]
    fn the_second_attach_of_the_same_bytes_is_a_cache_hit() {
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
                if request.path.contains("/parse") {
                    return (200, r#"{"result":{"document":{"markdown":"cached text","page_count":2},"units":2}}"#.to_string());
                }
                (404, "{}".to_string())
            },
            3,
        );
        let home = home();
        let client = client(port);
        let first = client
            .parse_within(&fixture(), "simple.csv", &home, Duration::from_secs(10))
            .expect("the first parse");
        let second = client
            .parse_within(&fixture(), "simple.csv", &home, Duration::from_secs(10))
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

    #[test]
    fn a_truncated_preview_carries_the_sizes() {
        let result = json!({
            "document": {"markdown": "retained", "page_count": 96},
            "units": 96,
            "disclosures": [{"code":"content_truncated","original_size":1000,"retained_size":65536}]
        });
        let parsed = finish(result, "job9".to_string()).expect("a result");
        assert_eq!(parsed.truncated, Some((1000, 65536)));
        assert_eq!(parsed.pages, Some(96));
    }

    #[test]
    fn a_failed_job_names_the_vendors_code() {
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
        let client = client(port);
        let error = client
            .parse_within(&fixture(), "simple.csv", &home(), Duration::from_secs(5))
            .expect_err("the failure");
        let _ = handle.join();
        assert!(error.contains("corrupt_file"), "{error}");
        assert!(error.contains("job2"), "{error}");
    }

    #[test]
    fn a_job_that_never_finishes_times_out_by_name() {
        let (port, _seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"U3","token":"t"}"#.to_string(),
                    );
                }
                if request.path == "/U3" || request.path == "U3" {
                    return (200, r#"{"file_id":"f"}"#.to_string());
                }
                if request.path.contains("/parse") {
                    return (202, r#"{"job_id":"job3"}"#.to_string());
                }
                (
                    200,
                    r#"{"status":"running","progress":{"units_done":1,"units_total":9}}"#
                        .to_string(),
                )
            },
            4,
        );
        let client = client(port);
        let error = client
            .parse_within(
                &fixture(),
                "simple.csv",
                &home(),
                Duration::from_millis(100),
            )
            .expect_err("the timeout");
        let _ = handle.join();
        assert!(error.contains("job3"), "{error}");
        assert!(error.contains("did not finish"), "{error}");
    }

    #[test]
    fn a_409_from_the_parse_names_the_status_and_the_vendors_code() {
        let (port, _seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"U5","token":"t"}"#.to_string(),
                    );
                }
                if request.path == "/U5" || request.path == "U5" {
                    return (200, r#"{"file_id":"f"}"#.to_string());
                }
                (409, r#"{"error":{"code":"content_not_ready","message":"the file is still preparing"}}"#.to_string())
            },
            3,
        );
        let client = client(port);
        let error = client
            .parse_within(&fixture(), "simple.csv", &home(), Duration::from_secs(5))
            .expect_err("the refusal");
        let _ = handle.join();
        assert!(error.contains("409"), "{error}");
        assert!(error.contains("content_not_ready"), "{error}");
    }

    #[test]
    fn an_unsupported_type_names_the_vendors_code() {
        let (port, _seen, handle) = served(
            |request| {
                if request.path.contains("upload-grants") {
                    return (
                        200,
                        r#"{"workspace_id":"ws","upload_url":"U4","token":"t"}"#.to_string(),
                    );
                }
                if request.path == "/U4" || request.path == "U4" {
                    return (200, r#"{"file_id":"f"}"#.to_string());
                }
                (422, r#"{"error":{"code":"unsupported_file_type","message":"no parser for that file"}}"#.to_string())
            },
            3,
        );
        let client = client(port);
        let error = client
            .parse_within(&fixture(), "simple.csv", &home(), Duration::from_secs(5))
            .expect_err("the refusal");
        let _ = handle.join();
        assert!(error.contains("422"), "{error}");
        assert!(error.contains("unsupported_file_type"), "{error}");
    }

    #[test]
    fn the_key_prefers_nace_and_falls_back_to_drex() {
        let map = |nace: Option<&str>, drex: Option<&str>| {
            let mut values = HashMap::new();
            if let Some(value) = nace {
                values.insert("NACE_API_KEY", value.to_string());
            }
            if let Some(value) = drex {
                values.insert("DREX_API_KEY", value.to_string());
            }
            values
        };
        let picked = pick_key(|name| map(Some("nace_sk_a"), Some("drex_b")).get(name).cloned());
        assert_eq!(picked.as_deref(), Some("nace_sk_a"));
        let fallback = pick_key(|name| map(None, Some("drex_b")).get(name).cloned());
        assert_eq!(fallback.as_deref(), Some("drex_b"));
        assert_eq!(pick_key(|_| None), None);
    }
}
