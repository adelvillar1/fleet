//! Append-only JSONL session logs at `~/.fleet/sessions/<id>.jsonl`.
//!
//! Format version 1: the first line is always `start`, and an orderly stop
//! appends `end`; a log without `end` was interrupted. Every message the
//! model sees or produces is logged as it happens, so a replayed log
//! reproduces the conversation exactly.
//!
//! A `compaction` record is the one record that *replaces* messages
//! rather than adding one: the turns it names stay in the log, because a
//! log is append-only, and replay materializes the compacted shape by
//! standing the summary in for them. That is what keeps
//! model-visible ⟺ logged true across a fold (hard rule 5).

use crate::api::{Message, Role, ToolCall};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bump only when existing logs cannot be read safely under the new code;
/// same-version additions require tolerant readers.
pub const SESSION_FORMAT_VERSION: u32 = 1;

/// The `kind` a delegated child run's `start` record carries. An
/// ordinary session's `start` record carries no `kind` at all, so an
/// ordinary log's first line is byte-for-byte what it has always been
/// and a reader learns a log is a child only when it says so.
pub const SUBAGENT_KIND: &str = "subagent";

/// The `kind` a scheduled run's `start` record carries. A scheduled run
/// is an ordinary session — the same loop, the same log format, the
/// same listing — so the marker is the whole difference between "a run
/// the owner started" and "a run a job started", and it is what makes
/// `fleet sessions` name one without opening its log.
pub const SCHEDULED_KIND: &str = "scheduled";

/// One line of a session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    /// First line of every log; pins the format version and the model.
    Start {
        v: u32,
        model: String,
        created: String,
        /// `subagent` for a delegated child run's log; absent for an
        /// ordinary session. Bookkeeping about the log itself, like
        /// `v` and `model`: it replays to no message.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        /// The delegating session's id, on a child's `start` record.
        /// What makes a child's lineage readable from its own log
        /// rather than only from the parent's `tool_result`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        /// The working directory this session was born in — the project
        /// the archive groups it under, which is what a remote face needs
        /// to show "sessions by project" without guessing from the id.
        /// Absent on a log written before the field existed: a
        /// same-version addition, so a tolerant reader takes it as "no
        /// project recorded" rather than as corruption.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace: Option<String>,
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
    /// The channel this session's peer arrived on, the peer's own id on
    /// it, and the peer's name — recorded once, when the channel driver
    /// creates the session. What makes an inbox able to say "via webhook ·
    /// the CI box" beside a conversation, and what tells the owner that a
    /// reply here goes to that peer rather than into the void. Bookkeeping,
    /// like `lane_tools`: it replays to no message.
    ///
    /// The id rides beside the name because a restart has to be able to
    /// tell one peer's conversation from another's, and a name is not
    /// unique: two webhook peers may both be called "the CI box", and a
    /// scan of the archive for a name would hand the wrong conversation
    /// to the wrong peer.
    Channel {
        channel: String,
        peer_id: String,
        peer: String,
    },
    /// The skills one run declared, recorded once at run start. The
    /// skills list is dynamic the way a lane's tools are — the files
    /// change between runs — and the system prompt carries it as a
    /// name+description list, so this record is what makes the prompt
    /// the model saw reconstructable from the log (hard rule 5).
    /// Bookkeeping, like `lane_tools`: it replays to no message. The
    /// body of a selected skill is not here — it is already in that
    /// call's `tool_result`.
    Skills {
        skills: Vec<crate::skills::SkillRef>,
    },
    /// The session's standing objective, recorded once per run when one
    /// is set. The goal rides the system prompt as a `## Goal` section,
    /// so this record is what makes the prompt the model saw
    /// reconstructable from the log (hard rule 5), the `skills` and
    /// `memory` records' precedent. Bookkeeping: it replays to no
    /// message.
    Goal {
        text: String,
    },
    /// The memory one run's turn context carried, recorded once at run
    /// start. The assembled block is dynamic two ways over — the
    /// corpus's files change between runs and the block depends on the
    /// instruction — so this record is what makes the system prompt the
    /// model saw reconstructable from the log (hard rule 5), the
    /// `skills` record's precedent applied to the memory seam.
    /// Bookkeeping, like `lane_tools` and `skills`: it replays to no
    /// message, and the block itself is re-renderable from the corpus
    /// and these names.
    Memory {
        /// The structure shape the instruction routed under, empty
        /// when unclassified — the documented surface-alone fallback.
        shape: String,
        /// Every fact name the block carried, in block order: the
        /// protected tier, then ranked recall, then working memory.
        facts: Vec<String>,
    },
    /// A rewind: the run was rewound to turn `turn`'s checkpoint, and
    /// every record after this one describes a future that did not
    /// happen. A log is append-only, so the abandoned records stay and
    /// a reseed stops here — the conversation continues from the turn
    /// the owner rewound to, and the record says where that was.
    Rewind {
        turn: u64,
        /// How many entries the restore put back and how many stray
        /// files it removed, named so the surface can say what a
        /// rewind did to the tree.
        restored: u64,
        removed: u64,
    },
    /// A plan-mode run's final message: the plan the owner edits and
    /// builds. Bookkeeping like `end` — it replays to no message of
    /// its own, because the plan rides the assistant record beside it.
    Plan {
        text: String,
    },
    /// An orderly stop; a log without it was interrupted.
    End {
        stop_reason: String,
        turns: u64,
        /// The model that actually served, when the run was routed.
        /// KI-073: the `start` record names the NOMINAL model — on a
        /// routed run both the parent's and the child's say the CLI
        /// default — so the record that ends the log carries the one
        /// that answered. `None` on an unrouted run, whose nominal model
        /// is its served model.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        served_model: Option<String>,
    },
    /// One compaction: the summary that stands in for a range of turns,
    /// the range it replaces, and what the context cost before and after.
    ///
    /// The turns it names stay in the log — a log is append-only — and
    /// replay stands the summary in their place, which is what makes the
    /// requests that followed reconstructable byte for byte. The
    /// compaction request itself is reconstructable the same way: the
    /// messages it replaced are the records before this one, the system
    /// prompt is the compiled-in constant, and the instruction is
    /// `compaction::COMPACTION_INSTRUCTION`.
    ///
    /// The record replays to exactly one message — the framed summary —
    /// and the tail it keeps rides along beside it.
    Compaction {
        /// The model's own words: the summary that replaces the range.
        summary: String,
        /// The first turn the summary replaces.
        first_turn: u64,
        /// The last turn the summary replaces, inclusive.
        last_turn: u64,
        /// Tokens the conversation carried when it crossed the budget:
        /// the API's own count when the last response reported one, the
        /// chars/4 estimate when it did not.
        tokens_before: u64,
        /// Tokens the compacted conversation carries: the estimate of the
        /// framed summary plus the verbatim tail.
        tokens_after: u64,
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
        Session::open_new(&dir, model, None, None)
    }

    /// Create a delegated child's log under `home` — the same convention
    /// as [`Session::create`], so every constructor takes a home and
    /// resolves the sessions directory the one way (KI-072) — marked
    /// `kind: subagent` and naming the session that delegated to it.
    ///
    /// A child's log lands beside its parent's, which is what makes the
    /// picker, `fleet sessions`, and replay treat it as an ordinary
    /// session with a marker: a child's work is real work in a real
    /// session, not a side channel.
    pub fn create_subagent(home: &Path, model: &str, parent: &str) -> io::Result<Session> {
        let dir = crate::home::ensure_sessions_dir(home)?;
        Session::open_new(&dir, model, Some(SUBAGENT_KIND), Some(parent))
    }

    /// Create a scheduled run's log under `sessions_dir(home)`, marked
    /// `kind: scheduled` with no parent — a scheduled run belongs to a
    /// job, not to a session that delegated to it, and the job's name
    /// is in the run's own framed task.
    ///
    /// The same seam `create_subagent` uses: a scheduled run is an
    /// ordinary session with a marker, so the picker, `fleet sessions`,
    /// and replay treat it as real work in a real session.
    pub fn create_scheduled(home: &Path, model: &str) -> io::Result<Session> {
        let dir = crate::home::ensure_sessions_dir(home)?;
        Session::open_new(&dir, model, Some(SCHEDULED_KIND), None)
    }

    /// The one place a log is created: the id, the file, and the `start`
    /// record.
    fn open_new(
        dir: &Path,
        model: &str,
        kind: Option<&str>,
        parent: Option<&str>,
    ) -> io::Result<Session> {
        let id = format!(
            "{}-{:06x}",
            compact_utc_stamp(SystemTime::now())?,
            unique_suffix()
        );
        let path = dir.join(format!("{id}.jsonl"));
        let log = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)?;
        let mut session = Session { path, log };
        let created = iso_utc_stamp(SystemTime::now())?;
        // The project this session belongs to, recorded at the one moment
        // it is unambiguous: the process's own working directory when the
        // log is created. A resumed session keeps the project it was born
        // in, which is what makes the archive groupable by project rather
        // than by wherever the owner happened to resume from. A process
        // whose directory has been deleted records none — an absent field
        // reads as "no project recorded", never as a corrupt log.
        let workspace = std::env::current_dir()
            .ok()
            .map(|dir| dir.display().to_string());
        session.record(&Record::Start {
            v: SESSION_FORMAT_VERSION,
            model: model.to_string(),
            created,
            kind: kind.map(str::to_string),
            parent: parent.map(str::to_string),
            workspace,
        })?;
        Ok(session)
    }

    /// Open an existing log in append mode.
    pub fn open(path: &Path) -> io::Result<Session> {
        let log = OpenOptions::new().append(true).open(path)?;
        Ok(Session {
            path: path.to_path_buf(),
            log,
        })
    }

    /// This log's path; the file stem is the session id.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The session id (file stem).
    pub fn id(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
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
///
/// One exception, at exactly one position: a FINAL line that fails to
/// parse is the torn tail of an interrupted `record` — the last write
/// died mid-line, and only ever the last line can be torn (KI-213). It
/// is dropped and named, so every intact record before it stays
/// readable and the session resumes. A malformed line anywhere else —
/// or a final line whose log ends with its newline — is corruption, and
/// still fails loud, naming the file and the line.
pub fn replay(path: &Path) -> io::Result<Vec<Record>> {
    let text = fs::read_to_string(path)?;
    let mut records = Vec::new();
    let torn_tail = !text.ends_with('\n');
    let mut lines = text.lines().enumerate().peekable();
    while let Some((index, line)) = lines.next() {
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(record) => records.push(record),
            Err(error) if torn_tail && lines.peek().is_none() => {
                eprintln!(
                    "fleet: {} line {}: tolerated a torn trailing record \
                     (the last write was interrupted): {error}",
                    path.display(),
                    index + 1
                );
            }
            Err(error) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} line {}: {error}", path.display(), index + 1),
                ));
            }
        }
    }
    Ok(records)
}

/// The records a reseed seeds from: everything before the last
/// [`Record::Rewind`], because the records after it are the future the
/// rewind abandoned. A log with no rewind seeds whole, which is every
/// log that was never rewound.
pub fn rewind_cut(records: &[Record]) -> &[Record] {
    match records
        .iter()
        .rposition(|record| matches!(record, Record::Rewind { .. }))
    {
        Some(at) => &records[..at],
        None => records,
    }
}

/// The summary a log's HEAD carries, read without replaying the whole
/// file: the `start` record is the first line, and `ended` is the last
/// record's kind. KI-056: `list` used to materialize every record of
/// every log to build summaries the head and tail already answer, and
/// `skills::scan_usage` then replayed the window's logs again — a home
/// with many sessions paid the full replay of all of them on every
/// `fleet skills`.
///
/// A log whose head is not a `start` record (empty, torn, or not ours)
/// reads as `None`: the caller keeps its own rule for what to do with
/// a log it cannot summarize, which is what `list` already does.
pub fn summarize(path: &Path) -> Option<SessionSummary> {
    use std::io::BufRead;
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let mut first = String::new();
    if reader.read_line(&mut first).ok()? == 0 {
        return None;
    }
    let Record::Start {
        model,
        created,
        kind,
        workspace,
        ..
    } = serde_json::from_str(first.trim()).ok()?
    else {
        return None;
    };
    let id = path.file_stem().and_then(|s| s.to_str())?.to_string();
    // The tail: the last non-empty line's kind. Bounded by the file's
    // own size the way `replay` is, but the last line only. An `end`
    // record also carries the model that actually served (KI-073),
    // which is what surfaces name instead of the head's nominal stamp.
    let text = fs::read_to_string(path).ok()?;
    let last = text.lines().rev().find(|line| !line.trim().is_empty())?;
    let (ended, served_model, stop_reason, turns) =
        match serde_json::from_str::<Record>(last.trim()) {
            Ok(Record::End {
                stop_reason,
                turns,
                served_model,
            }) => (true, served_model, Some(stop_reason), Some(turns)),
            _ => (false, None, None, None),
        };
    let last_write = fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|age| age.as_secs());
    // The first owner message's first line: the archive's fallback
    // title. The text is already in hand, so this costs one pass.
    let first_text = text
        .lines()
        .find_map(|line| match serde_json::from_str::<Record>(line.trim()) {
            Ok(Record::User { content }) => Some(content),
            _ => None,
        })
        .map(|content| {
            content
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .filter(|text| !text.is_empty());
    let compacted = text.lines().any(|line| {
        matches!(
            serde_json::from_str::<Record>(line.trim()),
            Ok(Record::Compaction { .. })
        )
    });
    // The channel record, when the log carries one: the peer's
    // conversation says so beside its first line.
    let channel = text
        .lines()
        .find_map(|line| match serde_json::from_str::<Record>(line.trim()) {
            Ok(Record::Channel {
                channel,
                peer_id,
                peer,
            }) => Some((channel, peer_id, peer)),
            _ => None,
        });
    // A log written before the start record carried a workspace still
    // names its project in its own paths: the first `/Projects/<name>`
    // a tool call or a command mentions is the directory the run was
    // working in. The summary marks it inferred, so a surface can say
    // where the grouping came from rather than passing a guess off as a
    // recorded fact.
    let (workspace, workspace_inferred) = match workspace {
        Some(workspace) => (Some(workspace), false),
        None => match infer_project(&text) {
            Some(inferred) => (Some(inferred), true),
            None => (None, false),
        },
    };
    Some(SessionSummary {
        id,
        model,
        first_text,
        created,
        ended,
        kind,
        workspace,
        workspace_inferred,
        channel,
        compacted,
        served_model,
        stop_reason,
        turns,
        last_write,
        path: path.to_path_buf(),
    })
}

/// The project a log's own paths name: the first `/Projects/<name>` in
/// the head of the file. Bounded, because a long log's paths repeat and
/// the first one is the run's own working directory often enough to be
/// worth naming — and the summary says it was inferred.
fn infer_project(text: &str) -> Option<String> {
    const HEAD: usize = 256 * 1024;
    let head = text.get(..text.len().min(HEAD)).unwrap_or(text);
    let marker = "/Projects/";
    let at = head.find(marker)?;
    let rest = &head[at + marker.len()..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    // The workspace is the absolute path the marker sits in, not the
    // prose before it: walk back to the token boundary and take from
    // there when what follows is a path.
    let path_start = head[..at]
        .rfind(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '(' || c == '[')
        .map(|index| index + 1)
        .unwrap_or(0);
    let path_start = if head[path_start..].starts_with('/') {
        path_start
    } else {
        at
    };
    Some(format!(
        "{}{}",
        &head[path_start..at + marker.len()],
        &rest[..end]
    ))
}

/// A summary row for `fleet sessions`.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub model: String,
    /// The session's first owner message, first line, trimmed — the
    /// fallback title the archive shows when the owner has not renamed
    /// the session. Read from the log already in hand, never a model
    /// call.
    pub first_text: Option<String>,
    pub created: String,
    /// True when the log's last record is `end`.
    pub ended: bool,
    /// `subagent` for a delegated child run's log; `None` for an
    /// ordinary session's. The picker and `fleet sessions` show it as a
    /// marker beside the row — a child is an ordinary session with a
    /// parent, not a second class of thing.
    pub kind: Option<String>,
    /// The working directory this session was born in, or `None` on a log
    /// written before the field existed. The archive groups by it.
    pub workspace: Option<String>,
    /// True when `workspace` was inferred from the log's own paths rather
    /// than recorded by the start record. A surface that groups by
    /// project says so, because an inference is not a fact the log holds.
    pub workspace_inferred: bool,
    /// The channel this session's peer arrived on, the peer's own id on
    /// it, and the peer's name — or `None` for an ordinary session the
    /// owner started themselves. The first and third are what a surface
    /// shows; the second is what matches a conversation back to a peer.
    pub channel: Option<(String, String, String)>,
    /// True when the log carries a `compaction` record: the conversation
    /// outgrew the window and was folded at least once. The picker and
    /// `fleet sessions` mark it, because a compacted session's replay is
    /// a compacted shape and a reader deserves to know before opening it.
    pub compacted: bool,
    /// The model that actually served, from the log's `end` record
    /// (KI-073). `None` while the run is in flight, on an unrouted run,
    /// and on an unreadable log.
    pub served_model: Option<String>,
    /// How the run ended, in the log's own words: `completed`,
    /// `completed without a report`, `max-turns`, or `stopped by the
    /// owner`. `None` when the log carries no `end` record — an
    /// interrupted run or one still in flight. The archive names this
    /// instead of a bare `ended` flag, because "finished" and "capped"
    /// are different things to the owner who has to decide what to do
    /// next.
    pub stop_reason: Option<String>,
    /// The turns the end record counted, beside `stop_reason`.
    pub turns: Option<u64>,
    /// The log file's own write time, in seconds since the epoch — the
    /// archive's one liveness signal. A log with no `end` younger than
    /// the live window may be a run on another process; an older one is
    /// interrupted. The bound is a heuristic and a surface says so.
    pub last_write: Option<u64>,
    pub path: PathBuf,
}

impl SessionSummary {
    /// The model a surface may NAME as the one in use. KI-073's rule,
    /// extended from the end record to every reader: the start record
    /// names the NOMINAL model — on a routed run that is a pre-route
    /// default and never the model that answered (the archive rendered
    /// `deepseek-flash` for runs step-5-preview served, which is why
    /// this method exists) — so a surface names the end record's
    /// served model, falls back to the nominal name only on an ended
    /// unrouted run (where the nominal model IS the served model), and
    /// names nothing while the run is still in flight.
    pub fn serving_model(&self) -> Option<&str> {
        match (self.ended, &self.served_model) {
            (true, Some(served)) => Some(served),
            (true, None) => Some(&self.model),
            (false, _) => None,
        }
    }
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
        // KI-056: the summary is a head-and-tail read, not a replay of
        // every record — `skills::scan_usage` used to pay the full
        // materialization of every log in the home just to learn these
        // five fields.
        match summarize(&path) {
            Some(summary) => entries.push(summary),
            None => {
                entries.push(SessionSummary {
                    id,
                    model: "unreadable".to_string(),
                    first_text: None,
                    created: String::new(),
                    ended: false,
                    kind: None,
                    workspace: None,
                    workspace_inferred: false,
                    channel: None,
                    compacted: false,
                    served_model: None,
                    stop_reason: None,
                    turns: None,
                    last_write: None,
                    path,
                });
            }
        }
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
/// are dynamic and only the log can reconstruct them. `skills` is the
/// same shape one seam over: the name+description list the system prompt
/// carried, recorded because the files can change between runs.
/// `memory` is the third: the shape and fact names the assembled block
/// carried, recorded because the block is dynamic and only the log can
/// reconstruct the prompt the model saw.
///
/// A `compaction` record is the one record that replaces messages
/// instead of adding them, and it is what makes the compacted shape
/// reconstructable: the summary stands in for the turns it names and the
/// recent tail stays verbatim, so the message list a compacted log
/// replays to is exactly the one the next request carried. See
/// [`conversation`].
pub fn messages(records: &[Record]) -> Vec<Message> {
    conversation(records).messages
}

/// What a log replays to: the wire messages, the turn number of the
/// request that first carried each one, and the turn the next request
/// would be.
///
/// The turn numbers are what a compaction record names, so the agent
/// needs them to write a range a reader can check, and a resumed run
/// needs `next_turn` so the turns it appends continue the log's own
/// numbering rather than restarting it.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub messages: Vec<Message>,
    /// One entry per message: the turn of the request that first carried
    /// it. A user message and the assistant answer that follows it share
    /// a turn; a tool result belongs to the turn whose request carried it.
    pub turns: Vec<u64>,
    /// The turn the next request would be: one past the last assistant
    /// answer in the log, or 1 for a log with none.
    pub next_turn: u64,
}

/// Rebuild the conversation a log represents, with turn numbers.
///
/// The turn rule is the one the agent's own `push_message` applies, so a
/// live conversation and a replayed one number their turns identically:
/// a `user` record takes the upcoming turn, an `assistant` record takes
/// it and advances it, and a `tool_result` takes the turn the request
/// after it will carry.
///
/// A `compaction` record folds what came before it: everything except
/// the most recent [`crate::compaction::COMPACTION_TAIL_TURNS`] turns is
/// dropped from the replay and the framed summary takes its place at the
/// front, carrying the record's own `first_turn`. The tail's records sit
/// *before* the compaction record in the log — they happened before the
/// fold — so the split is the same structural one the agent made when it
/// compacted, computed by the same function
/// ([`crate::compaction::tail_start`]) over the same message list.
pub fn conversation(records: &[Record]) -> Conversation {
    let mut messages: Vec<Message> = Vec::new();
    let mut turns: Vec<u64> = Vec::new();
    let mut next_turn: u64 = 1;
    for record in records {
        match record {
            // Bookkeeping: a rewind marks a cut the caller already
            // applied, and a plan rides the assistant message beside it.
            Record::Rewind { .. } | Record::Plan { .. } => {}
            // Bookkeeping: which channel the peer arrived on replays to
            // no message.
            Record::Channel { .. } => {}
            Record::User { content } => {
                messages.push(Message::text(Role::User, content.clone()));
                turns.push(next_turn);
            }
            Record::Assistant {
                content,
                tool_calls,
            } => {
                messages.push(Message {
                    role: Role::Assistant,
                    content: content.clone(),
                    tool_calls: tool_calls.clone(),
                    tool_call_id: None,
                    images: Vec::new(),
                });
                turns.push(next_turn);
                next_turn += 1;
            }
            Record::ToolResult {
                call_id, output, ..
            } => {
                messages.push(Message::tool_result(call_id.clone(), output.clone()));
                turns.push(next_turn);
            }
            Record::Compaction {
                summary,
                first_turn,
                last_turn,
                ..
            } => {
                let tail = crate::compaction::tail_start(&messages);
                messages = messages.split_off(tail);
                turns = turns.split_off(tail);
                messages.insert(
                    0,
                    Message::text(
                        Role::User,
                        crate::compaction::summary_frame(*first_turn, *last_turn, summary),
                    ),
                );
                turns.insert(0, *first_turn);
            }
            Record::Start { .. }
            | Record::End { .. }
            | Record::LaneTools { .. }
            | Record::Skills { .. }
            | Record::Memory { .. }
            | Record::Goal { .. } => {}
        }
    }
    let next_turn = turns
        .iter()
        .zip(&messages)
        .filter(|(_, message)| message.role == Role::Assistant)
        .map(|(turn, _)| *turn)
        .max()
        .map_or(next_turn, |last| last + 1);
    Conversation {
        messages,
        turns,
        next_turn,
    }
}

/// Resolve a `--session` id to its log path under `home`.
pub fn session_path(home: &Path, id: &str) -> PathBuf {
    crate::home::sessions_dir(home).join(format!("{}.jsonl", id))
}

/// `YYYYMMDDTHHMMSSZ` from the wall clock.
fn compact_utc_stamp(at: SystemTime) -> io::Result<String> {
    let (year, month, day, hour, minute, second) = civil_utc(at)?;
    Ok(format!(
        "{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z"
    ))
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the wall clock.
pub fn iso_utc_stamp(at: SystemTime) -> io::Result<String> {
    let (year, month, day, hour, minute, second) = civil_utc(at)?;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Split the wall clock into UTC civil fields.
fn civil_utc(at: SystemTime) -> io::Result<(i64, u32, u32, u32, u32, u32)> {
    let elapsed = at.duration_since(UNIX_EPOCH).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("clock before the epoch: {error}"),
        )
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
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
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
    nanos.rotate_left(7) ^ std::process::id()
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

    /// KI-056: the summary is a head-and-tail read that answers the same
    /// five fields the full replay did, without materializing every
    /// record — and a log that cannot be summarized reads as `None`
    /// rather than as a wrong summary.
    #[test]
    fn a_summary_is_a_head_and_tail_read() {
        let home = temp_home("summarize");
        let mut session = Session::create(&home, "m").unwrap();
        session
            .record(&Record::User {
                content: "the task".into(),
            })
            .unwrap();
        session
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 2,
                served_model: None,
            })
            .unwrap();
        let summary = summarize(session.path()).expect("the summary reads");
        assert_eq!(summary.model, "m");
        assert!(summary.created.ends_with('Z'), "{}", summary.created);
        assert!(summary.ended);
        // The ending rides the summary in the log's own words, with its
        // turn count and the file's write time — what the archive needs
        // to tell a capped run from a finished one.
        assert_eq!(summary.stop_reason.as_deref(), Some("completed"));
        assert_eq!(summary.turns, Some(2));
        assert!(summary.last_write.is_some(), "{summary:?}");
        // A log with no end record reads as open.
        let open_home = temp_home("summarize-open");
        let open = Session::create(&open_home, "m").unwrap();
        let open_summary = summarize(open.path()).unwrap();
        assert!(!open_summary.ended);
        assert_eq!(open_summary.stop_reason, None);
        assert_eq!(open_summary.turns, None);
        assert!(open_summary.last_write.is_some(), "{open_summary:?}");
        // A log that is not ours reads as nothing.
        let stray = open_home.join("stray.jsonl");
        std::fs::write(&stray, "not a session log\n").unwrap();
        assert!(summarize(&stray).is_none());
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&open_home);
    }

    #[test]
    fn create_records_start_and_round_trips() {
        let home = temp_home("create");
        let mut session = Session::create(&home, "deepseek-flash").unwrap();
        session
            .record(&Record::User {
                content: "hello".into(),
            })
            .unwrap();
        session
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            })
            .unwrap();
        let records = replay(session.path()).unwrap();
        assert_eq!(records.len(), 3);
        assert!(matches!(
            &records[0],
            Record::Start { v: 1, model, .. } if model == "deepseek-flash"
        ));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_new_log_records_the_workspace_it_was_born_in() {
        let home = temp_home("workspace");
        let mut session = Session::create(&home, "deepseek-flash").unwrap();
        session
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 0,
                served_model: None,
            })
            .unwrap();
        // The process's own working directory, recorded at the one moment
        // it is unambiguous. The test runner's cwd is whatever it is; the
        // point is the field is present and equals it.
        let expected = std::env::current_dir().unwrap().display().to_string();
        let records = replay(session.path()).unwrap();
        assert!(
            matches!(&records[0], Record::Start { workspace: Some(w), .. } if *w == expected),
            "{records:?}"
        );
        let listed = list(&home).unwrap();
        assert_eq!(listed.len(), 1, "{listed:?}");
        assert_eq!(listed[0].workspace.as_deref(), Some(expected.as_str()));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_log_written_before_the_field_reads_as_no_project() {
        // The tolerant-reader promise: a same-version addition means a log
        // without the field is a log from before it, not a corrupt one.
        // Hand-written here because the writer can no longer produce one.
        let home = temp_home("no-workspace");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        let path = dir.join("20261013T101010Z-abcdef.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"type":"start","v":1,"model":"deepseek-flash","created":"2026-10-13T10:10:10Z"}"#,
                "\n",
                r#"{"type":"end","stop_reason":"completed","turns":1}"#,
                "\n",
            ),
        )
        .unwrap();
        let listed = list(&home).unwrap();
        assert_eq!(listed.len(), 1, "{listed:?}");
        assert_eq!(listed[0].workspace, None);
        assert!(listed[0].ended);
        // And the records still replay: the missing field costs the
        // grouping, never the conversation.
        let records = replay(&path).unwrap();
        assert_eq!(records.len(), 2, "{records:?}");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_log_without_a_recorded_workspace_infers_one_from_its_paths() {
        // The archive still groups an older log by the project its own
        // tool calls name. The flag says the grouping was inferred, so a
        // surface can name where it came from.
        let home = temp_home("inferred-workspace");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        let path = dir.join("20261013T194909Z-bb2fc12.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"type":"start","v":1,"model":"deepseek-flash","created":"2026-10-13T19:49:09Z"}"#,
                "\n",
                r#"{"type":"user","content":"read /Users/owner/Projects/demo-repo/docs/plan.md"}"#,
                "\n",
            ),
        )
        .unwrap();
        let listed = list(&home).unwrap();
        assert_eq!(listed.len(), 1, "{listed:?}");
        assert_eq!(
            listed[0].workspace.as_deref(),
            Some("/Users/owner/Projects/demo-repo")
        );
        assert!(listed[0].workspace_inferred);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_torn_trailing_record_is_tolerated_and_the_session_survives() {
        let home = temp_home("torn-tail");
        let mut session = Session::create(&home, "deepseek-flash").unwrap();
        session
            .record(&Record::User {
                content: "hello".into(),
            })
            .unwrap();
        session
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            })
            .unwrap();
        // The interrupted last write: a partial record, no newline.
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(session.path())
            .unwrap();
        use std::io::Write;
        write!(file, "{{\"type\":\"tool_res").unwrap();
        drop(file);

        // Every intact record reads back; the torn tail is dropped and
        // named on stderr.
        let records = replay(session.path()).unwrap();
        assert_eq!(records.len(), 3);
        assert!(matches!(
            records.last().unwrap(),
            Record::End { stop_reason, .. } if stop_reason == "completed"
        ));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_malformed_mid_log_line_still_fails_loud() {
        let home = temp_home("mid-log-garbage");
        let mut session = Session::create(&home, "deepseek-flash").unwrap();
        session
            .record(&Record::User {
                content: "hello".into(),
            })
            .unwrap();
        let path = session.path().to_path_buf();
        drop(session);
        // Corruption in the middle: appended garbage with its newline,
        // so it is a complete line, not a torn tail.
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        use std::io::Write;
        writeln!(file, "not json at all").unwrap();
        drop(file);

        let error = replay(&path).unwrap_err();
        assert!(error.to_string().contains("line 3"), "{error}");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_final_line_that_parses_is_kept_even_without_its_newline() {
        let home = temp_home("untorn-final");
        let mut session = Session::create(&home, "deepseek-flash").unwrap();
        session
            .record(&Record::User {
                content: "hello".into(),
            })
            .unwrap();
        // A complete final record whose newline never landed — a full
        // write cut before the push('\n') — is a record, not a tear.
        let path = session.path().to_path_buf();
        drop(session);
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("{\"type\":\"end\",\"stop_reason\":\"completed\",\"turns\":1}");
        fs::write(&path, text).unwrap();

        let records = replay(&path).unwrap();
        assert_eq!(records.len(), 3);
        assert!(matches!(
            records.last().unwrap(),
            Record::End { stop_reason, .. } if stop_reason == "completed"
        ));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn messages_skip_bookkeeping_records() {
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".into(),
                created: "c".into(),
                kind: None,
                parent: None,
                workspace: None,
            },
            Record::User {
                content: "hi".into(),
            },
            Record::Assistant {
                content: Some("running".into()),
                tool_calls: vec![],
            },
            Record::ToolResult {
                call_id: "call-1".into(),
                name: "bash".into(),
                output: "out".into(),
                truncated: false,
                approved: true,
            },
            Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            },
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
            Record::Start {
                v: 1,
                model: "m".into(),
                created: "c".into(),
                kind: None,
                parent: None,
                workspace: None,
            },
            Record::User {
                content: "probe it".into(),
            },
            lane_tools_record(),
            Record::Assistant {
                content: None,
                tool_calls: Vec::new(),
            },
            Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            },
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
        assert!(
            recorded.starts_with("{\"type\":\"lane_tools\""),
            "{recorded}"
        );
        assert!(recorded.contains("\"lane\":\"sem1\""), "{recorded}");
        assert!(
            recorded.contains("\"name\":\"mcp__sem1__probe\""),
            "{recorded}"
        );
        // Both ends of the mapping ride along: the name the model saw and
        // the lane's own name the call carries.
        assert!(
            recorded.contains("\"lane_tool\":\"sem1_probe\""),
            "{recorded}"
        );

        let records = replay(session.path()).unwrap();
        match &records[1] {
            Record::LaneTools { lane, tools } => {
                assert_eq!(lane, "sem1");
                assert_eq!(tools.len(), 1);
                assert_eq!(tools[0].name, "mcp__sem1__probe");
                assert_eq!(tools[0].lane_tool, "sem1_probe");
                assert_eq!(tools[0].description, "Capability and reachability probe");
                assert_eq!(
                    tools[0].input_schema,
                    serde_json::json!({ "type": "object" })
                );
            }
            other => panic!("expected a lane_tools record, got {other:?}"),
        }
        // And the log still lists as a readable session.
        assert_eq!(list(&home).unwrap().len(), 1);
        fs::remove_dir_all(&home).ok();
    }

    /// One `skills` record, as the agent writes it when a run declares
    /// skills: the name+description list the system prompt carried.
    fn skills_record() -> Record {
        Record::Skills {
            skills: vec![crate::skills::SkillRef {
                name: "bash-recovery".to_string(),
                description: "Recover a failed bash call".to_string(),
            }],
        }
    }

    #[test]
    fn a_skills_record_replays_as_bookkeeping_not_conversation() {
        // The vocabulary grew mid-format-version: a log carrying the new
        // record must still replay, and the record must not become a
        // message — it describes the system prompt's skills list, not
        // anything the model said.
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".into(),
                created: "c".into(),
                kind: None,
                parent: None,
                workspace: None,
            },
            skills_record(),
            Record::User {
                content: "do it".into(),
            },
            Record::Assistant {
                content: Some("done".into()),
                tool_calls: vec![],
            },
            Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            },
        ];
        let messages = messages(&records);
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn a_skills_line_round_trips_through_the_log_file() {
        // The record's own line, written the way the log writes it and
        // read back by a reader that only knows the vocabulary: the tag
        // is `skills`, and every declared skill rides along with the
        // description the model was shown.
        let home = temp_home("skills");
        let mut session = Session::create(&home, "m").unwrap();
        session.record(&skills_record()).unwrap();
        let line = std::fs::read_to_string(session.path()).unwrap();
        let recorded = line.lines().nth(1).expect("the skills line");
        assert!(recorded.starts_with("{\"type\":\"skills\""), "{recorded}");
        assert!(
            recorded.contains("\"name\":\"bash-recovery\""),
            "{recorded}"
        );
        assert!(
            recorded.contains("\"description\":\"Recover a failed bash call\""),
            "{recorded}"
        );

        let records = replay(session.path()).unwrap();
        match &records[1] {
            Record::Skills { skills } => {
                assert_eq!(skills.len(), 1);
                assert_eq!(skills[0].name, "bash-recovery");
                assert_eq!(skills[0].description, "Recover a failed bash call");
            }
            other => panic!("expected a skills record, got {other:?}"),
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

    /// One `memory` record, as the agent writes it when a run's
    /// assembly produced a block: the shape the instruction routed
    /// under and the fact names the block carried, in block order.
    fn memory_record() -> Record {
        Record::Memory {
            shape: "deploy".to_string(),
            facts: vec![
                "identity".to_string(),
                "fleet-the-gate-is-local".to_string(),
                "working-note".to_string(),
            ],
        }
    }

    #[test]
    fn a_memory_record_replays_as_bookkeeping_not_conversation() {
        // The assembled block is dynamic, so the record is what keeps
        // the prompt reconstructable — and it must not become a
        // message: it describes the system prompt, not anything the
        // model said.
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".into(),
                created: "c".into(),
                kind: None,
                parent: None,
                workspace: None,
            },
            memory_record(),
            Record::User {
                content: "the deploy is failing".into(),
            },
            Record::Assistant {
                content: Some("checking".into()),
                tool_calls: Vec::new(),
            },
            Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            },
        ];
        let messages = messages(&records);
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn a_memory_line_round_trips_through_the_log_file() {
        // The record's own line, written the way the log writes it and
        // read back by a reader that only knows the vocabulary: the
        // tag is `memory`, and the shape and every fact name survive.
        let home = temp_home("memory-record");
        let mut session = Session::create(&home, "m").unwrap();
        session.record(&memory_record()).unwrap();
        let line = std::fs::read_to_string(session.path()).unwrap();
        let recorded = line.lines().nth(1).expect("the memory line");
        assert!(recorded.starts_with("{\"type\":\"memory\""), "{recorded}");
        assert!(recorded.contains("\"shape\":\"deploy\""), "{recorded}");
        assert!(recorded.contains("\"identity\""), "{recorded}");
        assert!(recorded.contains("\"working-note\""), "{recorded}");

        let records = replay(session.path()).unwrap();
        match &records[1] {
            Record::Memory { shape, facts } => {
                assert_eq!(shape, "deploy");
                assert_eq!(
                    facts,
                    &vec![
                        "identity".to_string(),
                        "fleet-the-gate-is-local".to_string(),
                        "working-note".to_string(),
                    ]
                );
            }
            other => panic!("expected a memory record, got {other:?}"),
        }
        // And the log still lists as a readable session.
        assert_eq!(list(&home).unwrap().len(), 1);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn list_reports_ended_state_and_skips_non_logs() {
        let home = temp_home("list");
        let mut ended = Session::create(&home, "m").unwrap();
        ended
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 0,
                served_model: None,
            })
            .unwrap();
        let _open = Session::create(&home, "m2").unwrap();
        fs::write(home.join("sessions").join("notes.txt"), "not a log").unwrap();
        let summaries = list(&home).unwrap();
        assert_eq!(summaries.len(), 2);
        let open = summaries
            .iter()
            .find(|summary| summary.model == "m2")
            .unwrap();
        assert!(!open.ended);
        let closed = summaries
            .iter()
            .find(|summary| summary.model == "m")
            .unwrap();
        assert!(closed.ended);
        fs::remove_dir_all(&home).ok();
    }

    /// One child log's `start` record, as the delegation path writes it:
    /// the model, the `kind` marker, and the delegating session's id.
    fn subagent_start() -> Record {
        Record::Start {
            v: 1,
            model: "test-model".to_string(),
            created: "2026-10-14T00:00:00Z".to_string(),
            kind: Some(SUBAGENT_KIND.to_string()),
            parent: Some("20261013T000000Z-abcdef".to_string()),
            workspace: None,
        }
    }

    #[test]
    fn a_child_logs_start_record_carries_its_kind_and_its_parent() {
        let home = temp_home("subagent-start");
        // KI-072: the constructor takes a home and resolves the sessions
        // directory itself — the test no longer resolves one to hand it.
        let mut child = Session::create_subagent(&home, "test-model", "parent-session").unwrap();
        child
            .record(&Record::User {
                content: "the delegated goal".into(),
            })
            .unwrap();
        child
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 2,
                served_model: None,
            })
            .unwrap();

        let line = fs::read_to_string(child.path()).unwrap();
        let recorded = line.lines().next().expect("the start line");
        // The marker and the lineage ride the log's own first line, and
        // the format version is untouched.
        assert!(
            recorded.starts_with("{\"type\":\"start\",\"v\":1"),
            "{recorded}"
        );
        assert!(recorded.contains("\"kind\":\"subagent\""), "{recorded}");
        assert!(
            recorded.contains("\"parent\":\"parent-session\""),
            "{recorded}"
        );

        let records = replay(child.path()).unwrap();
        match &records[0] {
            Record::Start {
                v,
                model,
                kind,
                parent,
                ..
            } => {
                assert_eq!(*v, SESSION_FORMAT_VERSION);
                assert_eq!(model, "test-model");
                assert_eq!(kind.as_deref(), Some("subagent"));
                assert_eq!(parent.as_deref(), Some("parent-session"));
            }
            other => panic!("expected a start record, got {other:?}"),
        }
        // The marker is bookkeeping: the log still replays to exactly the
        // conversation it holds, and it still lists as an ordinary session.
        let messages = messages(&records);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::User);
        let summaries = list(&home).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].kind.as_deref(), Some("subagent"));
        assert!(summaries[0].ended);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_summary_names_the_model_that_served_not_the_start_stamp() {
        // KI-073 put the model that answered in the end record; KI-255 is
        // the surfaces reading it. The start record's model is a
        // pre-route default on a routed run — the archive rendered it as
        // the model in use for runs step-5-preview served — so the
        // summary names the served model, nothing while a run is in
        // flight, and the nominal name only on an ended unrouted run,
        // where the nominal model IS the served model.
        let home = temp_home("serving-model");
        let mut routed = Session::create(&home, "deepseek-flash").unwrap();
        routed
            .record(&Record::User {
                content: "the task".into(),
            })
            .unwrap();
        routed
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: Some("step-5-preview".to_string()),
            })
            .unwrap();
        let mut open = Session::create(&home, "deepseek-flash").unwrap();
        open.record(&Record::User {
            content: "still running".into(),
        })
        .unwrap();
        let mut unrouted = Session::create(&home, "deepseek-flash").unwrap();
        unrouted
            .record(&Record::End {
                stop_reason: "completed".into(),
                turns: 1,
                served_model: None,
            })
            .unwrap();

        let summaries = list(&home).unwrap();
        let by_id = |id: &str| {
            summaries
                .iter()
                .find(|summary| summary.id == id)
                .unwrap_or_else(|| panic!("no summary for {id}"))
        };
        assert_eq!(
            by_id(routed.id()).serving_model(),
            Some("step-5-preview"),
            "the routed run names what answered"
        );
        assert_eq!(
            by_id(open.id()).serving_model(),
            None,
            "an open run names nothing — nothing has served yet"
        );
        assert_eq!(
            by_id(unrouted.id()).serving_model(),
            Some("deepseek-flash"),
            "an ended unrouted run's nominal model is its served model"
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_ordinary_logs_start_record_carries_no_kind() {
        // The ordinary run is byte-for-byte the run it has always been:
        // no `kind`, no `parent`, and no marker in the listing.
        let home = temp_home("ordinary-start");
        let session = Session::create(&home, "m").unwrap();
        let line = fs::read_to_string(session.path()).unwrap();
        let recorded = line.lines().next().expect("the start line");
        assert!(!recorded.contains("kind"), "{recorded}");
        assert!(!recorded.contains("parent"), "{recorded}");
        assert_eq!(list(&home).unwrap()[0].kind, None);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_childs_start_record_round_trips_from_a_reader_that_knows_the_vocabulary() {
        // The record read back is the record written: the marker and the
        // lineage both survive the round trip through the file.
        let home = temp_home("subagent-round-trip");
        let mut session = Session::create(&home, "m").unwrap();
        session.record(&subagent_start()).unwrap();
        let records = replay(session.path()).unwrap();
        // The log's own first line is the create's `start`; the child
        // record is the one this test appended.
        assert_eq!(
            serde_json::to_string(&records[1]).unwrap(),
            serde_json::to_string(&subagent_start()).unwrap()
        );
        // A pre-slice-9 first line, with neither field, reads as an
        // ordinary session rather than failing the whole log.
        let legacy = "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"c\"}\n";
        let parsed: Record = serde_json::from_str(legacy).unwrap();
        match parsed {
            Record::Start { kind, parent, .. } => {
                assert_eq!(kind, None);
                assert_eq!(parent, None);
            }
            other => panic!("expected a start record, got {other:?}"),
        }
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn replay_names_the_offending_line() {
        let home = temp_home("badline");
        let dir = crate::home::ensure_sessions_dir(&home).unwrap();
        let path = dir.join("bad.jsonl");
        fs::write(
            &path,
            "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"c\"}\nnot json\n",
        )
        .unwrap();
        let error = replay(&path).unwrap_err();
        assert!(error.to_string().contains("line 2"), "{error}");
        fs::remove_dir_all(&home).ok();
    }

    // --- compaction -------------------------------------------------------

    /// One `compaction` record, as the agent writes it when the window
    /// check fires: the summary, the range it replaces, and the counts
    /// either side of the fold.
    fn compaction_record() -> Record {
        Record::Compaction {
            summary: "- did the thing\n- then the other thing".to_string(),
            first_turn: 1,
            last_turn: 2,
            tokens_before: 120_000,
            tokens_after: 900,
        }
    }

    /// Six turns of a tool-using conversation: a task, then per turn one
    /// assistant call and one tool result.
    fn six_turns() -> Vec<Record> {
        let mut records = vec![Record::User {
            content: "the task".to_string(),
        }];
        for index in 0..6 {
            records.push(Record::Assistant {
                content: None,
                tool_calls: vec![crate::api::ToolCall {
                    id: format!("call-{index}"),
                    kind: "function".to_string(),
                    function: crate::api::FunctionCall {
                        name: "bash".to_string(),
                        arguments: "{}".to_string(),
                    },
                }],
            });
            records.push(Record::ToolResult {
                call_id: format!("call-{index}"),
                name: "bash".to_string(),
                output: format!("output {index}"),
                truncated: false,
                approved: true,
            });
        }
        records
    }

    #[test]
    fn a_compaction_record_replays_to_the_summary_plus_the_verbatim_tail() {
        // The fold: the summary stands in for the turns it names, the
        // most recent four turns stay exactly as they happened, and
        // nothing else survives.
        let mut records = six_turns();
        records.push(compaction_record());
        let conversation = conversation(&records);
        let messages = messages(&records);
        assert_eq!(messages, conversation.messages);

        // One summary message, then the tail's four turns verbatim.
        assert_eq!(messages.len(), 1 + 4 * 2, "{messages:?}");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(
            messages[0].content.as_deref(),
            Some(
                "Summary of turns 1 through 2, compacted to fit the window:\n\n- did the thing\n- then the other thing"
            )
        );
        // The tail opens on an assistant message, so the sequence stays
        // valid for the API (KI-016's failure in a new shape).
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].tool_calls[0].id, "call-2");
        assert_eq!(messages[2].role, Role::Tool);
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("call-2"));
        // The last turn is there whole, and nothing before turn 3 is.
        assert_eq!(
            messages[messages.len() - 1].content.as_deref(),
            Some("output 5")
        );
        assert!(
            !messages
                .iter()
                .any(|message| message.content.as_deref() == Some("output 0")),
            "{messages:?}"
        );
        assert!(!messages
            .iter()
            .any(|message| message.content.as_deref() == Some("the task")));
    }

    #[test]
    fn a_compaction_record_numbers_the_turns_a_resumed_run_continues() {
        let mut records = six_turns();
        records.push(compaction_record());
        // The tail is turns 3 through 6, so the next request is turn 7 —
        // the resumed run appends to the log's own numbering rather than
        // restarting it.
        let conversation = conversation(&records);
        assert_eq!(conversation.next_turn, 7);
        assert_eq!(
            conversation.turns,
            vec![1, 3, 4, 4, 5, 5, 6, 6, 7],
            "{:?}",
            conversation.turns
        );
        // The summary carries the range's first turn, which is what makes
        // a second compaction's range readable.
        assert_eq!(conversation.turns[0], 1);
    }

    #[test]
    fn a_second_compaction_folds_the_first_summary_into_a_new_one() {
        // A conversation that outgrows the window twice: the second
        // record replaces the first summary and the tail it kept, and
        // the replay holds one summary and one tail.
        let mut records = six_turns();
        records.push(compaction_record());
        for index in 6..10 {
            records.push(Record::Assistant {
                content: Some(format!("answer {index}")),
                tool_calls: Vec::new(),
            });
        }
        records.push(Record::Compaction {
            summary: "the consolidated summary".to_string(),
            first_turn: 1,
            last_turn: 6,
            tokens_before: 140_000,
            tokens_after: 800,
        });
        let messages = messages(&records);
        // One summary plus the last four assistant answers.
        assert_eq!(messages.len(), 5, "{messages:?}");
        assert_eq!(messages[0].content.as_deref(), Some(
            "Summary of turns 1 through 6, compacted to fit the window:\n\nthe consolidated summary"
        ));
        assert_eq!(messages[1].content.as_deref(), Some("answer 6"));
        assert_eq!(messages[4].content.as_deref(), Some("answer 9"));
        let conversation = conversation(&records);
        assert_eq!(conversation.next_turn, 11);
    }

    #[test]
    fn a_log_with_no_compaction_replays_byte_for_byte_as_it_always_did() {
        // The record is additive: an ordinary log's shape does not change.
        let records = six_turns();
        let messages = messages(&records);
        assert_eq!(messages.len(), 13, "{messages:?}");
        assert_eq!(messages[0].content.as_deref(), Some("the task"));
        assert_eq!(messages[1].tool_calls[0].id, "call-0");
        assert_eq!(messages[12].content.as_deref(), Some("output 5"));
        let conversation = conversation(&records);
        assert_eq!(conversation.turns.len(), messages.len());
        // Every assistant answer advances the turn; the tool result after
        // it belongs to the next request.
        assert_eq!(conversation.turns[0], 1);
        assert_eq!(conversation.turns[1], 1);
        assert_eq!(conversation.turns[2], 2);
        assert_eq!(conversation.next_turn, 7);
    }

    #[test]
    fn a_compaction_line_round_trips_through_the_log_file() {
        // The record's own line, written the way the log writes it and
        // read back by a reader that only knows the vocabulary.
        let home = temp_home("compaction-line");
        let mut session = Session::create(&home, "m").unwrap();
        session.record(&compaction_record()).unwrap();
        let line = fs::read_to_string(session.path()).unwrap();
        let recorded = line.lines().nth(1).expect("the compaction line");
        assert!(
            recorded.starts_with("{\"type\":\"compaction\""),
            "{recorded}"
        );
        assert!(recorded.contains("\"first_turn\":1"), "{recorded}");
        assert!(recorded.contains("\"last_turn\":2"), "{recorded}");
        assert!(recorded.contains("\"tokens_before\":120000"), "{recorded}");
        assert!(recorded.contains("\"tokens_after\":900"), "{recorded}");

        let records = replay(session.path()).unwrap();
        match &records[1] {
            Record::Compaction {
                summary,
                first_turn,
                last_turn,
                tokens_before,
                tokens_after,
            } => {
                assert_eq!(summary, "- did the thing\n- then the other thing");
                assert_eq!(*first_turn, 1);
                assert_eq!(*last_turn, 2);
                assert_eq!(*tokens_before, 120_000);
                assert_eq!(*tokens_after, 900);
            }
            other => panic!("expected a compaction record, got {other:?}"),
        }
        // And the log still lists, marked compacted.
        let summaries = list(&home).unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].compacted);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_log_without_a_compaction_lists_unmarked() {
        let home = temp_home("no-compaction");
        let mut session = Session::create(&home, "m").unwrap();
        session
            .record(&Record::User {
                content: "hi".into(),
            })
            .unwrap();
        let summaries = list(&home).unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(!summaries[0].compacted);
        // The other markers are untouched.
        assert_eq!(summaries[0].kind, None);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_interrupted_log_after_a_compaction_still_replays_its_compacted_shape() {
        // A log without `end` was interrupted; the compaction before the
        // interruption still folds, and the turn numbering continues from
        // the last assistant answer rather than from the last message.
        let mut records = six_turns();
        records.push(compaction_record());
        // The interrupted turn's call landed and its result did not.
        records.push(Record::Assistant {
            content: None,
            tool_calls: Vec::new(),
        });
        records.push(Record::ToolResult {
            call_id: "call-9".to_string(),
            name: "bash".to_string(),
            output: "partial".to_string(),
            truncated: false,
            approved: true,
        });
        let conversation = conversation(&records);
        assert_eq!(conversation.next_turn, 8);
        assert_eq!(conversation.turns.last().copied(), Some(8));
        // The summary, the tail's eight messages, and the interrupted
        // turn's own two.
        assert_eq!(conversation.messages.len(), 11);
    }
}
