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

use crate::agent::{
    Agent, AgentError, AgentOptions, ApproveAll, Approver, FileAccess, RunReport, StopReason,
};
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
    ScriptedResponse {
        status: "200 OK".to_string(),
        body: String::new(),
        sse: frames,
        delay_ms: 0,
    }
}

fn content_delta(text: &str) -> String {
    format!(
        "data: {}",
        json!({ "choices": [{ "delta": { "content": text } }] })
    )
}

fn tool_call_delta(index: usize, id: &str, name: &str, arguments: &str) -> String {
    format!(
        "data: {}",
        json!({ "choices": [{ "delta": { "tool_calls": [{ "index": index, "id": id,
            "type": "function", "function": { "name": name, "arguments": arguments } }] } }] })
    )
}

fn finish_frame(reason: &str) -> String {
    format!(
        "data: {}",
        json!({ "choices": [{ "delta": {}, "finish_reason": reason }] })
    )
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
        let text = std::fs::read_to_string(self.scratch.join("requests.jsonl")).unwrap_or_default();
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
                        message
                            .get("role")
                            .and_then(|role| role.as_str())
                            .map(str::to_string)
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
    Ok(MockApi {
        child,
        port,
        scratch: scratch.to_path_buf(),
    })
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
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
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
        let script = entries
            .iter()
            .map(|entry| format!("{entry}\n"))
            .collect::<String>();
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
            args: vec![self
                .scratch
                .join("script.jsonl")
                .to_string_lossy()
                .into_owned()],
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
    /// The delegation loop: the judgment against the child's own
    /// session log, its discipline, the (provider × brief-shape) cells,
    /// and the conservative ride-back (slice 20's D2).
    Delegation,
    /// A lane call whose result must carry a computed fact into the final
    /// answer unaltered — the grounded path blend1 exists for.
    GroundedLane,
    /// The nomenclature guard: the derived vocabulary, the lookup's
    /// four states, the write-time flag and refusal, and the drift
    /// signal a delegation's own log carries (slice 20's D7).
    Nomenclature,
    /// The improve loop's own record: the lesson store's write, its
    /// evidence refusal, and its surface (the adopt-on-contrast plan's
    /// P3).
    Improve,
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
            ScenarioFamily::Delegation => "delegation",
            ScenarioFamily::GroundedLane => "grounded-lane",
            ScenarioFamily::Nomenclature => "nomenclature",
            ScenarioFamily::Improve => "improve",
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
        Scenario {
            name,
            family,
            run: Box::new(run),
        }
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
        Scenario::new(
            "session-corpus-replay",
            ScenarioFamily::SessionCorpus,
            |bin| Box::pin(session_corpus_replay(bin)),
        ),
        Scenario::new(
            "agent-continuation",
            ScenarioFamily::AgentContinuation,
            |bin| Box::pin(agent_continuation(bin)),
        ),
        Scenario::new(
            "conversation-fold",
            ScenarioFamily::ConversationFold,
            |bin| Box::pin(conversation_fold(bin)),
        ),
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
        Scenario::new(
            "memory-failure-shape-recall",
            ScenarioFamily::Memory,
            |bin| Box::pin(memory_failure_shape_recall(bin)),
        ),
        Scenario::new("memory-phrases-round-trip", ScenarioFamily::Memory, |bin| {
            Box::pin(memory_phrases_round_trip(bin))
        }),
        Scenario::new(
            "blend1-grounded-fact",
            ScenarioFamily::GroundedLane,
            |bin| Box::pin(blend1_grounded_fact(bin)),
        ),
        Scenario::new("blend1-escalation", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_escalation(bin))
        }),
        Scenario::new("blend1-reprocess", ScenarioFamily::GroundedLane, |bin| {
            Box::pin(blend1_reprocess(bin))
        }),
        Scenario::new(
            "blend1-corpus-recall",
            ScenarioFamily::GroundedLane,
            |bin| Box::pin(blend1_corpus_recall(bin)),
        ),
        Scenario::new("explain-the-verdict", ScenarioFamily::Explain, |bin| {
            Box::pin(explain_the_verdict(bin))
        }),
        Scenario::new(
            "calibration-loop-closes",
            ScenarioFamily::Calibration,
            |bin| Box::pin(calibration_loop_closes(bin)),
        ),
        Scenario::new(
            "delegation-loop-closes",
            ScenarioFamily::Delegation,
            |bin| Box::pin(delegation_loop_closes(bin)),
        ),
        Scenario::new("nomenclature-guard", ScenarioFamily::Nomenclature, |bin| {
            Box::pin(nomenclature_guard(bin))
        }),
        Scenario::new("dependency-graph", ScenarioFamily::Nomenclature, |bin| {
            Box::pin(dependency_graph(bin))
        }),
        Scenario::new("delta-replan", ScenarioFamily::Nomenclature, |bin| {
            Box::pin(delta_replan(bin))
        }),
        Scenario::new("improve-lesson-store", ScenarioFamily::Improve, |bin| {
            Box::pin(improve_lesson_store(bin))
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
        self.scenarios
            .iter()
            .filter(|scenario| scenario.passed)
            .count()
    }

    /// The scenarios that failed, named, in corpus order.
    pub fn failed(&self) -> Vec<&ScenarioOutcome> {
        self.scenarios
            .iter()
            .filter(|scenario| !scenario.passed)
            .collect()
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
        self.failed()
            .into_iter()
            .map(|scenario| scenario.scenario.clone())
            .collect()
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
    rows.into_iter()
        .rev()
        .find(|row| row.op == calib::OP_EVAL_CORPUS)
}

/// The most recent `eval_corpus` row the stream holds, across days —
/// the baseline an apply's contrast is measured against. Today first
/// (the common case), then the days under the stream root, newest
/// first, so a machine whose last corpus run was yesterday still has a
/// baseline. `None` when the stream has never recorded one; the apply
/// then names "no baseline" rather than inventing a comparison.
pub fn latest_row(root: &Path) -> Option<calib::ReadRow> {
    if let Some(row) = last_row(root) {
        return Some(row);
    }
    let mut days: Vec<String> = Vec::new();
    let Ok(years) = std::fs::read_dir(root) else {
        return None;
    };
    for year in years.flatten() {
        let year_path = year.path();
        let Some(year_name) = year_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if year_name.len() != 4 || !year_name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(months) = std::fs::read_dir(&year_path) else {
            continue;
        };
        for month in months.flatten() {
            let month_path = month.path();
            let Some(month_name) = month_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if month_name.len() != 2 || !month_name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Ok(day_dirs) = std::fs::read_dir(&month_path) else {
                continue;
            };
            for day in day_dirs.flatten() {
                let day_path = day.path();
                let Some(day_name) = day_path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if day_name.len() != 2 || !day_name.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                days.push(format!("{year_name}-{month_name}-{day_name}"));
            }
        }
    }
    days.sort();
    for day in days.iter().rev() {
        let rows = calib::read_day(root, day, calib::READ_CAP);
        if let Some(row) = rows
            .into_iter()
            .rev()
            .find(|row| row.op == calib::OP_EVAL_CORPUS)
        {
            return Some(row);
        }
    }
    None
}

// --- the improve-gate probe ----------------------------------------------

/// What one probe run found — the corpus's own verdict, never a bare
/// green. `Err` from an [`EvalProbe`] is reserved for "could not be
/// measured": a child that never resolved, never ran, timed out, or
/// died without printing its verdict. The distinction is the rule the
/// contrast rests on — a judge failure is `None`, never a zero — so an
/// unmeasured run can never read as a regression.
#[derive(Debug, Clone, PartialEq)]
pub enum ProbeOutcome {
    /// The corpus ran and every scenario passed.
    Green { score: f64 },
    /// The corpus ran and named scenarios failed.
    Regressed { score: f64, scenarios: Vec<String> },
}

impl ProbeOutcome {
    /// The aggregate score the run carried.
    pub fn score(&self) -> f64 {
        match self {
            ProbeOutcome::Green { score } | ProbeOutcome::Regressed { score, .. } => *score,
        }
    }

    /// Whether every scenario passed.
    pub fn green(&self) -> bool {
        matches!(self, ProbeOutcome::Green { .. })
    }
}

/// The eval-corpus gate probe the improve gate runs: the corpus's
/// outcome when it ran, or the reason it could not be measured. The
/// probe never returns a bare ok — the contrast needs the score, and
/// "unmeasured" needs its own channel. `Send + Sync` so it can ride the
/// `Apply` context and be shared; a test supplies a stand-in.
pub type EvalProbe = Arc<dyn Fn() -> Result<ProbeOutcome, String> + Send + Sync>;

/// The bound the probe's `fleet eval` subprocess runs under: the
/// corpus is twenty-one deterministic scenarios across twelve families,
/// each graded headless against the mock API and, on the lane legs,
/// over a scripted MCP child — so a child that has not finished
/// within this many seconds is a hung probe — killed and named,
/// never a gate that hangs with it.
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
/// the time the eval step runs), and return the corpus's outcome —
/// green or regressed, with the score the contrast needs. A
/// subprocess rather than an in-process run is also what keeps the
/// gate off the async runtime it is called from — the gate is a sync
/// context, and a `block_on` there would panic (KI-021).
///
/// The probe **fails closed**. When the current executable does not
/// resolve, or resolves outside the gate root's `target/` directory —
/// an installed or stale binary, which would grade the wrong build
/// and read as a false green — the probe refuses naming why, and the
/// gate records the `eval_corpus` step failed: a source self-edit is
/// never waved through by an eval step that did not run. The
/// subprocess is bounded by [`PROBE_TIMEOUT_SECS`], so a hung child
/// is a named failure rather than a hung gate. A refusal is `Err` —
/// "could not be measured" — and the gate keeps it distinct from a
/// measured regression.
pub fn subprocess_probe(gate_root: &Path) -> EvalProbe {
    let target_root = gate_root.join("target");
    // The binary the probe will grade: the current executable, but
    // only when it lives under the gate root's target/ directory —
    // the freshly-built binary the four cargo steps produced.
    let resolved = std::env::current_exe()
        .ok()
        .filter(|exe| under_target(exe, &target_root));
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
    let root = target_root
        .canonicalize()
        .unwrap_or_else(|_| target_root.to_path_buf());
    let exe = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    exe.starts_with(&root)
}

/// Grade one `fleet eval` subprocess's output into the probe's outcome.
/// `fleet eval` always prints one `eval-score: <n>` footer line, and
/// one `regressions: <a>, <b>` line when it is red. Three shapes leave
/// this function: a green run with its score, a red run with its score
/// and the named scenarios, and `Err` when the child never printed a
/// verdict at all — a crash, a kill, or a nonzero exit with no
/// `regressions` line. The last is "could not be measured", never
/// "regressed": the misattribution the eval module's own docstring
/// names for exit 101.
fn grade_output(output: &std::process::Output) -> Result<ProbeOutcome, String> {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let score = text
        .lines()
        .find_map(|line| line.split_once("eval-score: "))
        .and_then(|(_, value)| strip_ansi(value).trim().parse::<f64>().ok());
    let named: Option<Vec<String>> = text
        .lines()
        .find_map(|line| line.split_once("regressions: "))
        .map(|(_, names)| strip_ansi(names).trim().to_string())
        .filter(|names| !names.is_empty())
        .map(|names| {
            names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|names| !names.is_empty());
    match (score, named) {
        (Some(score), None) if output.status.success() => Ok(ProbeOutcome::Green { score }),
        (Some(score), Some(scenarios)) => Ok(ProbeOutcome::Regressed { score, scenarios }),
        _ => Err(format!(
            "the eval corpus could not be measured: the probe exited {} without printing its verdict (run `fleet eval` for the detail)",
            exit_word(&output.status)
        )),
    }
}

/// How a subprocess ended, for a named refusal: the exit code, or the
/// fact that a signal ended it.
fn exit_word(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("with code {code}"),
        None => "on a signal".to_string(),
    }
}

/// Run `fleet eval` as a bounded subprocess and collect its output. A
/// reader thread drains both pipes, so a child that fills its pipe
/// buffer cannot deadlock the wait; the parent keeps the child so it
/// can kill it when the bound expires. The corpus is twenty
/// deterministic scenarios across twelve families — a child that has
/// not finished within `timeout` is a hung probe, not a hung gate.
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
            Ok(Some(status)) => {
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                })
            }
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
    // The one machine-readable line the improve gate's subprocess probe
    // reads back: the run's score, on every run, green or red. The
    // contrast is only meaningful if the probe never has to guess.
    out.push_str(&format!("{DIM}eval-score: {:.2}{RESET}\n", report.score()));
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
    let mut lines = vec![format!(
        "{DIM}\u{2502}{RESET} {DIM}{}{RESET}",
        scenario.family
    )];
    if let Some(failure) = &scenario.failure {
        lines.push(format!("{DIM}\u{2502}{RESET} {WARN}{failure}{RESET}"));
    }
    let bottom = format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}",
        "\u{2500}".repeat(CARD_WIDTH - 2)
    );
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
        format!(
            "the scratch dir {} could not be created: {error}",
            dir.display()
        )
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
        goal: None,
        model: "test-model".to_string(),
        max_turns,
        identity: None,
        ui_tx: None,
        lane_specs,
        calib: None,
        skills: Vec::new(),
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
        role_overlay: None,
        memory,
        scope: None,
        improve_home: None,
        route_home: None,
        gui_prompt: None,
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
            // The stop contract: a report, not an acknowledgment. The
            // harness asked once for it before this line existed, and
            // the corpus now pins the shape the contract wants.
            assistant_stream(
                "The command printed hello (exit 0). The tool loop is complete; nothing needed from you.",
            ),
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
            let outcome = drive(
                &mock,
                &mut session,
                Box::new(ApproveAll),
                5,
                None,
                "say hello",
            )
            .await;
            let records = session::replay(session.path()).unwrap_or_default();
            // The third record is the assistant's call, read through a
            // checked access: a short log grades the check failed
            // rather than panicking on the index.
            let assistant_call = records.get(3).is_some_and(|record| {
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
                            (
                                "the run completed",
                                report.stop_reason == StopReason::Completed,
                            ),
                            ("two turns", report.turns == 2),
                            (
                                "the final answer",
                                report.final_message.as_deref()
                                    == Some(
                                        "The command printed hello (exit 0). The tool loop is \
                                         complete; nothing needed from you.",
                                    ),
                            ),
                            ("both requests streamed with usage", bodies.len() == 2),
                            (
                                "the second request saw the tool result",
                                mock.roles(1) == vec!["system", "user", "assistant", "tool"],
                            ),
                            ("the log reconstructs the turn", records.len() == 8),
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
            let outcome = drive(
                &mock,
                &mut session,
                Box::new(ApproveAll),
                5,
                None,
                "replay me",
            )
            .await;
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(_) => {
                    let records = session::replay(session.path()).unwrap_or_default();
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
                                .filter_map(|message| serde_json::from_value(message.clone()).ok())
                                .collect()
                        })
                        .unwrap_or_default();
                    grade(
                        NAME,
                        family,
                        &[
                            ("the log reconstructs", records.len() == 8),
                            (
                                "the replayed conversation matches the request",
                                replayed == sent,
                            ),
                            (
                                "the reconstructed assistant carries its call",
                                replayed
                                    .iter()
                                    .any(|message| !message.tool_calls.is_empty()),
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
            let first = drive(
                &mock,
                &mut session,
                Box::new(ApproveAll),
                5,
                None,
                "first task",
            )
            .await;
            let log_path = session.path().to_path_buf();
            match first {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the first run failed: {error}"))
                }
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
                                                "system",
                                                "user",
                                                "assistant",
                                                "tool",
                                                "assistant",
                                                "user",
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
    // The post-fold stop is a report too: a run that folded its early
    // turns still owes the owner the result, the status, and the next
    // step.
    script.push(assistant_stream(
        "The early turns were folded into the summary above; the conversation continues from it and nothing needed from you.",
    ));
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
    let result =
        match create_session(&home, NAME, family) {
            Ok(mut session) => {
                let outcome = drive(
                    &mock,
                    &mut session,
                    Box::new(ApproveAll),
                    20,
                    None,
                    "the task",
                )
                .await;
                let records = session::replay(session.path()).unwrap_or_default();
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
            let outcome = drive(
                &mock,
                &mut session,
                Box::new(ApproveAll),
                5,
                None,
                "delegate the count",
            )
            .await;
            match outcome {
                Err(error) => {
                    ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                }
                Ok(_) => {
                    let parent_records = session::replay(session.path()).unwrap_or_default();
                    let summaries = session::list(&home).unwrap_or_default();
                    let children: Vec<&session::SessionSummary> = summaries
                        .iter()
                        .filter(|summary| summary.kind.as_deref() == Some("subagent"))
                        .collect();
                    // The parent's log carries the delegation call and its result.
                    let subagent_result = parent_records.iter().find_map(|record| match record {
                        Record::ToolResult {
                            name,
                            output,
                            approved,
                            ..
                        } if name == "subagent" => Some((output.clone(), *approved)),
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
                            (
                                "the parent's log carries the delegation result",
                                delegation_ok,
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
        fact_file(
            "identity",
            "the owner's standing rule about the gate",
            "confirmed-by",
            "feedback",
            "",
        ),
    )
    .and_then(|()| {
        std::fs::write(
            corpus.join("top.md"),
            fact_file(
                "top",
                "the railway deploy is failing per branch",
                "inferred",
                "project",
                "deploy",
            ),
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
                    let records = session::replay(session.path()).unwrap_or_default();
                    match outcome {
                        Err(error) => {
                            ScenarioOutcome::fail(NAME, family, format!("the run failed: {error}"))
                        }
                        Ok(_) => {
                            let system = mock.system(0);
                            let memory_record = records.iter().find_map(|record| match record {
                                Record::Memory { shape, facts } => {
                                    Some((shape.clone(), facts.clone()))
                                }
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
                                    (
                                        "the request carried the memory block",
                                        system.contains("## Memory"),
                                    ),
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
    std::fs::create_dir_all(root).map_err(|error| {
        format!(
            "the scratch dir {} could not be made: {error}",
            root.display()
        )
    })?;
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
                fact_file(
                    "same-shape",
                    "the branch pipeline is green",
                    "confirmed-by",
                    "project",
                    "deploy",
                ),
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
    let wrong = partition
        .spillover
        .first()
        .expect("the spillover has its fact");
    let ranked = memory.recall("the railway deploy is failing");
    let order = hit_names(&ranked);
    std::fs::remove_dir_all(&corpus).ok();
    grade(
        NAME,
        family,
        &[
            (
                "the instruction classifies into the shape",
                partition.shape == Some("deploy"),
            ),
            (
                "the same-shape fact is the primary pool",
                hit_names(&partition.primary) == ["same-shape"],
            ),
            (
                "the wrong-shape fact is spillover",
                hit_names(&partition.spillover) == ["wrong-shape"],
            ),
            (
                "the word evidence is on the wrong-shape side",
                wrong.score.total > same.score.total,
            ),
            (
                "the shape's own answer leads the ranking",
                order == ["same-shape", "wrong-shape"],
            ),
            (
                "the off-shape fact is marked",
                !same.off_shape && wrong.off_shape,
            ),
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
            fact_file(
                "off-one",
                "the railway deploy is failing per branch",
                "confirmed-by",
                "project",
                "rendering",
            ),
        )],
    ) {
        Ok(corpus) => corpus,
        Err(failure) => return failure,
    };
    let empty_pool = Memory::load_with(&route_a, Box::new(memory::KeywordOnly));
    let empty = empty_pool.assemble_within("the railway deploy is failing", &[], 1);
    checks.push((
        "route (a): the off-shape fact is the answer",
        hit_names(&empty.recalled) == ["off-one"],
    ));
    checks.push((
        "route (a): it is marked off-shape",
        empty.recalled.first().is_some_and(|hit| hit.off_shape),
    ));
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
                fact_file(
                    "same-shape",
                    "the branch pipeline is green",
                    "confirmed-by",
                    "project",
                    "deploy",
                ),
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
        weak.spillover_route
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
                fact_file(
                    "off-shape-fact",
                    "the railway deploy is failing",
                    "confirmed-by",
                    "project",
                    "rendering",
                ),
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
    checks.push((
        "the floor's other side: the pool clears it",
        best >= crate::memory::SPILLOVER_FLOOR,
    ));
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
    let block =
        crate::memory::render_assembly(&memory.assemble("the row vanished from the stream", &[]));
    let signs: Vec<crate::memory::Sign> =
        failure.primary.iter().map(|hit| hit.fact.sign()).collect();
    std::fs::remove_dir_all(&corpus).ok();
    grade(
        NAME,
        family,
        &[
            (
                "the instruction classifies into the failure shape",
                failure.shape == Some("drops-the-row"),
            ),
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
                rendering
                    .primary
                    .iter()
                    .any(|hit| hit.fact.name == "both-axes")
                    && bound.primary.iter().any(|hit| hit.fact.name == "both-axes"),
            ),
            (
                "the legend names every sign",
                block.contains("resolved:")
                    && block.contains("diverged:")
                    && block.contains("incomplete:"),
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
                    &[
                        "read CLAUDE.local.md directly",
                        "never grep-extract credentials",
                    ],
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
            (
                "the phrase-carrying fact is the only hit",
                ranked == ["deploy-credentials-rule"],
            ),
            (
                "the trajectory channel matched it",
                (trajectory - 0.625).abs() < 1e-9,
            ),
            (
                // 0.25, not the 0.125 the unstemmed tokenizer read: the
                // question's `credentials` and the fact's `credential`
                // are one stem now (R3). Still barely — two of the
                // question's eight stems, and neither is the fact's
                // direction, which is the point the scenario makes.
                "the topical channels barely did",
                (keyword - 0.25).abs() < 1e-9,
            ),
            ("it is the shape's own pool", primary),
            (
                "the twin with no direction is absent",
                !ranked.contains(&"deploy-credentials-twin"),
            ),
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
    let (home, root) = match scratch_dir("explain-home")
        .and_then(|home| scratch_dir("explain-calib").map(|root| (home, root)))
    {
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
    let change =
        r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#;
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
            calib::GateStepRow {
                step: "fmt".to_string(),
                ok: true,
                detail: "ok".to_string(),
            },
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
        !row_text.contains(EXPLANATION)
            && !row_text.contains("GATE OP")
            && !row_text.contains("ARTIFACT"),
    ));
    checks.push(("the row carries the eval-only marker", row.eval_only));
    checks.push((
        "the row joins on the artifact's sha",
        row.input_sha256 == artifact_sha,
    ));
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
    checks.push((
        "the dry run spends nothing",
        dry.row.is_none() && dry.refusal.is_none(),
    ));
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
        fail.refusal
            .as_deref()
            .is_some_and(|reason| reason.contains("LIQUID_API_KEY")),
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
    let text = rows
        .iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n");
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
        candidate(
            "deepseek",
            "https://api.deepseek.com",
            "DEEPSEEK_API_KEY",
            &["deepseek-flash"],
        ),
        candidate(
            "zai-coding-plan",
            "https://api.z.ai",
            "ZAI_CODING_API_KEY",
            &["GLM-5.3-Flash"],
        ),
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
    let route_rows: Vec<&calib::ReadRow> = rows
        .iter()
        .filter(|row| row.op == calib::OP_ROUTE)
        .collect();
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
    let answers: Vec<serde_json::Value> = (0..10u32)
        .map(|index| sys1_answer("hard", 0.95 - index as f64 * 0.05))
        .collect();
    let lane = match ScriptedLane::start(&mcp_bin, judge_lane_entries(&answers, &judge_hashes(10)))
    {
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
    checks.push((
        "the dry run rehearses every unjudged route row",
        dry.dry_run_prompts.len() == 10,
    ));
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
    checks.push((
        "the batch judged every route row",
        outcome.judged.len() == 10,
    ));
    checks.push((
        "the rows are keyed by the route rows' identities",
        outcome
            .judged
            .iter()
            .zip(keys.iter())
            .all(|(row, key)| &row.route_key == key),
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
                .all(|row| {
                    row.provider == route::HEAD_ID && row.reason.contains("never attempted")
                }),
    ));
    let rows = calib::read_day(&root, &day, 64);
    checks.push((
        "the judged rows ride beside the routes",
        rows.iter().filter(|row| row.op == calib::OP_ROUTE).count() == 10
            && rows
                .iter()
                .filter(|row| row.op == calib::OP_ROUTE_JUDGED)
                .count()
                == 10,
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
            && again
                .refusals
                .iter()
                .all(|(_, reason)| reason.contains("already judged")),
    ));
    drop(lane);

    // The fit rule: the `hard` target carries eight rows spanning 0.35,
    // so it is fit; the head carries two, so it is not — and an unfit
    // provider renders no score.
    let cells = calib::calibration(&rows);
    let hard_cell = calib::cell(&cells, hard_target(), "hard");
    let kit = calib::cell(&cells, route::HEAD_ID, "quick");
    checks.push((
        "the calibration names both cells",
        hard_cell.is_some() && kit.is_some(),
    ));
    let (Some(hard_cell), Some(kit)) = (hard_cell, kit) else {
        return grade(NAME, family, &checks);
    };
    checks.push((
        "the eight-row cell counts eight rows",
        hard_cell.judged == 8,
    ));
    checks.push(("the two-row cell counts two rows", kit.judged == 2));
    // The scripted seam answers each ask at its own walked confidence,
    // keyed on the route row's input hash — so the eight `hard` rows
    // span 0.35 (0.95 down to 0.60) and the cell is FIT. The walk is
    // pinned here rather than assumed: without the key every ask would
    // receive the script's first answer, the rows would all read 0.95,
    // the span would be zero, and the fit leg would grade an accident.
    checks.push((
        "the walked confidence reaches the rows the beat wrote",
        outcome
            .judged
            .iter()
            .enumerate()
            .all(|(index, row)| (row.confidence - (0.95 - index as f64 * 0.05)).abs() < 1e-9),
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
    let hard_walk = route::calibration_skips(&wrong_root, &day, &roster, "deepseek", "hard");
    checks.push((
        "a fit-and-wrong cell is skipped in the preference",
        hard_walk.len() == 1
            && hard_walk[0].provider == "zai-coding-plan"
            && hard_walk[0].workload == "hard",
    ));
    checks.push((
        "the same provider's right-for-prose cell skips nothing for prose",
        route::calibration_skips(&wrong_root, &day, &roster, "deepseek", "prose").is_empty(),
    ));
    checks.push((
        "a shape the day holds no cell for moves nothing",
        route::calibration_skips(&wrong_root, &day, &roster, "deepseek", "quick").is_empty(),
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
        route::calibration_skips(&unfit_root, &day, &roster, "deepseek", "hard").is_empty(),
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
        route::calibration_skips(&wrong_root, &day, &roster[..1], "deepseek", "hard").is_empty(),
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
                    ("deepseek", "failed", "auth"),
                    (hard_target(), "failed", "rate-limit"),
                    ("xiaomi-mimo", "served", ""),
                ],
                &format!("{index:032x}"),
            )
        })
        .collect();
    write_day_file(&open_root_for_third, &day, &third_rows);
    let third_answers: Vec<serde_json::Value> = (0..8u32)
        .map(|index| sys1_answer("hard", 0.95 - index as f64 * 0.05))
        .collect();
    let third_lane = match ScriptedLane::start(
        &mcp_bin,
        judge_lane_entries(&third_answers, &judge_hashes(8)),
    ) {
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
        route::calibration_skips(&open_root_for_third, &day, &roster, "deepseek", "hard")
            .is_empty(),
    ));
    let notes = route::calibration_notes(&open_root_for_third, &day, &roster, "deepseek", "hard");
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
            && fail
                .refusals
                .iter()
                .all(|(_, reason)| reason.contains("failed open")),
    ));
    checks.push((
        "the fail-open refusal carries the lane's own error",
        fail.refusals
            .iter()
            .all(|(_, reason)| reason.contains("FASTINO_API_KEY")),
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

// --- the delegation loop (slice 20's D2) -----------------------------------

/// One delegation row in the stream, the shape `fleet delegate` reads:
/// the child's session id is the key, the brief's shape and the serving
/// provider are the cell key, and the parent chain rides because a
/// chain cannot be backfilled honestly.
fn delegation_row_line(
    session: &str,
    brief_shape: &str,
    provider: &str,
    outcome: &str,
    turns: u64,
    log: &str,
    root_session: &str,
) -> serde_json::Value {
    serde_json::json!({
        "ts": "2026-10-14T12:00:00Z",
        "op": "delegation",
        "session": session,
        "brief_shape": brief_shape,
        "scope": "bash,read_file,write_file,list_dir",
        "provider": provider,
        "outcome": outcome,
        "turns": turns,
        "spend_usd": 0.0123,
        "duration_ms": 812,
        "log": log,
        "parent_session": "parent-session",
        "root_session": root_session,
        "depth": 1,
    })
}

/// One child's own session log — the evidence the beat reads — written
/// the way a real run leaves it: the start record, one assistant turn
/// carrying its tool calls, and the ending the log recorded. The
/// ending and the turns are what the grade's mechanical read joins the
/// row against.
fn child_log(path: &std::path::Path, ending: &str, turns: u64, wrote: &[&str], commands: u64) {
    let mut calls: Vec<serde_json::Value> = Vec::new();
    for (index, written) in wrote.iter().enumerate() {
        calls.push(serde_json::json!({
            "id": format!("call-{index}"),
            "type": "function",
            "function": {
                "name": "write_file",
                "arguments": serde_json::json!({ "path": written, "content": "x" }).to_string(),
            },
        }));
    }
    for index in 0..commands {
        calls.push(serde_json::json!({
            "id": format!("cmd-{index}"),
            "type": "function",
            "function": { "name": "bash", "arguments": serde_json::json!({ "command": "echo hi" }).to_string() },
        }));
    }
    let lines = [
        serde_json::json!({ "type": "start", "v": 1, "model": "test-model",
            "created": "2026-10-14T12:00:00Z", "kind": "subagent" })
        .to_string(),
        serde_json::json!({ "type": "assistant", "content": "working", "tool_calls": calls })
            .to_string(),
        serde_json::json!({ "type": "end", "stop_reason": ending, "turns": turns }).to_string(),
    ];
    std::fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

/// One `sys1_classify` answer over the delegation beat: the verdict
/// word, the confidence that weights it, and the lane's own reason. The
/// same answer shape the routing beat reads — one lane answer serves
/// both beats, which is the loop reused rather than forked.
fn delegate_answer(verdict: &str, confidence: f64, reason: &str) -> serde_json::Value {
    serde_json::json!({
        "task": "route",
        "verdict": verdict,
        "confidence": confidence,
        "reason": reason,
        "provider": "glide",
        "providers_used": ["glide"],
        "answers": { "glide": { "workload": { "label": "hard", "confidence": confidence } } },
        "latency_ms": 700,
        "cost_usd": 0.0002,
    })
}

/// The scripted lane's entries for the delegation beat: the handshake,
/// the tool list declaring `sys1_classify`, then one answer keyed on
/// the row's own root session id — which rides the composed prompt,
/// because the child's own summary text never does (the rows are
/// redacted by design). Keyed, so each ask is answered with its own
/// row's judgment rather than the script's first answer.
fn delegate_lane_entries(answers: &[(String, serde_json::Value)]) -> Vec<serde_json::Value> {
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
    for (root_session, answer) in answers {
        entries.push(serde_json::json!({
            "match": format!("root session {root_session}"),
            "reply": {
                "jsonrpc": "2.0", "id": "$echo",
                "result": { "content": [{ "type": "text", "text": answer.to_string() }],
                            "isError": false },
            },
        }));
    }
    entries
}

/// One delegation row the scenario's day holds, beside its child's own
/// log and the lane's answer for it.
struct DaySpec {
    session: &'static str,
    shape: &'static str,
    provider: &'static str,
    outcome: &'static str,
    turns: u64,
    ending: &'static str,
    log_turns: u64,
    wrote: &'static [&'static str],
    commands: u64,
    verdict: &'static str,
    confidence: f64,
    reason: &'static str,
}

/// The five delegation rows the scenario's day holds: two the logs
/// agree with, one the log contradicts, one that stopped at its cap,
/// and one the lane escalates. Each row's own evidence decides its
/// grade, which is the loop catching real disagreement rather than
/// agreeing with itself.
fn delegation_day_specs() -> Vec<DaySpec> {
    vec![
        DaySpec {
            session: "sess-r1",
            shape: "refactor",
            provider: "stepfun",
            outcome: "completed",
            turns: 3,
            ending: "completed",
            log_turns: 3,
            wrote: &["/tmp/a.rs"],
            commands: 1,
            verdict: "pass",
            confidence: 0.91,
            reason: "",
        },
        DaySpec {
            session: "sess-r2",
            shape: "refactor",
            provider: "stepfun",
            outcome: "completed",
            turns: 2,
            ending: "completed",
            log_turns: 2,
            wrote: &["/tmp/b.rs"],
            commands: 1,
            verdict: "pass",
            confidence: 0.83,
            reason: "",
        },
        DaySpec {
            // The row claims `completed`; the child's own log recorded
            // `failed` at fewer turns — the contradiction signal, and
            // the lane's `pass` cannot rescue it.
            session: "sess-w1",
            shape: "refactor",
            provider: "zai-coding-plan",
            outcome: "completed",
            turns: 4,
            ending: "failed",
            log_turns: 2,
            wrote: &[],
            commands: 1,
            verdict: "pass",
            confidence: 0.70,
            reason: "",
        },
        DaySpec {
            // The child stopped at its cap with the direction right:
            // the companion the minima needed was a longer turn budget,
            // named.
            session: "sess-i1",
            shape: "triage",
            provider: "stepfun",
            outcome: "max-turns",
            turns: 5,
            ending: "max-turns",
            log_turns: 5,
            wrote: &[],
            commands: 1,
            verdict: "pass",
            confidence: 0.88,
            reason: "",
        },
        DaySpec {
            // The log agrees and the lane escalates: the direction was
            // right, the answer needed more than the delegation
            // reached, and the companion is whatever the lane named.
            session: "sess-i2",
            shape: "triage",
            provider: "xiaomi-mimo",
            outcome: "completed",
            turns: 2,
            ending: "completed",
            log_turns: 2,
            wrote: &[],
            commands: 1,
            verdict: "escalate",
            confidence: 0.79,
            reason: "the answer needed the escalation leg",
        },
    ]
}

/// Write the specs' delegation rows and their children's logs into
/// `root`, returning the rows.
fn write_delegation_day(root: &std::path::Path, specs: &[DaySpec]) -> Vec<serde_json::Value> {
    let logs = root.join("sessions");
    std::fs::create_dir_all(&logs).unwrap();
    specs
        .iter()
        .map(|spec| {
            let log = logs.join(format!("{}.jsonl", spec.session));
            child_log(&log, spec.ending, spec.log_turns, spec.wrote, spec.commands);
            delegation_row_line(
                spec.session,
                spec.shape,
                spec.provider,
                spec.outcome,
                spec.turns,
                &log.to_string_lossy(),
                spec.session,
            )
        })
        .collect()
}

/// The delegation loop, graded end to end through scripted seams: the
/// day's delegation rows are judged against their outcome evidence —
/// the children's own session logs — the three verdicts carry their
/// signals, the cells key on (provider × brief shape) with the fit
/// rule's readiness, the conservative ride-back moves a fit-and-wrong
/// cell and nothing else, the dry run spends nothing, a lane that fails
/// open is refused by name, and the owner's disposition inverts. No
/// paid call: the lane is the scripted MCP child, the same seam the
/// calibration scenario drives.
async fn delegation_loop_closes(_mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "delegation-loop-closes";
    let family = ScenarioFamily::Delegation;
    let Some(mcp_bin) = mock_mcp_binary() else {
        return ScenarioOutcome::fail(
            NAME,
            family,
            "the mock-mcp binary was not found beside the fleet executable; the lane seam cannot run",
        );
    };
    let day = calib::today();
    let (home, root) = match scratch_dir("delegate-home")
        .and_then(|home| scratch_dir("delegate-root").map(|root| (home, root)))
    {
        Ok(pair) => pair,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let specs = delegation_day_specs();
    let mut rows = write_delegation_day(&root, &specs);
    // One already-judged delegation: the stream carries its judgment,
    // so the batch neither re-asks nor lands a second row.
    let judged_key = "sess-j1";
    let judged_log = root.join("sessions").join(format!("{judged_key}.jsonl"));
    child_log(&judged_log, "completed", 2, &[], 1);
    rows.push(delegation_row_line(
        judged_key,
        "refactor",
        "stepfun",
        "completed",
        2,
        &judged_log.to_string_lossy(),
        judged_key,
    ));
    rows.push(serde_json::json!({
        "ts": "2026-10-14T12:30:00Z",
        "op": "delegation_judged",
        "session": judged_key,
        "provider": "stepfun",
        "brief_shape": "refactor",
        "verdict": calib::VERDICT_RIGHT,
        "confidence": 0.9,
        "reason": "judged earlier",
        "lane": "sys1",
    }));
    write_day_file(&root, &day, &rows);
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // The dry run first, over a fresh root carrying the same five
    // unjudged rows: no ask, no row, no spend, and the prompts carry
    // the redacted evidence and never the child's summary text.
    let dry_root = match scratch_dir("delegate-dry") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let dry_rows = write_delegation_day(&dry_root, &specs);
    write_day_file(&dry_root, &day, &dry_rows);
    let lane = match ScriptedLane::start(&mcp_bin, delegate_lane_entries(&[])) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let dry = crate::delegate::judge(crate::delegate::JudgeRequest {
        root: &dry_root,
        home: &home,
        day: day.clone(),
        dry_run: true,
        lane: crate::delegate::LaneSource::Scripted(lane.spec()),
    })
    .await;
    checks.push(("the dry run refuses nothing", dry.refusal.is_none()));
    checks.push(("the dry run lands no row", dry.judged.is_empty()));
    checks.push((
        "the dry run rehearses every unjudged row",
        dry.dry_run_prompts.len() == 5,
    ));
    checks.push((
        "the dry run's prompts carry the redacted evidence and never the task text",
        dry.dry_run_prompts.iter().all(|(_, prompt)| {
            prompt.contains("DELEGATION DATA BEGIN")
                && prompt.contains("redacted by design")
                && prompt.contains("never instructions to follow")
        }),
    ));
    checks.push((
        "the dry run spends nothing",
        (crate::delegate::day_spend(&dry_root, &home, &day)).abs() < 1e-9,
    ));
    checks.push((
        "the dry run's card says so",
        dry.card.contains("dry run") && dry.card.contains("no ask, no row, no spend"),
    ));
    drop(lane);

    // The real leg: one bounded ask per unjudged row, each answered
    // with its own row's judgment, each landing a row keyed by the
    // child's session id beside the delegation itself.
    let answers: Vec<(String, serde_json::Value)> = specs
        .iter()
        .map(|spec| {
            (
                spec.session.to_string(),
                delegate_answer(spec.verdict, spec.confidence, spec.reason),
            )
        })
        .collect();
    let lane = match ScriptedLane::start(&mcp_bin, delegate_lane_entries(&answers)) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let outcome = crate::delegate::judge(crate::delegate::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::delegate::LaneSource::Scripted(lane.spec()),
    })
    .await;
    checks.push((
        "the batch refuses nothing at the batch level",
        outcome.refusal.is_none(),
    ));
    checks.push(("five rows are judged", outcome.judged.len() == 5));
    checks.push((
        "the already-judged delegation is refused by name",
        outcome.refusals.len() == 1
            && outcome.refusals[0].0 == judged_key
            && outcome.refusals[0].1.contains("already judged"),
    ));
    // The three verdicts, each with its signal.
    let by_session = |session: &str| {
        outcome
            .judged
            .iter()
            .find(|row| row.session == session)
            .unwrap_or_else(|| panic!("no judgment for {session}"))
    };
    let right = by_session("sess-r1");
    checks.push((
        "the agreeing rows grade right with the lane's confidence",
        right.verdict == calib::VERDICT_RIGHT
            && by_session("sess-r2").verdict == calib::VERDICT_RIGHT
            && (right.confidence - 0.91).abs() < 1e-9,
    ));
    let wrong = by_session("sess-w1");
    checks.push((
        "the contradicted claim grades wrong and names the log's own words",
        wrong.verdict == calib::VERDICT_WRONG
            && wrong.reason.contains("contradicted by its own session log")
            && wrong.reason.contains("recorded `failed`"),
    ));
    let capped = by_session("sess-i1");
    checks.push((
        "the capped run grades insufficient and names the companion",
        capped.verdict == calib::VERDICT_INSUFFICIENT
            && capped.reason.contains("a longer turn budget"),
    ));
    let escalated = by_session("sess-i2");
    checks.push((
        "the lane's escalation grades insufficient with the lane's companion",
        escalated.verdict == calib::VERDICT_INSUFFICIENT
            && escalated
                .reason
                .contains("the answer needed the escalation leg"),
    ));
    // The rows land in the stream beside the delegations, keyed by the
    // child's session id.
    let day_rows = crate::delegate::read_day(&root, &day);
    checks.push((
        "the judged rows ride the stream beside the delegations",
        day_rows.delegations.len() == 6 && day_rows.judged.len() == 6,
    ));
    checks.push((
        "the judgment joins the delegation it graded",
        every_judgment_joins(&day_rows),
    ));
    // The cells key on (provider × brief shape) and keep the three
    // rates apart; a one- or two-row cell is unfit and informs nothing.
    let cells = crate::delegate::calibration(&day_rows.judged);
    let refactor_stepfun = cells
        .iter()
        .find(|cell| cell.provider == "stepfun" && cell.shape() == "refactor")
        .expect("the (stepfun, refactor) cell");
    checks.push((
        "the cell counts its rows and keeps the rates apart",
        refactor_stepfun.judged == 3
            && refactor_stepfun.right == 3
            && refactor_stepfun.wrong == 0
            && refactor_stepfun.insufficient == 0,
    ));
    checks.push((
        "a three-row cell is unfit and informs nothing",
        !refactor_stepfun.informs_failover() && refactor_stepfun.needs_rows() > 0,
    ));
    // The batch is idempotent, and the second batch answers (the exit
    // code's all-already-judged leg).
    let again = crate::delegate::judge(crate::delegate::JudgeRequest {
        root: &root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::delegate::LaneSource::Scripted(lane.spec()),
    })
    .await;
    checks.push((
        "a second batch spends nothing on a delegation it already judged",
        again.judged.is_empty()
            && again.refusals.len() == 6
            && again
                .refusals
                .iter()
                .all(|(_, reason)| reason.contains("already judged"))
            && again.batch_answered(),
    ));
    drop(lane);

    // The conservative ride-back, over hand-written judged rows in
    // their own roots: a fit-and-wrong cell is skipped in the
    // preference, an unfit one never is however wrong it looks, and a
    // provider outside the candidate set is never skipped.
    let wrong_root = match scratch_dir("delegate-wrong") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let split: Vec<serde_json::Value> = (0..8u32)
        .map(|index| {
            delegation_judged_line(
                &format!("wrong-key-{index:016x}"),
                "zai-coding-plan",
                "refactor",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.05,
            )
        })
        .chain((0..8u32).map(|index| {
            delegation_judged_line(
                &format!("right-key-{index:016x}"),
                "zai-coding-plan",
                "triage",
                calib::VERDICT_RIGHT,
                0.95 - index as f64 * 0.05,
            )
        }))
        .collect();
    write_day_file(&wrong_root, &day, &split);
    checks.push((
        "a fit-and-wrong cell is skipped in the preference",
        crate::delegate::delegation_skips(
            &wrong_root,
            &day,
            &["zai-coding-plan", "stepfun"],
            "refactor",
        )
        .len()
            == 1,
    ));
    checks.push((
        "the same provider's right-for-triage cell skips nothing for triage",
        crate::delegate::delegation_skips(&wrong_root, &day, &["zai-coding-plan"], "triage")
            .is_empty(),
    ));
    checks.push((
        "a provider outside the candidate set is never skipped",
        crate::delegate::delegation_skips(&wrong_root, &day, &["stepfun"], "refactor").is_empty(),
    ));
    // An unfit cell: one wrong row at full confidence. The raw rate is
    // 1.0 and it still moves nothing, because the fit requirement
    // outranks the number.
    let unfit_root = match scratch_dir("delegate-unfit") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let unfit = vec![delegation_judged_line(
        "unfit-key-0000000000000001",
        "zai-coding-plan",
        "refactor",
        calib::VERDICT_WRONG,
        0.95,
    )];
    write_day_file(&unfit_root, &day, &unfit);
    checks.push((
        "an unfit cell never moves routing however wrong it looks",
        crate::delegate::delegation_skips(&unfit_root, &day, &["zai-coding-plan"], "refactor")
            .is_empty(),
    ));

    // The cells' named consumer, driven through the beat's real render
    // path: `fleet delegate` renders what the calibration would move
    // beside the graded rows. The cards wrap their clauses on a
    // character count and never drop one, so the assertions read the
    // section's prose with the frame's decoration and the spaces
    // removed — what an operator reads, wherever the wrap landed.
    let prose = |card: &str| -> String {
        strip_ansi(card)
            .lines()
            .map(|line| {
                line.trim_start()
                    .trim_start_matches('\u{2502}')
                    .trim_start()
                    .trim_start_matches('\u{b7}')
            })
            .collect::<String>()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let squeeze = |text: &str| -> String {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let ride_back = prose(&crate::delegate::render_ride_back_day(
        &wrong_root,
        &day,
        &["zai-coding-plan", "stepfun"],
        true,
    ));
    checks.push((
        "the beat renders what the calibration would move for the shape that earned it",
        ride_back.contains(&squeeze(
            "would move the suggestion past zai-coding-plan for shape `refactor`",
        )) && ride_back.contains(&squeeze("judged wrong at 1.00"))
            && ride_back.contains(&squeeze("8 graded verdicts from 8 judged rows")),
    ));
    checks.push((
        "and renders nothing for the shape that earned nothing",
        !ride_back.contains(&squeeze("for shape `triage`")),
    ));
    // A cell whose rows are mostly waived never earns its ride-back:
    // seven wrong verdicts beside two the owner set aside is nine rows
    // on the day and seven in the fit's sample, so the conservative
    // direction holds and the section moves no one.
    let waived_root = match scratch_dir("delegate-waived") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let mut waived_rows: Vec<serde_json::Value> = (0..7)
        .map(|index| {
            delegation_judged_line(
                &format!("waived-wrong-{index:016x}"),
                "zai-coding-plan",
                "refactor",
                calib::VERDICT_WRONG,
                0.95 - index as f64 * 0.06,
            )
        })
        .collect();
    for index in 0..2 {
        let mut row = delegation_judged_line(
            &format!("waived-aside-{index:016x}"),
            "zai-coding-plan",
            "refactor",
            calib::VERDICT_WRONG,
            0.90,
        );
        row["disposition"] = serde_json::json!(calib::DISPOSITION_WAIVED);
        waived_rows.push(row);
    }
    write_day_file(&waived_root, &day, &waived_rows);
    let waived_day = crate::delegate::read_day(&waived_root, &day);
    let waived_cell = crate::delegate::calibration(&waived_day.judged)
        .into_iter()
        .next()
        .expect("the (zai-coding-plan, refactor) cell");
    checks.push((
        "a cell whose rows are mostly waived holds nine rows and reads seven verdicts",
        waived_cell.judged == 9
            && waived_cell.waived == 2
            && waived_cell.graded_verdicts() == 7
            && waived_cell.needs_rows() == 0
            && waived_cell.needs_graded() == 1
            && waived_cell.fit == calib::FitState::Unfit
            && !waived_cell.informs_failover(),
    ));
    let waived_ride_back = prose(&crate::delegate::render_ride_back_day(
        &waived_root,
        &day,
        &["zai-coding-plan"],
        true,
    ));
    checks.push((
        "so the beat renders no move for it, however wrong the rate reads",
        !waived_ride_back.contains(&squeeze("would move the suggestion past")),
    ));

    // The fail-open leg: the lane's own fallback shape is a named
    // refusal, not a judgment — no row, and the recorded decisions
    // stand. A fresh root holds one unjudged delegation.
    let open_root = match scratch_dir("delegate-open") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let open_rows = write_delegation_day(&open_root, &specs[..1]);
    write_day_file(&open_root, &day, &open_rows);
    let fail_open = delegate_lane_entries(&[(
        "sess-r1".to_string(),
        serde_json::json!({
            "task": "route", "verdict": "error",
            "telemetry_error": "no provider answered the judgment head",
        }),
    )]);
    let lane = match ScriptedLane::start(&mcp_bin, fail_open) {
        Ok(lane) => lane,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let opened = crate::delegate::judge(crate::delegate::JudgeRequest {
        root: &open_root,
        home: &home,
        day: day.clone(),
        dry_run: false,
        lane: crate::delegate::LaneSource::Scripted(lane.spec()),
    })
    .await;
    checks.push((
        "a lane that fails open is a named refusal, not a judgment",
        opened.judged.is_empty()
            && opened.refusals.len() == 1
            && opened.refusals[0]
                .1
                .contains("no provider answered the judgment head"),
    ));
    checks.push((
        "the fail-open lands no row",
        crate::delegate::read_day(&open_root, &day)
            .judged
            .is_empty(),
    ));
    drop(lane);

    // The human seam: the owner's disposition lands a later append-only
    // row that spends nothing, and a disagreement inverts because the
    // owner's grading is the ground truth on whether a brief shape
    // drove.
    let spend_before = crate::delegate::day_spend(&root, &home, &day);
    let disagreed = crate::delegate::disposition(&root, &day, "sess-w1", "disagreed");
    checks.push((
        "the owner's disagreement lands a later row",
        disagreed.is_ok()
            && disagreed.as_ref().unwrap().disposition.as_deref() == Some("disagreed"),
    ));
    let waived = crate::delegate::disposition(&root, &day, "sess-r1", "waived");
    checks.push((
        "the owner's waiver sets the row aside",
        waived.is_ok() && waived.as_ref().unwrap().disposition.as_deref() == Some("waived"),
    ));
    checks.push((
        "a disposition spends nothing",
        (crate::delegate::day_spend(&root, &home, &day) - spend_before).abs() < 1e-9,
    ));
    // The freshest row stands: the disposition rides a later row that
    // keeps the original verdict beside the owner's word, and the CELL
    // math applies it — a disagreement inverts, because the owner's
    // grading is the ground truth on whether a brief shape drove.
    let after = crate::delegate::read_day(&root, &day);
    let w1 = after
        .judged
        .iter()
        .rev()
        .find(|row| row.session == "sess-w1")
        .expect("the disposition row");
    checks.push((
        "the freshest row stands and carries the owner's word",
        w1.disposition.as_deref() == Some("disagreed"),
    ));
    let cells_after = crate::delegate::calibration(&after.judged);
    let w1_cell = cells_after
        .iter()
        .find(|cell| cell.provider == "zai-coding-plan" && cell.shape() == "refactor")
        .expect("the (zai-coding-plan, refactor) cell");
    checks.push((
        "the cell math applies the disposition and the verdict inverts",
        w1_cell.wrong == 0 && w1_cell.right == 1 && w1_cell.judged == 1,
    ));
    // The waived row is set apart: neither right nor wrong, excluded
    // from the rate.
    let r1_cell = cells_after
        .iter()
        .find(|cell| cell.provider == "stepfun" && cell.shape() == "refactor")
        .expect("the (stepfun, refactor) cell");
    checks.push((
        "the waived row is set apart and excluded from the rate",
        r1_cell.waived == 1 && r1_cell.right == 2 && r1_cell.judged == 3,
    ));
    // An unknown key is a named refusal, not a silent write.
    checks.push((
        "a disposition naming no judged row refuses by name",
        crate::delegate::disposition(&root, &day, "no-such-session", "agreed").is_err(),
    ));

    for scratch in [
        &home,
        &root,
        &dry_root,
        &wrong_root,
        &unfit_root,
        &waived_root,
        &open_root,
    ] {
        let _ = std::fs::remove_dir_all(scratch);
    }
    grade(NAME, family, &checks)
}

/// `nomenclature-guard`: the guard's whole discipline, graded with no
/// model and no lane in the path.
///
/// The plan's six checks, each against the same derived vocabulary the
/// write-time guard reads — so the scenario grades the machinery rather
/// than a second opinion of it:
///
/// 1. a concept with an established term returns it with its sites;
/// 2. a concept with none says so, plainly;
/// 3. a near-exact variant is named as the drift it is (the state the
///    write guard refuses);
/// 4. a novel name for an existing concept is flagged with the
///    candidate named;
/// 5. a genuinely new concept is not flagged;
/// 6. a delegation whose child's own writes the guard flagged carries
///    the drift in its evidence and names it in the grading prompt.
async fn nomenclature_guard(_mock_bin: PathBuf) -> ScenarioOutcome {
    const NAME: &str = "nomenclature-guard";
    let family = ScenarioFamily::Nomenclature;
    let root = match scratch_dir("nomenclature-tree") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    // The synthetic tree establishes `judge_route` through both
    // channels the extractor reads: identifiers in source, and the
    // backtick span in a doc.
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(
        root.join("src/judge.rs"),
        "pub fn judge_route(row: &Row) -> Verdict {\n    Verdict::default()\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/route.rs"),
        "pub fn route_of(row: &Row) -> Route {\n    Route::default()\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("docs/notes.md"),
        "The `judge_route` helper grades a row.\n",
    )
    .unwrap();
    let lookup = |concept: &str| {
        crate::tools::convention_result(&root, &serde_json::json!({ "concept": concept }))
    };
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // 1 — the established term, with its sites.
    let exact = lookup("judge_route");
    checks.push((
        "a concept with an established term returns it with its sites",
        exact.starts_with("established term: judge_route") && exact.contains("src/judge.rs:1"),
    ));
    // 2 — the honest miss, stated plainly. The concept's words are
    // deliberate nonsense: a code file's string literals are
    // identifier evidence for the vocabulary, so a fixture naming an
    // ordinary word would retire some other proof's absence.
    let miss = lookup("the flumdorian gizmodium");
    checks.push((
        "a concept with no established term says so plainly",
        miss.starts_with("no established term") && miss.contains("genuinely new concept"),
    ));
    // 3 — the near-exact variant: the state the write guard refuses,
    // answered by the lookup first, naming the rename target.
    let variant = lookup("JudgeRoute");
    checks.push((
        "a near-exact variant is named as the drift it is",
        crate::nomenclature::is_near_exact("JudgeRoute", "judge_route")
            && variant.contains("case or separator variant of the established term `judge_route`"),
    ));
    // 4 — the flag: a novel name with a canonical near-neighbour.
    let near = crate::nomenclature::check(&root, "route_judge");
    checks.push((
        "a novel name for an existing concept is flagged with the candidate",
        near.as_ref()
            .and_then(|convention| convention.term.as_deref())
            == Some("judge_route"),
    ));
    // 5 — clean: a genuinely new concept, unflagged.
    let fresh = crate::nomenclature::check(&root, "the_quibbet_sled_concept");
    checks.push((
        "a genuinely new concept is not flagged",
        fresh.is_none() && lookup("the_quibbet_sled_concept").contains("genuinely new concept"),
    ));
    // 6 — the drift signal: a child's own log carries the guard's
    // flag, the evidence reads it back, and the grading prompt names
    // it where the grader reads.
    let sessions = root.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let log = sessions.join("sess-drift.jsonl");
    let note = "note: this write introduces names with canonical near-neighbours in this \
                project's vocabulary:\n- `routeJudge` \u{2014} the established term is \
                `judge_route` (src/judge.rs:1); consider renaming";
    let lines = [
        serde_json::json!({ "type": "start", "v": 1, "model": "test-model",
            "created": "2026-10-15T12:00:00Z", "kind": "subagent" })
        .to_string(),
        serde_json::json!({ "type": "assistant", "content": "working", "tool_calls": [
            { "id": "call-0", "type": "function", "function": {
                "name": "write_file",
                "arguments": serde_json::json!({ "path": "src/judge.rs", "content": "x" }).to_string(),
            } },
        ] })
        .to_string(),
        serde_json::json!({ "type": "tool_result", "call_id": "call-0", "name": "write_file",
            "output": format!("wrote 11 bytes\n{note}"), "truncated": false, "approved": true })
            .to_string(),
        serde_json::json!({ "type": "end", "stop_reason": "completed", "turns": 2 }).to_string(),
    ];
    std::fs::write(&log, format!("{}\n", lines.join("\n"))).unwrap();
    // The row reaches the grader the way a real one does: written as
    // the stream's own line and read back through the delegate reader
    // — only `delegate` constructs a delegation row.
    let day = calib::today();
    let stream_root = root.join("stream");
    let rows = vec![delegation_row_line(
        "sess-drift",
        "refactor",
        "stepfun",
        "completed",
        2,
        &log.to_string_lossy(),
        "sess-root",
    )];
    write_day_file(&stream_root, &day, &rows);
    let day_rows = crate::delegate::read_day(&stream_root, &day);
    let row = day_rows
        .delegations
        .first()
        .expect("the day's row reads back through the delegate reader");
    let evidence = crate::delegate::read_evidence(row);
    let prompt = crate::delegate::compose_prompt(row, &evidence);
    checks.push((
        "a drifted delegation carries the flag in its evidence and names it in the prompt",
        evidence.drifted == vec!["routeJudge".to_string()]
            && prompt.contains("routeJudge")
            && prompt.contains("canonical near-neighbours"),
    ));
    let _ = std::fs::remove_dir_all(&root);
    grade(NAME, family, &checks)
}

/// `dependency-graph`: the orchestration the waves replaced, graded
/// with no model and no lane in the path.
///
/// The plan's checks over the public surface: the diamond is the
/// shape, the fact-set is the gate, and every assertion reads the same
/// API the dispatcher and the render do — so the scenario grades the
/// machinery rather than a second opinion of it.
async fn dependency_graph(_mock_bin: PathBuf) -> ScenarioOutcome {
    use crate::graph::{Capabilities, Graph, Node, Status};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    const NAME: &str = "dependency-graph";
    let family = ScenarioFamily::Nomenclature;
    let facts: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let gate = Arc::clone(&facts);
    let mut graph =
        Graph::with_verifier(move |node: &Node| gate.lock().unwrap().contains(&node.id));
    // The diamond: ship needs left and right, both need base. Costs:
    // base 2, left 3, right 4, ship 6.
    graph
        .add(Node::new("base", 2).with_paths(["src/base.rs"]))
        .unwrap();
    graph
        .add(
            Node::new("left", 3)
                .with_prereqs(["base"])
                .with_paths(["src/left.rs"]),
        )
        .unwrap();
    graph
        .add(
            Node::new("right", 4)
                .with_prereqs(["base"])
                .with_paths(["src/right.rs"]),
        )
        .unwrap();
    graph
        .add(
            Node::new("ship", 6)
                .with_prereqs(["left", "right"])
                .with_capabilities(Capabilities {
                    tools: vec!["bash".to_string()],
                    skills: vec!["vendoring-lanes".to_string()],
                    workflows: Vec::new(),
                    loops: vec!["judge".to_string()],
                }),
        )
        .unwrap();
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // 1 — nothing has landed: base is ready, and nothing else is.
    let ready = graph.ready();
    checks.push((
        "a node with no prerequisites is ready immediately",
        ready == vec!["base".to_string()],
    ));

    // 2 — the arithmetic, pinned: two numbers, both from the same walk.
    // Remaining work is a sum over the SET of unfinished nodes in the
    // closure, so a shared base counts once — ship carries left(3) +
    // right(4) + base(2) = 9. Time to completion is the longest chain
    // below it — max(3+2, 4+2) = 6.
    //
    // P2: the score used to sum over PATHS, reading (3+2)+(4+2) = 11,
    // which was neither the work left nor the time left, and
    // `critical_path` picked its head by it — so a convergence node
    // outranked a genuinely longer single chain.
    let scores = graph.scores();
    let score_of = |id: &str| {
        scores
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, score)| *score)
            .unwrap_or(0)
    };
    checks.push((
        "the dependency score counts the closure's remaining work",
        score_of("base") == 0 && score_of("left") == 2 && score_of("ship") == 9,
    ));
    checks.push((
        "time to completion is the longest chain below, not the pile of work",
        graph.time_to_completion("ship") == Some(6) && graph.time_to_completion("left") == Some(2),
    ));

    // 3 — the later nodes dispatch the moment their prerequisite's
    // fact lands, without waiting for a wave: base verifies, and both
    // branches are dispatchable in the same pass. The order is the
    // documented one — the node blocking the most work first, then the
    // larger cost — so right (4) precedes left (3).
    facts.lock().unwrap().insert("base".to_string());
    graph.settle();
    let dispatchable = graph.dispatchable(3, &[]);
    checks.push((
        "a node that unlocks mid-flight dispatches without waiting for the wave",
        dispatchable == vec!["right".to_string(), "left".to_string()],
    ));

    // 4 — a verified completion drops every dependent's score by the
    // completed node's cost, and the arithmetic is the formula's own.
    let after_base = graph.scores();
    let after_of = |id: &str| {
        after_base
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, score)| *score)
            .unwrap_or(0)
    };
    checks.push((
        "a verified completion drops every dependent's score by its cost",
        after_of("left") == 0 && after_of("right") == 0 && after_of("ship") == 7,
    ));

    // 5 — the resource edge is mandatory: left is running and holds
    // src/left.rs, so a node naming the same path is held even at
    // score zero, with the holder named.
    graph.dispatch("left").unwrap();
    graph.dispatch("right").unwrap();
    graph
        .add(Node::new("twin", 1).with_paths(["src/left.rs"]))
        .unwrap();
    let twin_unlock = graph.unlock("twin").expect("twin is declared");
    checks.push((
        "a resource conflict holds a node and names the holder",
        !twin_unlock.is_unlocked()
            && twin_unlock
                .reason()
                .map(|reason| reason.contains("left"))
                .unwrap_or(false),
    ));
    // With both branches' facts landed, ship is unlocked: score zero
    // and no conflict of its own.
    facts.lock().unwrap().insert("left".to_string());
    facts.lock().unwrap().insert("right".to_string());
    graph.settle();
    let ship_unlock = graph.unlock("ship").expect("ship is declared");
    checks.push((
        "a zero-score node with no resource conflict is unlocked",
        ship_unlock.is_unlocked(),
    ));

    // 6 — a failed prerequisite never unlocks its dependent, and the
    // reason names it.
    graph.add(Node::new("seed", 1)).unwrap();
    graph
        .add(Node::new("crop", 2).with_prereqs(["seed"]))
        .unwrap();
    graph.dispatch("seed").unwrap();
    graph.fail("seed").unwrap();
    let crop_unlock = graph.unlock("crop").expect("crop is declared");
    checks.push((
        "a failed prerequisite does not unlock its dependent and names it",
        !crop_unlock.is_unlocked()
            && crop_unlock
                .reason()
                .map(|reason| reason.contains("seed"))
                .unwrap_or(false),
    ));

    // 7 — the render names the critical path and what holds each node.
    let cards = crate::graph::render(&graph);
    checks.push((
        "the render names the critical path and the holds",
        cards.contains("critical path")
            && cards.contains("twin")
            && cards.contains("crop")
            && cards.contains("vendoring-lanes"),
    ));

    // 8 — the surface's file: the graph round-trips through the home's
    // own graph.json with the state it stood in, and a file that will
    // not read is named rather than defaulted.
    let root = match scratch_dir("dependency-graph-home") {
        Ok(root) => root,
        Err(error) => return ScenarioOutcome::fail(NAME, family, error),
    };
    let path = crate::graph::path_in(&root);
    graph.save(&path).unwrap();
    let loaded = Graph::load(&path).unwrap();
    checks.push((
        "the graph round-trips through the home's file with its state",
        loaded.len() == graph.len()
            && loaded.node("left").map(|node| node.status) == Some(Status::Verified),
    ));
    let malformed = root.join("malformed.json");
    std::fs::write(&malformed, "{ not a graph").unwrap();
    checks.push((
        "a graph file that will not read is named, not defaulted",
        Graph::load(&malformed)
            .unwrap_err()
            .contains("is not a declared graph"),
    ));
    let _ = std::fs::remove_dir_all(&root);
    grade(NAME, family, &checks)
}

/// `delta-replan`: the replan the delta report drives, graded with no
/// model and no lane in the path.
///
/// The plan's checks over the public surface: the weights, the two
/// triggers, the append-only revision, and the verbatim ride up the
/// parent chain — the last through `Delta::parse_render`, which reads
/// a rendered result back the way the parent chain actually carries
/// it, so a middle node that summarized its children's deltas would
/// fail this check rather than pass silently.
async fn delta_replan(_mock_bin: PathBuf) -> ScenarioOutcome {
    use crate::graph::{Graph, Node};
    use crate::subagents::{Delta, DeltaReason, DeltaState};

    const NAME: &str = "delta-replan";
    let family = ScenarioFamily::Nomenclature;
    let mut checks: Vec<(&str, bool)> = Vec::new();

    let deviated = |reason: DeltaReason, forced: bool, instead: &str| Delta {
        state: DeltaState::Deviated,
        reason,
        companion: String::new(),
        instead: instead.to_string(),
        forced,
        children: Vec::new(),
        declares: Vec::new(),
        contradiction: None,
    };

    // 1 — a followed brief produces a followed delta and no replan.
    let mut graph = Graph::new();
    graph.add(Node::new("a", 1).with_approach("do a")).unwrap();
    graph
        .record_delta(
            "a",
            Delta {
                state: DeltaState::Followed,
                reason: DeltaReason::RightAndFollowed,
                companion: String::new(),
                instead: String::new(),
                forced: false,
                children: Vec::new(),
                declares: Vec::new(),
                contradiction: None,
            },
        )
        .unwrap();
    checks.push((
        "a followed brief produces a followed delta and no replan",
        graph.diff_score() == 0 && !graph.should_replan() && graph.replan().is_none(),
    ));

    // 2 — a forced deviation produces its reason and a replan that
    // revises the remaining approaches, naming the revision.
    let mut graph = Graph::new();
    graph
        .add(Node::new("a", 1).with_approach("write src/gone.rs"))
        .unwrap();
    graph
        .add(
            Node::new("b", 2)
                .with_prereqs(["a"])
                .with_approach("read what a wrote"),
        )
        .unwrap();
    graph
        .record_delta(
            "a",
            deviated(DeltaReason::Infeasible, true, "wrote src/here.rs instead"),
        )
        .unwrap();
    let revision = graph.replan();
    checks.push((
        "a forced deviation produces its reason and a replan that revises the rest",
        revision.is_some()
            && graph.node("a").unwrap().approach == "wrote src/here.rs instead"
            && graph
                .revisions()
                .first()
                .map(|revision| revision.cause.contains("diff score 6"))
                .unwrap_or(false),
    ));

    // 3 — a chosen improvement is recorded as a gain and moves nothing.
    let mut graph = Graph::new();
    graph
        .add(Node::new("a", 1).with_approach("do a the slow way"))
        .unwrap();
    graph
        .record_delta(
            "a",
            deviated(DeltaReason::BetterRoute, false, "did a the fast way"),
        )
        .unwrap();
    checks.push((
        "a chosen improvement is recorded as a gain and moves nothing",
        graph.diff_score() == 0
            && graph.replan().is_none()
            && graph.node("a").unwrap().approach == "do a the slow way",
    ));

    // 4 — a large aggregate triggers the replan and names the revision.
    let mut graph = Graph::new();
    for id in ["a", "b", "c"] {
        graph
            .add(Node::new(id, 1).with_approach("the infeasible way"))
            .unwrap();
        graph
            .record_delta(
                id,
                deviated(DeltaReason::Infeasible, false, "the feasible way"),
            )
            .unwrap();
    }
    let revision = graph.replan();
    checks.push((
        "a large aggregate triggers the replan and names the revision",
        revision.is_some()
            && revision
                .map(|revision| revision.seq == 1 && revision.changes.len() == 3)
                .unwrap_or(false),
    ));

    // 5 — a delta the log contradicts is priced as the false report it
    // is, and the render says so.
    let mut graph = Graph::new();
    graph.add(Node::new("a", 1).with_approach("do a")).unwrap();
    graph
        .record_delta(
            "a",
            Delta {
                contradiction: Some(
                    "the child claimed a deviation but its own session log holds no tool call"
                        .to_string(),
                ),
                ..deviated(DeltaReason::Infeasible, false, "")
            },
        )
        .unwrap();
    let cards = crate::graph::render(&graph);
    checks.push((
        "a delta the log contradicts is priced above every category and named",
        graph.diff_score() == 5
            && graph.should_replan()
            && cards.contains("CONTRADICTED")
            && cards.contains("diff score"),
    ));

    // 6 — a middle node preserves its child's delta verbatim up the
    // chain: the rendered result carries both blocks, and the parse
    // reads the child's back with every field intact rather than a
    // summary of it. The seam is the production one — a rendered
    // `SubagentResult` through `bound_result`, exactly what the
    // parent's tool_result carries — so a middle node that summarized
    // its children's deltas would fail this check rather than pass
    // silently.
    let child = deviated(
        DeltaReason::CompanionMissing,
        true,
        "wrote the parser first",
    );
    let mut parent = deviated(DeltaReason::Infeasible, false, "restructured first");
    parent.children = vec![child.clone()];
    let result = crate::subagents::SubagentResult {
        goal: String::new(),
        summary: "restructured first".to_string(),
        session: "sess-mid".to_string(),
        log: std::path::PathBuf::from("/tmp/sessions/sess-mid.jsonl"),
        turns: 2,
        outcome: crate::subagents::Outcome::Completed,
        reason: None,
        delta: crate::subagents::DeltaReport {
            own: Some(parent),
            children: Vec::new(),
            offered: true,
        },
    };
    let rendered = crate::subagents::bound_result(&result).output;
    let read = Delta::parse_render(&rendered).expect("the rendered result parses");
    checks.push((
        "a middle node preserves its child's delta verbatim up the chain",
        read.children.len() == 1
            && read.children[0] == child
            && read.children[0].reason == DeltaReason::CompanionMissing
            && rendered.contains("companion-missing"),
    ));

    grade(NAME, family, &checks)
}

/// The lesson store end to end — the write, the evidence refusal, and
/// the render — against a scratch home. No mock and no model: the store
/// is deterministic by construction, which is why the scenario is too.
async fn improve_lesson_store(_mock_bin: PathBuf) -> ScenarioOutcome {
    use crate::improve::{load_lessons, render_lessons, write_lesson, Lesson, Target};

    const NAME: &str = "improve-lesson-store";
    let family = ScenarioFamily::Improve;
    let home = match scratch_dir("improve-lessons") {
        Ok(dir) => dir,
        Err(reason) => return ScenarioOutcome::fail(NAME, family, reason),
    };
    let lesson = Lesson {
        name: "corpus-lesson".to_string(),
        target: Target::Source,
        target_name: "src/lib.rs".to_string(),
        contrast: "corpus 0.90 \u{2192} 1.00 (measured gain)".to_string(),
        cue: "a source proposal for `src/lib.rs`".to_string(),
        lesson: "the fold carries its summary".to_string(),
        evidence: "the corpus named the fold before the fix".to_string(),
        confidence: "measured".to_string(),
        created: crate::calib::today(),
    };
    if let Err(reason) = write_lesson(&home, &lesson) {
        return ScenarioOutcome::fail(NAME, family, reason);
    }
    let bare = Lesson {
        name: "bare-assertion".to_string(),
        evidence: "   ".to_string(),
        ..lesson.clone()
    };
    let bare_refused = match write_lesson(&home, &bare) {
        Ok(_) => false,
        Err(reason) => reason.contains("refuses bare assertions"),
    };
    let rendered = render_lessons(&load_lessons(&home));
    let checks = [
        (
            "the lesson file is written under the home",
            crate::improve::lessons_dir(&home)
                .join("corpus-lesson.md")
                .exists(),
        ),
        (
            "a lesson without evidence is refused by name and writes nothing",
            bare_refused
                && !crate::improve::lessons_dir(&home)
                    .join("bare-assertion.md")
                    .exists(),
        ),
        (
            "the surface renders the record and its contrast",
            rendered.contains("corpus-lesson") && rendered.contains("0.90 \u{2192} 1.00"),
        ),
    ];
    grade(NAME, family, &checks)
}

/// One hand-written `delegation_judged` row, the shape a writer would
/// have landed — the ride-back's own fixture.
fn delegation_judged_line(
    session: &str,
    provider: &str,
    brief_shape: &str,
    verdict: &str,
    confidence: f64,
) -> serde_json::Value {
    serde_json::json!({
        "ts": "2026-10-14T13:00:00Z",
        "op": "delegation_judged",
        "session": session,
        "provider": provider,
        "brief_shape": brief_shape,
        "verdict": verdict,
        "confidence": confidence,
        "reason": "the fixture's own reason",
        "lane": "sys1",
    })
}

/// Every judged row joins a delegation the day carries, by session id
/// — the join the stream promises. Written without naming the row
/// type: the delegate module's guard owns construction of the rows,
/// and this scenario builds its fixtures as stream JSON, never as the
/// struct.
fn every_judgment_joins(day: &crate::delegate::DayRows) -> bool {
    day.judged.iter().all(|judged| {
        day.delegations.iter().any(|delegation| {
            delegation.session == judged.session
                && crate::delegate::delegation_key(delegation) == judged.session
        })
    })
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
    let records = session::replay(session.path()).unwrap_or_default();
    let lane_answer = tool_message(&mock, 1);
    let call = records.iter().find_map(|record| match record {
        Record::Assistant { tool_calls, .. } => tool_calls
            .first()
            .map(|call| (call.function.name.clone(), call.function.arguments.clone())),
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
        call_name: call
            .as_ref()
            .map(|(name, _)| name.clone())
            .unwrap_or_default(),
        call_arguments: call
            .as_ref()
            .map(|(_, arguments)| arguments.clone())
            .unwrap_or_default(),
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
            (
                "the run completed",
                run.report.stop_reason == StopReason::Completed,
            ),
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
            (
                "the tabular fact reached the model's context",
                run.lane_answer.contains(FACT),
            ),
            (
                "the final answer carries the fact",
                final_answer.contains(FACT),
            ),
            ("the fact was not rounded", !rounded),
            (
                "the grounded path names itself and did not escalate",
                run.lane_answer.contains("\"path\":\"synthesized\"")
                    && run.lane_answer.contains("\"escalated\":false"),
            ),
            ("the log reconstructs the turn", run.records.len() == 9),
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
            (
                "the run completed",
                run.report.stop_reason == StopReason::Completed,
            ),
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
            ("the log reconstructs the turn", run.records.len() == 9),
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
            (
                "the run completed",
                run.report.stop_reason == StopReason::Completed,
            ),
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
            ("the log reconstructs the turn", run.records.len() == 9),
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
            (
                "the run completed",
                run.report.stop_reason == StopReason::Completed,
            ),
            ("two turns", run.report.turns == 2),
            (
                "the model asked the project's corpus",
                run.call_name == "mcp__blend1__search"
                    && run.call_arguments.contains("\"project\":\"fleet\""),
            ),
            (
                "the seeded fact reached the model's context",
                run.lane_answer.contains(fact),
            ),
            (
                "the final answer quotes the fact unaltered",
                final_answer.contains(fact),
            ),
            ("the log reconstructs the turn", run.records.len() == 9),
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
        assert_eq!(
            ScenarioFamily::AgentContinuation.label(),
            "agent-continuation"
        );
        assert_eq!(
            ScenarioFamily::ConversationFold.label(),
            "conversation-fold"
        );
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
                "delegation-loop-closes",
                "nomenclature-guard",
                "dependency-graph",
                "delta-replan",
                "improve-lesson-store",
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
        assert_eq!(
            entries[2]["reply"]["result"]["content"][0]["text"],
            "{\"ok\":true}"
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
        let broken = Scenario::always_fails(
            "injected-break",
            ScenarioFamily::ToolLoop,
            "the seam broke it",
        );
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
        assert!(
            rendered.contains("no eval-corpus run recorded"),
            "{rendered}"
        );
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
    fn a_green_corpus_output_carries_its_score_and_a_red_one_names_the_scenarios() {
        let green = std::process::Output {
            status: exit_status(0),
            stdout: "fleet eval \u{2014} 6 scenarios, score 1.00\n\
                     eval-score: 1.00\n"
                .as_bytes()
                .to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(grade_output(&green), Ok(ProbeOutcome::Green { score: 1.0 }));

        let red = std::process::Output {
            status: exit_status(1),
            stdout: format!(
                "eval-score: 0.90\n{WARN}regressions: conversation-fold, memory-recall{RESET}\n"
            )
            .into_bytes(),
            stderr: Vec::new(),
        };
        // The names the card carries must reach the gate step without
        // the palette's trailing reset riding along.
        let outcome = grade_output(&red).expect("a red corpus is measured, not unmeasured");
        assert_eq!(
            outcome,
            ProbeOutcome::Regressed {
                score: 0.90,
                scenarios: vec!["conversation-fold".to_string(), "memory-recall".to_string()],
            }
        );
        assert!(!format!("{outcome:?}").contains('\u{1b}'), "{outcome:?}");
    }

    #[test]
    fn a_red_corpus_without_a_verdict_is_unmeasured_rather_than_regressed() {
        // Exit 101 with no `regressions:` line — the driver-panic shape
        // the module docstring names — is "could not be measured", and
        // must never read as a measured regression.
        let red = std::process::Output {
            status: exit_status(101),
            stdout: Vec::new(),
            stderr: b"thread 'main' panicked at src/eval.rs:1:1\n".to_vec(),
        };
        let refused = grade_output(&red).expect_err("unmeasured");
        assert!(refused.contains("could not be measured"), "{refused}");
        assert!(refused.contains("with code 101"), "{refused}");
        assert!(!refused.contains("regressed"), "{refused}");
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

    #[test]
    fn a_scored_exit_without_a_regressions_line_is_unmeasured() {
        // A score alone is not a verdict: a child that printed its card
        // and then died before naming any scenario is still unmeasured.
        let weird = std::process::Output {
            status: exit_status(2),
            stdout: b"eval-score: 0.95\n".to_vec(),
            stderr: b"the mock binary is missing\n".to_vec(),
        };
        let refused = grade_output(&weird).expect_err("unmeasured");
        assert!(refused.contains("could not be measured"), "{refused}");
        assert!(!refused.contains("regressed"), "{refused}");
    }
}
