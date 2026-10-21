//! Subagents: one delegated task, run in its own child session on the
//! parent's own route, with a scoped tool set and the parent's approver.
//!
//! The port of the fork's `packages/subagent` — its depth accounting,
//! its parent-options-for-delegation pattern, and its model-selection
//! preflight — as a native implementation. Nothing here imports, vendors,
//! or shells out to the fork (hard rule 6); the vocabulary and the guard
//! shapes are what crossed over.
//!
//! Five decisions hold the whole slice together, and slice 20 adds
//! four more that only matter once delegation has to *learn*:
//!
//! 1. **The loop is reused, never forked.** A child is an ordinary
//!    [`Agent`] over an ordinary [`Session`], with the same compiled-in
//!    system prompt, the goal as its first and only user message, a
//!    scoped declaration set, and its own turn cap.
//! 2. **Same model, structurally.** The child runs on the *same* routed
//!    client the parent is using and takes its model from the parent's
//!    own options, so "never downgrades" is a property of the
//!    construction rather than a rule the model is asked to follow.
//!    There is no per-call model parameter anywhere.
//! 3. **Same approver.** The child's calls reach the owner through the
//!    parent's gate, labelled with the subagent and its goal, so the
//!    owner knows whose call it is before deciding.
//! 4. **Depth two, structurally.** [`SUBAGENT_MAX_DEPTH`] reads 2. The
//!    `subagent` tool is admitted to a depth-1 child's scope when the
//!    delegation *declared* it (D9: the declaration is the scope, not a
//!    hint beside one) and is stripped from a depth-2 child's, so the
//!    depth-2 call is refused by name citing the cap — the existing
//!    refusal shape, not a new one. A chain that goes one deeper meets
//!    the guard in [`run`] rather than inventing one.
//! 5. **Never silence.** A failure returns the partial result, the typed
//!    reason, and the child's log path, in the parent's ordinary bounded
//!    `tool_result` — so the parent's log carries the call and its
//!    outcome either way (hard rule 5).
//! 6. **The brief is a vector, and it is composed, not hoped for.**
//!    [`Brief`] composes the four parts — task, approach, context,
//!    what done looks like — between explicit markers, so a
//!    non-conforming model cannot read the background as the task or
//!    the proposal as an instruction. A call that carries none of the
//!    three new parts composes exactly the message this module has
//!    always sent: the goal, whole.
//! 7. **The declaration is the scope.** A delegated node names the
//!    capabilities its subtree needs, and that list *is* its admitted
//!    set: a declared tool is admitted and an undeclared one is
//!    refused by name. A declaration that sat beside a hand-written
//!    scope would be decoration, and decoration is worse than nothing
//!    because it looks like a guarantee.
//! 8. **Every delegation lands a row.** One `delegation` row per
//!    subagent completion, through the calibration stream's own
//!    writer, keyed by the child's session id and carrying the parent
//!    chain — because a chain cannot be backfilled honestly, and a
//!    delegation nobody recorded is a delegation the loop cannot
//!    grade.
//! 9. **The delta is verified, not believed.** A child reports whether
//!    it followed the approach it was handed, and the report is checked
//!    against the child's own session log — the same join the
//!    delegation beat's judge does. A claimed deviation the log does not
//!    show is a false report, and it is named as one.

use crate::agent::{Agent, AgentOptions, Approver, FileAccess, SubagentStage, UiEvent};
use crate::api::{Client, FunctionCall, ToolCall};
use crate::calib::{self, DelegatedRunRow, Stream};
use crate::route::RouteDecision;
use crate::session::{Record, Session};
use crate::skills::Skill;
use crate::tools::{self, ToolOutcome};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

/// The turn cap a delegation gets when it does not ask for one. Lower
/// than a run's own [`crate::agent::DEFAULT_MAX_TURNS`] on purpose: a
/// delegated task is one bounded piece of work, and a child that needs
/// twenty turns to say what it found is a child whose goal was wrong.
pub const SUBAGENT_TURN_CAP: usize = 20;

/// The floor a depth-stepped turn cap never drops below. A depth-2
/// chain cannot burn the parent's whole budget on one branch, and it
/// also cannot be starved into a single turn: five turns is the
/// smallest budget that can still read a file, write a file, and say
/// what it did.
pub const SUBAGENT_TURN_CAP_FLOOR: usize = 5;

/// The bounds on a requested turn cap. The schema carries them too, but
/// a bound only the schema carries is a bound a non-conforming caller
/// ignores, so the parse carries them as well.
pub const SUBAGENT_MIN_TURNS: usize = 1;
pub const SUBAGENT_MAX_TURNS: usize = 50;

/// The delegation-depth cap: a subagent's own run sits at depth 1, a
/// grandchild's at depth 2, and nothing sits deeper. Enforcement is
/// structural in both halves — the `subagent` tool is stripped from a
/// run at the cap (so its declarations never carry it and a call that
/// arrives anyway is refused by name citing this constant), and the
/// guard in [`run`] refuses a depth the scope somehow let through.
pub const SUBAGENT_MAX_DEPTH: usize = 2;

/// The turn cap a delegated run at `depth` gets when its call did not
/// ask for one: [`SUBAGENT_TURN_CAP`] at depth 1, halved at every level
/// below, floored at [`SUBAGENT_TURN_CAP_FLOOR`].
///
/// The step down is the depth model's budget rule: a depth-2 chain
/// cannot spend its parent's whole budget on one branch, and the
/// halving is what makes that true of a chain of any length rather than
/// of one particular depth. A depth-1 call that asks for nothing gets
/// [`SUBAGENT_TURN_CAP`]; a depth-2 one gets half of it.
pub fn turn_cap_for_depth(depth: usize) -> usize {
    (SUBAGENT_TURN_CAP >> depth.saturating_sub(1)).max(SUBAGENT_TURN_CAP_FLOOR)
}

/// The turn cap a delegated run at `depth` is held to, whatever it
/// asked for: the ordinary bound at depth 1 — `max-turns` still reads
/// 1..=[`SUBAGENT_MAX_TURNS`], because a first-level delegation is the
/// case this crate has always had — and the depth-stepped default below
/// it.
///
/// The asymmetry is the point. Depth 1 is not tightened, because
/// nothing about it changed; depth 2 is, because a depth-2 brief is a
/// paraphrase of a paraphrase and its budget is the halved default
/// rather than the full one.
pub fn turn_ceiling(depth: usize) -> usize {
    match depth {
        0 | 1 => SUBAGENT_MAX_TURNS,
        _ => turn_cap_for_depth(depth),
    }
}

/// The turn cap one delegated run actually gets, enforced at
/// construction rather than asked of the model: the requested cap,
/// bounded by the depth-stepped ceiling and by what the delegating run
/// has left.
///
/// `parent_remaining` is the delegating run's own remaining turns,
/// which only a caller that knows the parent's cap can name. A child
/// that outlived its parent's budget would spend turns on a run whose
/// own answer is already written, so the bound is taken wherever it
/// can be; `None` leaves the depth-stepped ceiling holding on its own.
pub fn inherit_turn_cap(requested: usize, depth: usize, parent_remaining: Option<usize>) -> usize {
    let bounded = requested.min(turn_ceiling(depth));
    match parent_remaining {
        // A parent mid-call always has a turn left, but a log that
        // reads otherwise is not a reason to hand a child zero turns:
        // the floor keeps the child runnable and the parent's own cap
        // is what actually bounds the branch.
        Some(remaining) => bounded.min(remaining.max(SUBAGENT_MIN_TURNS)),
        None => bounded,
    }
}

/// The delegating run's remaining turns, read off its own session log,
/// or `None` when they cannot be named.
///
/// A delegated parent's cap is this module's own construction, so the
/// depth-stepped default at the parent's own depth is the number it was
/// built around, and the turns it has spent are the assistant records
/// in its log. It is exact for a parent that took the default and
/// conservative for one that asked for more; a parent that asked for
/// less is the one direction the rule cannot see, because a session log
/// records the turns a run spent and never the cap it was given. The
/// depth-stepped ceiling is what holds the branch in that case.
///
/// An ordinary parent's cap is its own run's business — `--max-turns`
/// is the owner's, not this module's — so nothing is named and the
/// ceiling holds instead of a bound invented here.
fn parent_remaining_turns(
    sessions_dir: &std::path::Path,
    parent: &str,
    depth: usize,
) -> Option<usize> {
    if depth < 2 {
        return None;
    }
    let log = sessions_dir.join(format!("{parent}.jsonl"));
    let records = crate::session::replay(&log).ok()?;
    let delegated = records.first().is_some_and(|record| {
        matches!(record, Record::Start { kind: Some(kind), .. } if kind == crate::session::SUBAGENT_KIND)
    });
    if !delegated {
        return None;
    }
    let spent = turns_completed(&records);
    Some(turn_cap_for_depth(depth - 1).saturating_sub(spent as usize))
}

/// How long a goal reads in one progress line or an approval prompt.
const GOAL_PREVIEW: usize = 72;

/// How long each part of a brief reads where a whole brief is drawn on
/// one line: a progress line or a prompt renders the four parts
/// bounded rather than dumping them, and a part nobody bounded costs
/// the line rather than the terminal.
const BRIEF_PART_PREVIEW: usize = 48;

/// The bound each part of a brief is composed at. The background, the
/// definition of done, and the proposed approach are the parent's own
/// words, and a parent that pastes a file into `context` gets the same
/// courtesy every other model-visible text gets: a bound with an
/// elision marker rather than a flood that pushes the task out of the
/// child's context.
const BRIEF_PART_CAP: usize = 8 * 1024;

/// Every tool name a subagent may be admitted to. This is the same list
/// the `tools` parameter's JSON Schema enum carries; the two are kept
/// in step by the declaration test in `crate::tools`, because a scope
/// the schema cannot express is a scope the model will ask for anyway.
///
/// `subagent` is in the set because the depth cap is 2: a delegation
/// that names it opens a second level, and a delegation that does not
/// keeps the chain at one. It is stripped again at the cap, so the
/// admission a depth-2 run can be given is empty by construction.
pub const ADMISSIBLE_TOOLS: [&str; 8] = [
    "bash",
    "read_file",
    "write_file",
    "list_dir",
    tools::USE_SKILL_TOOL,
    tools::SUBAGENT_TOOL,
    // The live source rides a delegate by default: a child reasoning
    // about the world outside the tree is exactly the agent that needs
    // to check a fact rather than guess at one, and the lane's own
    // bounds and fail-open travel with it.
    tools::WEB_TOOL,
    // And the desktop, for the same reason — with the difference that
    // its approval cannot be waived by any mode, so a child driving the
    // desktop asks exactly as often as the owner does.
    tools::COMPUTER_USE_TOOL,
];

/// The tool set one subagent run is admitted to.
///
/// A scope does two things, and both matter: it decides which
/// declarations the child's requests carry, and it decides which calls
/// the loop refuses. A tool excluded from the declarations is not
/// callable by a conforming model; the refusal is what meets a
/// non-conforming one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Every admitted model-visible tool name, in the order the child
    /// declares them.
    admitted: Vec<String>,
    /// This run's delegation depth: 0 for an ordinary run, 1 for a
    /// subagent's own run, 2 for a grandchild's. The child's depth is
    /// this plus one.
    pub depth: usize,
}

impl Scope {
    /// The default admitted scope: the four built-ins plus `use_skill`.
    /// Everything else — the two memory tools, the lane namespace, and
    /// `subagent` itself — is outside it by construction.
    ///
    /// `subagent` being outside the *default* is what keeps a chain at
    /// one level unless the orchestrator asks for a second: the
    /// declaration is what opens depth 2, and a brief that declares
    /// nothing delegates exactly as this crate always has.
    pub fn default_admitted() -> Vec<String> {
        crate::tools::ALL_TOOLS
            .iter()
            .map(|tool| tool.name().to_string())
            .chain(std::iter::once(tools::USE_SKILL_TOOL.to_string()))
            .collect()
    }

    /// A child's scope: `requested` when the call narrowed it, the
    /// default otherwise, stamped with the child's delegation depth and
    /// stripped of the delegation tool at the cap.
    ///
    /// The strip is the depth cap's declarative half. A run at the cap
    /// cannot be admitted to `subagent` however the call asked, so a
    /// depth-2 child's declarations never carry it and a call that
    /// arrives anyway meets the loop's own refusal naming
    /// [`SUBAGENT_MAX_DEPTH`] — the existing refusal shape, not a new
    /// one.
    pub fn child(requested: Option<Vec<String>>, depth: usize) -> Scope {
        let admitted = requested.unwrap_or_else(Self::default_admitted);
        let admitted = if depth >= SUBAGENT_MAX_DEPTH {
            admitted
                .into_iter()
                .filter(|name| name != tools::SUBAGENT_TOOL)
                .collect()
        } else {
            admitted
        };
        Scope { admitted, depth }
    }

    /// Whether `name` is inside this scope.
    pub fn admits(&self, name: &str) -> bool {
        self.admitted.iter().any(|tool| tool == name)
    }

    /// Every admitted name, in declaration order.
    pub fn names(&self) -> &[String] {
        &self.admitted
    }

    /// The scope as the delegation row records it: the admitted names,
    /// comma-joined, or the word for a scope that admits nothing. The
    /// row is redacted against text, and a tool list is not text — it
    /// is the objective shape of what the child was given.
    pub fn recorded(&self) -> String {
        if self.admitted.is_empty() {
            "none".to_string()
        } else {
            self.admitted.join(",")
        }
    }
}

// --- the brief -------------------------------------------------------------

/// The marker each part of a composed brief opens with.
///
/// The markers are the whole point of composing rather than
/// concatenating: with every part saying which it is, a non-conforming
/// model cannot read the background as the task or the proposal as an
/// instruction, and a reader of the child's log can see the shape of
/// the brief that produced the run.
const TASK_SECTION: &str = "--- TASK ---";
const APPROACH_SECTION: &str = "--- APPROACH ---";
const CONTEXT_SECTION: &str = "--- CONTEXT ---";
const DONE_SECTION: &str = "--- WHAT DONE LOOKS LIKE ---";

/// Every section marker, in the order a composed brief carries them.
const SECTIONS: [&str; 4] = [
    TASK_SECTION,
    APPROACH_SECTION,
    CONTEXT_SECTION,
    DONE_SECTION,
];

/// The framing the APPROACH section opens with.
///
/// Two things it has to do, and they are the same thing seen twice: the
/// approach is a **proposal**, so a child that finds a better route is
/// deviating from a suggestion rather than disobeying an order; and the
/// delta block is where that deviation is reported, because the delta
/// is the only signal that tells the orchestrator which brief shapes
/// actually describe the territory.
const APPROACH_PREAMBLE: &str = "A proposed solution, not an instruction. It is this \
     delegation's best guess at the route; take a better one when you find it. Whatever you \
     do, end your final answer with this block and put nothing after it:\n\n\
     --- DELTA ---\n\
     status: followed | deviated\n\
     reason: infeasible | companion-missing | right-and-followed | better-route\n\
     companion: the absent companion, named — required when the reason is companion-missing\n\
     instead: what you did instead, one line\n\
     forced: true | false\n\
     --- END DELTA ---";

/// The two lines the composer adds to the head of the CONTEXT section
/// and strips back on the way in. They are reserved vocabulary: a
/// caller whose own background opens with the same two words loses
/// that one line to the round trip, which is the price of the child
/// being told what it has.
const ROLE_LINE: &str = "ROLE: ";
const CAPABILITIES_LINE: &str = "CAPABILITIES: ";

/// One delegation's brief: the four parts the parent hands the child.
///
/// The brief is a vector. Its proximity is how similar it is to briefs
/// written before; its direction is whether its *shape* drove toward
/// verified-complete work. Nothing here carries the shape's text into
/// a row — [`Brief::shape`] is what the calibration keys on, and the
/// words stay in the child's own log where they belong.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Brief {
    /// The delegated task, stated in full — the goal, verbatim.
    pub task: String,
    /// The orchestrator's proposed solution for this subtree. Bounded,
    /// and stated as a proposal rather than an instruction.
    pub approach: Option<String>,
    /// The background the child needs and cannot see.
    pub context: Option<String>,
    /// The shape of the answer the parent will accept — the definition
    /// of done.
    pub output: Option<String>,
    /// The role this delegation runs under, when the call named one.
    /// The role's own overlay is loaded beside the compiled-in system
    /// prompt; this is the name, riding the brief so the child knows
    /// which delegation it is.
    pub role: Option<String>,
}

impl Brief {
    /// Compose the child's first user message: the four parts between
    /// explicit markers, with the declaration's capabilities riding the
    /// CONTEXT section so the child knows what it has and, by omission,
    /// what it does not.
    ///
    /// A call that carries none of the new parts composes **exactly**
    /// the message this module has always sent — the goal, whole, with
    /// no marker anywhere — which is what makes the brief backward
    /// compatible rather than merely additive.
    pub fn compose(&self, admitted: &[String]) -> String {
        if self.approach.is_none()
            && self.context.is_none()
            && self.output.is_none()
            && self.role.is_none()
            && admitted == Scope::default_admitted()
        {
            return self.task.clone();
        }
        let mut text = format!("{TASK_SECTION}\n{}\n", bounded(&self.task, BRIEF_PART_CAP));
        if let Some(approach) = &self.approach {
            text.push_str(&format!(
                "{APPROACH_SECTION}\n{APPROACH_PREAMBLE}\n{}\n",
                bounded(approach, BRIEF_PART_CAP)
            ));
        }
        // The CONTEXT section rides when there is background to carry, a
        // role to name, or a scope the child could not otherwise know
        // it had: a brief that hides the declaration is a brief whose
        // child discovers an unadmitted tool at the call.
        if self.context.is_some() || self.role.is_some() || admitted != Scope::default_admitted() {
            text.push_str(&format!("{CONTEXT_SECTION}\n"));
            if let Some(role) = &self.role {
                text.push_str(&format!(
                    "{ROLE_LINE}{}\n",
                    bounded(role, BRIEF_PART_PREVIEW)
                ));
            }
            text.push_str(&format!("{CAPABILITIES_LINE}{}\n", admitted.join(", ")));
            if let Some(context) = &self.context {
                text.push_str(&format!("\n{}\n", bounded(context, BRIEF_PART_CAP)));
            }
        }
        if let Some(output) = &self.output {
            text.push_str(&format!(
                "{DONE_SECTION}\n{}\n",
                bounded(output, BRIEF_PART_CAP)
            ));
        }
        text
    }

    /// Read a composed brief back into its parts.
    ///
    /// The inverse of [`Brief::compose`], and it has to be: the
    /// composed message rides the `goal` argument through the caller
    /// that builds the child, so the run that composes it is not the
    /// run that reads it. A text with no marker in it reads as the
    /// task alone — the pre-slice-20 message, unchanged.
    pub fn parse(text: &str) -> Brief {
        let found = sections(text);
        if found.is_empty() {
            return Brief {
                task: text.trim().to_string(),
                ..Brief::default()
            };
        }
        let mut brief = Brief::default();
        for (marker, body) in found {
            match marker {
                TASK_SECTION => brief.task = body.trim().to_string(),
                APPROACH_SECTION => {
                    // The preamble is the composer's own framing, so it
                    // is the one thing stripped back out.
                    let approach = body
                        .strip_prefix(APPROACH_PREAMBLE)
                        .unwrap_or(&body)
                        .trim()
                        .to_string();
                    brief.approach = Some(approach);
                }
                CONTEXT_SECTION => {
                    let mut rest = body.as_str();
                    let mut role = None;
                    if let Some(after) = rest.strip_prefix(ROLE_LINE) {
                        let (name, tail) = after.split_once('\n').unwrap_or((after, ""));
                        role = Some(name.trim().to_string());
                        rest = tail;
                    }
                    if let Some(after) = rest.strip_prefix(CAPABILITIES_LINE) {
                        // The effective scope is re-derived from the
                        // run's own scope when the message is composed
                        // again, so it is read past rather than kept.
                        let (_, tail) = after.split_once('\n').unwrap_or((after, ""));
                        rest = tail;
                    }
                    let context = rest.trim();
                    if !context.is_empty() {
                        brief.context = Some(context.to_string());
                    }
                    brief.role = role.filter(|name| !name.is_empty());
                }
                DONE_SECTION => brief.output = Some(body.trim().to_string()),
                _ => {}
            }
        }
        brief
    }

    /// The brief's shape: which of the four parts it carried, in the
    /// order the markers name them.
    ///
    /// This is the cell key's shape half, and it is deliberately not
    /// the brief's text: two briefs that read differently but ask for
    /// the same kind of work are the same vector, and two that read
    /// alike and ask for different kinds are different vectors. The
    /// shape is what the calibration keys on; the words stay out of
    /// the row.
    pub fn shape(&self) -> String {
        let mut parts = vec!["task"];
        if self.approach.is_some() {
            parts.push("approach");
        }
        if self.context.is_some() {
            parts.push("context");
        }
        if self.output.is_some() {
            parts.push("done");
        }
        parts.join("+")
    }

    /// Whether this brief handed the child anything to diff against. A
    /// brief that states only a goal gives the child no second side, so
    /// its report can only be a summary — the thing the discipline
    /// already distrusts — and no delta is asked for.
    pub fn offers_an_approach(&self) -> bool {
        self.approach
            .as_ref()
            .is_some_and(|approach| !approach.trim().is_empty())
    }

    /// The brief as one bounded line, for a progress line or an
    /// approval prompt: the task alone when the brief carried nothing
    /// else — exactly the label this module has always drawn — and the
    /// four parts, each bounded and delimited, when it did. Never a raw
    /// dump.
    pub fn label(&self) -> String {
        if self.approach.is_none() && self.context.is_none() && self.output.is_none() {
            return bounded(&self.task, GOAL_PREVIEW);
        }
        let mut parts = vec![format!("task {}", bounded(&self.task, BRIEF_PART_PREVIEW))];
        if let Some(approach) = &self.approach {
            parts.push(format!(
                "approach {}",
                bounded(approach, BRIEF_PART_PREVIEW)
            ));
        }
        if let Some(context) = &self.context {
            parts.push(format!("context {}", bounded(context, BRIEF_PART_PREVIEW)));
        }
        if let Some(output) = &self.output {
            parts.push(format!("done {}", bounded(output, BRIEF_PART_PREVIEW)));
        }
        parts.join(" \u{b7} ")
    }
}

/// The sections a composed brief carries, in order: each marker and the
/// text under it up to the next marker. A text with no marker in it
/// carries no section, which is how the pre-slice-20 message reads as
/// the task alone.
fn sections(text: &str) -> Vec<(&'static str, String)> {
    let mut found: Vec<(&'static str, String)> = Vec::new();
    let mut open: Option<&'static str> = None;
    let mut body = String::new();
    for line in text.lines() {
        let trimmed = line.trim_end();
        if let Some(marker) = SECTIONS.iter().copied().find(|marker| *marker == trimmed) {
            if let Some(previous) = open.take() {
                found.push((previous, std::mem::take(&mut body)));
            }
            open = Some(marker);
            continue;
        }
        if open.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(previous) = open.take() {
        found.push((previous, body));
    }
    found
}

// --- the delta -------------------------------------------------------------

/// The marker one delta block opens and closes with. Every value inside
/// a block is one line, so a block's own text can never open a marker
/// of its own — which is what makes the parse of a rendered result
/// exact rather than heuristic.
const DELTA_BEGIN: &str = "--- DELTA ---";
const DELTA_END: &str = "--- END DELTA ---";

/// The path a run's own delta block carries: the root of its subtree.
const OWN_PATH: &str = ".";

/// The bound one delta field is drawn at. A deviation's reason is what
/// the loop reads, and the free text beside it is never graded — but a
/// field nobody bounded is a field that can flood a log.
const DELTA_FIELD_CAP: usize = 240;

/// Whether the child followed the approach it was handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaState {
    /// The child executed the approach it was given.
    Followed,
    /// The child executed something else.
    Deviated,
}

impl DeltaState {
    /// The word a delta block carries.
    pub fn label(self) -> &'static str {
        match self {
            DeltaState::Followed => "followed",
            DeltaState::Deviated => "deviated",
        }
    }

    /// The state a word names, or `None` for anything else.
    pub fn parse(word: &str) -> Option<DeltaState> {
        match word.trim() {
            "followed" => Some(DeltaState::Followed),
            "deviated" => Some(DeltaState::Deviated),
            _ => None,
        }
    }
}

/// Why a child deviated — the reason category, which is what makes an
/// aggregate meaningful rather than a count.
///
/// The four categories separate "the brief drove away" from "the brief
/// was right and incomplete" from "the brief was right and the child
/// found better". Collapsing them would make a chosen improvement read
/// as a defect, and an orchestrator that punishes improvement gets
/// children that stop reporting honestly — which destroys the only
/// signal this part runs on. The categories are load-bearing, not
/// documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeltaReason {
    /// The approach could not be executed: a missing path, a wrong
    /// assumption. The brief drove away.
    Infeasible,
    /// The approach was right and incomplete, and the companion that
    /// was absent is named.
    CompanionMissing,
    /// The approach was right and the child followed it.
    RightAndFollowed,
    /// The approach was right and the child found a better route to
    /// the same minima. A gain, not a loss.
    BetterRoute,
    /// P6 C4: the node was not atomic. Its run discovered that the work
    /// breaks down further, and the children it found ride
    /// [`Delta::declares`]. This is the mechanism that lets an
    /// orchestrator declare to a bounded depth and expand as it reaches
    /// a summary, rather than having to see N levels down on its first
    /// turn.
    ///
    /// It is a deviation, not a failure: the child did exactly what it
    /// was asked — it looked, and the looking is what produced the
    /// breakdown. The delta's own weight is zero, because a summary being
    /// recognised as a summary moves nothing that was wrong.
    NotAtomic,
}

impl DeltaReason {
    /// The word a delta block carries.
    pub fn label(self) -> &'static str {
        match self {
            DeltaReason::Infeasible => "infeasible",
            DeltaReason::CompanionMissing => "companion-missing",
            DeltaReason::RightAndFollowed => "right-and-followed",
            DeltaReason::BetterRoute => "better-route",
            DeltaReason::NotAtomic => "not-atomic",
        }
    }

    /// The category a word names, or `None` for anything else.
    pub fn parse(word: &str) -> Option<DeltaReason> {
        match word.trim() {
            "infeasible" => Some(DeltaReason::Infeasible),
            "companion-missing" => Some(DeltaReason::CompanionMissing),
            "right-and-followed" => Some(DeltaReason::RightAndFollowed),
            "better-route" => Some(DeltaReason::BetterRoute),
            _ => None,
        }
    }
}

/// The angle between the approach a node was handed and the one it
/// executed.
///
/// The child is the only participant who knows both sides, so the
/// child is the one who reports it — and the report is verified
/// against the child's own session log rather than believed, because a
/// claimed deviation the log does not show is a false report and the
/// most serious kind.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Delta {
    /// Whether the child followed the approach it was handed.
    pub state: DeltaState,
    /// Why it deviated, when it did. On a followed delta this reads
    /// [`DeltaReason::RightAndFollowed`].
    pub reason: DeltaReason,
    /// The companion that was absent, named. Required on a
    /// `companion-missing` deviation: "insufficient" without the
    /// absent companion is not actionable, and the next brief cannot
    /// carry a companion nobody named.
    pub companion: String,
    /// What the child did instead. Free text: it rides beside the
    /// categories and is never graded.
    pub instead: String,
    /// Whether the deviation was **forced** (a blocker) or **chosen**
    /// (a better route). The two do not read alike in an aggregate.
    pub forced: bool,
    /// The deltas of the delegations this run made, verbatim, in call
    /// order. A middle node preserves its children's deltas rather
    /// than summarizing them, because a summarized delta cannot be
    /// graded and the loss is invisible.
    pub children: Vec<Delta>,
    /// The contradiction the child's own session log holds against
    /// this claim, named. `None` when the log corroborates it.
    pub contradiction: Option<String>,
    /// P6 C4: the nodes this run declares as the node's children, when it
    /// found the node to be a summary rather than an atomic task. Empty
    /// on every other reason.
    ///
    /// This is the payload that makes bounded-depth planning work: the
    /// orchestrator declares to a depth it can see, and a child that
    /// reaches a summary hands back the breakdown instead of the
    /// orchestrator having to anticipate it.
    #[serde(default)]
    pub declares: Vec<crate::graph::DeclaredNode>,
}

impl Delta {
    /// The delta one final answer reports, or `None` when the answer
    /// carried no block — or carried one that does not parse.
    ///
    /// A malformed block is not a delta: the result says no delta was
    /// reported rather than inventing a state the child never named.
    pub fn parse(text: &str) -> Option<Delta> {
        let (_, delta) = blocks(text).into_iter().next()?;
        Some(delta)
    }

    /// The delta one rendered subagent result carries: the child's own,
    /// with the deltas of the delegations it made hanging off it.
    ///
    /// Read off the log rather than asked of the model, which is what
    /// makes the ride up the parent chain verbatim rather than
    /// paraphrased.
    pub fn parse_render(text: &str) -> Option<Delta> {
        let found = blocks(text);
        let (_, own) = found.first()?.clone();
        let mut cursor = 1;
        let children = assemble(&found, &mut cursor, OWN_PATH);
        Some(Delta { children, ..own })
    }

    /// This delta and its children as the blocks a rendered result
    /// carries: the run's own block first, then one block per child,
    /// each under a path that says where it hangs. The path is what
    /// makes nesting unambiguous — two blocks that both read
    /// `--- child delta ---` could not be told apart.
    pub fn render(&self) -> String {
        let mut text = String::new();
        self.render_into(OWN_PATH, &mut text);
        text
    }

    /// This delta's own block, then its children's, depth first.
    fn render_into(&self, path: &str, text: &mut String) {
        text.push_str(&format!("{DELTA_BEGIN}\npath: {path}\n"));
        text.push_str(&format!("status: {}\n", self.state.label()));
        text.push_str(&format!("reason: {}\n", self.reason.label()));
        if !self.companion.is_empty() {
            text.push_str(&format!("companion: {}\n", self.companion));
        }
        if !self.instead.is_empty() {
            text.push_str(&format!("instead: {}\n", self.instead));
        }
        text.push_str(&format!("forced: {}\n", self.forced));
        if let Some(contradiction) = &self.contradiction {
            text.push_str(&format!("contradiction: {}\n", one_line(contradiction)));
        }
        text.push_str(&format!("{DELTA_END}\n"));
        for (index, child) in self.children.iter().enumerate() {
            child.render_into(&format!("{path}.{}", index + 1), text);
        }
    }

    /// Check this delta against the child's own session log and name
    /// the contradiction, storing it on the delta rather than
    /// discarding the claim.
    ///
    /// A followed claim asserts nothing the log could contradict — the
    /// child did what it was told, and the log is where the work is
    /// either way — so only a deviation is checked. Three signals, each
    /// readable from the log beside the claim:
    ///
    /// 1. a deviation on a log that holds no tool call at all: the
    ///    claimed work is absent from the child's own log;
    /// 2. an `infeasible` claim on a log that holds no failed step:
    ///    the claimed blocker is absent, and a blocker nobody hit is a
    ///    blocker nobody met;
    /// 3. a `companion-missing` deviation that names no companion: the
    ///    next brief cannot carry a companion nobody named.
    pub fn verify(mut self, records: &[Record]) -> Delta {
        if self.state == DeltaState::Deviated {
            let calls = records
                .iter()
                .filter(|record| {
                    matches!(record, Record::Assistant { tool_calls, .. } if !tool_calls.is_empty())
                })
                .count();
            self.contradiction = if calls == 0 {
                Some(
                    "the child claimed a deviation but its own session log holds no tool call — \
                     the claimed work is absent from its own log"
                        .to_string(),
                )
            } else if self.reason == DeltaReason::Infeasible && !failed_step(records) {
                Some(
                    "the child claimed the approach was infeasible but its own session log \
                     holds no failed step — the claimed blocker is absent from its own log"
                        .to_string(),
                )
            } else if self.reason == DeltaReason::CompanionMissing
                && self.companion.trim().is_empty()
            {
                Some(
                    "the deviation names no companion, and a companion nobody named is one the \
                     next brief cannot carry"
                        .to_string(),
                )
            } else {
                None
            };
        }
        self
    }
}

/// The deltas hanging off `prefix`, consumed forward from `cursor`.
/// Blocks are emitted depth first, so a child's whole subtree is
/// contiguous after it and the walk is a single pass.
fn assemble(found: &[(String, Delta)], cursor: &mut usize, prefix: &str) -> Vec<Delta> {
    let mut children: Vec<Delta> = Vec::new();
    let mut index = 1usize;
    while *cursor < found.len() {
        let path = format!("{prefix}.{index}");
        if found[*cursor].0 != path {
            break;
        }
        let (_, mut delta) = found[*cursor].clone();
        *cursor += 1;
        delta.children = assemble(found, cursor, &path);
        children.push(delta);
        index += 1;
    }
    children
}

/// Every delta block in `text`, in order, as `(path, delta)`. A block
/// that does not parse is skipped rather than guessed at: a delta whose
/// status is missing or unknown, or a deviation whose reason is, is not
/// a delta.
fn blocks(text: &str) -> Vec<(String, Delta)> {
    let mut found: Vec<(String, Delta)> = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() != DELTA_BEGIN {
            continue;
        }
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut closed = false;
        for field in lines.by_ref() {
            let field = field.trim_end();
            if field == DELTA_END {
                closed = true;
                break;
            }
            if let Some((key, value)) = field.split_once(':') {
                fields.push((key.trim().to_string(), value.trim().to_string()));
            }
        }
        if closed {
            if let Some(block) = block(&fields) {
                found.push(block);
            }
        }
    }
    found
}

/// One block's fields as a delta, or `None` for a block that does not
/// parse.
fn block(fields: &[(String, String)]) -> Option<(String, Delta)> {
    let field = |key: &str| {
        fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    let state = DeltaState::parse(field("status")?)?;
    // A followed delta's reason defaults to the one that says so; a
    // deviation without a reason is not a deviation anybody can grade.
    let reason = match state {
        DeltaState::Followed => DeltaReason::parse(field("reason").unwrap_or("right-and-followed"))
            .unwrap_or(DeltaReason::RightAndFollowed),
        DeltaState::Deviated => DeltaReason::parse(field("reason")?)?,
    };
    Some((
        field("path").unwrap_or(OWN_PATH).to_string(),
        Delta {
            state,
            reason,
            companion: one_line(field("companion").unwrap_or_default()),
            instead: one_line(field("instead").unwrap_or_default()),
            forced: field("forced") == Some("true"),
            children: Vec::new(),
            declares: Vec::new(),
            contradiction: field("contradiction").map(one_line),
        },
    ))
}

/// One field's value as a single bounded line: a value carrying a
/// newline could open a marker of its own, and a value nobody bounded
/// could flood the log it rides in.
fn one_line(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    bounded(&collapsed, DELTA_FIELD_CAP)
}

/// Whether the child's own log holds a step that failed — the shape an
/// infeasibility claim has to leave behind. A command that exited
/// nonzero, a call the harness refused or could not run.
fn failed_step(records: &[Record]) -> bool {
    records.iter().any(|record| match record {
        Record::ToolResult { output, .. } => {
            let head = output.lines().next().unwrap_or_default().trim();
            (head.starts_with("exit ") && head != "exit 0")
                || head.starts_with("error:")
                || head.starts_with("denied")
        }
        _ => false,
    })
}

/// The deltas of the delegations one run made, read off its own session
/// log: one per `subagent` tool result that carried a delta, in the
/// order the calls landed, each carrying its own children.
///
/// This is the verbatim ride up the parent chain, and it is read rather
/// than asked for: a middle node that summarized what its children
/// reported would lose exactly the signal the delta exists to carry,
/// and the loss would be invisible.
fn delegated_deltas(records: &[Record]) -> Vec<Delta> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::ToolResult { name, output, .. } if name == tools::SUBAGENT_TOOL => {
                Delta::parse_render(output)
            }
            _ => None,
        })
        .collect()
}

// --- the parent chain ------------------------------------------------------

/// The chain one delegated run hangs from: the immediate parent's
/// session id, the root's, and the depth.
///
/// The chain rides every delegation row, and it is recorded from the
/// first row because a chain cannot be backfilled honestly: a depth-2
/// row has to join to the brief that actually set its direction, and
/// the only place that brief is named is the session that made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// The immediate parent's session id.
    pub parent: String,
    /// The root session's id — the delegation the whole chain started
    /// from.
    pub root: String,
    /// This run's depth: 1 for a parent's own child, 2 for a
    /// grandchild.
    pub depth: usize,
    /// Every session between the root and this one, root first: the
    /// orchestrator's own run is the first entry and the immediate
    /// parent the last, so an approval at depth 2 can name the whole
    /// chain rather than the two ends of it.
    pub lineage: Vec<String>,
}

impl Chain {
    /// Resolve one run's chain from the session logs beside it.
    ///
    /// Every session's `start` record names the session that delegated
    /// to it, so the chain is reconstructable from the logs alone
    /// (hard rule 5) rather than from anything a caller remembered. The
    /// walk stops at a session whose log names no parent — the
    /// orchestrator's own run — or at a log that cannot be read, in
    /// which case the chain is as long as it could be proven rather
    /// than as long as somebody assumed.
    pub fn resolve(sessions_dir: &std::path::Path, parent: &str, depth: usize) -> Chain {
        let mut lineage: Vec<String> = vec![parent.to_string()];
        let mut cursor = parent.to_string();
        for _ in 0..SUBAGENT_MAX_DEPTH {
            let Some(next) = parent_of(sessions_dir, &cursor) else {
                break;
            };
            lineage.push(next.clone());
            cursor = next;
        }
        lineage.reverse();
        Chain {
            parent: parent.to_string(),
            root: lineage
                .first()
                .cloned()
                .unwrap_or_else(|| parent.to_string()),
            depth,
            lineage,
        }
    }

    /// The chain as one bounded, delimited line, root first.
    ///
    /// An owner approving a call whose originating intent is two briefs
    /// away is approving a paraphrase, so the prompt names every brief
    /// that stands between them. At depth 1 there is nothing between
    /// the owner and the child, and the label is the child's own goal —
    /// exactly the label this module has always drawn.
    pub fn label(&self, brief: &Brief, sessions_dir: &std::path::Path) -> String {
        if self.depth < 2 {
            return brief.label();
        }
        let mut goals: Vec<String> = self
            .lineage
            .iter()
            .map(|session| session_goal(sessions_dir, session).unwrap_or_else(|| session.clone()))
            .collect();
        goals.push(bounded(&brief.task, GOAL_PREVIEW));
        goals
            .iter()
            .map(|goal| bounded(goal, GOAL_PREVIEW))
            .collect::<Vec<_>>()
            .join(" \u{2192} ")
    }
}

/// The session that delegated to `session`, read off its own log's
/// `start` record. `None` for an ordinary run, or for a log that
/// cannot be read.
fn parent_of(sessions_dir: &std::path::Path, session: &str) -> Option<String> {
    let log = sessions_dir.join(format!("{session}.jsonl"));
    let records = crate::session::replay(&log).ok()?;
    records.first().and_then(|record| match record {
        Record::Start {
            parent: Some(parent),
            workspace: None,
            ..
        } if !parent.is_empty() => Some(parent.clone()),
        _ => None,
    })
}

/// The goal one session's own log opens with: its first user record,
/// which is the brief it was handed. `None` for a log that cannot be
/// read or holds no user message.
fn session_goal(sessions_dir: &std::path::Path, session: &str) -> Option<String> {
    let log = sessions_dir.join(format!("{session}.jsonl"));
    let records = crate::session::replay(&log).ok()?;
    records.iter().find_map(|record| match record {
        Record::User { content } if !content.trim().is_empty() => Some(Brief::parse(content).task),
        _ => None,
    })
}

/// One `subagent` call's arguments, parsed and validated.
///
/// Every failure is a typed refusal the model can read and react to, in
/// the same `error:` vocabulary a built-in's argument failure uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRequest {
    /// The child's first user message: the **composed brief**. The task
    /// verbatim when the call carried none of the three new parts —
    /// which is what keeps every existing caller and test
    /// byte-identical — and the four parts between their markers when it
    /// did. It rides the `goal` argument because the caller that builds
    /// the child is not the module that composes the message, and the
    /// goal is the only channel between them.
    pub goal: String,
    /// The delegated task on its own: what the progress line, the
    /// approval label, and the delegation row name.
    pub task: String,
    /// The orchestrator's proposed solution, stated as a proposal.
    pub approach: Option<String>,
    /// The background the child needs and cannot see.
    pub context: Option<String>,
    /// The shape of the answer the parent will accept.
    pub output: Option<String>,
    /// The role this delegation runs under, when the call named one.
    pub role: Option<String>,
    /// The admitted scope the call asked for; `None` is the default.
    /// This is the capability declaration, and it is the scope: a
    /// declared tool is admitted and an undeclared one is refused by
    /// name.
    pub tools: Option<Vec<String>>,
    /// The child's turn cap.
    pub max_turns: usize,
    /// The declared plan node this call executes, when the call named
    /// one. The node's own approach and capability declaration become the
    /// brief, so the plan document and the run cannot drift: the contract
    /// the owner approved is the one the child is handed.
    pub node: Option<String>,
}

impl SubagentRequest {
    /// Parse one call's arguments object.
    pub fn parse(arguments: &serde_json::Value) -> Result<SubagentRequest, String> {
        // A call may name a declared node instead of carrying a goal: the
        // node's own approach is the task, which is what keeps the plan
        // document and the run from drifting. Naming a node with no goal
        // is fine — the node supplies it — but a call with neither has
        // nothing to delegate.
        let node = optional_text(arguments, "node")?
            .map(|node| node.trim().to_string())
            .filter(|node| !node.is_empty());
        let goal = arguments
            .get("goal")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|goal| !goal.is_empty())
            .map(str::to_string)
            .or_else(|| node.clone())
            .ok_or_else(|| {
                "the `goal` argument is required and must be a non-empty string, or name a declared `node` whose approach is the task".to_string()
            })?;
        let tools = match arguments.get("tools") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::Array(items)) => {
                let mut admitted: Vec<String> = Vec::with_capacity(items.len());
                for item in items {
                    let name = item.as_str().ok_or_else(|| {
                        format!("every `tools` entry must be a tool name; {item} is not a string")
                    })?;
                    if !ADMISSIBLE_TOOLS.contains(&name) {
                        return Err(format!(
                            "`{name}` is not admissible in a subagent's scope; the admissible tools are {}",
                            ADMISSIBLE_TOOLS.join(", ")
                        ));
                    }
                    // A repeated name narrows nothing twice.
                    if !admitted.iter().any(|tool| tool == name) {
                        admitted.push(name.to_string());
                    }
                }
                Some(admitted)
            }
            Some(other) => {
                return Err(format!(
                    "`tools` must be an array of tool names; {other} is not"
                ))
            }
        };
        let max_turns = match arguments.get("max-turns") {
            None | Some(serde_json::Value::Null) => SUBAGENT_TURN_CAP,
            Some(value) => {
                let asked = value.as_u64().filter(|asked| {
                    *asked >= SUBAGENT_MIN_TURNS as u64 && *asked <= SUBAGENT_MAX_TURNS as u64
                });
                match asked {
                    Some(asked) => asked as usize,
                    None => {
                        return Err(format!(
                            "`max-turns` must be an integer between {SUBAGENT_MIN_TURNS} and {SUBAGENT_MAX_TURNS}"
                        ))
                    }
                }
            }
        };
        // The three parts that make a brief gradeable, and the role the
        // delegation runs under. Each is the parent's own words about
        // work the child cannot see, so a part that is absent and a part
        // that is empty read the same: nothing was said.
        let approach = optional_text(arguments, "approach")?;
        let context = optional_text(arguments, "context")?;
        let output = optional_text(arguments, "output")?;
        let role = optional_text(arguments, "role")?;
        // A role rides through as a name: a value that could not name a
        // role file is refused rather than narrowed to whatever a loader
        // would make of it, because a silent default is the one thing
        // the role argument must never be.
        if let Some(role) = &role {
            if role.contains('/') || role.contains('\\') || role.contains("..") {
                return Err(format!(
                    "`role` must be a role's name, not a path: `{role}` names no role under \
                     <FLEET_HOME>/roles/"
                ));
            }
        }
        let brief = Brief {
            task: goal.clone(),
            approach,
            context,
            output,
            role,
        };
        // The declaration is the scope, so the composed message names the
        // capabilities the child actually has rather than leaving it to
        // discover an unadmitted tool at the call.
        let admitted = tools.clone().unwrap_or_else(Scope::default_admitted);
        let composed = brief.compose(&admitted);
        let Brief {
            task,
            approach,
            context,
            output,
            role,
        } = brief;
        Ok(SubagentRequest {
            goal: composed,
            task,
            approach,
            context,
            output,
            role,
            tools,
            max_turns,
            node,
        })
    }
}

/// One optional brief part read off the arguments: the parent's own
/// words, trimmed, or `None` when the call carried nothing.
///
/// A part that is present but is not a string is a typed refusal rather
/// than a silent absence — a caller that passed a number meant to say
/// something, and reading it as nothing would drop the one part of the
/// brief the child cannot reconstruct for itself.
fn optional_text(arguments: &serde_json::Value, key: &str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) => {
            Ok(Some(text.trim().to_string()).filter(|text| !text.is_empty()))
        }
        Some(other) => Err(format!("`{key}` must be a string; {other} is not")),
    }
}

/// The route one child runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRoute {
    /// The child's model: the parent's own, always.
    pub model: String,
    /// The parent's active provider, when its client is routed. Carried
    /// so the progress line and the log can name the route the child
    /// actually took.
    pub provider: Option<String>,
}

/// Resolve the route one child runs on.
///
/// This is the in-product form of the owner's never-downgrade directive
/// (plan directive 2, hard rule 9), and the port of the fork's
/// model-selection preflight: the fork checks a child's requested route
/// against an allowed list before creating it, and Fleet's equivalent is
/// structural rather than advisory. A delegation carries no model
/// parameter at all — there is no `model` in the schema and none in the
/// request — so the only way a child could run cheaper than its parent
/// is by resolving a different model right here. It does not: the
/// child's model is the parent's, read from the parent's own options,
/// and the parent's active decision rides along only to be named.
pub fn child_route(parent_model: &str, decision: Option<&RouteDecision>) -> ChildRoute {
    ChildRoute {
        model: parent_model.to_string(),
        provider: decision.map(|decision| decision.provider.clone()),
    }
}

/// Why a child stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The child answered without requesting tools.
    Completed,
    /// The child's turn cap was reached first.
    MaxTurns,
    /// The child failed: an API error, or a session it could not open.
    Failed,
}

impl Outcome {
    /// The outcome's name, in the session log's own ending vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::MaxTurns => "max-turns",
            Outcome::Failed => "failed",
        }
    }
}

/// What one delegated run produced.
///
/// The shape the fork's descriptor projects and the shape the parent's
/// `tool_result` renders: the child's final message, where its work
/// lives, how far it got, and how it ended. A failure fills `summary`
/// with whatever the child managed and `reason` with the typed cause —
/// never an empty `Ok`, never silence.
#[derive(Debug, Clone)]
pub struct SubagentResult {
    /// The child's final message, or its last words when it did not
    /// finish.
    pub summary: String,
    /// The child's session id.
    pub session: String,
    /// The child's log path.
    pub log: PathBuf,
    /// The turns the child ran.
    pub turns: u64,
    pub outcome: Outcome,
    /// The typed failure reason, on [`Outcome::Failed`].
    pub reason: Option<String>,
    /// The goal the child was handed — the delegated task on its own.
    /// KI-075: a resumed session replays this result as an ordinary tool
    /// card, and the card's preview used to carry the outcome and the
    /// turn count but not what the child was asked to do. The goal rides
    /// the recorded preview the way the live card already shows it.
    pub goal: String,
    /// What this run reports about the angle between the approach it was
    /// handed and the one it executed — its own delta, the deltas of the
    /// delegations it made, and whether there was anything to diff
    /// against at all. The one field the structured report needs, so a
    /// caller constructs a result with `delta: Default::default()`.
    pub delta: DeltaReport,
}

/// What one delegated run reports about the angle between the approach
/// it was handed and the one it executed.
///
/// The child is the only participant who knows both sides, so the child
/// is the one who reports it — and the report is verified against the
/// child's own session log rather than believed, because a claimed
/// deviation the log does not show is a false report and the most
/// serious kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeltaReport {
    /// The run's own delta, when its brief handed it an approach to diff
    /// against. `None` when there was nothing to diff against, or when
    /// the child's answer carried no delta block — and [`offered`] says
    /// which of the two it was, because the difference matters: a brief
    /// that stated only a goal asks for no delta at all.
    ///
    /// [`offered`]: DeltaReport::offered
    pub own: Option<Delta>,
    /// The deltas of the delegations this run made, verbatim, in call
    /// order. A middle node preserves its children's deltas rather than
    /// summarizing them: a summarized delta cannot be graded, and the
    /// loss is invisible, which is the worst kind.
    pub children: Vec<Delta>,
    /// Whether the brief handed this run an approach to diff against. It
    /// is what makes an absent delta legible — a brief that offered one
    /// and got none back is a fact the parent's model needs to see
    /// rather than infer.
    pub offered: bool,
}

impl SubagentResult {
    /// A run that never got as far as a child session.
    fn refused(reason: String) -> SubagentResult {
        SubagentResult {
            summary: String::new(),
            session: String::new(),
            log: PathBuf::new(),
            turns: 0,
            outcome: Outcome::Failed,
            reason: Some(reason),
            delta: DeltaReport::default(),
            goal: String::new(),
        }
    }

    /// The model-visible text the parent's `tool_result` carries: one
    /// header line naming the outcome, the turns, the session, and the
    /// log, then the child's own words. It rides the same bounded
    /// `tool_result` path every other tool's output rides.
    pub fn render(&self) -> String {
        let mut text = self.render_report();
        text.push_str(&self.delta_section());
        text
    }

    /// The report half of a rendered result: the header, the outcome,
    /// and the child's own words. This is exactly the text this module
    /// rendered before the delta existed, which is what makes a
    /// delta-less result byte-identical to the one it always was.
    fn render_report(&self) -> String {
        let head = match &self.reason {
            Some(reason) => format!("subagent failed ({reason})"),
            None => format!("subagent {}", self.outcome.label()),
        };
        let mut text = format!(
            "{head} \u{b7} {} turn{} \u{b7} session {}",
            self.turns,
            plural(self.turns),
            self.session
        );
        // The goal, one line, bounded — the live card shows it, and the
        // recorded preview now carries it too (KI-075).
        if !self.goal.trim().is_empty() {
            let goal = crate::agent::preview(self.goal.trim(), 72);
            text.push_str(&format!(" \u{b7} goal: {goal}"));
        }
        if !self.log.as_os_str().is_empty() {
            text.push_str(&format!(" \u{b7} log {}", self.log.display()));
        }
        text.push('\n');
        if self.summary.trim().is_empty() {
            // Never silence: a child that produced no words still says
            // so, and still names where its work is.
            text.push_str(match self.outcome {
                Outcome::Completed => {
                    "(the child finished with no final text \u{2014} its log holds the turns)"
                }
                _ => "(no partial answer \u{2014} the child's log holds what it did)",
            });
        } else {
            text.push_str(&self.summary);
        }
        text
    }

    /// The delta half of a rendered result: the run's own delta block,
    /// then one block per delegation it made, each carrying its own
    /// subtree. Empty when there is nothing to report, and the absence
    /// is named rather than left blank — a brief that offered an
    /// approach and got no delta back is a fact the parent's model
    /// needs.
    fn delta_section(&self) -> String {
        let mut text = String::new();
        if let Some(delta) = &self.delta.own {
            text.push('\n');
            delta.render_into(OWN_PATH, &mut text);
        } else if self.delta.offered {
            text.push_str(
                "\n(no delta reported \u{2014} the child's answer carried no delta \
                           block, so nothing is known about whether it followed the approach)\n",
            );
        }
        for (index, child) in self.delta.children.iter().enumerate() {
            text.push('\n');
            child.render_into(&format!("{OWN_PATH}.{}", index + 1), &mut text);
        }
        text
    }
}

/// Everything one delegated run needs, gathered by the caller so the
/// borrow of the parent's approver and progress sink is explicit rather
/// than implicit.
pub struct Delegation<'a> {
    /// The parent's own client, routed or not. The child uses it only
    /// when the run has no home to route from (a test double); a run
    /// with a home gives the child its own route, judged over its brief
    /// — the router rules the child (the owner's 2026-10-21 ruling).
    pub client: &'a Client,
    /// The parent's route pin, when its run has one: the child routes
    /// under the same standing choice, narrowed to that provider's
    /// models the way the parent was.
    pub pin: Option<String>,
    /// The parent's lane specs, so the child's judge spawns the way the
    /// parent's would — a test's injected fake, or the vendored lane
    /// when the map is empty.
    pub lane_specs: std::collections::HashMap<String, crate::mcp::ServerSpec>,
    /// The parent's active route decision, when its client is routed.
    pub route: Option<RouteDecision>,
    /// The parent's model, from its own options.
    pub model: String,
    /// The sessions directory the child's log belongs in: the parent
    /// log's own directory, so a child never lands somewhere else.
    pub sessions_dir: PathBuf,
    /// The delegating session's id, recorded on the child's start record.
    pub parent_session: String,
    /// The child's first user message: the **composed brief**. The
    /// delegated task verbatim when the call carried none of the brief's
    /// optional parts — which is what keeps every existing caller
    /// byte-identical — and the four parts between their markers when it
    /// did.
    pub goal: String,
    /// The admitted scope, already validated.
    pub tools: Option<Vec<String>>,
    /// The child's turn cap.
    pub max_turns: usize,
    /// The parent's approver. The child's calls cross it, labelled.
    pub approver: &'a mut (dyn Approver + 'a),
    /// The parent's progress sink. A child's own progress lines land
    /// here, so a headless run shows the child working and a TUI's sink
    /// stays quiet.
    pub progress: &'a mut dyn Write,
    /// Where the parent's live surface listens, for the child's
    /// spawned/working/done progress. The child's own turn, token, and
    /// finish events deliberately do not ride here: they belong to the
    /// child's conversation, and landing them in the parent's would
    /// move its turn counter and clear its working state mid-run.
    pub ui_tx: Option<UnboundedSender<UiEvent>>,
    /// The calibration stream the parent's rows go to. The child's turn
    /// and screening rows land in the same stream, naturally: they are
    /// real turns on the real roster, and no new op is needed.
    pub calib: Option<Stream>,
    /// The skills the parent declared. The child declares the same list,
    /// because `use_skill` is in its admitted scope and a tool whose
    /// every call fails is not a tool.
    pub skills: Vec<Skill>,
    /// The parent's identity base, from its own options. The child
    /// inherits it the way it inherits the parent's model — the voice
    /// follows the model — while the rest of the parent's state (its
    /// memory, its lanes, its home) stays the parent's own.
    pub identity: Option<String>,
    /// The home the child's role resolves under — the owner's home,
    /// where `<home>/roles/<name>.md` lives. `None` is a run with no
    /// home: a brief that names a role then meets the named refusal,
    /// never a silent default.
    pub home: Option<PathBuf>,
    /// The child's delegation depth.
    pub depth: usize,
    /// Plan mode rides the parent's floor down: a child of a planning
    /// run plans too, because a delegation that could write would be a
    /// hole in the read-only promise the parent made.
    pub plan_mode: bool,
    /// The parent's file access, inherited the way the model and the
    /// voice are: a child's built-ins work in the chat's folder, and a
    /// child of a chat that never chose one has no filesystem access
    /// either. A child that worked somewhere else would be the one hole
    /// in the boundary the parent promised its owner.
    pub file_access: FileAccess,
}

/// Run one delegated task to a stop and return what it produced.
///
/// Synchronous from the parent's side: the parent's turn waits. Every
/// exit path returns a [`SubagentResult`] — a stop, a cap, an API
/// failure, a session that could not be opened, a depth that exceeds the
/// cap — and every one of them carries the child's log path once a log
/// exists.
pub async fn run(delegation: Delegation<'_>) -> SubagentResult {
    // The depth cap, asserted before anything is created. Unreachable
    // through the tool today — a run at the cap has `subagent` stripped
    // from its scope, so the loop refuses the call first — and named
    // here so a chain that somehow goes one deeper meets the guard
    // rather than inventing one.
    if delegation.depth > SUBAGENT_MAX_DEPTH {
        return SubagentResult::refused(format!(
            "subagent depth {} exceeds the cap {SUBAGENT_MAX_DEPTH}",
            delegation.depth
        ));
    }
    let Delegation {
        client,
        pin,
        lane_specs,
        route,
        model,
        sessions_dir,
        parent_session,
        goal,
        tools,
        max_turns,
        approver,
        progress,
        ui_tx,
        calib,
        skills,
        identity,
        home,
        depth,
        plan_mode,
        file_access,
    } = delegation;

    // The brief, read back out of the message the caller composed. The
    // composed form rides the `goal` argument because the caller that
    // builds the child is not the module that composes the message, and
    // the parts are what the row, the label, and the delta read.
    let brief = Brief::parse(&goal);
    // The role, when the brief named one: resolved against the owner's
    // home BEFORE anything is created, so an unknown role is a named
    // refusal the caller sees rather than a child that ran without its
    // specialization. A run with no home refuses the same way — the
    // role's file has nowhere to live.
    let role = match &brief.role {
        Some(name) => {
            let Some(home) = home.as_deref() else {
                return SubagentResult::refused(format!(
                    "the brief names role `{name}` but this run has no home to resolve it under — \
                     a role lives at <home>/roles/<name>.md"
                ));
            };
            match crate::roles::load(home, name) {
                Ok(role) => Some(role),
                Err(reason) => return SubagentResult::refused(reason),
            }
        }
        None => None,
    };
    // The role narrows the declaration: its admitted set is the
    // specialization, and a tool outside it is not merely uncallable in
    // the child, it is absent from what the child saw. A role that
    // names no tools narrows nothing.
    //
    // The base is the set the child would otherwise have run with —
    // the call's own declaration when it carried one, the harness's
    // default-admitted set when it did not — because narrowing an
    // absent declaration would leave the child with nothing at all.
    let tools = match &role {
        Some(role) if !role.tools.is_empty() => Some(
            tools
                .unwrap_or_else(Scope::default_admitted)
                .into_iter()
                .filter(|tool| role.admits(tool))
                .collect(),
        ),
        _ => tools,
    };
    // The role's skills ride beside the parent's, loaded from the same
    // home; a role naming a skill that is not there is named on one
    // line and skipped, the skills loader's own forgiving shape.
    let skills = match (&role, home.as_deref()) {
        (Some(role), Some(home)) if !role.skills.is_empty() => {
            let mut skills = skills;
            let live = crate::skills::load(home);
            for name in &role.skills {
                match live.iter().find(|skill| &skill.name == name) {
                    Some(skill) if !skills.iter().any(|carried| carried.name == skill.name) => {
                        skills.push(skill.clone());
                    }
                    Some(_) => {}
                    None => eprintln!(
                        "fleet: roles: role `{}` names skill `{name}`, which is not in the home's \
                         skills — skipped",
                        role.name
                    ),
                }
            }
            skills
        }
        _ => skills,
    };
    // The overlay rides the child's system prompt beside the compiled-in
    // one, delimited so the child knows what is standing instruction
    // for this delegation's role and what is the harness prompt.
    let role_overlay = role
        .as_ref()
        .map(crate::roles::overlay_prompt)
        .filter(|frame| !frame.is_empty());
    // The declaration is the scope, and the depth cap is the one thing
    // the declaration cannot override.
    let scope = Scope::child(tools, depth);
    let recorded_scope = scope.recorded();
    // Turn caps inherit downward, enforced here rather than asked of the
    // model: a child cannot outspend the parent's own budget, and a
    // depth-2 brief is a paraphrase of a paraphrase with a budget to
    // match.
    let parent_remaining = parent_remaining_turns(&sessions_dir, &parent_session, depth);
    let max_turns = inherit_turn_cap(max_turns, depth, parent_remaining);
    // The chain, from the logs beside this one, and the label it draws.
    let chain = Chain::resolve(&sessions_dir, &parent_session, depth);
    let label = chain.label(&brief, &sessions_dir);
    // The child's first user message, composed against the scope this run
    // was actually given: a brief that claimed a capability the depth cap
    // stripped would be a brief that lies to the child.
    let message = brief.compose(scope.names());
    // The stream the row lands in, cloned before the child's own options
    // take the parent's handle.
    let stream = calib.clone();
    let started = Instant::now();

    // The router rules the child: its route is judged over its own brief
    // and walked as the judged workload's chain, under the parent's pin
    // — never the parent's committed candidate. That needs a router to
    // rule it and a home to route from: a parent on a direct, unrouted
    // client (a test double, pointed at a mock) or a run with no home
    // hands the child the same client, which is the shape every test
    // double has always had. A route that cannot be prepared is a named
    // refusal, the way a parent's would have failed its startup.
    let own_route = match (home.as_deref(), client.router().is_some()) {
        (Some(home), true) => {
            match crate::route::prepare_child(home, &message, &lane_specs, pin.as_deref()).await {
                Ok(prepared) => Some(prepared),
                Err(error) => {
                    return SubagentResult::refused(format!(
                        "the child's route could not be prepared: {error}"
                    ))
                }
            }
        }
        _ => None,
    };
    let own_client = own_route
        .as_ref()
        .map(|prepared| Client::routed(prepared.router.clone()));
    let client: &Client = own_client.as_ref().unwrap_or(client);
    let (model, route) = match &own_route {
        Some(prepared) => (
            prepared.decision.model.clone(),
            Some(prepared.decision.clone()),
        ),
        None => (model, route),
    };

    let child_route = child_route(&model, route.as_ref());
    // KI-072: every constructor takes a home. A run with no home of its
    // own still lands its child's log beside the parent's: the sessions
    // directory's parent IS the home by the layout's own convention, so
    // resolving it back puts the child exactly where it was.
    let child_home = home
        .clone()
        .or_else(|| sessions_dir.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| sessions_dir.clone());
    let mut child = match Session::create_subagent(&child_home, &child_route.model, &parent_session)
    {
        Ok(child) => child,
        Err(error) => {
            return SubagentResult::refused(format!(
                "the child session could not be created: {error}"
            ))
        }
    };
    let session = child.id().to_string();
    let log = child.path().to_path_buf();

    let route_note = match &child_route.provider {
        Some(provider) => format!(" on {provider}"),
        None => String::new(),
    };
    let _ = writeln!(
        progress,
        "\u{2014} subagent: {label} \u{b7} child {session} \u{b7} {max_turns}-turn cap{route_note}",
    );
    let _ = progress.flush();
    emit(
        &ui_tx,
        UiEvent::Subagent {
            goal: label.clone(),
            session: session.clone(),
            stage: SubagentStage::Spawned,
        },
    );

    let options = AgentOptions {
        model: child_route.model,
        max_turns,
        identity,
        ui_tx: None,
        lane_specs: None,
        calib,
        skills,
        role_overlay,
        memory: None,
        scope: Some(scope),
        // The child sees nothing of the parent: no memory, no lanes, no
        // home to file proposals into. The scope excludes `improve`
        // either way, so the declaration and the call agree.
        improve_home: None,
        gui_prompt: None,
        compact_now: false,
        // The child inherits the parent's floor: a planning parent's
        // child plans, and the parent's checkpoint store keys the
        // child's captures by the child's own session id.
        checkpoints: None,
        // The parent's access rides down with the rest of the run's
        // identity: the folder when there is one, and the refusal when
        // there is not.
        file_access,
        plan_mode,
    };
    let gate = InheritedApprover {
        inner: approver,
        label: label.clone(),
        ui_tx: ui_tx.clone(),
        session: session.clone(),
        calls: 0,
    };
    // The child is scoped to its own block so the borrow of `child`
    // ends before the log is read back for the partial result.
    let outcome = {
        let mut agent = match Agent::new(client, options, &mut child, Box::new(gate), progress) {
            Ok(agent) => agent,
            Err(error) => {
                let result = SubagentResult {
                    summary: String::new(),
                    goal: goal.clone(),
                    session,
                    log,
                    turns: 0,
                    outcome: Outcome::Failed,
                    reason: Some(error.to_string()),
                    delta: DeltaReport {
                        own: None,
                        children: Vec::new(),
                        offered: brief.offers_an_approach(),
                    },
                };
                // A run that never reached the loop still lands its row:
                // a delegation nobody recorded is a delegation the loop
                // cannot grade, and the reason it never ran is the most
                // gradeable fact about it.
                land_delegation_row(
                    stream.as_ref(),
                    delegation_row(
                        &brief,
                        &recorded_scope,
                        child_route.provider.as_deref(),
                        &result,
                        &chain,
                        started.elapsed().as_millis() as u64,
                    ),
                );
                emit(
                    &ui_tx,
                    UiEvent::Subagent {
                        goal: label.clone(),
                        session: result.session.clone(),
                        stage: SubagentStage::Done {
                            outcome: Outcome::Failed.label().to_string(),
                            turns: 0,
                        },
                    },
                );
                return result;
            }
        };
        agent.run(&message).await
    };
    // The child's own decision row, with every attempt its walk made:
    // the same stream the parent's row lands in, so the judgment that
    // routed the child is graded like any other.
    if let Some(prepared) = &own_route {
        prepared.record_route(&message);
    }

    let records = crate::session::replay(&log).unwrap_or_default();
    let mut result = match outcome {
        Ok(report) => SubagentResult {
            summary: report.final_message.unwrap_or_default(),
            session,
            log,
            turns: report.turns,
            outcome: match report.stop_reason {
                crate::agent::StopReason::Completed => Outcome::Completed,
                crate::agent::StopReason::MaxTurns => Outcome::MaxTurns,
            },
            reason: None,
            goal: goal.clone(),
            delta: DeltaReport {
                own: None,
                children: Vec::new(),
                offered: brief.offers_an_approach(),
            },
        },
        Err(error) => SubagentResult {
            summary: String::new(),
            session,
            log,
            // The turns the child actually completed, read back from its
            // own log: the loop's counter would name the turn it died on.
            turns: turns_completed(&records),
            outcome: Outcome::Failed,
            reason: Some(error.to_string()),
            goal: goal.clone(),
            delta: DeltaReport {
                own: None,
                children: Vec::new(),
                offered: brief.offers_an_approach(),
            },
        },
    };
    // A child that stopped without a final message still returns its
    // partial result: the last words in its own log. Silence is the one
    // thing this path never returns. A child that *completed* keeps its
    // own (possibly empty) final message — an earlier turn's text is not
    // its answer.
    if result.outcome != Outcome::Completed && result.summary.trim().is_empty() {
        if let Some(words) = last_words(&records) {
            result.summary = words;
        }
    }
    // The delta the child reported, verified against its own session log
    // rather than believed, and the deltas of the delegations it made,
    // read off the same log. A claimed deviation the log does not show
    // is a false report, and the contradiction rides the delta.
    result.delta = DeltaReport {
        own: Delta::parse(&result.summary).map(|delta| delta.verify(&records)),
        children: delegated_deltas(&records),
        offered: result.delta.offered,
    };
    let _ = writeln!(
        progress,
        "\u{2190} subagent {} \u{b7} {} turn{}",
        result.outcome.label(),
        result.turns,
        plural(result.turns)
    );
    let _ = progress.flush();
    emit(
        &ui_tx,
        UiEvent::Subagent {
            goal: label.clone(),
            session: result.session.clone(),
            stage: SubagentStage::Done {
                outcome: result.outcome.label().to_string(),
                turns: result.turns,
            },
        },
    );
    // The row, last: the run is over, so the outcome, the turns, and the
    // duration are the ones that actually happened.
    land_delegation_row(
        stream.as_ref(),
        delegation_row(
            &brief,
            &recorded_scope,
            child_route.provider.as_deref(),
            &result,
            &chain,
            started.elapsed().as_millis() as u64,
        ),
    );
    result
}

/// Land one completed run's delegation row through the stream's own
/// writer. A run whose parent carried no stream — a test, or a caller
/// that never opened one — records nothing and is not failed by it: the
/// stream is advisory by construction, and no delegation is ever
/// blocked, delayed, or failed by its own audit trail.
///
/// The write goes through the stream rather than through the beat that
/// reads the rows, because the beat's own guard is that no synchronous
/// path may name it — a beat that spends a lane ask must be unreachable
/// from a path that runs on every turn, and this is a path that runs on
/// every delegation.
fn land_delegation_row(stream: Option<&Stream>, row: DelegatedRunRow) {
    if let Some(stream) = stream {
        calib::record_delegation(stream, row);
    }
}

/// One completed run's delegation row: the brief's shape, the scope, the
/// outcome, the turns, the spend, the duration, the child's log path,
/// and the parent chain.
///
/// The child's own summary is not a field, by design: the row is the
/// objective evidence and the summary is the child's report, so grading
/// the row grades the work rather than the model's confidence in
/// itself. The `ts` is left empty for the writer, which stamps it with
/// the stream's own clock so a run that finished at 23:59:59 lands in
/// the day it finished in.
fn delegation_row(
    brief: &Brief,
    scope: &str,
    provider: Option<&str>,
    result: &SubagentResult,
    chain: &Chain,
    duration_ms: u64,
) -> DelegatedRunRow {
    DelegatedRunRow {
        ts: String::new(),
        // The row's key: the child's own session id, the join to the
        // child's session log and the value an owner can type.
        session: result.session.clone(),
        brief_shape: brief.shape(),
        scope: scope.to_string(),
        provider: provider.unwrap_or_default().to_string(),
        outcome: result.outcome.label().to_string(),
        turns: result.turns,
        // The crate prices no model tokens — there is no price table
        // anywhere in it — so the spend is zero rather than invented,
        // and the judge's prompt reads `unpriced` instead of printing a
        // confident `$0.0000`.
        spend_usd: 0.0,
        duration_ms,
        log: result.log.display().to_string(),
        reason: result.reason.clone(),
        parent_session: chain.parent.clone(),
        root_session: chain.root.clone(),
        depth: chain.depth as u32,
    }
}

/// The parent's [`Approver`], seen from the child's side.
///
/// One gate for the whole tree: the owner sees a subagent's `bash` call
/// before it runs, exactly as they see the parent's. The forwarded call
/// is renamed `subagent \u{2192} <tool>` and carries the label, so the
/// prompt answers "whose call is this?" without the owner having to
/// know which session is in flight. At depth 2 the label is the whole
/// chain — root goal, parent goal, this goal — because an owner
/// approving a call whose originating intent is two briefs away is
/// approving a paraphrase.
struct InheritedApprover<'a> {
    inner: &'a mut (dyn Approver + 'a),
    label: String,
    ui_tx: Option<UnboundedSender<UiEvent>>,
    session: String,
    /// How many of the child's calls this gate has forwarded. The
    /// child's liveness signal, reported to the parent's surface.
    calls: u64,
}

impl Approver for InheritedApprover<'_> {
    fn approve(&mut self, call: &ToolCall) -> bool {
        self.calls += 1;
        emit(
            &self.ui_tx,
            UiEvent::Subagent {
                goal: self.label.clone(),
                session: self.session.clone(),
                stage: SubagentStage::Working {
                    calls: self.calls,
                    tool: call.function.name.clone(),
                },
            },
        );
        self.inner.approve(&labelled(call, &self.label))
    }

    fn latched(&self) -> bool {
        self.inner.latched()
    }
}

/// The child's call as the parent's gate sees it: named for the
/// subagent it came from, carrying the label beside the call's own
/// arguments so the prompt can name both.
fn labelled(call: &ToolCall, label: &str) -> ToolCall {
    let arguments: serde_json::Value =
        serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::Value::Null);
    ToolCall {
        id: call.id.clone(),
        kind: call.kind.clone(),
        function: FunctionCall {
            name: format!("{}{}", tools::subagent_call_prefix(), call.function.name),
            arguments: serde_json::json!({
                "goal": bounded(label, GOAL_PREVIEW),
                "tool": call.function.name,
                "arguments": arguments,
            })
            .to_string(),
        },
    }
}

/// The child's last words, for a run that did not finish: the last
/// assistant message carrying text in its own log. `None` when the child
/// never got a word out.
fn last_words(records: &[Record]) -> Option<String> {
    records.iter().rev().find_map(|record| match record {
        Record::Assistant {
            content: Some(text),
            ..
        } if !text.trim().is_empty() => Some(text.clone()),
        _ => None,
    })
}

/// How many turns a run completed, read back from its log: one assistant
/// record per turn that produced a message. A run that failed mid-request
/// has no record for the turn it died on, so the log's count is the honest
/// "how far it got" — the loop's own counter would name a turn the model
/// never answered.
fn turns_completed(records: &[Record]) -> u64 {
    records
        .iter()
        .filter(|record| matches!(record, Record::Assistant { .. }))
        .count() as u64
}

/// Send one event when a surface is listening. A closed channel is not
/// a failure: the session log and the run report are the record.
fn emit(ui_tx: &Option<UnboundedSender<UiEvent>>, event: UiEvent) {
    if let Some(tx) = ui_tx {
        let _ = tx.send(event);
    }
}

/// `s` for anything but one.
fn plural(count: u64) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// First `limit` characters of `text` with an elision marker when
/// anything was cut, for a progress line or a prompt.
fn bounded(text: &str, limit: usize) -> String {
    let mut cut: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        cut.push('\u{2026}');
    }
    cut
}

/// Bound one child's rendered result the way every other tool's output
/// is bounded, so a child that wrote a novel still rides the parent's
/// ordinary `tool_result` path.
///
/// The delta section rides *after* the bound rather than inside it: a
/// child that wrote a novel must not cost the report of what it
/// actually did, and the delta is the one part of a result the next
/// level up has to be able to read. A result with no delta is bounded
/// exactly as it always was.
pub fn bound_result(result: &SubagentResult) -> ToolOutcome {
    let report = tools::bound(ToolOutcome {
        output: result.render_report(),
        truncated: false,
    });
    let mut output = report.output;
    output.push_str(&result.delta_section());
    ToolOutcome {
        output,
        truncated: report.truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentError;

    /// KI-075: the recorded preview carries the goal — a resumed session
    /// replays this result as an ordinary tool card, and the card must
    /// name what the child was asked to do, the way the live card does.
    #[test]
    fn the_recorded_preview_carries_the_goal() {
        let result = SubagentResult {
            summary: "the child's words".to_string(),
            session: "s-1".to_string(),
            log: std::path::PathBuf::from("/tmp/s-1.jsonl"),
            turns: 2,
            outcome: Outcome::Completed,
            reason: None,
            goal: "triage the KI ledger".to_string(),
            delta: DeltaReport::default(),
        };
        let text = result.render_report();
        assert!(text.contains("goal: triage the KI ledger"), "{text}");
        assert!(text.contains("session s-1"), "{text}");
        // No goal — an older result — renders exactly as it did.
        let plain = SubagentResult {
            goal: String::new(),
            ..result
        };
        assert!(
            !plain.render_report().contains("goal:"),
            "{}",
            plain.render_report()
        );
    }

    #[test]
    fn the_default_scope_is_the_four_builtins_plus_use_skill() {
        let scope = Scope::child(None, 1);
        assert_eq!(scope.depth, 1);
        assert_eq!(
            scope.names(),
            &vec![
                "bash".to_string(),
                "read_file".to_string(),
                "write_file".to_string(),
                "list_dir".to_string(),
                "use_skill".to_string(),
            ]
        );
        // Everything the design leaves out is out by construction.
        for excluded in [
            tools::REMEMBER_TOOL,
            tools::SUPERSEDE_TOOL,
            tools::SUBAGENT_TOOL,
        ] {
            assert!(
                !scope.admits(excluded),
                "{excluded} must not be admitted by default"
            );
        }
        assert!(
            !scope.admits("mcp__sem1__probe"),
            "a lane tool must not be admitted"
        );
    }

    #[test]
    fn a_declared_tool_is_admitted_and_an_undeclared_one_is_refused_by_name() {
        // The declaration IS the scope: what the call named is what the
        // child can reach, and the row records the same list.
        let scope = Scope::child(Some(vec!["read_file".to_string(), "bash".to_string()]), 1);
        assert!(scope.admits("read_file"));
        assert!(scope.admits("bash"));
        assert!(
            !scope.admits("write_file"),
            "an undeclared tool is not admitted"
        );
        assert!(
            !scope.admits("use_skill"),
            "nor is one the default would have carried"
        );
        assert_eq!(scope.recorded(), "read_file,bash");
        // A scope that admits nothing says so rather than recording an
        // empty string a reader would have to interpret.
        assert_eq!(Scope::child(Some(Vec::new()), 1).recorded(), "none");
    }

    #[test]
    fn the_delegation_tool_is_admitted_at_depth_one_and_stripped_at_the_cap() {
        // The n+2 model, in its declarative half: a delegation that
        // names `subagent` opens a second level, and a run at the cap
        // cannot be admitted to it however the call asked.
        let declared = Some(vec!["bash".to_string(), tools::SUBAGENT_TOOL.to_string()]);
        let opened = Scope::child(declared.clone(), 1);
        assert!(
            opened.admits(tools::SUBAGENT_TOOL),
            "a depth-1 scope may be admitted to it"
        );
        assert_eq!(SUBAGENT_MAX_DEPTH, 2, "the cap reads two");
        let capped = Scope::child(declared, SUBAGENT_MAX_DEPTH);
        assert!(
            !capped.admits(tools::SUBAGENT_TOOL),
            "a depth-2 scope cannot be admitted to it, so the loop refuses the call by name citing the cap"
        );
        assert_eq!(capped.names(), &vec!["bash".to_string()]);
        // The default is unchanged at every depth: a chain stays at one
        // level unless the orchestrator declares a second.
        for depth in [1, 2] {
            assert!(!Scope::child(None, depth).admits(tools::SUBAGENT_TOOL));
        }
    }

    #[test]
    fn the_turn_cap_default_steps_down_with_depth() {
        assert_eq!(turn_cap_for_depth(1), SUBAGENT_TURN_CAP);
        assert_eq!(turn_cap_for_depth(2), SUBAGENT_TURN_CAP / 2);
        // The floor holds: a deep chain is starved, never silenced.
        assert_eq!(turn_cap_for_depth(9), SUBAGENT_TURN_CAP_FLOOR);
        // Depth 1 keeps its own bound — a first-level delegation is the
        // case this crate has always had — and depth 2 is held to the
        // stepped-down default whatever it asked for.
        assert_eq!(turn_ceiling(1), SUBAGENT_MAX_TURNS);
        assert_eq!(turn_ceiling(2), SUBAGENT_TURN_CAP / 2);
    }

    #[test]
    fn a_childs_cap_cannot_exceed_its_parents_remaining_turns() {
        // Enforced at construction, not asked of the model.
        assert_eq!(inherit_turn_cap(30, 1, None), 30, "depth 1 is untouched");
        assert_eq!(inherit_turn_cap(30, 2, None), SUBAGENT_TURN_CAP / 2);
        // A parent with two turns left hands its child two turns.
        assert_eq!(inherit_turn_cap(20, 2, Some(2)), 2);
        // A parent with plenty left still cannot lend more than the
        // depth-stepped ceiling.
        assert_eq!(inherit_turn_cap(20, 2, Some(19)), SUBAGENT_TURN_CAP / 2);
        // A log that reads as exhausted still hands the child one turn
        // rather than none.
        assert_eq!(inherit_turn_cap(20, 2, Some(0)), SUBAGENT_MIN_TURNS);
    }

    #[test]
    fn a_requested_scope_narrows_and_keeps_its_order() {
        let scope = Scope::child(Some(vec!["read_file".to_string(), "bash".to_string()]), 1);
        assert_eq!(
            scope.names(),
            &vec!["read_file".to_string(), "bash".to_string()]
        );
        assert!(!scope.admits("write_file"));
    }

    #[test]
    fn an_empty_requested_scope_admits_nothing() {
        // A child with no tools is a legitimate narrowing: it can only
        // answer.
        let scope = Scope::child(Some(Vec::new()), 1);
        assert!(scope.names().is_empty());
        assert!(!scope.admits("bash"));
    }

    #[test]
    fn a_goal_is_required_and_must_say_something() {
        for missing in [
            serde_json::json!({}),
            serde_json::json!({ "goal": "" }),
            serde_json::json!({ "goal": "   " }),
            serde_json::json!({ "goal": 7 }),
        ] {
            let error = SubagentRequest::parse(&missing).unwrap_err();
            assert!(error.contains("goal"), "{error}");
        }
    }

    #[test]
    fn the_turn_cap_defaults_and_is_bounded_both_ways() {
        let asked = SubagentRequest::parse(&serde_json::json!({ "goal": "go" })).unwrap();
        assert_eq!(asked.max_turns, SUBAGENT_TURN_CAP);
        let capped =
            SubagentRequest::parse(&serde_json::json!({ "goal": "go", "max-turns": 7 })).unwrap();
        assert_eq!(capped.max_turns, 7);
        for out_of_range in [0, 51, 1_000] {
            let error = SubagentRequest::parse(&serde_json::json!({
                "goal": "go",
                "max-turns": out_of_range,
            }))
            .unwrap_err();
            assert!(error.contains("max-turns"), "{error}");
        }
    }

    #[test]
    fn an_inadmissible_tool_is_refused_by_name() {
        // The durable-write tools are never admissible: a child that
        // could write the memory corpus or retire a fact from it would
        // be a second writer of state the parent owns.
        for name in [
            tools::REMEMBER_TOOL,
            tools::SUPERSEDE_TOOL,
            tools::IMPROVE_TOOL,
        ] {
            let error = SubagentRequest::parse(&serde_json::json!({
                "goal": "go",
                "tools": ["bash", name],
            }))
            .unwrap_err();
            assert!(error.contains(name), "{error}");
            assert!(error.contains("not admissible"), "{error}");
        }
        // `subagent` IS admissible — the cap is two, and naming it opens a
        // second level — so the refusal that meets a depth-2 call is the
        // scope's, not the parse's.
        let declared = SubagentRequest::parse(&serde_json::json!({
            "goal": "go",
            "tools": ["bash", tools::SUBAGENT_TOOL],
        }))
        .unwrap();
        assert!(declared
            .tools
            .unwrap()
            .contains(&tools::SUBAGENT_TOOL.to_string()));
        // A lane tool is refused the same way, by its namespaced name.
        let error = SubagentRequest::parse(&serde_json::json!({
            "goal": "go",
            "tools": ["mcp__sem1__probe"],
        }))
        .unwrap_err();
        assert!(error.contains("mcp__sem1__probe"), "{error}");
    }

    #[test]
    fn a_repeated_tool_narrows_nothing_twice() {
        let request = SubagentRequest::parse(&serde_json::json!({
            "goal": "go",
            "tools": ["bash", "bash", "read_file"],
        }))
        .unwrap();
        let scope = Scope::child(request.tools.clone(), 1);
        assert_eq!(
            scope.names(),
            &vec!["bash".to_string(), "read_file".to_string()]
        );
    }

    #[test]
    fn a_child_runs_on_the_parents_model_whatever_the_decision_says() {
        // The never-downgrade guard, in its testable form: the child's
        // model is the parent's, and no decision can move it.
        let decision = RouteDecision {
            provider: "zai-coding-plan".to_string(),
            model: "GLM-5.3-Flash".to_string(),
            how: crate::route::How::Judged,
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            reason: "sys1 judged `hard`".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string())],
        };
        let routed = child_route("test-model", Some(&decision));
        assert_eq!(routed.model, "test-model");
        assert_eq!(routed.provider.as_deref(), Some("zai-coding-plan"));
        // An unrouted parent's child is unrouted too, and still on the
        // parent's model.
        let plain = child_route("test-model", None);
        assert_eq!(plain.model, "test-model");
        assert_eq!(plain.provider, None);
    }

    #[test]
    fn a_result_renders_its_outcome_its_turns_and_its_log() {
        let result = SubagentResult {
            summary: "found three call sites".to_string(),
            session: "20261014T000000Z-abcdef".to_string(),
            log: PathBuf::from("/tmp/sessions/20261014T000000Z-abcdef.jsonl"),
            turns: 3,
            outcome: Outcome::Completed,
            reason: None,
            delta: DeltaReport::default(),
            goal: String::new(),
        };
        let text = result.render();
        assert!(
            text.starts_with("subagent completed \u{b7} 3 turns \u{b7} session "),
            "{text}"
        );
        assert!(
            text.contains("/tmp/sessions/20261014T000000Z-abcdef.jsonl"),
            "{text}"
        );
        assert!(text.ends_with("found three call sites"), "{text}");
        // One turn is not "1 turns".
        let one = SubagentResult {
            turns: 1,
            ..result.clone()
        };
        assert!(one.render().contains("1 turn \u{b7}"), "{}", one.render());
    }

    #[test]
    fn a_failure_renders_its_reason_and_never_returns_silence() {
        let failed = SubagentResult {
            summary: "checked two of five files".to_string(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/sessions/child-1.jsonl"),
            turns: 2,
            outcome: Outcome::Failed,
            reason: Some("API error 402: the account needs a balance top-up".to_string()),
            delta: DeltaReport::default(),
            goal: String::new(),
        };
        let text = failed.render();
        assert!(text.contains("subagent failed (API error 402"), "{text}");
        assert!(text.contains("checked two of five files"), "{text}");
        // No words at all still says so, and still names the log.
        let silent = SubagentResult {
            summary: String::new(),
            ..failed.clone()
        };
        let text = silent.render();
        assert!(text.contains("no partial answer"), "{text}");
        assert!(text.contains("child-1.jsonl"), "{text}");
    }

    #[test]
    fn a_refused_run_names_its_reason_and_has_no_log() {
        let refused =
            SubagentResult::refused("the child session could not be created: nope".to_string());
        assert_eq!(refused.outcome, Outcome::Failed);
        assert!(refused.session.is_empty());
        assert!(refused.log.as_os_str().is_empty());
        assert!(refused
            .render()
            .contains("the child session could not be created"));
    }

    #[test]
    fn a_forwarded_call_is_named_for_the_subagent_and_carries_the_goal() {
        let call = ToolCall {
            id: "call-1".to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: "bash".to_string(),
                arguments: r#"{"command":"ls"}"#.to_string(),
            },
        };
        let forwarded = labelled(&call, "summarize the repo");
        assert_eq!(forwarded.function.name, "subagent \u{2192} bash");
        let arguments: serde_json::Value =
            serde_json::from_str(&forwarded.function.arguments).unwrap();
        assert_eq!(arguments["goal"], "summarize the repo");
        assert_eq!(arguments["tool"], "bash");
        assert_eq!(arguments["arguments"]["command"], "ls");
        // The child's own call is untouched: the labelling happens on the
        // way to the gate, never on the way back.
        assert_eq!(call.function.name, "bash");
    }

    #[test]
    fn a_long_goal_is_bounded_for_a_prompt_and_a_progress_line() {
        let long = "x".repeat(200);
        assert_eq!(
            bounded(&long, GOAL_PREVIEW).chars().count(),
            GOAL_PREVIEW + 1
        );
        assert!(bounded(&long, GOAL_PREVIEW).ends_with('\u{2026}'));
        assert_eq!(bounded("short", GOAL_PREVIEW), "short");
    }

    #[test]
    fn the_last_words_of_a_log_are_the_last_assistant_text() {
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".to_string(),
                created: "c".to_string(),
                kind: Some(crate::session::SUBAGENT_KIND.to_string()),
                parent: Some("p".to_string()),
                workspace: None,
            },
            Record::User {
                content: "the goal".to_string(),
            },
            Record::Assistant {
                content: Some("first try".to_string()),
                tool_calls: vec![],
            },
            Record::Assistant {
                content: None,
                tool_calls: vec![],
            },
            Record::Assistant {
                content: Some("last words".to_string()),
                tool_calls: vec![],
            },
            Record::End {
                stop_reason: "max-turns".to_string(),
                turns: 3,
                served_model: None,
            },
        ];
        assert_eq!(last_words(&records).as_deref(), Some("last words"));
        // A log with no assistant text has no last words.
        let empty = vec![Record::User {
            content: "the goal".to_string(),
        }];
        assert_eq!(last_words(&empty), None);
    }

    #[test]
    fn an_agent_error_names_the_class_the_calibration_row_would() {
        // The typed reason a failed child returns is the agent error's
        // own display, so the parent's log and the child's turn row name
        // the same cause.
        let error = AgentError::Api(crate::api::ClientError::Api(crate::api::ApiError {
            status: 402,
            message: "Insufficient Balance".to_string(),
        }));
        let result = SubagentResult {
            summary: String::new(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/child-1.jsonl"),
            turns: 1,
            outcome: Outcome::Failed,
            reason: Some(error.to_string()),
            delta: DeltaReport::default(),
            goal: String::new(),
        };
        assert!(
            result.reason.as_deref().unwrap().contains("402"),
            "{:?}",
            result.reason
        );
        assert!(result.render().contains("Insufficient Balance"));
    }

    // --- the brief ----------------------------------------------------------

    #[test]
    fn a_brief_with_no_new_parts_composes_exactly_todays_message() {
        let brief = Brief {
            task: "count the files in /tmp".to_string(),
            ..Brief::default()
        };
        // Byte-identical to the message this module has always sent: the
        // goal, whole, with no marker anywhere.
        assert_eq!(
            brief.compose(&Scope::default_admitted()),
            "count the files in /tmp"
        );
        // And the parse of a marker-free message reads it as the task.
        assert_eq!(Brief::parse("count the files in /tmp"), brief);
    }

    #[test]
    fn a_full_brief_composes_its_four_parts_between_markers() {
        let brief = Brief {
            task: "count the files".to_string(),
            approach: Some("use list_dir once, then count".to_string()),
            context: Some("the sessions live in ~/.fleet/sessions".to_string()),
            output: Some("a number and the command that produced it".to_string()),
            role: None,
        };
        let text = brief.compose(&Scope::default_admitted());
        assert!(
            text.starts_with("--- TASK ---\ncount the files\n"),
            "{text}"
        );
        for marker in [
            "--- APPROACH ---",
            "--- CONTEXT ---",
            "--- WHAT DONE LOOKS LIKE ---",
        ] {
            assert!(text.contains(marker), "{marker} is missing from\n{text}");
        }
        // The approach is a proposal rather than an instruction, and the
        // delta block rides beside it: a child that finds a better route
        // is deviating from a suggestion, not disobeying an order.
        assert!(
            text.contains("A proposed solution, not an instruction"),
            "{text}"
        );
        assert!(text.contains("--- DELTA ---"), "{text}");
        assert!(text.contains("companion-missing"), "{text}");
        // The round trip is exact: the parts read back as they went in.
        assert_eq!(Brief::parse(&text), brief);
        // The shape names the parts it carried and carries no text of
        // its own — it is the cell key, not the brief.
        assert_eq!(brief.shape(), "task+approach+context+done");
        assert!(!brief.shape().contains("list_dir"));
        assert!(!brief.shape().contains("sessions"));
    }

    #[test]
    fn the_declaration_rides_the_briefs_context_section() {
        let brief = Brief {
            task: "count the files".to_string(),
            ..Brief::default()
        };
        let declared = Scope::child(Some(vec!["bash".to_string(), "read_file".to_string()]), 1);
        let text = brief.compose(declared.names());
        // The child is told what it has and, by omission, what it does
        // not — so it can ask for the delta in its result rather than
        // failing silently at the call.
        assert!(text.contains("CAPABILITIES: bash, read_file"), "{text}");
        assert!(
            !text.contains("write_file"),
            "an undeclared tool is not named as held"
        );
        assert_eq!(Brief::parse(&text).task, "count the files");
        // The default scope is not spelled out: the child's own
        // declarations already name it, and repeating them would change
        // the shape of a brief for nothing.
        assert_eq!(brief.compose(&Scope::default_admitted()), "count the files");
    }

    #[test]
    fn a_role_rides_the_brief_and_the_parse_reads_it_back() {
        let brief = Brief {
            task: "review the diff".to_string(),
            role: Some("reviewer".to_string()),
            ..Brief::default()
        };
        let text = brief.compose(&Scope::default_admitted());
        assert!(text.contains("ROLE: reviewer"), "{text}");
        assert_eq!(Brief::parse(&text), brief);
    }

    #[test]
    fn the_briefs_label_is_the_task_alone_until_it_has_parts() {
        let plain = Brief {
            task: "count the files".to_string(),
            ..Brief::default()
        };
        // Exactly the label this module has always drawn.
        assert_eq!(plain.label(), "count the files");
        let composed = Brief {
            task: "count".to_string(),
            context: Some("the sessions".to_string()),
            ..Brief::default()
        };
        let label = composed.label();
        // The four parts, bounded and delimited: never a raw dump on a
        // progress line or in an approval prompt.
        assert!(label.contains("task count"), "{label}");
        assert!(label.contains("context the sessions"), "{label}");
    }

    #[test]
    fn a_call_without_the_new_parts_composes_todays_message() {
        let asked =
            SubagentRequest::parse(&serde_json::json!({ "goal": "count the files" })).unwrap();
        assert_eq!(asked.goal, "count the files");
        assert_eq!(asked.task, "count the files");
        assert_eq!(asked.approach, None);
        assert_eq!(asked.context, None);
        assert_eq!(asked.output, None);
        assert_eq!(asked.role, None);
    }

    #[test]
    fn the_three_optional_parts_compose_into_the_childs_first_message() {
        let asked = SubagentRequest::parse(&serde_json::json!({
            "goal": "count the files",
            "context": "the sessions live in ~/.fleet/sessions",
            "output": "a number",
            "approach": "use list_dir once",
            "role": "reviewer",
            "tools": ["bash"],
        }))
        .unwrap();
        // The goal the caller composes is the child's first message, and
        // the task rides beside it for the row, the label, and the delta.
        assert_ne!(asked.goal, "count the files");
        assert!(asked.goal.contains("--- TASK ---"));
        assert_eq!(asked.task, "count the files");
        assert_eq!(
            asked.context.as_deref(),
            Some("the sessions live in ~/.fleet/sessions")
        );
        assert_eq!(asked.output.as_deref(), Some("a number"));
        assert_eq!(asked.approach.as_deref(), Some("use list_dir once"));
        assert_eq!(asked.role.as_deref(), Some("reviewer"));
        // The declared capability is admitted; the default is not what
        // the child gets.
        assert_eq!(asked.tools, Some(vec!["bash".to_string()]));
    }

    #[test]
    fn a_part_that_is_not_a_string_is_a_typed_refusal() {
        for key in ["context", "output", "approach", "role"] {
            let error =
                SubagentRequest::parse(&serde_json::json!({ "goal": "go", key: 7 })).unwrap_err();
            assert!(error.contains(key), "{error}");
        }
        // An empty part reads as no part: nothing was said.
        let asked =
            SubagentRequest::parse(&serde_json::json!({ "goal": "go", "context": "   " })).unwrap();
        assert_eq!(asked.context, None);
    }

    #[test]
    fn a_role_that_could_not_name_a_role_is_refused_by_name() {
        // A silent default is the one thing the role argument must never
        // be, so a value that names no role file is refused rather than
        // narrowed to whatever a loader would make of it.
        for role in ["../etc/passwd", "roles/reviewer", "a\\b"] {
            let error = SubagentRequest::parse(&serde_json::json!({ "goal": "go", "role": role }))
                .unwrap_err();
            assert!(error.contains("role"), "{error}");
        }
    }

    #[test]
    fn the_delegation_tool_is_declarable_now_that_the_cap_is_two() {
        let asked = SubagentRequest::parse(&serde_json::json!({
            "goal": "delegate onward",
            "tools": ["bash", "subagent"],
        }))
        .unwrap();
        assert_eq!(
            asked.tools,
            Some(vec!["bash".to_string(), "subagent".to_string()]),
            "naming `subagent` opens a second delegation level"
        );
    }

    // --- the parent chain ---------------------------------------------------

    /// One session log written straight to disk: the `start` record
    /// naming its kind and parent, the brief it was handed as its first
    /// user message, and `turns` assistant records. The shape a real run
    /// leaves, so the chain walk reads what a real chain would.
    fn write_log(
        dir: &std::path::Path,
        session: &str,
        kind: Option<&str>,
        parent: Option<&str>,
        brief: &str,
        turns: usize,
    ) {
        std::fs::create_dir_all(dir).unwrap();
        let mut text = serde_json::to_string(&Record::Start {
            v: 1,
            model: "test-model".to_string(),
            created: "2026-10-15T00:00:00Z".to_string(),
            kind: kind.map(str::to_string),
            parent: parent.map(str::to_string),
            workspace: None,
        })
        .unwrap();
        text.push('\n');
        text.push_str(
            &serde_json::to_string(&Record::User {
                content: brief.to_string(),
            })
            .unwrap(),
        );
        text.push('\n');
        for _ in 0..turns {
            text.push_str(
                &serde_json::to_string(&Record::Assistant {
                    content: None,
                    tool_calls: vec![],
                })
                .unwrap(),
            );
            text.push('\n');
        }
        std::fs::write(dir.join(format!("{session}.jsonl")), text).unwrap();
    }

    #[test]
    fn the_chain_is_resolved_from_the_logs_beside_the_run() {
        let dir = std::env::temp_dir().join(format!("fleet-chain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // The orchestrator's own run, a child under it, and a grandchild
        // under that: the chain a depth-2 row joins on.
        write_log(&dir, "root-1", None, None, "ship the slice", 9);
        write_log(
            &dir,
            "child-1",
            Some(crate::session::SUBAGENT_KIND),
            Some("root-1"),
            "land D1 and D6",
            3,
        );
        let shallow = Chain::resolve(&dir, "root-1", 1);
        assert_eq!(shallow.parent, "root-1");
        assert_eq!(shallow.root, "root-1");
        assert_eq!(shallow.lineage, vec!["root-1".to_string()]);
        let deep = Chain::resolve(&dir, "child-1", 2);
        assert_eq!(deep.parent, "child-1");
        assert_eq!(
            deep.root, "root-1",
            "the root is where the chain started, not the middle node"
        );
        assert_eq!(
            deep.lineage,
            vec!["root-1".to_string(), "child-1".to_string()]
        );
        // A log that cannot be read leaves the chain as long as it could
        // be proved rather than as long as somebody assumed.
        let missing = Chain::resolve(&dir, "no-such-session", 2);
        assert_eq!(missing.root, "no-such-session");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_approval_at_depth_two_names_the_whole_chain() {
        let dir = std::env::temp_dir().join(format!("fleet-label-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_log(&dir, "root-1", None, None, "ship the slice", 9);
        write_log(
            &dir,
            "child-1",
            Some(crate::session::SUBAGENT_KIND),
            Some("root-1"),
            "land D1 and D6",
            3,
        );
        let brief = Brief {
            task: "write the row".to_string(),
            ..Brief::default()
        };
        let label = Chain::resolve(&dir, "child-1", 2).label(&brief, &dir);
        // Root first, delimited, bounded: an owner approving a call whose
        // originating intent is two briefs away is approving a
        // paraphrase, and has to be able to see what it paraphrases.
        assert!(label.contains("ship the slice"), "{label}");
        assert!(label.contains("land D1 and D6"), "{label}");
        assert!(label.contains("write the row"), "{label}");
        assert!(label.find("ship the slice").unwrap() < label.find("land D1 and D6").unwrap());
        assert!(label.find("land D1 and D6").unwrap() < label.find("write the row").unwrap());
        // At depth 1 there is nothing between the owner and the child,
        // and the label is the child's own goal.
        let shallow = Chain::resolve(&dir, "root-1", 1);
        assert_eq!(shallow.label(&brief, &dir), brief.label());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_parents_remaining_turns_are_read_off_its_own_log() {
        let dir = std::env::temp_dir().join(format!("fleet-turns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_log(
            &dir,
            "child-1",
            Some(crate::session::SUBAGENT_KIND),
            Some("root-1"),
            "land D1 and D6",
            3,
        );
        // A delegated parent that has spent three of its turns: the
        // depth-stepped default at its own depth, less those.
        assert_eq!(
            parent_remaining_turns(&dir, "child-1", 2),
            Some(turn_cap_for_depth(1) - 3)
        );
        // An ordinary parent's cap is its own run's business.
        write_log(&dir, "root-1", None, None, "ship the slice", 9);
        assert_eq!(parent_remaining_turns(&dir, "root-1", 2), None);
        // A depth-1 child has no delegated parent to read.
        assert_eq!(parent_remaining_turns(&dir, "child-1", 1), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    // --- the delta ----------------------------------------------------------

    #[test]
    fn a_delta_block_parses_each_of_the_four_reason_categories() {
        for reason in [
            DeltaReason::Infeasible,
            DeltaReason::CompanionMissing,
            DeltaReason::BetterRoute,
        ] {
            let text = format!(
                "the child's own words\n--- DELTA ---\nstatus: deviated\nreason: {}\n\
                 companion: the write path\ninstead: read the files\nforced: true\n--- END DELTA ---",
                reason.label()
            );
            let delta = Delta::parse(&text).expect("a deviation with a reason is a delta");
            assert_eq!(delta.state, DeltaState::Deviated);
            assert_eq!(delta.reason, reason);
            assert_eq!(delta.companion, "the write path");
            assert_eq!(delta.instead, "read the files");
            assert!(delta.forced, "a blocker is a forced deviation");
        }
        // A followed brief needs no reason: the default is the one that
        // says so, and free text rides beside it ungraded.
        let followed = Delta::parse("--- DELTA ---\nstatus: followed\n--- END DELTA ---").unwrap();
        assert_eq!(followed.state, DeltaState::Followed);
        assert_eq!(followed.reason, DeltaReason::RightAndFollowed);
        assert!(!followed.forced);
    }

    #[test]
    fn a_malformed_delta_block_is_not_a_delta() {
        // A block whose status is missing or unknown, or a deviation
        // whose reason is, is not a delta: the result says none was
        // reported rather than inventing a state the child never named.
        for text in [
            "no block at all",
            "--- DELTA ---\nstatus: waffled\n--- END DELTA ---",
            "--- DELTA ---\nstatus: deviated\n--- END DELTA ---",
            "--- DELTA ---\nstatus: deviated\nreason: the-vibes-were-off\n--- END DELTA ---",
        ] {
            assert_eq!(Delta::parse(text), None, "{text}");
        }
    }

    /// One child log for the delta's verification: one assistant turn
    /// carrying a tool call and one tool result per output.
    fn log_with(outputs: &[&str]) -> Vec<Record> {
        let mut records = vec![Record::User {
            content: "the goal".to_string(),
        }];
        for (index, output) in outputs.iter().enumerate() {
            records.push(Record::Assistant {
                content: None,
                tool_calls: vec![ToolCall {
                    id: format!("call-{index}"),
                    kind: "function".to_string(),
                    function: FunctionCall {
                        name: "bash".to_string(),
                        arguments: r#"{"command":"ls"}"#.to_string(),
                    },
                }],
            });
            records.push(Record::ToolResult {
                call_id: format!("call-{index}"),
                name: "bash".to_string(),
                output: output.to_string(),
                truncated: false,
                approved: true,
            });
        }
        records
    }

    #[test]
    fn a_deviation_the_log_does_not_show_is_a_false_report() {
        let claimed = Delta::parse(
            "--- DELTA ---\nstatus: deviated\nreason: infeasible\ninstead: read the files\n\
             --- END DELTA ---",
        )
        .unwrap();
        // A deviation on a log that holds no tool call at all: the
        // claimed work is absent from the child's own log.
        let verified = claimed.clone().verify(&log_with(&[]));
        assert!(
            verified
                .contradiction
                .as_deref()
                .is_some_and(|why| why.contains("no tool call")),
            "{verified:?}"
        );
        // An infeasibility claim on a log whose every step succeeded: a
        // blocker nobody hit is a blocker nobody met.
        let verified = claimed
            .clone()
            .verify(&log_with(&["exit 0\n--- stdout ---\nok"]));
        assert!(
            verified
                .contradiction
                .as_deref()
                .is_some_and(|why| why.contains("no failed step")),
            "{verified:?}"
        );
        // The same claim on a log that did hit a blocker verifies.
        let verified = claimed.verify(&log_with(&["exit 2\n--- stderr ---\nno such file"]));
        assert_eq!(verified.contradiction, None, "{verified:?}");
    }

    #[test]
    fn a_companion_missing_deviation_must_name_its_companion() {
        let unnamed = Delta::parse(
            "--- DELTA ---\nstatus: deviated\nreason: companion-missing\ninstead: stopped early\n\
             --- END DELTA ---",
        )
        .unwrap();
        let verified = unnamed.verify(&log_with(&["exit 0"]));
        assert!(
            verified
                .contradiction
                .as_deref()
                .is_some_and(|why| why.contains("names no companion")),
            "{verified:?}"
        );
        // Named, the same deviation verifies: the companion is what the
        // next brief can carry.
        let named = Delta::parse(
            "--- DELTA ---\nstatus: deviated\nreason: companion-missing\n\
             companion: the write path\n--- END DELTA ---",
        )
        .unwrap();
        assert_eq!(named.verify(&log_with(&["exit 0"])).contradiction, None);
    }

    #[test]
    fn a_middle_node_preserves_its_childrens_deltas_verbatim() {
        // The grandchild's delta, as it reported it.
        let grandchild = Delta::parse(
            "--- DELTA ---\nstatus: deviated\nreason: better-route\n\
             instead: read the files instead of running the command\nforced: false\n--- END DELTA ---",
        )
        .unwrap();
        // The middle node's own delta, with its child's riding beside it
        // exactly as `run` reads it off the log.
        let middle = Delta::parse("--- DELTA ---\nstatus: followed\n--- END DELTA ---").unwrap();
        let result = SubagentResult {
            summary: "the middle node's answer".to_string(),
            session: "middle-1".to_string(),
            log: PathBuf::from("/tmp/middle-1.jsonl"),
            turns: 4,
            outcome: Outcome::Completed,
            reason: None,
            goal: String::new(),
            delta: DeltaReport {
                own: Some(middle),
                children: vec![grandchild.clone()],
                offered: true,
            },
        };
        let rendered = result.render();
        // Reading the rendered result back rebuilds the tree rather than
        // a summary of it: the middle node's own delta, and its child's
        // verbatim under it.
        let read_back =
            Delta::parse_render(&rendered).expect("the rendered result carries a delta");
        assert_eq!(read_back.state, DeltaState::Followed);
        assert_eq!(read_back.children.len(), 1);
        assert_eq!(read_back.children[0], grandchild);
        // And the one-per-call read of a log finds the child's own delta
        // the same way, which is how the chain rides up a level.
        let records = vec![Record::ToolResult {
            call_id: "call-1".to_string(),
            name: tools::SUBAGENT_TOOL.to_string(),
            output: rendered,
            truncated: false,
            approved: true,
        }];
        let found = delegated_deltas(&records);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].children[0], grandchild);
    }

    #[test]
    fn the_delta_section_survives_the_bound() {
        let delta = Delta::parse(
            "--- DELTA ---\nstatus: deviated\nreason: companion-missing\n\
             companion: the write path\n--- END DELTA ---",
        )
        .unwrap();
        let result = SubagentResult {
            // A child that wrote a novel.
            summary: "x".repeat(tools::MAX_TOOL_OUTPUT * 2),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/child-1.jsonl"),
            turns: 2,
            outcome: Outcome::Completed,
            reason: None,
            goal: String::new(),
            delta: DeltaReport {
                own: Some(delta),
                children: Vec::new(),
                offered: true,
            },
        };
        let outcome = bound_result(&result);
        assert!(outcome.truncated, "the report half is bounded");
        assert!(
            outcome.output.contains("companion-missing"),
            "the delta rides past the bound rather than being cut by it"
        );
        assert!(
            outcome.output.contains("the write path"),
            "{}",
            outcome.output.len()
        );
        assert!(outcome.output.len() > tools::MAX_TOOL_OUTPUT);
        // A result with no delta is bounded exactly as it always was.
        let plain = SubagentResult {
            delta: DeltaReport::default(),
            goal: String::new(),
            ..result.clone()
        };
        let bounded = bound_result(&plain);
        let expected = tools::bound(ToolOutcome {
            output: plain.render_report(),
            truncated: false,
        });
        assert_eq!(bounded.output, expected.output);
        assert_eq!(bounded.truncated, expected.truncated);
        assert!(!bounded.output.contains("companion-missing"));
    }

    #[test]
    fn a_result_with_no_delta_names_the_absence_when_one_was_asked_for() {
        let offered = SubagentResult {
            summary: "I did the work".to_string(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/child-1.jsonl"),
            turns: 1,
            outcome: Outcome::Completed,
            reason: None,
            goal: String::new(),
            delta: DeltaReport {
                own: None,
                children: Vec::new(),
                offered: true,
            },
        };
        assert!(
            offered.render().contains("no delta reported"),
            "{}",
            offered.render()
        );
        // A brief that offered no approach asks for no delta, and the
        // rendered result is exactly the one this module always sent.
        let plain = SubagentResult {
            delta: DeltaReport::default(),
            goal: String::new(),
            ..offered.clone()
        };
        assert_eq!(plain.render(), "subagent completed \u{b7} 1 turn \u{b7} session child-1 \u{b7} log /tmp/child-1.jsonl\nI did the work");
    }

    // --- the row ------------------------------------------------------------

    #[test]
    fn the_row_writer_lands_a_row_the_reader_reads_back() {
        let root = std::env::temp_dir().join(format!("fleet-row-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let stream = crate::calib::Stream::open(&root);
        let brief = Brief {
            task: "land the row".to_string(),
            approach: Some("append through the stream".to_string()),
            context: Some("the stream is advisory".to_string()),
            output: Some("one row per run".to_string()),
            role: None,
        };
        let scope = Scope::child(Some(vec!["bash".to_string()]), 1);
        let result = SubagentResult {
            summary: "the child counted three files".to_string(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/sessions/child-1.jsonl"),
            turns: 3,
            outcome: Outcome::Completed,
            reason: None,
            goal: String::new(),
            delta: DeltaReport {
                own: None,
                children: Vec::new(),
                offered: true,
            },
        };
        let chain = Chain {
            parent: "root-1".to_string(),
            root: "root-1".to_string(),
            depth: 1,
            lineage: vec!["root-1".to_string()],
        };
        let row = delegation_row(
            &brief,
            &scope.recorded(),
            Some("stepfun"),
            &result,
            &chain,
            1_250,
        );
        // The row's key is the child's own session id — the identity
        // already, so the join needs no hashing and an owner can type it.
        assert_eq!(row.session, "child-1");
        calib::record_delegation(&stream, row);
        let (written, failed) = stream.finish();
        assert_eq!(
            (written, failed),
            (1, 0),
            "the row landed through the stream's own writer"
        );
        // The row reads back off the stream's own file, in the shape the
        // beat's reader joins on: the brief's shape, the scope, the
        // chain, and the outcome — and none of the child's own words.
        let path = calib::day_path(&root, &calib::today());
        let line = std::fs::read_to_string(&path).unwrap();
        let landed: DelegatedRunRow = serde_json::from_str(line.lines().next().unwrap()).unwrap();
        assert_eq!(landed.session, "child-1");
        assert_eq!(landed.brief_shape, "task+approach+context+done");
        assert_eq!(landed.scope, "bash");
        assert_eq!(landed.provider, "stepfun");
        assert_eq!(landed.outcome, "completed");
        assert_eq!(landed.turns, 3);
        assert_eq!(landed.duration_ms, 1_250);
        assert_eq!(landed.parent_session, "root-1");
        assert_eq!(landed.root_session, "root-1");
        assert_eq!(landed.depth, 1);
        assert!(landed.log.ends_with("child-1.jsonl"), "{}", landed.log);
        // The op tag rides the line, so the beat's reader joins it.
        let value: serde_json::Value = serde_json::from_str(line.lines().next().unwrap()).unwrap();
        assert_eq!(value["op"], "delegation");
        // The child's own summary is not a field, by design: the row is
        // the objective evidence, never the child's report.
        assert!(
            !line.contains("the child counted three files"),
            "the summary must not ride the row"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// P6 C2: a call may name a declared plan node instead of carrying a
    /// goal, and the node's identity rides the request so the run and the
    /// plan document cannot drift.
    #[test]
    fn a_call_may_name_a_declared_node_instead_of_a_goal() {
        let with_node = SubagentRequest::parse(&serde_json::json!({
            "node": "build-parser"
        }))
        .expect("a node stands in for a goal");
        assert_eq!(with_node.node.as_deref(), Some("build-parser"));
        assert_eq!(
            with_node.task, "build-parser",
            "the node's id is the task until the node's approach is resolved"
        );

        // A goal still wins when both are given, and the node rides along.
        let both = SubagentRequest::parse(&serde_json::json!({
            "goal": "wire the parser into the CLI",
            "node": "build-parser"
        }))
        .unwrap();
        assert_eq!(both.task, "wire the parser into the CLI");
        assert_eq!(both.node.as_deref(), Some("build-parser"));

        // Neither is a refusal that names both channels.
        let neither = SubagentRequest::parse(&serde_json::json!({})).unwrap_err();
        assert!(
            neither.contains("`goal`") && neither.contains("`node`"),
            "{neither}"
        );
        // An empty node is an absent node, not a goal of "".
        let blank = SubagentRequest::parse(&serde_json::json!({"node": "   "})).unwrap_err();
        assert!(blank.contains("`goal`"), "{blank}");
    }

    /// P6 C3: dispatch selects nodes with a zero dependency score, at
    /// whatever depth the work sits — which is `dispatchable`'s existing
    /// rule, now reachable through a plan.
    #[test]
    fn a_deep_node_dispatches_the_moment_its_chain_verifies() {
        // Four levels of declared work: the leaf at work-depth 4 is
        // unreachable until the whole chain above it lands, and then it is
        // immediately dispatchable — the executor's own depth cap has nothing
        // to do with it, because the graph carries the work structure and the
        // session tree carries only who spawned whom.
        let mut graph = crate::graph::Graph::new();
        graph.add(crate::graph::Node::new("a", 1)).unwrap();
        graph
            .add(crate::graph::Node::new("b", 1).with_prereqs(["a"]))
            .unwrap();
        graph
            .add(crate::graph::Node::new("c", 1).with_prereqs(["b"]))
            .unwrap();
        graph
            .add(crate::graph::Node::new("d", 1).with_prereqs(["c"]))
            .unwrap();

        // Only `a` is unlocked to begin with: everything else has unfinished
        // work in its closure.
        assert_eq!(graph.dispatchable(3, &[]), vec!["a".to_string()]);

        // Land the chain and the deep leaf becomes dispatchable in the same
        // pass — it never waited for a wave, only for its dependencies.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let gate = std::sync::Arc::clone(&seen);
        graph.set_verifier(Box::new(move |node: &crate::graph::Node| {
            gate.lock().unwrap().contains(&node.id)
        }));
        for id in ["a", "b", "c"] {
            seen.lock().unwrap().insert(id.to_string());
        }
        graph.settle();
        assert_eq!(
            graph.dispatchable(3, &[]),
            vec!["d".to_string()],
            "the leaf at work-depth 4 dispatches the moment its chain verifies"
        );
        assert_eq!(
            graph.time_to_completion("d"),
            Some(0),
            "and its closure is done"
        );
    }
}
