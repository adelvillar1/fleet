//! The work brief: what the leader's route judgment is handed instead of
//! the literal goal line.
//!
//! The route task's own spec says its input is the router's compact
//! state; what the leader actually sent was the task string, which for a
//! plan-implementation ask is a pointer to the work, not the work — the
//! live proof being a goal that said "implement plan as designed" and a
//! judgment that read `single` over a plan whose own criteria name
//! independent surfaces. This module resolves the pointer: the session's
//! goal, the plans the session drafted, the plan's own execution
//! verdict, and the task, rendered as one brief the judge reads.

use crate::plan_graph::{read_meta, ExecutionVerdict};
use std::path::{Path, PathBuf};

/// The resolved brief: the goal the session serves, the task that asked
/// for the run, and — when the work is a plan — the plan's slug, its own
/// execution verdict, and the criteria the verdict was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBrief {
    pub goal: String,
    pub task: String,
    pub plan_slug: Option<String>,
    pub plan_path: Option<PathBuf>,
    pub plan_execution: Option<ExecutionVerdict>,
    pub criteria: Vec<String>,
    /// The non-plan goal's implied work shape, named by the sys1
    /// `goal_work` judgment — the same ruling the plan verdict carries,
    /// for a goal that is not a plan.
    pub work_shape: Option<String>,
}

impl WorkBrief {
    /// The text the sys1 `route` task judges: the compact state the
    /// task's spec declares, plus the resolved work.
    pub fn render(&self) -> String {
        let mut parts = vec![
            format!("latest_instruction: {}", self.task),
            format!("goal: {}", self.goal),
        ];
        if let Some(slug) = &self.plan_slug {
            parts.push(format!("plan: {slug}"));
        }
        if let Some(verdict) = &self.plan_execution {
            parts.push(format!("plan execution verdict: {}", verdict.as_str()));
        }
        if !self.criteria.is_empty() {
            parts.push("plan tasks and acceptance criteria:".to_string());
            for criterion in &self.criteria {
                parts.push(format!("  {criterion}"));
            }
        }
        if let Some(shape) = &self.work_shape {
            parts.push(format!("implied work: {shape}"));
        }
        parts.join("\n")
    }
}

/// Resolve the brief for one run: the session's standing goal, the plans
/// the session drafted (its log's write-class calls, most recent last),
/// and the plan the task names — falling back to the most recently
/// drafted when the ask says "the plan" without naming it. The plan
/// root is the caller's cwd in production and injected in tests, because
/// a process cwd is a global two parallel tests race.
pub fn resolve(home: &Path, plan_root: &Path, session: Option<&str>, task: &str) -> WorkBrief {
    let goal = session
        .and_then(|session| crate::gui::bridge::session_goal(home, session))
        .unwrap_or_default();
    let mut brief = WorkBrief {
        goal,
        task: task.to_string(),
        plan_slug: None,
        plan_path: None,
        plan_execution: None,
        criteria: Vec::new(),
        work_shape: None,
    };

    let drafted = drafted_plans(home, plan_root, session);
    if drafted.is_empty() {
        return brief;
    }
    // A plan named in the ask (full or partial slug) wins; otherwise the
    // most recently drafted plan is "the plan" the owner means.
    let named = drafted
        .iter()
        .rev()
        .find(|(_, slug)| {
            let ask = format!("{} {}", brief.task, brief.goal);
            ask.contains(slug.as_str())
        })
        .map(|(path, slug)| (path.clone(), slug.clone()));
    let chosen = match named {
        Some(chosen) => Some(chosen),
        None => drafted.last().cloned(),
    };
    let Some((path, slug)) = chosen else {
        return brief;
    };
    if let Ok(meta) = read_meta(&path) {
        brief.plan_execution = meta.execution;
    }
    brief.criteria = plan_criteria(&path);
    brief.plan_slug = Some(slug);
    brief.plan_path = Some(path);
    brief
}

/// The plans the session drafted: the `docs/plans/*.md` paths its
/// write-class calls carried, in log order (so the last is the most
/// recent). Only the write class counts — a session reads and edits a
/// dozen plans it did not draft.
fn drafted_plans(home: &Path, plan_root: &Path, session: Option<&str>) -> Vec<(PathBuf, String)> {
    let Some(session) = session else {
        return Vec::new();
    };
    let log = crate::session::session_path(home, session);
    let Ok(records) = crate::session::replay(&log) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, String)> = Vec::new();
    for record in &records {
        let crate::session::Record::Assistant { tool_calls, .. } = record else {
            continue;
        };
        for call in tool_calls {
            let writes = matches!(
                call.function.name.as_str(),
                "write_file" | "edit" | "apply_patch"
            );
            if !writes {
                continue;
            }
            let Ok(arguments) = serde_json::from_str::<serde_json::Value>(&call.function.arguments)
            else {
                continue;
            };
            let Some(path) = arguments.get("path").and_then(|p| p.as_str()) else {
                continue;
            };
            if !path.starts_with("docs/plans/") || !path.ends_with(".md") {
                continue;
            }
            let slug = plan_slug(path);
            if slug.is_empty() {
                continue;
            }
            let full = plan_root.join(path);
            if !full.exists() {
                continue;
            }
            if !found.iter().any(|(seen, _)| *seen == full) {
                found.push((full, slug));
            }
        }
    }
    found
}

/// The plan file's slug: the date-stamped stem without its date —
/// `2026-11-05-open-ledger-closeout.md` → `open-ledger-closeout`.
fn plan_slug(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            // Strip a leading date stamp so an ask can name the plan by
            // its slug alone.
            stem.splitn(4, '-').skip(3).collect::<Vec<_>>().join("-")
        })
        .unwrap_or_default()
}

/// The plan's tasks and acceptance criteria: the checkbox lines, verbatim,
/// in file order — what the verdict was read over and what the brief
/// shows the judge.
fn plan_criteria(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let body = trimmed
                .strip_prefix("- [ ]")
                .or_else(|| trimmed.strip_prefix("- [x]"))?;
            Some(body.trim().to_string())
        })
        .filter(|body| !body.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_carrying_a_verdict_rides_the_brief() {
        let dir = std::env::temp_dir().join(format!("fleet-brief-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = dir.join("home");
        let repo = dir.join("repo");
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        std::fs::create_dir_all(repo.join("docs/plans")).unwrap();
        let plan = repo.join("docs/plans/2026-11-06-the-work.md");
        std::fs::write(
            &plan,
            "---\nstatus: judged\ncreated: 2026-11-06\nupdated: 2026-11-06\nslug: the-work\nexecution: swarm\n---\n\n# Plan: the work\n\n## Acceptance criteria\n\n- [ ] C0 — one thing\n- [ ] C1 — another\n",
        )
        .unwrap();
        // The session drafted the plan: one write_file call in its log.
        let session = "20261106T000000Z-brief";
        let log = crate::session::session_path(&home, session);
        std::fs::write(
            &log,
            concat!(
                "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"t\",\"workspace\":\"w\"}\n",
                "{\"type\":\"assistant\",\"tool_calls\":[{\"id\":\"c1\",\"type\":\"function\",\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"docs/plans/2026-11-06-the-work.md\\\"}\"}}]}\n",
            ),
        )
        .unwrap();

        let brief = resolve(&home, &repo, Some(session), "implement plan as designed");
        assert_eq!(brief.plan_slug.as_deref(), Some("the-work"));
        assert_eq!(brief.plan_execution, Some(ExecutionVerdict::Swarm));
        assert_eq!(brief.criteria.len(), 2, "{:?}", brief.criteria);
        let rendered = brief.render();
        assert!(
            rendered.contains("plan execution verdict: swarm"),
            "{rendered}"
        );
        assert!(rendered.contains("C0 — one thing"), "{rendered}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_goal_that_names_no_plan_takes_the_most_recent_drafted() {
        let dir = std::env::temp_dir().join(format!("fleet-brief-recency-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = dir.join("home");
        let repo = dir.join("repo");
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        std::fs::create_dir_all(repo.join("docs/plans")).unwrap();
        for (name, verdict) in [
            ("2026-11-06-older.md", "single"),
            ("2026-11-06-newer.md", "swarm"),
        ] {
            let status = if verdict == "swarm" {
                "judged"
            } else {
                "draft"
            };
            let execution_line = if verdict == "swarm" {
                format!("execution: {verdict}\n")
            } else {
                String::new()
            };
            std::fs::write(
                repo.join("docs/plans").join(name),
                format!(
                    "---\nstatus: {status}\ncreated: 2026-11-06\nupdated: 2026-11-06\nslug: {name}\n{execution_line}---\n\n# Plan\n\n- [ ] C0 — the work\n"
                ),
            )
            .unwrap();
        }
        let session = "20261106T000000Z-recency";
        let log = crate::session::session_path(&home, session);
        let mut log_text = String::new();
        log_text.push_str(
            "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"t\",\"workspace\":\"w\"}\n",
        );
        for name in ["2026-11-06-older.md", "2026-11-06-newer.md"] {
            log_text.push_str(&format!(
                "{{\"type\":\"assistant\",\"tool_calls\":[{{\"id\":\"c\",\"type\":\"function\",\"function\":{{\"name\":\"write_file\",\"arguments\":\"{{\\\"path\\\":\\\"docs/plans/{name}\\\"}}\"}}}}]}}\n"
            ));
        }
        std::fs::write(&log, log_text).unwrap();

        let brief = resolve(&home, &repo, Some(session), "implement plan as designed");
        assert_eq!(
            brief.plan_slug.as_deref(),
            Some("newer"),
            "the most recent drafted plan is the ask's referent"
        );
        assert_eq!(brief.plan_execution, Some(ExecutionVerdict::Swarm));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_without_a_plan_is_the_goal_and_the_task() {
        let brief = WorkBrief {
            goal: "ship it".to_string(),
            task: "implement plan as designed".to_string(),
            plan_slug: None,
            plan_path: None,
            plan_execution: None,
            criteria: Vec::new(),
            work_shape: None,
        };
        let rendered = brief.render();
        assert!(rendered.contains("latest_instruction: implement plan as designed"));
        assert!(rendered.contains("goal: ship it"));
        assert!(!rendered.contains("plan execution verdict"));
    }
}
