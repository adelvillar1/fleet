//! The workspace spine (U43): ONE read over every store, served to the shell.
//!
//! The rebuild's contract is that the face has one unit of record — the work
//! item — and every lens is a projection of it. This module is that read.
//! It composes the stores the harness already owns; it writes nothing and
//! creates no store (U43, and KI-300's one-derivation rule).
//!
//! What it derives, and from where:
//!
//! - **work items** — a task that records `works_in` a session is the work
//!   item, and the session is its transcript; a session no task claims is a
//!   work item of its own. Status is derived: an open log is `working`, an
//!   ended one is `done` when the run completed and `blocked` when it did
//!   not (named by the run's own stop reason).
//! - **edges** — `hands_to` from the desk's own `session` pointer, the graph
//!   log's task edges, `used_skill` and `wrote_fact` from the run's own tool
//!   calls, and `shared_mind` from the armada roster. An edge here names a
//!   relation a store already records; nothing is inferred from proximity.
//! - **nodes** — the same objects, bounded (`MAX_NODES`) so a grown corpus
//!   cannot hand the browser an unbounded SVG, with the remainder counted in
//!   `omitted` rather than dropped silently (U2).
//!
//! Cost discipline. The archive on this machine is 200 MiB across 813 logs;
//! `session::list` would parse every one of them on every pivot. The spine
//! instead sorts the directory by mtime and summarizes only the newest few,
//! and the log scan that extracts tool calls carries a hard byte budget.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// The newest work items the spine carries. The connections column draws a
/// neighborhood, not the archive; the lens lists page from here. Twelve is
/// the bound that keeps one read under half a second on the owner's archive
/// (a summarize parses a whole log; 24 of them cost ~1.7s per pivot).
pub const MAX_WORK: usize = 12;
/// The most nodes the connections column receives.
pub const MAX_NODES: usize = 60;
/// How many session logs are summarized (mtime order) before the bound.
const MAX_SESSIONS_SCANNED: usize = 14;
/// The total bytes the tool-call scan may read, across all sessions.
const SCAN_BUDGET: u64 = 8 * 1024 * 1024;
/// The most skill/fact names one work item contributes to the graph.
const MAX_NAMES_PER_WORK: usize = 6;

/// What a run's own log says about it, beyond the summary the archive keeps.
#[derive(Debug, Default, Clone)]
pub struct RunFacts {
    /// The skills the run actually invoked (`use_skill`), by name.
    pub skills: Vec<String>,
    /// The facts the run actually wrote (`remember`), by name.
    pub facts: Vec<String>,
    /// The end record's stop reason, when the log carries one.
    pub stop: Option<String>,
    /// The end record's turn count.
    pub turns: Option<u64>,
}

/// Read at most `budget` bytes of a file — the whole file when it fits, its
/// head and tail when it does not. A truncated middle can only lose tool
/// calls, never invent one; a line cut at either seam fails to parse and is
/// skipped.
fn read_bounded(path: &Path, budget: u64) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len <= budget {
        let mut text = String::new();
        file.read_to_string(&mut text).ok()?;
        return Some(text);
    }
    let half = (budget / 2) as usize;
    let mut head = vec![0u8; half];
    file.read_exact(&mut head).ok()?;
    let mut text = String::from_utf8_lossy(&head).into_owned();
    file.seek(SeekFrom::End(-(half as i64))).ok()?;
    let mut tail = vec![0u8; half];
    file.read_exact(&mut tail).ok()?;
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&tail));
    Some(text)
}

/// The name a tool call carries, from whichever key the tool's schema uses.
fn call_name(arguments: &str) -> Option<String> {
    let value: Value = serde_json::from_str(arguments).ok()?;
    for key in ["name", "skill", "fact", "title"] {
        if let Some(name) = value.get(key).and_then(|v| v.as_str()) {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn push_unique(list: &mut Vec<String>, name: String) {
    if list.len() < MAX_NAMES_PER_WORK && !list.contains(&name) {
        list.push(name);
    }
}

/// Scan one run's log for what it did: the skills it invoked, the facts it
/// wrote, and how it ended. Bounded by `budget`.
pub fn scan_run(path: &Path, budget: u64) -> RunFacts {
    let mut facts = RunFacts::default();
    let Some(text) = read_bounded(path, budget) else {
        return facts;
    };
    for line in text.lines() {
        if !line.starts_with('{') {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match record.get("type").and_then(|v| v.as_str()) {
            Some("assistant") => {
                let Some(calls) = record.get("tool_calls").and_then(|v| v.as_array()) else {
                    continue;
                };
                for call in calls {
                    let function = call.get("function");
                    let name = function
                        .and_then(|f| f.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let arguments = function
                        .and_then(|f| f.get("arguments"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if name == crate::tools::USE_SKILL_TOOL {
                        if let Some(skill) = call_name(arguments) {
                            push_unique(&mut facts.skills, skill);
                        }
                    } else if name == crate::tools::REMEMBER_TOOL {
                        if let Some(fact) = call_name(arguments) {
                            push_unique(&mut facts.facts, fact);
                        }
                    }
                }
            }
            Some("end") => {
                facts.stop = record
                    .get("stop_reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                facts.turns = record.get("turns").and_then(|v| v.as_u64());
            }
            _ => {}
        }
    }
    facts
}

/// The newest sessions, without parsing the whole archive: files by mtime,
/// then only the ones this read can use.
fn recent_sessions(home: &Path, limit: usize) -> Vec<crate::session::SessionSummary> {
    let dir = crate::home::sessions_dir(home);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "jsonl").unwrap_or(false))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();
    files.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    files
        .into_iter()
        .take(limit)
        .filter_map(|(_, path)| crate::session::summarize(&path))
        .collect()
}

/// A scheduled run's plain title: the job's own name, not the bracket
/// sentence the scheduler writes into the first message.
fn plain_title(summary: &crate::session::SessionSummary) -> String {
    let first = summary
        .first_text
        .as_deref()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty());
    if summary.kind.as_deref() == Some("scheduled") {
        if let Some(text) = first {
            if let Some(start) = text.find("job `") {
                let rest = &text[start + 5..];
                if let Some(end) = rest.find('`') {
                    let job = rest[..end].trim();
                    if !job.is_empty() {
                        return job.to_string();
                    }
                }
            }
        }
    }
    first
        .map(|text| text.to_string())
        .unwrap_or_else(|| summary.id.clone())
}

/// The one read. `memories` is the corpus root the face already resolved;
/// `driving` is the session the face is currently running, if any.
pub fn read(home: &Path, memories: &Path, driving: Option<&str>) -> Value {
    let graph = crate::workspace::read(home);
    let desks = crate::desk::projects(home);
    let roles = crate::roles::load_all(home);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or_default();
    let roster = crate::armada::roster::load(home);

    // The desk's own pointer is the artifact→transcript edge (desk.rs:203).
    let mut desks_by_session: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for desk in &desks {
        if let Some(session) = desk.session.as_deref() {
            desks_by_session
                .entry(session.to_string())
                .or_default()
                .push(desk.slug.clone());
        }
    }

    // A task that records its workspace session owns that transcript.
    let mut claimed_session: BTreeMap<String, String> = BTreeMap::new();
    for edge in &graph.edges {
        if edge.kind == crate::workspace::EdgeKind::WorksIn {
            claimed_session
                .entry(edge.to.id.clone())
                .or_insert_with(|| edge.from.id.clone());
        }
    }

    let sessions = recent_sessions(home, MAX_SESSIONS_SCANNED);
    let mut budget = SCAN_BUDGET;
    let mut run_facts: BTreeMap<String, RunFacts> = BTreeMap::new();
    for summary in &sessions {
        let path = crate::session::session_path(home, &summary.id);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > budget {
            // The budget ran out: the run still appears, its tool-call edges
            // simply are not claimed (never invented).
            continue;
        }
        budget -= size;
        run_facts.insert(summary.id.clone(), scan_run(&path, size.max(1)));
    }

    // ---- work items -------------------------------------------------------
    let mut work: Vec<Value> = Vec::new();
    let mut nodes: Vec<Value> = Vec::new();
    let mut edges: Vec<Value> = Vec::new();
    let mut omitted = 0usize;

    let push_node = |nodes: &mut Vec<Value>,
                     omitted: &mut usize,
                     id: &str,
                     kind: &str,
                     label: &str,
                     status: Option<&str>,
                     live: bool,
                     meta: Option<String>| {
        if nodes.len() >= MAX_NODES {
            *omitted += 1;
            return;
        }
        nodes.push(json!({
            "id": id, "kind": kind, "label": label,
            "status": status, "live": live, "meta": meta,
        }));
    };

    let mut seen_nodes: BTreeSet<String> = BTreeSet::new();
    let mut add_node = |nodes: &mut Vec<Value>,
                        omitted: &mut usize,
                        id: &str,
                        kind: &str,
                        label: &str,
                        status: Option<&str>,
                        live: bool,
                        meta: Option<String>| {
        let key = format!("{kind}:{id}");
        if !seen_nodes.insert(key) {
            return;
        }
        push_node(nodes, omitted, id, kind, label, status, live, meta);
    };

    let push_edge = |edges: &mut Vec<Value>, from: &str, to: &str, kind: &str| {
        if edges.len() >= MAX_NODES * 2 {
            return;
        }
        edges.push(json!({ "from": from, "to": to, "kind": kind, "hot": false }));
    };

    // Tasks first: they are the work items the graph already names.
    for (id, task) in &graph.tasks {
        if work.len() >= MAX_WORK {
            omitted += 1;
            continue;
        }
        let transcript = graph
            .edges
            .iter()
            .find(|edge| edge.kind == crate::workspace::EdgeKind::WorksIn && edge.from.id == *id)
            .map(|edge| edge.to.id.clone());
        let facts = transcript
            .as_ref()
            .and_then(|session| run_facts.get(session))
            .cloned()
            .unwrap_or_default();
        let artifacts = transcript
            .as_ref()
            .map(|session| desks_by_session.get(session).cloned().unwrap_or_default())
            .unwrap_or_default();
        work.push(json!({
            "id": id,
            "kind": "task",
            "title": task.title,
            "status": task.status.as_str(),
            "transcript": transcript,
            "owner": task.assignee.as_str(),
            "machine": Value::Null,
            "binding": task.binding,
            "artifacts": artifacts,
            "skills": facts.skills,
            "facts": facts.facts,
        }));
        add_node(
            &mut nodes,
            &mut omitted,
            id,
            "task",
            &task.title,
            Some(task.status.as_str()),
            task.status == crate::workspace::Status::Working,
            None,
        );
    }

    // Then the sessions no task claims: the work that happened anyway.
    for summary in &sessions {
        if work.len() >= MAX_WORK {
            omitted += 1;
            continue;
        }
        if claimed_session.contains_key(&summary.id) {
            continue;
        }
        let facts = run_facts.get(&summary.id).cloned().unwrap_or_default();
        let status = if !summary.ended {
            "working"
        } else if facts.stop.as_deref() == Some("completed") {
            "done"
        } else {
            "blocked"
        };
        let title = plain_title(summary);
        let artifacts = desks_by_session
            .get(&summary.id)
            .cloned()
            .unwrap_or_default();
        work.push(json!({
            "id": summary.id,
            "kind": "session",
            "title": title,
            "status": status,
            "transcript": summary.id,
            "owner": Value::Null,
            "machine": Value::Null,
            "binding": summary.workspace,
            "artifacts": artifacts,
            "skills": facts.skills,
            "facts": facts.facts,
            "turns": facts.turns,
            "created": summary.created,
        }));
        add_node(
            &mut nodes,
            &mut omitted,
            &summary.id,
            "thread",
            &title,
            Some(status),
            driving == Some(summary.id.as_str()),
            None,
        );
        // The edges this run actually proves.
        for artifact in &artifacts {
            push_edge(&mut edges, &summary.id, artifact, "hands_to");
        }
        for skill in &facts.skills {
            add_node(
                &mut nodes,
                &mut omitted,
                skill,
                "skill",
                skill,
                None,
                false,
                None,
            );
            push_edge(&mut edges, &summary.id, skill, "used_skill");
        }
        for fact in &facts.facts {
            // Only a fact the corpus still holds: a remembered name that was
            // superseded or removed is not a node the graph should claim.
            if !memories.join(format!("{fact}.md")).exists() {
                continue;
            }
            add_node(
                &mut nodes,
                &mut omitted,
                fact,
                "fact",
                fact,
                None,
                false,
                None,
            );
            push_edge(&mut edges, &summary.id, fact, "wrote_fact");
        }
    }

    // The artifacts, lessons, bots and machines the rest of the shell needs.
    for desk in &desks {
        add_node(
            &mut nodes,
            &mut omitted,
            &desk.slug,
            "deliverable",
            &desk.name,
            None,
            false,
            None,
        );
    }
    // Lessons come from the summaries already in hand: `workspace::view`
    // would re-walk the whole archive (813 logs) to find the same rows, and
    // that walk is what made this read 1.7s per pivot.
    for summary in &sessions {
        let kind = summary.kind.as_deref();
        if kind != Some(crate::session::LESSON_KIND) && kind != Some(crate::session::PRACTICE_KIND)
        {
            continue;
        }
        let label = summary
            .first_text
            .as_deref()
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .unwrap_or(summary.id.as_str())
            .to_string();
        add_node(
            &mut nodes,
            &mut omitted,
            &summary.id,
            if kind == Some(crate::session::PRACTICE_KIND) {
                "practice"
            } else {
                "lesson"
            },
            &label,
            None,
            false,
            None,
        );
    }
    for role in &roles {
        add_node(
            &mut nodes,
            &mut omitted,
            &format!("bot:{}", role.name),
            "bot",
            &role.name,
            None,
            false,
            None,
        );
    }
    let mut instance_ids: Vec<String> = Vec::new();
    for instance in roster.instances.values() {
        let id = format!("instance:{}", instance.sid);
        instance_ids.push(id.clone());
        add_node(
            &mut nodes,
            &mut omitted,
            &id,
            "instance",
            &instance.sid,
            None,
            instance.live(now),
            Some(format!("v{}", instance.version)),
        );
    }
    // The task graph's own edges keep their place in the document.
    for edge in &graph.edges {
        push_edge(&mut edges, &edge.from.id, &edge.to.id, edge.kind.as_str());
    }
    for id in &instance_ids {
        push_edge(&mut edges, id, "install", "shared_mind");
    }
    if nodes.is_empty() || !instance_ids.is_empty() {
        nodes.insert(
            0,
            json!({
                "id": "install", "kind": "install", "label": "this machine",
                "status": null, "live": true, "meta": null,
            }),
        );
    }

    // ---- the compatibility keys the current face still reads ---------------
    let tasks: Vec<Value> = graph
        .tasks
        .values()
        .map(|task| {
            json!({
                "id": task.id,
                "title": task.title,
                "status": task.status.as_str(),
                "assignee": task.assignee.as_str(),
                "binding": task.binding,
            })
        })
        .collect();
    let deliverables: Vec<Value> = desks
        .iter()
        .map(|desk| {
            json!({
                "id": desk.slug,
                "name": desk.name,
                "needs": !desk.refusals.is_empty(),
                "binding": desk.project,
                "session": desk.session,
                "stage": desk
                    .stages
                    .iter()
                    .find(|(_, state)| matches!(state, crate::desk::StageState::Running))
                    .map(|(stage, _)| stage.name().to_string()),
            })
        })
        .collect();
    let threads: Vec<Value> = work
        .iter()
        .filter_map(|item| {
            let id = item.get("transcript").and_then(|v| v.as_str())?;
            Some(json!({
                "id": id,
                "label": item.get("title").cloned().unwrap_or_else(|| json!(id)),
                "status": item.get("status").cloned(),
            }))
        })
        .collect();
    let instances: Vec<Value> = roster
        .instances
        .values()
        .map(|instance| {
            let grade = crate::armada::control::grade(home, &instance.sid);
            json!({
                "sid": instance.sid,
                "live": instance.live(now),
                "calibrated": grade.calibrated,
                "reliability": grade.reliability,
            })
        })
        .collect();

    json!({
        "type": "workspace",
        "work": work,
        "nodes": nodes,
        "edges": edges,
        "tasks": tasks,
        "deliverables": deliverables,
        "threads": threads,
        "instances": instances,
        "bounded": { "work": MAX_WORK, "nodes": MAX_NODES },
        "omitted": omitted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "fleet-spine-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|span| span.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(root.join("sessions")).unwrap();
        root
    }

    #[test]
    fn a_scheduled_runs_title_is_the_job_not_the_bracket_sentence() {
        let root = scratch("title");
        let path = root.join("sessions").join("20260101T000000Z-aaaa.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"type":"start","v":1,"model":"m","created":"2026-01-01T00:00:00Z","kind":"scheduled","workspace":"/"}"#,
                r#"{"type":"user","content":"[scheduled run — job `corpus-refresh-sys1` at 2026-01-01T00:00Z]"}"#,
                r#"{"type":"end","stop_reason":"completed","turns":3}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let summary = crate::session::summarize(&path).expect("summarize");
        assert_eq!(plain_title(&summary), "corpus-refresh-sys1");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn the_scan_reads_only_the_calls_the_run_made() {
        let root = scratch("scan");
        let path = root.join("sessions").join("20260101T000000Z-bbbb.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"type":"start","v":1,"model":"m","created":"2026-01-01T00:00:00Z","kind":null,"workspace":"/"}"#,
                r#"{"type":"assistant","content":"","tool_calls":[{"id":"1","type":"function","function":{"name":"use_skill","arguments":"{\"name\":\"project-methodology\"}"}},{"id":"2","type":"function","function":{"name":"remember","arguments":"{\"name\":\"a-fact-that-was-written\"}"}},{"id":"3","type":"function","function":{"name":"bash","arguments":"{}"}}]}"#,
                r#"{"type":"end","stop_reason":"completed","turns":7}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let facts = scan_run(&path, 1 << 20);
        assert_eq!(facts.skills, vec!["project-methodology".to_string()]);
        assert_eq!(facts.facts, vec!["a-fact-that-was-written".to_string()]);
        assert_eq!(facts.stop.as_deref(), Some("completed"));
        assert_eq!(facts.turns, Some(7));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_run_with_no_calls_claims_no_edges() {
        let root = scratch("quiet");
        let path = root.join("sessions").join("20260101T000000Z-cccc.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"type":"start","v":1,"model":"m","created":"2026-01-01T00:00:00Z","kind":null,"workspace":"/"}"#,
                r#"{"type":"assistant","content":"no tools here","tool_calls":[]}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let facts = scan_run(&path, 1 << 20);
        assert!(facts.skills.is_empty());
        assert!(facts.facts.is_empty());
        assert!(facts.stop.is_none());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn the_spine_serves_a_session_as_a_work_item() {
        let root = scratch("spine");
        let path = root.join("sessions").join("20260101T000000Z-dddd.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"type":"start","v":1,"model":"m","created":"2026-01-01T00:00:00Z","kind":null,"workspace":"/work"}"#,
                r#"{"type":"user","content":"build the thing"}"#,
                r#"{"type":"assistant","content":"","tool_calls":[{"id":"1","type":"function","function":{"name":"use_skill","arguments":"{\"name\":\"project-methodology\"}"}}]}"#,
                r#"{"type":"end","stop_reason":"completed","turns":2}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let home = root.join("home");
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        std::fs::copy(
            &path,
            home.join("sessions").join("20260101T000000Z-dddd.jsonl"),
        )
        .unwrap();
        let corpus = root.join("memories");
        std::fs::create_dir_all(&corpus).unwrap();
        let spine = read(&home, &corpus, None);
        let work = spine.get("work").and_then(|v| v.as_array()).expect("work");
        assert_eq!(work.len(), 1);
        assert_eq!(work[0]["title"], "build the thing");
        assert_eq!(work[0]["status"], "done");
        let edges = spine
            .get("edges")
            .and_then(|v| v.as_array())
            .expect("edges");
        assert!(
            edges.iter().any(|edge| edge["kind"] == "used_skill"),
            "the skill the run invoked is an edge: {edges:?}"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
