//! The second-brain tool plugin: a separate program, attached to rather
//! than reimplemented.
//!
//! Second brain is the owner's own dashboard over a growing project
//! landscape. It owns a repository (`~/Projects/second-brain`, also
//! reachable as the `~/.second-brain` symlink), a build pipeline, and a
//! local server on 4175 whose `/api/status` names the pipeline's current
//! job and steps. Fleet's boards already read the state files the tool
//! derives; what was missing is the tool's control plane: which jobs
//! exist, whether one is running, and what happened when it ran.
//!
//! This module is an adapter, not a second implementation. It carries
//! the tool's vocabulary — the nine jobs its `PIPE_DEFS` declares — and
//! forwards a refresh as the tool's own `POST /api/run`, with the
//! `X-Tower` header the tool uses as its cross-site guard. Fleet never
//! writes a state file and never decides what a job does.
//!
//! The bound is the machine: the address is loopback and the port is the
//! tool's, so there is no host to validate and no redirect to follow.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

/// The port the tool's own server listens on unless `SECOND_BRAIN_PORT`
/// says otherwise, the same value its `serve` argument defaults to.
pub const DEFAULT_PORT: u16 = 4175;

/// The pipeline jobs the tool's `PIPE_DEFS` declares, in its order. The
/// list is the tool's vocabulary, copied here so the face can offer a
/// job without inventing one: an unknown name is refused before it
/// reaches the tool, and the tool refuses it again if the list ever
/// drifts.
pub const JOBS: &[(&str, &str)] = &[
    ("all", "everything"),
    ("chats", "chats"),
    ("knowledge", "knowledge"),
    ("gates", "gates"),
    ("harness", "harness"),
    ("reports", "reports"),
    ("maturity", "maturity"),
    ("loops", "loops"),
    ("tower", "tower"),
    ("mirror", "the postgres mirror"),
];

/// The tool's status read is its heartbeat: short, because a tool that
/// has not answered in two seconds is not live for the face's purposes.
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
/// A control call answers before the job runs (the tool starts it on its
/// own thread), so this bound is for the answer, not the pipeline.
const ACTION_TIMEOUT: Duration = Duration::from_secs(5);
/// The status payload is a dozen steps and a short log tail; anything
/// larger is not the payload this adapter knows.
const MAX_BODY: u64 = 256 * 1024;

/// One tool plugin: where the tool lives and where it listens. The home
/// is the tool's repository, which is also where its derived state and
/// its built site live; the port is the server's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondBrain {
    home: PathBuf,
    port: u16,
}

/// The tool's `/api/status`, as this adapter reads it. Every field is
/// optional in the payload (a fresh server has no job yet), so the
/// defaults describe "idle and never run" rather than a failure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pipeline {
    pub running: bool,
    pub job: Option<String>,
    pub steps: Vec<Step>,
    /// The index of the step the tool is on, `-1` when idle.
    pub current: i64,
    /// The tool's own log tail, newest last.
    pub log: Vec<String>,
    pub started: Option<String>,
    pub finished: Option<String>,
    /// The failed step's reason, when the last run did not finish.
    pub error: Option<String>,
    /// The last run that completed, which can be older than `finished`.
    pub last_ok: Option<String>,
    /// What the derived state was built from, the tool's own stamp.
    pub built_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub status: String,
    pub ms: u64,
}

impl SecondBrain {
    pub fn new(home: PathBuf, port: u16) -> Self {
        Self { home, port }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn address(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Whether the tool's own entry point is in the home. A home can be
    /// resolved while the tool is not installed there (a stale
    /// `SECOND_BRAIN_HOME`, say), and that is a different fact from the
    /// server being down: one is "this is not the tool", the other is
    /// "the tool is not running".
    pub fn has_tool(&self) -> bool {
        self.home.join("control_tower.py").is_file()
    }

    /// The tool's heartbeat, as its own server reports it.
    pub fn status(&self) -> Result<Pipeline, String> {
        let url = format!("{}/api/status", self.address());
        let response = self
            .agent(STATUS_TIMEOUT)
            .get(&url)
            .call()
            .map_err(|error| self.refusal(error, &url))?;
        let body = read_body(response)?;
        parse_status(&body)
    }

    /// Start one of the tool's own pipeline jobs. The job name is
    /// validated against the tool's vocabulary here, so a typo is a
    /// sentence rather than a 400 from a program the owner is not
    /// looking at; the tool validates again, which is the authority.
    pub fn refresh(&self, job: &str) -> Result<Value, String> {
        let job = job.trim();
        if !JOBS.iter().any(|(name, _)| *name == job) {
            return Err(format!(
                "the second-brain tool runs these jobs: {}",
                job_names()
            ));
        }
        let url = format!("{}/api/run", self.address());
        let response = self
            .agent(ACTION_TIMEOUT)
            .post(&url)
            .set("X-Tower", "1")
            .send_json(json!({ "job": job }))
            .map_err(|error| self.refusal(error, &url))?;
        let body = read_body(response)?;
        serde_json::from_str(&body)
            .map_err(|error| format!("the second-brain tool's reply is not JSON: {error}"))
    }

    fn agent(&self, timeout: Duration) -> ureq::Agent {
        ureq::AgentBuilder::new().timeout(timeout).build()
    }

    /// The two failure shapes ureq reports, split the way fleet's other
    /// clients split them: an answered status carries the tool's own
    /// sentence, and a transport failure means the tool is not live.
    fn refusal(&self, error: ureq::Error, url: &str) -> String {
        match error {
            ureq::Error::Status(code, response) => {
                let body = response.into_string().unwrap_or_default();
                let body = body.trim();
                if body.is_empty() {
                    format!("the second-brain tool refused the request ({code})")
                } else {
                    format!("the second-brain tool refused the request ({code}): {body}")
                }
            }
            ureq::Error::Transport(error) => {
                format!("the second-brain tool did not answer at {url}: {error}")
            }
        }
    }
}

/// The job vocabulary as one sentence, for the refusal.
pub fn job_names() -> String {
    JOBS.iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_body(response: ureq::Response) -> Result<String, String> {
    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_BODY + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("the second-brain tool's reply could not be read: {error}"))?;
    if body.len() as u64 > MAX_BODY {
        return Err("the second-brain tool's reply was larger than its bound".to_string());
    }
    Ok(body)
}

/// Parse the tool's status. A payload without `running` is still the
/// tool's answer — an older server, or one whose state has not been
/// touched — so the missing fields read as idle rather than as a
/// refusal. Only a body that is not an object at all is refused.
pub fn parse_status(body: &str) -> Result<Pipeline, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| format!("the second-brain tool's status is not JSON: {error}"))?;
    if !value.is_object() {
        return Err("the second-brain tool's status is not an object".to_string());
    }
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    let steps = value
        .get("steps")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(Step {
                        label: row.get("label").and_then(Value::as_str)?.to_string(),
                        status: row
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("pending")
                            .to_string(),
                        ms: row.get("ms").and_then(Value::as_u64).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let log = value
        .get("log")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(Pipeline {
        running: value
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        job: text("job"),
        steps,
        current: value.get("current").and_then(Value::as_i64).unwrap_or(-1),
        log,
        started: text("started"),
        finished: text("finished"),
        error: text("error"),
        last_ok: text("last_ok"),
        built_at: text("built_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    fn tool(port: u16) -> SecondBrain {
        SecondBrain::new(PathBuf::from("/nonexistent"), port)
    }

    /// Read one whole request before answering. The body is read to its
    /// declared length rather than to whatever the first read returned:
    /// a reply sent while the client is still writing the body closes the
    /// socket under it, and the client reports a header error that has
    /// nothing to do with the code under test.
    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let read = std::io::Read::read(stream, &mut chunk).unwrap_or(0);
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
            let Some(head) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let text = String::from_utf8_lossy(&buffer[..head]).to_string();
            let length = text
                .lines()
                .filter_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap_or(0))
                })
                .next()
                .unwrap_or(0);
            if buffer.len() >= head + 4 + length {
                break;
            }
        }
        String::from_utf8_lossy(&buffer).to_string()
    }

    /// One request answered once, on an ephemeral port. The reply is the
    /// tool's contract shrunk to what this adapter reads; the request is
    /// returned so a test can assert what fleet actually sent.
    fn served(status: &str, body: &str) -> (u16, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
        let port = listener.local_addr().expect("the port").port();
        let status = status.to_string();
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("a read bound");
            let request = read_request(&mut stream);
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).expect("the reply");
            request
        });
        (port, handle)
    }

    #[test]
    fn the_status_names_the_running_job_and_its_steps() {
        let (port, handle) = served(
            "200 OK",
            r#"{"running":true,"job":"reports","steps":[
                 {"label":"derive reports","status":"done","ms":120},
                 {"label":"build dashboard","status":"running","ms":0}],
               "current":1,"log":["$ tower_reports.py derive"],
               "started":"2026-10-20T20:00:00","finished":null,"error":null,
               "last_ok":"2026-10-19T09:00:00","built_at":"2026-10-09 01:52"}"#,
        );
        let read = tool(port).status().expect("the status lands");
        handle.join().expect("the server thread");
        assert!(read.running);
        assert_eq!(read.job.as_deref(), Some("reports"));
        assert_eq!(read.steps.len(), 2);
        assert_eq!(read.steps[1].status, "running");
        assert_eq!(read.current, 1);
        assert_eq!(read.log[0], "$ tower_reports.py derive");
        assert_eq!(read.built_at.as_deref(), Some("2026-10-09 01:52"));
    }

    #[test]
    fn an_idle_tool_reads_as_live_with_its_last_build() {
        let (port, handle) = served(
            "200 OK",
            r#"{"running":false,"job":null,"steps":[],"current":-1,"log":[],
                "started":null,"finished":"2026-10-20T20:01:00","error":null,
                "last_ok":"2026-10-20T20:01:00","built_at":"2026-10-20 20:01"}"#,
        );
        let read = tool(port).status().expect("the status lands");
        handle.join().expect("the server thread");
        assert!(!read.running);
        assert_eq!(read.job, None);
        assert_eq!(read.last_ok.as_deref(), Some("2026-10-20T20:01:00"));
    }

    #[test]
    fn an_unknown_job_is_refused_with_the_tools_own_vocabulary() {
        let refusal = tool(1).refresh("everything").expect_err("not a job");
        assert!(
            refusal.contains("the second-brain tool runs these jobs"),
            "{refusal}"
        );
        assert!(refusal.contains("reports"), "{refusal}");
        assert!(refusal.contains("mirror"), "{refusal}");
    }

    #[test]
    fn a_refresh_posts_the_job_and_the_tools_guard_header() {
        let (port, handle) = served(
            "200 OK",
            r#"{"ok":true,"job":"reports","steps":["derive reports"]}"#,
        );
        let reply = tool(port).refresh("reports").expect("the action lands");
        let request = handle.join().expect("the server thread");
        assert_eq!(reply["job"], "reports");
        assert!(request.starts_with("POST /api/run "), "{request}");
        assert!(request.to_lowercase().contains("x-tower: 1"), "{request}");
        assert!(request.contains(r#"{"job":"reports"}"#), "{request}");
    }

    #[test]
    fn a_busy_tool_names_the_job_it_is_already_running() {
        let (port, handle) = served("409 Conflict", "pipeline busy: knowledge");
        let refusal = tool(port).refresh("all").expect_err("busy");
        handle.join().expect("the server thread");
        assert!(refusal.contains("409"), "{refusal}");
        assert!(refusal.contains("pipeline busy: knowledge"), "{refusal}");
    }

    #[test]
    fn a_tool_that_is_not_listening_names_its_address() {
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
            listener.local_addr().expect("the port").port()
        };
        let refusal = tool(port).status().expect_err("no server");
        assert!(refusal.contains(&format!("127.0.0.1:{port}")), "{refusal}");
        assert!(refusal.contains("did not answer"), "{refusal}");
    }

    #[test]
    fn the_home_reports_the_tool_only_when_its_entry_point_is_there() {
        let root = std::env::temp_dir().join(format!(
            "fleet-plugin-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("the scratch home");
        let plugin = SecondBrain::new(root.clone(), DEFAULT_PORT);
        assert!(!plugin.has_tool());
        std::fs::write(root.join("control_tower.py"), "# the tool").expect("the entry point");
        assert!(plugin.has_tool());
        let _ = std::fs::remove_dir_all(&root);
    }
}
