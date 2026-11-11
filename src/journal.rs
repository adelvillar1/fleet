//! The event journal — armada's durable, total-ordered bedrock (L2).
//!
//! Every state change the layers above care about lands here as an [`Event`]
//! with a global monotonic [`Seq`]: the security audit (L3), the
//! communication bus (L5), the workspace events (L7), and the control
//! plane's cross-install tracing (L8) all ride this one home. It is a
//! responsible citizen (D3): bounded by a retention budget, named in every
//! curtail, never a silent drop — deliberately unlike the advisory calib
//! stream, which stays lossy and untouched (D6).
//!
//! The shape (D10–D13):
//! - Day-partitioned append-only JSONL at `<home>/journal/YYYY/MM/DD.jsonl`,
//!   with a global `Seq` persisted at `<home>/journal/seq` so order is total
//!   across days and survives restart.
//! - Durability is `O_APPEND` + flush per record, fsync at a declared
//!   cadence (process-crash durable every record; power-loss durable at the
//!   cadence).
//! - Durable named consumers persist a cursor at
//!   `<home>/journal/consumers/<name>.cursor` and replay from it, so a lagged
//!   or restarted consumer never loses an event.
//!
//! This module is the home, not a producer: the bridge's `record()` hooks it
//! additively (append + broadcast), and the session log stays the
//! conversation record. JSONL before SQLite — no new dependency.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::filelock::FileLock;

/// The journal's total order: a global monotonic counter (D10).
pub type Seq = u64;

/// The default fsync cadence, in appends. Every record is flushed
/// (process-crash durable); every `fsync_every` records the day file is also
/// synced (power-loss durable). A declared choice, named in the contract.
pub const DEFAULT_FSYNC_EVERY: u64 = 64;

/// The default retention bound, in days. Day-files older than this are
/// curtailed by name — the journal is bounded, never run to exhaustion.
pub const DEFAULT_RETENTION_DAYS: u64 = 30;

/// One journaled state change (D10). The install-id (D7) rides every event;
/// the principal (D8) and session ride when there is one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// The global monotonic sequence number.
    pub seq: Seq,
    /// The UTC stamp, the session log's own format.
    pub ts: String,
    /// What happened — the producer's vocabulary (`guard`, `dispatch`,
    /// `work`, `corpus_sync`, a UI event's `type`, …).
    pub kind: String,
    /// The install this event belongs to (D7).
    pub install: String,
    /// The principal the action acted for, when there is one (D8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// The session the event belongs to, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The event's own body.
    pub payload: Value,
}

/// The journal: one home, one sequencer, day-partitioned files (D10).
pub struct Journal {
    home: PathBuf,
    install: String,
    seq: Seq,
    fsync_every: u64,
    appends_since_fsync: u64,
}

impl Journal {
    /// Open the journal for a home, reading the persisted sequencer and the
    /// install-id. A fresh home starts at seq 0.
    pub fn open(home: &Path) -> io::Result<Journal> {
        let install = crate::identity::install(home)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let seq = read_seq(&seq_path(home));
        Ok(Journal {
            home: home.to_path_buf(),
            install: install.as_str().to_string(),
            seq,
            fsync_every: DEFAULT_FSYNC_EVERY,
            appends_since_fsync: 0,
        })
    }

    /// The last sequence number assigned.
    pub fn last_seq(&self) -> Seq {
        self.seq
    }

    /// Append one event, assigning the next `Seq` (D10). The sequencer is
    /// re-read under its flock so concurrent writers serialize and never
    /// reuse a number; the event is flushed to its day file, the sequencer
    /// persisted atomically, and the day file synced at the declared cadence.
    pub fn append(
        &mut self,
        kind: &str,
        principal: Option<&str>,
        session: Option<&str>,
        payload: Value,
    ) -> io::Result<Seq> {
        let seq_file = seq_path(&self.home);
        let _lock = FileLock::take(&FileLock::beside(&seq_file));
        // Another writer may have advanced the sequencer since we opened.
        let seq = read_seq(&seq_file) + 1;
        let ts = crate::calib::stamp();
        let event = Event {
            seq,
            ts: ts.clone(),
            kind: kind.to_string(),
            install: self.install.clone(),
            principal: principal.map(str::to_string),
            session: session.map(str::to_string),
            payload,
        };
        let day_file = self.day_path(&ts[..10]);
        if let Some(parent) = day_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let line = serde_json::to_string(&event)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&day_file)?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
            file.flush()?;
            self.appends_since_fsync += 1;
            if self.appends_since_fsync >= self.fsync_every {
                file.sync_all()?;
                self.appends_since_fsync = 0;
            }
        }
        write_private(&seq_file, seq.to_string().as_bytes())?;
        self.seq = seq;
        Ok(seq)
    }

    /// Every event with `seq >= from`, in total order (D10). Day files are
    /// read in date order; the global `Seq` makes the order total across
    /// them. A torn final line is tolerated (the session log's own rule).
    pub fn replay(&self, from: Seq) -> io::Result<Vec<Event>> {
        let mut events = Vec::new();
        for day_file in self.day_files() {
            let text = match fs::read_to_string(&day_file) {
                Ok(text) => text,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Event>(line) {
                    Ok(event) if event.seq >= from => events.push(event),
                    Ok(_) => {}
                    // A torn tail is the crash shape, not corruption.
                    Err(_) => continue,
                }
            }
        }
        events.sort_by_key(|event| event.seq);
        Ok(events)
    }

    /// A named consumer's persisted cursor — the last `Seq` it acknowledged.
    /// An unknown consumer starts at 0 (D12).
    pub fn cursor(&self, consumer: &str) -> Seq {
        read_seq(&cursor_path(&self.home, consumer))
    }

    /// Persist a named consumer's cursor (D12), atomically.
    pub fn advance(&self, consumer: &str, seq: Seq) -> io::Result<()> {
        let path = cursor_path(&self.home, consumer);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_private(&path, seq.to_string().as_bytes())
    }

    /// The events a named consumer has not yet acknowledged — replay from
    /// its cursor (D12). A restarted consumer resumes exactly where it
    /// stopped.
    pub fn subscribe(&self, consumer: &str) -> io::Result<Vec<Event>> {
        let from = self.cursor(consumer);
        // The cursor is the last acknowledged seq; the next unread is +1.
        self.replay(from.saturating_add(1))
    }

    /// Curtail day-files older than `max_age_days`, returning the names
    /// removed (D11) — a named curtail, never a silent drop. The current
    /// day is never curtailed.
    pub fn curtail(&self, max_age_days: u64) -> io::Result<Vec<String>> {
        let today = crate::calib::today();
        let today_days = day_number(&today);
        let mut removed = Vec::new();
        for day_file in self.day_files() {
            let Some(day) = day_key_of(&day_file) else {
                continue;
            };
            if day == today {
                continue;
            }
            let age = today_days.saturating_sub(day_number(&day));
            if age > max_age_days {
                let name = day_file.display().to_string();
                fs::remove_file(&day_file)?;
                removed.push(name);
            }
        }
        Ok(removed)
    }

    /// The day file for a `YYYY-MM-DD` key: `<home>/journal/YYYY/MM/DD.jsonl`.
    fn day_path(&self, day: &str) -> PathBuf {
        let mut parts = day.split('-');
        let (Some(year), Some(month), Some(day_of_month)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return self.home.join("journal").join("events.jsonl");
        };
        self.home
            .join("journal")
            .join(year)
            .join(month)
            .join(format!("{day_of_month}.jsonl"))
    }

    /// Every day file, in date order (the directory layout sorts naturally).
    fn day_files(&self) -> Vec<PathBuf> {
        let root = self.home.join("journal");
        let mut files = Vec::new();
        let Ok(years) = fs::read_dir(&root) else {
            return files;
        };
        let mut year_dirs: Vec<PathBuf> = years
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        year_dirs.sort();
        for year in year_dirs {
            let Ok(months) = fs::read_dir(&year) else {
                continue;
            };
            let mut month_dirs: Vec<PathBuf> = months
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect();
            month_dirs.sort();
            for month in month_dirs {
                let Ok(days) = fs::read_dir(&month) else {
                    continue;
                };
                let mut day_files: Vec<PathBuf> = days
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
                    .collect();
                day_files.sort();
                files.extend(day_files);
            }
        }
        files
    }
}

/// The sequencer's path: `<home>/journal/seq`.
fn seq_path(home: &Path) -> PathBuf {
    home.join("journal").join("seq")
}

/// A named consumer's cursor path: `<home>/journal/consumers/<name>.cursor`.
fn cursor_path(home: &Path, consumer: &str) -> PathBuf {
    home.join("journal")
        .join("consumers")
        .join(format!("{consumer}.cursor"))
}

/// Read a persisted counter; an absent or unreadable file is 0.
fn read_seq(path: &Path) -> Seq {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse::<Seq>().ok())
        .unwrap_or(0)
}

/// The `YYYY-MM-DD` key a day file encodes, from its `YYYY/MM/DD.jsonl` path.
fn day_key_of(path: &Path) -> Option<String> {
    let day = path.file_stem()?.to_str()?;
    let month = path.parent()?.file_name()?.to_str()?;
    let year = path.parent()?.parent()?.file_name()?.to_str()?;
    Some(format!("{year}-{month}-{day}"))
}

/// A day key's age in days since the epoch — a monotonic day number for
/// comparing ages (the civil calendar's own arithmetic, via the stamp).
fn day_number(day: &str) -> u64 {
    // Parse YYYY-MM-DD into a day count using the session log's civil math.
    let mut parts = day.split('-');
    let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else {
        return 0;
    };
    let (Ok(y), Ok(m), Ok(d)) = (y.parse::<i64>(), m.parse::<u32>(), d.parse::<u32>()) else {
        return 0;
    };
    days_from_civil(y, m, d).max(0) as u64
}

/// Days since the epoch for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i64;
    let day = day as i64;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Write bytes atomically (temp + rename) and privately (0600) — the device
/// store's discipline, so a reader never sees a half-written counter. Flush
/// per record (process-crash durable, the hot path); power-loss durability
/// rides the day file's declared fsync cadence, not a sync per counter write.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.flush()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fleet-journal-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn append_assigns_a_strictly_increasing_seq_that_persists() {
        let home = scratch();
        let mut journal = Journal::open(&home).unwrap();
        let a = journal.append("test", None, None, json!({"n": 1})).unwrap();
        let b = journal.append("test", None, None, json!({"n": 2})).unwrap();
        assert!(b > a, "{a} {b}");
        // A fresh open reads the persisted sequencer and continues.
        let mut reopened = Journal::open(&home).unwrap();
        let c = reopened
            .append("test", None, None, json!({"n": 3}))
            .unwrap();
        assert!(c > b, "{b} {c}");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_appended_event_survives_a_restart() {
        let home = scratch();
        {
            let mut journal = Journal::open(&home).unwrap();
            journal
                .append(
                    "guard",
                    Some("owner"),
                    Some("s1"),
                    json!({"verdict": "granted"}),
                )
                .unwrap();
        }
        // A new Journal (a restart) replays the durable event.
        let journal = Journal::open(&home).unwrap();
        let events = journal.replay(0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "guard");
        assert_eq!(events[0].principal.as_deref(), Some("owner"));
        assert_eq!(events[0].session.as_deref(), Some("s1"));
        assert_eq!(events[0].payload["verdict"], "granted");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_named_consumer_resumes_exactly_where_it_stopped() {
        let home = scratch();
        let mut journal = Journal::open(&home).unwrap();
        let a = journal.append("test", None, None, json!({"n": 1})).unwrap();
        let b = journal.append("test", None, None, json!({"n": 2})).unwrap();
        // A fresh consumer sees everything.
        assert_eq!(journal.subscribe("worker").unwrap().len(), 2);
        // Acknowledge the first; the consumer now resumes at the second.
        journal.advance("worker", a).unwrap();
        let pending = journal.subscribe("worker").unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].seq, b);
        // The cursor persists across a restart.
        let reopened = Journal::open(&home).unwrap();
        assert_eq!(reopened.cursor("worker"), a);
        assert_eq!(reopened.subscribe("worker").unwrap()[0].seq, b);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn replay_is_totally_ordered_and_filters_from() {
        let home = scratch();
        let mut journal = Journal::open(&home).unwrap();
        for n in 0..5 {
            journal.append("test", None, None, json!({"n": n})).unwrap();
        }
        let all = journal.replay(0).unwrap();
        let seqs: Vec<Seq> = all.iter().map(|e| e.seq).collect();
        let mut sorted = seqs.clone();
        sorted.sort();
        assert_eq!(seqs, sorted);
        let from = journal.replay(3).unwrap();
        assert!(from.iter().all(|e| e.seq >= 3));
        assert_eq!(from.len(), 3);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn curtail_removes_old_day_files_by_name_and_keeps_today() {
        let home = scratch();
        let journal = Journal::open(&home).unwrap();
        // Fabricate an old day file and a today file.
        let old = home.join("journal/2020/01/01.jsonl");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "{\"seq\":1}\n").unwrap();
        let today = crate::calib::today();
        let today_file = journal.day_path(&today);
        fs::create_dir_all(today_file.parent().unwrap()).unwrap();
        fs::write(&today_file, "{\"seq\":2}\n").unwrap();
        let removed = journal.curtail(30).unwrap();
        assert_eq!(removed.len(), 1, "{removed:?}");
        assert!(removed[0].contains("2020"));
        assert!(!old.exists());
        assert!(today_file.exists(), "today is never curtailed");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_install_id_rides_every_event() {
        let home = scratch();
        let mut journal = Journal::open(&home).unwrap();
        journal.append("test", None, None, json!({})).unwrap();
        let events = journal.replay(0).unwrap();
        let install = crate::identity::install(&home).unwrap();
        assert_eq!(events[0].install, install.as_str());
        assert_eq!(events[0].install.len(), 32);
        fs::remove_dir_all(&home).ok();
    }
}
