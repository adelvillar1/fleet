//! The dependency graph: declared nodes with prerequisites and cost
//! estimates, the monotone dependency score whose zero is an exact
//! unlock signal, mandatory resource edges, and the critical path.
//!
//! Orchestration ran in waves: a batch was dispatched, the orchestrator
//! waited for all of it, then acted. A unit whose prerequisites cleared
//! at minute five waited for a sibling that needed fifty. The root
//! cause is that the orchestrator gated on **reports, not facts** — a
//! child's summary is a claim, and the commit landing is the fact. This
//! module is the replacement: a graph declared before dispatch whose
//! nodes start the moment their prerequisites' artifacts are verified,
//! not when a sibling's report arrives.
//!
//! ## The score
//!
//! ```text
//! score(n) = Σ over n's unfinished dependencies d of (cost(d) + score(d))
//! ```
//!
//! The total remaining work in a node's dependency closure, computed
//! from the declared estimates and recursed. Four properties hold it
//! together:
//!
//! - **Monotone.** The score is derived on every read, never stored: a
//!   verified completion removes a term from every dependent's sum and
//!   nothing ever adds one, so monotonicity is structural rather than
//!   maintained. A cost revision is the orchestrator correcting its own
//!   estimate — it reshapes the path to zero and never gates it.
//! - **Zero is exact.** Zero is reached only when every dependency's
//!   artifact is verified: the arithmetic counts unfinished work, and
//!   an estimate is not work. A wildly wrong estimate misorders the
//!   graph; it cannot unlock a node whose dependency never landed.
//! - **Facts gate; reports corroborate.** The completion predicate is
//!   the parent's — D2's judge — and the default is conservative: with
//!   no predicate nothing verifies and nothing unlocks.
//! - **A shared dependency is carried once per path.** In a diamond the
//!   base's cost enters the head's score through both branches, which
//!   is the formula's own arithmetic and the reason a shared
//!   dependency's completion drops a diamond's score by its cost times
//!   the paths that reach it. In a tree — the common case — the drop is
//!   exactly the completed node's cost.
//!
//! ## Unlock is three conditions
//!
//! Score zero, dependencies verified, AND resource-free — all three
//! necessary and separately reported by [`Unlock`]: a node fully
//! unblocked by its dependencies can still be held by a sibling on its
//! path, and "waiting on work" and "waiting on a lock" call for
//! different actions from the owner. The third condition sits beside
//! the score because it differs from it only when a declared cost is
//! zero or a prerequisite is unresolved — both named — which is why it
//! is checked beside the score rather than instead of it.
//!
//! ## The safety net
//!
//! The resource edge is mandatory, not advisory. Two nodes naming one
//! admitted path never run concurrently: [`Graph::dispatch`] refuses the
//! second by name and [`Graph::dispatchable`] never offers it. A missing
//! dependency edge costs serialization; an unmodeled resource edge costs
//! corruption (KI-172/173/177, three separate occurrences of exactly
//! that), so the graph errs toward edges and the constraint refuses
//! rather than reorders.
//!
//! ## What rides the node
//!
//! Each node carries its capability declaration (D9 — the declaration
//! IS the scope, and an undeclared tool is refused by name) and its
//! brief's approach (D1/D10 — a delta needs two sides), so the graph the
//! orchestrator declares is the graph the calibration and the replan
//! read. D10's resequencing lands here as a revision: new edges through
//! [`Graph::add_prerequisite`], corrected costs through
//! [`Graph::revise_cost`], and the append-only revision record itself is
//! the parent's to keep.

use std::fmt;

use crate::skills::wrap_detail;

// --- the node --------------------------------------------------------------

/// Where one node stands.
///
/// Transitions go through the graph — [`Graph::dispatch`],
/// [`Graph::settle`], [`Graph::fail`] — never by hand, because the score
/// is derived from these states and a hand-set one is a score nobody can
/// reconstruct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Declared, not yet dispatched.
    Pending,
    /// Dispatched and in flight.
    Running,
    /// Its artifact is verified — the only state that subtracts.
    Verified,
    /// Its run ended without a verified artifact. Unfinished for the
    /// score's purposes: its dependents stay blocked and the reason
    /// names it, because a dependency that failed never lands on its
    /// own.
    Failed,
}

/// A node stands where it was declared until the graph moves it, so
/// the default is the declared state — which is also what a file that
/// omits a node's status reads as.
impl Default for Status {
    fn default() -> Self {
        Status::Pending
    }
}

/// The capability declaration D9 asks for at decomposition: what this
/// subtree is likely to need. The declaration IS the scope — a declared
/// tool is admitted and an undeclared one is refused by name — so a
/// declaration that sits beside a hand-written list is decoration, and
/// decoration is worse than nothing because it looks like a guarantee.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    /// The tools, from the admissible set.
    pub tools: Vec<String>,
    /// The skills, by name.
    pub skills: Vec<String>,
    /// The workflows, named. An unavailable one renders as unavailable
    /// rather than vanishing — the parent's registry answers that.
    pub workflows: Vec<String>,
    /// The recurring processes the subtree triggers or depends on: the
    /// judge batch, the consolidation cycle, a cron registration.
    pub loops: Vec<String>,
}

impl Capabilities {
    /// Whether the declaration says nothing.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
            && self.skills.is_empty()
            && self.workflows.is_empty()
            && self.loops.is_empty()
    }

    /// The declaration as one phrase for a card or a brief's context:
    /// `tools: bash, read_file · skills: vendoring-lanes`. An empty list
    /// is omitted rather than rendered empty.
    pub fn phrase(&self) -> String {
        let lists = [
            ("tools", &self.tools),
            ("skills", &self.skills),
            ("workflows", &self.workflows),
            ("loops", &self.loops),
        ];
        lists
            .iter()
            .filter(|(_, names)| !names.is_empty())
            .map(|(label, names)| format!("{label}: {}", names.join(", ")))
            .collect::<Vec<_>>()
            .join(" \u{b7} ")
    }
}

/// One unit of work in the declared graph.
///
/// The graph is declared before dispatch, so a node is a declaration:
/// its identity, what must verify before it can start, what it is
/// estimated to cost, which paths it admits, what its subtree is likely
/// to need, and the approach its brief carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The node's identity: what the orchestrator, the log, and the
    /// render name it by.
    pub id: String,
    /// The identities that must verify before this node can start. A
    /// node with none is ready immediately.
    pub prereqs: Vec<String>,
    /// The estimated cost, in the orchestrator's own unit (turns). It
    /// shapes the ordering, the critical path, and the readiness bar;
    /// it never gates anything, because the zero is computed from
    /// verified completions rather than from this.
    pub cost: u64,
    /// The admitted paths. Two nodes naming one path never run
    /// concurrently — the mandatory resource edge, D4's one-writer
    /// discipline generalized from tool calls to delegation nodes.
    pub paths: Vec<String>,
    /// The capability declaration (D9).
    pub capabilities: Capabilities,
    /// The brief's approach (D1/D10): the orchestrator's proposed
    /// solution for this subtree, stated as a proposal so a child that
    /// finds a better route is deviating from a suggestion rather than
    /// disobeying an order.
    pub approach: String,
    /// The delta this node's run reported: the angle between the
    /// approach it was handed and the one it executed, with the reason
    /// category and — on a deviation that the child's own log
    /// contradicted — the named contradiction. `None` until the
    /// orchestrator lands one with [`Graph::record_delta`], because a
    /// delta is a report a run made, never something the declaration
    /// could know.
    pub delta: Option<crate::subagents::Delta>,
    /// Where the node stands. Transitions go through the graph.
    pub status: Status,
    /// The highest score this node's closure has carried: the readiness
    /// bar's denominator. Scores only fall, so the peak is the score at
    /// the graph's widest — a node's distance to its own minima.
    peak: u64,
}

impl Node {
    /// A node with no prerequisites, no admitted paths, no declaration,
    /// and no approach: ready the moment the graph is declared.
    pub fn new(id: impl Into<String>, cost: u64) -> Self {
        Self {
            id: id.into(),
            prereqs: Vec::new(),
            cost,
            paths: Vec::new(),
            capabilities: Capabilities::default(),
            approach: String::new(),
            delta: None,
            status: Status::Pending,
            peak: 0,
        }
    }

    /// The declared prerequisites.
    pub fn with_prereqs<I, S>(mut self, prereqs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.prereqs = prereqs.into_iter().map(Into::into).collect();
        self
    }

    /// The admitted paths, the resource edge's own declaration.
    pub fn with_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// The capability declaration.
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// The brief's approach.
    pub fn with_approach(mut self, approach: impl Into<String>) -> Self {
        self.approach = approach.into();
        self
    }

    /// The readiness bar's denominator: the highest score this node's
    /// closure has carried.
    pub fn peak(&self) -> u64 {
        self.peak
    }
}

// --- the graph -------------------------------------------------------------

/// A sibling already holding one of a node's admitted paths: the
/// resource conflict, named, so the render and the log can say which of
/// the two conditions is holding a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHold {
    /// The running node holding the path.
    pub holder: String,
    /// The path it holds.
    pub path: String,
}

/// The three conditions, reported separately and all necessary.
///
/// Collapsing them into one "blocked" state would hide the difference
/// between waiting on work, waiting on a missing prerequisite, and
/// waiting on a lock, and those call for different actions from the
/// owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unlock {
    /// The dependency score reached zero: no unfinished work stands in
    /// the closure.
    pub score_zero: bool,
    /// No running node holds one of this node's admitted paths.
    pub resource_free: bool,
    /// Every declared prerequisite is a node in the graph whose
    /// artifact is verified. It differs from `score_zero` only when a
    /// declared cost is zero or a prerequisite is unresolved — both
    /// named below — which is why it is checked beside the score rather
    /// than instead of it.
    pub deps_verified: bool,
    /// Prerequisites that are neither verified nor failed.
    pub waiting: Vec<String>,
    /// Prerequisites whose run ended without a verified artifact.
    pub failed: Vec<String>,
    /// Prerequisites that are not nodes in the graph.
    pub unresolved: Vec<String>,
    /// The siblings holding this node's paths, named.
    pub held: Vec<ResourceHold>,
}

impl Default for Unlock {
    /// The empty state: nothing verified, so nothing unlocked, and
    /// nothing held.
    fn default() -> Self {
        Self {
            score_zero: false,
            resource_free: true,
            deps_verified: false,
            waiting: Vec::new(),
            failed: Vec::new(),
            unresolved: Vec::new(),
            held: Vec::new(),
        }
    }
}

impl Unlock {
    /// Whether the node can start: score zero, dependencies verified,
    /// and resource-free.
    pub fn is_unlocked(&self) -> bool {
        self.score_zero && self.resource_free && self.deps_verified
    }

    /// The named reason this node is held, or `None` when it is not. A
    /// reason is always a name — a dependency, a failed one, a missing
    /// one, or a sibling and its path — never a bare "blocked".
    pub fn reason(&self) -> Option<String> {
        if self.is_unlocked() {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();
        if !self.waiting.is_empty() {
            parts.push(format!("waiting on {}", self.waiting.join(", ")));
        }
        if !self.failed.is_empty() {
            parts.push(format!("{} failed", self.failed.join(", ")));
        }
        if !self.unresolved.is_empty() {
            parts.push(format!("{} not in the graph", self.unresolved.join(", ")));
        }
        for hold in &self.held {
            parts.push(format!("{} holds {}", hold.holder, hold.path));
        }
        if parts.is_empty() {
            // Unreachable through the graph's own arithmetic — a held
            // node always names its holder — but a reason that reads
            // empty is the failure this module exists to prevent.
            parts.push("held".to_string());
        }
        Some(parts.join(" \u{b7} "))
    }
}

/// The graph's refusals, each naming what it refused. A graph that
/// would carry a cycle, a duplicate id, or a concurrent same-path run is
/// refused by name rather than silently reordered: the resource
/// constraint is the safety net, and a safety net that fails quietly is
/// not one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// A node id declared twice.
    DuplicateNode(String),
    /// A prerequisite edge that would close a cycle.
    Cycle { node: String, prereq: String },
    /// A node that is not in the graph.
    UnknownNode(String),
    /// A dispatch the resource edge refuses: a running node already
    /// holds the path.
    PathHeld { path: String, holder: String },
    /// A failure report against a node whose artifact is verified.
    AlreadyVerified(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::DuplicateNode(id) => {
                write!(f, "node `{id}` is already in the graph")
            }
            GraphError::Cycle { node, prereq } => {
                write!(
                    f,
                    "`{prereq}` already depends on `{node}`: the edge would close a cycle"
                )
            }
            GraphError::UnknownNode(id) => write!(f, "node `{id}` is not in the graph"),
            GraphError::PathHeld { path, holder } => {
                write!(
                    f,
                    "`{holder}` is running and holds `{path}`: the two cannot run together"
                )
            }
            GraphError::AlreadyVerified(id) => {
                write!(
                    f,
                    "`{id}`'s artifact is verified: a report does not demote a fact"
                )
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// The declared graph, and the predicate that decides which of its
/// nodes has actually landed.
pub struct Graph {
    nodes: Vec<Node>,
    /// The replans this run has landed, oldest first. Append-only: a
    /// replan revises the tree and records what it did, and nothing
    /// ever removes a revision, because the tree's history is what
    /// makes a later reader able to check the current model against
    /// the evidence that shaped it.
    revisions: Vec<Revision>,
    /// The fact-gate. `None` is the conservative default: nothing
    /// verifies, nothing unlocks, and every score sits where it was
    /// declared.
    verify: Option<Box<dyn Fn(&Node) -> bool>>,
}

impl fmt::Debug for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The predicate is the parent's closure and carries no
        // reconstructable text, so the debug shape names whether one is
        // set rather than trying to print it.
        f.debug_struct("Graph")
            .field("nodes", &self.nodes)
            .field("gated", &self.verify.is_some())
            .finish()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// An empty graph with no fact-gate: the conservative default.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            revisions: Vec::new(),
            verify: None,
        }
    }

    /// A graph whose completions are decided by `verify` — the parent's
    /// predicate over a node's artifact, which is D2's judge. The
    /// predicate is the authority: a node it confirms is verified
    /// whatever its run reported, and a node it refuses stays
    /// unfinished however confident the report was.
    pub fn with_verifier<F>(verify: F) -> Self
    where
        F: Fn(&Node) -> bool + 'static,
    {
        Self {
            nodes: Vec::new(),
            revisions: Vec::new(),
            verify: Some(Box::new(verify)),
        }
    }

    /// Declare a node. The graph is declared before dispatch, so a node
    /// may name prerequisites that are not in the graph yet — a forward
    /// reference scores zero until they land, and [`Graph::unresolved`]
    /// names it. A duplicate id and a cycle are refused by name.
    pub fn add(&mut self, node: Node) -> Result<(), GraphError> {
        if self.index_of(&node.id).is_some() {
            return Err(GraphError::DuplicateNode(node.id));
        }
        for prereq in &node.prereqs {
            if prereq == &node.id {
                return Err(GraphError::Cycle {
                    node: node.id.clone(),
                    prereq: prereq.clone(),
                });
            }
            if let Some(dep) = self.index_of(prereq) {
                let mut stack = Vec::new();
                if self.reaches(dep, &node.id, &mut stack) {
                    return Err(GraphError::Cycle {
                        node: node.id.clone(),
                        prereq: prereq.clone(),
                    });
                }
            }
        }
        self.nodes.push(node);
        self.resync();
        Ok(())
    }

    /// Add an edge to a declared graph — D10's resequencing, when a
    /// completion surfaces a dependency nobody declared. Refuses a cycle
    /// by name; a repeated edge is idempotent.
    pub fn add_prerequisite(&mut self, id: &str, prereq: &str) -> Result<(), GraphError> {
        let index = self
            .index_of(id)
            .ok_or_else(|| GraphError::UnknownNode(id.to_string()))?;
        if prereq == id {
            return Err(GraphError::Cycle {
                node: id.to_string(),
                prereq: prereq.to_string(),
            });
        }
        if let Some(dep) = self.index_of(prereq) {
            let mut stack = Vec::new();
            if self.reaches(dep, id, &mut stack) {
                return Err(GraphError::Cycle {
                    node: id.to_string(),
                    prereq: prereq.to_string(),
                });
            }
        }
        if !self.nodes[index].prereqs.iter().any(|edge| edge == prereq) {
            self.nodes[index].prereqs.push(prereq.to_string());
            self.resync();
        }
        Ok(())
    }

    /// Correct a node's cost estimate — D2's calibration as actual costs
    /// accumulate, so a shape whose briefs have historically cost N
    /// turns is estimated at N. The estimate shapes the ordering, the
    /// critical path, and the readiness bar; it never gates anything, so
    /// a revision can raise a score without ever unlocking a node.
    pub fn revise_cost(&mut self, id: &str, cost: u64) -> Result<(), GraphError> {
        let index = self
            .index_of(id)
            .ok_or_else(|| GraphError::UnknownNode(id.to_string()))?;
        self.nodes[index].cost = cost;
        self.resync();
        Ok(())
    }

    /// Record that a node started. The resource edge is enforced here:
    /// a running node already holding one of this node's admitted paths
    /// is a refusal naming the path and its holder, never a silent
    /// reordering.
    pub fn dispatch(&mut self, id: &str) -> Result<(), GraphError> {
        let index = self
            .index_of(id)
            .ok_or_else(|| GraphError::UnknownNode(id.to_string()))?;
        if let Some(hold) = self.path_held(index, &[]) {
            return Err(GraphError::PathHeld {
                path: hold.path,
                holder: hold.holder,
            });
        }
        self.nodes[index].status = Status::Running;
        Ok(())
    }

    /// Record that a node's run ended without its artifact. The node
    /// stays unfinished for the score's purposes — its dependents stay
    /// blocked and the reason names it — and a later verified artifact
    /// still promotes it, because a fact outranks a report. Refused by
    /// name on an already-verified node: nothing demotes a fact.
    pub fn fail(&mut self, id: &str) -> Result<(), GraphError> {
        let index = self
            .index_of(id)
            .ok_or_else(|| GraphError::UnknownNode(id.to_string()))?;
        if self.nodes[index].status == Status::Verified {
            return Err(GraphError::AlreadyVerified(id.to_string()));
        }
        self.nodes[index].status = Status::Failed;
        Ok(())
    }

    /// Run the fact-gate over every node that has not verified and
    /// promote the ones whose artifact the predicate confirms. Returns
    /// the newly verified ids in declaration order — the list the caller
    /// logs, because a score that moved is a model-visible event (hard
    /// rule 5).
    ///
    /// A node marked failed is still offered to the predicate: the
    /// artifact is the authority, so work that landed after a failed
    /// report verifies on the next settle rather than staying buried.
    pub fn settle(&mut self) -> Vec<String> {
        let Some(verify) = self.verify.as_ref() else {
            return Vec::new();
        };
        let mut landed = Vec::new();
        for index in 0..self.nodes.len() {
            if self.nodes[index].status == Status::Verified {
                continue;
            }
            if verify(&self.nodes[index]) {
                self.nodes[index].status = Status::Verified;
                landed.push(self.nodes[index].id.clone());
            }
        }
        landed
    }

    /// A node, by identity.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.index_of(id).map(|index| &self.nodes[index])
    }

    /// Every declared node, in declaration order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// How many nodes the graph carries.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph carries no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// A node's dependency score: the total remaining work in its
    /// dependency closure.
    ///
    /// ```text
    /// score(n) = Σ over n's unfinished dependencies d of (cost(d) + score(d))
    /// ```
    ///
    /// A verified dependency contributes nothing — its whole closure is
    /// done work — and a dependency that failed keeps contributing,
    /// because its artifact never lands. Monotone by construction: the
    /// sum only loses terms.
    pub fn score(&self, id: &str) -> Option<u64> {
        let index = self.index_of(id)?;
        let mut cache = vec![None; self.nodes.len()];
        let mut stack = Vec::new();
        Some(self.score_at(index, &mut cache, &mut stack))
    }

    /// Every node's score, in declaration order — the snapshot a caller
    /// diffs across a settle to record what moved.
    pub fn scores(&self) -> Vec<(String, u64)> {
        let mut cache = vec![None; self.nodes.len()];
        let mut stack = Vec::new();
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    node.id.clone(),
                    self.score_at(index, &mut cache, &mut stack),
                )
            })
            .collect()
    }

    /// The two conditions for one node, reported separately.
    pub fn unlock(&self, id: &str) -> Option<Unlock> {
        let index = self.index_of(id)?;
        let mut cache = vec![None; self.nodes.len()];
        let mut stack = Vec::new();
        // Start verified: the loop below is what takes it away, so a node
        // with no prerequisites reads verified rather than defaulting to
        // unverified.
        let mut unlock = Unlock {
            deps_verified: true,
            ..Unlock::default()
        };
        unlock.score_zero = self.score_at(index, &mut cache, &mut stack) == 0;
        for prereq in &self.nodes[index].prereqs {
            match self.index_of(prereq) {
                None => {
                    unlock.deps_verified = false;
                    unlock.unresolved.push(prereq.clone());
                }
                Some(dep) => match self.nodes[dep].status {
                    Status::Verified => {}
                    Status::Failed => {
                        unlock.deps_verified = false;
                        unlock.failed.push(prereq.clone());
                    }
                    _ => {
                        unlock.deps_verified = false;
                        unlock.waiting.push(prereq.clone());
                    }
                },
            }
        }
        for hold in self.holds_on(index) {
            unlock.resource_free = false;
            unlock.held.push(hold);
        }
        Some(unlock)
    }

    /// Whether one node can start right now.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlock(id).is_some_and(|unlock| unlock.is_unlocked())
    }

    /// The nodes whose two conditions both hold and that have not
    /// started, in declaration order. The graph's own view:
    /// [`Graph::dispatchable`] applies the concurrency bound and the
    /// caller's record of what is in flight.
    pub fn ready(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| node.status == Status::Pending)
            .filter(|node| self.is_unlocked(&node.id))
            .map(|node| node.id.clone())
            .collect()
    }

    /// The nodes that can start right now: unlocked, not already
    /// running, and serialized by name against the paths the running set
    /// holds. Ordered so the node that blocks the most work goes first,
    /// then the larger cost, then the lowest id — deterministic — and
    /// capped at the room the bound has left.
    ///
    /// `running` is the caller's own record of what is in flight. The
    /// graph's statuses and that list are the same set in honest use,
    /// and the resource edge is checked against both because it is
    /// mandatory rather than advisory.
    pub fn dispatchable(&self, bound: usize, running: &[String]) -> Vec<String> {
        let slots = bound.saturating_sub(running.len());
        if slots == 0 {
            return Vec::new();
        }
        let mut candidates: Vec<(usize, u64, String)> = self
            .nodes
            .iter()
            .filter(|node| node.status == Status::Pending)
            .filter(|node| !running.iter().any(|id| id == &node.id))
            .filter(|node| self.is_unlocked(&node.id))
            .map(|node| (self.appears_in_scores(&node.id), node.cost, node.id.clone()))
            .collect();
        // The node that appears in the most scores first: finishing it
        // drops the most scores soonest, which is the scheduling
        // decision the score answers directly.
        candidates.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then(right.1.cmp(&left.1))
                .then(left.2.cmp(&right.2))
        });
        // Serialize by name: two nodes on one path never both go, even
        // when the bound has room for both.
        let mut taken: Vec<String> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        for (_, _, id) in candidates {
            if out.len() >= slots {
                break;
            }
            let Some(index) = self.index_of(&id) else {
                continue;
            };
            if self.path_held(index, running).is_some() {
                continue;
            }
            if self.nodes[index]
                .paths
                .iter()
                .any(|path| taken.contains(path))
            {
                continue;
            }
            taken.extend(self.nodes[index].paths.iter().cloned());
            out.push(id);
        }
        out
    }

    /// The critical path: the longest chain of remaining work, ordered
    /// from the most-blocked node down to the deepest unfinished
    /// dependency. Empty when nothing is unfinished — there is no long
    /// pole to name.
    ///
    /// The head is the highest score, which is the longest chain; each
    /// link is the unfinished dependency whose own closure carries the
    /// most work. Ties break to the lowest id, so the path is
    /// deterministic.
    pub fn critical_path(&self) -> Vec<String> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let mut cache = vec![None; self.nodes.len()];
        let mut stack = Vec::new();
        let mut head: Option<usize> = None;
        for index in 0..self.nodes.len() {
            let score = self.score_at(index, &mut cache, &mut stack);
            let better = match head {
                None => true,
                Some(current) => {
                    let standing = self.score_at(current, &mut cache, &mut stack);
                    score > standing
                        || (score == standing && self.nodes[index].id < self.nodes[current].id)
                }
            };
            if better {
                head = Some(index);
            }
        }
        let Some(head) = head else {
            return Vec::new();
        };
        if self.score_at(head, &mut cache, &mut stack) == 0 {
            return Vec::new();
        }
        let mut path = vec![self.nodes[head].id.clone()];
        let mut current = head;
        loop {
            let mut next: Option<(u64, usize)> = None;
            for prereq in &self.nodes[current].prereqs {
                let Some(dep) = self.index_of(prereq) else {
                    continue;
                };
                if self.nodes[dep].status == Status::Verified {
                    continue;
                }
                if path.iter().any(|seen| seen == &self.nodes[dep].id) {
                    continue;
                }
                let weight = self.nodes[dep].cost + self.score_at(dep, &mut cache, &mut stack);
                let better = match next {
                    None => true,
                    Some((standing, chosen)) => {
                        weight > standing
                            || (weight == standing && self.nodes[dep].id < self.nodes[chosen].id)
                    }
                };
                if better {
                    next = Some((weight, dep));
                }
            }
            match next {
                Some((_, dep)) => {
                    path.push(self.nodes[dep].id.clone());
                    current = dep;
                }
                None => break,
            }
        }
        path
    }

    /// The node whose completion moves the most work: the unfinished
    /// node that appears in the most scores. Ties break to the larger
    /// cost, then to the lowest id, so the answer is deterministic.
    pub fn blocks_most_work(&self) -> Option<String> {
        let mut best: Option<(usize, usize)> = None;
        for index in 0..self.nodes.len() {
            if self.nodes[index].status == Status::Verified {
                continue;
            }
            let count = self.appears_in_count(index);
            let better = match best {
                None => true,
                Some((current, standing)) => {
                    count > standing
                        || (count == standing
                            && (self.nodes[index].cost > self.nodes[current].cost
                                || (self.nodes[index].cost == self.nodes[current].cost
                                    && self.nodes[index].id < self.nodes[current].id)))
                }
            };
            if better {
                best = Some((index, count));
            }
        }
        best.map(|(index, _)| self.nodes[index].id.clone())
    }

    /// How many nodes' scores carry this node's cost — the "appears in
    /// the most scores" measure. Finishing the node with the highest
    /// count drops the most scores soonest.
    pub fn appears_in_scores(&self, id: &str) -> usize {
        let Some(target) = self.index_of(id) else {
            return 0;
        };
        self.appears_in_count(target)
    }

    /// The nodes that declare this one as a direct prerequisite.
    pub fn dependents(&self, id: &str) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| node.prereqs.iter().any(|prereq| prereq == id))
            .map(|node| node.id.clone())
            .collect()
    }

    /// Every (node, prerequisite) pair whose prerequisite is not a node
    /// in the graph. A forward declaration is legal and scores zero
    /// until it lands; an unresolved one at dispatch time is a named
    /// defect, never a silent zero.
    pub fn unresolved(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for node in &self.nodes {
            for prereq in &node.prereqs {
                if self.index_of(prereq).is_none() {
                    out.push((node.id.clone(), prereq.clone()));
                }
            }
        }
        out
    }

    // --- the arithmetic ----------------------------------------------------

    /// A node's position in the declaration order.
    fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }

    /// The score, memoized per call. The visiting stack is a defensive
    /// guard rather than a reachable path — cycles are refused at
    /// declaration — and it returns 0 rather than hanging.
    fn score_at(&self, index: usize, cache: &mut Vec<Option<u64>>, stack: &mut Vec<usize>) -> u64 {
        if let Some(cached) = cache[index] {
            return cached;
        }
        if stack.contains(&index) {
            return 0;
        }
        stack.push(index);
        let mut total: u64 = 0;
        for prereq in &self.nodes[index].prereqs {
            // An unresolved prerequisite carries no declared cost; it is
            // named by `unresolved` and holds its dependent through the
            // verification condition rather than through the arithmetic.
            let Some(dep) = self.index_of(prereq) else {
                continue;
            };
            if self.nodes[dep].status == Status::Verified {
                continue;
            }
            total = total.saturating_add(self.nodes[dep].cost);
            total = total.saturating_add(self.score_at(dep, cache, stack));
        }
        stack.pop();
        cache[index] = Some(total);
        total
    }

    /// Recompute every node's score and raise its peak to the highest it
    /// has carried. Called whenever the graph's shape or an estimate
    /// changes: a node added before its prerequisites exist reads zero
    /// until they land, and the peak must not remember that.
    fn resync(&mut self) {
        let mut cache = vec![None; self.nodes.len()];
        let mut stack = Vec::new();
        for index in 0..self.nodes.len() {
            let score = self.score_at(index, &mut cache, &mut stack);
            self.nodes[index].peak = self.nodes[index].peak.max(score);
        }
    }

    /// Whether `target`'s cost stands in `from`'s score: a chain of
    /// unfinished declarations leads from `from` to `target`.
    fn appears_in(&self, from: usize, target: usize, stack: &mut Vec<usize>) -> bool {
        if stack.contains(&from) {
            return false;
        }
        stack.push(from);
        for prereq in &self.nodes[from].prereqs {
            let Some(dep) = self.index_of(prereq) else {
                continue;
            };
            if self.nodes[dep].status == Status::Verified {
                continue;
            }
            if dep == target || self.appears_in(dep, target, stack) {
                stack.pop();
                return true;
            }
        }
        stack.pop();
        false
    }

    /// How many nodes' scores carry `target`'s cost. A verified node's
    /// score is finished work, so it is not counted.
    fn appears_in_count(&self, target: usize) -> usize {
        let mut count = 0;
        for other in 0..self.nodes.len() {
            if other == target || self.nodes[other].status == Status::Verified {
                continue;
            }
            let mut stack = Vec::new();
            if self.appears_in(other, target, &mut stack) {
                count += 1;
            }
        }
        count
    }

    /// Whether `from`'s declaration chain reaches `target` — the cycle
    /// check's question: does the prerequisite already depend on the node
    /// the new edge would point at?
    fn reaches(&self, from: usize, target: &str, stack: &mut Vec<usize>) -> bool {
        if stack.contains(&from) {
            return false;
        }
        stack.push(from);
        for prereq in &self.nodes[from].prereqs {
            if prereq == target {
                stack.pop();
                return true;
            }
            if let Some(dep) = self.index_of(prereq) {
                if self.reaches(dep, target, stack) {
                    stack.pop();
                    return true;
                }
            }
        }
        stack.pop();
        false
    }

    /// The running nodes holding one of `index`'s admitted paths. The
    /// graph's own record and the caller's list are both consulted,
    /// because the constraint is mandatory rather than advisory.
    fn holds_on(&self, index: usize) -> Vec<ResourceHold> {
        let mut out: Vec<ResourceHold> = Vec::new();
        for path in &self.nodes[index].paths {
            for other in 0..self.nodes.len() {
                if other == index || self.nodes[other].status != Status::Running {
                    continue;
                }
                if self.nodes[other].paths.iter().any(|held| held == path) {
                    out.push(ResourceHold {
                        holder: self.nodes[other].id.clone(),
                        path: path.clone(),
                    });
                }
            }
        }
        out
    }

    /// Whether a running node — the graph's own record or the caller's
    /// list — already holds one of `index`'s admitted paths.
    fn path_held(&self, index: usize, running: &[String]) -> Option<ResourceHold> {
        for path in &self.nodes[index].paths {
            for other in 0..self.nodes.len() {
                if other == index {
                    continue;
                }
                let in_flight = self.nodes[other].status == Status::Running
                    || running.iter().any(|id| id == &self.nodes[other].id);
                if !in_flight {
                    continue;
                }
                if self.nodes[other].paths.iter().any(|held| held == path) {
                    return Some(ResourceHold {
                        holder: self.nodes[other].id.clone(),
                        path: path.clone(),
                    });
                }
            }
        }
        None
    }
}

// --- the surface -----------------------------------------------------------

/// The card width, the house width `calib`, `eval`, `judge`, and the
/// rest of the card family draw at.
const CARD_WIDTH: usize = 72;

/// How many cells the readiness bar draws.
const BAR_CELLS: usize = 10;

/// The room a card's title gets. A node id the operator never bounded
/// costs its tail, never the card's width.
const TITLE_CAP: usize = 24;

/// The room a card's right edge gets, so the header's dash run never
/// collapses to its one-dash floor and pushes the line past the house
/// width.
const RIGHT_CAP: usize = 38;

/// The widest remaining-work number a line draws before it folds to a
/// bounded marker.
const SCORE_CEILING: u64 = 99999;

/// How much of a brief's approach the card shows. The full text rides
/// the brief; the card is a surface, not a dump.
const APPROACH_CHARS: usize = 200;

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

use palette::{ACCENT, BOLD, DIM, ERROR, OK, RESET, TEXT, WARN};

/// The English plural for a count, the way the sibling surfaces carry
/// theirs.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// Truncate `text` to at most `cap` characters with a marker, never
/// mid-word silently. The marker counts against `cap`, so the result is
/// never longer than `cap` — a caller that sizes a line around
/// `bound(text, cap)` can rely on `cap` being the whole width.
fn bound(text: &str, cap: usize) -> String {
    const MARKER: &str = " …[truncated]";
    if text.chars().count() <= cap {
        return text.to_string();
    }
    let keep = cap.saturating_sub(MARKER.chars().count());
    let kept: String = text.chars().take(keep).collect();
    format!("{kept}{MARKER}")
}

/// The drawn width of a card line: the characters a terminal shows, not
/// the bytes the escapes cost.
fn plain_width(text: &str) -> usize {
    let mut width = 0;
    let mut escape = false;
    for character in text.chars() {
        match character {
            '\u{1b}' => escape = true,
            'm' if escape => escape = false,
            _ if escape => {}
            _ => width += 1,
        }
    }
    width
}

/// The dash run that pads a card header so its right edge aligns. One
/// character tighter than the sibling surfaces' arithmetic, so the
/// header lands exactly on the house width and nothing clips at 80
/// columns. The caller bounds the title and the right edge, so the
/// one-dash floor is never reached.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 5).max(1))
}

/// A card in the house shape — `┌─┐ │ └─┘`, the ANSI palette, the width
/// the sibling surfaces draw at — so the graph reads as one more Fleet
/// surface rather than a debug dump. `title` and `right` are plain; the
/// card bounds them and colours them, so the header's arithmetic runs on
/// drawn widths only.
fn card(title: &str, right: &str, right_color: &str, body: Vec<String>, footer: &str) -> String {
    let title = bound(title, TITLE_CAP);
    let right = bound(right, RIGHT_CAP);
    let dashes = dashes_for(&title, &right);
    let mut out = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{title}{RESET} {DIM}{dashes} \
         {right_color}{right}{RESET} {DIM}\u{2510}{RESET}\n"
    );
    for line in body {
        out.push_str(&format!("{DIM}\u{2502}{RESET} {line}\n"));
    }
    out.push_str(&format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}\n",
        "\u{2500}".repeat(CARD_WIDTH - 2)
    ));
    // The footer is a caption, not a card line: it wraps to the house
    // width so the whole card reads inside 80 columns.
    for line in wrap_detail(footer, CARD_WIDTH) {
        out.push_str(&format!("{DIM}{line}{RESET}\n"));
    }
    out
}

/// The remaining-work number as a word: the raw score, folded to a
/// bounded marker past the ceiling so a line's arithmetic never depends
/// on a number's width.
fn score_word(score: u64) -> String {
    if score > SCORE_CEILING {
        format!("{SCORE_CEILING}+")
    } else {
        score.to_string()
    }
}

/// The readiness bar: the score's distance from this node's own peak,
/// drawn in `BAR_CELLS` cells. A node whose closure never carried work
/// — no prerequisites — draws full, because nothing ever stood between
/// it and unlocked.
///
/// Rounded to the nearest cell, so the bar moves on the completion that
/// moves it rather than sitting still for several.
fn readiness_bar(score: u64, peak: u64, color: &str) -> String {
    let filled = if peak == 0 {
        BAR_CELLS
    } else {
        let done = peak.saturating_sub(score.min(peak));
        let scaled = done as u128 * BAR_CELLS as u128 + peak as u128 / 2;
        (scaled / peak as u128) as usize
    };
    let filled = filled.min(BAR_CELLS);
    format!(
        "{color}[{}{}]{RESET}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(BAR_CELLS - filled)
    )
}

/// One node's status as a dot, a word, and a colour. Verified is the
/// only state that subtracts; a node that is unblocked but held reads
/// `held` rather than `waiting`, because those call for different
/// actions from the owner.
fn node_state(node: &Node, unlock: &Unlock) -> (&'static str, &'static str, &'static str) {
    match node.status {
        Status::Verified => ("\u{2713}", "verified", OK),
        Status::Failed => ("\u{2717}", "failed", ERROR),
        Status::Running => ("\u{25cf}", "running", ACCENT),
        Status::Pending if unlock.is_unlocked() => ("\u{25cf}", "ready", OK),
        Status::Pending if !unlock.resource_free => ("\u{25cf}", "held", WARN),
        Status::Pending => ("\u{25cb}", "waiting", DIM),
    }
}

// --- the replan -------------------------------------------------------------

/// What one delta costs the aggregate.
///
/// The weights encode the plan's own semantics: a followed brief and a
/// chosen improvement cost nothing — the last is a gain, and an
/// orchestrator that priced improvement as a defect would teach its
/// children to stop reporting honestly, which destroys the only signal
/// the part runs on. An infeasible approach costs most, because the
/// brief drove away; a missing companion costs less, because the
/// direction was right and something was absent. A **forced**
/// deviation weighs double: a blocker the child actually hit is a
/// stronger statement about the brief than one it chose around.
///
/// The one weight that is not the category's is a **contradicted**
/// report: a claimed deviation the child's own session log does not
/// show is a false report, which is the most serious kind, and it is
/// priced as such whatever the category claimed.
pub fn diff_weight(delta: &crate::subagents::Delta) -> u64 {
    if delta.contradiction.is_some() {
        return CONTRADICTED_WEIGHT;
    }
    let base = match delta.reason {
        crate::subagents::DeltaReason::RightAndFollowed
        | crate::subagents::DeltaReason::BetterRoute => 0,
        crate::subagents::DeltaReason::CompanionMissing => COMPANION_WEIGHT,
        crate::subagents::DeltaReason::Infeasible => INFEASIBLE_WEIGHT,
    };
    if delta.forced {
        base * 2
    } else {
        base
    }
}

/// What a chosen deviation around a missing companion costs.
const COMPANION_WEIGHT: u64 = 2;

/// What a chosen deviation around an infeasible approach costs — the
/// largest category weight, because "the brief drove away" is the
/// statement the score exists to make.
const INFEASIBLE_WEIGHT: u64 = 3;

/// What a false report costs: a claimed deviation the child's own log
/// contradicts. Priced above every category so one lies about the work
/// moves the aggregate on its own.
const CONTRADICTED_WEIGHT: u64 = 5;

/// The aggregate trigger: a tree whose diff score reaches this has a
/// plan that was probably not optimal, and the response is to act on
/// the current tree rather than only learn for the next one.
pub const DIFF_THRESHOLD: u64 = 4;

/// The per-node trigger: a single deviation this large replans on its
/// own, because one node that far off its brief is a fact about the
/// plan even when the rest of the tree is clean.
pub const NODE_DIFF_THRESHOLD: u64 = 5;

/// How many revisions one run may land. The bound is what keeps the
/// replan from thrashing: an orchestrator that replanned on every
/// deviation would never finish anything, and a tree that changes
/// under its own children is worse than a tree that was wrong once.
pub const REPLAN_CAP: usize = 2;

/// One replan, recorded append-only.
///
/// The revision is what makes the tree's history reconstructable
/// rather than overwritten: the graph a replan produces is the current
/// best model, and the revisions say how it got there and why — the
/// score that triggered it, the node whose deviation did, and every
/// change the replan made in words a later reader can check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Revision {
    /// The revision's sequence number, one-based: the first replan is
    /// revision 1.
    pub seq: usize,
    /// Why the replan happened, named: the aggregate score and the
    /// node whose deviation triggered it, or the aggregate alone.
    pub cause: String,
    /// What the replan changed, one line per change.
    pub changes: Vec<String>,
}

impl Graph {
    /// Land one run's delta on its node.
    ///
    /// The delta rides the node rather than a side table because the
    /// score is derived from the nodes on every read — the same
    /// discipline the dependency score follows — and a report that
    /// lived anywhere else could not be part of the arithmetic. A node
    /// the graph does not carry is a named error: a delta for work
    /// nobody declared is a delta nobody can act on.
    pub fn record_delta(
        &mut self,
        id: &str,
        delta: crate::subagents::Delta,
    ) -> Result<(), GraphError> {
        let index = self
            .index_of(id)
            .ok_or_else(|| GraphError::UnknownNode(id.to_string()))?;
        self.nodes[index].delta = Some(delta);
        Ok(())
    }

    /// The aggregate diff score: the sum of every landed delta's
    /// weight, plus the largest single node's weight as the per-node
    /// trigger's input.
    ///
    /// Monotone by construction — a delta lands once and nothing
    /// removes one — so the score only rises as the tree reports, and
    /// the render can show it climbing toward the threshold without
    /// the reader wondering whether it could fall back.
    pub fn diff_score(&self) -> u64 {
        self.nodes
            .iter()
            .filter_map(|node| node.delta.as_ref())
            .map(diff_weight)
            .sum()
    }

    /// The largest weight any single landed delta carries: the
    /// per-node trigger's own input. Zero when nothing has reported.
    pub fn worst_delta_weight(&self) -> u64 {
        self.nodes
            .iter()
            .filter_map(|node| node.delta.as_ref())
            .map(diff_weight)
            .max()
            .unwrap_or(0)
    }

    /// Whether either trigger has fired: the aggregate at
    /// [`DIFF_THRESHOLD`], or one node's deviation at
    /// [`NODE_DIFF_THRESHOLD`].
    pub fn should_replan(&self) -> bool {
        self.diff_score() >= DIFF_THRESHOLD || self.worst_delta_weight() >= NODE_DIFF_THRESHOLD
    }

    /// The revisions this run has landed, oldest first.
    pub fn revisions(&self) -> &[Revision] {
        &self.revisions
    }

    /// Replan the tree against its deltas, and record what changed.
    ///
    /// The action is the plan's first one: **revise the remaining
    /// nodes' approaches.** A node whose own run reported infeasible
    /// gets its approach replaced with what the child did instead —
    /// the one fact the tree now holds about a route that works — and
    /// the nodes still waiting behind it carry the revision forward,
    /// because a brief written against an approach its own tree just
    /// falsified is a brief that will falsify again. A chosen
    /// improvement revises nothing: it is a gain, and moving a node
    /// because its child found a better route would punish exactly
    /// the honesty the part runs on.
    ///
    /// `None` when nothing fired, when the revisions cap is reached,
    /// or when no remaining node carries a falsified approach — the
    /// three ways a replan can be a no-op, each of them a fact rather
    /// than a failure. The cause names which trigger fired and which
    /// node's deviation carried it.
    pub fn replan(&mut self) -> Option<Revision> {
        if !self.should_replan() || self.revisions.len() >= REPLAN_CAP {
            return None;
        }
        // The falsified approaches, in declaration order: the nodes
        // whose own runs reported infeasible and said what they did
        // instead.
        let falsified: Vec<(String, String)> = self
            .nodes
            .iter()
            .filter_map(|node| match &node.delta {
                Some(delta)
                    if delta.reason == crate::subagents::DeltaReason::Infeasible
                        && !delta.instead.trim().is_empty() =>
                {
                    Some((node.id.clone(), delta.instead.clone()))
                }
                _ => None,
            })
            .collect();
        if falsified.is_empty() {
            return None;
        }
        let mut changes = Vec::new();
        for (id, instead) in &falsified {
            let index = self.index_of(id).expect("a falsified node is in the graph");
            self.nodes[index].approach = instead.clone();
            changes.push(format!(
                "`{id}`'s approach revised to what its own run did instead"
            ));
        }
        // The nodes still waiting behind a revised one carry the
        // revision forward: their briefs were written against an
        // approach the tree has now falsified.
        let revised: Vec<String> = falsified.iter().map(|(id, _)| id.clone()).collect();
        for node in &mut self.nodes {
            if node.status != Status::Pending || revised.iter().any(|id| id == &node.id) {
                continue;
            }
            if node.prereqs.iter().any(|prereq| revised.contains(prereq)) {
                changes.push(format!(
                    "`{}`'s brief now descends from the revised approach{}",
                    node.id,
                    revised
                        .iter()
                        .filter(|id| node.prereqs.contains(id))
                        .map(|id| format!(" of `{id}`"))
                        .collect::<Vec<_>>()
                        .join("")
                ));
            }
        }
        let worst = self.worst_node();
        let cause = format!(
            "diff score {} at the threshold {DIFF_THRESHOLD}{}",
            self.diff_score(),
            worst
                .map(|(id, weight)| format!("; the largest single deviation is `{id}` at {weight}"))
                .unwrap_or_default()
        );
        let revision = Revision {
            seq: self.revisions.len() + 1,
            cause,
            changes,
        };
        self.revisions.push(revision.clone());
        Some(revision)
    }

    /// The node whose landed delta weighs most, as `(id, weight)` —
    /// the per-node trigger's own named input. `None` when nothing
    /// has reported, or every report weighs nothing.
    fn worst_node(&self) -> Option<(String, u64)> {
        self.nodes
            .iter()
            .filter_map(|node| {
                node.delta
                    .as_ref()
                    .map(|delta| (node.id.clone(), diff_weight(delta)))
            })
            .max_by_key(|(_, weight)| *weight)
            .filter(|(_, weight)| *weight > 0)
    }
}

// --- the file ---------------------------------------------------------------

/// The declared graph as it persists under the home: the declaration
/// and the states, and nothing else.
///
/// What is deliberately absent is the fact-gate. A verifier is the
/// parent's own closure over a node's artifact — D2's judge — and a
/// closure is not a value that can be written down, so a loaded graph
/// carries the conservative default until its caller attaches the real
/// predicate. That is the honest shape: the file is the declaration,
/// the gate rides the run that owns it, and a graph read back off disk
/// unlocks nothing its parent has not re-declared.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphFile {
    /// The nodes, in declaration order.
    pub nodes: Vec<NodeFile>,
    /// The replans the tree has landed, oldest first. Append-only on
    /// disk exactly as it is in memory: the file is rewritten with the
    /// revisions it carried plus the new one, and a reader can
    /// reconstruct how the current model was shaped.
    #[serde(default)]
    pub revisions: Vec<Revision>,
}

/// One node as it persists: every declared field, and the state it
/// stood in when the graph was written.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeFile {
    pub id: String,
    pub cost: u64,
    #[serde(default)]
    pub prereqs: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub approach: String,
    /// The delta this node's run reported, when one has landed. It
    /// rides the file because the diff score is derived from the
    /// nodes: a graph that lost its deltas on save would read back as
    /// a tree that never deviated, which is the one reading the
    /// surface must never produce by accident.
    #[serde(default)]
    pub delta: Option<crate::subagents::Delta>,
    #[serde(default)]
    pub status: Status,
}

/// Where a home's declared graph lives: `<home>/graph.json`, beside the
/// sessions and the roles.
pub fn path_in(home: &std::path::Path) -> std::path::PathBuf {
    home.join("graph.json")
}

impl Graph {
    /// Write this graph to `path` as the home's declared graph.
    ///
    /// The parent directory is created: a first save into a fresh home
    /// is the ordinary case, and refusing it would make the surface's
    /// first act a failure.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let file = GraphFile {
            nodes: self
                .nodes
                .iter()
                .map(|node| NodeFile {
                    id: node.id.clone(),
                    cost: node.cost,
                    prereqs: node.prereqs.clone(),
                    paths: node.paths.clone(),
                    capabilities: node.capabilities.clone(),
                    approach: node.approach.clone(),
                    delta: node.delta.clone(),
                    status: node.status,
                })
                .collect(),
            revisions: self.revisions.clone(),
        };
        let text = serde_json::to_string_pretty(&file)
            .map_err(|error| format!("the graph would not serialize: {error}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(path, format!("{text}\n"))
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Read a declared graph back from `path`.
    ///
    /// Every failure is named rather than defaulted: an absent file, an
    /// unreadable one, and a malformed one are three different facts,
    /// and an empty graph silently substituted for any of them would
    /// render as "nothing is blocking anything" — the one reading the
    /// surface must never produce by accident. The loaded graph carries
    /// no fact-gate (see [`GraphFile`]); the caller attaches its own.
    pub fn load(path: &std::path::Path) -> Result<Graph, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let file: GraphFile = serde_json::from_str(&text)
            .map_err(|error| format!("{} is not a declared graph: {error}", path.display()))?;
        let mut graph = Graph::new();
        for node in file.nodes {
            let id = node.id.clone();
            let delta = node.delta.clone();
            graph
                .add(
                    Node::new(node.id, node.cost)
                        .with_prereqs(node.prereqs)
                        .with_paths(node.paths)
                        .with_capabilities(node.capabilities)
                        .with_approach(node.approach),
                )
                .map_err(|error| error.to_string())?;
            // The state rides the file. A load that reset every node to
            // pending would render verified work as blocking work —
            // the one reading the surface would see the graph standing
            // still when it had moved. The file is the recorded state,
            // so the load restores it rather than re-declaring it.
            if let Some(added) = graph.nodes.iter_mut().find(|added| added.id == id) {
                added.status = node.status;
                // And so does the delta: the diff score is derived from
                // the nodes, so a graph that lost its deltas on save
                // would read back as a tree that never deviated.
                added.delta = delta;
            }
        }
        // The revisions ride the file append-only, exactly as they are
        // held: the tree's history is what lets a later reader check
        // the current model against the evidence that shaped it.
        graph.revisions = file.revisions;
        Ok(graph)
    }
}

/// The graph as cards at the house bar: a summary card naming the
/// critical path and what blocks the most work, then one card per node
/// with its status dot, its score as a readiness bar beside the raw
/// remaining-work number, its edges named, and what is holding it.
/// Never a plain-text table.
pub fn render(graph: &Graph) -> String {
    let path = graph.critical_path();
    let mut out = summary_card(graph, &path);
    for index in 0..graph.nodes.len() {
        out.push_str(&node_card(graph, index, &path));
    }
    out
}

/// The summary card: the counts, the critical path named, the node that
/// blocks the most work, and what is ready or running.
fn summary_card(graph: &Graph, path: &[String]) -> String {
    let verified = graph
        .nodes
        .iter()
        .filter(|node| node.status == Status::Verified)
        .count();
    let running = graph
        .nodes
        .iter()
        .filter(|node| node.status == Status::Running)
        .count();
    let failed = graph
        .nodes
        .iter()
        .filter(|node| node.status == Status::Failed)
        .count();
    let mut right = format!("{} nodes", graph.nodes.len());
    if verified > 0 {
        right.push_str(&format!(" \u{b7} {verified} verified"));
    }
    if running > 0 {
        right.push_str(&format!(" \u{b7} {running} running"));
    }
    if failed > 0 {
        right.push_str(&format!(" \u{b7} {failed} failed"));
    }
    let mut body: Vec<String> = Vec::new();
    let unresolved = graph.unresolved();
    if path.is_empty() {
        if unresolved.is_empty() {
            body.push(format!(
                "{DIM}nothing stands between any node and unlocked{RESET}"
            ));
        }
    } else {
        let head = path.first().cloned().unwrap_or_default();
        let standing = graph.score(&head).unwrap_or(0);
        body.extend(summary_line(
            "\u{25c6}",
            ACCENT,
            "critical path",
            &format!(
                "{} \u{b7} {} left",
                path.join(" \u{2192} "),
                score_word(standing)
            ),
        ));
    }
    if !unresolved.is_empty() {
        // A declared edge whose other end is not in the graph: named, so
        // the surface never reads as though the graph were complete.
        let named: Vec<String> = unresolved
            .iter()
            .map(|(node, prereq)| format!("{node} needs {prereq}"))
            .collect();
        body.extend(summary_line(
            "\u{b7}",
            WARN,
            "unresolved",
            &named.join(", "),
        ));
    }
    if let Some(blocker) = graph.blocks_most_work() {
        let count = graph.appears_in_scores(&blocker);
        body.extend(summary_line(
            "\u{25c6}",
            ACCENT,
            "blocks the most",
            &format!("{blocker} \u{b7} appears in {count} score{}", plural(count)),
        ));
    }
    // The diff score: the aggregate over the tree's landed deltas, with
    // the threshold it climbs toward. The owner sees the plan being
    // corrected rather than silently mutated — a replan lands as a
    // revision, and the score is what moved.
    let score = graph.diff_score();
    if score > 0 || !graph.revisions.is_empty() {
        let at = score >= DIFF_THRESHOLD;
        let color = if at { WARN } else { DIM };
        let mut line = format!("{score} of the {DIFF_THRESHOLD} replan threshold",);
        if at {
            line.push_str(" \u{b7} the replan has fired");
        }
        body.extend(summary_line("\u{25c6}", color, "diff score", &line));
        for revision in &graph.revisions {
            body.extend(summary_line(
                "\u{b7}",
                DIM,
                &format!("revision {}", revision.seq),
                &format!("{} \u{b7} {}", revision.cause, revision.changes.join("; ")),
            ));
        }
    }
    let ready = graph.ready();
    let running_ids: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| node.status == Status::Running)
        .map(|node| node.id.clone())
        .collect();
    if ready.is_empty() && running_ids.is_empty() {
        body.push(format!(
            "{DIM}nothing is ready to dispatch and nothing is running{RESET}"
        ));
    } else {
        if !ready.is_empty() {
            body.extend(summary_line("\u{b7}", DIM, "ready now", &ready.join(", ")));
        }
        if !running_ids.is_empty() {
            body.extend(summary_line(
                "\u{b7}",
                DIM,
                "running",
                &running_ids.join(", "),
            ));
        }
    }
    card(
        "graph",
        &right,
        TEXT,
        body,
        "the score is a distance to the minima: it falls only on a verified completion, so the zero is exact",
    )
}

/// One summary line: a marker, a label, and a value wrapped to the room
/// the label leaves. The label rides the first line and the value's tail
/// wraps under it, so a long path or a long ready-list never pushes the
/// card past the house width.
fn summary_line(marker: &str, color: &str, label: &str, value: &str) -> Vec<String> {
    let head = marker.chars().count() + 1 + label.chars().count() + 1;
    let room = (CARD_WIDTH - 2).saturating_sub(head).max(16);
    wrap_detail(value, room)
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("{color}{marker}{RESET} {BOLD}{label}{RESET} {TEXT}{line}{RESET}")
            } else {
                format!("{DIM}\u{b7}{RESET} {TEXT}{line}{RESET}")
            }
        })
        .collect()
}

/// One node's card: the status dot and the state word on the header's
/// right edge, the readiness bar beside the raw remaining-work number,
/// the edges named, what is holding it, and the declaration and approach
/// the node was decomposed with.
fn node_card(graph: &Graph, index: usize, path: &[String]) -> String {
    let node = &graph.nodes[index];
    let unlock = graph.unlock(&node.id).unwrap_or_default();
    let score = graph.score(&node.id).unwrap_or(0);
    let (dot, word, color) = node_state(node, &unlock);
    let on_path = path.iter().any(|id| id == &node.id);
    let title = if on_path {
        format!("\u{25c6} {}", node.id)
    } else {
        node.id.clone()
    };
    let mut body: Vec<String> = Vec::new();
    body.push(readiness_line(
        node,
        score,
        color,
        graph.appears_in_scores(&node.id),
    ));
    body.extend(edge_lines(graph, node));
    for hold in &unlock.held {
        body.push(format!(
            "{WARN}held by {holder} on {path}{RESET}",
            holder = hold.holder,
            path = hold.path
        ));
    }
    if node.status == Status::Failed {
        let waiting = graph.dependents(&node.id).len();
        body.push(format!(
            "{ERROR}failed \u{b7} {waiting} node{} named it as a prerequisite{RESET}",
            plural(waiting)
        ));
    }
    if node.status == Status::Verified {
        body.push(format!(
            "{OK}verified \u{b7} cost {cost} subtracted from every dependent's score{RESET}",
            cost = node.cost
        ));
    }
    if !node.paths.is_empty() {
        body.extend(rail_lines(&format!("paths: {}", node.paths.join(", "))));
    }
    if !node.capabilities.is_empty() {
        body.extend(rail_lines(&format!(
            "declares {}",
            node.capabilities.phrase()
        )));
    }
    if !node.approach.is_empty() {
        body.extend(rail_lines(&format!(
            "approach: {}",
            bound(&node.approach, APPROACH_CHARS)
        )));
    }
    // The delta this node's run reported: the reason category, whether
    // the deviation was forced, and — on a report the child's own log
    // contradicted — the named contradiction, which is the most
    // serious kind and reads as one.
    if let Some(delta) = &node.delta {
        let mut text = format!("delta: {}", delta.reason.label());
        if delta.state == crate::subagents::DeltaState::Deviated {
            text.push_str(if delta.forced {
                " (forced)"
            } else {
                " (chosen)"
            });
        }
        if let Some(contradiction) = &delta.contradiction {
            text.push_str(&format!(" \u{b7} CONTRADICTED: {contradiction}"));
        }
        body.extend(rail_lines_colored(
            &text,
            if delta.contradiction.is_some() {
                ERROR
            } else {
                TEXT
            },
        ));
    }
    card(&title, &format!("{dot} {word}"), color, body, "")
}

/// The readiness line: the score as a bar beside the raw remaining-work
/// number. The bar is the distance from this node's own peak — its
/// distance to the minima — and the number is the score itself, so the
/// estimate's shape stays visible and the exact value is never hidden
/// behind it.
fn readiness_line(node: &Node, score: u64, color: &str, appears: usize) -> String {
    let bar = readiness_bar(score, node.peak, color);
    let mut line = format!(
        "{bar} {TEXT}{number:>5} left{RESET}",
        number = score_word(score)
    );
    // Both clauses are bounded by construction; the width check is the
    // belt over the braces, so a line that would reach the house width
    // drops its last clause rather than passing it (the KI-179 class).
    if node.peak > score {
        line = with_clause(&line, &format!("{} at its widest", score_word(node.peak)));
    }
    if appears > 0 {
        line = with_clause(
            &line,
            &format!("appears in {appears} score{}", plural(appears)),
        );
    }
    line
}

/// Append one `· clause` to a readiness line when it still fits the room
/// a body line has. The escapes are the terminal's, not the layout's, so
/// the measurement is the drawn width.
fn with_clause(line: &str, clause: &str) -> String {
    let drawn = plain_width(line) + 3 + clause.chars().count();
    if drawn <= CARD_WIDTH - 2 {
        format!("{line} {DIM}\u{b7} {clause}{RESET}")
    } else {
        line.to_string()
    }
}

/// The edges, named: every declared prerequisite with what stands behind
/// it — the work left, `verified`, `failed`, or `not in the graph`. An
/// edge the graph cannot resolve is named, never dropped.
fn edge_lines(graph: &Graph, node: &Node) -> Vec<String> {
    if node.prereqs.is_empty() {
        return vec![format!("{DIM}\u{b7}{RESET} {TEXT}needs nothing{RESET}")];
    }
    let mut stalled = false;
    let named: Vec<String> = node
        .prereqs
        .iter()
        .map(|prereq| match graph.node(prereq) {
            None => format!("{prereq} (not in the graph)"),
            Some(dep) => match dep.status {
                Status::Verified => format!("{prereq} (verified)"),
                Status::Failed => {
                    stalled = true;
                    format!("{prereq} (failed)")
                }
                _ => format!(
                    "{prereq} ({} left)",
                    score_word(graph.score(prereq).unwrap_or(0))
                ),
            },
        })
        .collect();
    let text = format!("needs {}", named.join(", "));
    let color = if stalled { WARN } else { TEXT };
    rail_lines_colored(&text, color)
}

/// A detail line under a card's headline: the `·` rail, the text wrapped
/// to the room the rail and the frame leave.
fn rail_lines(text: &str) -> Vec<String> {
    rail_lines_colored(text, TEXT)
}

/// A detail line in a named colour — amber when a prerequisite failed,
/// because the reason a node will never unlock should not read like
/// ordinary waiting.
fn rail_lines_colored(text: &str, color: &str) -> Vec<String> {
    wrap_detail(text, CARD_WIDTH - 8)
        .into_iter()
        .map(|line| format!("{DIM}\u{b7}{RESET} {color}{line}{RESET}"))
        .collect()
}

#[cfg(test)]
mod tests;
