//! The eval corpus: a fixed, graded, deterministic suite of scenarios,
//! each a scripted mock-server run with asserted outcomes.
//!
//! Slice 13's centre. The corpus is the harness's model of its own
//! performance — the three families the plan names (session-corpus
//! replay, agent-continuation, conversation-fold) plus the everyday
//! shapes the crate already proves (a two-turn tool loop, a subagent
//! delegation, a memory recall) — graded pass/fail per scenario with the
//! failure named, run headless against the mock with no live model and
//! no network beyond loopback.
//!
//! Spec: the fork's `benchmarks/` tree (the scenario families' shapes)
//! and `@fleet/dsh-sdm1`'s telemetry loop (the turn-duration/outcome
//! scoring vocabulary) — read as references, never as dependencies (hard
//! rule 6). Ported: the fixed-corpus shape, the per-scenario grading,
//! and the score-as-a-stream-row idea. Deferred: the fork's median-timing
//! budgets and machine-profile scaling — a Rust unit that replays a
//! recorded session grades an *outcome* (does the log reconstruct, does
//! the request match), not a wall-clock median, so the corpus carries
//! verdicts and a score rather than latency budgets.
//!
//! ## Determinism
//!
//! Every scenario drives a **plain single-endpoint client** pointed at
//! the mock (`Client::new(mock.url(), "test-key")`), never the routed
//! client. There is no roster to walk and no live endpoint to reach, so
//! no run can spill onto the real API — the strongest form of KI-066's
//! smoke-safety, applied structurally rather than by a guard. The
//! [`MockApi`] harness still asserts the port file exists before a run
//! and treats an empty request log after a served turn as a live-run
//! alarm, the way the integration harness does.
//!
//! ## Failures are verdicts, never panics
//!
//! A driver touches the filesystem — a scratch dir, a session log,
//! the mock's script and port — and every one of those touches
//! faults into a **graded failure**: the scenario fails naming the
//! fault, and `fleet eval` exits on the verdict. A panic in a driver
//! is the KI-027 spawn/port pattern promoted into `fleet eval`'s
//! production path — exit 101, with the improve gate's refusal
//! misattributed to a "regressed" corpus — which is why the module's
//! contract is that a scenario turns *every* failure into a graded
//! outcome.
//!
//! ## The three consumers
//!
//! 1. `fleet eval` runs the whole corpus, prints house cards, and lands
//!    one `eval_corpus` row; `fleet eval --last` reads the last row back.
//! 2. The slice-11 scheduler runs the corpus on its own daily cadence
//!    (see [`crate::cron`]), its row landing in the stream like any run.
//! 3. A slice-12 **source** proposal's gate runs the corpus as a step
//!    (see [`crate::improve`]); a regression refuses the apply with the
//!    scenario named. The production gate reaches the corpus through a
//!    subprocess ([`subprocess_probe`]) that **fails closed** — it
//!    refuses unless the current executable is the freshly-built binary
//!    under the gate root's `target/`, and its child is bounded — so a
//!    source self-edit is never waved through by an eval step that did
//!    not run; a test drives the grading through the output seam.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use serde_json::json;

use crate::agent::{Agent, AgentError, AgentOptions, ApproveAll, Approver, RunReport, StopReason};
use crate::api::{Client, Message};
use crate::calib::{self, EvalCorpusRow, EvalScenarioRow, Row, Stream};
use crate::memory::{self, Memory};
use crate::session::{self, Record, Session};

// --- the mock harness -----------------------------------------------------

/// One scripted response: a whole document, or a list of raw SSE frames.
#[derive(Debug, Clone)]
struct ScriptedResponse {
    status: String,
    body: String,
    /// Raw `data: …` frame strings; present means "answer as a stream".
    sse: Vec<String>,
    delay_ms: u64,
}

fn stream(frames: Vec<String>) -> ScriptedResponse {
    ScriptedResponse { status: "200 OK".to_string(), body: String::new(), sse: frames, delay_ms: 0 }
}

fn content_delta(text: &str) -> String {
    format!("data: {}", json!({ "choices": [{ "delta": { "content": text } }] }))
}

fn tool_call_delta(index: usize, id: &str, name: &str, arguments: &str) -> String {
    format!(
        "data: {}",
        json!({ "choices": [{ "delta": { "tool_calls": [{ "index": index, "id": id,
            "type": "function", "function": { "name": name, "arguments": arguments } }] } }] })
    )
}

fn finish_frame(reason: &str) -> String {
    format!("data: {}", json!({ "choices": [{ "delta": {}, "finish_reason": reason }] }))
}

fn usage_frame(prompt: u64, completion: u64) -> String {
    format!(
        "data: {}",
        json!({ "choices": [], "usage": { "prompt_tokens": prompt,
            "completion_tokens": completion, "total_tokens": prompt + completion } })
    )
}

/// A complete streamed text answer with the accounting the fixtures use.
fn assistant_stream(content: &str) -> ScriptedResponse {
    stream(vec![
        content_delta(content),
        finish_frame("stop"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ])
}

/// A complete streamed answer requesting one tool call, in fragments.
fn tool_call_stream(id: &str, name: &str, arguments: &str) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        usage_frame(10, 5),
        "data: [DONE]".to_string(),
    ])
}

/// A tool-calling turn that reports `prompt` prompt tokens — the only way
/// a fold can be driven from the API's own count.
fn counting_turn(index: u32, prompt: u64) -> ScriptedResponse {
    tool_call_stream_counting(
        &format!("call-{index}"),
        "bash",
        &format!(r#"{{"command":"echo {index}"}}"#),
        prompt,
    )
}

fn tool_call_stream_counting(
    id: &str,
    name: &str,
    arguments: &str,
    prompt: u64,
) -> ScriptedResponse {
    let (head, tail) = arguments.split_at(arguments.len() / 2);
    stream(vec![
        tool_call_delta(0, id, name, head),
        tool_call_delta(0, "", "", tail),
        finish_frame("tool_calls"),
        usage_frame(prompt, 5),
        "data: [DONE]".to_string(),
    ])
}

/// A local chat-completions stand-in, served by the `mock-server` binary
/// in a child process. The binary's path is passed in: production
/// resolves it beside the fleet executable, a test resolves it from
/// `CARGO_BIN_EXE`. Cross-process by necessity — ureq's reads misbehave
/// when the client and an in-process loopback server share one process.
struct MockApi {
    child: Child,
    port: u16,
    scratch: PathBuf,
}

impl MockApi {
    /// Start the mock over `responses`. Every fault — an unwritable
    /// scratch dir, a script that will not write, a child that will
    /// not spawn, a port that never arrives — is a named error, which
    /// the calling scenario grades as its own failure: a scenario
    /// turns every failure into a graded outcome, never a panic.
    fn start(mock_bin: &Path, responses: Vec<ScriptedResponse>) -> Result<MockApi, String> {
        let scratch = scratch_dir("eval-mock")?;
        match start_over(&scratch, mock_bin, responses) {
            Ok(mock) => Ok(mock),
            Err(error) => {
                // The scratch dir is this call's own; a failed start
                // leaves nothing behind.
                std::fs::remove_dir_all(&scratch).ok();
                Err(error)
            }
        }
    }

    /// The base URL the plain client talks to.
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The recorded request bodies, parsed, in arrival order. Each
    /// recorded line is `{"head":…,"body":…}` where `body` is the request
    /// document as text, so it is parsed a second time into a value.
    fn bodies(&self) -> Vec<serde_json::Value> {
        let text =
            std::fs::read_to_string(self.scratch.join("requests.jsonl")).unwrap_or_default();
        text.lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let recorded: serde_json::Value = serde_json::from_str(line).ok()?;
                let body = recorded.get("body")?.as_str()?;
                serde_json::from_str(body).ok()
            })
            .collect()
    }

    /// The message roles one recorded request carried.
    fn roles(&self, index: usize) -> Vec<String> {
        self.bodies()
            .get(index)
            .and_then(|body| body.get("messages"))
            .and_then(|messages| messages.as_array())
            .map(|messages| {
                messages
                    .iter()
                    .filter_map(|message| {
                        message.get("role").and_then(|role| role.as_str()).map(str::to_string)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The text of one message in a recorded request.
    fn message_text(&self, index: usize, position: usize) -> Option<String> {
        self.bodies()
            .get(index)
            .and_then(|body| body.get("messages"))
            .and_then(|messages| messages.as_array())
            .and_then(|messages| messages.get(position))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .map(str::to_string)
    }

    /// The system prompt one recorded request carried.
    fn system(&self, index: usize) -> String {
        self.message_text(index, 0).unwrap_or_default()
    }
}

impl Drop for MockApi {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::fs::remove_dir_all(&self.scratch).ok();
    }
}

/// The mock start over an already-made scratch dir, so the caller
/// owns the cleanup. KI-066: the port file must exist and hold a real
/// port before a run — and a child that exits first, or a port that
/// never arrives, is a named error rather than a panic, so the
/// scenario grades it.
fn start_over(
    scratch: &Path,
    mock_bin: &Path,
    responses: Vec<ScriptedResponse>,
) -> Result<MockApi, String> {
    let script_path = scratch.join("responses.jsonl");
    let script = responses
        .iter()
        .map(|response| {
            json!({
                "status": response.status,
                "body": response.body,
                "sse": response.sse,
                "delay_ms": response.delay_ms,
            })
            .to_string()
        })
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    std::fs::write(&script_path, script)
        .map_err(|error| format!("the mock script could not be written: {error}"))?;
    let requests_path = scratch.join("requests.jsonl");
    let port_path = scratch.join("port");
    let mut child = Command::new(mock_bin)
        .arg(&script_path)
        .arg(&requests_path)
        .arg(&port_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("the mock-server child could not be spawned: {error}"))?;
    let port = wait_for_port(&port_path, &mut child)?;
    Ok(MockApi { child, port, scratch: scratch.to_path_buf() })
}

/// Poll for the child's port file; a child that exits first, or a
/// port that never arrives, fails with a named error — the KI-027
/// flaky-spawn shape graded rather than panicked.
fn wait_for_port(port_path: &Path, child: &mut Child) -> Result<u16, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(port_path) {
            if let Ok(port) = text.trim().parse() {
                return Ok(port);
            }
        }
        if child
            .try_wait()
            .map_err(|error| format!("the mock-server child could not be waited on: {error}"))?
            .is_some()
        {
            return Err("the mock-server child exited before writing its port".to_string());
        }
        if std::time::Instant::now() >= deadline {
            return Err("the mock-server port never arrived".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// The mock-server binary beside the fleet executable. `None` when it
/// cannot be resolved, which the caller names rather than guessing at.
pub fn mock_server_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) { "mock-server.exe" } else { "mock-server" };
    let path = exe.parent()?.join(name);
    path.exists().then_some(path)
}

// --- the vocabulary -------------------------------------------------------

/// One scenario's family. The three the plan names port the fork's
/// `benchmarks/` families; the three everyday shapes are the crate's own
/// proven behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioFamily {
    /// A recorded session replays and the outcomes assert: the log
    /// reconstructs, the requests match.
    SessionCorpus,
    /// A session resumes and continues coherently.
    AgentContinuation,
    /// A long session folds and the post-fold requests carry the summary.
    ConversationFold,
    /// The two-turn tool loop the crate has proven since slice 1.
    ToolLoop,
    /// A subagent delegation runs in its own session on the parent's
    /// model (slice 9).
    Subagent,
    /// A memory recall enters the turn's context (slice 8).
    Memory,
}

impl ScenarioFamily {
    /// The word the row and the card carry.
    pub fn label(self) -> &'static str {
        match self {
            ScenarioFamily::SessionCorpus => "session-corpus",
            ScenarioFamily::AgentContinuation => "agent-continuation",
            ScenarioFamily::ConversationFold => "conversation-fold",
            ScenarioFamily::ToolLoop => "tool-loop",
            ScenarioFamily::Subagent => "subagent",
            ScenarioFamily::Memory => "memory",
        }
    }
}

/// One scenario's graded verdict: its name, its family, whether it
/// passed, and the failure's name when it did not. The whole point of
/// the corpus is that a failure is *named*.
#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub scenario: String,
    pub family: String,
    pub passed: bool,
    pub failure: Option<String>,
}

impl ScenarioOutcome {
    fn pass(scenario: &str, family: ScenarioFamily) -> ScenarioOutcome {
        ScenarioOutcome {
            scenario: scenario.to_string(),
            family: family.label().to_string(),
            passed: true,
            failure: None,
        }
    }

    fn fail(scenario: &str, family: ScenarioFamily, reason: impl Into<String>) -> ScenarioOutcome {
        ScenarioOutcome {
            scenario: scenario.to_string(),
            family: family.label().to_string(),
            passed: false,
            failure: Some(reason.into()),
        }
    }
}

/// The graded verdict of a set of checks: pass when every one holds,
/// otherwise fail naming the first that did not.
fn grade(
    scenario: &'static str,
    family: ScenarioFamily,
    checks: &[(&str, bool)],
) -> ScenarioOutcome {
    for (label, ok) in checks {
        if !ok {
            return ScenarioOutcome::fail(scenario, family, *label);
        }
    }
    ScenarioOutcome::pass(scenario, family)
}

/// One scenario in the fixed corpus: a name, a family, and the run that
/// grades it against the mock. The run is a boxed future so a `Vec` of
/// scenarios can hold heterogeneous drivers; it takes the mock path by
/// value so the future is `'static`, and it is `!Send` because the agent
/// holds a `Box<dyn Approver>` — the corpus runs on one task.
type ScenarioFuture = Pin<Box<dyn Future<Output = ScenarioOutcome>>>;

pub struct Scenario {
    pub name: &'static str,
    pub family: ScenarioFamily,
    run: Box<dyn Fn(PathBuf) -> ScenarioFuture>,
}

impl Scenario {
    fn new(
        name: &'static str,
        family: ScenarioFamily,
        run: impl Fn(PathBuf) -> ScenarioFuture + 'static,
    ) -> Scenario {
        Scenario { name, family, run: Box::new(run) }
    }

    /// Grade this scenario against the mock at `mock_bin`.
    pub async fn run(&self, mock_bin: &Path) -> ScenarioOutcome {
        (self.run)(mock_bin.to_path_buf()).await
    }

    /// A scenario that always fails, naming its reason. The seam a test
    /// injects to prove the corpus grades a failure and names it, without
    /// spawning a mock.
    pub fn always_fails(
        name: &'static str,
        family: ScenarioFamily,
        reason: &'static str,
    ) -> Scenario {
        Scenario::new(name, family, move |_bin| {
            Box::pin(async move { ScenarioOutcome::fail(name, family, reason) })
        })
    }
}

/// The fixed corpus: the three fork families plus the three everyday
/// shapes, in a stable order. Adding a scenario is a code change here,
/// never a runtime file — that is what "fixed" means.
pub fn corpus() -> Vec<Scenario> {
    vec![
        Scenario::new("two-turn-tool-loop", ScenarioFamily::ToolLoop, |bin| {
            Box::pin(two_turn_tool_loop(bin))
        }),
        Scenario::new("session-corpus-replay", ScenarioFamily::SessionCorpus, |bin| {
            Box::pin(session_corpus_replay(bin))
        }),
        Scenario::new("agent-continuation", ScenarioFamily::AgentContinuation, |bin| {
            Box::pin(agent_continuation(bin))
        }),
        Scenario::new("conversation-fold", ScenarioFamily::ConversationFold, |bin| {
            Box::pin(conversation_fold(bin))
        }),
        Scenario::new("subagent-delegation", ScenarioFamily::Subagent, |bin| {
            Box::pin(subagent_delegation(bin))
        }),
        Scenario::new("memory-recall", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_recall(bin))
        }),
    ]
}

// --- the report -----------------------------------------------------------

/// One corpus run's aggregate: every scenario's verdict, the pass/fail
/// tally, and the score. Queryable, and the shape the improve gate reads
/// a regression from.
#[derive(Debug, Clone)]
pub struct EvalReport {
    pub scenarios: Vec<ScenarioOutcome>,
}

impl EvalReport {
    /// Grade a set of outcomes into a report.
    pub fn from(scenarios: Vec<ScenarioOutcome>) -> EvalReport {
        EvalReport { scenarios }
    }

    /// The scenarios that passed.
    pub fn passed(&self) -> usize {
        self.scenarios.iter().filter(|scenario| scenario.passed).count()
    }

    /// The scenarios that failed, named, in corpus order.
    pub fn failed(&self) -> Vec<&ScenarioOutcome> {
        self.scenarios.iter().filter(|scenario| !scenario.passed).collect()
    }

    /// The aggregate score: `passed / total`, or 1.0 for an empty corpus.
    pub fn score(&self) -> f64 {
        if self.scenarios.is_empty() {
            return 1.0;
        }
        self.passed() as f64 / self.scenarios.len() as f64
    }

    /// Whether the corpus is green: every scenario passed.
    pub fn green(&self) -> bool {
        self.failed().is_empty()
    }

    /// The names of the scenarios that regressed, in corpus order.
    pub fn regressions(&self) -> Vec<String> {
        self.failed().into_iter().map(|scenario| scenario.scenario.clone()).collect()
    }

    /// The one `eval_corpus` row this run lands, stamped `ts`.
    pub fn to_row(&self, ts: String) -> EvalCorpusRow {
        EvalCorpusRow {
            ts,
            passed: self.passed(),
            failed: self.failed().len(),
            score: self.score(),
            scenarios: self
                .scenarios
                .iter()
                .map(|scenario| EvalScenarioRow {
                    scenario: scenario.scenario.clone(),
                    family: scenario.family.clone(),
                    passed: scenario.passed,
                    failure: scenario.failure.clone(),
                })
                .collect(),
        }
    }
}

// --- the runner -----------------------------------------------------------

/// Run the whole corpus headless against the mock, land one `eval_corpus`
/// row in the stream at `root`, and return the graded report. The run is
/// deterministic: same corpus, same scripts, same verdicts.
pub async fn run(root: &Path, mock_bin: &Path) -> EvalReport {
    let scenarios = corpus();
    let mut outcomes = Vec::with_capacity(scenarios.len());
    for scenario in &scenarios {
        outcomes.push(scenario.run(mock_bin).await);
    }
    let report = EvalReport::from(outcomes);
    let stream = Stream::open(root);
    stream.record(Row::EvalCorpus(report.to_row(calib::stamp())));
    // The last handle's drop drains the queue before this returns, so the
    // row is on disk when a caller reads it back.
    drop(stream);
    report
}

/// The most recent `eval_corpus` row in today's stream, or `None` when
/// the corpus has not run today. The backing read for `fleet eval --last`.
pub fn last_row(root: &Path) -> Option<calib::ReadRow> {
    let rows = calib::read_day(root, &calib::today(), calib::READ_CAP);
    rows.into_iter().rev().find(|row| row.op == calib::OP_EVAL_CORPUS)
}

// --- the improve-gate probe ----------------------------------------------

/// The eval-corpus gate probe the slice-12 gate runs for a source
/// target: `Ok(())` when the corpus is green, `Err` naming the
/// regressing scenario(s) when it is not. `Send + Sync` so it can ride
/// the `Apply` context and be shared; a test supplies a stand-in.
pub type EvalProbe = Arc<dyn Fn() -> Result<(), String> + Send + Sync>;

/// The bound the probe's `fleet eval` subprocess runs under: the
/// corpus is six deterministic mock-server runs, so a child that has
/// not finished within this many seconds is a hung probe — killed
/// and named, never a gate that hangs with it.
pub const PROBE_TIMEOUT_SECS: u64 = 120;

/// The same bound as a [`std::time::Duration`], the form the runner
/// takes.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(PROBE_TIMEOUT_SECS);

/// The grace the child gets to exit once its output pipes have
/// closed: the corpus prints its cards and exits, so anything past
/// this is a child that will not end, killed with the same named
/// refusal.
const PROBE_EXIT_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// The production probe over the gate root: run `fleet eval` as a
/// **subprocess** so the gate grades the freshly-built binary (the
/// four cargo steps, `build` among them, have already rebuilt it by
/// the time the eval step runs), and refuse on a regression naming
/// the failing scenarios. A subprocess rather than an in-process run
/// is also what keeps the gate off the async runtime it is called
/// from — the gate is a sync context, and a `block_on` there would
/// panic (KI-021).
///
/// The probe **fails closed**. When the current executable does not
/// resolve, or resolves outside the gate root's `target/` directory —
/// an installed or stale binary, which would grade the wrong build
/// and read as a false green — the probe refuses naming why, and the
/// gate records the `eval_corpus` step failed: a source self-edit is
/// never waved through by an eval step that did not run. The
/// subprocess is bounded by [`PROBE_TIMEOUT_SECS`], so a hung child
/// is a named failure rather than a hung gate.
pub fn subprocess_probe(gate_root: &Path) -> EvalProbe {
    let target_root = gate_root.join("target");
    // The binary the probe will grade: the current executable, but
    // only when it lives under the gate root's target/ directory —
    // the freshly-built binary the four cargo steps produced.
    let resolved = std::env::current_exe().ok().filter(|exe| under_target(exe, &target_root));
    Arc::new(move || {
        let Some(exe) = resolved.clone() else {
            return Err(format!(
                "the eval corpus could not run: the fleet executable did not resolve under {}; the corpus must grade the freshly-built binary",
                target_root.display()
            ));
        };
        grade_output(&run_eval_subprocess(&exe, PROBE_TIMEOUT)?)
    })
}

/// Whether `exe` lives under `target_root`, compared on canonical
/// paths: macOS resolves `/var` to `/private/var` (and `/tmp` to
/// `/private/tmp`), so a lexical prefix check would refuse the
/// freshly-built binary on a symlinked path.
fn under_target(exe: &Path, target_root: &Path) -> bool {
    let root = target_root.canonicalize().unwrap_or_else(|_| target_root.to_path_buf());
    let exe = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    exe.starts_with(&root)
}

/// Grade one `fleet eval` subprocess's output: `Ok(())` when the
/// corpus is green, `Err` naming the regressing scenario(s) when it
/// is not. `fleet eval` prints one `regressions: <a>, <b>` line when
/// it is red; a missing one falls back to a named refusal.
fn grade_output(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let named = text
        .lines()
        .find_map(|line| line.split_once("regressions: "))
        .map(|(_, names)| strip_ansi(names).trim().to_string())
        .filter(|names| !names.is_empty());
    match named {
        Some(names) => Err(format!("the eval corpus regressed: {names}")),
        None => {
            Err("the eval corpus regressed (run `fleet eval` for the detail)".to_string())
        }
    }
}

/// Run `fleet eval` as a bounded subprocess and collect its output. A
/// reader thread drains both pipes, so a child that fills its pipe
/// buffer cannot deadlock the wait; the parent keeps the child so it
/// can kill it when the bound expires. The corpus is six
/// deterministic mock-server runs — a child that has not finished
/// within `timeout` is a hung probe, not a hung gate.
fn run_eval_subprocess(
    exe: &Path,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let mut child = Command::new(exe)
        .arg("eval")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("the eval corpus could not run: {error}"))?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<std::io::Result<(Vec<u8>, Vec<u8>)>>();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let drained = (|| {
            if let Some(pipe) = stdout.as_mut() {
                pipe.read_to_end(&mut out)?;
            }
            if let Some(pipe) = stderr.as_mut() {
                pipe.read_to_end(&mut err)?;
            }
            Ok(())
        })();
        let _ = tx.send(drained.map(|()| (out, err)));
    });
    let (stdout, stderr) = match rx.recv_timeout(timeout) {
        Ok(Ok(pipes)) => pipes,
        Ok(Err(error)) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("the eval corpus output could not be read: {error}"));
        }
        Err(_) => {
            // The bound expired: kill the child — which closes its
            // pipes and ends the reader thread — and refuse on its
            // behalf.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "the eval corpus did not finish within {} seconds",
                timeout.as_secs()
            ));
        }
    };
    // The child exits once its pipes close; a short grace poll keeps
    // even the pathological case inside a bound.
    let deadline = std::time::Instant::now() + PROBE_EXIT_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(std::process::Output { status, stdout, stderr }),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the eval corpus did not exit after its output closed".to_string());
            }
            Err(error) => {
                return Err(format!("the eval corpus could not be waited on: {error}"));
            }
        }
    }
}

/// Strip ANSI escape sequences from text: the surface's cards carry
/// color, and a name greped out of a card must not carry the trailing
/// reset into the gate step's detail.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            // An escape sequence: ESC, then bytes up to the final byte
            // — an ASCII letter for every sequence the palette emits
            // (SGR's `m`) and the only kind `fleet eval` prints.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

// --- the surface ----------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, OK, RESET, TEXT, WARN};

const CARD_WIDTH: usize = 72;

/// Render one run's report as house cards: a header with the aggregate
/// score, one card per scenario, and a footer naming any regression the
/// subprocess probe greps for. Never a plain-text table.
pub fn render(report: &EvalReport) -> String {
    let mut out = String::new();
    let verdict = if report.green() {
        format!("{OK}green{RESET}")
    } else {
        format!("{WARN}regression{RESET}")
    };
    out.push_str(&format!(
        "{BOLD}fleet eval{RESET} {DIM}\u{2014} {} scenario{} \u{b7} score {:.2} \u{b7} {verdict}{RESET}\n\n",
        report.scenarios.len(),
        plural(report.scenarios.len()),
        report.score(),
    ));
    for scenario in &report.scenarios {
        out.push_str(&scenario_card(scenario));
    }
    let regressions = report.regressions();
    if !regressions.is_empty() {
        // The one line the improve-gate subprocess probe greps for.
        out.push_str(&format!(
            "{WARN}regressions: {}{RESET}\n",
            regressions.join(", ")
        ));
    }
    out
}

fn scenario_card(scenario: &ScenarioOutcome) -> String {
    let (state, color) = if scenario.passed {
        ("pass", OK)
    } else {
        ("FAIL", WARN)
    };
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {color}{state}{RESET} {DIM}\u{2510}{RESET}",
        name = scenario.scenario,
        dashes = dashes_for(&format!(" {} ", scenario.scenario), state),
    );
    let mut lines = vec![format!("{DIM}\u{2502}{RESET} {DIM}{}{RESET}", scenario.family)];
    if let Some(failure) = &scenario.failure {
        lines.push(format!("{DIM}\u{2502}{RESET} {WARN}{failure}{RESET}"));
    }
    let bottom = format!("{DIM}\u{2514}{}\u{2518}{RESET}", "\u{2500}".repeat(CARD_WIDTH - 2));
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// Render the last recorded run back, for `fleet eval --last`.
pub fn render_last(root: &Path) -> String {
    let Some(row) = last_row(root) else {
        return format!(
            "{DIM}no eval-corpus run recorded for {} yet \u{b7} `fleet eval` runs the corpus and lands a row{RESET}\n",
            calib::today()
        );
    };
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet eval{RESET} {DIM}\u{2014} last run {} (UTC) \u{b7} {} passed \u{b7} {} failed \u{b7} score {:.2}{RESET}\n\n",
        row.ts, row.passed, row.failed, row.score
    ));
    for scenario in &row.scenarios {
        let (state, color) = if scenario.passed {
            ("pass", OK)
        } else {
            ("FAIL", WARN)
        };
        let mut line = format!(
            "  {color}{state}{RESET} {DIM}{family}{RESET}  {TEXT}{name}{RESET}",
            family = scenario.family,
            name = scenario.scenario,
        );
        if !scenario.failure.is_empty() {
            line.push_str(&format!("  {WARN}\u{2014} {}{RESET}", scenario.failure));
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

// --- the scratch dirs -----------------------------------------------------

/// A scratch directory unique per call, three ways over (pid, clock,
/// serial) so two scenarios never share one. A clock that reads
/// before the epoch, or a directory that cannot be created, is a
/// named error the scenario grades — never a panic in a driver.
fn scratch_dir(tag: &str) -> Result<PathBuf, String> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("the clock read before the epoch: {error}"))?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "fleet-eval-{tag}-{}-{nanos}-{serial}",
        std::process::id(),
    ));
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!("the scratch dir {} could not be created: {error}", dir.display())
    })?;
    Ok(dir)
}

// --- the driver -----------------------------------------------------------

/// One plain agent run: a single-endpoint client pointed at the mock, no
/// roster, no routing, no live endpoint. The shape every scenario drives.
async fn drive(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    max_turns: usize,
    memory: Option<Arc<Memory>>,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        model: "test-model".to_string(),
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib: None,
        skills: Vec::new(),
        memory,
        scope: None,
        improve_home: None,
    };
    let mut progress = Vec::new();
    // The agent's own construction replays the session's log, so a
    // fault there is an ordinary run error the scenario grades — the
    // driver never panics.
    let mut agent = match Agent::new(&client, options, session, approver, &mut progress) {
        Ok(agent) => agent,
        Err(error) => return Err(error),
    };
    agent.run(task).await
}

/// A scenario's session under `home`, or the graded failure naming
/// why it could not be created: a filesystem fault is a verdict, not
/// a panic.
fn create_session(
    home: &Path,
    name: &str,
    family: ScenarioFamily,
) -> Result<Session, ScenarioOutcome> {
    Session::create(home, "test-model").map_err(|error| {
        ScenarioOutcome::fail(
            name,
            family,
            format!("the session could not be created: {error}"),
        )
    })
}

/// A scenario's resumed session from `log_path`, or the graded
/// failure naming why it could not be reopened.
fn open_session(
    log_path: &Path,
    name: &str,
    family: ScenarioFamily,
) -> Result<Session, ScenarioOutcome> {
    Session::open(log_path).map_err(|error| {
        ScenarioOutcome::fail(
            name,
            family,
            format!("the session could not be reopened: {error}"),
        )
    })
}

// --- the scenarios --------------------------------------------------------

/// The two-turn tool loop: a call, its result, and the answer.
async fn two_turn_tool_loop(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "two-turn-tool-loop";
    let family = ScenarioFamily::ToolLoop;
    let mock = match MockApi::start(
        &mock_bin,
        vec![
            tool_call_stream("call-1", "bash", r#"{"command":"echo hello"}"#),
            assistant_stream("All done."),
        ],
    ) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let home = match scratch_dir("tool-loop") {
        Ok(home) => home,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    let result = match create_session(&home, NAME, family) {
        Ok(mut session) => {
            let outcome =
                drive(&mock, &mut session, Box::new(ApproveAll), 5, None, "say hello").await;
            let records = session::replay(&session.path()).unwrap_or_default();
            // The third record is the assistant's call, read through a
            // checked access: a short log grades the check failed
            // rather than panicking on the index.
            let assistant_call = records.get(2).is_some_and(|record| {
                matches!(record, Record::Assistant { tool_calls, .. }
                    if tool_calls.len() == 1 && tool_calls[0].id == "call-1")
            });
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(report) => {
                    let bodies = mock.bodies();
                    grade(
                        NAME,
                        family,
                        &[
                            ("the run completed", report.stop_reason == StopReason::Completed),
                            ("two turns", report.turns == 2),
                            (
                                "the final answer",
                                report.final_message.as_deref() == Some("All done."),
                            ),
                            ("both requests streamed with usage", bodies.len() == 2),
                            ("the second request saw the tool result", mock.roles(1)
                                == vec!["system", "user", "assistant", "tool"]),
                            ("the log reconstructs the turn", records.len() == 6),
                            ("the assistant record carries its call", assistant_call),
                        ],
                    )
                }
            }
        }
        Err(failure) => failure,
    };
    std::fs::remove_dir_all(&home).ok();
    result
}

/// Session-corpus replay: a recorded session replays and the outcomes
/// assert — the log reconstructs, and the requests it reconstructs match
/// what the model was actually sent.
async fn session_corpus_replay(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "session-corpus-replay";
    let family = ScenarioFamily::SessionCorpus;
    let mock = match MockApi::start(
        &mock_bin,
        vec![
            tool_call_stream("call-1", "bash", r#"{"command":"echo replay"}"#),
            assistant_stream("Replayed."),
        ],
    ) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let home = match scratch_dir("session-corpus") {
        Ok(home) => home,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    let result = match create_session(&home, NAME, family) {
        Ok(mut session) => {
            let outcome =
                drive(&mock, &mut session, Box::new(ApproveAll), 5, None, "replay me").await;
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(_) => {
                    let records = session::replay(&session.path()).unwrap_or_default();
                    // The reconstructed conversation (message-only face) must equal
                    // the messages the second request actually carried, minus the
                    // compiled-in system prompt. The request was built before the
                    // final answer was logged, so the comparison is against the
                    // records as they stood then — everything before the final
                    // answer's own record, the integration suite's own cut.
                    let cut = records
                        .iter()
                        .rposition(|record| {
                            matches!(
                                record,
                                Record::Assistant { content, tool_calls }
                                    if tool_calls.is_empty() && content.is_some()
                            )
                        })
                        .unwrap_or(records.len());
                    let replayed = session::messages(&records[..cut]);
                    let sent: Vec<Message> = mock
                        .bodies()
                        .get(1)
                        .and_then(|body| body.get("messages"))
                        .and_then(|messages| messages.as_array())
                        .map(|messages| {
                            messages
                                .iter()
                                .skip(1)
                                .filter_map(|message| {
                                    serde_json::from_value(message.clone()).ok()
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    grade(
                        NAME,
                        family,
                        &[
                            ("the log reconstructs", records.len() == 6),
                            ("the replayed conversation matches the request", replayed == sent),
                            (
                                "the reconstructed assistant carries its call",
                                replayed.iter().any(|message| !message.tool_calls.is_empty()),
                            ),
                        ],
                    )
                }
            }
        }
        Err(failure) => failure,
    };
    std::fs::remove_dir_all(&home).ok();
    result
}

/// Agent-continuation: a session resumes and continues coherently — the
/// resumed request carries the prior turns before the new task.
async fn agent_continuation(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "agent-continuation";
    let family = ScenarioFamily::AgentContinuation;
    let mock = match MockApi::start(
        &mock_bin,
        vec![
            tool_call_stream("call-1", "bash", r#"{"command":"echo one"}"#),
            assistant_stream("First answer."),
            assistant_stream("Second answer."),
        ],
    ) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let home = match scratch_dir("continuation") {
        Ok(home) => home,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    let result = match create_session(&home, NAME, family) {
        Ok(mut session) => {
            let first =
                drive(&mock, &mut session, Box::new(ApproveAll), 5, None, "first task").await;
            let log_path = session.path().to_path_buf();
            match first {
                Err(error) => ScenarioOutcome::fail(
                    NAME,
                    family,
                    format!("the first run failed: {error}"),
                ),
                Ok(_) => match open_session(&log_path, NAME, family) {
                    Ok(mut resumed) => {
                        match drive(
                            &mock,
                            &mut resumed,
                            Box::new(ApproveAll),
                            5,
                            None,
                            "second task",
                        )
                        .await
                        {
                            Err(error) => ScenarioOutcome::fail(
                                NAME,
                                family,
                                format!("the resumed run failed: {error}"),
                            ),
                            Ok(report) => grade(
                                NAME,
                                family,
                                &[
                                    (
                                        "the resumed run completed",
                                        report.stop_reason == StopReason::Completed,
                                    ),
                                    (
                                        "the resumed request carried the prior turns",
                                        mock.roles(2)
                                            == vec![
                                                "system", "user", "assistant", "tool",
                                                "assistant", "user",
                                            ],
                                    ),
                                    (
                                        "the new task rides last",
                                        mock.message_text(2, 5).as_deref() == Some("second task"),
                                    ),
                                ],
                            ),
                        }
                    }
                    Err(failure) => failure,
                },
            }
        }
        Err(failure) => failure,
    };
    std::fs::remove_dir_all(&home).ok();
    result
}

/// Conversation-fold: a long session folds and the post-fold request
/// carries the framed summary plus the verbatim tail.
async fn conversation_fold(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "conversation-fold";
    let family = ScenarioFamily::ConversationFold;
    // Five ordinary turns, a sixth whose API count crosses the budget, the
    // compaction answer (the summary), and the post-fold answer.
    let mut script: Vec<ScriptedResponse> = (1..=5)
        .map(|index| {
            tool_call_stream(
                &format!("call-{index}"),
                "bash",
                &format!(r#"{{"command":"echo {index}"}}"#),
            )
        })
        .collect();
    script.push(counting_turn(6, 120_000));
    script.push(assistant_stream("- the summary of the early turns"));
    script.push(assistant_stream("All done."));
    let mock = match MockApi::start(&mock_bin, script) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let home = match scratch_dir("fold") {
        Ok(home) => home,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    let result = match create_session(&home, NAME, family) {
        Ok(mut session) => {
            let outcome =
                drive(&mock, &mut session, Box::new(ApproveAll), 20, None, "the task").await;
            let records = session::replay(&session.path()).unwrap_or_default();
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(report) => {
                    let folded = records
                        .iter()
                        .any(|record| matches!(record, Record::Compaction { .. }));
                    // The post-fold request (the eighth) opens on the framed
                    // summary; the tail rides verbatim after it.
                    let post_fold = mock.message_text(7, 1).unwrap_or_default();
                    grade(
                        NAME,
                        family,
                        &[
                            ("the run completed", report.stop_reason == StopReason::Completed),
                            ("a compaction record landed", folded),
                            (
                                "the post-fold request carries the framed summary",
                                post_fold.starts_with(
                                    "Summary of turns 1 through 2, compacted to fit the window:",
                                ),
                            ),
                            (
                                "the summary text rode along",
                                post_fold.contains("- the summary of the early turns"),
                            ),
                        ],
                    )
                }
            }
        }
        Err(failure) => failure,
    };
    std::fs::remove_dir_all(&home).ok();
    result
}

/// Subagent-delegation: a delegated task runs in its own session on the
/// parent's model, and the parent's log carries the call and its result.
async fn subagent_delegation(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "subagent-delegation";
    let family = ScenarioFamily::Subagent;
    let mock = match MockApi::start(
        &mock_bin,
        vec![
            tool_call_stream(
                "call-1",
                "subagent",
                &json!({ "goal": "count the files" }).to_string(),
            ),
            tool_call_stream("child-1", "use_skill", r#"{"name":"demo-skill"}"#),
            assistant_stream("there are three files"),
            assistant_stream("the child counted three files"),
        ],
    ) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let home = match scratch_dir("subagent") {
        Ok(home) => home,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    let result = match create_session(&home, NAME, family) {
        Ok(mut session) => {
            let outcome =
                drive(&mock, &mut session, Box::new(ApproveAll), 5, None, "delegate the count")
                    .await;
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(_) => {
                    let parent_records = session::replay(&session.path()).unwrap_or_default();
                    let summaries = session::list(&home).unwrap_or_default();
                    let children: Vec<&session::SessionSummary> = summaries
                        .iter()
                        .filter(|summary| summary.kind.as_deref() == Some("subagent"))
                        .collect();
                    // The parent's log carries the delegation call and its result.
                    let subagent_result = parent_records.iter().find_map(|record| match record {
                        Record::ToolResult { name, output, approved, .. } if name == "subagent" => {
                            Some((output.clone(), *approved))
                        }
                        _ => None,
                    });
                    let delegation_ok = match &subagent_result {
                        Some((output, approved)) => {
                            *approved && output.starts_with("subagent completed")
                        }
                        None => false,
                    };
                    grade(
                        NAME,
                        family,
                        &[
                            ("two sessions exist", summaries.len() == 2),
                            ("one child log", children.len() == 1),
                            ("the parent's log carries the delegation result", delegation_ok),
                        ],
                    )
                }
            }
        }
        Err(failure) => failure,
    };
    std::fs::remove_dir_all(&home).ok();
    result
}

/// Memory-recall: a durable fact enters the turn's context, and the
/// `memory` record keeps the prompt reconstructable (hard rule 5).
async fn memory_recall(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "memory-recall";
    let family = ScenarioFamily::Memory;
    let mock = match MockApi::start(&mock_bin, vec![assistant_stream("Done.")]) {
        Ok(mock) => mock,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the mock could not start: {error}"),
            )
        }
    };
    let corpus = match scratch_dir("memory-corpus") {
        Ok(corpus) => corpus,
        Err(error) => {
            return ScenarioOutcome::fail(
                NAME,
                family,
                format!("the scratch dir could not be made: {error}"),
            )
        }
    };
    // The two fact files are the scenario's own substrate: a write
    // that faults grades the scenario rather than panicking.
    let written = std::fs::write(
        corpus.join("identity.md"),
        fact_file("identity", "the owner's standing rule about the gate", "confirmed-by", "feedback", ""),
    )
    .and_then(|()| {
        std::fs::write(
            corpus.join("top.md"),
            fact_file("top", "the railway deploy is failing per branch", "inferred", "project", "deploy"),
        )
    });
    let result = match written {
        Err(error) => ScenarioOutcome::fail(
            NAME,
            family,
            format!("the memory corpus could not be written: {error}"),
        ),
        Ok(()) => {
            // Keyword-only: the corpus never touches the shared loopback
            // llama-server, so the scenario is deterministic and offline.
            let memory = Arc::new(Memory::load_with(&corpus, Box::new(memory::KeywordOnly)));
            let home = match scratch_dir("memory-turn") {
                Ok(home) => home,
                Err(error) => {
                    std::fs::remove_dir_all(&corpus).ok();
                    return ScenarioOutcome::fail(
                        NAME,
                        family,
                        format!("the scratch dir could not be made: {error}"),
                    );
                }
            };
            let graded = match create_session(&home, NAME, family) {
                Ok(mut session) => {
                    let outcome = drive(
                        &mock,
                        &mut session,
                        Box::new(ApproveAll),
                        5,
                        Some(memory),
                        "the railway deploy is failing",
                    )
                    .await;
                    let records = session::replay(&session.path()).unwrap_or_default();
                    match outcome {
                        Err(error) => ScenarioOutcome::fail(
                            NAME,
                            family,
                            format!("the run failed: {error}"),
                        ),
                        Ok(_) => {
                            let system = mock.system(0);
                            let memory_record = records.iter().find_map(|record| match record {
                                Record::Memory { shape, facts } => Some((shape.clone(), facts.clone())),
                                _ => None,
                            });
                            let memory_ok = match &memory_record {
                                Some((shape, facts)) => {
                                    shape == "deploy"
                                        && facts == &vec!["identity".to_string(), "top".to_string()]
                                }
                                None => false,
                            };
                            grade(
                                NAME,
                                family,
                                &[
                                    ("the request carried the memory block", system.contains("## Memory")),
                                    (
                                        "the recalled fact entered the prompt",
                                        system.contains("the railway deploy is failing per branch"),
                                    ),
                                    ("the memory record names the shape and facts", memory_ok),
                                ],
                            )
                        }
                    }
                }
                Err(failure) => failure,
            };
            std::fs::remove_dir_all(&home).ok();
            graded
        }
    };
    std::fs::remove_dir_all(&corpus).ok();
    result
}

/// One durable fact in the writer's field order.
fn fact_file(name: &str, description: &str, basis: &str, fact_type: &str, shape: &str) -> String {
    format!(
        "---\nname: {name}\ndescription: \"{description}\"\ntype: {fact_type}\ndate: {today}\nbasis: {basis}\ntier: durable\nshape: [{shape}]\nrecall-count: 0\nsuperseded-by:\nvalid-until:\n---\n\nThe body of {name}.\n",
        today = calib::today()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(name: &str, passed: bool) -> ScenarioOutcome {
        ScenarioOutcome {
            scenario: name.to_string(),
            family: "tool-loop".to_string(),
            passed,
            failure: (!passed).then(|| format!("{name} broke").to_string()),
        }
    }

    #[test]
    fn the_families_name_the_forks_benchmarks_and_the_everyday_shapes() {
        assert_eq!(ScenarioFamily::SessionCorpus.label(), "session-corpus");
        assert_eq!(ScenarioFamily::AgentContinuation.label(), "agent-continuation");
        assert_eq!(ScenarioFamily::ConversationFold.label(), "conversation-fold");
        assert_eq!(ScenarioFamily::ToolLoop.label(), "tool-loop");
        assert_eq!(ScenarioFamily::Subagent.label(), "subagent");
        assert_eq!(ScenarioFamily::Memory.label(), "memory");
    }

    #[test]
    fn the_corpus_is_fixed_and_names_every_scenario() {
        let names: Vec<&str> = corpus().iter().map(|scenario| scenario.name).collect();
        assert_eq!(
            names,
            vec![
                "two-turn-tool-loop",
                "session-corpus-replay",
                "agent-continuation",
                "conversation-fold",
                "subagent-delegation",
                "memory-recall",
            ]
        );
    }

    #[test]
    fn a_green_corpus_scores_one_and_has_no_regression() {
        let report = EvalReport::from(vec![
            outcome("a", true),
            outcome("b", true),
            outcome("c", true),
        ]);
        assert!(report.green());
        assert_eq!(report.passed(), 3);
        assert_eq!(report.score(), 1.0);
        assert!(report.regressions().is_empty());
    }

    #[test]
    fn a_regressed_corpus_names_the_failing_scenario() {
        // The grading is exercised directly: a report holding one failing
        // scenario scores below one and names it, which is the fact the
        // improve gate and the surface both read.
        let report = EvalReport::from(vec![
            outcome("session-corpus-replay", true),
            ScenarioOutcome::fail(
                "conversation-fold",
                ScenarioFamily::ConversationFold,
                "the post-fold request lost the summary",
            ),
        ]);
        assert!(!report.green());
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed().len(), 1);
        assert_eq!(report.score(), 0.5);
        assert_eq!(report.regressions(), vec!["conversation-fold".to_string()]);
    }

    #[tokio::test]
    async fn a_broken_scenario_injected_through_the_seam_fails_with_its_name() {
        // The seam: a scenario that always fails, run without a mock. The
        // corpus grades it failed and names it, which is what the AC asks
        // of a deliberately broken scenario.
        let broken =
            Scenario::always_fails("injected-break", ScenarioFamily::ToolLoop, "the seam broke it");
        let result = broken.run(Path::new("/nonexistent/mock-server")).await;
        assert!(!result.passed);
        assert_eq!(result.scenario, "injected-break");
        assert_eq!(result.failure.as_deref(), Some("the seam broke it"));
        let report = EvalReport::from(vec![outcome("a", true), result]);
        assert_eq!(report.regressions(), vec!["injected-break".to_string()]);
    }

    #[test]
    fn a_green_report_renders_green_and_a_red_one_names_the_regression() {
        let green = render(&EvalReport::from(vec![outcome("two-turn-tool-loop", true)]));
        assert!(green.contains("fleet eval"), "{green}");
        assert!(green.contains("score 1.00"), "{green}");
        assert!(!green.contains("regressions:"), "{green}");

        let red = render(&EvalReport::from(vec![
            outcome("two-turn-tool-loop", true),
            ScenarioOutcome::fail(
                "conversation-fold",
                ScenarioFamily::ConversationFold,
                "the summary did not ride",
            ),
        ]));
        assert!(red.contains("regression"), "{red}");
        assert!(red.contains("regressions: conversation-fold"), "{red}");
        assert!(red.contains("the summary did not ride"), "{red}");
    }

    #[test]
    fn an_empty_stream_names_itself_rather_than_rendering_nothing() {
        let root = std::env::temp_dir().join(format!(
            "fleet-eval-last-empty-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let rendered = render_last(&root);
        assert!(rendered.contains("no eval-corpus run recorded"), "{rendered}");
        assert!(last_row(&root).is_none());
    }

    // --- the probe ---

    /// An exit status for the probe's tests: success or failure, the
    /// portable way — a real shell child's status, never a raw code.
    fn exit_status(code: i32) -> std::process::ExitStatus {
        let command = if cfg!(windows) { "cmd" } else { "sh" };
        let flag = if cfg!(windows) { "/C" } else { "-c" };
        std::process::Command::new(command)
            .arg(flag)
            .arg(format!("exit {code}"))
            .status()
            .expect("the status child")
    }

    #[test]
    fn a_green_corpus_output_grades_ok_and_a_red_one_names_the_scenarios() {
        let green = std::process::Output {
            status: exit_status(0),
            stdout: "fleet eval \u{2014} 6 scenarios, score 1.00\n".as_bytes().to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(grade_output(&green), Ok(()));

        let red = std::process::Output {
            status: exit_status(1),
            stdout: format!("{WARN}regressions: conversation-fold, memory-recall{RESET}\n")
                .into_bytes(),
            stderr: Vec::new(),
        };
        // The names the card carries must reach the gate step without
        // the palette's trailing reset riding along.
        let refused = grade_output(&red).expect_err("a red corpus refuses");
        assert_eq!(refused, "the eval corpus regressed: conversation-fold, memory-recall");
        assert!(!refused.contains('\u{1b}'), "{refused}");
    }

    #[test]
    fn a_red_corpus_without_a_named_line_still_refuses_named() {
        let red = std::process::Output {
            status: exit_status(1),
            stdout: Vec::new(),
            stderr: b"fleet: no home directory known\n".to_vec(),
        };
        let refused = grade_output(&red).expect_err("a red corpus refuses");
        assert!(refused.contains("run `fleet eval` for the detail"), "{refused}");
    }

    #[test]
    fn a_probe_refuses_when_the_binary_is_not_the_freshly_built_one() {
        // The gate root has no target/ holding this executable, so the
        // probe refuses naming it — never a silent skip that would
        // wave a source self-edit through on a stale binary's green.
        let probe = subprocess_probe(Path::new("/nonexistent-gate-root"));
        let refused = probe().expect_err("an absent binary refuses");
        assert!(refused.contains("did not resolve under"), "{refused}");
        assert!(refused.contains("target"), "{refused}");
        assert!(refused.contains("freshly-built"), "{refused}");
    }

    #[test]
    fn the_binary_check_accepts_the_gate_roots_own_target_dir() {
        // The unit-test binary sits under the repo's own target/
        // (deps included), and nowhere else — the freshly-built binary
        // is the one the gate may grade.
        let exe = std::env::current_exe().expect("the test binary's path");
        let target = exe.ancestors().nth(3).expect("target/ above debug/deps");
        assert!(under_target(&exe, target));
        assert!(!under_target(&exe, &target.join("release")));
        assert!(!under_target(&exe, Path::new("/usr/local")));
    }

    #[cfg(unix)]
    #[test]
    fn a_hung_corpus_is_killed_at_the_bound_rather_than_hanging_the_gate() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "fleet-eval-probe-hang-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let hang = dir.join("hang.sh");
        std::fs::write(&hang, "#!/bin/sh\nsleep 30\n").unwrap();
        let mut permissions = std::fs::metadata(&hang).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&hang, permissions).unwrap();
        // A short bound stands in for PROBE_TIMEOUT_SECS so the test
        // is fast; the kill and the named refusal are the production
        // ones.
        let started = std::time::Instant::now();
        let refused =
            run_eval_subprocess(&hang, std::time::Duration::from_millis(150)).expect_err("hung");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "the bound did not fire: {:?}",
            started.elapsed()
        );
        assert!(refused.contains("did not finish within"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
