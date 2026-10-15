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
//! A lane scenario adds a second seam beside the mock API: the lane
//! itself is the `mock-mcp` binary over a script ([`ScriptedLane`]), so
//! the blend1 scenarios drive the lane through a **scripted reply** —
//! the Rust-side form of the lane's own `_lane_overrides` test seam —
//! and never a live paid call. The lane's `ask` reply is a BlendResult
//! whose keys follow the vendored core's own additive contract
//! (`escalation`, `score`, `escalated_answer` beside the rest), so a
//! scenario grades fleet's side of the contract — the fact that reaches
//! the model, the answer that ships, and the keys that name what
//! happened — rather than the lane's internals.
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
use crate::route;
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
    sibling_binary("mock-server")
}

/// A sibling cargo binary by name, resolved beside the current executable
/// and then one directory up: cargo lays bin targets out at the profile
/// root (`target/debug/mock-server`) and the test binaries that ask for
/// them one directory down in `deps/`, so a corpus driven from a test
/// binary finds the same child a `fleet eval` subprocess does. `None`
/// when neither holds it, which the caller names rather than guessing at.
fn sibling_binary(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent().map(Path::to_path_buf);
    // Three ancestors covers `deps/` under either profile and a binary
    // launched from a nested directory, without an unbounded walk.
    for _ in 0..3 {
        let Some(candidate) = dir.clone() else { break };
        let path = candidate.join(&file);
        if path.exists() {
            return Some(path);
        }
        dir = candidate.parent().map(Path::to_path_buf);
    }
    None
}

/// The `mock-mcp` binary beside the fleet executable — the scripted MCP
/// child a lane scenario drives its lane through. Resolved the same way
/// [`mock_server_binary`] is, and `None` when it cannot be, which the
/// scenario grades as its own named failure rather than guessing at.
pub fn mock_mcp_binary() -> Option<PathBuf> {
    sibling_binary("mock-mcp")
}

// --- the scripted lane child -----------------------------------------------

/// A scripted MCP lane in a child process: the `mock-mcp` binary over a
/// script, standing in for a vendored lane core. Cross-process for the
/// same reason the mock API is — the MCP client's reads misbehave when
/// client and server share a process — and scripted rather than live, so
/// no scenario can spend a paid call. Every fault is a named error the
/// calling scenario grades.
struct ScriptedLane {
    child: Child,
    /// The `mock-mcp` binary's path, so the launch spec names the same
    /// child the scenario spawned.
    program: String,
    scratch: PathBuf,
}

impl ScriptedLane {
    /// Start the lane over `entries` (one script entry per request the
    /// client sends with an id: initialize, tools/list, then each call).
    fn start(mcp_bin: &Path, entries: Vec<serde_json::Value>) -> Result<ScriptedLane, String> {
        let scratch = scratch_dir("blend1-lane")?;
        let script_path = scratch.join("script.jsonl");
        let script = entries.iter().map(|entry| format!("{entry}\n")).collect::<String>();
        std::fs::write(&script_path, script)
            .map_err(|error| format!("the lane script could not be written: {error}"))?;
        let child = Command::new(mcp_bin)
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("the mock-mcp child could not be spawned: {error}"))?;
        Ok(ScriptedLane {
            child,
            program: mcp_bin.to_string_lossy().into_owned(),
            scratch,
        })
    }

    /// The client-side launch spec: the child over its own script, with
    /// no environment of its own — a scripted lane needs none.
    fn spec(&self) -> crate::mcp::ServerSpec {
        crate::mcp::ServerSpec {
            program: self.program.clone(),
            args: vec![self.scratch.join("script.jsonl").to_string_lossy().into_owned()],
            env: Vec::new(),
            passthrough: Vec::new(),
            cwd: None,
        }
    }
}

impl Drop for ScriptedLane {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::fs::remove_dir_all(&self.scratch).ok();
    }
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
    /// The explanation beat's discipline, graded end to end (slice 17).
    Explain,
    /// The calibration loop: the judgment, its discipline, the fit rule,
    /// and the conservative ride-back (slice 18).
    Calibration,
    /// A lane call whose result must carry a computed fact into the final
    /// answer unaltered — the grounded path blend1 exists for.
    GroundedLane,
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
            ScenarioFamily::Explain => "explain",
            ScenarioFamily::Calibration => "calibration",
            ScenarioFamily::GroundedLane => "grounded-lane",
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
        Scenario::new("memory-shape-inversion", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_shape_inversion(bin))
        }),
        Scenario::new("memory-spillover-routes", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_spillover_routes(bin))
        }),
        Scenario::new("memory-failure-shape-recall", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_failure_shape_recall(bin))
        }),
        Scenario::new("memory-phrases-round-trip", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_phrases_round_trip(bin))
        }),
        Scenario::new("blend1-grounded-fact", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_grounded_fact(bin))
        }),
        Scenario::new("blend1-escalation", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_escalation(bin))
        }),
        Scenario::new("blend1-reprocess", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_reprocess(bin))
        }),
        Scenario::new("blend1-corpus-recall", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_corpus_recall(bin))
        }),
        Scenario::new("explain-the-verdict", ScenarioFamily::Explain, |bin| {
            Box::pin(explain_the_verdict(bin))
        }),
        Scenario::new("calibration-loop-closes", ScenarioFamily::Calibration, |bin| {
            Box::pin(calibration_loop_closes(bin))
        }),
    ]
}

/// The judged artifact's definition for an `eval_corpus` row: the
/// corpus's own fixed text — every scenario's name and family in
/// corpus order, under the crate's version. The corpus is code, so its
/// artifact is its definition: an explanation keyed to it names the
/// exact suite that graded the run, and a corpus that changed since the
/// row was written is a drift the explanation refuses rather than
/// papers over (slice 17). The one construction both the corpus row and
/// the explanation beat key on, so the two cannot diverge.
pub fn corpus_definition() -> String {
    let mut text = format!("fleet-eval-corpus/{}", env!("CARGO_PKG_VERSION"));
    for scenario in corpus() {
        text.push('\n');
        text.push_str(scenario.name);
        text.push(' ');
        text.push_str(scenario.family.label());
    }
    text
}

/// The judged artifact's identity for an `eval_corpus` row: the 16 hex
/// characters of the sha256 over the corpus's own fixed definition (see
/// [`corpus_definition`]).
pub fn corpus_identity() -> String {
    crate::calib::hash16(&corpus_definition())
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
            // The judged artifact's identity: the corpus's own fixed
            // definition, so an explanation can key to the exact suite
            // that graded this run and refuse a drifted one.
            input_sha256: Some(corpus_identity()),
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
/// corpus is ten deterministic runs — six mock-server runs and four
/// lane runs over a scripted MCP child — so a child that has
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
/// can kill it when the bound expires. The corpus is ten
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
/// reset into the gate step's detail. Shared with the TUI, so the
/// explanation modal's width budget is spent on visible columns only.
pub(crate) fn strip_ansi(text: &str) -> String {
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
    drive_over(mock, session, approver, max_turns, memory, None, task).await
}

/// `drive` with lane specs injected: a lane scenario's run meets its
/// scripted lane through this rather than through the vendored registry,
/// exactly as the integration suite's `run_with_lanes` does.
async fn drive_over(
    mock: &MockApi,
    session: &mut Session,
    approver: Box<dyn Approver>,
    max_turns: usize,
    memory: Option<Arc<Memory>>,
    lane_specs: Option<std::collections::HashMap<String, crate::mcp::ServerSpec>>,
    task: &str,
) -> Result<RunReport, AgentError> {
    let client = Client::new(mock.url(), "test-key");
    let options = AgentOptions {
        model: "test-model".to_string(),
        max_turns,
        ui_tx: None,
        lane_specs,
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

/// One durable fact in the writer's field order with the slice-19
/// fields: the shape tags, the resolution's phrases, the missing
/// companions, and the recall history — the substrate the shape-first
/// recall scenarios grade.
fn trajectory_fact_file(
    name: &str,
    description: &str,
    basis: &str,
    shape: &str,
    phrases: &[&str],
    missing: &[&str],
    recalls: u64,
) -> String {
    format!(
        "---\nname: {name}\ndescription: \"{description}\"\ntype: project\ndate: {today}\nbasis: {basis}\ntier: durable\nshape: [{shape}]\nphrases: [{}]\nmissing: [{}]\nrecall-count: {recalls}\nsuperseded-by:\nvalid-until:\n---\n\nThe body of {name}.\n",
        phrases.join(", "),
        missing.join(", "),
        today = calib::today()
    )
}

/// Write one fact file into a scenario's scratch corpus. A fault is a
/// named error the scenario grades — a driver never panics.
fn write_fact_file(root: &Path, name: &str, text: &str) -> Result<(), String> {
    std::fs::create_dir_all(root)
        .map_err(|error| format!("the scratch dir {} could not be made: {error}", root.display()))?;
    std::fs::write(root.join(format!("{name}.md")), text)
        .map_err(|error| format!("the fact {name} could not be written: {error}"))
}

/// One memory-family scenario's corpus, graded on a fault: the scratch
/// dir is created and every fact written, or the scenario fails naming
/// the fault.
fn memory_corpus(
    name: &str,
    family: ScenarioFamily,
    tag: &str,
    facts: &[(&str, String)],
) -> Result<PathBuf, ScenarioOutcome> {
    let corpus = match scratch_dir(tag) {
        Ok(corpus) => corpus,
        Err(error) => {
            return Err(ScenarioOutcome::fail(
                name,
                family,
                format!("the scratch dir could not be made: {error}"),
            ))
        }
    };
    for (fact, text) in facts {
        if let Err(error) = write_fact_file(&corpus, fact, text) {
            std::fs::remove_dir_all(&corpus).ok();
            return Err(ScenarioOutcome::fail(name, family, error));
        }
    }
    Ok(corpus)
}

/// The names a ranked recall surfaces, in rank order.
fn hit_names(hits: &[crate::memory::ScoredFact]) -> Vec<&str> {
    hits.iter().map(|hit| hit.fact.name.as_str()).collect()
}

// --- the memory family: shape-first recall ---------------------------------

/// The inversion, at the corpus level: a same-shape fact with WEAK word
/// evidence outranks a wrong-shape fact with STRONG word evidence and a
/// long recall history. This is M1's own proof — under the old
/// weighting (the word channels at 0.80 plus a 0.15 shape boost) the
/// wrong-shape fact ranked first, and the corpus is built to fail there.
async fn memory_shape_inversion(_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "memory-shape-inversion";
    let family = ScenarioFamily::Memory;
    let corpus = match memory_corpus(
        NAME,
        family,
        "memory-inversion",
        &[
            (
                "same-shape",
                fact_file("same-shape", "the branch pipeline is green", "confirmed-by", "project", "deploy"),
            ),
            (
                "wrong-shape",
                trajectory_fact_file(
                    "wrong-shape",
                    "the railway deploy is failing",
                    "confirmed-by",
                    "rendering",
                    &[],
                    &[],
                    8,
                ),
            ),
        ],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    // Keyword-only: the scenario never touches the shared loopback
    // llama-server, so it is deterministic and offline.
    let memory = Memory::load_with(&corpus, Box::new(memory::KeywordOnly));
    let partition = memory.partition("the railway deploy is failing");
    let same = partition.primary.first().expect("the pool has its fact");
    let wrong = partition.spillover.first().expect("the spillover has its fact");
    let ranked = memory.recall("the railway deploy is failing");
    let order = hit_names(&ranked);
    std::fs::remove_dir_all(&corpus).ok();
    grade(
        NAME,
        family,
        &[
            ("the instruction classifies into the shape", partition.shape == Some("deploy")),
            ("the same-shape fact is the primary pool", hit_names(&partition.primary) == ["same-shape"]),
            ("the wrong-shape fact is spillover", hit_names(&partition.spillover) == ["wrong-shape"]),
            (
                "the word evidence is on the wrong-shape side",
                wrong.score.total > same.score.total,
            ),
            (
                "the shape's own answer leads the ranking",
                order == ["same-shape", "wrong-shape"],
            ),
            ("the off-shape fact is marked", !same.off_shape && wrong.off_shape),
        ],
    )
}

/// The spillover's three conditions, each on its own corpus: the pool
/// is empty, the pool's best is below the floor, and the pool's best
/// clears it. The routes are what keep an off-shape fact from being
/// silently mixed into the shape's answers.
async fn memory_spillover_routes(_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "memory-spillover-routes";
    let family = ScenarioFamily::Memory;
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // Route (a): the pool is empty, so the off-shape facts are the
    // answer — and the strongest of them enters even at a budget that
    // fits nothing else.
    let route_a = match memory_corpus(
        NAME,
        family,
        "memory-spill-empty",
        &[(
            "off-one",
            fact_file("off-one", "the railway deploy is failing per branch", "confirmed-by", "project", "rendering"),
        )],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let empty_pool = Memory::load_with(&route_a, Box::new(memory::KeywordOnly));
    let empty = empty_pool.assemble_within("the railway deploy is failing", &[], 1);
    checks.push(("route (a): the off-shape fact is the answer", hit_names(&empty.recalled) == ["off-one"]));
    checks.push(("route (a): it is marked off-shape", empty.recalled.first().is_some_and(|hit| hit.off_shape)));
    checks.push((
        "route (a): the route names the empty pool",
        empty
            .spillover_route
            .as_deref()
            .is_some_and(|route| route.contains("no fact carries `deploy`")),
    ));
    std::fs::remove_dir_all(&route_a).ok();

    // Route (b): the pool's best is below the floor, so the shape
    // matched without corroborating itself and the strongest off-shape
    // fact enters beside the weak pool — again under budget pressure.
    let route_b = match memory_corpus(
        NAME,
        family,
        "memory-spill-weak",
        &[
            (
                "same-shape",
                fact_file("same-shape", "the branch pipeline is green", "confirmed-by", "project", "deploy"),
            ),
            (
                "wrong-shape",
                trajectory_fact_file(
                    "wrong-shape",
                    "the railway deploy is failing",
                    "confirmed-by",
                    "rendering",
                    &[],
                    &[],
                    8,
                ),
            ),
        ],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let weak_pool = Memory::load_with(&route_b, Box::new(memory::KeywordOnly));
    let weak = weak_pool.assemble_within("the railway deploy is failing", &[], 1);
    checks.push((
        "route (b): the weak pool's neighbour enters",
        hit_names(&weak.recalled) == ["same-shape", "wrong-shape"],
    ));
    checks.push((
        "route (b): the route names the floor",
        weak
            .spillover_route
            .as_deref()
            .is_some_and(|route| route.contains("below the SPILLOVER_FLOOR")),
    ));
    std::fs::remove_dir_all(&route_b).ok();

    // Route (c) and the floor's other side: a pool whose best clears the
    // floor stands alone, so the off-shape fact enters only on the
    // budget's room — and is dropped, named, when there is none.
    let route_c = match memory_corpus(
        NAME,
        family,
        "memory-spill-strong",
        &[
            (
                "strong",
                trajectory_fact_file(
                    "strong",
                    "the railway deploy is failing per branch",
                    "confirmed-by",
                    "deploy",
                    &["read the railway deploy logs before failing again"],
                    &[],
                    8,
                ),
            ),
            (
                "off-shape-fact",
                fact_file("off-shape-fact", "the railway deploy is failing", "confirmed-by", "project", "rendering"),
            ),
        ],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let strong_pool = Memory::load_with(&route_c, Box::new(memory::KeywordOnly));
    let partition = strong_pool.partition("the railway deploy is failing");
    let best = partition.best().unwrap_or(0.0);
    let room = strong_pool.assemble("the railway deploy is failing", &[]);
    let tight = strong_pool.assemble_within("the railway deploy is failing", &[], 1);
    checks.push(("the floor's other side: the pool clears it", best >= crate::memory::SPILLOVER_FLOOR));
    checks.push((
        "route (c): the budget had room, so the off-shape fact enters",
        hit_names(&room.recalled) == ["strong", "off-shape-fact"],
    ));
    checks.push((
        "route (c): no room, so it is dropped and named",
        hit_names(&tight.recalled) == ["strong"] && tight.dropped == ["off-shape-fact"],
    ));
    checks.push((
        "route (c): the tight assembly carries no off-shape marker",
        !crate::memory::render_assembly(&tight).contains("off-shape"),
    ));
    std::fs::remove_dir_all(&route_c).ok();

    grade(NAME, family, &checks)
}

/// The failure-shape recall: the avoidance half of the shelf, served
/// through the same partition as a resolution, with the three signs
/// named so an anti-vector is never read as a candidate and a stalled
/// trajectory is never read as done.
async fn memory_failure_shape_recall(_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "memory-failure-shape-recall";
    let family = ScenarioFamily::Memory;
    let corpus = match memory_corpus(
        NAME,
        family,
        "memory-failure-shape",
        &[
            (
                "what-broke",
                trajectory_fact_file(
                    "what-broke",
                    "the calibration reader cannot read a null",
                    "confirmed-by",
                    "drops-the-row",
                    &["parse_row drops the whole line"],
                    &[],
                    0,
                ),
            ),
            (
                "what-fixed-it",
                trajectory_fact_file(
                    "what-fixed-it",
                    "the reader now takes a null into a numeric field",
                    "confirmed-by",
                    "drops-the-row, resolved",
                    &["serde default for Option", "parse_row keeps the row"],
                    &[],
                    0,
                ),
            ),
            (
                "what-stalled",
                trajectory_fact_file(
                    "what-stalled",
                    "the reader keeps the row but loses its timestamp",
                    "confirmed-by",
                    "drops-the-row, incomplete",
                    &["serde default for Option"],
                    &["the timezone-aware parser"],
                    0,
                ),
            ),
            (
                "both-axes",
                fact_file(
                    "both-axes",
                    "the shader layout repaints and the tower scan read was unbounded",
                    "confirmed-by",
                    "project",
                    "rendering, bound-exceeded",
                ),
            ),
        ],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let memory = Memory::load_with(&corpus, Box::new(memory::KeywordOnly));
    let failure = memory.partition("the row vanished from the stream");
    let rendering = memory.partition("the shader layout repaints every frame");
    let bound = memory.partition("the tower scan read was unbounded");
    let block = crate::memory::render_assembly(&memory.assemble("the row vanished from the stream", &[]));
    let signs: Vec<crate::memory::Sign> =
        failure.primary.iter().map(|hit| hit.fact.sign()).collect();
    std::fs::remove_dir_all(&corpus).ok();
    grade(
        NAME,
        family,
        &[
            ("the instruction classifies into the failure shape", failure.shape == Some("drops-the-row")),
            (
                "the failure pool is the shape's own facts",
                hit_names(&failure.primary) == ["what-broke", "what-fixed-it", "what-stalled"],
            ),
            (
                "the three signs are named apart",
                signs
                    == [
                        crate::memory::Sign::Divergence,
                        crate::memory::Sign::Resolution,
                        crate::memory::Sign::Incomplete,
                    ],
            ),
            (
                "the failure-shape header names all three outcomes",
                block.contains("what broke before, what resolved it, and what stalled"),
            ),
            (
                "the stalled trajectory names its companions",
                block.contains("missing: the timezone-aware parser"),
            ),
            (
                "either axis matches a both-axes fact",
                rendering.primary.iter().any(|hit| hit.fact.name == "both-axes")
                    && bound.primary.iter().any(|hit| hit.fact.name == "both-axes"),
            ),
            (
                "the legend names every sign",
                block.contains("resolved:") && block.contains("diverged:") && block.contains("incomplete:"),
            ),
        ],
    )
}

/// The phrases' round trip: the write path records the vocabulary that
/// resolved the problem, and a later recall for the shape matches it
/// even when the question is worded differently — while the same fact
/// with no recorded direction never clears the floor.
async fn memory_phrases_round_trip(_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "memory-phrases-round-trip";
    let family = ScenarioFamily::Memory;
    let corpus = match memory_corpus(
        NAME,
        family,
        "memory-phrases",
        &[
            (
                "deploy-credentials-rule",
                trajectory_fact_file(
                    "deploy-credentials-rule",
                    "the deploy credential rule",
                    "confirmed-by",
                    "deploy",
                    &["read CLAUDE.local.md directly", "never grep-extract credentials"],
                    &[],
                    0,
                ),
            ),
            (
                "deploy-credentials-twin",
                fact_file(
                    "deploy-credentials-twin",
                    "the deploy credential rule",
                    "confirmed-by",
                    "project",
                    "deploy",
                ),
            ),
        ],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let memory = Memory::load_with(&corpus, Box::new(memory::KeywordOnly));
    // The question is worded differently from the phrases: it says
    // "for the railway credentials", the resolution says
    // "read CLAUDE.local.md directly".
    let hits = memory.recall("read CLAUDE.local.md for the railway credentials");
    let ranked = hit_names(&hits);
    let trajectory = hits.first().map(|hit| hit.score.trajectory).unwrap_or(0.0);
    let keyword = hits.first().map(|hit| hit.score.keyword).unwrap_or(0.0);
    let primary = hits.first().is_some_and(|hit| !hit.off_shape);
    // The phrases ride the corpus's own card, where the operator reads
    // what drove beside what the fact is about.
    let cards = crate::memory::render(&memory, None);
    std::fs::remove_dir_all(&corpus).ok();
    grade(
        NAME,
        family,
        &[
            ("the phrase-carrying fact is the only hit", ranked == ["deploy-credentials-rule"]),
            ("the trajectory channel matched it", (trajectory - 0.625).abs() < 1e-9),
            ("the topical channels barely did", (keyword - 0.125).abs() < 1e-9),
            ("it is the shape's own pool", primary),
            ("the twin with no direction is absent", !ranked.contains(&"deploy-credentials-twin")),
            (
                "the surface cards the resolution's vocabulary",
                cards.contains("drove: read CLAUDE.local.md directly"),
            ),
        ],
    )
}

// --- blend1: the grounded lane --------------------------------------------

/// The lane name every blend1 scenario addresses, and the namespace its
/// tools ride under in the model's tool set.
const BLEND1: &str = "blend1";

/// The fact the tabular lane computes in every blend1 scenario: a TabPFN
/// probability. "LLMs do not do math; TabPFN does" — the number reaches
/// the synthesis model as a given, and the scenario's whole job is to
/// prove nothing between the lane and the operator rewrites it.
const FACT: &str = "0.924";

/// The `ask` arguments every blend1 scenario's model turn calls with: one
/// prompt, a three-row table whose third row is the query, and the target
/// column TabPFN predicts.
const ASK_ARGUMENTS: &str = r#"{"prompt":"will customer 1042 churn?","table":[{"tenure_months":4,"monthly_charges":89.5,"churned":true},{"tenure_months":52,"monthly_charges":21.0,"churned":false},{"tenure_months":7,"monthly_charges":95.2}],"target":"churned"}"#;

/// The blend1 lane's five tools, as the vendored adapter declares them.
/// The corpus tools ride beside `ask` and `doctor` (plan B12).
fn blend1_tools() -> serde_json::Value {
    serde_json::json!([
        { "name": "ask", "description": "one blended ask", "inputSchema": { "type": "object" } },
        { "name": "doctor", "description": "per-lane availability", "inputSchema": { "type": "object" } },
        { "name": "search", "description": "semantic search over a project's corpus", "inputSchema": { "type": "object" } },
        { "name": "refresh", "description": "docs-derived corpus refresh, hash-gated", "inputSchema": { "type": "object" } },
        { "name": "seed", "description": "the corpus's write path", "inputSchema": { "type": "object" } },
    ])
}

/// The scripted lane's three entries: initialize, tools/list, and the one
/// `tools/call` whose text block is `reply`.
fn blend1_lane_entries(reply: &str) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "blend1", "version": "0.1.0" },
            },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "tools": blend1_tools() },
        } }),
        serde_json::json!({ "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text", "text": reply }], "isError": false },
        } }),
    ]
}

/// A BlendResult fixture with the wire contract's own keys, plus `extra`
/// merged over it. The result-key contract is additive, so a leg built
/// upstream rides in as an extra key without touching the base.
fn blend1_result(extra: serde_json::Value) -> serde_json::Value {
    let mut result = serde_json::json!({
        "prompt_sha256": "9f2c1d4b6a8e0357",
        "prompt_chars": 47,
        "path": "synthesized",
        "route": {
            "status": "ok", "provider": "liquid",
            "needs_generation": 0.978, "needs_live_facts": 0.0,
            "answer_shape": "synthesized", "latency_ms": 900,
        },
        "semantic": {
            "status": "ok", "provider": "llama-server", "dims": 768,
            "neighbors": [{ "key": "churn-playbook", "score": 0.771,
                            "text": "the churn playbook's threshold is p=0.90" }],
        },
        "tabular": {
            "status": "ok", "model": "tabpfn-hosted", "verdict": "ok",
            "predictions": [{ "label": "churned",
                              "probabilities": { "churned": 0.924, "stayed": 0.076 } }],
            "n_context_rows": 40, "n_query_rows": 1, "latency_ms": 11_800,
        },
        "generated": { "status": "ok", "model": "lfm2.5-24b-a2b" },
        "validation": { "status": "ok", "provider": "glide" },
        "escalation": {},
        "score": {},
        "voice": {},
        "web": {},
        "total_latency_ms": 15_400,
        // `escalated_answer` flags the SHIPPED answer as the escalated
        // one. `escalated` is the library's own property and means "a
        // lane failed open" — which includes a gate whose verdict did
        // not clear, because the core computes the gate as `not passed`
        // and `None` (an inconclusive verdict) is not truthy. So the
        // escalation leg, which fires only on an unresolved gate, always
        // sets BOTH keys: `escalated` from the gate, `escalated_answer`
        // to say the shipped answer is the escalation's. The adapter
        // adds `escalated` and `failed_lanes` beside the result's own
        // keys.
        "escalated_answer": false,
        "escalated": false,
        "failed_lanes": [],
    });
    if let (Some(base), Some(overlay)) = (result.as_object_mut(), extra.as_object()) {
        for (key, value) in overlay {
            base.insert(key.clone(), value.clone());
        }
    }
    result
}

// --- the explanation beat (slice 17) --------------------------------------

/// The prose the scripted lane answers an explain ask with.
const EXPLANATION: &str = "The gate refused the change at the `test` step: the ranking \
assertion `the_ranker_orders_a_corpus` no longer holds, because the patched comparator \
sorts on the score's absolute value rather than its rank. Restore the rank comparison, or \
update the assertion to the ordering the patch intends — the rest of the gate is green.";

/// The explanation beat, graded end to end through the scripted lane: a
/// failed verdict is explained beside itself, the verdict stands exactly
/// as recorded, the row is redacted, the text is recoverable by its sha,
/// the day's spend is recorded, and `--dry-run` spends nothing. No paid
/// call: the lane is the scripted MCP child, the same seam the blend1
/// scenarios drive.
async fn explain_the_verdict(_mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "explain-the-verdict";
    let family = ScenarioFamily::Explain;
    let Some(mcp_bin) = mock_mcp_binary() else {
        return ScenarioOutcome::fail(
            NAME,
            family,
            "the mock-mcp binary was not found beside the fleet executable; the lane seam cannot run",
        );
    };
    let (home, root) = match scratch_dir("explain-home").and_then(|home| {
        scratch_dir("explain-calib").map(|root| (home, root))
    }) {
        Ok(pair) => pair,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    // A scratch repo the proposal is filed against, so the draft stamps
    // its target's sha the way the real tool does.
    let repo = match scratch_dir("explain-repo") {
        Ok(repo) => repo,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    if let Err(error) = std::fs::create_dir_all(repo.join("src"))
        .and_then(|()| std::fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n"))
    {
        return ScenarioOutcome::fail(NAME, family, format!("the scratch repo: {error}"));
    }
    // The proposal the gate refused: filed through the tool's own path,
    // so the drift guard's stamping is the real one.
    let change = r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
    let filed = crate::improve::file_from_tool(
        &home,
        &repo,
        &serde_json::json!({
            "name": "fix-the-ranker",
            "target": "source",
            "target-name": "src/lib.rs",
            "basis": "inferred",
            "justification": "the ranking assertion failed after the comparator changed",
            "acceptance": "the corpus's ranking assertion holds",
            "effect": "the ranker orders a corpus again",
            "change": change,
        }),
    );
    if !filed.contains("filed proposal") {
        return ScenarioOutcome::fail(NAME, family, format!("the proposal did not file: {filed}"));
    }
    let artifact_sha = crate::improve::parse_change(
        &serde_json::from_str::<serde_json::Value>(change).unwrap_or_default(),
    )
    .map(|change| change.artifact_sha())
    .unwrap_or_default();
    // The verdict row, exactly as the gate's writer lands it: refused,
    // with the per-step evidence and the judged artifact's sha.
    let stream = crate::calib::Stream::open(&root);
    stream.record(Row::Propose(calib::ProposeRow {
        ts: calib::stamp(),
        proposal: "fix-the-ranker".to_string(),
        target: "source".to_string(),
        target_name: "src/lib.rs".to_string(),
        basis: "inferred".to_string(),
        verdict: calib::VERDICT_REFUSED.to_string(),
        reason: "test failed: the_ranker_orders_a_corpus".to_string(),
        gate: vec![
            calib::GateStepRow { step: "fmt".to_string(), ok: true, detail: "ok".to_string() },
            calib::GateStepRow {
                step: "test".to_string(),
                ok: false,
                detail: "error: test failed: the_ranker_orders_a_corpus".to_string(),
            },
        ],
        input_sha256: Some(artifact_sha.clone()),
    }));
    drop(stream);
    let selector = crate::explain::Selector::Proposal("fix-the-ranker".to_string());
    let verdict_before = crate::explain::recorded_verdict(&root, &selector);

    // The scripted lane: one ask, answered with the BlendResult's own
    // keys plus the explanation's text and its cost.
    let lane = match ScriptedLane::start(
        &mcp_bin,
        blend1_lane_entries(
            &blend1_result(serde_json::json!({
                "path": "synthesized",
                "route": { "status": "ok", "provider": "liquid", "needs_generation": 0.978,
                           "answer_shape": "synthesized", "latency_ms": 900 },
                "generated": { "status": "ok", "model": "lfm2.5-24b-a2b", "text": EXPLANATION,
                               "cost_usd": 0.0018 },
                "validation": { "status": "ok", "provider": "glide", "passed": true,
                                "outcome": "passed", "number_check": "ok",
                                "answers_the_question": true, "contradicts_facts": false },
                "final_answer": EXPLANATION,
                "total_latency_ms": 15_400,
            }))
            .to_string(),
        ),
    ) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };

    let outcome = crate::explain::explain(crate::explain::ExplainRequest {
        root: &root,
        home: &home,
        selector: selector.clone(),
        dry_run: false,
        lane: crate::explain::LaneSource::Scripted(lane.spec()),
    })
    .await;

    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("the beat explained the verdict", outcome.explained()));
    checks.push((
        "the verdict stands exactly as recorded",
        crate::explain::recorded_verdict(&root, &selector) == verdict_before,
    ));
    let Some(row) = outcome.row.as_ref() else {
        return grade(NAME, family, &checks);
    };
    // The row is redacted: no prompt text and no answer text rides in
    // it, under any key a reader might look for.
    let row_text = serde_json::to_string(row).unwrap_or_default();
    checks.push((
        "the row is redacted",
        !row_text.contains(EXPLANATION) && !row_text.contains("GATE OP") && !row_text.contains("ARTIFACT"),
    ));
    checks.push(("the row carries the eval-only marker", row.eval_only));
    checks.push(("the row joins on the artifact's sha", row.input_sha256 == artifact_sha));
    // The text is recoverable by its sha: the file the row indexes
    // hashes to the sha the row recorded.
    let path = crate::explain::explanation_path(&home, &row.input_sha256);
    let recovered = std::fs::read_to_string(&path).unwrap_or_default();
    checks.push(("the text is recoverable by its sha", {
        !recovered.is_empty() && crate::calib::hash16(&recovered) == row.explanation_sha256
    }));
    // The card names the verdict and the cost, and never reads as a
    // plain-text dump.
    checks.push((
        "the card carries the verdict and the cost",
        outcome.card.contains("refused") && outcome.card.contains("$0.0018"),
    ));
    // The day's spend is recorded from the row, and the ask's telemetry
    // rode with it.
    checks.push((
        "the day's spend is recorded",
        (crate::explain::day_spend(&root, &home, &calib::today()) - 0.0018).abs() < 1e-9,
    ));
    checks.push((
        "the route and validation telemetry rode the row",
        row.route.path == "synthesized" && row.validation.outcome == "passed",
    ));

    // The dry-run leg: no ask, no row, no file. The stream still holds
    // exactly the one explanation the first leg landed, and the card
    // says so.
    let dry = crate::explain::explain(crate::explain::ExplainRequest {
        root: &root,
        home: &home,
        selector,
        dry_run: true,
        lane: crate::explain::LaneSource::Scripted(lane.spec()),
    })
    .await;
    checks.push(("the dry run spends nothing", dry.row.is_none() && dry.refusal.is_none()));
    checks.push((
        "the dry run's card shows what would be asked",
        dry.card.contains("dry run") && dry.card.contains("no ask, no row, no file"),
    ));
    checks.push((
        "the dry run lands no row and no file",
        crate::explain::explain_rows(&root).len() == 1
            && std::fs::read_to_string(&path).unwrap_or_default() == recovered,
    ));

    // The fail-open leg: a lane whose generative leg failed open — the
    // vendored core's LIQUID-unset shape (generated.status error, the
    // fallback prose shipped as final_answer) — is a named refusal, not
    // an explanation. The verdict stands, no second row, no fallback
    // file written under the artifact's sha.
    let fallback_reply = blend1_result(serde_json::json!({
        "path": "synthesized",
        "generated": { "status": "error",
                       "error": "generate lane is not configured: set LIQUID_API_KEY (the same key the d1 lane uses) in ~/.config/blend1/env",
                       "model": "lfm2.5-24b-a2b" },
        "final_answer": "Generative lane unavailable: generate lane is not configured: set LIQUID_API_KEY (the same key the d1 lane uses) in ~/.config/blend1/env",
    }))
    .to_string();
    let fail_lane = match ScriptedLane::start(&mcp_bin, blend1_lane_entries(&fallback_reply)) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let fail = crate::explain::explain(crate::explain::ExplainRequest {
        root: &root,
        home: &home,
        selector: crate::explain::Selector::Proposal("fix-the-ranker".to_string()),
        dry_run: false,
        lane: crate::explain::LaneSource::Scripted(fail_lane.spec()),
    })
    .await;
    checks.push((
        "a lane that fails open is refused by name, not shipped as the explanation",
        fail.refusal.is_some() && fail.row.is_none(),
    ));
    checks.push((
        "the fail-open refusal carries the lane's own error",
        fail.refusal.as_deref().is_some_and(|reason| reason.contains("LIQUID_API_KEY")),
    ));
    checks.push((
        "the fail-open lands no second row and no fallback file",
        crate::explain::explain_rows(&root).len() == 1
            && std::fs::read_to_string(&path).unwrap_or_default() == recovered,
    ));

    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&repo);
    grade(NAME, family, &checks)
}

// --- the calibration loop (slice 18) -------------------------------------

/// One `route` row as the router's writer lands it, for the scenario's
/// own stream. `provider` served; the walk is named by `attempts`.
fn route_row_line(
    ts: &str,
    provider: &str,
    workload: &str,
    attempts: &[(&str, &str, &str)],
    input_hash: &str,
) -> serde_json::Value {
    serde_json::json!({
        "ts": ts,
        "op": "route",
        "provider": provider,
        "how": "judged",
        "workload": workload,
        "verdict": "pass",
        "reason": format!("sys1 judged `{workload}`"),
        "model": "GLM-5.3-Flash",
        "providers_used": attempts.iter().map(|(id, _, _)| id).collect::<Vec<_>>(),
        "attempts": attempts
            .iter()
            .map(|(id, outcome, class)| serde_json::json!({
                "provider": id, "outcome": outcome, "class": class,
            }))
            .collect::<Vec<_>>(),
        "input_chars": 812,
        "input_hash": input_hash,
        "latency_ms": 812,
    })
}

/// One `route_judged` row as the beat's writer lands it.
fn judged_row_line(
    ts: &str,
    route_key: &str,
    provider: &str,
    workload: &str,
    verdict: &str,
    confidence: f64,
) -> serde_json::Value {
    serde_json::json!({
        "ts": ts,
        "op": "route_judged",
        "route_key": route_key,
        "provider": provider,
        "workload": workload,
        "verdict": verdict,
        "confidence": confidence,
        "reason": "the lane judged the workload and the outcome evidence agreed",
        "cost_usd": 0.0002,
        "latency_ms": 900,
        "lane": "sys1",
    })
}

/// The scripted lane's entries for one judgment batch: the handshake,
/// the tool list, and one `tools/call` reply per ask the batch makes.
/// The reply is the `sys1_classify` answer's own shape, so fleet's side
/// of the contract is what the scenario grades.
///
/// `hashes` carries one route row's `input_hash` per answer: each call
/// entry is keyed on `input hash: <hash>`, the line compose_prompt
/// emits, so an ask's own CONTENT selects its answer. The judge spawns
/// a fresh child per ask and every child reads the script from its
/// head, so without the key every ask would receive the first entry —
/// the batch's confidences would all read alike and the fit leg would
/// grade an accident rather than the design.
///
/// The tool list is ONE entry, not one per ask: each child makes
/// exactly one `tools/list` request before its call, and an entry
/// carrying no `match` is consumed by whatever request arrives next —
/// so a tool list left between two keyed calls would be served to the
/// call and the beat would read a tool list as an answer.
fn judge_lane_entries(answers: &[serde_json::Value], hashes: &[String]) -> Vec<serde_json::Value> {
    let handshake = serde_json::json!({ "reply": {
        "jsonrpc": "2.0", "id": "$echo",
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "sys1", "version": "0.1.0" },
        },
    } });
    let tool_list = serde_json::json!({ "reply": {
        "jsonrpc": "2.0", "id": "$echo",
        "result": { "tools": [{ "name": "sys1_classify", "description": "one classify",
                                "inputSchema": { "type": "object" } }] },
    } });
    let mut entries = vec![handshake, tool_list];
    for (answer, hash) in answers.iter().zip(hashes.iter()) {
        entries.push(serde_json::json!({
            "match": format!("input hash: {hash}"),
            "reply": {
            "jsonrpc": "2.0", "id": "$echo",
            "result": { "content": [{ "type": "text", "text": answer.to_string() }],
                        "isError": false },
        } }));
    }
    entries
}

/// One `input_hash` per ask: the route fixtures' own `{index:032x}`.
fn judge_hashes(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("{index:032x}")).collect()
}

/// One `sys1_classify` answer over the route task: a workload label with
/// its confidence, and the ask's cost.
fn sys1_answer(label: &str, confidence: f64) -> serde_json::Value {
    serde_json::json!({
        "task": "route",
        "verdict": "pass",
        "provider": "glide",
        "providers_used": ["glide"],
        "answers": { "glide": {
            "workload": { "label": label, "confidence": confidence },
            "execution": { "label": "single", "confidence": 0.8 },
        } },
        "latency_ms": 900,
        "cost_usd": 0.0002,
    })
}

/// The provider the preference table maps `hard` to today — derived,
/// never named. The judge grades a route row against this same table,
/// so a corpus built from it can never disagree with the grader, and
/// when the owner's table moves (a different provider preferred for
/// `hard`) the corpus moves with it instead of breaking. The fallback
/// is the declared head, which is what the judge itself falls back to
/// for an unmapped workload.
fn hard_target() -> &'static str {
    route::workload_candidate("hard").unwrap_or(route::HEAD_ID)
}

/// The ten route rows the scenario's day holds: eight judged routes to
/// the `hard` workload's target in the preference table and two judged
/// routes to the declared head, with the first a full walk.
fn calibration_day_rows() -> Vec<serde_json::Value> {
    let hard = hard_target();
    (0..10u32)
        .map(|index| {
            // The eight `hard` rows are served by the table's target;
            // the two `quick` rows are served by the head. The lane
            // re-judges every row as `hard`, so the quick rows grade
            // wrong — the required candidate never served on them.
            let (provider, workload) = if index < 8 {
                (hard, "hard")
            } else {
                (route::HEAD_ID, "quick")
            };
            let attempts: Vec<(&str, &str, &str)> = if index == 0 {
                vec![(route::HEAD_ID, "failed", "auth"), (hard, "served", "")]
            } else {
                vec![(provider, "served", "")]
            };
            route_row_line(
                &format!("2026-10-14T12:00:{index:02}Z"),
                provider,
                workload,
                &attempts,
                &format!("{index:032x}"),
            )
        })
        .collect()
}

/// Write `rows` as one day's stream file under `root`.
fn write_day_file(root: &Path, day: &str, rows: &[serde_json::Value]) {
    let path = calib::day_path(root, day);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text = rows.iter().map(|row| row.to_string()).collect::<Vec<_>>().join("\n");
    std::fs::write(&path, format!("{text}\n")).unwrap();
}

/// The three roster candidates the ride-back's checks walk.
fn calibration_roster() -> Vec<route::Resolved> {
    let candidate = |id: &str, base_url: &str, key_env: &str, models: &[&str]| route::Resolved {
        candidate: route::Candidate {
            id: id.to_string(),
            base_url: base_url.to_string(),
            key_env: key_env.to_string(),
            models: models.iter().map(|model| model.to_string()).collect(),
            billing: route::Billing::Plan,
            kind: route::CandidateKind::Http,
            enabled: true,
            key_required: false,
            disabled_reason: None,
        },
        key: None,
    };
    vec![
        candidate("router-kit", "http://127.0.0.1:8300/v1", "AGNOSTIC_ROUTER_TOKEN", &["auto"]),
        candidate("zai-coding-plan", "https://api.z.ai", "ZAI_CODING_API_KEY", &["GLM-5.3-Flash"]),
        candidate(
            "xiaomi-mimo",
            "https://token-plan-sgp.xiaomimimo.com/v1",
            "XIAOMI_MIMO_API_KEY",
            &["mimo-v2.6-flash"],
        ),
    ]
}

/// The calibration loop, graded end to end through scripted seams: a
/// day's route rows are judged beside themselves, the fit rule trusts a
/// sample only when it carries rows and span, the conservative
/// ride-back skips a fit-and-wrong provider while leaving an unfit one
/// alone, the third vector's insufficient state counts apart and moves
/// nothing, the dry run spends nothing, and a lane that fails open is
/// refused by name. No paid call: the lane is the scripted MCP child,
/// the same seam the blend1 and explain scenarios drive.
async fn calibration_loop_closes(_mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "calibration-loop-closes";
    let family = ScenarioFamily::Calibration;
    let Some(mcp_bin) = mock_mcp_binary() else {
        return ScenarioOutcome::fail(
            NAME,
            family,
            "the mock-mcp binary was not found beside the fleet executable; the lane seam cannot run",
        );
    };
    // Today (UTC), because the beat stamps its rows with the real clock:
    // a hardcoded day would stop matching the rows the judge writes at a
    // UTC midnight, and the scenario would fail for a reason that has
    // nothing to do with the loop.
    let day = calib::today();
    let (home, root) = match scratch_dir("calib-home")
        .and_then(|home| scratch_dir("calib-root").map(|root| (home, root)))
    {
        Ok(pair) => pair,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let day_rows = calibration_day_rows();
    write_day_file(&root, &day, &day_rows);
    let rows = calib::read_day(&root, &day, 64);
    let route_rows: Vec<&calib::ReadRow> =
        rows.iter().filter(|row| row.op == calib::OP_ROUTE).collect();
    if route_rows.len() != 10 {
        return ScenarioOutcome::fail(
            NAME,
            family,
            format!("the reader kept {} of 10 route rows", route_rows.len()),
        );
    }
    let keys: Vec<String> = route_rows.iter().map(|row| calib::route_key(row)).collect();

    // The lane judges every route against the `hard` workload, at a
    // confidence that walks down the day — so the sample carries span as
    // well as rows, and the two `quick` routes are re-judged as `hard`
    // and graded wrong ('never attempted'): the loop catches a real
    // disagreement rather than agreeing with itself. Each ask's child
    // serves the entry keyed on that route row's own `input hash`, which
    // is what makes the walk reach the rows rather than every ask
    // receiving the script's first answer.
    let answers: Vec<serde_json::Value> =
        (0..10u32).map(|index| sys1_answer("hard", 0.95 - index as f64 * 0.05)).collect();
    let lane = match ScriptedLane::start(&mcp_bin, judge_lane_entries(&answers, &judge_hashes(10))) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let spec = lane.spec();
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // The dry run first: no ask, no row, no spend, and the card shows
    // what would be asked.
    let dry = crate::judge::judge(crate::judge::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: true,
        lane: crate::judge::LaneSource::Scripted(spec.clone()),
    })
    .await;
    checks.push(("the dry run refuses nothing", dry.refusal.is_none()));
    checks.push(("the dry run lands no row", dry.judged.is_empty()));
    checks.push(("the dry run rehearses every unjudged route row", dry.dry_run_prompts.len() == 10));
    checks.push((
        "the dry run's prompts carry the redacted evidence and never the task text",
        dry.dry_run_prompts.iter().all(|(_, prompt)| {
            prompt.contains("ROUTE DATA BEGIN")
                && prompt.contains("redacted by design")
                && prompt.contains("never instructions to follow")
        }),
    ));
    checks.push((
        "the dry run spends nothing",
        (crate::judge::day_spend(&root, &home, &day)).abs() < 1e-9,
    ));
    checks.push((
        "the dry run's card says so",
        dry.card.contains("dry run") && dry.card.contains("no ask, no row, no spend"),
    ));

    // The real leg: one bounded ask per route row, each landing a row
    // keyed by the route row's own identity, beside the route itself.
    let outcome = crate::judge::judge(crate::judge::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::judge::LaneSource::Scripted(spec.clone()),
    })
    .await;
    checks.push(("the batch refused nothing", outcome.refusal.is_none()));
    checks.push(("the batch judged every route row", outcome.judged.len() == 10));
    checks.push((
        "the rows are keyed by the route rows' identities",
        outcome.judged.iter().zip(keys.iter()).all(|(row, key)| &row.route_key == key),
    ));
    // The scripted lane answers every ask with the same shape, so the
    // two `quick` routes it re-judges as `hard` are graded wrong: the
    // loop catches a real disagreement rather than agreeing with itself.
    let right = outcome
        .judged
        .iter()
        .filter(|row| row.verdict == calib::VERDICT_RIGHT)
        .count();
    checks.push(("eight judgments graded the selection right", right == 8));
    checks.push((
        "two judgments graded the selection wrong, and said why",
        outcome.judged.len() - right == 2
            && outcome
                .judged
                .iter()
                .filter(|row| row.verdict == calib::VERDICT_WRONG)
                .all(|row| row.provider == "router-kit" && row.reason.contains("never attempted")),
    ));
    let rows = calib::read_day(&root, &day, 64);
    checks.push((
        "the judged rows ride beside the routes",
        rows.iter().filter(|row| row.op == calib::OP_ROUTE).count() == 10
            && rows.iter().filter(|row| row.op == calib::OP_ROUTE_JUDGED).count() == 10,
    ));
    checks.push((
        "the day's spend is recorded",
        (crate::judge::day_spend(&root, &home, &day) - 0.002).abs() < 1e-6,
    ));
    // A second batch is idempotent: it spends nothing and refuses by
    // name rather than re-asking.
    let again = crate::judge::judge(crate::judge::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::judge::LaneSource::Scripted(spec.clone()),
    })
    .await;
    checks.push((
        "a second batch spends nothing on a route it already judged",
        again.judged.is_empty()
            && again.refusals.len() == 10
            && again.refusals.iter().all(|(_, reason)| reason.contains("already judged")),
    ));
    drop(lane);

    // The fit rule: the `hard` target carries eight rows spanning 0.35,
    // so it is fit; the head carries two, so it is not — and an unfit
    // provider renders no score.
    let cells = calib::calibration(&rows);
    let hard_cell = calib::cell(&cells, hard_target(), "hard");
    let kit = calib::cell(&cells, route::HEAD_ID, "quick");
    checks.push(("the calibration names both cells", hard_cell.is_some() && kit.is_some()));
    let (Some(hard_cell), Some(kit)) = (hard_cell, kit) else {
        return grade(NAME, family, &checks);
    };
    checks.push(("the eight-row cell counts eight rows", hard_cell.judged == 8));
    checks.push(("the two-row cell counts two rows", kit.judged == 2));
    // The scripted seam answers each ask at its own walked confidence,
    // keyed on the route row's input hash — so the eight `hard` rows
    // span 0.35 (0.95 down to 0.60) and the cell is FIT. The walk is
    // pinned here rather than assumed: without the key every ask would
    // receive the script's first answer, the rows would all read 0.95,
    // the span would be zero, and the fit leg would grade an accident.
    checks.push((
        "the walked confidence reaches the rows the beat wrote",
        outcome.judged.iter().enumerate().all(|(index, row)| {
            (row.confidence - (0.95 - index as f64 * 0.05)).abs() < 1e-9
        }),
    ));
    checks.push((
        "the eight-row cell is fit on its span and needs no more of it",
        hard_cell.fit == calib::FitState::Fit
            && (hard_cell.confidence_span - 0.35).abs() < 1e-9
            && hard_cell.needs_span() == 0.0,
    ));
    checks.push((
        "the two-row cell names what it needs and draws no score",
        kit.needs_rows() > 0
            && calib::render_calibration(&root, &day, true).contains("no score is drawn"),
    ));
    checks.push((
        "an unfit cell informs nothing however wrong it looks",
        !hard_cell.informs_failover() && !kit.informs_failover(),
    ));

    // The conservative ride-back: a fit-and-wrong provider is skipped in
    // the preference; an unfit one never is, however wrong it looks;
    // and the selection is never skipped.
    let wrong_root = match scratch_dir("calib-wrong") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    // The split the owner's directive names: zai is WRONG for `hard` and
    // RIGHT for `prose`, both on fit samples. The two cells are separate
    // records, and each moves only its own shape's walk.
    let split_rows: Vec<serde_json::Value> = (0..8u32)
        .map(|index| {
            judged_row_line(
                &format!("{day}T13:00:{index:02}Z"),
                &format!("wrong-key-{index:016x}"),
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
            )
        })
        .chain((0..8u32).map(|index| {
            judged_row_line(
                &format!("{day}T13:30:{index:02}Z"),
                &format!("right-key-{index:016x}"),
                "zai-coding-plan",
                "prose",
                calib::VERDICT_RIGHT,
                0.95 - index as f64 * 0.05,
            )
        }))
        .collect();
    write_day_file(&wrong_root, &day, &split_rows);
    let roster = calibration_roster();
    let hard_walk = route::calibration_skips(&wrong_root, &day, &roster, "router-kit", "hard");
    checks.push((
        "a fit-and-wrong cell is skipped in the preference",
        hard_walk.len() == 1
            && hard_walk[0].provider == "zai-coding-plan"
            && hard_walk[0].workload == "hard",
    ));
    checks.push((
        "the same provider's right-for-prose cell skips nothing for prose",
        route::calibration_skips(&wrong_root, &day, &roster, "router-kit", "prose").is_empty(),
    ));
    checks.push((
        "a shape the day holds no cell for moves nothing",
        route::calibration_skips(&wrong_root, &day, &roster, "router-kit", "quick").is_empty(),
    ));
    checks.push((
        "the selected provider is never skipped",
        route::calibration_skips(&wrong_root, &day, &roster, "zai-coding-plan", "hard").is_empty(),
    ));
    let unfit_root = match scratch_dir("calib-unfit") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    // An unfit cell: wrong at full confidence, but one row. The raw rate
    // is 1.0 and it still moves nothing, because the fit requirement
    // outranks the number.
    write_day_file(
        &unfit_root,
        &day,
        &[judged_row_line(
            &format!("{day}T14:00:00Z"),
            "unfit-key-0000000000000001",
            "zai-coding-plan",
            "hard",
            calib::VERDICT_WRONG,
            0.95,
        )],
    );
    checks.push((
        "an unfit cell never moves routing however wrong it looks",
        route::calibration_skips(&unfit_root, &day, &roster, "router-kit", "hard").is_empty(),
    ));
    // The span requirement's own fixture: eight rows — enough rows — all
    // at ONE confidence. The rows requirement is satisfied and the cell
    // is still unfit, because a flat sample carries no information about
    // how sure the lane was. The beat's own batch cannot build this
    // shape any more (its lane walks), so it is stated directly.
    let flat_root = match scratch_dir("calib-flat") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let flat_rows: Vec<serde_json::Value> = (0..8u32)
        .map(|index| {
            judged_row_line(
                &format!("{day}T14:30:{index:02}Z"),
                &format!("flat-key-{index:016x}"),
                "zai-coding-plan",
                "hard",
                calib::VERDICT_WRONG,
                0.9,
            )
        })
        .collect();
    write_day_file(&flat_root, &day, &flat_rows);
    let flat_day = calib::read_day(&flat_root, &day, 64);
    let flat_cells = calib::calibration(&flat_day);
    checks.push((
        "a flat sample is unfit however many rows it carries",
        calib::cell(&flat_cells, "zai-coding-plan", "hard").is_some_and(|cell| {
            cell.judged == 8
                && cell.fit == calib::FitState::Unfit
                && cell.needs_span() > 0.0
                && calib::render_calibration(&flat_root, &day, true).contains("no score is drawn")
        }),
    ));
    // A provider outside the roster is never skipped either: the
    // calibration names the day's cells, and the walk only skips what it
    // could otherwise attempt.
    checks.push((
        "a provider the roster does not carry is never skipped",
        route::calibration_skips(&wrong_root, &day, &roster[..1], "router-kit", "hard").is_empty(),
    ));
    // The cells are separate records on the surface, and neither hides
    // the other.
    let cells = calib::calibration(&calib::read_day(&wrong_root, &day, 64));
    checks.push((
        "the surface renders the split as two cells",
        cells.len() == 2
            && calib::cell(&cells, "zai-coding-plan", "hard")
                .is_some_and(|cell| cell.informs_failover())
            && calib::cell(&cells, "zai-coding-plan", "prose")
                .is_some_and(|cell| !cell.informs_failover()),
    ));

    // The third vector's leg: a day whose every route row's required
    // candidate failed on an availability class while a companion
    // served. The direction was right every time and the minima was not
    // reached, so the batch grades INSUFFICIENT — the cell keeps the
    // rate apart from the wrong-rate, the failover skips nothing, and
    // the route row's reason names the companions the minima needs.
    let open_root_for_third = match scratch_dir("calib-third") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let third_rows: Vec<serde_json::Value> = (0..8u32)
        .map(|index| {
            route_row_line(
                &format!("{day}T15:00:{index:02}Z"),
                "xiaomi-mimo",
                "hard",
                &[
                    ("router-kit", "failed", "auth"),
                    (hard_target(), "failed", "rate-limit"),
                    ("xiaomi-mimo", "served", ""),
                ],
                &format!("{index:032x}"),
            )
        })
        .collect();
    write_day_file(&open_root_for_third, &day, &third_rows);
    let third_answers: Vec<serde_json::Value> =
        (0..8u32).map(|index| sys1_answer("hard", 0.95 - index as f64 * 0.05)).collect();
    let third_lane =
        match ScriptedLane::start(&mcp_bin, judge_lane_entries(&third_answers, &judge_hashes(8))) {
            Ok(lane) => lane,
            Err(error) => return ScenarioOutcome::fail(NAME, family, error),
        };
    let third = crate::judge::judge(crate::judge::JudgeRequest {
        root: &open_root_for_third,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::judge::LaneSource::Scripted(third_lane.spec()),
    })
    .await;
    checks.push((
        "a failover that followed the served turn grades insufficient, not wrong",
        third.judged.len() == 8
            && third
                .judged
                .iter()
                .all(|row| row.verdict == calib::VERDICT_INSUFFICIENT),
    ));
    checks.push((
        "the insufficient reason names the companion that was absent",
        third.judged.iter().all(|row| {
            row.reason.contains("direction was right") && row.reason.contains("want of a companion")
        }),
    ));
    // The cell is read from THE BEAT'S OWN ROWS: the day file is not
    // rewritten with a hand-made fixture, so the chain beat → row →
    // cell → ride-back runs end to end for the third vector. The
    // previous shape truncated the file with a fixture and graded the
    // fixture, which meant the beat's confidence never reached the cell
    // math at all.
    let third_day = calib::read_day(&open_root_for_third, &day, 64);
    let third_cells = calib::calibration(&third_day);
    let mimo = calib::cell(&third_cells, "xiaomi-mimo", "hard");
    checks.push((
        "the insufficient cell counts apart from the wrongs and the rights",
        mimo.is_some_and(|cell| {
            cell.judged == 8
                && cell.insufficient == 8
                && cell.wrong == 0
                && cell.right == 0
                && cell.fit == calib::FitState::Fit
                && cell.weighted_insufficient_rate >= calib::INSUFFICIENT_RATE_FLOOR
                && cell.weighted_wrong_rate.abs() < 1e-9
        }),
    ));
    checks.push((
        "an insufficient cell informs nothing however high its rate",
        mimo.is_some_and(|cell| !cell.informs_failover() && cell.notes_companions()),
    ));
    checks.push((
        "the failover skips nothing on an insufficient cell",
        route::calibration_skips(&open_root_for_third, &day, &roster, "router-kit", "hard")
            .is_empty(),
    ));
    let notes = route::calibration_notes(&open_root_for_third, &day, &roster, "router-kit", "hard");
    checks.push((
        "the route row's reason names the companions the minima needs",
        notes.len() == 1
            && notes[0].provider == "xiaomi-mimo"
            && route::calibration_note_clause(&notes[0]).contains("the minima needs companions")
            && route::calibration_note_clause(&notes[0]).contains("is not skipped"),
    ));
    drop(third_lane);

    // The fail-open leg: the lane's own fallback shape is a named
    // refusal, not a judgment — no row, and the recorded decisions
    // stand. A fresh root holds the same ten route rows unjudged, so
    // the refusals are the fail-open's rather than the idempotence's.
    let open_root = match scratch_dir("calib-open") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    write_day_file(&open_root, &day, &day_rows);
    let fallback = blend1_result(serde_json::json!({
        "task": "route",
        "verdict": "error",
        "provider": null,
        "providers_used": [],
        "answers": {},
        "telemetry": { "glide": { "error": "FASTINO_API_KEY is not set",
                                  "error_kind": "provider" } },
        "reason": "FASTINO_API_KEY is not set",
    }));
    let fail_lane = match ScriptedLane::start(
        &mcp_bin,
        judge_lane_entries(&vec![fallback; 10], &judge_hashes(10)),
    ) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let fail = crate::judge::judge(crate::judge::JudgeRequest {
        root: &open_root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::judge::LaneSource::Scripted(fail_lane.spec()),
    })
    .await;
    checks.push((
        "a lane that fails open is refused by name, not shipped as a judgment",
        fail.judged.is_empty()
            && fail.refusals.len() == 10
            && fail.refusals.iter().all(|(_, reason)| reason.contains("failed open")),
    ));
    checks.push((
        "the fail-open refusal carries the lane's own error",
        fail.refusals.iter().all(|(_, reason)| reason.contains("FASTINO_API_KEY")),
    ));
    checks.push((
        "the fail-open lands no row and spends nothing",
        calib::read_day(&open_root, &day, 64)
            .iter()
            .filter(|row| row.op == calib::OP_ROUTE_JUDGED)
            .count()
            == 0
            && (crate::judge::day_spend(&open_root, &home, &day)).abs() < 1e-9,
    ));

    // The human seam: the owner's disposition rides a later row for the
    // same key and overrides the verdict in the math.
    let seam_key = keys[0].clone();
    match crate::judge::disposition(&root, &day, &seam_key, calib::DISPOSITION_DISAGREED) {
        Ok(_) => checks.push(("the owner's disposition lands", true)),
        Err(reason) => {
            checks.push(("the owner's disposition lands", false));
            eprintln!("{NAME}: the disposition refused: {reason}");
        }
    }
    let freshest = calib::judged_rows(&calib::read_day(&root, &day, 64));
    checks.push((
        "the disposition is the freshest row for its key and leaves the verdict on the record",
        freshest.len() == 10
            && freshest
                .iter()
                .find(|row| row.route_key == seam_key)
                .is_some_and(|row| {
                    row.disposition == calib::DISPOSITION_DISAGREED
                        && row.verdict == calib::VERDICT_RIGHT
                }),
    ));

    for scratch in [
        &home,
        &root,
        &wrong_root,
        &unfit_root,
        &flat_root,
        &open_root,
        &open_root_for_third,
    ] {
        let _ = std::fs::remove_dir_all(scratch);
    }
    grade(NAME, family, &checks)
}

/// The tool message one recorded request carried: the lane's answer as
/// the model read it (the fourth message — system, user, assistant,
/// tool).
fn tool_message(mock: &MockApi, index: usize) -> String {
    mock.bodies()
        .get(index)
        .and_then(|body| body.get("messages"))
        .and_then(|messages| messages.as_array())
        .and_then(|messages| messages.get(3))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .unwrap_or_default()
        .to_string()
}

/// What one blend1 lane run produced: the pieces every blend1 scenario
/// grades against.
struct Blend1Run {
    report: RunReport,
    /// The lane's answer as the model read it.
    lane_answer: String,
    /// The assistant's tool call, as the log recorded it.
    call_name: String,
    call_arguments: String,
    /// The `lane_tools` record's model-visible names, in declaration
    /// order.
    lane_tools: Vec<String>,
    /// The reconstructed log.
    records: Vec<Record>,
}

/// One blend1 lane run against a scripted seam: the mock API's two
/// scripted turns (the model's call, then its answer) and the scripted
/// lane child. Every fault — a missing binary, a scratch dir that will
/// not make, a session that will not create, a run that fails — is a
/// named error the scenario grades, never a panic.
async fn run_blend1(
    mock_bin: &Path,
    tool_name: &str,
    arguments: &str,
    lane_reply: &str,
    answer: &str,
    task: &str,
) -> Result<Blend1Run, String> {
    let Some(mcp_bin) = mock_mcp_binary() else {
        return Err(
            "the mock-mcp binary was not found beside the fleet executable; the lane seam cannot run"
                .to_string(),
        );
    };
    let lane = ScriptedLane::start(&mcp_bin, blend1_lane_entries(lane_reply))?;
    let mock = MockApi::start(
        mock_bin,
        vec![
            tool_call_stream("call-1", tool_name, arguments),
            assistant_stream(answer),
        ],
    )?;
    let home = scratch_dir("blend1-turn")?;
    let mut session = Session::create(&home, "test-model")
        .map_err(|error| format!("the session could not be created: {error}"))?;
    let mut specs = std::collections::HashMap::new();
    specs.insert(BLEND1.to_string(), lane.spec());
    let report = drive_over(
        &mock,
        &mut session,
        Box::new(ApproveAll),
        5,
        None,
        Some(specs),
        task,
    )
    .await
    .map_err(|error| format!("the run failed: {error}"))?;
    let records = session::replay(&session.path()).unwrap_or_default();
    let lane_answer = tool_message(&mock, 1);
    let call = records.iter().find_map(|record| match record {
        Record::Assistant { tool_calls, .. } => tool_calls.first().map(|call| {
            (call.function.name.clone(), call.function.arguments.clone())
        }),
        _ => None,
    });
    let lane_tools = records
        .iter()
        .find_map(|record| match record {
            Record::LaneTools { lane, tools } if lane == BLEND1 => {
                Some(tools.iter().map(|tool| tool.name.clone()).collect())
            }
            _ => None,
        })
        .unwrap_or_default();
    let run = Blend1Run {
        report,
        lane_answer,
        call_name: call.as_ref().map(|(name, _)| name.clone()).unwrap_or_default(),
        call_arguments: call.as_ref().map(|(_, arguments)| arguments.clone()).unwrap_or_default(),
        lane_tools,
        records,
    };
    std::fs::remove_dir_all(&home).ok();
    Ok(run)
}

/// The grounded path: the model asks the lane for a tabular fact, the
/// lane's answer carries TabPFN's computed probability, and the final
/// answer carries it **unaltered** — never invented, never rounded.
/// "LLMs do not do math; TabPFN does."
async fn blend1_grounded_fact(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "blend1-grounded-fact";
    let family = ScenarioFamily::GroundedLane;
    let answer = format!(
        "Customer 1042 will churn. TabPFN scores the row at p={FACT} against 40 labeled \
         context rows; the semantic lane's closest neighbor is the churn playbook. \
         The probability is {FACT} exactly, as the tabular lane computed it."
    );
    let run = match run_blend1(
        &mock_bin,
        "mcp__blend1__ask",
        ASK_ARGUMENTS,
        &blend1_result(serde_json::json!({
            "generated": { "status": "ok", "model": "lfm2.5-24b-a2b", "text": answer },
            "validation": { "status": "ok", "provider": "glide", "passed": true,
                            "outcome": "passed", "number_check": "ok" },
            "score": { "status": "ok", "provider": "liquid", "correct": true,
                       "reprocessed": false, "shipped": "original" },
            "final_answer": answer,
        }))
        .to_string(),
        &answer,
        "will customer 1042 churn? ground it in the tabular lane",
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let final_answer = run.report.final_message.clone().unwrap_or_default();
    // Every metric-shaped rendering of the fact in the answer is the
    // exact one the lane computed: a rounded 0.92 at a word boundary, or
    // a bare percentage, is the fabrication the number check exists for.
    let rounded = ["0.92,", "0.92 ", "0.92.", "0.92)", "92%", "0.9 "]
        .iter()
        .any(|shape| final_answer.contains(shape));
    grade(
        NAME,
        family,
        &[
            ("the run completed", run.report.stop_reason == StopReason::Completed),
            ("two turns", run.report.turns == 2),
            (
                "blend1 joined the run with its five tools",
                run.lane_tools
                    == vec![
                        "mcp__blend1__ask".to_string(),
                        "mcp__blend1__doctor".to_string(),
                        "mcp__blend1__search".to_string(),
                        "mcp__blend1__refresh".to_string(),
                        "mcp__blend1__seed".to_string(),
                    ],
            ),
            (
                "the model asked the lane for the grounded path",
                run.call_name == "mcp__blend1__ask"
                    && run.call_arguments.contains("\"table\"")
                    && run.call_arguments.contains("\"target\":\"churned\""),
            ),
            ("the tabular fact reached the model's context", run.lane_answer.contains(FACT)),
            ("the final answer carries the fact", final_answer.contains(FACT)),
            ("the fact was not rounded", !rounded),
            (
                "the grounded path names itself and did not escalate",
                run.lane_answer.contains("\"path\":\"synthesized\"")
                    && run.lane_answer.contains("\"escalated\":false"),
            ),
            ("the log reconstructs the turn", run.records.len() == 7),
        ],
    )
}

/// The escalation: the validation gate lands inconclusive (`passed`
/// null — no critic answered), the answer escalates to a larger
/// configured model with the same givens, and the escalated answer ships
/// **flagged** under the result's own `escalated_answer` key. The gate's
/// unresolved verdict is itself what sets `escalated` (the core computes
/// the gate as `not passed`, and `None` is not truthy), so the two keys
/// **always co-occur** on this path — `escalated_answer` is what says the
/// shipped answer is the escalation's rather than the pre-escalation
/// one. The gate never blocks: the pre-escalation answer is retained in
/// the result, and an escalation that is unconfigured, keyless, or
/// failed is named with the original answer shipping.
async fn blend1_escalation(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "blend1-escalation";
    let family = ScenarioFamily::GroundedLane;
    let original = "Customer 1042 will probably churn; the tabular lane scored the row.";
    let escalated = format!(
        "Customer 1042 will churn. TabPFN scores the row at p={FACT} against 40 labeled \
         context rows, and the churn playbook is the closest semantic neighbor."
    );
    let run = match run_blend1(
        &mock_bin,
        "mcp__blend1__ask",
        ASK_ARGUMENTS,
        &blend1_result(serde_json::json!({
            "generated": { "status": "ok", "model": "lfm2.5-24b-a2b", "text": original },
            "validation": { "status": "ok", "provider": "glide", "answers_the_question": 0.71,
                            "contradicts_facts": 0.28, "number_check": "ok",
                            "passed": null, "outcome": "inconclusive" },
            "escalation": { "status": "ok", "provider": "escalation-target",
                            "model": "qwen3.8-flash", "text": escalated, "shipped": true },
            "escalated_answer": true,
            // The real contract: the unresolved gate sets `escalated`
            // too — the core reads the gate as failed when `passed` is
            // not truthy, and `None` is not truthy.
            "escalated": true,
            "final_answer": escalated,
        }))
        .to_string(),
        &escalated,
        "will customer 1042 churn? the gate could not rate the first answer",
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let final_answer = run.report.final_message.clone().unwrap_or_default();
    grade(
        NAME,
        family,
        &[
            ("the run completed", run.report.stop_reason == StopReason::Completed),
            ("two turns", run.report.turns == 2),
            (
                "the validation outcome is named inconclusive",
                run.lane_answer.contains("\"outcome\":\"inconclusive\"")
                    && run.lane_answer.contains("\"passed\":null"),
            ),
            (
                "the escalation leg fired and shipped",
                run.lane_answer.contains("\"escalation\"")
                    && run.lane_answer.contains("\"shipped\":true"),
            ),
            ("the escalated answer shipped", final_answer == escalated),
            (
                "the shipped answer is flagged on its own key",
                run.lane_answer.contains("\"escalated_answer\":true"),
            ),
            (
                // The real contract, pinned: the escalation fires only on
                // an unresolved gate, and the gate's unresolved verdict is
                // itself what sets `escalated` — so the two keys always
                // co-occur. `escalated_answer` is what says the SHIPPED
                // answer is the escalation's.
                "the unresolved gate and the escalated answer both flag",
                run.lane_answer.contains("\"escalated\":true")
                    && run.lane_answer.contains("\"escalated_answer\":true"),
            ),
            (
                "the pre-escalation answer is retained, not silently dropped",
                run.lane_answer.contains(original),
            ),
            (
                "the escalated answer still carries the computed fact",
                final_answer.contains(FACT),
            ),
            ("the log reconstructs the turn", run.records.len() == 7),
        ],
    )
}

/// The reprocess: sys1 (d1) scores the generated answer's correctness
/// against the givens, a wrong score reprocesses (re-synthesis with the
/// score's feedback), and the corrected answer ships — the score carrying
/// `reprocessed` and which answer shipped, and the wrong answer retained
/// in the result rather than dropped. The score never blocks: a
/// reprocess that fails ships the scored answer with the failure named.
async fn blend1_reprocess(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "blend1-reprocess";
    let family = ScenarioFamily::GroundedLane;
    let wrong = "Customer 1042 will churn; the tabular lane puts the probability at p=0.87.";
    let corrected = format!(
        "Customer 1042 will churn. TabPFN puts the probability at p={FACT} against 40 \
         labeled context rows; the first attempt's figure was wrong and was reprocessed."
    );
    let run = match run_blend1(
        &mock_bin,
        "mcp__blend1__ask",
        ASK_ARGUMENTS,
        &blend1_result(serde_json::json!({
            "generated": { "status": "ok", "model": "lfm2.5-24b-a2b", "text": wrong },
            "score": { "status": "ok", "provider": "liquid", "is_correct": 0.12,
                       "correctness": 1, "correct": false,
                       "reprocessed": true, "shipped": "reprocessed" },
            "final_answer": corrected,
        }))
        .to_string(),
        &corrected,
        "will customer 1042 churn? the first answer got the number wrong",
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let final_answer = run.report.final_message.clone().unwrap_or_default();
    grade(
        NAME,
        family,
        &[
            ("the run completed", run.report.stop_reason == StopReason::Completed),
            ("two turns", run.report.turns == 2),
            (
                "the sys1 score names the first answer wrong",
                run.lane_answer.contains("\"score\"")
                    && run.lane_answer.contains("\"correct\":false"),
            ),
            (
                "the reprocess fired and is bounded",
                run.lane_answer.contains("\"reprocessed\":true")
                    && run.lane_answer.contains("\"shipped\":\"reprocessed\""),
            ),
            ("the corrected answer shipped", final_answer == corrected),
            (
                "the corrected answer carries the computed fact and not the wrong one",
                final_answer.contains(FACT) && !final_answer.contains("0.87"),
            ),
            (
                "the wrong answer is retained in the result, not dropped",
                run.lane_answer.contains(wrong),
            ),
            ("the log reconstructs the turn", run.records.len() == 7),
        ],
    )
}

/// The corpus recall: a seeded project fact is retrieved by `search` and
/// quoted **unaltered** — the project's facts, not the agent's durable
/// ones, and the seam the cron refresh keeps current.
async fn blend1_corpus_recall(mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "blend1-corpus-recall";
    let family = ScenarioFamily::GroundedLane;
    let fact = "the churn playbook's threshold is p=0.90, reviewed 2026-10-09";
    let search_reply = serde_json::json!({
        "project": "fleet",
        "query": "what is the churn playbook's threshold?",
        "k": 3,
        "results": [{ "key": "churn-playbook", "score": 0.771, "text": fact }],
    })
    .to_string();
    let answer = format!(
        "The project's churn playbook sets the threshold at p=0.90 (\"{fact}\"), \
         retrieved from the fleet corpus at cosine 0.771."
    );
    let run = match run_blend1(
        &mock_bin,
        "mcp__blend1__search",
        r#"{"query":"what is the churn playbook's threshold?","project":"fleet","k":3}"#,
        &search_reply,
        &answer,
        "what is the churn playbook's threshold? recall it from the project corpus",
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let final_answer = run.report.final_message.clone().unwrap_or_default();
    grade(
        NAME,
        family,
        &[
            ("the run completed", run.report.stop_reason == StopReason::Completed),
            ("two turns", run.report.turns == 2),
            (
                "the model asked the project's corpus",
                run.call_name == "mcp__blend1__search"
                    && run.call_arguments.contains("\"project\":\"fleet\""),
            ),
            ("the seeded fact reached the model's context", run.lane_answer.contains(fact)),
            ("the final answer quotes the fact unaltered", final_answer.contains(fact)),
            ("the log reconstructs the turn", run.records.len() == 7),
        ],
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
        assert_eq!(ScenarioFamily::Explain.label(), "explain");
        assert_eq!(ScenarioFamily::GroundedLane.label(), "grounded-lane");
    }

    #[test]
    fn the_corpus_identity_is_stable_and_names_every_scenario() {
        // The judged artifact an `eval_corpus` row carries: the corpus's
        // own fixed definition, hashed. Stable across calls, 16 hex
        // characters, and different from any other corpus's — the join
        // key an explanation attaches to.
        let first = corpus_identity();
        assert_eq!(first, corpus_identity());
        assert_eq!(first.len(), 16);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, crate::calib::hash16("fleet-eval-corpus"));
        // Every scenario in the corpus rides the identity, so a corpus
        // that gains or loses a scenario changes its own artifact.
        let mut text = format!("fleet-eval-corpus/{}", env!("CARGO_PKG_VERSION"));
        for scenario in corpus() {
            text.push('\n');
            text.push_str(scenario.name);
            text.push(' ');
            text.push_str(scenario.family.label());
        }
        assert_eq!(first, crate::calib::hash16(&text));
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
                "memory-shape-inversion",
                "memory-spillover-routes",
                "memory-failure-shape-recall",
                "memory-phrases-round-trip",
                "blend1-grounded-fact",
                "blend1-escalation",
                "blend1-reprocess",
                "blend1-corpus-recall",
                "explain-the-verdict",
                "calibration-loop-closes",
            ]
        );
    }

    /// The memory family is shape-first recall's own: the recall that
    /// enters a turn, the inversion the partition proves, the
    /// spillover's three routes, the failure axis's recall, and the
    /// phrases' round trip.
    #[test]
    fn the_memory_scenarios_are_the_shape_first_recall_family() {
        let memory: Vec<&str> = corpus()
            .iter()
            .filter(|scenario| scenario.family == ScenarioFamily::Memory)
            .map(|scenario| scenario.name)
            .collect();
        assert_eq!(
            memory,
            vec![
                "memory-recall",
                "memory-shape-inversion",
                "memory-spillover-routes",
                "memory-failure-shape-recall",
                "memory-phrases-round-trip",
            ]
        );
    }

    /// The four blend1 scenarios share one family: they all grade a lane
    /// call whose result must survive the trip into the answer.
    #[test]
    fn the_blend1_scenarios_are_the_grounded_lane_family() {
        let grounded: Vec<&str> = corpus()
            .iter()
            .filter(|scenario| scenario.family == ScenarioFamily::GroundedLane)
            .map(|scenario| scenario.name)
            .collect();
        assert_eq!(
            grounded,
            vec![
                "blend1-grounded-fact",
                "blend1-escalation",
                "blend1-reprocess",
                "blend1-corpus-recall",
            ]
        );
    }

    /// The scripted lane's `tools/list` carries the adapter's five tools
    /// in declaration order, so a scenario's `lane_tools` assertion names
    /// what the lane really declares.
    #[test]
    fn the_scripted_lane_declares_the_adapters_five_tools() {
        let names: Vec<String> = blend1_tools()
            .as_array()
            .expect("a tools array")
            .iter()
            .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(names, vec!["ask", "doctor", "search", "refresh", "seed"]);
    }

    /// The BlendResult fixture is additive: an extra key rides in beside
    /// the wire contract's own without disturbing them, which is what
    /// lets a leg added upstream be scripted before it is vendored.
    #[test]
    fn an_extra_result_key_rides_in_beside_the_contracts_own() {
        let base = blend1_result(serde_json::json!({}));
        // The contract's own keys, including the escalation and score legs.
        assert_eq!(base["path"], "synthesized");
        assert_eq!(base["escalated"], false);
        assert_eq!(base["escalated_answer"], false);
        assert_eq!(base["score"], serde_json::json!({}));
        assert!(base.get("decompose").is_none());
        let extended = blend1_result(serde_json::json!({ "decompose": { "status": "ok" } }));
        // The contract's own keys are untouched.
        assert_eq!(extended["path"], "synthesized");
        assert_eq!(extended["escalated_answer"], false);
        // And the new one rides beside them.
        assert_eq!(extended["decompose"]["status"], "ok");
    }

    /// The scripted lane's frames are well-formed: three entries, each a
    /// reply whose id echoes the request's, so any request order serves.
    #[test]
    fn the_scripted_lane_answers_initialize_list_and_one_call() {
        let entries = blend1_lane_entries("{\"ok\":true}");
        assert_eq!(entries.len(), 3);
        for entry in &entries {
            assert_eq!(entry["reply"]["id"], "$echo");
        }
        assert!(entries[1]["reply"]["result"]["tools"].is_array());
        assert_eq!(entries[2]["reply"]["result"]["content"][0]["text"], "{\"ok\":true}");
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
