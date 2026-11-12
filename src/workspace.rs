//! The work-object graph (armada L7): the connective tissue between the
//! four stores fleet already has — the thread (a session), the deliverable
//! (a desk project), the lesson (a learning session's memory facts), the
//! bot (a role) — plus the one object fleet lacked, the **task**.
//!
//! Only `task` is store-owned. Thread/deliverable/lesson/bot are typed
//! references derived on read from the session/desk/roles stores (KI-300:
//! one derivation, no parallel store). Tasks and edges persist as
//! append-only JSONL at `<home>/workspace/graph.jsonl`, folded to current
//! state on read — the journal's shape (D10) in a home exempt from journal
//! retention (D11 curtails day-files by age; durable work objects need
//! their own home). This is NOT `graph.rs` (cwd-bound dev-plan machinery).
//!
//! The graph composes the bedrock rather than re-implementing it: the
//! assignee is L1's `PrincipalId`, work events ride L2's journal, and
//! assignment admits through L3's guard (D37–D43). A bot's work is graded
//! against its outcome — the task reads `done` only when the linked
//! deliverable passes the desk end gate, otherwise `blocked`, named (D43).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::identity::PrincipalId;
use crate::security::guard;
use crate::security::{ActionClass, Decision, Dimension, GrantStore};

/// The closed vocabulary of work objects. Only `Task` is store-owned; the
/// rest are typed references resolved live from the existing stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Task,
    Thread,
    Deliverable,
    Bot,
    Lesson,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Task => "task",
            Kind::Thread => "thread",
            Kind::Deliverable => "deliverable",
            Kind::Bot => "bot",
            Kind::Lesson => "lesson",
        }
    }

    /// Parse a kind name; an unknown name is `None` so the caller refuses
    /// it by name rather than guessing (the closed vocabulary, D37).
    pub fn parse(name: &str) -> Option<Kind> {
        match name.trim().to_lowercase().as_str() {
            "task" => Some(Kind::Task),
            "thread" => Some(Kind::Thread),
            "deliverable" => Some(Kind::Deliverable),
            "bot" => Some(Kind::Bot),
            "lesson" => Some(Kind::Lesson),
            _ => None,
        }
    }
}

/// A work object: a stable id and its kind. The id is the referenced
/// store's own key (a session id, a desk slug, a role name) for reference
/// kinds, and the minted task id for `Task`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkObject {
    pub id: String,
    pub kind: Kind,
}

impl WorkObject {
    pub fn new(id: impl Into<String>, kind: Kind) -> Self {
        WorkObject {
            id: id.into(),
            kind,
        }
    }
}

/// The closed vocabulary of edges between work objects (D39). Each names
/// the endpoint kinds it joins, so a dangling or mis-typed edge is refused
/// by name rather than recorded as a silent broken link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// task → thread: the conversation the task is worked in.
    WorksIn,
    /// task → deliverable: the work the task produces.
    Produces,
    /// task → bot: the persona assigned to work the task.
    AssignedBot,
    /// task → lesson: the learning the task yields.
    Learns,
    /// thread → deliverable: the hand from a conversation to the desk.
    HandsTo,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeKind::WorksIn => "works_in",
            EdgeKind::Produces => "produces",
            EdgeKind::AssignedBot => "assigned_bot",
            EdgeKind::Learns => "learns",
            EdgeKind::HandsTo => "hands_to",
        }
    }

    pub fn parse(name: &str) -> Option<EdgeKind> {
        match name.trim().to_lowercase().as_str() {
            "works_in" => Some(EdgeKind::WorksIn),
            "produces" => Some(EdgeKind::Produces),
            "assigned_bot" => Some(EdgeKind::AssignedBot),
            "learns" => Some(EdgeKind::Learns),
            "hands_to" => Some(EdgeKind::HandsTo),
            _ => None,
        }
    }

    /// The (from, to) kinds this edge joins.
    pub fn endpoints(self) -> (Kind, Kind) {
        match self {
            EdgeKind::WorksIn => (Kind::Task, Kind::Thread),
            EdgeKind::Produces => (Kind::Task, Kind::Deliverable),
            EdgeKind::AssignedBot => (Kind::Task, Kind::Bot),
            EdgeKind::Learns => (Kind::Task, Kind::Lesson),
            EdgeKind::HandsTo => (Kind::Thread, Kind::Deliverable),
        }
    }
}

/// A directed, typed edge between two work objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: WorkObject,
    pub to: WorkObject,
    pub kind: EdgeKind,
}

/// A task's lifecycle (D38). `done` is earned by the deliverable passing
/// the desk end gate, never asserted (D43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Working,
    Blocked,
    Done,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Working => "working",
            Status::Blocked => "blocked",
            Status::Done => "done",
        }
    }
}

/// The one store-owned work object (D38): a stable cross-session id, a
/// lifecycle status, and an assignee that is L1's principal. The `todo`
/// renderer and the per-session `Tasks` record are untouched — this is the
/// additive durable unit armada assigns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub assignee: PrincipalId,
    /// The workspace binding (a project folder) the task groups under in
    /// the surface; `None` is unbound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

/// One append-only record in the graph log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "lowercase")]
enum Record {
    Task(Task),
    Edge(Edge),
}

/// The graph log's path: `<home>/workspace/graph.jsonl`.
pub fn graph_path(home: &Path) -> PathBuf {
    home.join("workspace").join("graph.jsonl")
}

/// Ensure the graph's home exists (the journal-retention-exempt home).
pub fn ensure_graph_home(home: &Path) -> std::io::Result<PathBuf> {
    let path = graph_path(home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

/// The folded current state: tasks keyed by id (last write wins) and the
/// accumulated edges. Read fresh on every call — the log is append-only,
/// so the fold is the truth (D10).
#[derive(Debug, Default)]
pub struct Graph {
    pub tasks: BTreeMap<String, Task>,
    pub edges: Vec<Edge>,
}

/// Read and fold the graph log. A missing log is an empty graph; a torn
/// line is skipped rather than fatal (the log is a convenience record, the
/// stores it references are the truth).
pub fn read(home: &Path) -> Graph {
    let mut graph = Graph::default();
    let Ok(text) = std::fs::read_to_string(graph_path(home)) else {
        return graph;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Record>(line) else {
            continue;
        };
        match record {
            Record::Task(task) => {
                graph.tasks.insert(task.id.clone(), task);
            }
            Record::Edge(edge) => graph.edges.push(edge),
        }
    }
    graph
}

/// Append one record to the graph log, creating the home on first write.
fn append(home: &Path, record: &Record) -> Result<(), String> {
    use std::io::Write;
    let path = ensure_graph_home(home).map_err(|e| format!("the graph home: {e}"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("the graph log: {e}"))?;
    let line = serde_json::to_string(record).map_err(|e| format!("the graph record: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("the graph write: {e}"))?;
    Ok(())
}

/// Whether a work object's endpoint resolves to a real object in its store
/// (D37: references derive live, never copied). A task resolves against the
/// folded graph; the reference kinds against the session/desk/roles stores.
fn resolves(home: &Path, object: &WorkObject, graph: &Graph) -> bool {
    match object.kind {
        Kind::Task => graph.tasks.contains_key(&object.id),
        Kind::Thread => {
            crate::session::summarize(&crate::session::session_path(home, &object.id)).is_some()
        }
        Kind::Deliverable => crate::desk::load(home, &object.id).is_ok(),
        Kind::Bot => crate::roles::load(home, &object.id).is_ok(),
        // A lesson references the learning session that carries its memory
        // facts — the same source the projects surface reads lessons from.
        Kind::Lesson => crate::session::summarize(&crate::session::session_path(home, &object.id))
            .map(|summary| {
                summary.kind.as_deref() == Some(crate::session::LESSON_KIND)
                    || summary.kind.as_deref() == Some(crate::session::PRACTICE_KIND)
            })
            .unwrap_or(false),
    }
}

/// Record an edge (D39). A dangling endpoint, an endpoint kind that does
/// not match the edge, or an unknown edge kind is refused by name — never
/// a silent broken link. The endpoints are validated against the live
/// stores, so the graph cannot reference what does not exist.
pub fn link(home: &Path, from: WorkObject, to: WorkObject, kind: EdgeKind) -> Result<(), String> {
    let (want_from, want_to) = kind.endpoints();
    if from.kind != want_from || to.kind != want_to {
        return Err(format!(
            "the edge `{}` joins {}→{}, not {}→{}",
            kind.as_str(),
            want_from.as_str(),
            want_to.as_str(),
            from.kind.as_str(),
            to.kind.as_str()
        ));
    }
    let graph = read(home);
    if !resolves(home, &from, &graph) {
        return Err(format!(
            "the edge `{}` names a dangling {}: `{}` does not resolve",
            kind.as_str(),
            from.kind.as_str(),
            from.id
        ));
    }
    if !resolves(home, &to, &graph) {
        return Err(format!(
            "the edge `{}` names a dangling {}: `{}` does not resolve",
            kind.as_str(),
            to.kind.as_str(),
            to.id
        ));
    }
    append(home, &Record::Edge(Edge { from, to, kind }))
}

/// Link by name, parsing the closed vocabularies: an unknown object kind
/// or edge kind is refused by name (the CLI's seam, C0).
pub fn link_named(
    home: &Path,
    from_id: &str,
    from_kind: &str,
    to_id: &str,
    to_kind: &str,
    edge_kind: &str,
) -> Result<(), String> {
    let from_kind =
        Kind::parse(from_kind).ok_or_else(|| format!("unknown work-object kind `{from_kind}`"))?;
    let to_kind =
        Kind::parse(to_kind).ok_or_else(|| format!("unknown work-object kind `{to_kind}`"))?;
    let edge_kind =
        EdgeKind::parse(edge_kind).ok_or_else(|| format!("unknown edge kind `{edge_kind}`"))?;
    link(
        home,
        WorkObject::new(from_id, from_kind),
        WorkObject::new(to_id, to_kind),
        edge_kind,
    )
}

/// Mint a task (D38): a stable id assigned once, `working` status, and the
/// L1 principal as assignee (lane 1 resolves `owner`). The id is unique
/// across the graph log; a restart reads the same id back (C1).
pub fn create_task(
    home: &Path,
    title: &str,
    assignee: PrincipalId,
    binding: Option<String>,
) -> Result<Task, String> {
    let id = mint_task_id(home);
    let task = Task {
        id,
        title: title.to_string(),
        status: Status::Working,
        assignee,
        binding,
    };
    append(home, &Record::Task(task.clone()))?;
    Ok(task)
}

/// A task id unique against the folded graph: a timestamp prefix plus a
/// short random suffix, retried on the (vanishingly rare) collision.
fn mint_task_id(home: &Path) -> String {
    let graph = read(home);
    loop {
        let candidate = format!(
            "task-{}-{:04x}",
            crate::calib::stamp().replace([':', '-'], ""),
            rand_suffix()
        );
        if !graph.tasks.contains_key(&candidate) {
            return candidate;
        }
    }
}

fn rand_suffix() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos ^ (nanos >> 16)) as u16
}

/// Set a task's status, appending the new state (last write wins on fold).
/// A task that does not exist is refused by name.
pub fn set_status(home: &Path, task_id: &str, status: Status) -> Result<Task, String> {
    let graph = read(home);
    let mut task = graph
        .tasks
        .get(task_id)
        .cloned()
        .ok_or_else(|| format!("the task `{task_id}` does not exist"))?;
    task.status = status;
    append(home, &Record::Task(task.clone()))?;
    Ok(task)
}

/// One row of the object-keyed view (D40): the object, its workspace
/// binding derived from its store, and the surface entry the face renders.
/// The reference kinds derive live from the same stores the folder-keyed
/// union read, so the projection reproduces its rows exactly (C4).
#[derive(Debug, Clone)]
pub struct ViewRow {
    pub object: WorkObject,
    pub binding: Option<String>,
    pub entry: Value,
    /// A lesson row drawn from a practice session (vs a teaching session);
    /// the surface routes it to the practices rail. Kept off `entry` so the
    /// lesson/practice entry shape is unchanged from the folder-keyed union.
    pub practice: bool,
}

/// The object-keyed read (D40). Deliverables, lessons and practices derive
/// from the desk/session stores in the same order the folder-keyed union
/// read them; tasks come from the graph log. An optional kind filter
/// narrows the view. The binding is each object's own workspace, so the
/// surface groups by object, not by folder.
pub fn view(home: &Path, filter: Option<Kind>) -> Vec<ViewRow> {
    let mut rows: Vec<ViewRow> = Vec::new();
    let want = |kind: Kind| filter.is_none_or(|f| f == kind);

    if want(Kind::Deliverable) {
        for row in crate::desk::project_rows(home) {
            let Ok(desk) = row else { continue };
            rows.push(ViewRow {
                object: WorkObject::new(desk.slug.clone(), Kind::Deliverable),
                binding: desk.project.clone(),
                entry: json!({
                    "slug": desk.slug,
                    "name": desk.name,
                    "needs": !desk.refusals.is_empty() || !desk.number_refusals.is_empty(),
                }),
                practice: false,
            });
        }
    }

    if want(Kind::Lesson) {
        for kind in [crate::session::LESSON_KIND, crate::session::PRACTICE_KIND] {
            for summary in crate::session::list_kind(home, Some(kind)).unwrap_or_default() {
                rows.push(ViewRow {
                    object: WorkObject::new(summary.id.clone(), Kind::Lesson),
                    binding: summary.workspace.clone(),
                    entry: json!({
                        "id": summary.id,
                        "title": summary
                            .last_plan
                            .clone()
                            .or(summary.first_text.clone())
                            .unwrap_or_default(),
                        "ended": summary.ended,
                    }),
                    practice: kind == crate::session::PRACTICE_KIND,
                });
            }
        }
    }

    if want(Kind::Task) {
        let graph = read(home);
        for task in graph.tasks.values() {
            rows.push(ViewRow {
                object: WorkObject::new(task.id.clone(), Kind::Task),
                binding: task.binding.clone(),
                entry: json!({
                    "id": task.id,
                    "title": task.title,
                    "status": task.status.as_str(),
                    "assignee": task.assignee.as_str(),
                }),
                practice: false,
            });
        }
    }

    rows
}

/// Append a work-object event to the journal (D10/D43): the durable record
/// of a mutation, carrying the principal and the object ids. A journal
/// failure is named but never breaks the graph write — the journal is the
/// audit, the graph log is the state.
fn journal_event(home: &Path, kind: &str, principal: &PrincipalId, task_id: &str, payload: Value) {
    match crate::journal::Journal::open(home) {
        Ok(mut journal) => {
            if let Err(error) =
                journal.append(kind, Some(principal.as_str()), Some(task_id), payload)
            {
                eprintln!("fleet workspace: journal append: {error}");
            }
        }
        Err(error) => eprintln!("fleet workspace: journal open: {error}"),
    }
}

/// Admit an assignment through the L3 guard (D42): the assignee principal
/// must hold a `Dispatch` grant on the task's workspace scope. The owner
/// wildcard short-circuits lane 1. A refusal is named and settles a `guard`
/// journal event (D17) — the workspace does not re-implement the guard.
pub fn admit_assignment(home: &Path, task: &Task, principal: &PrincipalId) -> Result<(), String> {
    let store = GrantStore::load(home).map_err(|e| format!("the grant store: {e}"))?;
    let scope = task
        .binding
        .clone()
        .unwrap_or_else(|| "instance".to_string());
    let decision = guard::check_grant(
        principal.as_str(),
        Dimension::Workflow,
        &scope,
        ActionClass::Dispatch,
        &store,
    );
    match decision {
        Decision::Granted => Ok(()),
        Decision::Denied { name } => {
            journal_event(
                home,
                "guard",
                principal,
                &task.id,
                json!({"action": "assign", "decision": "denied", "name": name}),
            );
            Err(format!(
                "the principal `{}` is not granted to assign task `{}`: {name}",
                principal, task.id
            ))
        }
        Decision::StepUpRequired => {
            journal_event(
                home,
                "guard",
                principal,
                &task.id,
                json!({"action": "assign", "decision": "step_up"}),
            );
            Err(format!(
                "assigning task `{}` requires step-up approval",
                task.id
            ))
        }
    }
}

/// Settle a task against its deliverable's outcome (D43): the task reads
/// `done` only when the deliverable passes the desk end gate, otherwise
/// `blocked` with the named refusal. The deliverable is linked to the task
/// and a `work` journal event carries the principal, the object ids, and
/// the graded status — a bot's work is graded, never trusted by assertion.
pub fn settle_work(
    home: &Path,
    task_id: &str,
    deliverable: &str,
    principal: &PrincipalId,
) -> Result<Status, String> {
    let desk = crate::desk::load(home, deliverable)
        .map_err(|e| format!("the deliverable `{deliverable}`: {e}"))?;
    let refusal = crate::desk::end_stands(&desk);
    let status = if refusal.is_none() {
        Status::Done
    } else {
        Status::Blocked
    };
    set_status(home, task_id, status)?;
    // The link is best-effort: a deliverable that resolves is linked; the
    // settlement stands even if the edge write fails (the status is the
    // graded truth, the edge is the tissue).
    let _ = link(
        home,
        WorkObject::new(task_id, Kind::Task),
        WorkObject::new(deliverable, Kind::Deliverable),
        EdgeKind::Produces,
    );
    journal_event(
        home,
        "work",
        principal,
        task_id,
        json!({
            "task": task_id,
            "deliverable": deliverable,
            "status": status.as_str(),
            "refusal": refusal,
        }),
    );
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::PrincipalId;

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-workspace-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// C0: the closed kinds parse, and an unknown kind is refused by name.
    #[test]
    fn kinds_are_closed_and_unknown_is_named() {
        assert_eq!(Kind::parse("task"), Some(Kind::Task));
        assert_eq!(Kind::parse("THREAD"), Some(Kind::Thread));
        assert_eq!(Kind::parse("bogus"), None);
        assert_eq!(EdgeKind::parse("produces"), Some(EdgeKind::Produces));
        assert_eq!(EdgeKind::parse("bogus"), None);

        let home = scratch("kinds");
        let err = link_named(&home, "t", "task", "d", "deliverable", "bogus").unwrap_err();
        assert!(err.contains("unknown edge kind"), "{err}");
        let err = link_named(&home, "t", "widget", "d", "deliverable", "produces").unwrap_err();
        assert!(err.contains("unknown work-object kind"), "{err}");
    }

    /// C0/C2: a dangling endpoint is refused by name; a resolved edge lands.
    #[test]
    fn link_refuses_dangling_and_records_resolved() {
        let home = scratch("link");
        let task = create_task(&home, "a task", PrincipalId::owner(), None).unwrap();
        // No deliverable named `ghost` exists.
        let err = link(
            &home,
            WorkObject::new(task.id.clone(), Kind::Task),
            WorkObject::new("ghost", Kind::Deliverable),
            EdgeKind::Produces,
        )
        .unwrap_err();
        assert!(err.contains("dangling deliverable"), "{err}");
        assert!(
            read(&home).edges.is_empty(),
            "a refused edge leaves no trace"
        );

        // A real deliverable resolves and the edge lands.
        crate::desk::create(
            &home,
            "real",
            "Real",
            "the owner",
            "a claim",
            Vec::new(),
            None,
        )
        .unwrap();
        link(
            &home,
            WorkObject::new(task.id.clone(), Kind::Task),
            WorkObject::new("real", Kind::Deliverable),
            EdgeKind::Produces,
        )
        .unwrap();
        let graph = read(&home);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].kind, EdgeKind::Produces);
    }

    /// C0: a mis-typed edge (wrong endpoint kinds) is refused by name.
    #[test]
    fn link_refuses_mistyped_endpoints() {
        let home = scratch("mistyped");
        let task = create_task(&home, "a task", PrincipalId::owner(), None).unwrap();
        let err = link(
            &home,
            WorkObject::new(task.id.clone(), Kind::Task),
            WorkObject::new("also-task", Kind::Task),
            EdgeKind::Produces,
        )
        .unwrap_err();
        assert!(err.contains("joins task→deliverable"), "{err}");
    }

    /// C1: a task's id is stable across a restart (a fresh read), its
    /// status is the closed set, and its assignee is the L1 principal.
    #[test]
    fn task_identity_survives_restart() {
        let home = scratch("identity");
        let task = create_task(&home, "durable", PrincipalId::owner(), None).unwrap();
        assert_eq!(task.status, Status::Working);
        assert_eq!(task.assignee, PrincipalId::owner());
        // A fresh read (the "restart") sees the same id.
        let again = read(&home);
        assert!(again.tasks.contains_key(&task.id));
        assert_eq!(again.tasks[&task.id].title, "durable");
        // Status settles and folds to the latest.
        set_status(&home, &task.id, Status::Done).unwrap();
        assert_eq!(read(&home).tasks[&task.id].status, Status::Done);
    }

    /// C2: reference kinds resolve to the existing stores, and the graph
    /// writes no parallel object store — only tasks and edges.
    #[test]
    fn references_resolve_to_existing_stores() {
        let home = scratch("resolve");
        // A thread (session) and a deliverable (desk) in their own stores.
        let session = crate::session::Session::create(&home, "m").unwrap();
        let thread_id = session.id().to_string();
        crate::desk::create(
            &home,
            "deliv",
            "Deliv",
            "the owner",
            "a claim",
            Vec::new(),
            None,
        )
        .unwrap();
        let task = create_task(&home, "work", PrincipalId::owner(), None).unwrap();

        link(
            &home,
            WorkObject::new(task.id.clone(), Kind::Task),
            WorkObject::new(thread_id.clone(), Kind::Thread),
            EdgeKind::WorksIn,
        )
        .unwrap();
        link(
            &home,
            WorkObject::new(thread_id, Kind::Thread),
            WorkObject::new("deliv", Kind::Deliverable),
            EdgeKind::HandsTo,
        )
        .unwrap();

        // The graph log holds only task + edge records — no copied session
        // or desk object (KI-300).
        let text = std::fs::read_to_string(graph_path(&home)).unwrap();
        for line in text.lines() {
            let value: Value = serde_json::from_str(line).unwrap();
            let record = value["record"].as_str().unwrap();
            assert!(
                record == "task" || record == "edge",
                "parallel object: {line}"
            );
        }
        // The view resolves the deliverable live from the desk store.
        let rows = view(&home, Some(Kind::Deliverable));
        assert!(rows.iter().any(|r| r.object.id == "deliv"));
    }

    /// C5: an ungranted principal is refused by name and the refusal
    /// settles a `guard` journal event; the owner wildcard admits.
    #[test]
    fn assignment_is_gated_and_audited() {
        let home = scratch("gate");
        let task = create_task(&home, "gated", PrincipalId::owner(), None).unwrap();
        // The owner wildcard admits (lane 1).
        assert!(admit_assignment(&home, &task, &PrincipalId::owner()).is_ok());
        // A named stranger with no grant is refused.
        let stranger = PrincipalId::new("stranger");
        let err = admit_assignment(&home, &task, &stranger).unwrap_err();
        assert!(err.contains("not granted"), "{err}");
        // The refusal settled a guard event in the journal.
        let journal = crate::journal::Journal::open(&home).unwrap();
        let events = journal.replay(0).unwrap();
        assert!(
            events
                .iter()
                .any(|e| e.kind == "guard" && e.payload["decision"] == "denied"),
            "the denial is audited: {events:?}"
        );
    }

    /// C7: a passing deliverable settles the task `done`; a refused one
    /// settles `blocked`, named; both ride a `work` journal event.
    #[test]
    fn settlement_grades_against_the_desk_end_gate() {
        let home = scratch("settle");
        let task = create_task(&home, "graded", PrincipalId::owner(), None).unwrap();
        // A clean deliverable passes the end gate → done.
        crate::desk::create(
            &home,
            "clean",
            "Clean",
            "the owner",
            "a claim",
            Vec::new(),
            None,
        )
        .unwrap();
        let status = settle_work(&home, &task.id, "clean", &PrincipalId::owner()).unwrap();
        assert_eq!(status, Status::Done);
        assert_eq!(read(&home).tasks[&task.id].status, Status::Done);

        // A deliverable carrying a refusal blocks the task, named.
        let mut blocked = crate::desk::create(
            &home,
            "dirty",
            "Dirty",
            "the owner",
            "a claim",
            Vec::new(),
            None,
        )
        .unwrap();
        blocked.refusals.push(crate::desk::UnsourcedAssertion {
            assertion: "an unsourced claim".to_string(),
            near_miss: String::new(),
            near_miss_source: String::new(),
            overlap: 0.0,
        });
        crate::desk::save(&home, &blocked).unwrap();
        let status = settle_work(&home, &task.id, "dirty", &PrincipalId::owner()).unwrap();
        assert_eq!(status, Status::Blocked);

        let journal = crate::journal::Journal::open(&home).unwrap();
        let events = journal.replay(0).unwrap();
        let work: Vec<_> = events.iter().filter(|e| e.kind == "work").collect();
        assert_eq!(work.len(), 2, "both settlements audited");
        assert!(work.iter().any(|e| e.payload["status"] == "done"));
        assert!(work
            .iter()
            .any(|e| e.payload["status"] == "blocked" && e.payload["refusal"].as_str().is_some()));
    }
}
