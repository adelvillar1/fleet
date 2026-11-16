//! The cleanup a release cut or a deployment runs.
//!
//! A run the resource contract curtailed queues the files it wrote instead
//! of deleting them while its diagnosis window is open — the owner's
//! 2026-11-16 ruling (KI-553): "delete on release cut and/or deployment".
//! The queue is `<home>/cleanup/pending.jsonl`, one line per curtailed run,
//! written by `Agent::queue_cleanup` from the run's own transcript.
//!
//! The deletion is bounded three ways: only files the run **wrote through
//! `write_file`** (an `edit` changes a file that already existed, and
//! deleting it would destroy the owner's copy), only paths **inside the
//! folder the run was granted**, and only what still exists. A path outside
//! its folder is refused by name, never deleted.

use std::path::Path;

/// What one cleanup pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    /// The curtailed runs the queue named.
    pub runs: usize,
    /// Files deleted.
    pub deleted: usize,
    /// Queued files that were already gone.
    pub gone: usize,
    /// Files refused (outside their folder, or unremovable).
    pub refused: usize,
    /// One line per deleted file, for the verb to print.
    pub deleted_paths: Vec<String>,
}

impl Report {
    /// The verb's summary line.
    pub fn summary(&self) -> String {
        format!(
            "cleanup: {} file(s) deleted, {} already gone, {} refused, from {} curtailed run(s)",
            self.deleted, self.gone, self.refused, self.runs
        )
    }
}

/// The files a run wrote through `write_file`, resolved against its
/// folder, read off the run's own transcript: an assistant call whose
/// result did not carry the `error:` prefix succeeded, and only those
/// count. `edit` and `apply_patch` are deliberately absent — they change a
/// file that already existed, and deleting it would destroy the owner's
/// copy, not the run's debris.
pub fn written_files(messages: &[crate::api::Message], folder: &Path) -> Vec<std::path::PathBuf> {
    use crate::api::Role;
    let mut answered: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for message in messages.iter().filter(|one| one.role == Role::Tool) {
        let Some(id) = &message.tool_call_id else {
            continue;
        };
        let failed = message
            .content
            .as_deref()
            .is_some_and(|text| text.trim_start().starts_with("error:"));
        if !failed {
            answered.insert(id.clone());
        }
    }
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    for message in messages.iter().filter(|one| one.role == Role::Assistant) {
        for call in &message.tool_calls {
            if call.function.name != "write_file" || !answered.contains(&call.id) {
                continue;
            }
            let Ok(arguments) = serde_json::from_str::<serde_json::Value>(&call.function.arguments)
            else {
                continue;
            };
            let Some(raw) = arguments.get("path").and_then(|path| path.as_str()) else {
                continue;
            };
            let path = Path::new(raw);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                folder.join(path)
            };
            if resolved.starts_with(folder) && !out.contains(&resolved) {
                out.push(resolved);
            }
        }
    }
    out
}

/// The queue's path under `home`.
pub fn queue_path(home: &Path) -> std::path::PathBuf {
    home.join("cleanup").join("pending.jsonl")
}

/// Run the cleanup over `home`'s queue. The queue is consumed whatever the
/// outcome: a refusal was reported, and keeping the line would loop it at
/// every cut.
pub fn run(home: &Path) -> Report {
    let mut report = Report::default();
    let path = queue_path(home);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return report;
    };
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        report.runs += 1;
        let folder = entry
            .get("folder")
            .and_then(|one| one.as_str())
            .unwrap_or_default();
        for file in entry
            .get("files")
            .and_then(|one| one.as_array())
            .into_iter()
            .flatten()
            .filter_map(|one| one.as_str())
        {
            let target = Path::new(file);
            if folder.is_empty() || !target.starts_with(folder) {
                report.refused += 1;
                continue;
            }
            match std::fs::remove_file(target) {
                Ok(()) => {
                    report.deleted += 1;
                    report.deleted_paths.push(file.to_string());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => report.gone += 1,
                Err(_) => report.refused += 1,
            }
        }
    }
    let _ = std::fs::remove_file(&path);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-cleanup-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cleanup")).unwrap();
        dir
    }

    #[test]
    fn the_queue_deletes_inside_its_folder_and_refuses_outside() {
        let home = home("inside");
        let folder = home.join("project");
        std::fs::create_dir_all(&folder).unwrap();
        let inside = folder.join("made-by-the-run.txt");
        std::fs::write(&inside, b"debris").unwrap();
        let outside = home.join("keep-me.txt");
        std::fs::write(&outside, b"the owner's").unwrap();
        let entry = serde_json::json!({
            "at": "2026-11-16T00:00:00Z",
            "session": "s-1",
            "folder": folder.display().to_string(),
            "files": [
                inside.display().to_string(),
                outside.display().to_string(),
                folder.join("already-gone.txt").display().to_string()
            ],
            "why": "curtailed by the resource contract",
        });
        std::fs::write(queue_path(&home), format!("{entry}\n")).unwrap();
        let report = run(&home);
        assert_eq!(report.runs, 1);
        assert_eq!(report.deleted, 1, "{report:?}");
        assert_eq!(report.gone, 1, "{report:?}");
        assert_eq!(report.refused, 1, "the outside file is refused");
        assert!(!inside.exists(), "the created file is gone");
        assert!(outside.exists(), "the owner's file stands");
        assert!(!queue_path(&home).exists(), "the queue is consumed");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn only_a_successful_write_file_inside_the_folder_counts() {
        use crate::api::{FunctionCall, Message, Role, ToolCall};
        let folder = Path::new("/project");
        let call = |id: &str, name: &str, path: &str| ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: format!("{{\"path\":\"{path}\"}}"),
            },
        };
        let answer = |id: &str, text: &str| Message {
            role: Role::Tool,
            content: Some(text.to_string()),
            tool_calls: Vec::new(),
            tool_call_id: Some(id.to_string()),
            images: Vec::new(),
        };
        let asked = |calls: Vec<ToolCall>| Message {
            role: Role::Assistant,
            content: None,
            tool_calls: calls,
            tool_call_id: None,
            images: Vec::new(),
        };
        let messages = vec![
            asked(vec![
                call("ok-rel", "write_file", "made.txt"),
                call("ok-abs", "write_file", "/project/abs.txt"),
                call("refused", "write_file", "denied.txt"),
                call("out", "write_file", "/elsewhere/file.txt"),
                call("edited", "edit", "existing.txt"),
            ]),
            answer("ok-rel", "wrote 6 bytes"),
            answer("ok-abs", "wrote 4 bytes"),
            answer("refused", "error: the owner refused this write"),
            answer("out", "wrote 8 bytes"),
            answer("edited", "changed 3 lines"),
        ];
        let files = written_files(&messages, folder);
        assert_eq!(
            files,
            vec![
                std::path::PathBuf::from("/project/made.txt"),
                std::path::PathBuf::from("/project/abs.txt")
            ],
            "{files:?}"
        );
    }

    #[test]
    fn an_empty_or_missing_queue_is_a_no_op() {
        let home = home("empty");
        let report = run(&home);
        assert_eq!(report, Report::default());
        let _ = std::fs::remove_dir_all(&home);
    }
}
