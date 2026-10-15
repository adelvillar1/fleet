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
use serde_json::json;

use crate::api::Usage;
use crate::route::Attempt;
use crate::skills::wrap_detail;

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
    /// The judged artifact's identity: the 16 hex characters of the
    /// sha256 over the content this change stages, in the devdec join
    /// shape. It is what makes the verdict explainable — the
    /// explanation beat keys its prose to the exact input the gate
    /// judged, and a drifted artifact is refused rather than explained
    /// (slice 17). `None` for a row written before the field existed,
    /// and for a change that stages no content of its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
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
    /// The judged artifact's identity: the 16 hex characters of the
    /// sha256 over the corpus's own fixed definition — every scenario's
    /// name and family in corpus order, under the crate's version. The
    /// corpus is code, so its artifact is its definition: an explanation
    /// keyed to it names the exact suite that graded the run, and a
    /// corpus that changed since the row was written is a drift the
    /// explanation refuses rather than papers over (slice 17).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
}

/// One explanation beat's row: the verdict that was explained, the
/// artifact it was keyed to, and the ask's telemetry — and never the
/// prose. The explanation text lands on disk at
/// `<home>/explanations/<input_sha256>.md` and is recoverable by its
/// sha; this row is the redacted index of it, the shape
/// dev-decisions' synthesis lane writes (op/input_sha256/explained_op/
/// route/validation/explanation_sha256/explanation_chars/cost_usd/
/// latency_ms) plus Fleet's own selector and the eval-only marker.
///
/// The marker is not decoration: `eval_only: true` is the row saying,
/// in the stream every gate reads, that nothing here gates, blocks, or
/// applies anything. No verdict reads this op — there is no code path
/// by which an explanation changes a gate's outcome.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainRow {
    pub ts: String,
    /// Always `true`. The discipline's own marker: an explanation is
    /// eval-only prose, never a second verdict.
    pub eval_only: bool,
    /// The op of the verdict this explains: `propose` or `eval_corpus`.
    pub explained_op: String,
    /// What was selected: the proposal's name, or the scenario's.
    pub selector: String,
    /// The judged artifact's sha, the 16 hex characters the explained
    /// row carries — the join key, and the explanation file's name.
    pub input_sha256: String,
    /// The sha of the explanation text on disk, so the file's integrity
    /// is checkable without reading it into a log.
    pub explanation_sha256: String,
    /// The explanation's length in characters.
    pub explanation_chars: usize,
    /// The ask's route telemetry, as the lane reported it.
    pub route: ExplainRouteRow,
    /// The ask's validation telemetry, as the lane reported it.
    pub validation: ExplainValidationRow,
    /// What the ask cost, when the path spent anything. Absent on the
    /// direct path, which costs nothing: the reader's zero is the same
    /// fact, and a `null` would break the flat reader that shares this
    /// file with dev-decisions' own rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// The ask's latency in milliseconds, when the lane reported one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// The lane that answered: `blend1`.
    pub lane: String,
}

/// The route half of an explanation row's telemetry.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainRouteRow {
    /// The router's generation score, when it reported one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_generation: Option<f64>,
    /// The path the ask took: `synthesized`, `direct`, or the lane's
    /// own word for it.
    pub path: String,
    /// The answer shape the router picked.
    pub answer_shape: String,
}

/// The validation half of an explanation row's telemetry.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainValidationRow {
    /// The gate's outcome word: `passed`, `failed`, or the lane's own.
    pub outcome: String,
    /// Whether the answer answers the question, when the gate rated it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers_the_question: Option<bool>,
    /// Whether the answer contradicts recorded facts, when rated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contradicts_facts: Option<bool>,
    /// The number check's word, when the gate ran one.
    pub number_check: String,
}

/// One judgment beat's row: the routing decision the sys1 lane graded,
/// the verdict it reached, its confidence, and the reason — and never the
/// task's text, which the route rows are redacted against by design.
///
/// The row is keyed by the judged route row's identity
/// ([`route_key`]), the 16 hex characters of the sha256 over the route
/// row's own canonical form, in devdec's join shape
/// (`sha256(x).hexdigest()[:16]`). The key is what makes the judgment
/// joinable: `fleet judge --agree <key>` addresses one row by it, and the
/// calibration surface reads the day's judgments beside the routes they
/// graded.
///
/// The owner's disposition rides the SAME row, appended again with the
/// key unchanged: the stream is append-only, so a disposition is a later
/// row for the same route, and a reader takes the freshest one per key
/// (see [`judged_rows`]). That is what makes the human seam the ground
/// truth without a rewrite of what the lane judged — the automated
/// verdict stays on the row it was written with, and the disposition
/// overrides it in the math rather than in the record.
#[derive(Debug, Clone, Serialize)]
pub struct RouteJudgedRow {
    pub ts: String,
    /// The judged route row's identity — the join key, and the value
    /// `fleet judge --agree`/`--disagree`/`--waive` addresses.
    pub route_key: String,
    /// The provider the judgment grades: the candidate the route row
    /// names as having served (or the selected one when none did).
    pub provider: String,
    /// The workload shape the judgment grades it *for* — the route row's
    /// own `workload` label, carried through so the calibration is keyed
    /// by (provider × shape) rather than averaged per provider. A
    /// provider that is right for one shape and wrong for another has
    /// two cells, and neither hides the other.
    pub workload: String,
    /// `right` or `wrong` — see [`VERDICT_RIGHT`] and [`VERDICT_WRONG`].
    pub verdict: String,
    /// The lane's confidence in its own workload judgment, in `[0, 1]`.
    /// It is the weight the confidence-weighted wrong-rate applies, so a
    /// confident wrong judgment counts for more than a tentative one.
    pub confidence: f64,
    /// Why, bounded: the lane's workload label, the provider it maps to,
    /// and the outcome evidence the grade rested on.
    pub reason: String,
    /// The owner's disposition, when one has been recorded: `agreed`,
    /// `disagreed`, or `waived` — see [`DISPOSITIONS`]. Absent on a row
    /// the lane's judgment alone wrote, and for a row written before the
    /// field existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    /// What the ask cost, when the path spent anything. Absent on a
    /// disposition row, which spends nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// The ask's latency in milliseconds, when the lane reported one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// The lane that answered: `sys1`.
    pub lane: String,
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
    Explain(ExplainRow),
    RouteJudged(RouteJudgedRow),
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
            Row::Explain(_) => OP_EXPLAIN,
            Row::RouteJudged(_) => OP_ROUTE_JUDGED,
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
            Row::Explain(row) => &row.ts,
            Row::RouteJudged(row) => &row.ts,
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
/// The explanation op: one verdict explained through the blend1 lane,
/// eval-only and keyed to the judged artifact's sha. No gate reads it —
/// the row exists so the prose is auditable beside the verdict, not so
/// anything can act on it.
pub const OP_EXPLAIN: &str = "explain";
/// The judgment op: one routing decision graded against its outcome
/// evidence through the sys1 lane, keyed by the judged route row's
/// identity. The row that closes the calibration loop — it is what the
/// per-provider calibration surface reads and what the failover's
/// preference consults.
pub const OP_ROUTE_JUDGED: &str = "route_judged";
/// The ops this slice emits, in the order `fleet calib` groups them.
pub const OPS: [&str; 9] = [
    OP_ROUTE,
    OP_TOOL_SCREEN,
    OP_TURN,
    OP_RECALL_HIT,
    OP_CONSOLIDATE,
    OP_PROPOSE,
    OP_EVAL_CORPUS,
    OP_EXPLAIN,
    OP_ROUTE_JUDGED,
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

/// The judgment's verdict when the routing decision was right: the lane's
/// workload label mapped to the candidate that served, or to a candidate
/// that was tried and failed on an availability class while a later one
/// served in its place.
pub const VERDICT_RIGHT: &str = "right";
/// The judgment's verdict when the routing decision was wrong: the lane's
/// workload label mapped to a different candidate than the one that
/// served, and that candidate was never tried or failed in a way that
/// was its own.
pub const VERDICT_WRONG: &str = "wrong";
/// Every judgment verdict, in the order the surface tallies them.
pub const JUDGMENT_VERDICTS: [&str; 2] = [VERDICT_RIGHT, VERDICT_WRONG];

/// The owner's disposition confirming the automated verdict: the judge
/// was right, and the owner says so.
pub const DISPOSITION_AGREED: &str = "agreed";
/// The owner's disposition inverting the automated verdict: the judge was
/// wrong, and the owner's grading is the ground truth.
pub const DISPOSITION_DISAGREED: &str = "disagreed";
/// The owner's disposition setting a judged row aside: neither right nor
/// wrong, excluded from the calibration math and counted apart on the
/// card.
pub const DISPOSITION_WAIVED: &str = "waived";
/// Every disposition, in the order the surface tallies them.
pub const DISPOSITIONS: [&str; 3] =
    [DISPOSITION_AGREED, DISPOSITION_DISAGREED, DISPOSITION_WAIVED];

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

    /// Consume the handle, drain the queue, and report what the writer
    /// did with the rows it was given: `(written, failed)`. The last
    /// handle's drop already drains; this names the outcome at the call
    /// site, so a caller whose row did not reach disk (a full or
    /// unwritable root) can act on it rather than assume success. The
    /// counters are read through a clone of their own `Arc`, which does
    /// not keep the writer's `Inner` alive, so the drain still runs and
    /// the counts read back are final.
    pub fn finish(self) -> (usize, usize) {
        let counters = Arc::clone(&self.inner.counters);
        drop(self);
        (
            counters.written.load(Ordering::Relaxed),
            counters.failed.load(Ordering::Relaxed),
        )
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
    /// The route row's workload (`quick`, `hard`, …), the word sys1
    /// judged. Empty for every other op.
    #[serde(default)]
    pub workload: String,
    /// The route row's `how`: `judged`, `fallback`, or `pinned`. Empty
    /// for every other op.
    #[serde(default)]
    pub how: String,
    /// The route row's model, the served candidate's own. Empty for every
    /// other op.
    #[serde(default)]
    pub model: String,
    /// The candidates a route row attempted, in order. Empty for every
    /// other op.
    #[serde(default)]
    pub providers_used: Vec<String>,
    /// The task's length in characters, as a route or turn row recorded
    /// it. Zero for every other op.
    #[serde(default)]
    pub input_chars: usize,
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
    /// The judged artifact's sha a `propose` or `eval_corpus` row
    /// carries, and the key an `explain` row joins on — one field, three
    /// ops, the devdec join shape. Empty for every other op and for a
    /// row written before the field existed.
    #[serde(default)]
    pub input_sha256: String,
    /// The op an `explain` row explains: `propose` or `eval_corpus`.
    /// Empty for every other op.
    #[serde(default)]
    pub explained_op: String,
    /// What an `explain` row selected: the proposal's name, or the
    /// scenario's. Empty for every other op.
    #[serde(default)]
    pub selector: String,
    /// The sha of the explanation text an `explain` row indexes, which
    /// is also its file's name under `<home>/explanations/`. Empty for
    /// every other op.
    #[serde(default)]
    pub explanation_sha256: String,
    /// The explanation's length in characters, as an `explain` row
    /// recorded it. Zero for every other op.
    #[serde(default)]
    pub explanation_chars: usize,
    /// The eval-only marker an `explain` row carries. False for every
    /// other op, which is what makes the marker mean something: no
    /// verdict-bearing row claims it.
    #[serde(default)]
    pub eval_only: bool,
    /// What an `explain` row's ask cost. Zero for a direct-path ask and
    /// for every other op.
    #[serde(default)]
    pub cost_usd: f64,
    /// The ask's latency in milliseconds an `explain` row recorded. Zero
    /// for every other op.
    #[serde(default)]
    pub latency_ms: u64,
    /// The judged route row's identity a `route_judged` row carries — the
    /// devdec join shape, the value `fleet judge --agree` addresses.
    /// Empty for every other op.
    #[serde(default)]
    pub route_key: String,
    /// The lane's confidence in its own workload judgment, as a
    /// `route_judged` row recorded it, in `[0, 1]`. Zero for every other
    /// op.
    #[serde(default)]
    pub confidence: f64,
    /// The owner's disposition a `route_judged` row carries: `agreed`,
    /// `disagreed`, or `waived`. Empty on a row the lane's judgment alone
    /// wrote, and for every other op.
    #[serde(default)]
    pub disposition: String,
    /// The lane that answered a `route_judged` row: `sys1`. Empty for
    /// every other op.
    #[serde(default)]
    pub lane: String,
    /// The ask's route telemetry an `explain` row carries, nested the
    /// way the row writes it. Empty for every other op.
    #[serde(default)]
    pub route: ReadExplainRoute,
    /// The ask's validation telemetry an `explain` row carries. Empty
    /// for every other op.
    #[serde(default)]
    pub validation: ReadExplainValidation,
}

/// One `explain` row's route half, as the reader keeps it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadExplainRoute {
    #[serde(default)]
    pub needs_generation: Option<f64>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub answer_shape: String,
}

/// One `explain` row's validation half, as the reader keeps it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadExplainValidation {
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub answers_the_question: Option<bool>,
    #[serde(default)]
    pub contradicts_facts: Option<bool>,
    #[serde(default)]
    pub number_check: String,
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

/// One attempt as a row carries it: the provider, its outcome, and the
/// failure class when it failed (`no-key`, `disabled`, or `calibration`
/// when skipped) — all the demotion reader and the judgment's grade
/// need.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReadAttempt {
    pub provider: String,
    pub outcome: String,
    #[serde(default)]
    pub class: Option<String>,
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

// --- the judgment's join key and the calibration read ----------------------

/// A route row's identity: one canonical line over the fields the reader
/// carries, hashed to the 16 hex characters dev-decisions joins on
/// (`sha256(x).hexdigest()[:16]`).
///
/// The identity is built from the reader's shape rather than the
/// writer's on purpose: the judgment reads the day's rows off the stream,
/// `fleet judge --agree <key>` addresses one by the same key, and a
/// later reader recomputes it from the same fields — so the key the beat
/// wrote and the key an owner types are the same value. It is stable for
/// as long as the field set is; adding a field to the line is a key
/// migration, which is why the set is fixed here and documented rather
/// than "whatever the row happens to carry".
pub fn route_key(row: &ReadRow) -> String {
    let attempts = row
        .attempts
        .iter()
        .map(|attempt| format!("{}:{}", attempt.provider, attempt.outcome))
        .collect::<Vec<_>>()
        .join(",");
    let identity = format!(
        "route|{ts}|{provider}|{how}|{workload}|{verdict}|{reason}|{model}|{used}|{attempts}|{chars}|{hash}|{latency}",
        ts = row.ts,
        provider = row.provider,
        how = row.how,
        workload = row.workload,
        verdict = row.verdict,
        reason = row.reason,
        model = row.model,
        used = row.providers_used.join(","),
        attempts = attempts,
        chars = row.input_chars,
        hash = row.input_hash,
        latency = row.latency_ms,
    );
    hash16(&identity)
}

/// The day's judged rows, one per route key — the freshest, because the
/// stream is append-only and a later row for a key (a re-judgment, or the
/// owner's disposition) is the one that stands. Order is first
/// appearance, so a surface that walks the result reads the day in the
/// order it happened.
pub fn judged_rows(rows: &[ReadRow]) -> Vec<ReadRow> {
    let mut order: Vec<String> = Vec::new();
    let mut freshest: HashMap<String, ReadRow> = HashMap::new();
    for row in rows.iter().filter(|row| row.op == OP_ROUTE_JUDGED) {
        let key = row.route_key.clone();
        if !freshest.contains_key(&key) {
            order.push(key.clone());
        }
        freshest.insert(key, row.clone());
    }
    order.into_iter().filter_map(|key| freshest.remove(&key)).collect()
}

/// How many judged rows a provider's sample must carry before its floor
/// is trusted.
///
/// Sized for fleet's volume, not the sibling's: dev-decisions grades many
/// heads across many tasks a day and asks for 20 rows; fleet is one
/// owner's tool, where a day's route rows are the runs they did — single
/// digits on a quiet day, low double digits on a heavy one. Twenty rows
/// would take a week to gather and the calibration would never inform
/// anything. Eight is reachable within a day or two, and it is chosen so
/// the floor cannot be crossed by fewer than **three** wrong judgments:
/// two wrong of eight is 0.25, below the 0.50 floor, and three is 0.375 —
/// still below it unweighted, so a bare-majority-of-a-small-sample swing
/// never moves routing on its own.
pub const FIT_MIN_ROWS: usize = 8;

/// How much confidence span a provider's judged sample must carry before
/// its floor is trusted: `max(confidence) - min(confidence)` across the
/// graded rows, at least this wide.
///
/// The confidence-weighted wrong-rate weights each judgment by the
/// lane's own confidence, so a sample whose confidences are all the same
/// number carries no information about how confidence relates to
/// correctness — the weighting degenerates into an unweighted rate and
/// the "confidence-weighted" claim is vacuous. The sibling asks for 0.4
/// over its volume; fleet's rows per day are fewer and a decision model
/// answering over one compact evidence record clusters high, so 0.4
/// would rarely be reachable at eight rows and the fit rule would refuse
/// samples that are genuinely informative. 0.30 is the span at which the
/// weighting is doing real work at fleet's sample sizes.
pub const FIT_MIN_SPAN: f64 = 0.30;

/// The confidence-weighted wrong-rate at or above which a **fit**
/// provider is skipped in the failover's preference.
///
/// The sibling's calibration is a report and carries no such floor;
/// fleet's ride-back needs one, and 0.50 is the most conservative
/// crossing that still means something: weighted by the judge's own
/// confidence, the provider is wrong at least as often as it is right. A
/// lower floor would move routing on a provider that is more right than
/// wrong, which is not what "conservative" means.
pub const WRONG_RATE_FLOOR: f64 = 0.50;

/// Whether a provider's judged sample is fit to have its floor trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitState {
    /// Enough graded rows and enough confidence span: the floor is
    /// trusted, and the wrong-rate is a score rather than a noise.
    Fit,
    /// Not yet: the sample is too small, too flat, or both. The surface
    /// names what it still needs and renders no score.
    Unfit,
}

impl FitState {
    /// The word the surface and the row carry.
    pub fn label(self) -> &'static str {
        match self {
            FitState::Fit => "fit",
            FitState::Unfit => "unfit",
        }
    }
}

/// One calibration cell: a provider's judged quality **for one workload
/// shape**. The cell, not the provider, is the unit of calibration,
/// because a keyword or phrase in the wrong context is meaningless: a
/// provider that is right for `hard` and wrong for `prose` has two
/// cells, and averaging them into one wrong-rate would hide exactly the
/// split the owner's directive names (2026-10-14).
///
/// Every number on a cell — the counts, the confidence-weighted
/// wrong-rate, the cost per correct call, and the fit requirement — is
/// computed inside it. A provider with twenty judged rows across three
/// shapes has three cells, each with its own count, span, and fit state.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationCell {
    pub provider: String,
    /// The workload shape this cell grades, the route row's own label.
    /// Empty for a route that carried no judged workload.
    pub workload: String,
    /// Every judged row in this cell, including waived ones.
    pub judged: usize,
    /// The rows that graded right after the owner's disposition is
    /// applied.
    pub right: usize,
    /// The rows that graded wrong after the owner's disposition is
    /// applied.
    pub wrong: usize,
    /// The rows the owner set aside: neither right nor wrong, excluded
    /// from the rate.
    pub waived: usize,
    /// The rows whose verdict the owner's disposition flipped, which is
    /// the human seam's footprint in the math.
    pub overridden: usize,
    /// `sum(confidence * wrong) / sum(confidence)` over the graded rows,
    /// or the plain wrong fraction when every confidence is zero (a
    /// sample that reported no confidence at all).
    pub weighted_wrong_rate: f64,
    /// `max(confidence) - min(confidence)` across the graded rows.
    pub confidence_span: f64,
    pub fit: FitState,
    /// What the day's judgments in this cell cost.
    pub spend_usd: f64,
    /// The spend divided by the right count, or `None` when nothing
    /// graded right — a cell with no correct turn has no cost per
    /// correct turn to report.
    pub cost_per_right: Option<f64>,
}

impl CalibrationCell {
    /// How many more graded rows this cell needs before it is fit.
    pub fn needs_rows(&self) -> usize {
        FIT_MIN_ROWS.saturating_sub(self.right + self.wrong)
    }

    /// How much more confidence span this cell needs before it is fit.
    pub fn needs_span(&self) -> f64 {
        (FIT_MIN_SPAN - self.confidence_span).max(0.0)
    }

    /// Whether this cell may move the failover's preference: fit, and
    /// its confidence-weighted wrong-rate at or over
    /// [`WRONG_RATE_FLOOR`].
    pub fn informs_failover(&self) -> bool {
        self.fit == FitState::Fit && self.weighted_wrong_rate >= WRONG_RATE_FLOOR
    }

    /// The shape this cell grades, as the surface names it. A route with
    /// no judged workload says so rather than printing an empty shape.
    pub fn shape(&self) -> &str {
        if self.workload.is_empty() {
            "unshaped"
        } else {
            &self.workload
        }
    }
}

/// The cell for one (provider, workload shape) pair, or `None` when the
/// day holds no judged rows for it. The ride-back's only lookup: a
/// provider's shape-Y record never answers a shape-X question.
pub fn cell<'a>(
    cells: &'a [CalibrationCell],
    provider: &str,
    workload: &str,
) -> Option<&'a CalibrationCell> {
    cells.iter().find(|cell| cell.provider == provider && cell.workload == workload)
}

/// The day's calibration cells, keyed by (provider × workload shape), in
/// the order the judged rows first name each pair.
///
/// The owner's disposition is applied here and only here: `agreed`
/// confirms the automated verdict, `disagreed` inverts it, and `waived`
/// sets the row aside entirely. The judge is a signal; the owner's
/// grading is the ground truth.
pub fn calibration(rows: &[ReadRow]) -> Vec<CalibrationCell> {
    let judged = judged_rows(rows);
    let mut order: Vec<(String, String)> = Vec::new();
    let mut grouped: HashMap<(String, String), Vec<&ReadRow>> = HashMap::new();
    for row in &judged {
        let key = (row.provider.clone(), row.workload.clone());
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(row);
    }
    order
        .into_iter()
        .filter_map(|key| grouped.remove(&key))
        .map(|rows| summarize(&rows[0].provider, &rows[0].workload, &rows))
        .collect()
}

/// One cell's summary from its judged rows.
fn summarize(provider: &str, workload: &str, rows: &[&ReadRow]) -> CalibrationCell {
    let mut right = 0usize;
    let mut wrong = 0usize;
    let mut waived = 0usize;
    let mut overridden = 0usize;
    let mut weight_sum = 0.0f64;
    let mut weighted_wrong = 0.0f64;
    let mut spend = 0.0f64;
    let mut confidences: Vec<f64> = Vec::new();
    for row in rows {
        spend += row.cost_usd;
        let automated = row.verdict.as_str();
        let graded = match row.disposition.as_str() {
            DISPOSITION_WAIVED => {
                waived += 1;
                None
            }
            DISPOSITION_DISAGREED => {
                overridden += 1;
                Some(invert(automated))
            }
            // `agreed` and an absent disposition both stand on the
            // automated verdict; an unknown word is read as an absent
            // one rather than silently waiving the row.
            _ => Some(automated.to_string()),
        };
        let Some(graded) = graded else { continue };
        confidences.push(row.confidence);
        if graded == VERDICT_WRONG {
            wrong += 1;
            weighted_wrong += row.confidence;
        } else {
            right += 1;
        }
        weight_sum += row.confidence;
    }
    let graded_count = right + wrong;
    let weighted_wrong_rate = if weight_sum > 0.0 {
        weighted_wrong / weight_sum
    } else if graded_count > 0 {
        // A sample that reported no confidence at all: the weighting has
        // nothing to weight by, so the honest rate is the plain fraction.
        wrong as f64 / graded_count as f64
    } else {
        0.0
    };
    let confidence_span = match confidences.iter().copied().fold(None::<(f64, f64)>, |bounds, confidence| {
        Some(match bounds {
            None => (confidence, confidence),
            Some((low, high)) => (low.min(confidence), high.max(confidence)),
        })
    }) {
        Some((low, high)) => high - low,
        None => 0.0,
    };
    let fit = if graded_count >= FIT_MIN_ROWS && confidence_span >= FIT_MIN_SPAN {
        FitState::Fit
    } else {
        FitState::Unfit
    };
    CalibrationCell {
        provider: provider.to_string(),
        workload: workload.to_string(),
        judged: rows.len(),
        right,
        wrong,
        waived,
        overridden,
        weighted_wrong_rate: weighted_wrong_rate + 0.0,
        confidence_span: confidence_span + 0.0,
        fit,
        spend_usd: spend + 0.0,
        cost_per_right: (right > 0).then(|| spend / right as f64),
    }
}

/// The opposite verdict, for a row the owner disagreed with.
fn invert(verdict: &str) -> String {
    if verdict == VERDICT_WRONG {
        VERDICT_RIGHT.to_string()
    } else {
        VERDICT_WRONG.to_string()
    }
}

// --- the home-side spend ledger --------------------------------------------

/// The home-side spend ledger for one beat: `home` joined with `name`,
/// one JSON line per ask whose row never reached the calibration stream.
///
/// The ledger is the home's, not the stream's and not the beat's own
/// artifact directory's, so it survives a failure in either: money the
/// lane already spent is money spent, and a cap that only moves on
/// success lets a broken writer spend uncapped (the consuming-lanes
/// rule). Each beat names its own file, so the two ledgers never share a
/// line.
pub fn spend_sidecar(home: &Path, name: &str) -> PathBuf {
    home.join(name)
}

/// Append one failed ask's cost to the beat's home-side ledger. Best
/// effort: a ledger that cannot be written (an unwritable home) is not a
/// second failure to report — the beat's own refusal names the ask that
/// could not complete.
pub fn record_spend(home: &Path, name: &str, cost: Option<f64>) {
    // The direct path spent nothing, so there is nothing to record.
    let Some(cost) = cost else { return };
    let line = json!({ "ts": stamp(), "cost_usd": cost }).to_string();
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(spend_sidecar(home, name))
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

/// The day's spend recorded only to a beat's home-side ledger: asks whose
/// row never reached the calibration stream. An absent or unreadable
/// ledger spends nothing, which is the honest reading.
pub fn sidecar_spend(home: &Path, name: &str, day: &str) -> f64 {
    let Ok(text) = std::fs::read_to_string(spend_sidecar(home, name)) else {
        return 0.0;
    };
    text.lines()
        .filter_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).ok()?;
            let ts = value.get("ts").and_then(serde_json::Value::as_str)?;
            ts.starts_with(day).then(|| value.get("cost_usd")?.as_f64())?
        })
        .sum()
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
use palette::{ACCENT, BOLD, DIM, RESET, TEXT, WARN};

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
        OP_EXPLAIN => explain_detail(&op_rows),
        OP_ROUTE_JUDGED => judged_detail(&op_rows),
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

/// The explanation op's facts: how many verdicts were explained, the
/// ops they came from, and the last one — what it explained, the path
/// its ask took, and what it cost. The eval-only marker rides in the
/// card's first line: the surface that shows the op also says what the
/// op is not.
fn explain_detail(rows: &[&ReadRow]) -> Vec<String> {
    let Some(last) = rows.last() else {
        return Vec::new();
    };
    let mut lines = vec![format!(
        "{} explained \u{b7} eval-only, no gate reads this op",
        rows.len()
    )];
    let ops: Vec<&str> = rows.iter().map(|row| row.explained_op.as_str()).collect();
    if !ops.is_empty() {
        lines.push(format!("explained ops: {}", ops.join(" \u{b7} ")));
    }
    let cost = rows.iter().map(|row| row.cost_usd).sum::<f64>();
    lines.push(format!(
        "last {} `{}` \u{2192} {} \u{b7} {} chars \u{b7} cost_usd {cost:.4}",
        last.explained_op, last.selector, last.route.path, last.explanation_chars
    ));
    if !last.validation.outcome.is_empty() {
        lines.push(format!("validation: {}", last.validation.outcome));
    }
    lines
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// The judgment op's facts on the stream's own card: the right/wrong
/// split, the dispositions that moved it, and the fit rule's verdict on
/// the day's cells.
fn judged_detail(rows: &[&ReadRow]) -> Vec<String> {
    let tally = JUDGMENT_VERDICTS
        .iter()
        .filter_map(|verdict| {
            let count = rows.iter().filter(|row| row.verdict == *verdict).count();
            (count > 0).then(|| format!("{count} {verdict}"))
        })
        .collect::<Vec<_>>();
    let dispositions = DISPOSITIONS
        .iter()
        .filter_map(|disposition| {
            let count = rows.iter().filter(|row| row.disposition == *disposition).count();
            (count > 0).then(|| format!("{count} {disposition}"))
        })
        .collect::<Vec<_>>();
    let judged = calib_rows_from(rows);
    let cells = calibration(&judged);
    let fit = cells.iter().filter(|cell| cell.fit == FitState::Fit).count();
    let mut lines = vec![format!("verdicts: {}", tally.join(" \u{b7} "))];
    lines.push(format!(
        "fit: {fit} of {} cell{} \u{b7} the floor is trusted at {FIT_MIN_ROWS} rows and \
         {FIT_MIN_SPAN:.2} span",
        cells.len(),
        plural(cells.len())
    ));
    if !dispositions.is_empty() {
        lines.push(format!("the owner: {}", dispositions.join(" \u{b7} ")));
    }
    lines
}

/// The rows a detail function holds, owned back into the reader's shape
/// so the calibration math can run over them.
fn calib_rows_from(rows: &[&ReadRow]) -> Vec<ReadRow> {
    rows.iter().map(|row| (*row).clone()).collect()
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
        OP_ROUTE_JUDGED => format!(
            "{}  route_judged  {}  {}  conf {:.2}  key {}{}",
            row.ts,
            row.provider,
            row.verdict,
            row.confidence,
            row.route_key,
            if row.disposition.is_empty() {
                String::new()
            } else {
                format!("  \u{b7} the owner {}", row.disposition)
            },
        ),
        _ => format!("{}  {}", row.ts, row.op),
    }
}

// --- the calibration surface ----------------------------------------------

/// Strip ANSI escapes for a width measurement, the way `route` and
/// `explain` do: a card's drawn width is what the terminal shows, not
/// what the bytes say.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            out.push(character);
        }
    }
    out
}

/// The status dot a calibration cell's card carries: green when the
/// cell is fit and below the floor, amber when it is fit and at or over
/// it (the cell informs the failover's preference), and dim when it is
/// unfit — an unfit cell is not a score, so it is not colored like one.
fn calibration_dot(cell: &CalibrationCell) -> (&'static str, &'static str) {
    if cell.informs_failover() {
        ("\u{25d8}", palette::WARN)
    } else if cell.fit == FitState::Fit {
        ("\u{2713}", palette::OK)
    } else {
        ("\u{25cb}", palette::DIM)
    }
}

/// One cell's card: the judged count, the right/wrong split, the
/// confidence-weighted wrong-rate, the cost per correct call, and the
/// fit state — all inside the cell, never averaged across shapes. An
/// unfit cell renders as unfit with what it still needs and never as a
/// score.
fn calibration_card(cell: &CalibrationCell) -> String {
    let (dot, color) = calibration_dot(cell);
    let title = format!("{} \u{b7} {}", cell.provider, cell.shape());
    let header_right = format!("{dot} {}", cell.fit.label());
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{id}{RESET} {DIM}{dashes} {color}{right}{RESET} {DIM}\u{2510}{RESET}",
        DIM = palette::DIM,
        RESET = palette::RESET,
        BOLD = palette::BOLD,
        ACCENT = palette::ACCENT,
        id = title,
        dashes = dashes_for(&format!(" {title} "), &plain(&header_right)),
        color = color,
        right = header_right,
    );
    let mut lines = vec![format!(
        "{DIM}\u{2502}{RESET} {TEXT}{judged} judged \u{b7} {right} right \u{b7} {wrong} wrong{extra}{RESET}",
        DIM = palette::DIM,
        RESET = palette::RESET,
        TEXT = palette::TEXT,
        judged = cell.judged,
        right = cell.right,
        wrong = cell.wrong,
        extra = if cell.waived == 0 && cell.overridden == 0 {
            String::new()
        } else {
            let mut parts = Vec::new();
            if cell.waived > 0 {
                parts.push(format!("{} waived", cell.waived));
            }
            if cell.overridden > 0 {
                parts.push(format!("{} overridden by the owner", cell.overridden));
            }
            format!(" \u{b7} {}", parts.join(" \u{b7} "))
        },
    )];
    if cell.fit == FitState::Fit {
        let rate_color = if cell.informs_failover() { palette::WARN } else { palette::OK };
        lines.push(format!(
            "{DIM}\u{2502}{RESET} {TEXT}confidence-weighted wrong-rate {rate_color}{rate:.2}{RESET} \
             {DIM}\u{b7} floor {floor:.2} \u{b7} span {span:.2}{RESET}",
            DIM = palette::DIM,
            RESET = palette::RESET,
            TEXT = palette::TEXT,
            rate_color = rate_color,
            rate = cell.weighted_wrong_rate,
            floor = WRONG_RATE_FLOOR,
            span = cell.confidence_span,
        ));
        if cell.informs_failover() {
            // Wrapped rather than clipped: the clause says what the
            // ride-back does, and a reader should not have to guess at a
            // cut tail. It names the shape the cell grades, because the
            // influence is shape-scoped and never a global.
            for line in wrap_detail(
                &format!(
                    "informs the failover for shape `{}` only: skipped in the preference when a \
                     later candidate is healthy",
                    cell.shape()
                ),
                CARD_WIDTH - 4,
            ) {
                lines.push(format!("{DIM}\u{2502}{RESET} {WARN}{line}{RESET}"));
            }
        }
    } else {
        // Unfit: what it still needs, named, and no score.
        let mut needs = Vec::new();
        if cell.needs_rows() > 0 {
            needs.push(format!(
                "{} more judged row{}",
                cell.needs_rows(),
                plural(cell.needs_rows())
            ));
        }
        if cell.needs_span() > 0.0 {
            needs.push(format!("{:.2} more confidence span", cell.needs_span()));
        }
        for line in
            wrap_detail(&format!("unfit \u{b7} needs {}", needs.join(" \u{b7} ")), CARD_WIDTH - 4)
        {
            lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{line}{RESET}"));
        }
        for line in wrap_detail("no score is drawn from this sample", CARD_WIDTH - 4) {
            lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{line}{RESET}"));
        }
    }
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {TEXT}judge spend ${spend:.4}{cost}{RESET}",
        DIM = palette::DIM,
        RESET = palette::RESET,
        TEXT = palette::TEXT,
        spend = cell.spend_usd,
        cost = cell
            .cost_per_right
            .map(|cost| format!(" \u{b7} ${cost:.4} per right call"))
            .unwrap_or_else(|| " \u{b7} no right call to price".to_string()),
    ));
    let bottom = format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}",
        "\u{2500}".repeat(CARD_WIDTH - 2),
        DIM = palette::DIM,
        RESET = palette::RESET
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

/// The calibration surface: one day's (provider \u{d7} workload shape)
/// cells as cards, the fit rule's thresholds named, and the ride-back's
/// gate state. `informs_failover` is the config gate's resolved value,
/// so the surface says whether the calibration is allowed to move
/// anything at all rather than leaving it to be inferred.
///
/// The cells, never a per-provider average: a provider that is right for
/// one shape and wrong for another has two cards, and neither hides the
/// other. A day with no judged rows says so rather than rendering an
/// empty surface \u{2014} "no judgments yet" names the command that makes
/// them.
pub fn render_calibration(root: &Path, day: &str, informs_failover: bool) -> String {
    let rows = read_day(root, day, DISPLAY_CAP);
    let cells = calibration(&rows);
    let mut out = format!(
        "{BOLD}fleet calib --calibration{RESET} {DIM}\u{2014} the judged quality, per provider and \
         shape{RESET}\n{DIM}{day} (UTC){RESET}\n\n",
        BOLD = palette::BOLD,
        RESET = palette::RESET,
        DIM = palette::DIM,
    );
    if cells.is_empty() {
        out.push_str(&format!(
            "{DIM}no judged routes for {day} yet \u{b7} `fleet judge` grades a day's route rows \
             through the sys1 lane{RESET}\n",
            DIM = palette::DIM,
            RESET = palette::RESET,
        ));
        return out;
    }
    for cell in &cells {
        out.push_str(&calibration_card(cell));
    }
    let judged: usize = cells.iter().map(|cell| cell.judged).sum();
    let fit = cells.iter().filter(|cell| cell.fit == FitState::Fit).count();
    let informing = cells.iter().filter(|cell| cell.informs_failover()).count();
    let mut providers: Vec<&str> = cells.iter().map(|cell| cell.provider.as_str()).collect();
    providers.sort_unstable();
    providers.dedup();
    let gate = if informs_failover { "on" } else { "off" };
    // The footer: four purpose-built lines, each inside the house width,
    // so the surface reads the same at 80 columns as it does at 120.
    out.push_str(&format!(
        "{DIM}{judged} judged row{judged_plural} \u{b7} {cells} cell{cells_plural} over \
         {providers} provider{provider_plural}{RESET}\n\
         {DIM}{fit} fit \u{b7} {informing} informing the failover{RESET}\n",
        judged_plural = plural(judged),
        cells_plural = plural(cells.len()),
        provider_plural = plural(providers.len()),
        DIM = palette::DIM,
        RESET = palette::RESET,
        judged = judged,
        cells = cells.len(),
        providers = providers.len(),
        fit = fit,
        informing = informing,
    ));
    out.push_str(&format!(
        "{DIM}gate calibration.informs_failover {gate} \u{b7} floor {floor:.2} at {rows} rows \
         \u{b7} {span:.2} span{RESET}\n",
        DIM = palette::DIM,
        RESET = palette::RESET,
        gate = gate,
        floor = WRONG_RATE_FLOOR,
        rows = FIT_MIN_ROWS,
        span = FIT_MIN_SPAN,
    ));
    out.push_str(&format!(
        "{DIM}the declared chain's order and its head are untouched{RESET}\n",
        DIM = palette::DIM,
        RESET = palette::RESET,
    ));
    out.push_str(&format!(
        "{DIM}the calibration moves only the failover's preference{RESET}\n",
        DIM = palette::DIM,
        RESET = palette::RESET,
    ));
    out.push_str(&format!(
        "{DIM}and only for the workload shape's own cell{RESET}\n",
        DIM = palette::DIM,
        RESET = palette::RESET,
    ));
    out
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
            input_sha256: Some("a1b2c3d4e5f60718".to_string()),
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
        // The judged artifact's identity rides beside the verdict: the
        // join key an explanation attaches to (slice 17).
        assert_eq!(value["input_sha256"], "a1b2c3d4e5f60718");
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
                input_sha256: Some("a1b2c3d4e5f60718".to_string()),
            })],
        );
        let rows = read_day(&root, "2026-10-14", 512);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].op, OP_PROPOSE);
        assert_eq!(rows[0].proposal, "fix-the-ranker");
        assert_eq!(rows[0].verdict, "refused");
        assert_eq!(rows[0].input_sha256, "a1b2c3d4e5f60718");
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
                "eval_corpus",
                "explain",
                "route_judged"
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
            input_sha256: Some("0f1e2d3c4b5a6978".to_string()),
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

    // --- the explain op ---

    fn explain_row(explained_op: &str, selector: &str, sha: &str) -> Row {
        Row::Explain(ExplainRow {
            ts: "2026-10-14T13:00:00Z".to_string(),
            eval_only: true,
            explained_op: explained_op.to_string(),
            selector: selector.to_string(),
            input_sha256: sha.to_string(),
            explanation_sha256: "5a6b7c8d9e0f1123".to_string(),
            explanation_chars: 812,
            route: ExplainRouteRow {
                needs_generation: Some(0.978),
                path: "synthesized".to_string(),
                answer_shape: "synthesized".to_string(),
            },
            validation: ExplainValidationRow {
                outcome: "passed".to_string(),
                answers_the_question: Some(true),
                contradicts_facts: Some(false),
                number_check: "ok".to_string(),
            },
            cost_usd: Some(0.0018),
            latency_ms: Some(15_400),
            lane: "blend1".to_string(),
        })
    }

    #[test]
    fn the_explain_row_is_redacted_and_carries_the_join_key_and_the_telemetry() {
        let row = serde_json::to_string(&explain_row("propose", "fix-the-ranker", "a1b2c3d4e5f60718"))
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(value["op"], "explain");
        // The eval-only marker is the row's first claim about itself.
        assert_eq!(value["eval_only"], true);
        assert_eq!(value["explained_op"], "propose");
        assert_eq!(value["selector"], "fix-the-ranker");
        assert_eq!(value["input_sha256"], "a1b2c3d4e5f60718");
        assert_eq!(value["explanation_sha256"], "5a6b7c8d9e0f1123");
        assert_eq!(value["explanation_chars"], 812);
        assert_eq!(value["route"]["needs_generation"], 0.978);
        assert_eq!(value["validation"]["outcome"], "passed");
        assert_eq!(value["cost_usd"], 0.0018);
        assert_eq!(value["latency_ms"], 15_400);
        assert_eq!(value["lane"], "blend1");
        // Redacted by shape: no prompt text and no answer text rides in
        // the row, under any key a reader might look for.
        for key in ["prompt", "answer", "text", "explanation", "final_answer", "body"] {
            assert!(value.get(key).is_none(), "the row carries `{key}`: {row}");
        }
    }

    #[test]
    fn the_reader_keeps_explain_rows_and_the_surface_cards_the_op() {
        let root = scratch("read-explain");
        write_rows(
            &root,
            "2026-10-14",
            &[
                explain_row("propose", "fix-the-ranker", "a1b2c3d4e5f60718"),
                explain_row("eval_corpus", "conversation-fold", "0f1e2d3c4b5a6978"),
            ],
        );
        let rows = read_day(&root, "2026-10-14", 512);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].op, OP_EXPLAIN);
        assert_eq!(rows[0].explained_op, "propose");
        assert_eq!(rows[0].selector, "fix-the-ranker");
        assert_eq!(rows[0].input_sha256, "a1b2c3d4e5f60718");
        assert!(rows[0].eval_only);
        assert_eq!(rows[0].explanation_chars, 812);
        assert_eq!(rows[0].route.path, "synthesized");
        assert_eq!(rows[0].route.needs_generation, Some(0.978));
        assert_eq!(rows[0].validation.outcome, "passed");
        // The card names the op and says what it is not.
        let rendered = render(&root, "2026-10-14");
        assert!(rendered.contains("explain"), "{rendered}");
        assert!(rendered.contains("2 explained"), "{rendered}");
        assert!(rendered.contains("eval-only, no gate reads this op"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
