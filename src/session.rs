//! Append-only JSONL session logs at `~/.fleet/sessions/<id>.jsonl`.
//!
//! Format version 1: the first line is always `start`, and an orderly stop
//! appends `end`; a log without `end` was interrupted. Every message the
//! model sees or produces is logged as it happens, so a replayed log
//! reproduces the conversation exactly.

use crate::api::{Message, Role, ToolCall};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bump only when existing logs cannot be read safely under the new code;
/// same-version additions require tolerant readers.
pub const SESSION_FORMAT_VERSION: u32 = 1;

/// One line of a session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    /// First line of every log; pins the format version and the model.
    Start {
        v: u32,
        model: String,
        created: String,
    },
    User {
        content: String,
    },
    Assistant {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
    },
    /// One executed or denied tool call, with exactly the output text the
    /// model received.
    ToolResult {
        call_id: String,
        name: String,
        output: String,
        truncated: bool,
        approved: bool,
    },
    /// The tools one lane brought into a run, recorded the moment the
    /// lane joined. The tool declarations are dynamic now — a lane's
    /// tools enter the model's tool set mid-run — so this record is what
    /// makes the request the model saw reconstructable from the log
    /// (hard rule 5). Bookkeeping, like `start` and `end`: it replays to
    /// no message.
    LaneTools {
        lane: String,
        tools: Vec<crate::tools::LaneTool>,
    },
    /// An orderly stop; a log without it was interrupted.
    End {
        stop_reason: String,
        turns: u64,
    },
}

/// An open session log in append mode.
pub struct Session {
    path: PathBuf,
    log: File,
}

impl Session {
    /// Create a new session under `sessions_dir(home)`, writing the `start`
    /// record. The id is a UTC stamp plus a uniqueness suffix from the
    /// clock's sub-second bits and the process id.
    pub fn create(home: &Path, model: &str) -> io::Result<Session> {
        let dir = crate::home::ensure_sessions_dir(home)?;
        let id = format!("{}-{:06x}", compact_utc_stamp(SystemTime::now())?, unique_suffix());
        let path = dir.join(format!("{id}.jsonl"));
        let log = OpenOptions::new().create_new(true).append(true).open(&path)?;
        let mut session = Session { path, log };
        let created = iso_utc_stamp(SystemTime::now())?;
        session.record(&Record::Start {
            v: SESSION_FORMAT_VERSION,
            model: model.to_string(),
            created,
        })?;
        Ok(session)
    }

    /// Open an existing log in append mode.
    pub fn open(path: &Path) -> io::Result<Session> {
        let log = OpenOptions::new().append(true).open(path)?;
        Ok(Session { path: path.to_path_buf(), log })
    }

    /// This log's path; the file stem is the session id.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The session id (file stem).
    pub fn id(&self) -> &str {
        self.path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("")
    }

    /// Append one record as one JSON line and flush it, so an interrupted
    /// run loses nothing already reported.
    pub fn record(&mut self, record: &Record) -> io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        line.push('\n');
        self.log.write_all(line.as_bytes())?;
        self.log.flush()
    }
}

/// Read every record of a log. Unknown record types fail loud: reading an
/// unreadable format silently would corrupt replays.
pub fn replay(path: &Path) -> io::Result<Vec<Record>> {
    let text = fs::read_to_string(path)?;
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        records.push(serde_json::from_str(line).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} line {}: {error}", path.display(), index + 1),
            )
        })?);
    }
    Ok(records)
}

/// A summary row for `fleet sessions`.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub model: String,
    pub created: String,
    /// True when the log's last record is `end`.
    pub ended: bool,
    pub path: PathBuf,
}

/// List every log in `sessions_dir(home)`, oldest first; logs that fail to
/// parse are reported as entries with an `unreadable` model rather than
/// hiding them.
pub fn list(home: &Path) -> io::Result<Vec<SessionSummary>> {
    let dir = crate::home::sessions_dir(home);
    let mut entries: Vec<SessionSummary> = Vec::new();
    let read_dir = match fs::read_dir(&dir) {
        Ok(read_dir) => read_dir,
        // No sessions yet.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(entries),
        Err(error) => return Err(error),
    };
    for entry in read_dir {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string();
        let records = match replay(&path) {
            Ok(records) if !records.is_empty() => records,
            Ok(_) => continue,
            Err(_) => {
                entries.push(SessionSummary {
                    id,
                    model: "unreadable".to_string(),
                    created: String::new(),
                    ended: false,
                    path,
                });
                continue;
            }
        };
        let (model, created) = match &records[0] {
            Record::Start { model, created, .. } => (model.clone(), created.clone()),
            _ => (String::new(), String::new()),
        };
        let ended = matches!(records.last(), Some(Record::End { .. }));
        entries.push(SessionSummary { id, model, created, ended, path });
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(entries)
}

/// Rebuild the wire messages a log represents: `start`/`end` are bookkeeping,
/// everything else is model-visible.
///
/// `lane_tools` is bookkeeping too: it names the tools a lane contributed
/// to a request, not anything the model said, so it replays to no
/// message. It is still recorded, because the declarations it describes
/// are dynamic and only the log can reconstruct them.
pub fn messages(records: &[Record]) -> Vec<Message> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::User { content } => Some(Message::text(Role::User, content.clone())),
            Record::Assistant { content, tool_calls } => Some(Message {
                role: Role::Assistant,
                content: content.clone(),
                tool_calls: tool_calls.clone(),
                tool_call_id: None,
            }),
            Record::ToolResult { call_id, output, .. } => {
                Some(Message::tool_result(call_id.clone(), output.clone()))
            }
            Record::Start { .. } | Record::End { .. } | Record::LaneTools { .. } => None,
        })
        .collect()
}

/// Resolve a `--session` id to its log path under `home`.
pub fn session_path(home: &Path, id: &str) -> PathBuf {
    crate::home::sessions_dir(home).join(format!("{}.jsonl", id))
}

/// `YYYYMMDDTHHMMSSZ` from the wall clock.
fn compact_utc_stamp(at: SystemTime) -> io::Result<String> {
    let (year, month, day, hour, minute, second) = civil_utc(at)?;
    Ok(format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z"))
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the wall clock.
pub(crate) fn iso_utc_stamp(at: SystemTime) -> io::Result<String> {
    let (year, month, day, hour, minute, second) = civil_utc(at)?;
    Ok(format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"))
}

/// Split the wall clock into UTC civil fields.
fn civil_utc(at: SystemTime) -> io::Result<(i64, u32, u32, u32, u32, u32)> {
    let elapsed = at.duration_since(UNIX_EPOCH).map_err(|error| {
        io::Error::new(io::ErrorKind::InvalidData, format!("clock before the epoch: {error}"))
    })?;
    let days = (elapsed.as_secs() / 86_400) as i64;
    let seconds_of_day = elapsed.as_secs() % 86_400;
    let (year, month, day) = civil_from_days(days);
    Ok((
        year,
        month,
        day,
        (seconds_of_day / 3_600) as u32,
        ((seconds_of_day % 3_600) / 60) as u32,
        (seconds_of_day % 60) as u32,
    ))
}

/// Days since 1970-01-01 to `(year, month, day)`; Howard Hinnant's civil
/// calendar algorithm, valid over the full range that matters here.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Low bits of the clock's nanoseconds mixed with the pid: enough to keep
/// same-second session ids distinct.
fn unique_suffix() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or(0);
    nanos.rotate_left(7) ^ (std::process::id() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-session-test-{tag}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn create_records_start_and_round_trips() {
        let home = temp_home("create");
        let mut session = Session::create(&home, "deepseek-chat").unwrap();
        session.record(&Record::User { content: "hello".into() }).unwrap();
        session
            .record(&Record::End { stop_reason: "completed".into(), turns: 1 })
            .unwrap();
        let records = replay(&session.path()).unwrap();
        assert_eq!(records.len(), 3);
        assert!(matches!(
            &records[0],
            Record::Start { v: 1, model, .. } if model == "deepseek-chat"
        ));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn messages_skip_bookkeeping_records() {
        let records = vec![
            Record::Start { v: 1, model: "m".into(), created: "c".into() },
            Record::User { content: "hi".into() },
            Record::Assistant { content: Some("running".into()), tool_calls: vec![] },
            Record::ToolResult {
                call_id: "call-1".into(),
                name: "bash".into(),
                output: "out".into(),
                truncated: false,
                approved: true,
            },
            Record::End { stop_reason: "completed".into(), turns: 1 },
        ];
        let messages = messages(&records);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::Tool);
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("call-1"));
    }

    /// One `lane_tools` record, as the agent writes it when a lane joins.
    fn lane_tools_record() -> Record {
        Record::LaneTools {
            lane: "sem1".to_string(),
            tools: vec![crate::tools::LaneTool {
                name: "mcp__sem1__probe".to_string(),
                lane_tool: "sem1_probe".to_string(),
                description: "Capability and reachability probe".to_string(),
                input_schema: serde_json::json!({ "type": "object" }),
            }],
        }
    }

    #[test]
    fn a_lane_tools_record_replays_as_bookkeeping_not_conversation() {
        // The vocabulary grew mid-format-version: a log carrying the new
        // record must still replay, and the record must not become a
        // message — it describes the tool set, not anything the model
        // said.
        let records = vec![
            Record::Start { v: 1, model: "m".into(), created: "c".into() },
            Record::User { content: "probe it".into() },
            lane_tools_record(),
            Record::Assistant { content: None, tool_calls: Vec::new() },
            Record::End { stop_reason: "completed".into(), turns: 1 },
        ];
        let messages = messages(&records);
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn a_lane_tools_line_round_trips_through_the_log_file() {
        // The record's own line, written the way the log writes it and
        // read back by a reader that only knows the vocabulary: the tag
        // is `lane_tools`, and the lane, the namespaced name, the
        // description and the schema all survive.
        let home = temp_home("lane-tools");
        let mut session = Session::create(&home, "m").unwrap();
        session.record(&lane_tools_record()).unwrap();
        let line = std::fs::read_to_string(session.path()).unwrap();
        let recorded = line.lines().nth(1).expect("the lane_tools line");
        assert!(recorded.starts_with("{\"type\":\"lane_tools\""), "{recorded}");
        assert!(recorded.contains("\"lane\":\"sem1\""), "{recorded}");
        assert!(recorded.contains("\"name\":\"mcp__sem1__probe\""), "{recorded}");
        // Both ends of the mapping ride along: the name the model saw and
        // the lane's own name the call carries.
        assert!(recorded.contains("\"lane_tool\":\"sem1_probe\""), "{recorded}");

        let records = replay(&session.path()).unwrap();
        match &records[1] {
            Record::LaneTools { lane, tools } => {
                assert_eq!(lane, "sem1");
                assert_eq!(tools.len(), 1);
                assert_eq!(tools[0].name, "mcp__sem1__probe");
                assert_eq!(tools[0].lane_tool, "sem1_probe");
                assert_eq!(tools[0].description, "Capability and reachability probe");
                assert_eq!(tools[0].input_schema, serde_json::json!({ "type": "object" }));
            }
            other => panic!("expected a lane_tools record, got {other:?}"),
        }
        // And the log still lists as a readable session.
        assert_eq!(list(&home).unwrap().len(), 1);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_454), (2026, 1, 1));
        assert_eq!(civil_from_days(20_685), (2026, 8, 20));
    }

    #[test]
    fn list_reports_ended_state_and_skips_non_logs() {
        let home = temp_home("list");
        let mut ended = Session::create(&home, "m").unwrap();
        ended.record(&Record::End { stop_reason: "completed".into(), turns: 0 }).unwrap();
        let _open = Session::create(&home, "m2").unwrap();
        fs::write(home.join("sessions").join("notes.txt"), "not a log").unwrap();
        let summaries = list(&home).unwrap();
        assert_eq!(summaries.len(), 2);
        let open = summaries.iter().find(|summary| summary.model == "m2").unwrap();
        assert!(!open.ended);
        let closed = summaries.iter().find(|summary| summary.model == "m").unwrap();
        assert!(closed.ended);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn replay_names_the_offending_line() {
        let home = temp_home("badline");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        let path = dir.join("bad.jsonl");
        fs::write(&path, "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"c\"}\nnot json\n")
            .unwrap();
        let error = replay(&path).unwrap_err();
        assert!(error.to_string().contains("line 2"), "{error}");
        fs::remove_dir_all(&home).ok();
    }
}
