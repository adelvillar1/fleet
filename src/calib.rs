//! The calibration stream: Fleet's audit spine.
//!
//! One `events.jsonl` per UTC day under the dev-decisions log root, in
//! `devdec`'s row vocabulary, written through a bounded non-blocking queue
//! that one background thread drains. Six ops are emitted today —
//! `route` (slice 5's decision row, shape unchanged), `tool_screen` (one
//! approval decision), `turn` (one run ending, graded ok/failed with
//! the partial endings named), `recall_hit` (slice 8's memory
//! feedback: one fact retrieval surfaced, the shape it routed under,
//! and the score it ranked at), `consolidate` (slice 8 part 2's
//! lifecycle: one compress, supersede, or forget action, the
//! fact it touched, and the reason), `propose` (slice 12's
//! self-maintenance gate: one proposal's verdict — applied, refused, or
//! left waiting — with the gate's per-step evidence), and `eval_corpus`
//! (slice 13's eval-corpus run: one run's per-scenario verdicts and its
//! aggregate score — the harness's graded model of its own performance).
//!
//! The contract is dev-decisions' own: it **gates the operations, grades
//! the outcomes, and earns autonomy on evidence**. Fleet's rows land in
//! the file devdec itself writes, so `dev-decisions log` reads them beside
//! its own and the calibration dataset is one stream rather than two.
//!
//! The stream is advisory by construction. A full queue drops a row and
//! counts it; an unwritable root logs and counts; no decision, screening,
//! or run is ever blocked, delayed, or failed by its own audit trail.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use crate::api::Usage;
use crate::route::Attempt;

// --- the vocabulary --------------------------------------------------------

/// One routing decision, in devdec's row vocabulary. This is slice 5's
/// `DecisionRow`, absorbed here unchanged: the same field names and the
/// same op, so an existing reader sees no difference.
#[derive(Debug, Clone, Serialize)]
pub struct RouteRow {
    pub ts: String,
    /// The candidate that served, or the selected one when none did.
    pub provider: String,
    /// `judged`, `fallback`, or `pinned`.
    pub how: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    pub reason: String,
    pub model: String,
    /// The candidates actually attempted, in order.
    pub providers_used: Vec<String>,
    pub attempts: Vec<Attempt>,
    pub input_chars: usize,
    /// A stable content hash of the task, for joining — never the text.
    pub input_hash: String,
    pub latency_ms: u128,
}

/// One tool-call screening decision: the approval gate's verdict for one
/// call, recorded after the decision and never inside it.
#[derive(Debug, Clone, Serialize)]
pub struct ToolScreenRow {
    pub ts: String,
    /// The call's name, namespaced for a lane call (`mcp__<lane>__<tool>`).
    pub tool: String,
    /// `approved`, `denied`, or `always` — see [`ScreenVerdict`].
    pub verdict: String,
    /// The lane a namespaced call went to; absent for a built-in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane: Option<String>,
    pub input_chars: usize,
    /// A hash of the call's arguments, for joining to the session log's
    /// `tool_result` record — never the arguments themselves.
    pub input_hash: String,
}

/// One run's ending: the outcome grade and the stop reason named.
#[derive(Debug, Clone, Serialize)]
pub struct TurnRow {
    pub ts: String,
    /// `ok` or `failed` — see [`TurnStop::outcome`].
    pub outcome: String,
    /// The ending, in the session log's own vocabulary, carried verbatim.
    pub stop_reason: String,
    pub turns: u64,
    pub usage: Usage,
    pub input_chars: usize,
    /// A hash of the task, for joining to the session log's first record —
    /// never the task text.
    pub input_hash: String,
    /// The failure class, when the ending was a failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One recall hit: a fact the retrieval surfaced, the shape the
/// instruction routed under, and the score it ranked at. The feedback
/// rung's audit row — recall's effect is measurable in the stream
/// rather than asserted.
#[derive(Debug, Clone, Serialize)]
pub struct RecallHitRow {
    pub ts: String,
    /// The fact's name, the file stem under `memories/`.
    pub fact: String,
    /// The classified structure shape, or empty when unclassified.
    pub shape: String,
    /// The composite score the fact ranked at.
    pub score: f64,
}

/// One consolidation action: what the memory lifecycle did to one
/// fact, and why. Three actions — `compress` folds old working memory
/// into an episodic summary, `supersede` retires a stale inferred fact
/// from recall, `forget` archives an expired one — and every one of
/// them lands here, so the loop's effect on the corpus is auditable
/// rather than asserted. The `reason` carries the justification a
/// `supersede` proposal required, which is what makes it slice 12's
/// guardrail input.
#[derive(Debug, Clone, Serialize)]
pub struct ConsolidateRow {
    pub ts: String,
    /// `compress`, `supersede`, or `forget` — see [`ACTION_COMPRESS`].
    pub action: String,
    /// The fact the action touched, the file stem under `memories/`.
    pub fact: String,
    /// Why: the cycle's named bound, or the proposal's justification.
    pub reason: String,
}

/// One gate step's evidence, as a `propose` row carries it: the step's
/// name, whether it passed, and the bounded detail that names the
/// failure when it did not. A step that could not run at all — an
/// absent tool — carries `ok: false` with the reason in `detail`, so a
/// gate step that cannot run is never a silent pass.
#[derive(Debug, Clone, Serialize)]
pub struct GateStepRow {
    pub step: String,
    pub ok: bool,
    pub detail: String,
}

/// One self-maintenance proposal's verdict: the proposal the slice-12
/// gate disposed of, the target it addressed, the basis it carried, and
/// the gate's per-step evidence. Three verdicts — `applied`, `refused`,
/// `waiting` — and every proposal that reaches one lands here exactly
/// once, so the loop's self-modification is auditable rather than
/// asserted. A proposal left waiting (a destructive change, or a
/// confirmed-by memory target, without approval in the current turn)
/// lands a row with the gate not yet run, which is what distinguishes
/// "waiting on the owner" from "refused by the gate".
#[derive(Debug, Clone, Serialize)]
pub struct ProposeRow {
    pub ts: String,
    /// The proposal's name, the file stem under `<FLEET_HOME>/proposals/`
    /// or the slice-7 draft's name under `skills/proposals/`.
    pub proposal: String,
    /// The target kind: `skill`, `memory`, `source`, or `config`.
    pub target: String,
    /// The target's name or path, as the proposal names it.
    pub target_name: String,
    /// The proposal's basis: `inferred` or `confirmed-by`.
    pub basis: String,
    /// `applied`, `refused`, or `waiting` — see [`VERDICT_APPLIED`].
    pub verdict: String,
    /// Why, bounded: what landed for an apply, the named failure for a
    /// refusal, and what will be destroyed (or which fact is
    /// confirmed-by) for a wait.
    pub reason: String,
    /// The gate's per-step evidence, in the order the steps ran. Empty
    /// for a `waiting` verdict, which never reached the gate.
    pub gate: Vec<GateStepRow>,
}

/// One scenario's verdict inside an `eval_corpus` run: the scenario's
/// name, the family it belongs to, whether it passed, and the failure's
/// name when it did not. The whole point of the corpus is that a failure
/// is *named*, so the row carries the name rather than a bare count.
#[derive(Debug, Clone, Serialize)]
pub struct EvalScenarioRow {
    pub scenario: String,
    /// The family the scenario belongs to, e.g. `conversation-fold`.
    pub family: String,
    pub passed: bool,
    /// The failure's name, omitted when the scenario passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
}

/// One eval-corpus run: the fixed suite's per-scenario verdicts and its
/// aggregate score. Slice 13's scores land here — the same stream every
/// other graded outcome lands in — so the corpus is queryable the way a
/// `turn` or `propose` row is, and the improve gate reads a regression
/// from the same place a reader does.
#[derive(Debug, Clone, Serialize)]
pub struct EvalCorpusRow {
    pub ts: String,
    /// The scenarios that passed.
    pub passed: usize,
    /// The scenarios that failed.
    pub failed: usize,
    /// The aggregate score: `passed / (passed + failed)`, or 1.0 for an
    /// empty corpus. In `[0, 1]`.
    pub score: f64,
    /// Every scenario's verdict, in corpus order.
    pub scenarios: Vec<EvalScenarioRow>,
}

/// One row in the stream, tagged by its op the way devdec's rows are: the
/// tag is a field of the JSON object, not a wrapper.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Row {
    Route(RouteRow),
    ToolScreen(ToolScreenRow),
    Turn(TurnRow),
    RecallHit(RecallHitRow),
    Consolidate(ConsolidateRow),
    Propose(ProposeRow),
    EvalCorpus(EvalCorpusRow),
}

impl Row {
    /// This row's op, the way the JSON tag carries it.
    pub fn op(&self) -> &'static str {
        match self {
            Row::Route(_) => OP_ROUTE,
            Row::ToolScreen(_) => OP_TOOL_SCREEN,
            Row::Turn(_) => OP_TURN,
            Row::RecallHit(_) => OP_RECALL_HIT,
            Row::Consolidate(_) => OP_CONSOLIDATE,
            Row::Propose(_) => OP_PROPOSE,
            Row::EvalCorpus(_) => OP_EVAL_CORPUS,
        }
    }

    /// This row's own UTC stamp; the day it belongs to is its leading date.
    pub fn ts(&self) -> &str {
        match self {
            Row::Route(row) => &row.ts,
            Row::ToolScreen(row) => &row.ts,
            Row::Turn(row) => &row.ts,
            Row::RecallHit(row) => &row.ts,
            Row::Consolidate(row) => &row.ts,
            Row::Propose(row) => &row.ts,
            Row::EvalCorpus(row) => &row.ts,
        }
    }
}

/// The routing op.
pub const OP_ROUTE: &str = "route";
/// The approval-screening op.
pub const OP_TOOL_SCREEN: &str = "tool_screen";
/// The run-ending op.
pub const OP_TURN: &str = "turn";
/// The memory-recall op: one hit per fact a recall surfaced.
pub const OP_RECALL_HIT: &str = "recall_hit";
/// The memory-lifecycle op: one compress, supersede, or forget action.
pub const OP_CONSOLIDATE: &str = "consolidate";
/// The self-maintenance op: one proposal's verdict under the slice-12
/// gate — applied, refused, or left waiting — with the gate's per-step
/// evidence.
pub const OP_PROPOSE: &str = "propose";
/// The eval-corpus op: one run of slice 13's fixed, graded suite — its
/// per-scenario verdicts and its aggregate score. The harness's model of
/// its own performance, and the row the improve gate reads a regression
/// from.
pub const OP_EVAL_CORPUS: &str = "eval_corpus";
/// The ops this slice emits, in the order `fleet calib` groups them.
pub const OPS: [&str; 7] = [
    OP_ROUTE,
    OP_TOOL_SCREEN,
    OP_TURN,
    OP_RECALL_HIT,
    OP_CONSOLIDATE,
    OP_PROPOSE,
    OP_EVAL_CORPUS,
];

/// The verdict a `propose` row carries when the change landed: the gate
/// was green, the apply committed or wrote, and the row records it.
pub const VERDICT_APPLIED: &str = "applied";
/// The verdict a `propose` row carries when the gate refused the change
/// (or a validation named a reason before the gate ran): nothing
/// survived, and the row's gate evidence names the failure.
pub const VERDICT_REFUSED: &str = "refused";
/// The verdict a `propose` row carries when the proposal is sound but
/// waits on the owner: a destructive change, or a confirmed-by memory
/// target, without approval in the current turn. The gate has not run.
pub const VERDICT_WAITING: &str = "waiting";

/// The consolidation action that folds old working memory into an
/// episodic summary. The vocabulary lives here, beside the op, so the
/// writer and the reader name the same three words.
pub const ACTION_COMPRESS: &str = "compress";
/// The consolidation action that retires a stale inferred fact from
/// recall without deleting it.
pub const ACTION_SUPERSEDE: &str = "supersede";
/// The consolidation action that archives an expired fact.
pub const ACTION_FORGET: &str = "forget";
/// Every consolidation action, in the order the surface tallies them.
pub const ACTIONS: [&str; 3] = [ACTION_COMPRESS, ACTION_SUPERSEDE, ACTION_FORGET];

/// One approval decision's verdict, as the row names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenVerdict {
    /// This call was allowed on its own decision.
    Approved,
    /// This call was refused.
    Denied,
    /// This call was allowed by a run-scoped latch — `--yes`, or the
    /// console/TUI `a` key — so no per-call decision happened.
    Always,
}

impl ScreenVerdict {
    /// Every verdict, in the order the surface tallies them.
    pub const ALL: [ScreenVerdict; 3] =
        [ScreenVerdict::Approved, ScreenVerdict::Denied, ScreenVerdict::Always];

    /// The word the row carries.
    pub fn label(self) -> &'static str {
        match self {
            ScreenVerdict::Approved => "approved",
            ScreenVerdict::Denied => "denied",
            ScreenVerdict::Always => "always",
        }
    }
}

/// Why a run ended, named the way the session log names it: an orderly
/// stop appends `end` with `completed` or `max-turns`, and a log without
/// `end` was `interrupted` (`src/session.rs`). A failure that left the log
/// in that shape is therefore `interrupted`; an API failure that reached
/// the loop's own end is `failed`. Carried verbatim so one reader grades
/// both stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStop {
    Completed,
    Interrupted,
    MaxTurns,
    Failed,
}

impl TurnStop {
    /// Every ending, in the vocabulary's own order.
    pub const ALL: [TurnStop; 4] = [
        TurnStop::Completed,
        TurnStop::Interrupted,
        TurnStop::MaxTurns,
        TurnStop::Failed,
    ];

    /// The word the row carries.
    pub fn label(self) -> &'static str {
        match self {
            TurnStop::Completed => "completed",
            TurnStop::Interrupted => "interrupted",
            TurnStop::MaxTurns => "max-turns",
            TurnStop::Failed => "failed",
        }
    }

    /// The grade: `ok` for an orderly stop, `failed` for an ending that
    /// did not finish the task.
    pub fn outcome(self) -> &'static str {
        match self {
            TurnStop::Completed | TurnStop::MaxTurns => "ok",
            TurnStop::Interrupted | TurnStop::Failed => "failed",
        }
    }
}

// --- the log root ----------------------------------------------------------

/// dev-decisions' own log root: `<user home>/.local/share/dev-decisions/logs`.
///
/// dev-decisions pins this as a literal in `scripts/devdec/config.py`
/// (`LOG_DIR`) and reads and writes `<LOG_DIR>/<YYYY>/<MM>/<DD>/events.jsonl`
/// from there. **No env var and no config key names it** — verified by
/// grepping the package for `LOG_DIR`, `os.environ`, and the config
/// DEFAULTS: the only overridable path in the whole config surface is
/// `providers.*`, and the CLI's `--log-dir` flag is a hidden argument on
/// `semantic-index` alone. Fleet therefore reproduces the same literal
/// rather than inventing a second home for its rows.
pub fn devdec_log_root(user_home: Option<&str>) -> Option<PathBuf> {
    user_home.map(|home| {
        PathBuf::from(home).join(".local").join("share").join("dev-decisions").join("logs")
    })
}

/// Where the stream lives, resolved in this order:
///
/// 1. `<FLEET_HOME>/config.json`'s `calib.log_root` — an explicit override,
///    for a machine that keeps its dev-decisions tree elsewhere.
/// 2. dev-decisions' own convention, [`devdec_log_root`]. This is the
///    default because the point of the stream is that the external `devdec`
///    CLI reads Fleet's rows beside its own.
/// 3. `<FLEET_HOME>/logs` — the fallback when no user home is known, so the
///    root always resolves.
///
/// An override that cannot be created is not a resolution failure: the
/// writer counts the append failure and the run continues (see
/// [`Stream::open`]).
pub fn resolve_root(home: &Path, user_home: Option<&str>) -> PathBuf {
    if let Some(root) = crate::route::calib_log_root(home) {
        return root;
    }
    devdec_log_root(user_home).unwrap_or_else(|| home.join("logs"))
}

/// One day's file under `root`: `<root>/<YYYY>/<MM>/<DD>/events.jsonl`, the
/// layout dev-decisions writes and reads. A `day` that is not
/// `YYYY-MM-DD` has no path — the caller passes [`today`]'s output or a
/// validated `--day`.
pub fn day_path(root: &Path, day: &str) -> PathBuf {
    let mut parts = day.split('-');
    let (Some(year), Some(month), Some(day_of_month)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return root.join("events.jsonl");
    };
    root.join(year).join(month).join(day_of_month).join("events.jsonl")
}

/// Today's UTC day key, `YYYY-MM-DD` — the stream's unit of time. An
/// unreadable clock is the empty string, which resolves to the root's
/// bare `events.jsonl` rather than a wrong day.
pub fn today() -> String {
    crate::session::iso_utc_stamp(SystemTime::now())
        .map(|stamp| stamp.chars().take(10).collect())
        .unwrap_or_default()
}

/// A UTC stamp for a row, in the session log's own format.
pub fn stamp() -> String {
    crate::session::iso_utc_stamp(SystemTime::now()).unwrap_or_default()
}

// --- hashing ---------------------------------------------------------------

/// A SHA-256 over `text`, returned as the 16 hex characters
/// dev-decisions joins on (`hashlib.sha256(x).hexdigest()[:16]`). Hand-
/// rolled: the crate carries no crypto dependency and needs exactly one
/// primitive, and the join only works if the algorithm matches.
pub fn hash16(text: &str) -> String {
    let digest = sha256::digest(text.as_bytes());
    let mut out = String::with_capacity(16);
    for byte in &digest[..8] {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// SHA-256 (FIPS 180-4), the whole digest, for the crate's internal use.
mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    /// The 32-byte digest of `data`.
    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut state = H0;
        let bit_length = (data.len() as u64).wrapping_mul(8);
        let mut message = data.to_vec();
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&bit_length.to_be_bytes());

        for block in message.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (index, word) in block.chunks_exact(4).enumerate() {
                w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for index in 16..64 {
                let s0 = w[index - 15].rotate_right(7) ^ w[index - 15].rotate_right(18)
                    ^ (w[index - 15] >> 3);
                let s1 = w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19)
                    ^ (w[index - 2] >> 10);
                w[index] = w[index - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[index - 7])
                    .wrapping_add(s1);
            }

            let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
            let (mut e, mut f, mut g, mut h) = (state[4], state[5], state[6], state[7]);
            for index in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 =
                    h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[index]).wrapping_add(w[index]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);
                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }
            state[0] = state[0].wrapping_add(a);
            state[1] = state[1].wrapping_add(b);
            state[2] = state[2].wrapping_add(c);
            state[3] = state[3].wrapping_add(d);
            state[4] = state[4].wrapping_add(e);
            state[5] = state[5].wrapping_add(f);
            state[6] = state[6].wrapping_add(g);
            state[7] = state[7].wrapping_add(h);
        }

        let mut out = [0u8; 32];
        for (index, word) in state.iter().enumerate() {
            out[4 * index..4 * index + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

// --- the writer ------------------------------------------------------------

/// The queue's bound. Deep enough that a busy run never fills it, shallow
/// enough that a wedged writer cannot buffer the machine's memory.
pub const QUEUE_CAPACITY: usize = 1024;

/// How long the last handle waits for the writer to drain what is already
/// queued. A bounded wait: the stream never holds a process open, and a
/// writer that cannot write (a full disk) cannot stall a shutdown.
const DRAIN_BUDGET: Duration = Duration::from_secs(2);

/// The writer's live counters, shared with the thread that owns the file.
#[derive(Debug, Default)]
struct Counters {
    /// Rows accepted into the queue and not yet finished by the writer.
    pending: AtomicUsize,
    written: AtomicUsize,
    failed: AtomicUsize,
    dropped: AtomicUsize,
    /// True once the writer has finished everything, including its final
    /// state write. A stream whose writer never started sets it at once,
    /// so a teardown never waits on a thread that does not exist.
    done: AtomicBool,
}

/// What the writer shares with every handle: the root, its counters, and
/// the queue's receiving end.
struct Inner {
    root: PathBuf,
    counters: Arc<Counters>,
    /// The queue's receiving end, shared with the writer so a shutdown
    /// that exhausts its drain budget can pull the still-buffered rows
    /// and count them rather than lose them silently — see
    /// [`count_undrained`].
    receiver: Arc<Mutex<Receiver<Row>>>,
}

/// The calibration stream: a bounded queue drained by one background
/// thread that serializes every append.
///
/// Cloning shares one queue and one writer, so a run's route row, its
/// screening rows, and its turn row all cross the same thread. The handle
/// is cheap and `Send`, which is what lets the agent carry it.
#[derive(Clone)]
pub struct Stream {
    /// `None` once this handle has closed the queue (the last handle's
    /// drop); the writer then drains and exits.
    queue: Option<Arc<SyncSender<Row>>>,
    inner: Arc<Inner>,
}

/// The stream's `Debug`: the root and its counters, which is what a log
/// line or a test failure wants to see — never the queue.
impl fmt::Debug for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream")
            .field("root", &self.inner.root)
            .field("written", &self.written())
            .field("dropped", &self.dropped())
            .field("failed", &self.failed())
            .finish()
    }
}

impl Stream {
    /// A writer under `root`. The directory is created lazily, by the
    /// writer thread, one day at a time: opening a stream never touches
    /// the disk, and a root that cannot be created surfaces as a counted
    /// append failure rather than a blocked run.
    pub fn open(root: impl Into<PathBuf>) -> Stream {
        Stream::with_capacity(root, QUEUE_CAPACITY)
    }

    /// A writer under `root` with an explicit queue bound. The bound is
    /// the stream's backpressure policy and nothing else: a full queue
    /// drops a row, it never blocks the caller.
    pub fn with_capacity(root: impl Into<PathBuf>, capacity: usize) -> Stream {
        let root = root.into();
        let counters = Arc::new(Counters::default());
        let (queue, receiver) = std::sync::mpsc::sync_channel::<Row>(capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        let spawned = thread::Builder::new()
            .name("fleet-calib".to_string())
            .spawn({
                let root = root.clone();
                let counters = Arc::clone(&counters);
                let receiver = Arc::clone(&receiver);
                move || drain(root, receiver, counters)
            });
        // A thread that could not spawn leaves the queue disconnected:
        // every later record counts as a drop, which is the honest shape
        // of "the stream is not running" — and the writer is already
        // finished, so a teardown never waits on it.
        if spawned.is_err() {
            counters.done.store(true, Ordering::Relaxed);
            eprintln!(
                "fleet: the calibration writer could not start; rows will be counted as dropped"
            );
        }
        Stream {
            queue: Some(Arc::new(queue)),
            inner: Arc::new(Inner { root, counters, receiver }),
        }
    }

    /// Enqueue one row. Never blocks and never fails the caller: a full
    /// queue drops the row, counts it, and logs one line. This is the
    /// whole reason the stream exists behind a queue — a decision is never
    /// blocked or failed by its own audit trail.
    pub fn record(&self, row: Row) {
        let Some(queue) = &self.queue else { return };
        match queue.try_send(row) {
            Ok(()) => {
                self.inner.counters.pending.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Full(row)) => {
                self.inner.counters.dropped.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "fleet: the calibration queue is full ({} rows); one `{}` row dropped \u{2014} the run is unaffected",
                    QUEUE_CAPACITY,
                    row.op()
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                self.inner.counters.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// The stream's root, for the reader and for `fleet calib`.
    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    /// Rows the writer has appended.
    pub fn written(&self) -> usize {
        self.inner.counters.written.load(Ordering::Relaxed)
    }

    /// Rows the queue could not take (full, or no writer).
    pub fn dropped(&self) -> usize {
        self.inner.counters.dropped.load(Ordering::Relaxed)
    }

    /// Rows the writer could not append (a full disk, permissions).
    pub fn failed(&self) -> usize {
        self.inner.counters.failed.load(Ordering::Relaxed)
    }

    /// Rows accepted into the queue that the writer has not finished yet:
    /// the stream's depth, and what a shutdown is still waiting on.
    pub fn pending(&self) -> usize {
        self.inner.counters.pending.load(Ordering::Relaxed)
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // Only the last handle tears the writer down: closing the queue
        // lets it drain the rows already accepted and exit.
        if Arc::strong_count(&self.inner) != 1 {
            return;
        }
        self.queue.take();
        let deadline = Instant::now() + DRAIN_BUDGET;
        while self.inner.counters.pending.load(Ordering::Relaxed) > 0
            || !self.inner.counters.done.load(Ordering::Relaxed)
        {
            if Instant::now() >= deadline {
                // The budget is spent: pull every row still buffered in
                // the queue and count it as dropped, so a wedged writer's
                // leftovers are named rather than silently lost. The rows
                // the writer has already taken (it is mid-append on them)
                // stay the writer's — counting them here too would
                // double-count a row it still writes.
                let uncounted = count_undrained(&self.inner.receiver, &self.inner.counters);
                // Persist the counted drops now: the writer is what ran
                // out of budget and may never reach its own final write,
                // and the counter's whole point is that a later
                // `fleet calib` reads it.
                let state = WriterState {
                    written: self.inner.counters.written.load(Ordering::Relaxed),
                    dropped: self.inner.counters.dropped.load(Ordering::Relaxed),
                    failed: self.inner.counters.failed.load(Ordering::Relaxed),
                };
                write_state(&self.inner.root, &state);
                eprintln!(
                    "fleet: the calibration writer did not drain within {}s; {} queued row(s) were not written (the run is unaffected)",
                    DRAIN_BUDGET.as_secs(),
                    uncounted
                );
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

/// The queue's lock, recovered from poison: the only critical section is
/// a `recv`/`try_recv`, which cannot panic, so a poisoned lock means some
/// unrelated panic tripped the flag and the queue is still sound. The
/// crate's rule is that a tool or API failure is a value, never a panic,
/// and a lock is the last place to introduce one.
fn lock_queue(queue: &Mutex<Receiver<Row>>) -> std::sync::MutexGuard<'_, Receiver<Row>> {
    queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A shutdown that has spent its drain budget pulls every row still
/// buffered in the queue and counts it as dropped, so a wedged writer's
/// leftovers are named rather than silently lost. The rows the writer has
/// already taken are left alone — it owns them and may still write them,
/// and counting them here too would double-count. Returns how many were
/// counted. This is the seam the drain-budget path is tested against.
fn count_undrained(queue: &Mutex<Receiver<Row>>, counters: &Counters) -> usize {
    let receiver = lock_queue(queue);
    let mut counted = 0;
    while let Ok(_row) = receiver.try_recv() {
        counters.dropped.fetch_add(1, Ordering::Relaxed);
        counters.pending.fetch_sub(1, Ordering::Relaxed);
        counted += 1;
    }
    counted
}

/// The writer thread: one row at a time, in arrival order, until the
/// queue closes.
fn drain(root: PathBuf, queue: Arc<Mutex<Receiver<Row>>>, counters: Arc<Counters>) {
    let mut persisted = WriterState::default();
    loop {
        // Take one row under the lock, then release it before the
        // (possibly slow) append: a shutdown that has run out of drain
        // budget must still be able to reach the rows still buffered.
        let received = { lock_queue(&queue).recv() };
        let Ok(row) = received else { break };
        let appended = append(&root, &row);
        match &appended {
            Ok(()) => {
                counters.written.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                counters.failed.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "fleet: the calibration append failed (best-effort, the run is unaffected): {error}"
                );
            }
        }
        // The counters ride in the shared struct, so a drop counted on a
        // caller's thread is visible here; persist whenever one moved.
        let state = WriterState {
            written: counters.written.load(Ordering::Relaxed),
            dropped: counters.dropped.load(Ordering::Relaxed),
            failed: counters.failed.load(Ordering::Relaxed),
        };
        if state.dropped != persisted.dropped || state.failed != persisted.failed {
            write_state(&root, &state);
            persisted = state;
        }
        counters.pending.fetch_sub(1, Ordering::Relaxed);
    }
    // The writer's last act: the final counters, so a later `fleet calib`
    // reads what this process actually did with the rows it was given.
    // One write per writer lifetime, on a queue that is already closed.
    write_state(
        &root,
        &WriterState {
            written: counters.written.load(Ordering::Relaxed),
            dropped: counters.dropped.load(Ordering::Relaxed),
            failed: counters.failed.load(Ordering::Relaxed),
        },
    );
    // Everything is finished, including the state write: a teardown
    // waiting on the queue can now stop.
    counters.done.store(true, Ordering::Relaxed);
}

/// Append one row as one JSON line to its own day's file.
///
/// The day comes from the row's own timestamp, so a row written at
/// 23:59:59 lands in the day it was stamped for even if the writer thread
/// gets to it after midnight.
fn append(root: &Path, row: &Row) -> io::Result<()> {
    let day = row.ts().get(..10).unwrap_or_default();
    let path = day_path(root, day);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(row)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    line.push('\n');
    // One write, not two: under O_APPEND a single write_all is atomic
    // enough that two fleet processes appending side by side cannot
    // interleave into a corrupt line.
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()
}

// --- the writer's persisted state ------------------------------------------

/// The writer's counters as a later process reads them: what the stream
/// dropped and failed, so `fleet calib` can show the stream's health
/// rather than only its contents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WriterState {
    pub written: usize,
    pub dropped: usize,
    pub failed: usize,
}

/// Where the counters live: `<root>/fleet-stream-state.json`. A `.json`
/// name, so dev-decisions' own readers (`rglob("events.jsonl")`,
/// `rglob("*.jsonl")`) never pick it up.
pub fn state_path(root: &Path) -> PathBuf {
    root.join("fleet-stream-state.json")
}

/// Read the persisted counters. Any failure reads as zeros: this is
/// telemetry, never a contract.
pub fn read_state(root: &Path) -> WriterState {
    fs::read_to_string(state_path(root))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// A per-write serial for the state file's temp name, so two fleet
/// processes — or two writes in one process — never share a temp file and
/// rename a corrupt one over the good state.
static STATE_WRITE_SERIAL: AtomicU64 = AtomicU64::new(0);

/// The unique temp path for one state write: the state file's own name
/// with `.<pid>.<serial>.tmp` appended, so the final name stays
/// `fleet-stream-state.json` and no two writers collide on a temp.
fn temp_state_path(path: &Path) -> PathBuf {
    let serial = STATE_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{}.{}.tmp", std::process::id(), serial));
    PathBuf::from(name)
}

/// Persist the counters through a unique temp file and a rename, so a
/// reader never sees a half-written state and two writers cannot
/// interleave into one shared temp. Best-effort throughout: a disk that
/// cannot take one more small file is named once, and nothing else
/// changes.
fn write_state(root: &Path, state: &WriterState) {
    let Ok(text) = serde_json::to_string_pretty(state) else { return };
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let temp = temp_state_path(&path);
    if fs::write(&temp, text).is_ok() {
        let _ = fs::rename(&temp, &path);
    } else {
        eprintln!(
            "fleet: the calibration counters could not be persisted (best-effort, the run is unaffected): {}",
            path.display()
        );
    }
}

// --- the reader ------------------------------------------------------------

/// The longest line the reader takes from a stream file. A day's rows are
/// kilobytes; a line past this is skipped rather than read into memory,
/// the way the MCP client caps its frames (KI-035).
const MAX_LINE_BYTES: u64 = 1 << 20;

/// One row read back off the stream, owned so it can outlive the file.
///
/// Every field is defaulted: dev-decisions' own rows share the file and
/// carry none of Fleet's ops, and a row from a future slice may carry
/// fields this reader does not know. Only the ops Fleet emits are kept
/// (see [`parse_row`]).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadRow {
    #[serde(default)]
    pub ts: String,
    pub op: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub attempts: Vec<ReadAttempt>,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub tool: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub stop_reason: String,
    #[serde(default)]
    pub turns: u64,
    /// The task's hash, carried so slice 7's drafter can join a `turn`
    /// row back to the session log that produced it — the same 16 hex
    /// characters the row was written with.
    #[serde(default)]
    pub input_hash: String,
    /// The failure class a `failed` turn row names, which is what makes
    /// the drafter's failed-turn lesson class-specific rather than
    /// generic — and the reason a `consolidate` row carries: the
    /// justification a supersession was required to record, or the
    /// named bound the cycle acted on.
    #[serde(default)]
    pub reason: String,
    /// The fact a `recall_hit` row names — slice 8's memory feedback.
    #[serde(default)]
    pub fact: String,
    /// The shape a `recall_hit` row routed under; empty when the
    /// instruction was unclassified.
    #[serde(default)]
    pub shape: String,
    /// The composite score a `recall_hit` row ranked at, and the
    /// aggregate score an `eval_corpus` run carried (`passed/total`, in
    /// `[0, 1]`) — one numeric field the two ops share. Zero for every
    /// other op.
    #[serde(default)]
    pub score: f64,
    /// The action a `consolidate` row names: `compress`, `supersede`,
    /// or `forget`. Empty for every other op.
    #[serde(default)]
    pub action: String,
    /// The proposal a `propose` row names, the file stem under
    /// `<FLEET_HOME>/proposals/` or the slice-7 draft's name.
    #[serde(default)]
    pub proposal: String,
    /// The target kind a `propose` row names: `skill`, `memory`,
    /// `source`, or `config`. Empty for every other op.
    #[serde(default)]
    pub target: String,
    /// The target's name or path a `propose` row names. Empty for every
    /// other op.
    #[serde(default)]
    pub target_name: String,
    /// The basis a `propose` row names: `inferred` or `confirmed-by`.
    #[serde(default)]
    pub basis: String,
    /// The gate's per-step evidence a `propose` row carries, in the
    /// order the steps ran. Empty for a `waiting` verdict, which never
    /// reached the gate, and for every other op.
    #[serde(default)]
    pub gate: Vec<ReadGateStep>,
    /// The scenarios that passed in an `eval_corpus` run. Zero for every
    /// other op.
    #[serde(default)]
    pub passed: usize,
    /// The scenarios that failed in an `eval_corpus` run. Zero for every
    /// other op.
    #[serde(default)]
    pub failed: usize,
    /// Every scenario's verdict in an `eval_corpus` run, in corpus order.
    /// Empty for every other op.
    #[serde(default)]
    pub scenarios: Vec<ReadEvalScenario>,
}

/// One scenario's verdict as an `eval_corpus` row carries it: the name,
/// the family, whether it passed, and the failure's name when it did not.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadEvalScenario {
    pub scenario: String,
    pub family: String,
    pub passed: bool,
    #[serde(default)]
    pub failure: String,
}

/// One gate step as a `propose` row carries it: the step's name, whether
/// it passed, and the bounded detail naming the failure when it did not.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadGateStep {
    pub step: String,
    pub ok: bool,
    pub detail: String,
}

/// One attempt as a row carries it: the provider and its outcome, which is
/// all the demotion reader grades.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadAttempt {
    pub provider: String,
    pub outcome: String,
}

/// Read the last `cap` rows of one day's stream, oldest first.
///
/// Fleet's rows only — devdec's own rows in the same file are skipped by
/// op. An unreadable or absent file is an empty read: the reader is
/// advisory, and "no evidence" is never an error.
pub fn read_day(root: &Path, day: &str, cap: usize) -> Vec<ReadRow> {
    if cap == 0 {
        return Vec::new();
    }
    let Ok(file) = File::open(day_path(root, day)) else { return Vec::new() };
    let mut reader = BufReader::new(file);
    let mut tail: VecDeque<ReadRow> = VecDeque::new();
    let mut line: Vec<u8> = Vec::new();
    loop {
        line.clear();
        let read = reader.by_ref().take(MAX_LINE_BYTES + 1).read_until(b'\n', &mut line);
        match read {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        if line.len() as u64 > MAX_LINE_BYTES {
            // Drain the rest of the oversized line, then skip it.
            let mut sink = Vec::new();
            let _ = reader.by_ref().take(MAX_LINE_BYTES + 1).read_until(b'\n', &mut sink);
            continue;
        }
        let Ok(text) = std::str::from_utf8(&line) else { continue };
        if let Some(row) = parse_row(text) {
            if tail.len() == cap {
                tail.pop_front();
            }
            tail.push_back(row);
        }
    }
    tail.into()
}

/// Parse one line into a Fleet row, or `None` for anything else: a
/// dev-decisions row, a malformed line, a blank.
fn parse_row(text: &str) -> Option<ReadRow> {
    let row: ReadRow = serde_json::from_str(text.trim()).ok()?;
    OPS.contains(&row.op.as_str()).then_some(row)
}

/// How many of a day's recent rows the demotion reader grades. The stream
/// is an audit trail, not a training set: the bounded read is what keeps
/// the first reader honest about "recent".
pub const READ_CAP: usize = 512;

/// How many consecutive failed attempts against one candidate demote it.
pub const DEMOTION_THRESHOLD: usize = 3;

/// The candidates `rows` demote: every provider whose trailing streak of
/// consecutive failed attempts is at or over `threshold`. A `served`
/// attempt resets the streak; a `skipped` one leaves it alone. Providers
/// are named in the order they first appear.
pub fn demoted_providers(rows: &[ReadRow], threshold: usize) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut streaks: HashMap<String, usize> = HashMap::new();
    for row in rows.iter().filter(|row| row.op == OP_ROUTE) {
        for attempt in &row.attempts {
            let streak = streaks.entry(attempt.provider.clone()).or_insert(0);
            match attempt.outcome.as_str() {
                "failed" => *streak += 1,
                "served" => *streak = 0,
                _ => {}
            }
            if !order.iter().any(|seen| seen == &attempt.provider) {
                order.push(attempt.provider.clone());
            }
        }
    }
    order
        .into_iter()
        .filter(|provider| streaks.get(provider).copied().unwrap_or(0) >= threshold)
        .collect()
}

/// The calibration stream's first read: which of `candidates` today's
/// rows demote to the end of the failover walk. Bounded — today's file,
/// the last [`READ_CAP`] rows — and a no-op when the stream is unreadable
/// or absent.
pub fn demotions(root: &Path, day: &str, candidates: &[&str]) -> Vec<String> {
    let rows = read_day(root, day, READ_CAP);
    demoted_providers(&rows, DEMOTION_THRESHOLD)
        .into_iter()
        .filter(|provider| candidates.contains(&provider.as_str()))
        .collect()
}

// --- the surface -----------------------------------------------------------

/// The card width, matching `lanes` and `providers`.
const CARD_WIDTH: usize = 72;

/// How many rows `fleet calib` prints verbatim at the end.
const RECENT_ROWS: usize = 5;

/// How many of a day's rows the surface tallies.
const DISPLAY_CAP: usize = 4096;

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const ERROR: &str = "\u{1b}[31m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

// The card's colors, in scope for the format strings below.
use palette::{ACCENT, BOLD, DIM, RESET, TEXT};

/// Render one day's stream as cards: one per op with its counts, the
/// writer's persisted counters, and the last few rows verbatim. Never a
/// plain-text table.
pub fn render(root: &Path, day: &str) -> String {
    let rows = read_day(root, day, DISPLAY_CAP);
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet calib{RESET} {DIM}\u{2014} the calibration stream, {day} (UTC){RESET}\n\n"
    ));
    if rows.is_empty() {
        // "Fleet rows", not "rows": dev-decisions' own rows share the
        // file, and a day with hundreds of theirs and none of ours is a
        // day Fleet has not run.
        out.push_str(&format!(
            "{DIM}no Fleet rows for {day} yet \u{b7} a run appends to {}{RESET}\n",
            day_path(root, day).display()
        ));
        return out;
    }
    for op in OPS {
        if let Some(card) = op_card(op, &rows) {
            out.push_str(&card);
        }
    }
    out.push_str(&stream_card(root, &rows));
    out.push_str(&recent_rows(&rows));
    out.push_str(&format!(
        "\n{DIM}{} row{} \u{b7} {}{RESET}\n",
        rows.len(),
        plural(rows.len()),
        day_path(root, day).display()
    ));
    out
}

/// One op's card, or `None` when the day holds no rows for it.
fn op_card(op: &str, rows: &[ReadRow]) -> Option<String> {
    let op_rows: Vec<&ReadRow> = rows.iter().filter(|row| row.op == op).collect();
    if op_rows.is_empty() {
        return None;
    }
    let detail = match op {
        OP_ROUTE => route_detail(&op_rows),
        OP_TOOL_SCREEN => screen_detail(&op_rows),
        OP_TURN => turn_detail(&op_rows),
        OP_RECALL_HIT => recall_detail(&op_rows),
        OP_CONSOLIDATE => consolidate_detail(&op_rows),
        OP_PROPOSE => propose_detail(&op_rows),
        OP_EVAL_CORPUS => eval_detail(&op_rows),
        _ => Vec::new(),
    };
    let header_right = format!("{} row{}", op_rows.len(), plural(op_rows.len()));
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{op}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
        op = op,
        dashes = dashes_for(&format!(" {op} "), &header_right),
        right = header_right,
    );
    let mut lines: Vec<String> = detail
        .iter()
        .map(|line| format!("{DIM}\u{2502}{RESET} {TEXT}{line}{RESET}"))
        .collect();
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}last {}{RESET}",
        op_rows.last().map(|row| row.ts.as_str()).unwrap_or("")
    ));
    let bottom = format!("{DIM}\u{2514}{}\u{2518}{RESET}", "\u{2500}".repeat(CARD_WIDTH - 2));
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    Some(card)
}

/// The route card's facts: attempts served and failed, and the chain the
/// last decision walked.
fn route_detail(rows: &[&ReadRow]) -> Vec<String> {
    let served: usize = rows
        .iter()
        .flat_map(|row| row.attempts.iter())
        .filter(|attempt| attempt.outcome == "served")
        .count();
    let failed: usize = rows
        .iter()
        .flat_map(|row| row.attempts.iter())
        .filter(|attempt| attempt.outcome == "failed")
        .count();
    // `op_card` calls this only on a non-empty selection, so the empty
    // case is unreachable — but a panic is never the answer: an empty
    // selection renders an empty detail rather than crashing the surface.
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    vec![
        format!("{served} served \u{b7} {failed} failed \u{b7} {} skipped", skipped(rows)),
        format!("last {} \u{b7} {} attempts in the walk", last.provider, last.attempts.len()),
    ]
}

fn skipped(rows: &[&ReadRow]) -> usize {
    rows.iter()
        .flat_map(|row| row.attempts.iter())
        .filter(|attempt| attempt.outcome == "skipped")
        .count()
}

/// The screening card's facts: the verdict tally and the tools seen.
fn screen_detail(rows: &[&ReadRow]) -> Vec<String> {
    let tally = ScreenVerdict::ALL.iter().map(|verdict| {
        let count = rows.iter().filter(|row| row.verdict == verdict.label()).count();
        (count > 0).then(|| format!("{} {}", count, verdict.label()))
    });
    let mut tools: Vec<&str> = rows.iter().map(|row| row.tool.as_str()).collect();
    tools.sort_unstable();
    tools.dedup();
    vec![
        format!("calls: {}", tally.flatten().collect::<Vec<_>>().join(" \u{b7} ")),
        format!("tools: {}", tools.join(" \u{b7} ")),
    ]
}

/// The turn card's facts: the outcome grade, the named endings, and the
/// turns the runs spent.
fn turn_detail(rows: &[&ReadRow]) -> Vec<String> {
    let ok = rows.iter().filter(|row| row.outcome == "ok").count();
    let failed = rows.iter().filter(|row| row.outcome == "failed").count();
    let turns: u64 = rows.iter().map(|row| row.turns).sum();
    vec![
        format!("{ok} ok \u{b7} {failed} failed \u{b7} {turns} turn{}", plural(turns as usize)),
        format!("endings: {}", ending_tally(rows).join(" \u{b7} ")),
    ]
}

/// The recall-hit card's facts: hits, distinct facts recalled, and the
/// last hit — the fact, the shape it routed under, and its score.
fn recall_detail(rows: &[&ReadRow]) -> Vec<String> {
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    let mut facts: Vec<&str> = rows.iter().map(|row| row.fact.as_str()).collect();
    facts.sort_unstable();
    facts.dedup();
    vec![
        format!(
            "{} hit{} \u{b7} {} fact{} recalled",
            rows.len(),
            plural(rows.len()),
            facts.len(),
            plural(facts.len())
        ),
        format!(
            "last {} \u{b7} shape {} \u{b7} score {:.2}",
            last.fact,
            if last.shape.is_empty() { "unclassified" } else { &last.shape },
            last.score
        ),
    ]
}

/// The consolidation card's facts: the action tally and the last
/// action — the fact it touched and the reason it carried.
fn consolidate_detail(rows: &[&ReadRow]) -> Vec<String> {
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    let tally = ACTIONS
        .iter()
        .filter_map(|action| {
            let count = rows.iter().filter(|row| row.action == *action).count();
            (count > 0).then(|| format!("{count} {action}"))
        })
        .collect::<Vec<_>>();
    vec![
        format!("actions: {}", tally.join(" \u{b7} ")),
        format!("last {} `{}` \u{2014} {}", last.action, last.fact, last.reason),
    ]
}

/// The propose card's facts: the verdict tally and the last proposal —
/// what it targeted, the basis it carried, and the gate steps it ran.
/// A `waiting` verdict names the gate as not yet run, which is the
/// distinction the row exists to carry.
fn propose_detail(rows: &[&ReadRow]) -> Vec<String> {
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    let tally = [VERDICT_APPLIED, VERDICT_REFUSED, VERDICT_WAITING]
        .iter()
        .filter_map(|verdict| {
            let count = rows.iter().filter(|row| row.verdict == *verdict).count();
            (count > 0).then(|| format!("{count} {verdict}"))
        })
        .collect::<Vec<_>>();
    let mut lines = vec![
        format!("verdicts: {}", tally.join(" \u{b7} ")),
        format!(
            "last `{}` \u{2192} {} {} ({}) \u{2014} {}",
            last.proposal, last.target, last.target_name, last.basis, last.verdict
        ),
    ];
    if !last.reason.is_empty() {
        lines.push(format!("reason: {}", last.reason));
    }
    if !last.gate.is_empty() {
        let steps = last
            .gate
            .iter()
            .map(|step| format!("{} {}", step.step, if step.ok { "ok" } else { "FAILED" }))
            .collect::<Vec<_>>();
        lines.push(format!("gate: {}", steps.join(" \u{b7} ")));
    } else if last.verdict == VERDICT_WAITING {
        lines.push("gate: not yet run \u{b7} waiting on the owner".to_string());
    }
    lines
}

/// The eval-corpus card's facts: the last run's aggregate score and its
/// pass/fail tally, and the scenarios that failed in it — named, because
/// a corpus failure the surface does not name is the thing the corpus
/// exists to prevent.
fn eval_detail(rows: &[&ReadRow]) -> Vec<String> {
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    let mut lines = vec![format!(
        "last {} passed \u{b7} {} failed \u{b7} score {:.2}",
        last.passed, last.failed, last.score
    )];
    let failed: Vec<&str> = last
        .scenarios
        .iter()
        .filter(|scenario| !scenario.passed)
        .map(|scenario| scenario.scenario.as_str())
        .collect();
    if failed.is_empty() {
        lines.push("every scenario passed".to_string());
    } else {
        lines.push(format!("failed: {}", failed.join(" \u{b7} ")));
    }
    lines
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// The English plural for a count, the way `lanes` and `providers` carry
/// theirs.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// The named endings, in the vocabulary's own order, with their counts.
fn ending_tally(rows: &[&ReadRow]) -> Vec<String> {
    TurnStop::ALL
        .iter()
        .filter_map(|stop| {
            let count = rows.iter().filter(|row| row.stop_reason == stop.label()).count();
            (count > 0).then(|| format!("{} \u{d7}{}", stop.label(), count))
        })
        .collect()
}

/// The stream's own card: where the file is and what the writer did with
/// what it was given.
fn stream_card(root: &Path, rows: &[ReadRow]) -> String {
    let state = read_state(root);
    let header_right = "the writer".to_string();
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}stream{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}",
        dashes = dashes_for(" stream ", &header_right),
        right = header_right,
    );
    let dropped_color = if state.dropped == 0 { palette::OK } else { palette::WARN };
    let failed_color = if state.failed == 0 { palette::OK } else { palette::ERROR };
    let lines = vec![
        format!("{DIM}\u{2502}{RESET} {TEXT}{} rows today{RESET}", rows.len()),
        format!(
            "{DIM}\u{2502}{RESET} {TEXT}written {written} (last writer) \u{b7} {dropped_color}dropped {dropped}{RESET} \u{b7} {failed_color}failed {failed}{RESET}",
            written = state.written,
            dropped_color = dropped_color,
            dropped = state.dropped,
            failed_color = failed_color,
            failed = state.failed,
        ),
    ];
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

/// The last few rows, verbatim: an audit view shows the row, not a summary
/// of it.
fn recent_rows(rows: &[ReadRow]) -> String {
    // The verbatim lines come from the file, so the surface prints what a
    // reader of the stream would read.
    let mut out = format!("{DIM}last {RECENT_ROWS} rows{RESET}\n");
    for row in rows.iter().rev().take(RECENT_ROWS).rev() {
        out.push_str(&format!("{DIM}  {}{RESET}\n", row_summary(row)));
    }
    out
}

/// One row as a single dim line: the stamp, the op, and the facts that
/// identify it. The full row is one `dev-decisions log` away.
fn row_summary(row: &ReadRow) -> String {
    match row.op.as_str() {
        OP_ROUTE => format!(
            "{}  route  {}  {} attempts",
            row.ts,
            row.provider,
            row.attempts.len()
        ),
        OP_TOOL_SCREEN => {
            format!("{}  tool_screen  {}  {}", row.ts, row.tool, row.verdict)
        }
        OP_TURN => format!(
            "{}  turn  {}  {}  {} turns",
            row.ts, row.outcome, row.stop_reason, row.turns
        ),
        OP_RECALL_HIT => format!(
            "{}  recall_hit  {}  {}  {:.2}",
            row.ts,
            row.fact,
            if row.shape.is_empty() { "unclassified" } else { &row.shape },
            row.score
        ),
        OP_CONSOLIDATE => format!(
            "{}  consolidate  {}  {}  {}",
            row.ts, row.action, row.fact, row.reason
        ),
        OP_PROPOSE => format!(
            "{}  propose  {}  {} {}  {}",
            row.ts, row.proposal, row.target, row.target_name, row.verdict
        ),
        OP_EVAL_CORPUS => format!(
            "{}  eval_corpus  {} passed \u{b7} {} failed \u{b7} score {:.2}",
            row.ts, row.passed, row.failed, row.score
        ),
        _ => format!("{}  {}", row.ts, row.op),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch root, unique per call.
    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-calib-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn route_row(attempts: &[(&'static str, &'static str)]) -> Row {
        Row::Route(RouteRow {
            ts: "2026-10-13T12:00:00Z".to_string(),
            provider: attempts.first().map(|(id, _)| id.to_string()).unwrap_or_default(),
            how: "fallback".to_string(),
            workload: None,
            verdict: None,
            reason: "test".to_string(),
            model: "test-model".to_string(),
            providers_used: attempts.iter().map(|(id, _)| id.to_string()).collect(),
            attempts: attempts
                .iter()
                .map(|(id, outcome)| Attempt {
                    provider: id.to_string(),
                    outcome,
                    class: (*outcome == "failed").then_some("server"),
                })
                .collect(),
            input_chars: 4,
            input_hash: hash16("task"),
            latency_ms: 3,
        })
    }

    /// Wait for the writer to finish everything queued. A test that reads
    /// the file the writer just wrote needs this; the drain-on-last-drop
    /// path has its own test.
    fn settle(stream: &Stream) {
        let deadline = Instant::now() + DRAIN_BUDGET;
        while stream.pending() > 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn turn_row(stop: TurnStop, turns: u64) -> Row {
        Row::Turn(TurnRow {
            ts: "2026-10-13T12:00:01Z".to_string(),
            outcome: stop.outcome().to_string(),
            stop_reason: stop.label().to_string(),
            turns,
            usage: Usage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
            input_chars: 4,
            input_hash: hash16("task"),
            reason: None,
        })
    }

    fn screen_row(tool: &str, verdict: ScreenVerdict) -> Row {
        Row::ToolScreen(ToolScreenRow {
            ts: "2026-10-13T12:00:02Z".to_string(),
            tool: tool.to_string(),
            verdict: verdict.label().to_string(),
            lane: None,
            input_chars: 2,
            input_hash: hash16("{}"),
        })
    }

    fn recall_hit_row(fact: &str, shape: &str, score: f64) -> Row {
        Row::RecallHit(RecallHitRow {
            ts: "2026-10-13T12:00:03Z".to_string(),
            fact: fact.to_string(),
            shape: shape.to_string(),
            score,
        })
    }

    fn consolidate_row(action: &str, fact: &str, reason: &str) -> Row {
        Row::Consolidate(ConsolidateRow {
            ts: "2026-10-13T12:00:04Z".to_string(),
            action: action.to_string(),
            fact: fact.to_string(),
            reason: reason.to_string(),
        })
    }

    // --- the vocabulary ---

    #[test]
    fn the_hash_matches_devdecisions_sha16() {
        // sha256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(hash16("abc"), "ba7816bf8f01cfea");
        assert_eq!(hash16(""), "e3b0c44298fc1c14");
    }

    #[test]
    fn each_row_carries_devdecisions_field_names() {
        let route = serde_json::to_string(&route_row(&[("router-kit", "failed")])).unwrap();
        let value: serde_json::Value = serde_json::from_str(&route).unwrap();
        assert_eq!(value["op"], "route");
        for field in [
            "ts",
            "provider",
            "how",
            "reason",
            "model",
            "providers_used",
            "attempts",
            "input_chars",
            "input_hash",
            "latency_ms",
        ] {
            assert!(value.get(field).is_some(), "the route row lacks {field}: {route}");
        }
        assert_eq!(value["attempts"][0]["outcome"], "failed");
        assert_eq!(value["attempts"][0]["class"], "server");

        let screen = serde_json::to_string(&screen_row("bash", ScreenVerdict::Approved)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&screen).unwrap();
        assert_eq!(value["op"], "tool_screen");
        assert_eq!(value["tool"], "bash");
        assert_eq!(value["verdict"], "approved");

        let turn = serde_json::to_string(&turn_row(TurnStop::MaxTurns, 25)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&turn).unwrap();
        assert_eq!(value["op"], "turn");
        assert_eq!(value["outcome"], "ok");
        assert_eq!(value["stop_reason"], "max-turns");
        assert_eq!(value["turns"], 25);
        assert_eq!(value["usage"]["total_tokens"], 15);
    }

    #[test]
    fn the_task_text_never_rides_in_a_row() {
        let row = serde_json::to_string(&Row::Turn(TurnRow {
            ts: "2026-10-13T12:00:01Z".to_string(),
            outcome: "ok".to_string(),
            stop_reason: "completed".to_string(),
            turns: 1,
            usage: Usage::default(),
            input_chars: 19,
            input_hash: hash16("refactor the parser"),
            reason: None,
        }))
        .unwrap();
        assert!(!row.contains("refactor the parser"));
        assert!(row.contains("input_hash"));
    }

    #[test]
    fn the_four_stop_reasons_are_named_verbatim() {
        assert_eq!(TurnStop::Completed.label(), "completed");
        assert_eq!(TurnStop::Interrupted.label(), "interrupted");
        assert_eq!(TurnStop::MaxTurns.label(), "max-turns");
        assert_eq!(TurnStop::Failed.label(), "failed");
        // A partial ending is a failed grade; an orderly stop is not.
        assert_eq!(TurnStop::Completed.outcome(), "ok");
        assert_eq!(TurnStop::MaxTurns.outcome(), "ok");
        assert_eq!(TurnStop::Failed.outcome(), "failed");
        assert_eq!(TurnStop::Interrupted.outcome(), "failed");
    }

    // --- the root ---

    #[test]
    fn the_day_path_is_devdecisions_nested_layout() {
        let root = Path::new("/root");
        assert_eq!(
            day_path(root, "2026-10-13"),
            Path::new("/root/2026/10/13/events.jsonl")
        );
    }

    #[test]
    fn the_devdec_root_is_its_own_literal_convention() {
        let root = devdec_log_root(Some("/Users/x")).unwrap();
        assert_eq!(
            root,
            Path::new("/Users/x/.local/share/dev-decisions/logs")
        );
        assert!(devdec_log_root(None).is_none());
    }

    #[test]
    fn the_root_falls_back_to_fleet_home_without_a_user_home() {
        let home = Path::new("/fleet");
        // No config.json in the scratch home, and no user home: the
        // FLEET_HOME fallback applies.
        assert_eq!(resolve_root(home, None), Path::new("/fleet/logs"));
    }

    // --- the queue ---

    #[test]
    fn the_queue_is_bounded_and_refuses_rather_than_blocks() {
        // The bound is the channel's, and `record` leans on `try_send`:
        // a full queue is a refusal, never a wait. That is the property
        // that keeps a decision from ever being blocked by its own audit
        // trail, so it is asserted on the primitive directly.
        let (queue, _held_receiver) = std::sync::mpsc::sync_channel::<Row>(2);
        queue.try_send(route_row(&[("a", "failed")])).unwrap();
        queue.try_send(route_row(&[("a", "failed")])).unwrap();
        assert!(matches!(
            queue.try_send(route_row(&[("a", "failed")])),
            Err(TrySendError::Full(_))
        ));
    }

    #[test]
    fn a_full_queue_drops_the_row_and_counts_it() {
        let root = scratch("queue");
        // A bound of one: the sender outruns the writer, so the queue
        // fills and the drop counter moves — the row is gone, the run is
        // not.
        let stream = Stream::with_capacity(&root, 1);
        let mut guard = 0;
        while stream.dropped() == 0 {
            stream.record(route_row(&[("a", "failed")]));
            guard += 1;
            assert!(guard < 50_000, "the queue never filled");
        }
        drop(stream);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_root_that_cannot_be_created_counts_a_failure_and_never_fails_a_run() {
        let base = scratch("bad-root");
        // A regular file where the day directory would go: no directory
        // can be created under it.
        let blocker = base.join("not-a-directory");
        std::fs::write(&blocker, "x").unwrap();
        let stream = Stream::open(blocker.join("2026").join("10").join("13"));
        stream.record(route_row(&[("a", "served")]));
        settle(&stream);
        // The append failed and was counted; the run that asked for the
        // row was never told.
        assert!(stream.failed() >= 1, "the unwritable root counted no failure");
        drop(stream);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_writer_serializes_one_row_per_line_in_the_day_file() {
        let root = scratch("writer");
        let stream = Stream::open(&root);
        stream.record(route_row(&[("router-kit", "served")]));
        stream.record(turn_row(TurnStop::Completed, 3));
        stream.record(screen_row("bash", ScreenVerdict::Denied));
        // The last handle's drop drains the queue before the file is read:
        // a row is never lost to a process that is on its way out.
        drop(stream);
        let text = std::fs::read_to_string(day_path(&root, "2026-10-13")).unwrap();
        assert_eq!(text.lines().count(), 3);
        for line in text.lines() {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(OPS.contains(&value["op"].as_str().unwrap()));
        }
        assert!(text.ends_with('\n'));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_row_lands_in_the_day_its_own_stamp_names() {
        let root = scratch("own-day");
        let stream = Stream::open(&root);
        let mut row = route_row(&[("a", "served")]);
        if let Row::Route(inner) = &mut row {
            inner.ts = "2026-01-02T03:04:05Z".to_string();
        }
        stream.record(row);
        drop(stream);
        assert!(day_path(&root, "2026-01-02").exists());
        assert!(!day_path(&root, "2026-10-13").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_persisted_state_is_readable_by_a_later_process() {
        let root = scratch("state");
        let stream = Stream::open(&root);
        stream.record(route_row(&[("a", "served")]));
        drop(stream);
        let state = read_state(&root);
        assert_eq!(state.written, 1);
        assert_eq!(state.dropped, 0);
        assert_eq!(state.failed, 0);
        // The state file is a `.json`, invisible to devdec's own readers.
        assert_eq!(state_path(&root).extension().unwrap(), "json");
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the reader and the demotion ---

    /// Script one day's rows as the stream would have written them.
    fn write_rows(root: &Path, day: &str, rows: &[Row]) {
        let path = day_path(root, day);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut text = String::new();
        for row in rows {
            text.push_str(&serde_json::to_string(row).unwrap());
            text.push('\n');
        }
        // A dev-decisions row shares the file; the reader must skip it.
        text.push_str("{\"op\": \"classify-diff\", \"repo\": \"fleet\", \"verdict\": \"ok\", \"ts\": \"2026-10-13T12:00:09Z\"}\n");
        std::fs::write(&path, text).unwrap();
    }

    #[test]
    fn the_reader_keeps_fleet_rows_and_skips_devdecisions_own() {
        let root = scratch("read");
        write_rows(&root, "2026-10-13", &[route_row(&[("a", "served")]), turn_row(TurnStop::Completed, 1)]);
        let rows = read_day(&root, "2026-10-13", 512);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].op, "route");
        assert_eq!(rows[1].op, "turn");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unreadable_or_absent_stream_is_a_no_op() {
        let root = scratch("absent");
        assert!(read_day(&root, "2026-10-13", 512).is_empty());
        assert!(demotions(&root, "2026-10-13", &["a"]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn three_consecutive_failures_demote_and_two_do_not() {
        let root = scratch("demote");
        let day = "2026-10-13";
        write_rows(&root, day, &vec![route_row(&[("a", "failed")]); 3]);
        assert_eq!(demotions(&root, day, &["a", "b"]), vec!["a".to_string()]);
        write_rows(&root, day, &vec![route_row(&[("a", "failed")]); 2]);
        assert!(demotions(&root, day, &["a"]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_served_attempt_resets_the_streak() {
        let root = scratch("reset");
        let day = "2026-10-13";
        write_rows(
            &root,
            day,
            &[
                route_row(&[("a", "failed")]),
                route_row(&[("a", "failed")]),
                route_row(&[("a", "served")]),
                route_row(&[("a", "failed")]),
                route_row(&[("a", "failed")]),
            ],
        );
        assert!(demotions(&root, day, &["a"]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_skipped_attempt_leaves_the_streak_alone() {
        let root = scratch("skip");
        let day = "2026-10-13";
        write_rows(
            &root,
            day,
            &[
                route_row(&[("a", "failed")]),
                route_row(&[("b", "skipped")]),
                route_row(&[("a", "failed")]),
                route_row(&[("a", "failed")]),
            ],
        );
        assert_eq!(demotions(&root, day, &["a", "b"]), vec!["a".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_read_is_capped_to_the_most_recent_rows() {
        let root = scratch("cap");
        let day = "2026-10-13";
        let mut rows = Vec::new();
        for _ in 0..4 {
            rows.push(route_row(&[("a", "failed")]));
        }
        rows.push(route_row(&[("a", "served")]));
        write_rows(&root, day, &rows);
        // The last three rows carry one failure and one serve: the capped
        // read never sees the older three.
        let read = read_day(&root, day, 3);
        assert_eq!(read.len(), 3);
        assert!(demoted_providers(&read, DEMOTION_THRESHOLD).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the surface ---

    #[test]
    fn the_surface_is_cards_and_never_a_table() {
        let root = scratch("render");
        write_rows(
            &root,
            "2026-10-13",
            &[
                route_row(&[("router-kit", "failed"), ("zai-coding-plan", "served")]),
                screen_row("bash", ScreenVerdict::Approved),
                screen_row("mcp__sys1__classify", ScreenVerdict::Always),
                turn_row(TurnStop::Completed, 2),
                turn_row(TurnStop::Failed, 4),
            ],
        );
        let rendered = render(&root, "2026-10-13");
        assert!(rendered.contains("fleet calib"), "{rendered}");
        assert_eq!(rendered.matches('\u{250c}').count(), 4, "{rendered}");
        assert!(rendered.contains("route"), "{rendered}");
        assert!(rendered.contains("tool_screen"), "{rendered}");
        assert!(rendered.contains("turn"), "{rendered}");
        assert!(rendered.contains("1 ok \u{b7} 1 failed"), "{rendered}");
        assert!(rendered.contains("dropped 0"), "{rendered}");
        // The last-writer counter is labeled honestly beside the file's
        // own row count — they can disagree, because one counts the file
        // and the other is the last process to persist.
        assert!(rendered.contains("(last writer)"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_day_says_so_rather_than_rendering_nothing() {
        let root = scratch("empty");
        let rendered = render(&root, "2026-10-13");
        assert!(rendered.contains("no Fleet rows for 2026-10-13"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the recall_hit op ---

    #[test]
    fn the_recall_hit_row_carries_the_fact_the_shape_and_the_score() {
        let row = serde_json::to_string(&recall_hit_row("fleet-gate-is-local", "build", 0.42)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(value["op"], "recall_hit");
        assert_eq!(value["fact"], "fleet-gate-is-local");
        assert_eq!(value["shape"], "build");
        assert_eq!(value["score"], 0.42);
    }

    #[test]
    fn the_propose_row_carries_the_verdict_and_the_gates_per_step_evidence() {
        let row = serde_json::to_string(&Row::Propose(ProposeRow {
            ts: "2026-10-14T12:00:00Z".to_string(),
            proposal: "fix-the-ranker".to_string(),
            target: "source".to_string(),
            target_name: "src/route.rs".to_string(),
            basis: "inferred".to_string(),
            verdict: VERDICT_REFUSED.to_string(),
            reason: "the gate refused the change: test failed".to_string(),
            gate: vec![
                GateStepRow { step: "fmt".to_string(), ok: true, detail: "ok".to_string() },
                GateStepRow {
                    step: "test".to_string(),
                    ok: false,
                    detail: "error: test failed: the_ranker".to_string(),
                },
            ],
        }))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(value["op"], "propose");
        assert_eq!(value["proposal"], "fix-the-ranker");
        assert_eq!(value["target"], "source");
        assert_eq!(value["target_name"], "src/route.rs");
        assert_eq!(value["basis"], "inferred");
        assert_eq!(value["verdict"], "refused");
        assert_eq!(value["reason"], "the gate refused the change: test failed");
        assert_eq!(value["gate"][0]["step"], "fmt");
        assert_eq!(value["gate"][0]["ok"], true);
        assert_eq!(value["gate"][1]["step"], "test");
        assert_eq!(value["gate"][1]["ok"], false);
        assert_eq!(value["gate"][1]["detail"], "error: test failed: the_ranker");
        // The reader keeps propose rows and reads their gate evidence back.
        let root = scratch("read-propose");
        write_rows(
            &root,
            "2026-10-14",
            &[Row::Propose(ProposeRow {
                ts: "2026-10-14T12:00:00Z".to_string(),
                proposal: "fix-the-ranker".to_string(),
                target: "source".to_string(),
                target_name: "src/route.rs".to_string(),
                basis: "inferred".to_string(),
                verdict: VERDICT_REFUSED.to_string(),
                reason: "test failed".to_string(),
                gate: vec![GateStepRow {
                    step: "test".to_string(),
                    ok: false,
                    detail: "the_ranker".to_string(),
                }],
            })],
        );
        let rows = read_day(&root, "2026-10-14", 512);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].op, OP_PROPOSE);
        assert_eq!(rows[0].proposal, "fix-the-ranker");
        assert_eq!(rows[0].verdict, "refused");
        assert_eq!(rows[0].gate.len(), 1);
        assert_eq!(rows[0].gate[0].step, "test");
        assert!(!rows[0].gate[0].ok);
        assert_eq!(rows[0].gate[0].detail, "the_ranker");
        let _ = std::fs::remove_dir_all(&root);
        // The op list names propose beside the rest.
        assert_eq!(
            OPS,
            [
                "route",
                "tool_screen",
                "turn",
                "recall_hit",
                "consolidate",
                "propose",
                "eval_corpus"
            ]
        );
    }

    #[test]
    fn the_reader_keeps_recall_hit_rows_with_their_shape() {
        let root = scratch("read-recall");
        write_rows(&root, "2026-10-13", &[recall_hit_row("a-fact", "deploy", 0.5)]);
        let rows = read_day(&root, "2026-10-13", 512);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].op, OP_RECALL_HIT);
        assert_eq!(rows[0].fact, "a-fact");
        assert_eq!(rows[0].shape, "deploy");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_surface_cards_the_recall_hit_op_beside_the_others() {
        let root = scratch("render-recall-hit");
        write_rows(
            &root,
            "2026-10-13",
            &[
                recall_hit_row("a-fact", "deploy", 0.5),
                recall_hit_row("b-fact", "", 0.31),
                turn_row(TurnStop::Completed, 1),
            ],
        );
        let rendered = render(&root, "2026-10-13");
        assert!(rendered.contains("recall_hit"), "{rendered}");
        assert!(rendered.contains("2 hits \u{b7} 2 facts recalled"), "{rendered}");
        // An unclassified hit names itself unclassified rather than
        // printing an empty shape.
        assert!(rendered.contains("shape unclassified"), "{rendered}");
        assert!(rendered.contains("score 0.31"), "{rendered}");
        // The verbatim tail carries the row's own facts.
        assert!(rendered.contains("recall_hit  b-fact  unclassified  0.31"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the consolidate op ---

    #[test]
    fn the_consolidate_row_carries_the_action_the_fact_and_the_reason() {
        let row = serde_json::to_string(&consolidate_row(
            ACTION_SUPERSEDE,
            "stale-fact",
            "inferred, never recalled, older than 180 days",
        ))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(value["op"], "consolidate");
        assert_eq!(value["action"], "supersede");
        assert_eq!(value["fact"], "stale-fact");
        assert_eq!(value["reason"], "inferred, never recalled, older than 180 days");
        // The three actions are the vocabulary the writer emits and the
        // reader tallies — one home, beside the op.
        assert_eq!(ACTIONS, ["compress", "supersede", "forget"]);
        assert_eq!(OP_CONSOLIDATE, "consolidate");
    }

    #[test]
    fn the_reader_keeps_consolidate_rows_with_their_action() {
        let root = scratch("read-consolidate");
        write_rows(
            &root,
            "2026-10-13",
            &[
                consolidate_row(ACTION_COMPRESS, "old-note", "working memory past its valid-until"),
                consolidate_row(ACTION_FORGET, "expired-fact", "expired 2026-09-01"),
            ],
        );
        let rows = read_day(&root, "2026-10-13", 512);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].op, OP_CONSOLIDATE);
        assert_eq!(rows[0].action, "compress");
        assert_eq!(rows[0].fact, "old-note");
        assert_eq!(rows[1].action, "forget");
        assert_eq!(rows[1].reason, "expired 2026-09-01");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_surface_cards_the_consolidate_op_with_its_action_tally() {
        let root = scratch("render-consolidate");
        write_rows(
            &root,
            "2026-10-13",
            &[
                consolidate_row(ACTION_COMPRESS, "old-note", "working memory older than 7 days"),
                consolidate_row(
                    ACTION_SUPERSEDE,
                    "stale-fact",
                    "the owner's justified proposal",
                ),
                consolidate_row(ACTION_COMPRESS, "older-note", "working memory past its valid-until"),
            ],
        );
        let rendered = render(&root, "2026-10-13");
        assert!(rendered.contains("consolidate"), "{rendered}");
        assert!(rendered.contains("2 compress \u{b7} 1 supersede"), "{rendered}");
        // The last action's own facts ride the card and the verbatim tail.
        assert!(rendered.contains("last compress `older-note`"), "{rendered}");
        assert!(
            rendered.contains("consolidate  compress  older-note  working memory past its valid-until"),
            "{rendered}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the state file's temp name and the drain-budget expiry ---

    #[test]
    fn the_state_temp_name_is_unique_per_write_and_the_final_name_is_stable() {
        let root = scratch("state-temp");
        // Two writes to one root: each takes a distinct temp name and
        // leaves only the final file behind (the temp is renamed away).
        write_state(&root, &WriterState { written: 1, dropped: 0, failed: 0 });
        write_state(&root, &WriterState { written: 2, dropped: 1, failed: 0 });
        // The final name is the contract; no temp survives a clean write.
        assert_eq!(state_path(&root).extension().unwrap(), "json");
        let leftovers: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
        // The last write wins, read back whole.
        let state = read_state(&root);
        assert_eq!(state.written, 2);
        assert_eq!(state.dropped, 1);
        // Distinct temp names for distinct writes, both derived from the
        // state path and carrying this process's pid.
        let first = temp_state_path(&state_path(&root));
        let second = temp_state_path(&state_path(&root));
        assert_ne!(first, second, "each write takes its own temp name");
        assert!(first.to_string_lossy().contains(&std::process::id().to_string()));
        assert!(first.to_string_lossy().ends_with(".tmp"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn drain_budget_expiry_counts_the_still_queued_rows() {
        // A writer that never runs leaves its rows in the queue; the
        // shutdown seam pulls them out and counts them as dropped rather
        // than losing them silently. This is the seam the drop-budget
        // path calls when the writer cannot finish in time.
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Row>(8);
        let receiver = Mutex::new(receiver);
        let counters = Counters::default();
        for _ in 0..4 {
            sender.try_send(route_row(&[("a", "failed")])).unwrap();
            counters.pending.fetch_add(1, Ordering::Relaxed);
        }
        let counted = count_undrained(&receiver, &counters);
        assert_eq!(counted, 4);
        assert_eq!(counters.dropped.load(Ordering::Relaxed), 4);
        assert_eq!(counters.pending.load(Ordering::Relaxed), 0);
        // A second sweep finds an empty queue and counts nothing more.
        assert_eq!(count_undrained(&receiver, &counters), 0);
        assert_eq!(counters.dropped.load(Ordering::Relaxed), 4);
    }

    #[test]
    #[cfg(unix)]
    fn a_wedged_writer_counts_its_backlog_on_shutdown() {
        // A FIFO with no reader wedges the writer inside its append: the
        // open blocks until a reader appears, so the writer never drains
        // and the shutdown runs out of its budget with rows still queued.
        // End to end, the backlog must be counted, not lost.
        let root = scratch("wedge");
        let fifo = day_path(&root, "2026-10-13");
        std::fs::create_dir_all(fifo.parent().unwrap()).unwrap();
        let made = std::process::Command::new("mkfifo").arg(&fifo).status().unwrap();
        assert!(made.success(), "mkfifo failed");
        let stream = Stream::open(&root);
        // More rows than the wedged writer can take: it blocks on the
        // first, the rest stay queued.
        for _ in 0..5 {
            stream.record(route_row(&[("a", "failed")]));
        }
        // The last handle's drop waits out the drain budget, then counts
        // the backlog and persists it — the writer never reaches its own
        // final write, so the drop's persist is the only one.
        drop(stream);
        let state = read_state(&root);
        assert_eq!(state.written, 0, "the wedged writer appended nothing: {state:?}");
        assert!(state.dropped >= 1, "the backlog was not counted: {state:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the eval_corpus op ---

    fn eval_corpus_row(scenarios: Vec<EvalScenarioRow>) -> Row {
        let passed = scenarios.iter().filter(|scenario| scenario.passed).count();
        let failed = scenarios.len() - passed;
        let score = if scenarios.is_empty() {
            1.0
        } else {
            passed as f64 / scenarios.len() as f64
        };
        Row::EvalCorpus(EvalCorpusRow {
            ts: "2026-10-14T12:00:00Z".to_string(),
            passed,
            failed,
            score,
            scenarios,
        })
    }

    #[test]
    fn the_eval_corpus_row_carries_per_scenario_verdicts_and_the_score() {
        let row = serde_json::to_string(&eval_corpus_row(vec![
            EvalScenarioRow {
                scenario: "two-turn-tool-loop".to_string(),
                family: "tool-loop".to_string(),
                passed: true,
                failure: None,
            },
            EvalScenarioRow {
                scenario: "conversation-fold".to_string(),
                family: "conversation-fold".to_string(),
                passed: false,
                failure: Some("the post-fold request lost the summary".to_string()),
            },
        ]))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(value["op"], "eval_corpus");
        assert_eq!(value["passed"], 1);
        assert_eq!(value["failed"], 1);
        assert_eq!(value["score"], 0.5);
        assert_eq!(value["scenarios"][0]["scenario"], "two-turn-tool-loop");
        assert_eq!(value["scenarios"][0]["passed"], true);
        // A passed scenario omits its failure; a failed one names it.
        assert!(value["scenarios"][0].get("failure").is_none());
        assert_eq!(value["scenarios"][1]["failure"], "the post-fold request lost the summary");
    }

    #[test]
    fn the_reader_keeps_eval_corpus_rows_with_their_scenarios() {
        let root = scratch("read-eval");
        write_rows(
            &root,
            "2026-10-14",
            &[eval_corpus_row(vec![
                EvalScenarioRow {
                    scenario: "session-corpus".to_string(),
                    family: "session-corpus".to_string(),
                    passed: true,
                    failure: None,
                },
                EvalScenarioRow {
                    scenario: "agent-continuation".to_string(),
                    family: "agent-continuation".to_string(),
                    passed: false,
                    failure: Some("the resume lost the tail".to_string()),
                },
            ])],
        );
        let rows = read_day(&root, "2026-10-14", 512);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].op, OP_EVAL_CORPUS);
        assert_eq!(rows[0].passed, 1);
        assert_eq!(rows[0].failed, 1);
        assert_eq!(rows[0].scenarios.len(), 2);
        assert_eq!(rows[0].scenarios[1].scenario, "agent-continuation");
        assert!(!rows[0].scenarios[1].passed);
        assert_eq!(rows[0].scenarios[1].failure, "the resume lost the tail");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_surface_cards_the_eval_corpus_op_and_names_a_failing_scenario() {
        let root = scratch("render-eval");
        write_rows(
            &root,
            "2026-10-14",
            &[eval_corpus_row(vec![
                EvalScenarioRow {
                    scenario: "memory-recall".to_string(),
                    family: "memory".to_string(),
                    passed: true,
                    failure: None,
                },
                EvalScenarioRow {
                    scenario: "conversation-fold".to_string(),
                    family: "conversation-fold".to_string(),
                    passed: false,
                    failure: Some("the summary did not ride".to_string()),
                },
            ])],
        );
        let rendered = render(&root, "2026-10-14");
        assert!(rendered.contains("eval_corpus"), "{rendered}");
        assert!(rendered.contains("1 passed \u{b7} 1 failed \u{b7} score 0.50"), "{rendered}");
        // The failing scenario is named on the card, not just counted.
        assert!(rendered.contains("failed: conversation-fold"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
