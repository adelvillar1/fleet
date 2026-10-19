//! The plan-tied graph: one graph per plan document, generated only from
//! an approved plan.
//!
//! `docs/plans/README.md` already owns the shape this builds on. Plans
//! are contracts, one file per body of work, with acceptance criteria
//! that are "verifiable, specific," and "the plan file is the source of
//! truth for 'is X done'." What it lacked was an execution record. That
//! is what this module is.
//!
//! Three pieces, in one place because they are one idea:
//!
//! - **The declaration.** A fenced JSON block in the plan document
//!   declares nodes — id, cost, prerequisites, paths, capabilities,
//!   approach, and the acceptance criterion each node serves. The prose
//!   stays the contract for humans; the block is the contract for the
//!   harness. One file, so they cannot drift.
//! - **The runtime file.** A sibling beside the plan carries the graph's
//!   state — status, deltas, revisions — plus a digest of the
//!   declaration it was built from. Gitignored the way
//!   `memories/.recall-index.json` is a cache beside its corpus: the plan
//!   is git-tracked and human-edited, and its execution state must not
//!   dirty the tree on every transition.
//! - **The roll-up.** A node is verified when the criterion it serves is
//!   met, and the plan's own status rule already says `active →
//!   completed` when every criterion is confirmed met. So the plan's
//!   acceptance-criteria table is the progress surface, read straight off
//!   the graph.
//!
//! **Only an approved plan generates a graph.** `draft` generates
//! nothing. `active` generates one, as a pure function of the approved
//! artifact. `completed` and `cancelled` leave it terminal and retained —
//! the repo's own rule ("never reopen a completed plan") makes
//! permanence the natural behaviour, which is the traceability
//! requirement. A plan that does not conform to the documented format has
//! no graph, and the refusal names why rather than guessing.
//!
//! **Drift is the normal case, not an edge case.** The repo's rule is
//! "update the plan's acceptance criteria first, then build," so a plan
//! edited after its graph was generated is expected. The runtime file
//! carries the digest it was built from and a mismatch is named with both
//! versions — never silently mismapped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::graph::{Capabilities, DeclaredNode, Graph, Node, Status};

/// The four statuses the plan format declares, and the only ones a plan
/// may carry. A value outside the set is a malformed plan, not a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Draft,
    Active,
    Completed,
    Cancelled,
}

impl PlanStatus {
    /// The word the frontmatter carries.
    pub fn as_str(self) -> &'static str {
        match self {
            PlanStatus::Draft => "draft",
            PlanStatus::Active => "active",
            PlanStatus::Completed => "completed",
            PlanStatus::Cancelled => "cancelled",
        }
    }

    fn parse(text: &str) -> Option<PlanStatus> {
        match text.trim() {
            "draft" => Some(PlanStatus::Draft),
            "active" => Some(PlanStatus::Active),
            "completed" => Some(PlanStatus::Completed),
            "cancelled" => Some(PlanStatus::Cancelled),
            _ => None,
        }
    }

    /// Whether this status may carry a generated graph. Only an approved
    /// plan executes, so only an approved plan has a graph to generate.
    pub fn generates_a_graph(self) -> bool {
        matches!(self, PlanStatus::Active)
    }
}

/// The plan document's frontmatter: the identity and the status the
/// graph's lifecycle hangs off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanMeta {
    pub status: PlanStatus,
    pub created: String,
    pub updated: String,
    pub slug: String,
}

/// The declaration block, as the plan states it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Declaration {
    pub nodes: Vec<DeclaredNode>,
}

/// The runtime file beside a plan: the graph's own persistence plus the
/// two things that tie it to the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGraphFile {
    /// The plan's slug, so a stray runtime file can be matched back to
    /// its document.
    pub slug: String,
    /// The digest of the declaration this graph was generated from. A
    /// mismatch against the plan as it now stands is drift, and it is
    /// named rather than silently mismapped.
    pub declaration: String,
    /// Which criterion each node serves.
    #[serde(default)]
    pub serves: HashMap<String, String>,
    /// The graph's own file: nodes, deltas, statuses, revisions.
    #[serde(flatten)]
    pub graph: crate::graph::GraphFile,
}

/// P7 C2: whether the graph's cross-plan edges close a cycle across
/// plans.
///
/// A cycle through another plan is invisible from either side alone: plan
/// A's node waits on plan B's, and plan B's waits back on A's. Neither
/// graph contains the edge that closes the loop, so each one's own cycle
/// check — which walks only its own nodes — passes. Two plans blocking on
/// each other deadlock with nothing to notice, which is the nastiest
/// failure this design has.
///
/// The check builds the combined adjacency — this graph's edges and every
/// registered plan's, each namespaced by `<slug>/<node>` so two plans'
/// node ids cannot collide — and walks it for a cycle. `None` when the
/// combined graph is acyclic; the cycle's path when it is not, so the
/// refusal can name it.
pub fn cross_plan_cycle(graph: &Graph, registry: &PlanRegistry) -> Option<Vec<String>> {
    let mut edges: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    // This graph's own edges, under its own slug.
    for node in graph.nodes() {
        let here = format!("{}/{}", registry.self_slug, node.id);
        edges.entry(here.clone()).or_default();
        for prereq in &node.prereqs {
            let named = match cross_plan_edge(prereq) {
                // A cross-plan edge already names its plan.
                Some(edge) => format!("{}/{}", edge.slug, edge.node),
                None => format!("{}/{prereq}", registry.self_slug),
            };
            edges.entry(here.clone()).or_default().push(named);
        }
    }
    // Every registered plan's edges, under that plan's slug. The registry
    // carries them precisely so a cycle is visible from either side.
    for (slug, entry) in &registry.plans {
        if slug == &registry.self_slug {
            continue;
        }
        for (node, prereq) in &entry.edges {
            let here = format!("{slug}/{node}");
            let named = match cross_plan_edge(prereq) {
                Some(edge) => format!("{}/{}", edge.slug, edge.node),
                None => format!("{slug}/{prereq}"),
            };
            edges.entry(here).or_default().push(named);
        }
    }
    let mut state: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    let mut path: Vec<String> = Vec::new();
    let mut starts: Vec<String> = edges.keys().cloned().collect();
    starts.sort();
    for start in starts {
        if let Some(cycle) = walk_for_cycle(&edges, &start, &mut state, &mut path) {
            return Some(cycle);
        }
    }
    None
}

/// The depth-first cycle walk: `state` is 0 unvisited, 1 on the stack, 2
/// done. Returns the cycle's path when it closes one.
fn walk_for_cycle(
    edges: &std::collections::HashMap<String, Vec<String>>,
    at: &str,
    state: &mut std::collections::HashMap<String, u8>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    match state.get(at).copied().unwrap_or(0) {
        2 => return None,
        1 => {
            // On the stack: the cycle is everything from `at` onward.
            let from = path.iter().position(|seen| seen == at)?;
            let mut cycle = path[from..].to_vec();
            cycle.push(at.to_string());
            return Some(cycle);
        }
        _ => {}
    }
    state.insert(at.to_string(), 1);
    path.push(at.to_string());
    let mut next: Vec<String> = edges.get(at).cloned().unwrap_or_default();
    next.sort();
    for edge in next {
        if let Some(cycle) = walk_for_cycle(edges, &edge, state, path) {
            return Some(cycle);
        }
    }
    path.pop();
    state.insert(at.to_string(), 2);
    None
}

/// P7 C3: the cross-plan prerequisites that can never clear, named.
///
/// A dependency on a plan that does not exist, on one whose status does
/// not generate a graph, or on a node that plan does not declare is not
/// ordinary waiting — it will never resolve. Left in `unresolved` it
/// reads as a typo and the dependent blocks forever with nothing to say
/// why, which is the failure this names.
pub fn unresolvable_edges(graph: &Graph, registry: &PlanRegistry) -> Vec<String> {
    let mut found = Vec::new();
    for node in graph.nodes() {
        for prereq in &node.prereqs {
            let Some(edge) = cross_plan_edge(prereq) else {
                continue;
            };
            match registry.resolve(&edge) {
                CrossPlanResolution::Verified | CrossPlanResolution::Waiting(_) => {}
                other => found.push(format!(
                    "`{}` needs `{}` in the `{}` plan: {}",
                    node.id,
                    edge.node,
                    edge.slug,
                    match other {
                        CrossPlanResolution::NoSuchPlan => "no such plan holds a graph",
                        CrossPlanResolution::NotApproved(status) => {
                            let _ = status;
                            return_unapproved()
                        }
                        CrossPlanResolution::NoSuchNode => "the plan does not declare it",
                        _ => "unresolvable",
                    }
                )),
            }
        }
    }
    found
}

/// The words a not-approved status carries in a named refusal.
fn return_unapproved() -> &'static str {
    "the plan's status does not generate a graph, so its work can never verify"
}

/// One plan's graph: the generated graph, the digest it was built from,
/// and the node-to-criterion join.
///
/// Not `Clone`: the graph carries a boxed verifier closure, which is not
/// a value that can be copied — the same reason `Graph` is not `Clone`.
#[derive(Debug)]
pub struct PlanGraph {
    pub slug: String,
    pub declaration: String,
    pub serves: HashMap<String, String>,
    pub graph: Graph,
}

/// Why a plan could not produce a graph. Every refusal names the plan and
/// the reason, because a plan that silently has no graph is worse than
/// one that says why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The file does not read as a plan document at all.
    NotAPlan(String),
    /// The frontmatter is missing a field the format declares.
    MissingField(String, String),
    /// A frontmatter value is outside its vocabulary.
    BadStatus(String, String),
    /// The plan's status does not generate a graph.
    NotApproved(String, String),
    /// The declaration block is absent, malformed, or would not build.
    Declaration(String),
    /// The graph itself refused the declaration.
    Graph(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NotAPlan(path) => write!(f, "{path} is not a plan document"),
            PlanError::MissingField(path, field) => {
                write!(f, "{path}: the frontmatter is missing `{field}`")
            }
            PlanError::BadStatus(path, value) => write!(
                f,
                "{path}: `{value}` is not a plan status (draft, active, completed, cancelled)"
            ),
            PlanError::NotApproved(path, status) => write!(
                f,
                "{path}: a `{status}` plan does not generate a graph — only an approved one does"
            ),
            PlanError::Declaration(reason) => write!(f, "{reason}"),
            PlanError::Graph(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for PlanError {}

/// The runtime file beside a plan: the plan's own path with `.md`
/// swapped for `.graph.json`. Keyed to the file, not the slug, so two
/// plans sharing a slug on different dates never collide.
pub fn runtime_path_for(plan: &Path) -> PathBuf {
    let mut path = plan.to_path_buf();
    path.set_extension("graph.json");
    path
}

/// The plan document a runtime file belongs to — the inverse of
/// [`runtime_path_for`].
///
/// Not `with_extension("md")`: the runtime name is `<plan>.graph.json`,
/// and replacing the last extension would give `<plan>.graph.md`. The
/// `.graph.json` suffix is stripped whole and `.md` put in its place.
pub fn plan_path_for(runtime: &Path) -> PathBuf {
    let name = match runtime.file_name().and_then(|name| name.to_str()) {
        Some(name) => name
            .strip_suffix(".graph.json")
            .map(str::to_string)
            .unwrap_or_else(|| name.to_string()),
        None => return runtime.to_path_buf(),
    };
    runtime.with_file_name(format!("{name}.md"))
}

/// Read a plan's frontmatter. The format is the README's: a `---` fence,
/// `key: value` lines, a closing `---` fence. Hand-rolled because the
/// crate carries no YAML dependency and needs exactly these four fields.
pub fn read_meta(plan: &Path) -> Result<PlanMeta, PlanError> {
    let text = std::fs::read_to_string(plan)
        .map_err(|error| PlanError::NotAPlan(format!("{}: {error}", plan.display())))?;
    let shown = plan.display().to_string();
    let mut status = None;
    let mut created = None;
    let mut updated = None;
    let mut slug = None;
    let mut in_fence = false;
    let mut saw_fence = false;
    for line in text.lines() {
        if line.trim_end() == "---" {
            if !saw_fence {
                saw_fence = true;
                in_fence = true;
                continue;
            }
            break;
        }
        if !in_fence {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = unquote(value.trim());
        match key.trim() {
            "status" => {
                status = Some(
                    PlanStatus::parse(value)
                        .ok_or_else(|| PlanError::BadStatus(shown.clone(), value.to_string()))?,
                );
            }
            "created" => created = Some(value.to_string()),
            "updated" => updated = Some(value.to_string()),
            "slug" => slug = Some(value.to_string()),
            _ => {}
        }
    }
    if !saw_fence {
        return Err(PlanError::NotAPlan(shown));
    }
    let field = |value: Option<String>, name: &str| {
        value
            .filter(|value| !value.is_empty())
            .ok_or_else(|| PlanError::MissingField(shown.clone(), name.to_string()))
    };
    Ok(PlanMeta {
        status: status.ok_or_else(|| PlanError::MissingField(shown.clone(), "status".into()))?,
        created: field(created, "created")?,
        updated: field(updated, "updated")?,
        slug: field(slug, "slug")?,
    })
}

/// A value, with one layer of matching quotes taken off — the frontmatter
/// subset's own rule.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// The fenced declaration block inside a plan document, as raw text.
///
/// The fence is ```` ```graph ````, chosen because it is the one marker
/// no other use of the plan document claims and it reads as what it is.
/// The first such fence wins; a second is a malformed plan rather than a
/// merge, because two declarations for one graph is a contradiction.
pub fn declaration_block(text: &str) -> Option<String> {
    let mut inside = false;
    let mut body = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !inside {
            if trimmed == "```graph" {
                inside = true;
            }
        } else if trimmed == "```" {
            return Some(body);
        } else {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }
    None
}

/// Parse the declaration block out of a plan document.
pub fn parse_declaration(text: &str) -> Result<Declaration, PlanError> {
    let Some(block) = declaration_block(text) else {
        return Err(PlanError::Declaration(
            "the plan carries no ```graph declaration block".to_string(),
        ));
    };
    serde_json::from_str(&block)
        .map_err(|error| PlanError::Declaration(format!("the declaration block: {error}")))
}

/// The digest a runtime file records its declaration by: the first 16 hex
/// characters of a SHA-256 over the block, the same hash the route cache
/// joins on.
pub fn declaration_digest(text: &str) -> String {
    match declaration_block(text) {
        Some(block) => crate::calib::hash16(&block),
        None => String::new(),
    }
}

/// The acceptance criteria a plan document declares, by id.
///
/// The plan format's own convention is a bold id at the head of a list
/// item — `- **C1** the parser builds` — and every plan in this repo
/// writes it that way. Scanning for it is what lets the graph say which
/// criteria nothing serves: without the document's own list, the only
/// criteria the graph knows about are the ones already covered, and a
/// criterion nobody is working on would read as merely unmet.
///
/// A criterion the document writes some other way is not found, and that
/// is the honest answer — the convention is the contract, and a plan that
/// breaks it has no machine-readable criteria to check.
pub fn criteria_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("- **") else {
            continue;
        };
        let Some((id, _)) = rest.split_once("**") else {
            continue;
        };
        let id = id.trim();
        if !id.is_empty() && !found.iter().any(|seen| seen == id) {
            found.push(id.to_string());
        }
    }
    found
}

impl PlanGraph {
    /// Generate a graph from a plan document. Only an approved plan
    /// generates one; every other status is a named refusal.
    ///
    /// The declaration is re-declared through `Graph::add`, so a cycle, a
    /// duplicate id, or an unknown prerequisite is refused by name at
    /// generation time exactly as it would be at declaration — the graph
    /// never carries a declaration it would have refused.
    pub fn generate(plan: &Path) -> Result<PlanGraph, PlanError> {
        let meta = read_meta(plan)?;
        if !meta.status.generates_a_graph() {
            return Err(PlanError::NotApproved(
                plan.display().to_string(),
                meta.status.as_str().to_string(),
            ));
        }
        let text = std::fs::read_to_string(plan)
            .map_err(|error| PlanError::NotAPlan(format!("{}: {error}", plan.display())))?;
        let declaration = parse_declaration(&text)?;
        let digest = declaration_digest(&text);
        let mut graph = Graph::new();
        let mut serves = HashMap::new();
        for declared in &declaration.nodes {
            // A summary node has children and no work of its own, so its
            // cost is forced to zero rather than trusted to the author.
            let cost = if declared.summary { 0 } else { declared.cost };
            graph
                .add(
                    Node::new(declared.id.clone(), cost)
                        .with_prereqs(declared.prereqs.clone())
                        .with_paths(declared.paths.clone())
                        .with_capabilities(Capabilities::from(&declared.capabilities))
                        .with_approach(declared.approach.clone()),
                )
                .map_err(|error| PlanError::Graph(error.to_string()))?;
            if !declared.serves.trim().is_empty() {
                serves.insert(declared.id.clone(), declared.serves.trim().to_string());
            }
        }
        // `Graph::add` tolerates a forward declaration — a node declared
        // before its prerequisite exists scores zero until it lands — but
        // the whole declaration is here at once, so a prerequisite that
        // resolves to nothing is a plan error, not a pending edge. Refused
        // by name rather than left for `unlock` to surface later.
        let declared: std::collections::HashSet<&str> = declaration
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        for node in &declaration.nodes {
            for prereq in &node.prereqs {
                // P7 C1: a prerequisite that names another plan's node is
                // not resolved against this declaration — the registry
                // resolves it, and `unlock_with` reports what it found.
                // Only a name that is neither local nor cross-plan is a
                // declaration error.
                if cross_plan_edge(prereq).is_some() {
                    continue;
                }
                if !declared.contains(prereq.as_str()) {
                    return Err(PlanError::Declaration(format!(
                        "`{}` names `{prereq}` as a prerequisite, which the declaration does not declare",
                        node.id
                    )));
                }
            }
        }
        Ok(PlanGraph {
            slug: meta.slug,
            declaration: digest,
            serves,
            graph,
        })
    }

    /// Read a plan's runtime file. An absent file is a named error rather
    /// than an empty graph, because "this plan has never been executed"
    /// and "this plan has no work" are different facts.
    pub fn load(plan: &Path) -> Result<PlanGraph, PlanError> {
        let path = runtime_path_for(plan);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| PlanError::Declaration(format!("{}: {error}", path.display())))?;
        let file: PlanGraphFile = serde_json::from_str(&text)
            .map_err(|error| PlanError::Declaration(format!("{}: {error}", path.display())))?;
        Ok(PlanGraph {
            slug: file.slug,
            declaration: file.declaration,
            serves: file.serves,
            graph: crate::graph::Graph::from_file(file.graph, &path).map_err(PlanError::Graph)?,
        })
    }

    /// Write the runtime file, atomically — the same discipline the
    /// graph's own save runs on, and it matters more here: this file is
    /// the execution record of an approved contract.
    pub fn save(&self, plan: &Path) -> Result<(), PlanError> {
        let path = runtime_path_for(plan);
        let file = PlanGraphFile {
            slug: self.slug.clone(),
            declaration: self.declaration.clone(),
            serves: self.serves.clone(),
            graph: self.graph.to_file(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|error| {
            PlanError::Declaration(format!("the graph would not serialize: {error}"))
        })?;
        let serial = PLAN_WRITE_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut temp = path.clone().into_os_string();
        temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
        let temp = PathBuf::from(temp);
        std::fs::write(&temp, format!("{text}\n"))
            .map_err(|error| PlanError::Declaration(format!("{}: {error}", temp.display())))?;
        std::fs::rename(&temp, &path).map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            PlanError::Declaration(format!("{}: {error}", path.display()))
        })
    }

    /// Whether the plan document has drifted from the declaration this
    /// graph was generated from, named with both digests.
    ///
    /// This is the normal case, not an edge case: the repo's rule is
    /// "update the plan's acceptance criteria first, then build," so a
    /// plan edited mid-flight is expected. What must never happen is the
    /// graph silently mismapping nodes onto criteria that moved.
    pub fn drift(&self, plan: &Path) -> Option<String> {
        let text = std::fs::read_to_string(plan).ok()?;
        let now = declaration_digest(&text);
        if now == self.declaration {
            return None;
        }
        Some(format!(
            "{}: the plan's declaration has changed since this graph was generated \
             (was {}, now {}); regenerate before dispatching",
            plan.display(),
            self.declaration,
            now
        ))
    }

    /// The acceptance-criteria roll-up: each criterion a node serves, the
    /// nodes serving it, and whether every one of them has verified.
    ///
    /// This is the join that makes the plan's own table the progress
    /// surface — the plan already owns "is X done," and this reads it off
    /// the graph rather than off a human's memory.
    pub fn criteria(&self) -> Vec<Criterion> {
        let mut order: Vec<String> = Vec::new();
        let mut by_criterion: HashMap<&str, Vec<&str>> = HashMap::new();
        for (node, criterion) in &self.serves {
            if !by_criterion.contains_key(criterion.as_str()) {
                order.push(criterion.clone());
            }
            by_criterion
                .entry(criterion.as_str())
                .or_default()
                .push(node.as_str());
        }
        order.sort();
        order
            .into_iter()
            .map(|criterion| {
                let mut nodes: Vec<&str> = by_criterion
                    .get(criterion.as_str())
                    .map(|nodes| nodes.to_vec())
                    .unwrap_or_default();
                nodes.sort_unstable();
                let met = nodes.iter().all(|node| {
                    self.graph
                        .node(node)
                        .is_some_and(|node| node.status == Status::Verified)
                });
                Criterion {
                    name: criterion,
                    nodes: nodes.into_iter().map(|node| node.to_string()).collect(),
                    met,
                }
            })
            .collect()
    }

    /// The criteria no node serves — named rather than silently unmet,
    /// because a criterion nobody is working on is a plan that says one
    /// thing and executes another.
    pub fn unserved(&self, criteria: &[String]) -> Vec<String> {
        criteria
            .iter()
            .filter(|criterion| !self.serves.values().any(|served| served == *criterion))
            .cloned()
            .collect()
    }
}

/// P6 C1: what the plan may admit right now, re-consulted after every
/// completion rather than trusted from a plan computed once.
///
/// This is the mechanism the wave loop needs and the reason it is a
/// separate call rather than a field on the plan: a wave plan is computed
/// from the calls the model emitted, and a completion that unblocks new
/// work cannot be honoured by a plan that has already been emitted. So
/// the loop asks this again after each completion, and the answer is
/// always the current one — because the score is derived on read, there is
/// nothing to invalidate.
///
/// Bounded by `bound`, serialized on the resource edge, and ordered by
/// "blocks the most work first" — `dispatchable`'s own rule, which is
/// already the right one. The bound counts what is already running, so a
/// caller passes its in-flight list and the answer fills the remaining
/// slots.
pub fn admit_now(graph: &Graph, bound: usize, running: &[String]) -> Vec<String> {
    graph.dispatchable(bound, running)
}

// --- cross-plan dependencies -----------------------------------------------

/// How a prerequisite names a node in another plan: `<slug>/<node-id>`.
///
/// The slash is the separator because a plan slug is `[a-z0-9-]` and a
/// node id is the same shape, so neither can contain one — the split is
/// unambiguous and a typo reads as a name that resolves to nothing rather
/// than silently to a different node.
pub const CROSS_PLAN_SEPARATOR: char = '/';

/// One cross-plan prerequisite, split into its plan and its node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossPlanEdge {
    /// The plan's slug.
    pub slug: String,
    /// The node's id inside that plan.
    pub node: String,
}

/// Whether a prerequisite names another plan's node, and its two halves.
pub fn cross_plan_edge(prereq: &str) -> Option<CrossPlanEdge> {
    let (slug, node) = prereq.split_once(CROSS_PLAN_SEPARATOR)?;
    if slug.is_empty() || node.is_empty() {
        return None;
    }
    Some(CrossPlanEdge {
        slug: slug.to_string(),
        node: node.to_string(),
    })
}

/// What a cross-plan prerequisite resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossPlanResolution {
    /// The plan is not in the registry at all: no such plan has ever
    /// generated a graph.
    NoSuchPlan,
    /// The plan exists but its status does not let it hold a graph —
    /// a draft, or a cancelled one. A dependency on work that was never
    /// approved, or that was abandoned, is not a dependency that can
    /// ever clear.
    NotApproved(String),
    /// The plan exists and holds the node, and the node has verified.
    Verified,
    /// The plan exists and holds the node, and it has not verified.
    Waiting(String),
    /// The plan exists but does not declare the node.
    NoSuchNode,
}

/// The registry of plans that hold a graph, read from the plans
/// directory.
///
/// There is no separate index file: a plan's runtime file IS its registry
/// entry, and scanning the directory is what keeps the two from drifting.
/// A plan whose runtime file is absent simply is not in the registry,
/// which is the honest answer — it has never generated a graph.
#[derive(Debug, Clone, Default)]
pub struct PlanRegistry {
    plans: std::collections::HashMap<String, RegistryEntry>,
    /// The slug of the plan this registry was built for — the one whose
    /// graph is asking about its cross-plan edges. Empty when the
    /// registry was built without one.
    pub self_slug: String,
}

/// One plan as the registry sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub slug: String,
    pub status: PlanStatus,
    /// The node ids the plan's graph declares.
    pub nodes: Vec<String>,
    /// Which of them have verified.
    pub verified: Vec<String>,
    /// The plan's own edges, as `(node, prereq)` — including its
    /// cross-plan ones, which is what makes a cycle through another plan
    /// visible from either side.
    pub edges: Vec<(String, String)>,
}

impl PlanRegistry {
    /// Scan `plans_dir` for runtime files and read what each holds.
    pub fn scan(plans_dir: &Path) -> PlanRegistry {
        PlanRegistry::scan_for(plans_dir, "")
    }

    /// Scan for one plan's own cross-plan edges: `self_slug` names the
    /// plan asking, so its own edges are namespaced under that slug and
    /// the other plans' under theirs.
    pub fn scan_for(plans_dir: &Path, self_slug: &str) -> PlanRegistry {
        let mut plans = std::collections::HashMap::new();
        let Ok(entries) = std::fs::read_dir(plans_dir) else {
            return PlanRegistry {
                plans,
                self_slug: self_slug.to_string(),
            };
        };
        // The plan DOCUMENTS are enumerated, not the runtime files: a plan
        // that has never been run has no runtime file, and skipping it
        // would make an approved plan unresolvable until somebody ran it
        // first. A draft is in the registry too — that is what makes
        // `NotApproved` a resolvable answer rather than a missing plan.
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
            .collect();
        paths.sort();
        for path in paths {
            // The path IS the plan document: the scan enumerates the
            // documents, not the runtime files, so there is nothing to
            // derive. (`plan_path_for` is the other direction and would
            // append a second `.md` here.)
            let plan_path = path.as_path();
            let Ok(meta) = read_meta(plan_path) else {
                continue;
            };
            // The runtime file carries the state, and the plan document
            // carries the declaration. A plan that has never been run has
            // no runtime file, and reading its declaration rather than
            // skipping it is what makes an approved plan resolvable
            // without somebody having to run it first.
            let plan_graph = match PlanGraph::load(plan_path) {
                Ok(loaded) => loaded,
                Err(_) => match PlanGraph::generate(plan_path) {
                    Ok(generated) => generated,
                    Err(_) => continue,
                },
            };
            let nodes: Vec<String> = plan_graph
                .graph
                .nodes()
                .iter()
                .map(|node| node.id.clone())
                .collect();
            let verified: Vec<String> = plan_graph
                .graph
                .nodes()
                .iter()
                .filter(|node| node.status == Status::Verified)
                .map(|node| node.id.clone())
                .collect();
            let edges: Vec<(String, String)> = plan_graph
                .graph
                .nodes()
                .iter()
                .flat_map(|node| {
                    node.prereqs
                        .iter()
                        .map(move |prereq| (node.id.clone(), prereq.clone()))
                })
                .collect();
            plans.insert(
                meta.slug.clone(),
                RegistryEntry {
                    slug: meta.slug,
                    status: meta.status,
                    nodes,
                    verified,
                    edges,
                },
            );
        }
        PlanRegistry {
            plans,
            self_slug: self_slug.to_string(),
        }
    }

    /// The entry for a slug, or `None` when no plan of that name holds a
    /// graph.
    pub fn entry(&self, slug: &str) -> Option<&RegistryEntry> {
        self.plans.get(slug)
    }

    /// Resolve one cross-plan prerequisite.
    pub fn resolve(&self, edge: &CrossPlanEdge) -> CrossPlanResolution {
        let Some(entry) = self.plans.get(&edge.slug) else {
            return CrossPlanResolution::NoSuchPlan;
        };
        // A plan whose status does not generate a graph cannot hold work
        // that will ever verify: a draft was never approved and a
        // cancelled one was abandoned. Either way the dependency cannot
        // clear, and saying so is the whole point.
        if !entry.status.generates_a_graph() {
            return CrossPlanResolution::NotApproved(entry.status.as_str().to_string());
        }
        if !entry.nodes.iter().any(|node| node == &edge.node) {
            return CrossPlanResolution::NoSuchNode;
        }
        if entry.verified.iter().any(|node| node == &edge.node) {
            CrossPlanResolution::Verified
        } else {
            CrossPlanResolution::Waiting(edge.node.clone())
        }
    }

    /// Every edge that points at this plan — the reverse index a cycle
    /// check needs, because a cycle through another plan is only visible
    /// from the other side.
    pub fn edges_into(&self, slug: &str) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for (other, entry) in &self.plans {
            if other == slug {
                continue;
            }
            for node in &entry.nodes {
                // A node whose id carries another plan's slug names it.
                if let Some(edge) = cross_plan_edge(node) {
                    if edge.slug == slug {
                        found.push((other.clone(), edge.node.clone()));
                    }
                }
            }
        }
        found
    }
}

/// P6 C4: merge the sub-plan a summary's child run handed back.
///
/// A child that reaches a node which is not atomic reports
/// [`DeltaReason::NotAtomic`](crate::subagents::DeltaReason::NotAtomic)
/// with the children it found riding [`Delta::declares`]. This is what
/// that report does: the node becomes a summary — its cost forced to zero,
/// because a node that only groups others does no work of its own — and
/// each declared child is added with the summary as its prerequisite.
///
/// That last part is the whole point. A summary's children ARE its
/// prerequisites, so the summary cannot reach a zero dependency score
/// until every child verifies, and `admit_now` offers the children the
/// moment they exist. Bounded-depth planning falls out of this: the
/// orchestrator declares to a depth it can see, and expansion happens
/// where the work actually turns out to break down rather than where the
/// orchestrator guessed it would.
///
/// Every child goes through `Graph::add`, so a cycle, a duplicate id, or
/// an unknown prerequisite among them is refused by name — and the whole
/// merge is atomic: if any child is refused, none of them land and the
/// node stays atomic.
pub fn merge_declared(
    graph: &mut Graph,
    node: &str,
    children: &[DeclaredNode],
) -> Result<Vec<String>, String> {
    graph
        .index_of(node)
        .ok_or_else(|| format!("node `{node}` is not in the graph"))?;
    if children.is_empty() {
        return Err(format!(
            "`{node}` reported not-atomic but declared no children"
        ));
    }
    // The declared children must resolve among themselves and against the
    // graph as it stands, before anything lands.
    let known: std::collections::HashSet<&str> = graph
        .nodes()
        .iter()
        .map(|existing| existing.id.as_str())
        .chain(children.iter().map(|child| child.id.as_str()))
        .collect();
    for child in children {
        for prereq in &child.prereqs {
            if !known.contains(prereq.as_str()) {
                return Err(format!(
                    "`{}` names `{prereq}` as a prerequisite, which neither the graph nor the sub-plan declares",
                    child.id
                ));
            }
        }
        if child.id == node {
            return Err(format!("`{}` declared itself as its own child", child.id));
        }
    }
    // The dry run, before anything lands. A cycle between two children is
    // not visible until the second one is added, and by then the first
    // has already landed — so the whole sub-plan is rehearsed against a
    // scratch graph carrying the same edges, and only a rehearsal that
    // builds reaches the real one.
    let mut rehearsal = Graph::new();
    for existing in graph.nodes() {
        rehearsal
            .add(Node::new(existing.id.clone(), existing.cost))
            .map_err(|error| error.to_string())?;
        for prereq in &existing.prereqs {
            rehearsal
                .add_prerequisite(&existing.id, prereq)
                .map_err(|error| error.to_string())?;
        }
    }
    for child in children {
        rehearsal
            .add(Node::new(child.id.clone(), child.cost).with_prereqs(child.prereqs.clone()))
            .map_err(|error| error.to_string())?;
    }

    let mut added: Vec<String> = Vec::new();
    for child in children {
        graph
            .add(
                Node::new(child.id.clone(), child.cost)
                    .with_prereqs(child.prereqs.clone())
                    .with_paths(child.paths.clone())
                    .with_capabilities(Capabilities::from(&child.capabilities))
                    .with_approach(child.approach.clone()),
            )
            .map_err(|error| error.to_string())?;
        added.push(child.id.clone());
    }
    // The node is a summary now: no cost of its own, and its children are
    // its prerequisites. Re-declared through `with_prereqs` so the cycle
    // check runs over the new edges too.
    let summary_prereqs: Vec<String> = children.iter().map(|child| child.id.clone()).collect();
    graph
        .add_prerequisites(node, &summary_prereqs)
        .map_err(|error| error.to_string())?;
    // A summary does no work of its own.
    graph
        .revise_cost(node, 0)
        .map_err(|error| error.to_string())?;
    Ok(added)
}

/// P6 C5: demote a node whose delivered work does not integrate, and
/// record why.
///
/// The repo's own rule — "update the plan's acceptance criteria first,
/// then build" — finally has an execution record to apply it to: the node
/// goes back to pending so `admit_now` offers it again, the reason lands
/// as an append-only revision, and every dependent's score rises on the
/// next read because the node is unfinished again.
pub fn demote(graph: &mut Graph, node: &str, reason: &str) -> Result<(), String> {
    graph
        .demote(node, reason)
        .map_err(|error| error.to_string())
}

/// One acceptance criterion, as the graph reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    pub name: String,
    /// The nodes serving it, sorted — the facts that have to verify.
    pub nodes: Vec<String>,
    /// Whether every one of them has.
    pub met: bool,
}

/// A per-write serial for the runtime file's temp name.
static PLAN_WRITE_SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fleet-plan-graph-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A plan document in the README's format, with an optional
    /// declaration block and status.
    fn plan(root: &Path, status: &str, declaration: Option<&str>) -> std::path::PathBuf {
        let path = root.join("2026-10-19-a-plan.md");
        let mut text = format!(
            "---\nstatus: {status}\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: a-plan\n---\n\n## Context\n\nA body of work.\n"
        );
        if let Some(block) = declaration {
            text.push_str("\n## The declaration\n\n```graph\n");
            text.push_str(block);
            text.push_str("\n```\n");
        }
        std::fs::write(&path, text).unwrap();
        path
    }

    const DECLARATION: &str = r#"{
  "nodes": [
    {"id": "parse-grammar", "cost": 2, "approach": "read the grammar"},
    {"id": "build-parser", "cost": 4, "prereqs": ["parse-grammar"], "serves": "C1"},
    {"id": "wire-it-up", "cost": 3, "prereqs": ["build-parser"], "serves": "C2"}
  ]
}"#;

    #[test]
    fn the_readme_format_parses_and_a_nonconforming_plan_is_named() {
        let dir = scratch("meta");
        let path = plan(&dir, "active", Some(DECLARATION));
        let meta = read_meta(&path).expect("the README's format parses");
        assert_eq!(meta.status, PlanStatus::Active);
        assert_eq!(meta.slug, "a-plan");
        assert_eq!(meta.created, "2026-10-19");

        // A plan with no frontmatter at all is not a plan.
        let bare = dir.join("bare.md");
        std::fs::write(&bare, "# Just a heading\n").unwrap();
        assert!(matches!(read_meta(&bare), Err(PlanError::NotAPlan(_))));

        // A missing field is named, not defaulted.
        let partial = dir.join("partial.md");
        std::fs::write(&partial, "---\nstatus: active\n---\n").unwrap();
        assert!(
            matches!(&read_meta(&partial), Err(PlanError::MissingField(_, field)) if field == "created"),
            "{:?}",
            read_meta(&partial)
        );

        // A status outside the vocabulary is named.
        let odd = dir.join("odd.md");
        std::fs::write(
            &odd,
            "---\nstatus: wip\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: odd\n---\n",
        )
        .unwrap();
        assert!(
            matches!(&read_meta(&odd), Err(PlanError::BadStatus(_, value)) if value == "wip"),
            "{:?}",
            read_meta(&odd)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_plans_own_criteria_are_read_so_an_unserved_one_is_named() {
        // P4 C4: the criteria list comes from the document's own
        // convention. Without it, the only criteria the graph knows are
        // the ones already covered, and a criterion nobody is working on
        // would read as merely unmet rather than as a plan that says one
        // thing and executes another.
        let text = "## Acceptance criteria\n\n- **C1** the parser builds\n- **C2** it is wired up\n- **C7** nobody serves this\n\nProse that mentions **C9** inline is not a list item.\n";
        assert_eq!(criteria_in(text), ids(&["C1", "C2", "C7"]));

        let dir = scratch("unserved");
        let path = plan(
            &dir,
            "active",
            Some(r#"{"nodes":[{"id":"a","serves":"C1"}]}"#),
        );
        let full = std::fs::read_to_string(&path).unwrap();
        // The plan document the fixture writes carries no criteria list,
        // so write one and confirm the roll-up names the gap.
        let with_criteria =
            format!("{full}\n## Acceptance criteria\n\n- **C1** one\n- **C4** unserved\n");
        std::fs::write(&path, &with_criteria).unwrap();
        let graph = PlanGraph::generate(&path).unwrap();
        assert_eq!(graph.criteria().len(), 1, "only C1 is served");
        let declared = criteria_in(&with_criteria);
        assert_eq!(
            graph.unserved(&declared),
            vec!["C4".to_string()],
            "a declared criterion no node serves is named"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_an_approved_plan_generates_a_graph() {
        let dir = scratch("approval");
        for status in ["draft", "completed", "cancelled"] {
            let path = plan(&dir, status, Some(DECLARATION));
            let error = PlanGraph::generate(&path).expect_err("no graph from this status");
            assert!(
                matches!(&error, PlanError::NotApproved(_, named) if named == status),
                "{status}: {error}"
            );
        }
        // And the approved one does.
        let path = plan(&dir, "active", Some(DECLARATION));
        let generated = PlanGraph::generate(&path).expect("an approved plan generates a graph");
        assert_eq!(generated.graph.len(), 3);
        assert_eq!(generated.slug, "a-plan");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_declaration_is_refused_where_the_graph_would_refuse_it() {
        let dir = scratch("refuse");
        // A cycle.
        let cyclic = r#"{"nodes":[{"id":"a","prereqs":["b"]},{"id":"b","prereqs":["a"]}]}"#;
        let path = plan(&dir, "active", Some(cyclic));
        assert!(
            matches!(PlanGraph::generate(&path), Err(PlanError::Graph(_))),
            "a cycle in the declaration is refused by name"
        );
        // An unknown prerequisite. `Graph::add` tolerates a forward
        // declaration, but a whole declaration is available at once, so a
        // reference to a node nobody declared is refused here rather than
        // left for `unlock` to surface as unresolved.
        let ghost = r#"{"nodes":[{"id":"a","prereqs":["nowhere"]}]}"#;
        let path = plan(&dir, "active", Some(ghost));
        let error = PlanGraph::generate(&path).expect_err("an unknown prerequisite is refused");
        assert!(
            matches!(&error, PlanError::Declaration(reason) if reason.contains("nowhere")),
            "{error}"
        );
        // No declaration block at all.
        let path = plan(&dir, "active", None);
        assert!(
            matches!(PlanGraph::generate(&path), Err(PlanError::Declaration(_))),
            "a plan with no declaration is named, not guessed at"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_runtime_file_rides_beside_the_plan_and_survives_completion() {
        let dir = scratch("runtime");
        let path = plan(&dir, "active", Some(DECLARATION));
        let mut graph = PlanGraph::generate(&path).unwrap();

        // The sibling is where the plan's own path points, and it is
        // gitignored rather than tracked.
        let runtime = runtime_path_for(&path);
        assert_eq!(
            runtime.file_name().and_then(|name| name.to_str()),
            Some("2026-10-19-a-plan.graph.json")
        );

        // Mid-flight state rides it.
        graph.graph.dispatch("parse-grammar").unwrap();
        graph.save(&path).unwrap();
        let loaded = PlanGraph::load(&path).expect("the runtime file reads back");
        assert_eq!(
            loaded.graph.node("parse-grammar").map(|node| node.status),
            Some(crate::graph::Status::Running),
            "a graph saved mid-flight reads back mid-flight"
        );
        assert_eq!(
            loaded.serves.get("build-parser").map(String::as_str),
            Some("C1")
        );

        // And it is retained after the plan completes: the repo's own
        // rule is that a completed plan is never reopened, so the graph
        // is a permanent record rather than something cleaned up.
        std::fs::write(
            &path,
            std::fs::read_to_string(&path)
                .unwrap()
                .replace("status: active", "status: completed"),
        )
        .unwrap();
        assert!(
            PlanGraph::generate(&path).is_err(),
            "a completed plan no longer generates"
        );
        let retained = PlanGraph::load(&path).expect("but its graph is retained");
        assert_eq!(retained.graph.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plan_edited_after_its_graph_was_generated_is_named() {
        let dir = scratch("drift");
        let path = plan(&dir, "active", Some(DECLARATION));
        let graph = PlanGraph::generate(&path).unwrap();
        assert!(
            graph.drift(&path).is_none(),
            "a fresh graph has not drifted"
        );

        // The repo's own rule: update the plan's acceptance criteria
        // first, then build. So drift is the normal case, and it must be
        // named rather than silently mismapping nodes onto criteria that
        // moved.
        let edited = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"cost\": 4", "\"cost\": 9");
        std::fs::write(&path, edited).unwrap();
        let drift = graph.drift(&path).expect("the drift is named");
        assert!(
            drift.contains("has changed since this graph was generated"),
            "{drift}"
        );
        assert!(
            drift.contains(&graph.declaration) && drift.contains("now "),
            "both digests are named: {drift}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_criteria_roll_up_off_the_graph() {
        let dir = scratch("criteria");
        let path = plan(&dir, "active", Some(DECLARATION));
        let mut graph = PlanGraph::generate(&path).unwrap();

        let criteria = graph.criteria();
        assert_eq!(criteria.len(), 2, "two criteria are served");
        let c1 = criteria.iter().find(|c| c.name == "C1").expect("C1");
        assert_eq!(c1.nodes, vec!["build-parser".to_string()]);
        assert!(!c1.met, "nothing has verified");

        // Verify the node serving C1 through the graph's own gate. The
        // verifier is attached after construction — the seam a loaded
        // graph needs, because a closure cannot ride a file.
        let facts = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let seen = std::sync::Arc::clone(&facts);
        graph.graph.set_verifier(Box::new(move |node: &Node| {
            seen.lock().unwrap().contains(&node.id)
        }));
        facts.lock().unwrap().insert("parse-grammar".to_string());
        facts.lock().unwrap().insert("build-parser".to_string());
        graph.graph.settle();
        let criteria = graph.criteria();
        assert!(
            criteria.iter().find(|c| c.name == "C1").expect("C1").met,
            "C1 is met when its node verifies"
        );
        assert!(
            !criteria.iter().find(|c| c.name == "C2").expect("C2").met,
            "and C2 is not, because its node has not"
        );

        // A criterion no node serves is named rather than silently unmet.
        let unserved = graph.unserved(&["C1".into(), "C2".into(), "C7".into()]);
        assert_eq!(unserved, vec!["C7".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P6 C4: a child that reaches a node which is not atomic hands back
    /// the children it found, and the orchestrator merges them.
    #[test]
    fn a_child_that_finds_its_node_is_not_atomic_hands_back_its_children() {
        let dir = scratch("summary-expand");
        let path = plan(
            &dir,
            "active",
            Some(r#"{"nodes":[{"id":"group","cost":9,"approach":"do the whole thing"}]}"#),
        );
        let mut plan_graph = PlanGraph::generate(&path).unwrap();
        assert_eq!(plan_graph.graph.node("group").map(|n| n.cost), Some(9));

        // The child ran, looked, and found the work breaks in two.
        let children = vec![
            DeclaredNode {
                id: "half-a".to_string(),
                cost: 3,
                prereqs: Vec::new(),
                paths: vec!["src/a.rs".to_string()],
                capabilities: Default::default(),
                approach: "the first half".to_string(),
                serves: "C1".to_string(),
                summary: false,
            },
            DeclaredNode {
                id: "half-b".to_string(),
                cost: 4,
                prereqs: vec!["half-a".to_string()],
                paths: Vec::new(),
                capabilities: Default::default(),
                approach: "the second half".to_string(),
                serves: "C1".to_string(),
                summary: false,
            },
        ];
        let added =
            merge_declared(&mut plan_graph.graph, "group", &children).expect("the sub-plan merges");
        assert_eq!(added, vec!["half-a".to_string(), "half-b".to_string()]);

        // The node is a summary now: no cost, and its children are its
        // prerequisites — so it cannot start until they verify.
        assert_eq!(
            plan_graph.graph.node("group").map(|node| node.cost),
            Some(0),
            "a summary does no work of its own"
        );
        assert!(
            !plan_graph.graph.is_unlocked("group"),
            "it waits on its children"
        );
        assert_eq!(
            plan_graph.graph.dispatchable(3, &[]),
            vec!["half-a".to_string()],
            "the children are offered the moment they exist"
        );
        // And the criterion roll-up now sees the nodes serving it.
        plan_graph
            .serves
            .insert("half-a".to_string(), "C1".to_string());
        plan_graph
            .serves
            .insert("half-b".to_string(), "C1".to_string());
        let criteria = plan_graph.criteria();
        let c1 = criteria.iter().find(|c| c.name == "C1").expect("C1");
        assert_eq!(c1.nodes, vec!["half-a".to_string(), "half-b".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P6 C4: the merge is atomic. A sub-plan that would not build leaves
    /// the graph exactly as it was, and the node stays atomic.
    #[test]
    fn a_sub_plan_that_would_not_build_changes_nothing() {
        let dir = scratch("summary-refused");
        let path = plan(
            &dir,
            "active",
            Some(r#"{"nodes":[{"id":"group","cost":9}]}"#),
        );
        let mut plan_graph = PlanGraph::generate(&path).unwrap();

        // A child naming a prerequisite nobody declares.
        let ghost = vec![DeclaredNode {
            id: "a".to_string(),
            cost: 1,
            prereqs: vec!["nowhere".to_string()],
            paths: Vec::new(),
            capabilities: Default::default(),
            approach: String::new(),
            serves: String::new(),
            summary: false,
        }];
        let error = merge_declared(&mut plan_graph.graph, "group", &ghost)
            .expect_err("the unknown prerequisite is refused");
        assert!(error.contains("nowhere"), "{error}");
        assert_eq!(
            plan_graph.graph.len(),
            1,
            "nothing landed: the merge is atomic"
        );
        assert_eq!(
            plan_graph.graph.node("group").map(|node| node.cost),
            Some(9),
            "and the node is still atomic"
        );

        // A cycle among the children.
        let cyclic = vec![
            DeclaredNode {
                id: "x".to_string(),
                cost: 1,
                prereqs: vec!["y".to_string()],
                paths: Vec::new(),
                capabilities: Default::default(),
                approach: String::new(),
                serves: String::new(),
                summary: false,
            },
            DeclaredNode {
                id: "y".to_string(),
                cost: 1,
                prereqs: vec!["x".to_string()],
                paths: Vec::new(),
                capabilities: Default::default(),
                approach: String::new(),
                serves: String::new(),
                summary: false,
            },
        ];
        assert!(merge_declared(&mut plan_graph.graph, "group", &cyclic).is_err());
        assert_eq!(plan_graph.graph.len(), 1, "still nothing landed");

        // A child declaring itself as the summary's child.
        let itself = vec![DeclaredNode {
            id: "group".to_string(),
            cost: 1,
            prereqs: Vec::new(),
            paths: Vec::new(),
            capabilities: Default::default(),
            approach: String::new(),
            serves: String::new(),
            summary: false,
        }];
        let error = merge_declared(&mut plan_graph.graph, "group", &itself).unwrap_err();
        assert!(error.contains("its own child"), "{error}");

        // And a not-atomic report with no children at all.
        let error = merge_declared(&mut plan_graph.graph, "group", &[]).unwrap_err();
        assert!(error.contains("declared no children"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P6 C4: a summary's own cost is forced to zero, so a node declared
    /// with one is corrected rather than trusted.
    #[test]
    fn a_summary_declared_with_a_cost_is_corrected() {
        let dir = scratch("summary-cost");
        let path = plan(
            &dir,
            "active",
            Some(
                r#"{"nodes":[
                    {"id":"leaf","cost":2},
                    {"id":"group","summary":true,"cost":9,"prereqs":["leaf"]}
                ]}"#,
            ),
        );
        let plan_graph = PlanGraph::generate(&path).unwrap();
        assert_eq!(
            plan_graph.graph.node("group").map(|node| node.cost),
            Some(0),
            "a declared summary's cost is forced to zero"
        );
        // It still waits on its declared child.
        assert!(!plan_graph.graph.is_unlocked("group"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P6 C4: the fifth reason is priced at zero. A node recognised as a
    /// summary is not a deviation from anything, so it fires no replan.
    #[test]
    fn a_not_atomic_report_costs_nothing_and_fires_no_replan() {
        let mut graph = Graph::new();
        graph
            .add(Node::new("group", 9).with_approach("do the whole thing"))
            .unwrap();
        graph
            .record_delta(
                "group",
                crate::subagents::Delta {
                    state: crate::subagents::DeltaState::Deviated,
                    reason: crate::subagents::DeltaReason::NotAtomic,
                    companion: String::new(),
                    instead: String::new(),
                    forced: false,
                    children: Vec::new(),
                    contradiction: None,
                    declares: vec![DeclaredNode {
                        id: "half".to_string(),
                        cost: 4,
                        prereqs: Vec::new(),
                        paths: Vec::new(),
                        capabilities: Default::default(),
                        approach: "the first half".to_string(),
                        serves: String::new(),
                        summary: false,
                    }],
                },
            )
            .unwrap();
        assert_eq!(graph.diff_score(), 0, "a summary costs nothing");
        assert!(!graph.should_replan(), "and fires no replan");
        assert!(graph.replan().is_none());
        // And the merge is what acts on it.
        let declared = graph
            .node("group")
            .and_then(|node| node.delta.as_ref())
            .map(|delta| delta.declares.clone())
            .unwrap_or_default();
        let added = merge_declared(&mut graph, "group", &declared).unwrap();
        assert_eq!(added, vec!["half".to_string()]);
    }

    // --- P7: cross-plan dependencies -----------------------------------------

    /// A plans directory with two plans, so a cross-plan edge has
    /// something to resolve against.
    fn two_plans(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let plans = root.join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        let a = plans.join("2026-10-19-alpha.md");
        std::fs::write(
            &a,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: alpha\n---\n\n```graph\n{\"nodes\":[{\"id\":\"needs-b\",\"prereqs\":[\"beta/b-one\"]}]}\n```\n",
        )
        .unwrap();
        let b = plans.join("2026-10-19-beta.md");
        std::fs::write(
            &b,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: beta\n---\n\n```graph\n{\"nodes\":[{\"id\":\"b-one\",\"cost\":2}]}\n```\n",
        )
        .unwrap();
        (a, b)
    }

    #[test]
    fn a_cross_plan_prerequisite_resolves_instead_of_reading_as_a_typo() {
        // P7 C1: `Unlock::unresolved` used to be the only answer for a
        // prerequisite the graph does not carry, which made a cross-plan
        // dependency indistinguishable from a typo.
        let dir = scratch("cross-resolve");
        let (a, b) = two_plans(&dir);
        let graph_a = PlanGraph::generate(&a).unwrap();
        let graph_b = PlanGraph::generate(&b).unwrap();
        graph_b.save(&b).unwrap();

        let registry = PlanRegistry::scan_for(&dir.join("plans"), "alpha");
        // beta's b-one is pending, so alpha's node waits on it and says
        // which plan and node.
        let unlock = graph_a
            .graph
            .unlock_with("needs-b", Some(&registry))
            .expect("declared");
        assert!(!unlock.is_unlocked(), "the cross-plan edge holds");
        assert!(
            unlock.unresolved.is_empty(),
            "it is not a typo: {:?}",
            unlock.unresolved
        );
        assert_eq!(
            unlock.cross_plan,
            vec!["waiting on `b-one` in the `beta` plan".to_string()]
        );
        assert!(unlock.unresolvable.is_empty(), "it can still clear");
        assert!(unlock.held_by_cross_plan());

        // beta's node verifies, and the edge clears.
        let mut graph_b = graph_b;
        graph_b.graph.mark_verified("b-one");
        graph_b.save(&b).unwrap();
        let registry = PlanRegistry::scan_for(&dir.join("plans"), "alpha");
        let unlock = graph_a
            .graph
            .unlock_with("needs-b", Some(&registry))
            .expect("declared");
        assert!(
            unlock.is_unlocked(),
            "the verified cross-plan node clears the edge"
        );
        assert!(unlock.cross_plan.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dependency_on_a_plan_that_can_never_verify_is_named() {
        // P7 C3: a dependency on a draft, a cancelled plan, a plan that
        // does not exist, or a node the plan does not declare is not
        // ordinary waiting. Left in `unresolved` it reads as a typo and
        // the dependent blocks forever with nothing to say why.
        let dir = scratch("cross-dead");
        let plans = dir.join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        let a = plans.join("2026-10-19-alpha.md");
        std::fs::write(
            &a,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: alpha\n---\n\n```graph\n{\"nodes\":[\n  {\"id\":\"on-draft\",\"prereqs\":[\"gamma/g\"]},\n  {\"id\":\"on-cancelled\",\"prereqs\":[\"delta/d\"]},\n  {\"id\":\"on-nothing\",\"prereqs\":[\"nowhere/n\"]},\n  {\"id\":\"on-ghost-node\",\"prereqs\":[\"beta/absent\"]}\n]}\n```\n",
        )
        .unwrap();
        // A draft: never approved, so its work can never verify.
        std::fs::write(
            plans.join("2026-10-19-gamma.md"),
            "---\nstatus: draft\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: gamma\n---\n\n```graph\n{\"nodes\":[{\"id\":\"g\",\"cost\":1}]}\n```\n",
        )
        .unwrap();
        // A cancelled plan: abandoned.
        std::fs::write(
            plans.join("2026-10-19-delta.md"),
            "---\nstatus: cancelled\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: delta\n---\n\n```graph\n{\"nodes\":[{\"id\":\"d\",\"cost\":1}]}\n```\n",
        )
        .unwrap();
        // A real plan that does not declare the node.
        let beta = plans.join("2026-10-19-beta.md");
        std::fs::write(
            &beta,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: beta\n---\n\n```graph\n{\"nodes\":[{\"id\":\"b-one\",\"cost\":1}]}\n```\n",
        )
        .unwrap();

        let graph_a = PlanGraph::generate(&a).unwrap();
        let registry = PlanRegistry::scan_for(&plans, "alpha");

        for (node, words) in [
            ("on-draft", "gamma"),
            ("on-cancelled", "delta"),
            ("on-nothing", "nowhere"),
            ("on-ghost-node", "beta"),
        ] {
            let unlock = graph_a
                .graph
                .unlock_with(node, Some(&registry))
                .expect("declared");
            assert!(!unlock.is_unlocked(), "{node}: an unresolvable edge holds");
            assert!(
                unlock
                    .unresolvable
                    .iter()
                    .any(|named| named.contains(words)),
                "{node}: the reason names the plan — {:?}",
                unlock.unresolvable
            );
            assert!(
                unlock.unresolved.is_empty(),
                "{node}: it is not a typo — {:?}",
                unlock.unresolved
            );
        }
        // And the surface names every one of them, with the node that
        // needs it.
        let named = unresolvable_edges(&graph_a.graph, &registry);
        assert_eq!(named.len(), 4, "all four are named: {named:?}");
        assert!(named
            .iter()
            .all(|line| line.contains("alpha") || line.contains("needs") || line.contains("on-")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_plans_waiting_on_each_other_are_refused_by_name() {
        // P7 C2: a cycle through another plan is invisible from either
        // side alone, so each graph's own check passes and the two
        // deadlock with nothing to notice.
        let dir = scratch("cross-cycle");
        let plans = dir.join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        let a = plans.join("2026-10-19-alpha.md");
        std::fs::write(
            &a,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: alpha\n---\n\n```graph\n{\"nodes\":[{\"id\":\"a-one\",\"prereqs\":[\"beta/b-one\"]}]}\n```\n",
        )
        .unwrap();
        let b = plans.join("2026-10-19-beta.md");
        std::fs::write(
            &b,
            "---\nstatus: active\ncreated: 2026-10-19\nupdated: 2026-10-19\nslug: beta\n---\n\n```graph\n{\"nodes\":[{\"id\":\"b-one\",\"prereqs\":[\"alpha/a-one\"]}]}\n```\n",
        )
        .unwrap();
        let graph_a = PlanGraph::generate(&a).unwrap();
        let graph_b = PlanGraph::generate(&b).unwrap();
        graph_b.save(&b).unwrap();

        // Neither graph's own check sees it: each holds one edge.
        assert!(graph_a.graph.ready().is_empty());
        assert!(graph_b.graph.ready().is_empty());

        let registry = PlanRegistry::scan_for(&plans, "alpha");
        let cycle =
            cross_plan_cycle(&graph_a.graph, &registry).expect("the cross-plan cycle is found");
        assert!(cycle.len() >= 3, "a cycle names its path: {cycle:?}");
        assert!(
            cycle.iter().any(|step| step.contains("alpha/a-one"))
                && cycle.iter().any(|step| step.contains("beta/b-one")),
            "both halves are named: {cycle:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_acyclic_cross_plan_edge_finds_no_cycle() {
        // The other half of C2: a legitimate cross-plan dependency must
        // not be refused.
        let dir = scratch("cross-acyclic");
        let (a, b) = two_plans(&dir);
        let graph_a = PlanGraph::generate(&a).unwrap();
        let graph_b = PlanGraph::generate(&b).unwrap();
        graph_b.save(&b).unwrap();
        let registry = PlanRegistry::scan_for(&dir.join("plans"), "alpha");
        assert!(
            cross_plan_cycle(&graph_a.graph, &registry).is_none(),
            "one plan waiting on another is not a cycle"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_admission_is_re_consulted_after_every_completion() {
        // P6 C1: a wave plan computed once cannot honour a completion
        // that unblocks new work, so the loop asks again after each one
        // and the answer is always current. Bounded by the concurrency
        // bound and by what is already running.
        let dir = scratch("admit");
        let path = plan(
            &dir,
            "active",
            Some(
                r#"{"nodes":[
                    {"id":"a","cost":1},
                    {"id":"b","cost":1,"prereqs":["a"]},
                    {"id":"c","cost":1,"prereqs":["a"]}
                ]}"#,
            ),
        );
        let mut plan_graph = PlanGraph::generate(&path).unwrap();
        let graph = &mut plan_graph.graph;

        // One slot, one running node: nothing else fits.
        assert_eq!(admit_now(graph, 1, &[]), vec!["a".to_string()]);
        assert!(
            admit_now(graph, 1, &["a".to_string()]).is_empty(),
            "the bound counts what is already running"
        );

        // `a` verifies. Both branches unlock, and the bound admits one of
        // them — the one blocking the most work, then the larger cost,
        // then the id.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let gate = std::sync::Arc::clone(&seen);
        graph.set_verifier(Box::new(move |node: &Node| {
            gate.lock().unwrap().contains(&node.id)
        }));
        seen.lock().unwrap().insert("a".to_string());
        graph.settle();
        assert_eq!(graph.node("a").map(|n| n.status), Some(Status::Verified));
        let admitted = admit_now(graph, 1, &[]);
        assert_eq!(admitted.len(), 1, "the bound admits one");
        assert!(
            admitted[0] == "b" || admitted[0] == "c",
            "and it is a newly-unlocked branch: {admitted:?}"
        );
        // With the first branch running, the other is still admitted into
        // the free slot — they name no shared path.
        assert_eq!(admit_now(graph, 2, &[admitted[0].clone()]).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_node_whose_work_does_not_integrate_is_demoted_with_its_reason() {
        // P6 C5: the repo's own rule — "update the plan's acceptance
        // criteria first, then build" — finally has an execution record.
        let dir = scratch("demote-plan");
        let path = plan(
            &dir,
            "active",
            Some(r#"{"nodes":[{"id":"a","cost":2},{"id":"b","cost":3,"prereqs":["a"]}]}"#),
        );
        let mut plan_graph = PlanGraph::generate(&path).unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let gate = std::sync::Arc::clone(&seen);
        plan_graph.graph.set_verifier(Box::new(move |node: &Node| {
            gate.lock().unwrap().contains(&node.id)
        }));
        seen.lock().unwrap().insert("a".to_string());
        seen.lock().unwrap().insert("b".to_string());
        plan_graph.graph.settle();
        assert_eq!(plan_graph.graph.score("b"), Some(0));

        // b's work does not integrate.
        demote(
            &mut plan_graph.graph,
            "b",
            "the artifact does not compile against a",
        )
        .unwrap();
        assert_eq!(
            plan_graph.graph.node("b").map(|node| node.status),
            Some(Status::Pending),
            "a demoted node returns to pending, so it is offered again"
        );
        assert!(
            plan_graph.graph.ready().contains(&"b".to_string()),
            "and `admit_now` will hand it out again"
        );
        let revisions = plan_graph.graph.revisions();
        assert!(
            revisions
                .last()
                .is_some_and(|revision| revision.cause.contains("does not compile against a")),
            "the revision names the reason: {:?}",
            revisions.last().map(|revision| revision.cause.clone())
        );
        // And it rides the runtime file, so the record survives.
        plan_graph.save(&path).unwrap();
        let reloaded = PlanGraph::load(&path).unwrap();
        assert_eq!(
            reloaded.graph.node("b").map(|node| node.status),
            Some(Status::Pending)
        );
        assert_eq!(reloaded.graph.revisions().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_summary_node_carries_no_cost_of_its_own() {
        let dir = scratch("summary");
        let declaration = r#"{"nodes":[
            {"id":"a","cost":3},
            {"id":"b","cost":4},
            {"id":"group","summary":true,"prereqs":["a","b"],"serves":"C1"}
        ]}"#;
        let path = plan(&dir, "active", Some(declaration));
        let graph = PlanGraph::generate(&path).unwrap();
        assert_eq!(
            graph.graph.node("group").map(|node| node.cost),
            Some(0),
            "a summary node's cost is forced to zero rather than trusted to the author"
        );
        // It cannot start until its children verify, because its
        // prerequisites ARE its children.
        assert!(
            !graph.graph.is_unlocked("group"),
            "a summary node waits on its children"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
