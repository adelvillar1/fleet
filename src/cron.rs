//! Cron-shaped scheduled runs: a persistent job store, a hand-rolled
//! five-field cron parser, a background scheduler with catch-up on
//! missed windows, and the `fleet cron` surface.
//!
//! The store is one JSON document — `<FLEET_HOME>/cron/jobs.json` —
//! listing the jobs, rewritten atomically on every change. A job is a
//! name, a cron expression, a self-contained task, an `auto_approve`
//! declaration, an `enabled` flag, and the last run it recorded. The
//! scheduler reads that document fresh every tick, so `fleet cron add`
//! from another process is picked up within a second.
//!
//! A fire is an ordinary headless agent run: the same loop, the same
//! routed client and roster, its own session log under the sessions
//! directory, the memory corpus wired the way `fleet run` wires it. The
//! only difference is the gate — nobody is there to approve a tool call,
//! so a job that is not declared `auto_approve` has its calls skipped
//! and named rather than silently allowed.
//!
//! Spec: the fork's `packages/schedule` + `packages/jobs` — read as a
//! reference, never as a dependency (hard rule 6). Ported vocabulary:
//! the job record's shape, the catch-up rule (one run for the missed
//! window, resolved near the decision and not across the whole missed
//! history), the reminder framing a scheduled turn carries, and the
//! delivery seam, which this crate does not build (see
//! `docs/features/cron.md`).

use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::agent::{
    Agent, AgentError, AgentOptions, Approver, FileAccess, RunReport, UiEvent, DEFAULT_MAX_TURNS,
};
use crate::api::Client;
use crate::route;
use crate::session::{self, Session};

/// The length of one stamped minute, `YYYY-MM-DDTHH:MMZ`.
const MINUTE_STAMP_LEN: usize = 17;

// --- the minute -------------------------------------------------------------

/// One UTC calendar minute: the unit a cron expression matches, the
/// unit the scheduler ticks against, and the unit a job's `last_run` is
/// stored in. The field order is the chronological order, so the
/// derived `Ord` compares minutes correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Minute {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Minute {
    /// The current UTC minute, truncated: a tick anywhere inside a
    /// minute decides for that whole minute.
    pub fn now() -> Minute {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|span| span.as_secs())
            .unwrap_or(0);
        let days = (elapsed / 86_400) as i64;
        let seconds_of_day = elapsed % 86_400;
        let (year, month, day) = session::civil_from_days(days);
        Minute {
            year,
            month,
            day,
            hour: (seconds_of_day / 3_600) as u32,
            minute: ((seconds_of_day % 3_600) / 60) as u32,
        }
    }

    /// `YYYY-MM-DDTHH:MMZ`, the stored spelling of one minute.
    pub fn stamp(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }

    /// Read one stamped minute back. A stamp that is not exactly this
    /// shape, or names a day or time that does not exist, is `None` —
    /// the store refuses it rather than guessing.
    pub fn parse(text: &str) -> Option<Minute> {
        if text.len() != MINUTE_STAMP_LEN || !text.ends_with('Z') {
            return None;
        }
        let digits = |range: std::ops::Range<usize>| {
            text.get(range).and_then(|part| part.parse::<u32>().ok())
        };
        let separators = text.as_bytes();
        if separators[4] != b'-'
            || separators[7] != b'-'
            || separators[10] != b'T'
            || separators[13] != b':'
        {
            return None;
        }
        let minute = Minute {
            year: digits(0..4)? as i64,
            month: digits(5..7)?,
            day: digits(8..10)?,
            hour: digits(11..13)?,
            minute: digits(14..16)?,
        };
        if minute.hour > 23 || minute.minute > 59 {
            return None;
        }
        let day_key = format!("{:04}-{:02}-{:02}", minute.year, minute.month, minute.day);
        crate::cli::valid_calendar_day(&day_key).then_some(minute)
    }

    /// The weekday, 0 (Sunday) through 6 (Saturday).
    pub fn day_of_week(&self) -> u32 {
        (days_from_civil(self.year, self.month, self.day) + 4).rem_euclid(7) as u32
    }

    /// The next calendar day at 00:00 — the search walk's date carrier.
    fn next_day(&self) -> Minute {
        let (year, month, day) =
            session::civil_from_days(days_from_civil(self.year, self.month, self.day) + 1);
        Minute {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
        }
    }

    /// The previous calendar day at 00:00 — the search walk's date
    /// carrier.
    fn previous_day(&self) -> Minute {
        let (year, month, day) =
            session::civil_from_days(days_from_civil(self.year, self.month, self.day) - 1);
        Minute {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
        }
    }

    /// One minute earlier, carrying the borrow back across the hour,
    /// day, month, and year boundaries. The passed-window walk's step
    /// into the minute a job was first due, so that minute can itself
    /// be named when it passed unserved.
    fn previous_minute(&self) -> Minute {
        if self.minute > 0 {
            return Minute {
                minute: self.minute - 1,
                ..*self
            };
        }
        if self.hour > 0 {
            return Minute {
                hour: self.hour - 1,
                minute: 59,
                ..*self
            };
        }
        let (year, month, day) =
            session::civil_from_days(days_from_civil(self.year, self.month, self.day) - 1);
        Minute {
            year,
            month,
            day,
            hour: 23,
            minute: 59,
        }
    }
}

/// Days since 1970-01-01 for one civil date: Howard Hinnant's
/// `days_from_civil`, the inverse of the session log's `civil_from_days`.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = if shifted >= 0 { shifted } else { shifted - 399 } / 400;
    let year_of_era = shifted - era * 400;
    let day_of_year =
        ((153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1) as i64;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

// --- the expression ---------------------------------------------------------

/// One cron field's bounds: the name diagnostics use, the accepted
/// range, and the canonical maximum after Sunday's 7 folds onto 0.
struct FieldSpec {
    name: &'static str,
    min: u32,
    max: u32,
    canonical_max: u32,
}

/// The five fields, in evaluation order: minute hour day-of-month month
/// day-of-week.
const FIELDS: [FieldSpec; 5] = [
    FieldSpec {
        name: "minute",
        min: 0,
        max: 59,
        canonical_max: 59,
    },
    FieldSpec {
        name: "hour",
        min: 0,
        max: 23,
        canonical_max: 23,
    },
    FieldSpec {
        name: "day-of-month",
        min: 1,
        max: 31,
        canonical_max: 31,
    },
    FieldSpec {
        name: "month",
        min: 1,
        max: 12,
        canonical_max: 12,
    },
    // Sunday is both 0 and 7; 7 folds onto 0 so the matched set stays
    // 0-6 and a reader never has to remember which spelling was typed.
    FieldSpec {
        name: "day-of-week",
        min: 0,
        max: 7,
        canonical_max: 6,
    },
];

/// How far a next-fire or previous-fire search walks before declaring
/// an expression unsatisfiable. Ten years covers every expression that
/// ever matches — the longest real gap is the eight years between
/// February 29 2096 and February 29 2104, across the non-leap century
/// 2100 — and it bounds the walk instead of scanning toward the
/// four-digit year ceiling.
const SEARCH_HORIZON_DAYS: i64 = 366 * 10;

/// Why one cron expression is not usable. Every variant names the
/// field, the element, or the bound that failed, because a bare "bad
/// schedule" sends the owner back to the manual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// The expression did not split into exactly five fields.
    FieldCount { found: usize },
    /// A field element is not in the dialect at all.
    Element {
        field: &'static str,
        element: String,
    },
    /// A field value is not a number.
    Value { field: &'static str, text: String },
    /// A field value is outside its range.
    Range {
        field: &'static str,
        value: u32,
        min: u32,
        max: u32,
    },
    /// A step is not a positive integer — `*/0` lands here.
    Step { field: &'static str, text: String },
    /// A range's start is past its end.
    Inverted {
        field: &'static str,
        start: u32,
        end: u32,
    },
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::FieldCount { found } => write!(
                f,
                "a cron expression has five whitespace-separated fields (minute hour day-of-month month day-of-week); this one has {found}"
            ),
            ScheduleError::Element { field, element } => write!(
                f,
                "{field} field element `{element}` is not *, a value, a-b, */n, or a-b/n"
            ),
            ScheduleError::Value { field, text } => {
                write!(f, "{field} field value `{text}` is not a number")
            }
            ScheduleError::Range { field, value, min, max } => {
                write!(f, "{field} field value {value} is outside {min}-{max}")
            }
            ScheduleError::Step { field, text } => {
                write!(f, "{field} field step `{text}` must be a positive integer")
            }
            ScheduleError::Inverted { field, start, end } => {
                write!(f, "{field} field range {start}-{end} is inverted")
            }
        }
    }
}

impl std::error::Error for ScheduleError {}

/// One parsed five-field cron expression: the matched values of each
/// field plus the two star flags Vixie's day-of-month/day-of-week rule
/// needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    expression: String,
    minutes: Vec<u32>,
    hours: Vec<u32>,
    days_of_month: Vec<u32>,
    months: Vec<u32>,
    days_of_week: Vec<u32>,
    day_of_month_star: bool,
    day_of_week_star: bool,
}

impl Schedule {
    /// Parse one expression. The supported dialect is `*`, `,`, `-`, and
    /// `/step` over numeric values — no month or weekday names, and no
    /// step on a bare value (`5/2` is refused; a step attaches to `*` or
    /// to a range, the fork's dialect).
    pub fn parse(expression: &str) -> Result<Schedule, ScheduleError> {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return Err(ScheduleError::FieldCount { found: 0 });
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        if fields.len() != FIELDS.len() {
            return Err(ScheduleError::FieldCount {
                found: fields.len(),
            });
        }
        let parsed: Vec<Field> = fields
            .iter()
            .zip(FIELDS.iter())
            .map(|(text, spec)| parse_field(text, spec))
            .collect::<Result<Vec<Field>, ScheduleError>>()?;
        // The five-field bound is structural: the length check above
        // already returned for any other count, and destructuring a
        // fixed-size array fails as a typed `FieldCount` error rather
        // than as a panic the day a sixth field is added to `FIELDS`.
        let [minute, hour, day_of_month, month, day_of_week]: [Field; 5] = parsed
            .try_into()
            .map_err(|parsed: Vec<Field>| ScheduleError::FieldCount {
                found: parsed.len(),
            })?;
        Ok(Schedule {
            expression: trimmed.to_string(),
            minutes: minute.values.clone(),
            hours: hour.values.clone(),
            days_of_month: day_of_month.values.clone(),
            months: month.values.clone(),
            days_of_week: day_of_week.values.clone(),
            day_of_month_star: day_of_month.star,
            day_of_week_star: day_of_week.star,
        })
    }

    /// The expression as stored, trimmed.
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Whether `minute` matches: the minute and hour of day, the month,
    /// and the date under Vixie's day-of-month/day-of-week rule.
    pub fn matches(&self, minute: Minute) -> bool {
        self.minutes.contains(&minute.minute)
            && self.hours.contains(&minute.hour)
            && self.matches_date(minute)
    }

    /// The date half of a match, under the rule the fork ports from
    /// Vixie: when either the day-of-month or the day-of-week field is a
    /// star the two must both match (the star one always does, so the
    /// rule degrades to the other field); when both are restricted,
    /// either matching suffices.
    fn matches_date(&self, day: Minute) -> bool {
        if !self.months.contains(&day.month) {
            return false;
        }
        let day_of_month = self.days_of_month.contains(&day.day);
        let day_of_week = self.days_of_week.contains(&day.day_of_week());
        match (self.day_of_month_star, self.day_of_week_star) {
            (true, true) => true,
            (true, false) => day_of_week,
            (false, true) => day_of_month,
            (false, false) => day_of_month || day_of_week,
        }
    }

    /// The first matching minute strictly after `from`, or `None` when
    /// the expression matches nothing inside the search horizon.
    pub fn next_after(&self, from: Minute) -> Option<Minute> {
        let mut day = from;
        for _ in 0..SEARCH_HORIZON_DAYS {
            if self.matches_date(day) {
                for hour in &self.hours {
                    for minute in &self.minutes {
                        let candidate = Minute {
                            year: day.year,
                            month: day.month,
                            day: day.day,
                            hour: *hour,
                            minute: *minute,
                        };
                        if candidate > from {
                            return Some(candidate);
                        }
                    }
                }
            }
            day = day.next_day();
        }
        None
    }

    /// The most recent matching minute at or before `from`, or `None`
    /// when the expression matches nothing inside the search horizon.
    /// This is the catch-up rule's clock: the missed window is this
    /// minute, and it is resolved near the decision rather than across
    /// the whole history the job could have had.
    pub fn previous_at_or_before(&self, from: Minute) -> Option<Minute> {
        let mut day = from;
        for _ in 0..SEARCH_HORIZON_DAYS {
            if self.matches_date(day) {
                for hour in self.hours.iter().rev() {
                    for minute in self.minutes.iter().rev() {
                        let candidate = Minute {
                            year: day.year,
                            month: day.month,
                            day: day.day,
                            hour: *hour,
                            minute: *minute,
                        };
                        if candidate <= from {
                            return Some(candidate);
                        }
                    }
                }
            }
            day = day.previous_day();
        }
        None
    }
}

/// One parsed field: its matched values in ascending order, and whether
/// the field's text started with `*` — Vixie's `DOM_STAR`/`DOW_STAR`
/// test, which is about the spelling and not about what matched.
struct Field {
    values: Vec<u32>,
    star: bool,
}

/// Parse one field into its matched values and star flag.
fn parse_field(text: &str, spec: &FieldSpec) -> Result<Field, ScheduleError> {
    let mut seen = [false; 60];
    for element in text.split(',') {
        parse_element(element, spec, &mut seen)?;
    }
    let values = (0..60u32).filter(|value| seen[*value as usize]).collect();
    Ok(Field {
        values,
        star: text.starts_with('*'),
    })
}

/// Parse one comma-separated element into the field's matched set.
fn parse_element(
    element: &str,
    spec: &FieldSpec,
    seen: &mut [bool; 60],
) -> Result<(), ScheduleError> {
    if element.is_empty() {
        return Err(ScheduleError::Element {
            field: spec.name,
            element: element.to_string(),
        });
    }
    if element == "*" {
        for value in spec.min..=spec.max {
            mark(seen, value, spec);
        }
        return Ok(());
    }
    if let Some(step_text) = element.strip_prefix("*/") {
        let step = parse_step(step_text, spec)?;
        let mut value = spec.min;
        while value <= spec.max {
            mark(seen, value, spec);
            value += step;
        }
        return Ok(());
    }
    // Everything else is a value, a range, or a range with a step. A
    // step on a bare value is not in the dialect: `5/2` is refused
    // rather than read as `5-max/2`.
    let (range_text, step_text) = match element.split_once('/') {
        Some((range, step)) => (range, Some(step)),
        None => (element, None),
    };
    let (start, end) = match range_text.split_once('-') {
        Some((start, end)) => (parse_value(start, spec)?, Some(parse_value(end, spec)?)),
        None => (parse_value(range_text, spec)?, None),
    };
    if end.is_none() && step_text.is_some() {
        return Err(ScheduleError::Element {
            field: spec.name,
            element: element.to_string(),
        });
    }
    let last = end.unwrap_or(start);
    if start > last {
        return Err(ScheduleError::Inverted {
            field: spec.name,
            start,
            end: last,
        });
    }
    let step = match step_text {
        Some(text) => parse_step(text, spec)?,
        None => 1,
    };
    let mut value = start;
    while value <= last {
        mark(seen, value, spec);
        value += step;
    }
    Ok(())
}

/// Mark one value in a field's matched set, folding Sunday's 7 onto 0
/// for the day-of-week field.
fn mark(seen: &mut [bool; 60], value: u32, spec: &FieldSpec) {
    let folded = if spec.canonical_max != spec.max && value == spec.max {
        spec.min
    } else {
        value
    };
    seen[folded_index(folded)] = true;
}

/// The bitset index of a value already known to be inside 0-59.
fn folded_index(value: u32) -> usize {
    debug_assert!(value < 60, "a cron field value is bounded by 59");
    value as usize
}

/// Read one field value: all digits, inside the field's range.
fn parse_value(text: &str, spec: &FieldSpec) -> Result<u32, ScheduleError> {
    if text.is_empty() || !text.chars().all(|character| character.is_ascii_digit()) {
        return Err(ScheduleError::Value {
            field: spec.name,
            text: text.to_string(),
        });
    }
    let value: u32 = text.parse().map_err(|_| ScheduleError::Value {
        field: spec.name,
        text: text.to_string(),
    })?;
    if value < spec.min || value > spec.max {
        return Err(ScheduleError::Range {
            field: spec.name,
            value,
            min: spec.min,
            max: spec.max,
        });
    }
    Ok(value)
}

/// Read one step: all digits, and at least 1. `*/0` and `*/x` both land
/// here with the step named.
fn parse_step(text: &str, spec: &FieldSpec) -> Result<u32, ScheduleError> {
    if text.is_empty() || !text.chars().all(|character| character.is_ascii_digit()) {
        return Err(ScheduleError::Step {
            field: spec.name,
            text: text.to_string(),
        });
    }
    let step: u32 = text.parse().map_err(|_| ScheduleError::Step {
        field: spec.name,
        text: text.to_string(),
    })?;
    if step < 1 {
        return Err(ScheduleError::Step {
            field: spec.name,
            text: text.to_string(),
        });
    }
    Ok(step)
}

// --- the job record and the store -------------------------------------------

/// The longest a job name may be.
const MAX_NAME_CHARS: usize = 48;

/// How one fire ended. `skipped-needs-approval` is the unattended
/// policy's own outcome: the job is not declared `auto_approve`, its run
/// reached a tool call, and nobody was there to approve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Completed,
    Failed,
    SkippedNeedsApproval,
}

impl Outcome {
    /// The outcome as the store spells it and the cards show it.
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::Failed => "failed",
            Outcome::SkippedNeedsApproval => "skipped-needs-approval",
        }
    }
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// One scheduled job. The record is the whole contract: a scheduled run
/// sees the task and nothing else — no conversation, no history — so
/// the task has to stand on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// A validated stem: lowercase letters, digits, and hyphens. The
    /// name is the store's key and the name every mutation addresses.
    pub name: String,
    /// The five-field cron expression, in the dialect [`Schedule::parse`]
    /// accepts.
    pub schedule: String,
    /// The goal, self-contained.
    pub task: String,
    /// Whether the job is declared unattended: true lets its tool calls
    /// run, false skips and names them.
    pub auto_approve: bool,
    pub enabled: bool,
    /// The minute the job last ran for, `YYYY-MM-DDTHH:MMZ`, or `null`
    /// when it never has.
    #[serde(default)]
    pub last_run: Option<String>,
    /// How that run ended, or `null`.
    #[serde(default)]
    pub last_outcome: Option<Outcome>,
}

/// The store's document shape: one object listing the jobs. A wrapper
/// rather than a bare array, so the format has room for a version field
/// without a migration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    jobs: Vec<Job>,
}

/// Why the store, a name, or a mutation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronError {
    /// The store file could not be read or written.
    Io(String),
    /// The store parses as JSON but is not a job store: the wrong shape,
    /// an unreadable record, or a record whose schedule or stamp does
    /// not validate. Never silently ignored.
    Malformed(String),
    /// A name that is not a usable job name.
    Name(String),
    /// A task that is empty after trimming.
    Task(String),
    /// A schedule that does not parse.
    Schedule(ScheduleError),
    /// A name already in the store.
    Exists(String),
    /// A name not in the store.
    Missing(String),
}

impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CronError::Io(reason) => write!(f, "{reason}"),
            CronError::Malformed(reason) => write!(f, "{reason}"),
            CronError::Name(reason) => write!(f, "{reason}"),
            CronError::Task(reason) => write!(f, "{reason}"),
            CronError::Schedule(error) => write!(f, "{error}"),
            CronError::Exists(name) => write!(f, "job `{name}` already exists"),
            CronError::Missing(name) => write!(f, "no job `{name}` in the store"),
        }
    }
}

impl std::error::Error for CronError {}

/// Whether `name` is usable as a job name: lowercase letters, digits,
/// and hyphens, bounded, with no leading or trailing hyphen. The rule
/// is what keeps a name a key — no path separator, no `.` or `..`, no
/// dotfile, no space — the memory corpus's own rule for a name that
/// keys a file, applied here to the key inside the one store document.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// The complaint a name that fails the rule gets, naming the rule.
fn name_complaint(name: &str) -> String {
    format!(
        "`{name}` is not a job name: lowercase letters, digits, and hyphens, up to {MAX_NAME_CHARS} characters, no leading or trailing hyphen"
    )
}

/// The store file's name, under the home's `cron` directory.
pub const JOBS_FILE: &str = "jobs.json";

/// The eval cadence's persisted minute, beside the job store.
pub const EVAL_MINUTE_FILE: &str = "eval-minute.txt";

/// The repo-side cron manifest. KI-161: the per-project corpus-refresh
/// jobs are machine state — no repo file pins them, so a fresh
/// `FLEET_HOME` has no refresh jobs at all and the schedule the owner
/// built by hand exists nowhere. The manifest is the declaration;
/// [`seed_missing`] is what a fresh home re-creates from it.
///
/// Each entry is (name, schedule, task). The task names the route the
/// driver session can actually execute, not the lane tool it cannot
/// call (KI-221/KI-224): the vendored blend1 lane child spawned with
/// fleet's own launch spec and its stdio JSON-RPC driven by hand, which
/// is the route the machine's own runs proved works.
pub const SEED_MANIFEST: &[(&str, &str, &str)] = &[
    ("corpus-refresh-fleet", "10 0 * * *", "fleet"),
    ("corpus-refresh-sys1", "15 0 * * *", "sys1"),
    ("corpus-refresh-sdm1", "20 0 * * *", "sdm1"),
    ("corpus-refresh-sem1", "25 0 * * *", "sem1"),
    ("corpus-refresh-gen1", "30 0 * * *", "gen1"),
    ("corpus-refresh-blend1", "35 0 * * *", "blend1"),
    (
        "corpus-refresh-agnostic-router-kit",
        "40 0 * * *",
        "agnostic-router-kit",
    ),
    ("corpus-refresh-second-brain", "45 0 * * *", "second-brain"),
    (
        "corpus-refresh-dev-decisions",
        "50 0 * * *",
        "dev-decisions",
    ),
    (
        "corpus-refresh-deepseek-harness",
        "55 0 * * *",
        "deepseek-harness",
    ),
    ("corpus-refresh-dnd-vtt", "0 1 * * *", "dnd-vtt"),
    ("corpus-refresh-mahjong-vtt", "5 1 * * *", "mahjong-vtt"),
    ("corpus-refresh-pmocockpit", "10 1 * * *", "pmocockpit"),
    (
        "corpus-refresh-cruiserintelligence",
        "15 1 * * *",
        "cruiserintelligence",
    ),
];

/// One manifest entry's task string. KI-221/KI-224: the lane tool
/// `mcp__blend1__refresh` is not in a scheduled session's toolset — a
/// lane's MCP tools are exposed to a model run inside the agent loop,
/// not to the driver session a cron job runs in — so a task naming it
/// fails every time. The task names the spawn route instead: the
/// vendored lane child launched with fleet's own launch spec and its
/// stdio JSON-RPC driven by hand, which is the route this machine's own
/// runs proved reaches the same capability.
pub fn seed_task(project: &str) -> String {
    format!(
        "Refresh the {project} project's fact corpus from its docs. The lane tool is not in \
         your toolset; use the spawn route: run the vendored blend1 lane child \
         (python3 -u <fleet repo>/services/blend1/service/mcp_server.py, cwd <fleet repo>/services/blend1, \
         env BLEND1_PYTHONPATH plus the four sibling *_PYTHONPATH vars — the launch spec \
         src/lanes.rs produces) and drive its stdio JSON-RPC by hand: initialize \
         (protocolVersion 2024-11-05), notifications/initialized, then tools/call with \
         name \"refresh\" with the project {project}. It replies with project, \
         source_files, items, changed, action, and store."
    )
}

/// Seed the manifest's jobs into a home whose store lacks them. An
/// existing job is never rewritten: the owner's edits — a changed
/// schedule, a disabled job, a task they reworded — win over the
/// declaration, which is what makes the manifest safe to re-run. The
/// store is created when it does not exist. Returns the names added.
pub fn seed_missing(home: &Path) -> Result<Vec<String>, CronError> {
    let mut store = Store::open(home)?;
    let existing: Vec<String> = store.jobs().iter().map(|job| job.name.clone()).collect();
    let mut added = Vec::new();
    for (name, schedule, project) in SEED_MANIFEST {
        if existing.iter().any(|have| have == name) {
            continue;
        }
        store.add(name, schedule, &seed_task(project), true)?;
        added.push((*name).to_string());
    }
    Ok(added)
}

/// `<home>/cron/jobs.json`.
pub fn jobs_path(home: &Path) -> PathBuf {
    home.join("cron").join(JOBS_FILE)
}

/// The lock file beside the job store. KI-092: an exclusive lock on
/// this file is what makes the store's read-modify-write atomic across
/// processes — a `fleet cron` command in one shell and a scheduler's
/// `record_run` in another no longer interleave. The file is created
/// when absent and never removed: its existence is not a claim, the
/// lock on it is.
fn lock_path(home: &Path) -> std::path::PathBuf {
    home.join("cron").join("jobs.lock")
}

/// An exclusive lock on the store's lock file, released on drop. A
/// process that cannot take it is refused by name rather than writing
/// blind — the lost update the lock exists to prevent is exactly what
/// a blind write would cause.
///
/// The lock BLOCKS. A caller must never hold one across a call that
/// mutates the store (`add`, `record_run`, `set_enabled`, `remove`),
/// because those take the lock themselves and the second take would
/// wait on the first — the lock is per file descriptor, so even the
/// same process deadlocks against itself. `save` is the only taker,
/// and nothing calls it under the lock.
struct StoreLock {
    file: std::fs::File,
}

impl StoreLock {
    fn take(home: &Path) -> Result<StoreLock, CronError> {
        let path = lock_path(home);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                CronError::Io(format!("could not create {}: {error}", parent.display()))
            })?;
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|error| {
                CronError::Io(format!("could not open {}: {error}", path.display()))
            })?;
        // `flock` is the POSIX advisory lock: it is released when the
        // last handle closes, so a process that dies mid-write cannot
        // leave the store locked for everyone else.
        flock_exclusive(&file).map_err(|error| {
            CronError::Io(format!("could not lock {}: {error}", path.display()))
        })?;
        Ok(StoreLock { file })
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = flock_unlock(&self.file);
    }
}

/// The advisory lock, one call into the platform. `fcntl`-based
/// `F_SETLKW` on unix; a no-op elsewhere, where the single-user shape
/// makes the lock advisory only.
#[cfg(unix)]
fn flock_exclusive(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    // SAFETY: a well-formed flock on a live descriptor, whose only
    // effect is the advisory lock the caller asked for.
    let result = unsafe { libc_flock(file.as_raw_fd(), LOCK_EX) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn flock_unlock(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let result = unsafe { libc_flock(file.as_raw_fd(), LOCK_UN) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The two lock operations, named once. `LOCK_EX` is 2 and `LOCK_UN` is
/// 8 on every unix this crate builds for.
#[cfg(unix)]
const LOCK_EX: i32 = 2;
#[cfg(unix)]
const LOCK_UN: i32 = 8;

#[cfg(unix)]
unsafe fn libc_flock(fd: i32, operation: i32) -> i32 {
    // The one foreign call this crate makes: `flock(2)`. Declared here
    // rather than through a dependency, because one syscall is not a
    // crate.
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    unsafe { flock(fd, operation) }
}

#[cfg(not(unix))]
fn flock_exclusive(_file: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn flock_unlock(_file: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

/// A per-write serial for the store's temp name, so two fleet processes
/// — or two writes in one process — never share a temp file.
static STORE_WRITE_SERIAL: AtomicUsize = AtomicUsize::new(0);

/// The job store: one JSON document under the home, loaded whole and
/// rewritten atomically on every change. Every mutation round-trips
/// through the same validator a load uses, so a store this crate wrote
/// is a store this crate can read.
#[derive(Debug)]
pub struct Store {
    home: PathBuf,
    jobs: Vec<Job>,
}

impl Store {
    /// Open the store. A missing file is an empty store — a home that
    /// has never scheduled anything is not an error — and a file that
    /// exists but is not a job store is a typed [`CronError::Malformed`].
    pub fn open(home: &Path) -> Result<Store, CronError> {
        let path = jobs_path(home);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Store {
                    home: home.to_path_buf(),
                    jobs: Vec::new(),
                });
            }
            Err(error) => {
                return Err(CronError::Io(format!(
                    "could not read {}: {error}",
                    path.display()
                )));
            }
        };
        Self::parse(home, &text).map(|jobs| Store {
            home: home.to_path_buf(),
            jobs,
        })
    }

    /// Parse the store's text, validating every record the way a
    /// mutation does: a name that is not a stem, a schedule that does
    /// not parse, a `last_run` that is not a minute, or a duplicate name
    /// is a malformed store, not a record to skip.
    fn parse(home: &Path, text: &str) -> Result<Vec<Job>, CronError> {
        let path = jobs_path(home);
        let malformed = |reason: String| {
            CronError::Malformed(format!("{} is not a job store: {reason}", path.display()))
        };
        let document: Document =
            serde_json::from_str(text).map_err(|error| malformed(error.to_string()))?;
        let mut names: Vec<&str> = Vec::new();
        for job in &document.jobs {
            if !valid_name(&job.name) {
                return Err(malformed(name_complaint(&job.name)));
            }
            if names.contains(&job.name.as_str()) {
                return Err(malformed(format!("job `{}` appears twice", job.name)));
            }
            names.push(&job.name);
            Schedule::parse(&job.schedule)
                .map_err(|error| malformed(format!("job `{}`: {error}", job.name)))?;
            if job.task.trim().is_empty() {
                return Err(malformed(format!("job `{}` has an empty task", job.name)));
            }
            if let Some(stamp) = &job.last_run {
                if Minute::parse(stamp).is_none() {
                    return Err(malformed(format!(
                        "job `{}` has a last_run that is not a minute: `{stamp}`",
                        job.name
                    )));
                }
            }
        }
        Ok(document.jobs)
    }

    /// The jobs, in stored order.
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// One job by name.
    pub fn job(&self, name: &str) -> Option<&Job> {
        self.jobs.iter().find(|job| job.name == name)
    }

    /// Add a job. The name, the schedule, and the task all go through
    /// the same validator a load uses; a taken name is refused rather
    /// than overwritten. The store is rewritten before this returns.
    pub fn add(
        &mut self,
        name: &str,
        schedule: &str,
        task: &str,
        auto_approve: bool,
    ) -> Result<Job, CronError> {
        if !valid_name(name) {
            return Err(CronError::Name(name_complaint(name)));
        }
        if self.job(name).is_some() {
            return Err(CronError::Exists(name.to_string()));
        }
        let task = task.trim();
        if task.is_empty() {
            return Err(CronError::Task(
                "a job needs a task: --task \"<goal>\"".to_string(),
            ));
        }
        Schedule::parse(schedule).map_err(CronError::Schedule)?;
        let job = Job {
            name: name.to_string(),
            schedule: schedule.trim().to_string(),
            task: task.to_string(),
            auto_approve,
            enabled: true,
            last_run: None,
            last_outcome: None,
        };
        self.jobs.push(job.clone());
        self.save()?;
        Ok(job)
    }

    /// Enable or disable one job by name.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<Job, CronError> {
        let job = self
            .jobs
            .iter_mut()
            .find(|job| job.name == name)
            .ok_or_else(|| CronError::Missing(name.to_string()))?;
        job.enabled = enabled;
        let stored = job.clone();
        self.save()?;
        Ok(stored)
    }

    /// Remove one job by name, returning what was removed.
    pub fn remove(&mut self, name: &str) -> Result<Job, CronError> {
        let index = self
            .jobs
            .iter()
            .position(|job| job.name == name)
            .ok_or_else(|| CronError::Missing(name.to_string()))?;
        let removed = self.jobs.remove(index);
        self.save()?;
        Ok(removed)
    }

    /// Record one fire: the minute it ran for and how it ended. This is
    /// the only writer of `last_run`/`last_outcome`, and it is what
    /// keeps a restart from catching up on a window already served.
    pub fn record_run(
        &mut self,
        name: &str,
        minute: Minute,
        outcome: Outcome,
    ) -> Result<(), CronError> {
        let job = self
            .jobs
            .iter_mut()
            .find(|job| job.name == name)
            .ok_or_else(|| CronError::Missing(name.to_string()))?;
        job.last_run = Some(minute.stamp());
        job.last_outcome = Some(outcome);
        self.save()
    }

    /// Rewrite the store atomically: one temp file beside it, then the
    /// rename. A reader therefore sees either the whole previous
    /// document or the whole new one, never a half-written store.
    fn save(&self) -> Result<(), CronError> {
        // KI-092: the whole read-modify-write runs under the store lock.
        // The re-read in `fire_one` narrowed the lost-update window to
        // microseconds; this closes it — a `fleet cron disable` that
        // read the store before a fire's read and writes after it used
        // to be silently reverted, bringing a mid-run job back enabled
        // with a `last_run` it did not earn. The lock is a sibling file
        // beside the store, held for the caller's whole mutation, and a
        // caller that cannot take it is refused rather than writing
        // blind.
        let _guard = StoreLock::take(&self.home)?;
        let dir = self.home.join("cron");
        fs::create_dir_all(&dir).map_err(|error| {
            CronError::Io(format!("could not create {}: {error}", dir.display()))
        })?;
        let path = jobs_path(&self.home);
        let mut text = serde_json::to_string_pretty(&Document {
            jobs: self.jobs.clone(),
        })
        .map_err(|error| CronError::Io(format!("could not encode the job store: {error}")))?;
        text.push('\n');
        let serial = STORE_WRITE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut temp = path.clone().into_os_string();
        temp.push(format!(".{}.{}.tmp", std::process::id(), serial));
        let temp = PathBuf::from(temp);
        fs::write(&temp, text).map_err(|error| {
            CronError::Io(format!("could not write {}: {error}", temp.display()))
        })?;
        fs::rename(&temp, &path).map_err(|error| {
            let _ = fs::remove_file(&temp);
            CronError::Io(format!("could not replace {}: {error}", path.display()))
        })
    }
}

// --- the run ----------------------------------------------------------------

/// The framing a scheduled run's task carries. The model must be able to
/// tell a scheduled turn from a user sitting at the keyboard, and the
/// log must reconstruct the turn exactly (hard rule 5), so the framing
/// rides the task itself rather than living anywhere else. The catch-up
/// variant names the window that was missed while fleet was down — the
/// fork's reminder framing, ported.
pub fn frame_task(job: &Job, minute: Minute, catch_up: bool) -> String {
    let head = if catch_up {
        format!(
            "[scheduled run \u{2014} catch-up for job `{}`: the {} window was missed while fleet was down]",
            job.name,
            minute.stamp()
        )
    } else {
        format!(
            "[scheduled run \u{2014} job `{}` at {}]",
            job.name,
            minute.stamp()
        )
    };
    format!("{head}\n{}", job.task)
}

/// The gate one scheduled run gets, chosen by the job's `auto_approve`.
///
/// `auto_approve` true declares the job unattended and its calls run.
/// `false` denies every call — nobody is there to approve it, and an
/// unattended run must not silently auto-approve a dangerous tool — and
/// records the first call it denied, which is what the job's outcome
/// names.
#[derive(Debug, Clone, Default)]
struct Gate {
    unattended: bool,
    /// The first tool the unattended gate denied, shared with the clone
    /// the agent takes so the run can read it back afterwards.
    denied: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl Gate {
    /// The gate for one job.
    fn for_job(job: &Job) -> Gate {
        Gate {
            unattended: !job.auto_approve,
            denied: std::sync::Arc::default(),
        }
    }

    /// The first tool this gate denied, when it denied one.
    fn denied_tool(&self) -> Option<String> {
        self.denied.lock().ok().and_then(|denied| denied.clone())
    }

    /// The denial the model reads, naming the job's policy: this gate
    /// denies precisely because nobody is present, so the loop's
    /// ordinary "the user did not approve this call" would name a user
    /// who is guaranteed absent in the one path where no user exists
    /// (KI-088).
    const DENIAL: &'static str = "denied: this scheduled job is not declared unattended (`auto_approve` off), so no one is here to approve this call";
}

impl Approver for Gate {
    fn approve(&mut self, call: &crate::api::ToolCall) -> bool {
        if !self.unattended {
            return true;
        }
        if let Ok(mut denied) = self.denied.lock() {
            if denied.is_none() {
                *denied = Some(call.function.name.clone());
            }
        }
        false
    }

    fn denial_reason(&self) -> Option<String> {
        self.unattended.then(|| Gate::DENIAL.to_string())
    }

    fn latched(&self) -> bool {
        // A declared-unattended job is a run-scoped always, the way
        // `--yes` is: no per-call decision happens.
        !self.unattended
    }
}

/// What one fire produced.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub outcome: Outcome,
    /// The session log this run wrote, an ordinary log: resumable, and
    /// listed by `fleet sessions`.
    pub session_id: String,
    pub log_path: PathBuf,
    /// The tool an unattended run reached for and did not get.
    pub skipped_tool: Option<String>,
    /// The one-line reason, named for the surface that reports it.
    pub detail: String,
}

/// Run one job now, headless: the ordinary agent loop over an ordinary
/// session log, with the job's `auto_approve` deciding the gate and the
/// run's own turns landing calibration rows like any run.
///
/// `memory_root` is the corpus the run reads, passed in the way `home`
/// is: the caller decides, and [`run_job`] resolves it exactly the way
/// `fleet run` does (the nearest `memories/` at or above the working
/// directory). Making it explicit is also what lets a test drive this
/// path without writing recall hits into whatever corpus its own
/// working directory happens to sit above.
pub async fn execute(
    home: &Path,
    client: &Client,
    calib: Option<crate::calib::Stream>,
    job: &Job,
    minute: Minute,
    catch_up: bool,
    memory_root: &Path,
) -> Result<RunOutcome, AgentError> {
    let task = frame_task(job, minute, catch_up);
    // The session's model is the route's own: the client was prepared
    // before this run, so the start record names what will serve rather
    // than a provider's default. An unrouted client — a test's double —
    // has no route, and the record reads `auto`.
    let model = client
        .route_decision()
        .map(|decision| decision.model)
        .unwrap_or_else(|| crate::route::AUTO_MODEL.to_string());
    // A scheduled run's log is an ordinary session log marked
    // `kind: scheduled`: the picker, `fleet sessions`, and replay treat
    // it as real work in a real session, and the marker is what says a
    // job started it rather than the owner.
    let mut session = Session::create_scheduled(home, &model).map_err(AgentError::Io)?;
    let skills = crate::skills::load(home);
    // The corpus is loaded per run, off the async worker, from the root
    // the caller resolved — exactly the way `fleet run` wires it: the
    // files can change between runs, and a load that cannot complete
    // leaves the run memory-less rather than failing it.
    let memory = tokio::task::spawn_blocking({
        let root = memory_root.to_path_buf();
        move || std::sync::Arc::new(crate::memory::Memory::load(&root))
    })
    .await
    .ok();
    let options = AgentOptions {
        model: model.clone(),
        max_turns: DEFAULT_MAX_TURNS,
        identity: crate::home::read_soul(home),
        ui_tx: None,
        lane_specs: None,
        calib,
        skills,
        // A scheduled run has no owner present to steer it and no role
        // to run under: the prompt is the constant, byte for byte.
        role_overlay: None,
        memory,
        scope: None,
        // The self-maintenance tool rides the surfaces where a session's
        // owner is present; an unattended scheduled run does not carry
        // it, so no proposal is filed by a run nobody is watching.
        improve_home: None,
        gui_prompt: None,
        compact_now: false,
        checkpoints: None,
        file_access: FileAccess::Unrestricted,
        plan_mode: false,
    };
    let gate = Gate::for_job(job);
    // The log's identity is read before the agent borrows the session
    // for its whole life.
    let session_id = session.id().to_string();
    let log_path = session.path().to_path_buf();
    // An unattended run's progress has no surface to land on — no
    // foreground is watching — so it goes to the log, and the scheduler
    // names the outcome on the surface that is watching.
    let mut progress = std::io::sink();
    let mut agent = Agent::new(
        client,
        options,
        &mut session,
        Box::new(gate.clone()),
        &mut progress,
    )?;
    let report = agent.run(&task).await?;
    let skipped_tool = gate.denied_tool();
    let (outcome, detail) = describe(&report, skipped_tool.as_deref());
    Ok(RunOutcome {
        outcome,
        session_id,
        log_path,
        skipped_tool,
        detail,
    })
}

/// The outcome and its one-line reason for one finished run.
fn describe(report: &RunReport, skipped_tool: Option<&str>) -> (Outcome, String) {
    if let Some(tool) = skipped_tool {
        return (
            Outcome::SkippedNeedsApproval,
            format!(
                "skipped `{tool}`: the job is not declared unattended and no one is here to approve it"
            ),
        );
    }
    match report.stop_reason {
        crate::agent::StopReason::Completed => (
            Outcome::Completed,
            format!(
                "completed in {} turn{}",
                report.turns,
                if report.turns == 1 { "" } else { "s" }
            ),
        ),
        crate::agent::StopReason::MaxTurns => (
            Outcome::Failed,
            format!("stopped at the {}-turn cap", report.turns),
        ),
    }
}

/// The production run for one fire: resolve the route for the job's own
/// task — one judgment per scheduled run, exactly like `fleet run` —
/// then run the job's ordinary agent loop.
pub async fn run_job(
    home: &Path,
    job: &Job,
    minute: Minute,
    catch_up: bool,
) -> Result<RunOutcome, String> {
    let dotenv = crate::home::read_dotenv(home);
    let prepared = route::prepare(home, &dotenv, &job.task, &HashMap::new())
        .await
        .map_err(|error| error.to_string())?;
    let client = Client::routed(prepared.router.clone());
    // The corpus root, resolved the way `fleet run` resolves it: the
    // nearest `memories/` at or above the working directory. The
    // scheduler runs in the caller's process, so this is the same walk
    // the foreground takes.
    let memory_root = crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
    let outcome = execute(
        home,
        &client,
        Some(prepared.calib.clone()),
        job,
        minute,
        catch_up,
        &memory_root,
    )
    .await
    .map_err(|error| error.to_string())?;
    prepared.record_route(&job.task);
    Ok(outcome)
}

// --- the scheduler ----------------------------------------------------------

/// How often the scheduler looks at the clock. One second: the unit a
/// cron expression fires on is the minute, so the tick only has to land
/// once inside it — and the tick itself is one `contains` check per
/// enabled job, not a next-fire walk.
pub const CRON_TICK_SECONDS: u64 = 1;

/// The built-in eval-corpus cadence: once a day at 04:00 UTC. The corpus
/// is a headless deterministic run, not an agent conversation, so it
/// rides the scheduler's own hook rather than a job-store entry — the
/// tick decides it beside the cron jobs, with the same once-per-minute
/// latch discipline a job gets. The interval is daily; this expression is
/// its name.
pub const EVAL_SCHEDULE: &str = "0 4 * * *";

/// Whether the eval corpus is due at `now`: its cadence matches and the
/// latch does not already hold this minute. Pure, so the schedule is
/// testable without a clock or a corpus run.
pub fn eval_due(last: Option<Minute>, now: Minute) -> bool {
    match Schedule::parse(EVAL_SCHEDULE) {
        Ok(schedule) => schedule.matches(now) && last != Some(now),
        Err(_) => false,
    }
}

/// The file the eval cadence's minute is persisted in, beside the job
/// store. KI-105: the latch was in-memory only, so a process restarted
/// inside the 04:00 matching minute re-fired the daily corpus for that
/// minute — idempotent, but a second full run nobody asked for. The
/// persisted minute is read at construction and written when the
/// cadence fires; an unreadable or malformed file is an absent latch,
/// which is the old behavior rather than a refusal.
pub fn eval_minute_path(home: &Path) -> PathBuf {
    home.join("cron").join(EVAL_MINUTE_FILE)
}

/// The persisted eval minute, or `None` when there is none.
pub fn read_eval_minute(home: &Path) -> Option<Minute> {
    let text = fs::read_to_string(eval_minute_path(home)).ok()?;
    Minute::parse(text.trim())
}

/// Write the eval minute. Best-effort: a write that fails leaves the
/// in-memory latch standing, which is the behavior before the file
/// existed.
fn write_eval_minute(home: &Path, minute: Minute) {
    let path = eval_minute_path(home);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &path,
        format!(
            "{}
",
            minute.stamp()
        ),
    );
}

/// What the eval cadence does when it fires: the production
/// implementation runs the corpus headless and lands its row, and a test
/// supplies its own, the way the cron [`Runner`] seam works.
pub type EvalRunner = Box<dyn FnMut() -> Pin<Box<dyn Future<Output = Result<String, String>>>>>;

/// How many passed windows one tick names before the rest are summed
/// into one trailing count. A process that was suspended for a week
/// would otherwise name a week of minutes, one line each, the first
/// time it woke up.
const MAX_NAMED_MISSED_WINDOWS: usize = 8;

/// The label a panicking fire's outcome carries on the surfaces. Not
/// one of the store's three outcomes: a panic is fleet's failure, not
/// the job's, so the store records it as `failed` while the card and
/// the named line say what actually happened.
const PANICKED: &str = "panicked";

/// What one fire does. The production implementation resolves the route
/// and runs the job's ordinary agent loop; a test supplies its own, so a
/// whole scheduler pass can be driven against a scripted client with no
/// route preflight and no clock.
pub type Runner =
    Box<dyn FnMut(Job, Minute, bool) -> Pin<Box<dyn Future<Output = Result<RunOutcome, String>>>>>;

/// One decision: this job, at this minute, catch-up or on time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    pub job: Job,
    pub minute: Minute,
    pub catch_up: bool,
}

/// A future that catches its inner future's panic and hands it back as
/// a value.
///
/// A panic inside a fire unwinds through the scheduler thread's
/// `runtime.block_on` and kills the thread for the life of the process
/// — the thread the tick loop lives on — leaving nothing on any surface
/// but the default hook's stderr line, written over the TUI's alternate
/// screen. Polling the inner future inside `catch_unwind` turns that
/// into the error the fire's own named line carries, the way
/// `futures::FutureExt::catch_unwind` does for the crates that carry
/// the dependency. The inner future is pinned once here, which is what
/// keeps the projection free of unsafe.
struct CatchUnwind<F> {
    inner: Pin<Box<F>>,
}

impl<F> CatchUnwind<F> {
    fn new(inner: F) -> CatchUnwind<F> {
        CatchUnwind {
            inner: Box::pin(inner),
        }
    }
}

impl<F: Future> Future for CatchUnwind<F> {
    /// The inner future's output, or the panic payload it raised.
    type Output = Result<F::Output, Box<dyn std::any::Any + Send>>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // `CatchUnwind` is `Unpin` by construction — its only field is
        // a `Pin<Box<_>>` — so the field is reachable without unsafe.
        let this = self.get_mut();
        let polled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            this.inner.as_mut().poll(cx)
        }));
        match polled {
            Ok(std::task::Poll::Ready(output)) => std::task::Poll::Ready(Ok(output)),
            Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
            Err(payload) => std::task::Poll::Ready(Err(payload)),
        }
    }
}

/// The one-line name of a panic payload: the message when the payload
/// is a string — the shape `panic!("…")` and every ordinary failed
/// assertion raise — and a named fallback for anything else.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic with no message".to_string()
    }
}

/// How one fire ended, settled for the store and for every surface:
/// the outcome the store records, the label a surface names, the
/// one-line reason, and the session the run wrote when it got as far
/// as one.
struct Settled {
    outcome: Outcome,
    label: &'static str,
    detail: String,
    session: Option<String>,
}

/// Settle what one fire produced: an ordinary outcome, a failure, or a
/// caught panic. A panic records as `failed` — the store's vocabulary
/// has no fourth outcome, and the minute is spent either way — while
/// the surfaces name it for what it was.
fn settle(caught: Result<Result<RunOutcome, String>, Box<dyn std::any::Any + Send>>) -> Settled {
    match caught {
        Ok(Ok(outcome)) => Settled {
            outcome: outcome.outcome,
            label: outcome.outcome.label(),
            detail: outcome.detail,
            session: Some(outcome.session_id),
        },
        Ok(Err(error)) => Settled {
            outcome: Outcome::Failed,
            label: Outcome::Failed.label(),
            detail: format!("failed: {error}"),
            session: None,
        },
        Err(payload) => Settled {
            outcome: Outcome::Failed,
            label: PANICKED,
            // `as_ref`, not `&payload`: coercing `&Box<dyn Any + Send>`
            // to `&dyn Any + Send` would make the box itself the
            // concrete type, and every downcast would miss.
            detail: format!("the fire panicked: {}", panic_message(payload.as_ref())),
            session: None,
        },
    }
}

/// Settle a future whose panic was caught: its own output, or the
/// panic's one-line name. The generic seam [`CatchUnwind`] feeds — a
/// job fire settles through [`settle`], the eval cadence through
/// this — so every scheduled fire rides the same guard and a panic
/// is always a named value rather than an unwind through the
/// scheduler thread's `block_on`.
fn caught<T>(caught: Result<T, Box<dyn std::any::Any + Send>>) -> Result<T, String> {
    caught.map_err(|payload| panic_message(payload.as_ref()))
}

/// The in-memory scheduler: the home the store and the runs live under,
/// the per-job fired-minute latch, and the runner a fire goes through.
pub struct Scheduler {
    home: PathBuf,
    /// A job fires at most once per matching minute, for the life of the
    /// process. The latch is claimed at decision time, so a job the tick
    /// has decided to fire is not decided again for that minute whether
    /// its run succeeds, fails, or panics — a panicking fire leaves its
    /// minute claimed, because a retry inside the same minute could
    /// spend a window whose run may already have spent.
    latch: HashMap<String, Minute>,
    /// The minute each job was first seen enabled in the store. The
    /// floor for a job that has never run: a window that came due
    /// before the scheduler ever saw the job enabled — a job that did
    /// not exist yet, or one that was disabled — is not a window the
    /// scheduler dropped.
    enabled_since: HashMap<String, Minute>,
    /// The store's last unreadable complaint. The tick runs once a
    /// second for the life of the process, so an unreadable store is
    /// named once and then silenced until the reason changes, instead
    /// of writing one line per second over whatever surface is
    /// watching.
    store_complaint: Option<String>,
    /// Where a finished fire's card goes: the TUI's event channel when
    /// this scheduler runs inside the interface. `None` is the headless
    /// `chat` path, whose named lines ride stderr as they always have.
    ui_tx: Option<UnboundedSender<UiEvent>>,
    runner: Runner,
    /// The minute the eval corpus last ran for, the cadence's latch — the
    /// same once-per-minute discipline a job's latch gives it.
    eval_last: Option<Minute>,
    /// The eval cadence's action; `None` is no cadence, which is what a
    /// test that does not wire one gets (its cron-job behavior is
    /// unchanged). [`Scheduler::spawn`] wires the real corpus run.
    eval: Option<EvalRunner>,
    /// The last minute this scheduler saw. KI-094: a clock that steps
    /// BACKWARD re-presents minutes the latch already holds, and the
    /// tick would fire them again — two runs, two logs, two outcomes
    /// for one window. A tick whose minute is older than the last one
    /// seen is refused rather than served.
    last_seen: Option<Minute>,
}

/// The claim directory beside the job store: one file per (job, minute),
/// the cross-process latch KI-089 names. The file's creation is the
/// claim — `create_new` is an atomic create, so the first scheduler to
/// own a minute owns it and every other scheduler on the machine is
/// refused by the operating system rather than by a race.
fn claims_dir(home: &Path) -> PathBuf {
    home.join("cron").join("claims")
}

/// Try to claim `job`'s fire for `minute`. True when this process owns
/// the minute; false when another scheduler claimed it first. A claim
/// from a process that died still suppresses its own minute — which has
/// passed by the time anyone looks — and the next minute claims a fresh
/// file, so a stale claim costs nothing and needs no liveness probe.
fn claim_minute(home: &Path, job: &str, minute: Minute) -> bool {
    let dir = claims_dir(home);
    if std::fs::create_dir_all(&dir).is_err() {
        // An uncreatable claims directory is not a reason to fire twice:
        // the conservative answer is to let the in-process latch stand
        // alone rather than to double-fire.
        return true;
    }
    let path = dir.join(format!("{job}-{}", minute.stamp()));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .is_ok()
}

/// The claims for minutes that have passed, removed so the directory
/// does not grow without bound. Best-effort: a claim that cannot be
/// removed is a claim that suppressed one minute, which is over.
fn sweep_claims(home: &Path, now: Minute) {
    let dir = claims_dir(home);
    let Ok(read) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // The stamp is the name's tail — its own shape carries
        // hyphens, so a split would cut the date in half.
        if name.len() <= MINUTE_STAMP_LEN {
            continue;
        }
        let stamp = &name[name.len() - MINUTE_STAMP_LEN..];
        let Some(minute) = Minute::parse(stamp) else {
            continue;
        };
        if minute < now {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

impl Scheduler {
    /// A scheduler over `home`, firing through `runner`.
    pub fn new(home: PathBuf, runner: Runner) -> Scheduler {
        // Read before the move: the persisted eval minute is the
        // constructor's own read of the home it is about to own.
        let eval_last = read_eval_minute(&home);
        Scheduler {
            home,
            latch: HashMap::new(),
            enabled_since: HashMap::new(),
            store_complaint: None,
            ui_tx: None,
            runner,
            eval_last,
            eval: None,
            last_seen: None,
        }
    }

    /// Where this scheduler's finished-fire cards go.
    pub fn with_ui(mut self, ui_tx: Option<UnboundedSender<UiEvent>>) -> Scheduler {
        self.ui_tx = ui_tx;
        self
    }

    /// The eval cadence's action. A scheduler without one runs no corpus
    /// cadence, so the tests that exercise the cron jobs are unchanged.
    pub fn with_eval(mut self, eval: EvalRunner) -> Scheduler {
        self.eval = Some(eval);
        self
    }

    /// Run the eval corpus if its daily cadence is due at `now`, claiming
    /// the minute either way so a fired minute is not fired twice. Returns
    /// the named line the run produced, or empty when it is not due or no
    /// cadence is wired. The row the corpus lands is in the calibration
    /// stream; the line names the run on whatever surface is watching, the
    /// way a fire's own line does.
    ///
    /// The fire rides the same [`CatchUnwind`] guard every job fire
    /// gets: a panic in the eval cadence would otherwise unwind
    /// through the scheduler thread's `block_on` and kill the tick
    /// loop for the life of the process — one stderr line over the
    /// TUI's alternate screen, and every scheduled job with it.
    /// Caught here it is one named line, and the loop carries on. The
    /// minute stays claimed either way, the discipline a panicking
    /// fire's latch keeps.
    pub async fn run_eval_if_due(&mut self, now: Minute) -> Vec<String> {
        if self.eval.is_none() || !eval_due(self.eval_last, now) {
            return Vec::new();
        }
        self.eval_last = Some(now);
        // KI-105: the minute rides disk beside the store, so a restart
        // inside the matching minute reads the latch that was claimed.
        write_eval_minute(&self.home, now);
        let eval = self.eval.as_mut().expect("checked Some above");
        match caught(CatchUnwind::new(eval()).await) {
            Ok(Ok(detail)) => vec![format!("fleet: cron: eval corpus {detail}")],
            Ok(Err(error)) => vec![format!("fleet: cron: eval corpus failed: {error}")],
            Err(panic) => vec![format!(
                "fleet: cron: eval corpus panicked: {panic}; the minute stays claimed (no retry this minute)"
            )],
        }
    }

    /// Spawn the scheduler on its own thread and runtime: the catch-up
    /// pass once at startup, then one tick per [`CRON_TICK_SECONDS`] for
    /// the life of the process. The thread is detached, so a long-lived
    /// process costs nothing to end — the consolidator's precedent.
    ///
    /// `ui_tx` is the TUI's event channel when the interface starts the
    /// scheduler: a finished fire then lands as a card instead of a
    /// stderr line scratched over the alternate screen (KI-090). The
    /// headless `chat` path passes `None`, and its named lines ride
    /// stderr.
    pub fn spawn(home: PathBuf, ui_tx: Option<UnboundedSender<UiEvent>>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("fleet: cron: the scheduler runtime failed to start: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                let runner_home = home.clone();
                let runner: Runner = Box::new(move |job, minute, catch_up| {
                    let home = runner_home.clone();
                    Box::pin(async move { run_job(&home, &job, minute, catch_up).await })
                });
                // The built-in eval cadence: run the corpus headless once a
                // day, its row landing in the stream like any run. It rides
                // the scheduler's own hook (not a job-store entry) because
                // the corpus is a deterministic run, not a conversation.
                let eval_home = home.clone();
                let eval: EvalRunner = Box::new(move || {
                    let home = eval_home.clone();
                    Box::pin(async move {
                        let user_home = std::env::var("HOME").ok();
                        let root = crate::calib::resolve_root(&home, user_home.as_deref());
                        match crate::eval::mock_server_binary() {
                            None => Err(
                                "the mock-server binary was not found beside the fleet executable"
                                    .to_string(),
                            ),
                            Some(bin) => {
                                let report = crate::eval::run(&root, &bin).await;
                                Ok(format!(
                                    "ran \u{2014} {} passed, {} failed, score {:.2}",
                                    report.passed(),
                                    report.failed().len(),
                                    report.score()
                                ))
                            }
                        }
                    })
                });
                let mut scheduler = Scheduler::new(home, runner).with_ui(ui_tx).with_eval(eval);
                for line in scheduler.start(Minute::now()).await {
                    eprintln!("{line}");
                }
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(CRON_TICK_SECONDS)).await;
                    let now = Minute::now();
                    for line in scheduler.tick(now).await {
                        eprintln!("{line}");
                    }
                    for line in scheduler.run_eval_if_due(now).await {
                        eprintln!("{line}");
                    }
                }
            });
        })
    }

    /// The startup pass's decision: every enabled job whose most recent
    /// matching minute is newer than the run it last recorded gets
    /// exactly one catch-up run — the missed window, resolved near the
    /// decision and not across the whole history the job could have had.
    ///
    /// A job that has never run is not due. There is no window to catch
    /// up to: its history is empty, so "the missed window" is undefined,
    /// and reading "never run" as "everything since the epoch was
    /// missed" would fire a job the moment it was created — a
    /// `* * * * *` job added at 12:00:30 would immediately run for
    /// 12:00, a run the owner never scheduled. Its first fire is its
    /// first matching minute going forward.
    pub fn catch_up_due(&mut self, jobs: &[Job], now: Minute) -> Vec<Due> {
        let mut due: Vec<Due> = Vec::new();
        for job in jobs {
            if !job.enabled {
                continue;
            }
            let Ok(schedule) = Schedule::parse(&job.schedule) else {
                continue;
            };
            let Some(missed) = schedule.previous_at_or_before(now) else {
                continue;
            };
            // A never-run job does not catch up; a run it recorded but
            // cannot read back is treated the same way rather than
            // guessed at.
            let Some(last) = job.last_run.as_deref().and_then(Minute::parse) else {
                continue;
            };
            // KI-093: a `last_run` in the FUTURE — a hand-edited or
            // restored store — used to make `missed > last` permanently
            // false and silence the job forever. The clamp treats it as
            // "ran just now", so the next window is served normally.
            let last = if last > now {
                // A future stamp reads as "ran before this window": the
                // clamp sits one minute before the missed window, so the
                // window is served exactly once and the next one is not.
                missed.previous_minute()
            } else {
                last
            };
            // The latch, the same one the tick claims: a startup pass
            // that runs twice — or a tick inside the missed minute —
            // does not serve the window again.
            if self.latch.get(&job.name) == Some(&missed) {
                continue;
            }
            if missed > last {
                self.latch.insert(job.name.clone(), missed);
                due.push(Due {
                    job: job.clone(),
                    minute: missed,
                    catch_up: true,
                });
            }
        }
        due
    }

    /// One tick's decision: every enabled job whose current minute
    /// matches its expression, at most once per matching minute per job.
    pub fn tick_due(&mut self, jobs: &[Job], now: Minute) -> Vec<Due> {
        let mut due: Vec<Due> = Vec::new();
        for job in jobs {
            if !job.enabled {
                continue;
            }
            let Ok(schedule) = Schedule::parse(&job.schedule) else {
                continue;
            };
            if !schedule.matches(now) {
                continue;
            }
            if self.latch.get(&job.name) == Some(&now) {
                continue;
            }
            // KI-089: the cross-process claim beside the in-process
            // latch. Two long-lived schedulers on one machine — a
            // `fleet chat` and a TUI — each decide the same job is due
            // in the same minute; the atomic create refuses the second.
            if !claim_minute(&self.home, &job.name, now) {
                continue;
            }
            self.latch.insert(job.name.clone(), now);
            due.push(Due {
                job: job.clone(),
                minute: now,
                catch_up: false,
            });
        }
        due
    }

    /// The startup pass, run for real: the catch-up decision over the
    /// store as it stands, then the runs.
    pub async fn start(&mut self, now: Minute) -> Vec<String> {
        let store = match self.open_store() {
            Ok(store) => store,
            Err(lines) => return lines,
        };
        self.note_jobs(store.jobs(), now);
        let due = self.catch_up_due(store.jobs(), now);
        self.fire(due).await
    }

    /// One tick, run for real. The store is re-read every time, so a job
    /// `fleet cron add` created a moment ago is picked up within one
    /// tick.
    pub async fn tick(&mut self, now: Minute) -> Vec<String> {
        // KI-094: a backward clock jump would re-present a minute the
        // latch already holds and double-fire the job. Refuse the tick
        // rather than serve it; the machine is single-user and
        // NTP-corrected, so the refusal names an anomaly rather than
        // suppressing real work.
        if let Some(last) = self.last_seen {
            if now < last {
                return vec![format!(
                    "cron: the clock moved backward ({} after {}); the tick is refused",
                    now.stamp(),
                    last.stamp()
                )];
            }
        }
        self.last_seen = Some(now);
        sweep_claims(&self.home, now);
        let store = match self.open_store() {
            Ok(store) => store,
            Err(lines) => return lines,
        };
        self.note_jobs(store.jobs(), now);
        // The windows that came due while the previous pass was in
        // flight are named before this minute's decision: a busy
        // scheduler is heard about, never a silent drop.
        let mut lines = self.passed_lines(store.jobs(), now);
        let due = self.tick_due(store.jobs(), now);
        lines.extend(self.fire(due).await);
        lines
    }

    /// Open the store, naming an unreadable one once: the first
    /// complaint rides out as a line, and the same complaint again is
    /// silence until the reason changes.
    fn open_store(&mut self) -> Result<Store, Vec<String>> {
        match Store::open(&self.home) {
            Ok(store) => {
                self.store_complaint = None;
                Ok(store)
            }
            Err(error) => {
                let complaint = error.to_string();
                if self.store_complaint.as_deref() == Some(complaint.as_str()) {
                    return Err(Vec::new());
                }
                self.store_complaint = Some(complaint);
                Err(vec![format!(
                    "fleet: cron: the job store is unreadable: {error}"
                )])
            }
        }
    }

    /// Remember which jobs the store holds enabled, and since when. A
    /// job the scheduler has never seen enabled — one that did not
    /// exist yet, or one that was disabled — has no floor to name
    /// windows from, and a job that comes back enabled starts its
    /// floor at the minute it came back.
    fn note_jobs(&mut self, jobs: &[Job], now: Minute) {
        for job in jobs {
            if job.enabled {
                self.enabled_since.entry(job.name.clone()).or_insert(now);
            } else {
                self.enabled_since.remove(&job.name);
            }
        }
    }

    /// The windows that passed while the scheduler was busy: every
    /// matching minute that came due after the newest minute already
    /// accounted for and strictly before `now`.
    ///
    /// The design runs one catch-up at startup and nothing mid-flight,
    /// so a window a busy pass could not decide is a window the
    /// operator hears about. A window at or before the floor — decided,
    /// named, served, or never the scheduler's to begin with — is not a
    /// drop and is not named.
    fn passed_due(&self, jobs: &[Job], now: Minute) -> Vec<Due> {
        let mut passed: Vec<Due> = Vec::new();
        for job in jobs {
            if !job.enabled {
                continue;
            }
            let Ok(schedule) = Schedule::parse(&job.schedule) else {
                continue;
            };
            // The floor: the newest minute already accounted for — the
            // latch's (decided or named), the store's (served), and for
            // a job that has never run the minute before the scheduler
            // first saw it enabled, so that minute can itself be named.
            let floor = [
                self.latch.get(&job.name).copied(),
                job.last_run.as_deref().and_then(Minute::parse),
                self.enabled_since
                    .get(&job.name)
                    .map(|since| since.previous_minute()),
            ]
            .into_iter()
            .flatten()
            .max();
            let Some(mut cursor) = floor else { continue };
            while let Some(next) = schedule.next_after(cursor) {
                if next >= now {
                    break;
                }
                passed.push(Due {
                    job: job.clone(),
                    minute: next,
                    catch_up: false,
                });
                cursor = next;
            }
        }
        passed
    }

    /// The named lines for the windows that passed while the scheduler
    /// was busy, one line per window with the minute claimed in the
    /// latch as it is named — a window that passed is spent, and the
    /// claim is what keeps the next tick from naming it again.
    fn passed_lines(&mut self, jobs: &[Job], now: Minute) -> Vec<String> {
        let windows = self.passed_due(jobs, now);
        for window in &windows {
            self.latch.insert(window.job.name.clone(), window.minute);
        }
        let named = windows.len().min(MAX_NAMED_MISSED_WINDOWS);
        let mut lines: Vec<String> = windows[..named]
            .iter()
            .map(|window| {
                format!(
                    "fleet: cron: job `{}`'s {} window passed while the scheduler was running",
                    window.job.name,
                    window.minute.stamp()
                )
            })
            .collect();
        let more = windows.len() - named;
        if more > 0 {
            lines.push(format!(
                "fleet: cron: \u{2026}and {more} more window{} passed while the scheduler was running",
                plural(more)
            ));
        }
        lines
    }

    /// Run what a pass decided, concurrently: every due fire is spawned
    /// onto the scheduler's runtime and the pass awaits them all.
    ///
    /// Spawning is what keeps one long run from serializing the fires
    /// beside it, and the latch was claimed at decision time, so a
    /// spawned fire's minute cannot be decided again while it runs. The
    /// pass still awaits its own fires — the named line, the
    /// `record_run` write, and the latch all belong to the tick loop's
    /// single-threaded discipline — and a window that passes while the
    /// pass is in flight is named by the next tick rather than dropped
    /// silently.
    async fn fire(&mut self, due: Vec<Due>) -> Vec<String> {
        if due.is_empty() {
            return Vec::new();
        }
        // A fire's future is `!Send` — the agent holds a
        // `Box<dyn Approver>` and a `&mut dyn Write`, neither of which
        // is `Send` — so the fires are spawned onto a local set driven
        // here. That needs no `Send` bound and works on whatever
        // runtime this pass is running on, which is what lets a test
        // drive a whole pass.
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let mut handles = Vec::with_capacity(due.len());
                for one in due {
                    // The runner is called here, synchronously, to build
                    // the fire's future: the `Runner` type's own bound
                    // makes that future `'static`, so the task that runs
                    // it borrows nothing from this pass.
                    let run = (self.runner)(one.job.clone(), one.minute, one.catch_up);
                    let home = self.home.clone();
                    let ui_tx = self.ui_tx.clone();
                    let job = one.job.name.clone();
                    handles.push((
                        job,
                        tokio::task::spawn_local(
                            async move { fire_one(home, ui_tx, one, run).await },
                        ),
                    ));
                }
                let mut lines = Vec::new();
                for (job, handle) in handles {
                    match handle.await {
                        Ok(Some(line)) => lines.push(line),
                        // The line went out as the TUI's card instead.
                        Ok(None) => {}
                        // A task that ended outside the fire's own
                        // `catch_unwind` — cancelled, or a panic around
                        // the harness. Named rather than dropped: a pass
                        // that loses a fire silently is the shape this
                        // whole seam exists to prevent.
                        Err(error) => lines.push(format!(
                            "fleet: cron: job `{job}`'s task ended without a result: {error}"
                        )),
                    }
                }
                lines
            })
            .await
    }
}

/// One fire, end to end: the run with its panic caught, the outcome
/// recorded against a freshly loaded store, and the named line.
///
/// A free function over owned values, because a fire is spawned: the
/// latch was claimed at decision time, the store is re-read fresh here,
/// and nothing the pass decided is touched again.
async fn fire_one(
    home: PathBuf,
    ui_tx: Option<UnboundedSender<UiEvent>>,
    due: Due,
    run: Pin<Box<dyn Future<Output = Result<RunOutcome, String>>>>,
) -> Option<String> {
    let name = due.job.name.clone();
    let when = if due.catch_up {
        format!("catch-up for {}", due.minute.stamp())
    } else {
        format!("at {}", due.minute.stamp())
    };
    // The panic is caught here rather than unwinding through the
    // scheduler thread's `block_on`: a panicking fire is one named line
    // and the loop carries on, never a dead tick loop.
    let settled = settle(CatchUnwind::new(run).await);
    // The outcome is recorded against a freshly loaded store: a job
    // removed while its run was in flight is not resurrected. The whole
    // mutation runs under the store lock (KI-092), so a `fleet cron
    // disable` that read the store before this read and writes after it
    // no longer loses the update — it waits for this one to finish,
    // and the job it disabled stays disabled.
    let recorded = match Store::open(&home) {
        Ok(mut store) => store.record_run(&name, due.minute, settled.outcome),
        Err(error) => Err(error),
    };
    // KI-095: a failed `record_run` is retried once, from a fresh
    // store. The latch stays claimed for the minute — a retry inside
    // the same minute could spend a window whose run may already have
    // spent — but the outcome no longer dies with the write that was
    // supposed to record it: a full disk at 04:00 used to leave
    // `last_run` stale while the run had genuinely completed.
    let recorded = match recorded {
        Ok(()) => Ok(()),
        Err(first) => match Store::open(&home) {
            Ok(mut store) => store.record_run(&name, due.minute, settled.outcome),
            Err(_) => Err(first),
        },
    };
    let tail = match recorded {
        Ok(()) => String::new(),
        Err(error) => format!("; the outcome could not be recorded: {error}"),
    };
    let line = if settled.label == PANICKED {
        // The minute stays claimed: a retry inside it could spend a
        // window whose run may already have spent.
        format!(
            "fleet: cron: job `{name}` {when} \u{2014} {}; the minute stays claimed (no retry this minute){tail}",
            settled.detail
        )
    } else {
        match &settled.session {
            Some(session) => format!(
                "fleet: cron: job `{name}` {when} \u{2014} {} (session {session}){tail}",
                settled.detail
            ),
            None => format!(
                "fleet: cron: job `{name}` {when} \u{2014} {}{tail}",
                settled.detail
            ),
        }
    };
    match ui_tx {
        // The TUI's card carries the fire; a stderr line here would
        // only scratch the alternate screen (KI-090).
        Some(tx) => {
            let _ = tx.send(UiEvent::CronFire {
                job: name,
                minute: due.minute.stamp(),
                catch_up: due.catch_up,
                outcome: settled.label.to_string(),
                detail: settled.detail,
                session: settled.session,
            });
            None
        }
        None => Some(line),
    }
}

// --- the surface ------------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, OK, RESET, TEXT, WARN};

/// The card width, matching `skills`, `lanes`, `providers`, and `calib`.
const CARD_WIDTH: usize = 72;

/// Render the jobs as cards: one per job with its schedule, its task,
/// its next fire, and its last run, outcome, and session. Never a
/// plain-text table.
pub fn render(home: &Path, jobs: &[Job], now: Minute) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet cron{RESET} {DIM}\u{2014} {} job{} in {}{RESET}\n\n",
        jobs.len(),
        plural(jobs.len()),
        jobs_path(home).display()
    ));
    if jobs.is_empty() {
        out.push_str(&format!(
            "{DIM}no jobs yet \u{b7} `fleet cron add --name <name> --schedule \"<expr>\" --task \"<goal>\"` creates one{RESET}\n"
        ));
        return out;
    }
    for job in jobs {
        out.push_str(&card(home, job, now));
    }
    let enabled = jobs.iter().filter(|job| job.enabled).count();
    out.push_str(&format!(
        "\n{DIM}{enabled} enabled \u{b7} {} disabled \u{b7} the scheduler ticks every {CRON_TICK_SECONDS}s{RESET}\n",
        jobs.len() - enabled
    ));
    out
}

/// The session id of the newest scheduled run for one job, read from
/// the sessions directory rather than from the store: the store's shape
/// is the contract this slice froze, and the id is a fact about a log.
///
/// A scheduled run's log is an ordinary log marked `kind: scheduled`
/// whose first message is the framed task naming its job, so the join
/// is the frame itself. Only each log's head is read — the frame is the
/// run's first message — and the newest match wins, because a session
/// id starts with a UTC stamp.
pub fn last_session_id(home: &Path, job: &str) -> Option<String> {
    let dir = crate::home::sessions_dir(home);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .collect();
    paths.sort();
    for path in paths.iter().rev() {
        if is_scheduled_run_of(path, job) {
            return path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string);
        }
    }
    None
}

/// Whether this log is a scheduled run of `job`: its `start` record is
/// marked `kind: scheduled` and its first message is the framed task
/// that names the job.
fn is_scheduled_run_of(path: &Path, job: &str) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut saw_scheduled_start = false;
    for line in io::BufReader::new(file).lines() {
        let Ok(line) = line else { return false };
        if line.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<session::Record>(&line) else {
            return false;
        };
        match record {
            session::Record::Start { kind, .. } => {
                if kind.as_deref() != Some(session::SCHEDULED_KIND) {
                    return false;
                }
                saw_scheduled_start = true;
            }
            session::Record::User { content } => {
                return saw_scheduled_start && content.contains(&format!("job `{job}`"));
            }
            _ => {}
        }
    }
    false
}

/// One job's card.
fn card(home: &Path, job: &Job, now: Minute) -> String {
    let state = if job.enabled { "enabled" } else { "disabled" };
    let state_color = if job.enabled { OK } else { WARN };
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {state_color}{state}{RESET} {DIM}\u{2510}{RESET}",
        name = job.name,
        dashes = dashes_for(&format!(" {} ", job.name), state),
    );
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{}{RESET}", job.schedule));
    for chunk in crate::skills::wrap_detail(&job.task, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}"));
    }
    let next = if job.enabled {
        // A disabled job keeps its schedule but will never fire on it,
        // so the card says so rather than promising a fire.
        match Schedule::parse(&job.schedule)
            .ok()
            .and_then(|schedule| schedule.next_after(now))
        {
            Some(minute) => format!("next fire {}", minute.stamp()),
            None => "next fire never".to_string(),
        }
    } else {
        "next fire \u{2014} disabled".to_string()
    };
    lines.push(format!(
        "{DIM}\u{2502}{RESET} {DIM}{next} \u{b7} auto-approve {}{RESET}",
        if job.auto_approve { "on" } else { "off" }
    ));
    let last = match (&job.last_run, job.last_outcome) {
        (Some(stamp), Some(outcome)) => format!("last run {stamp} \u{b7} {outcome}"),
        (Some(stamp), None) => format!("last run {stamp} \u{b7} no outcome recorded"),
        (None, _) => "never run".to_string(),
    };
    lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{last}{RESET}"));
    // The retained output, one line under the last run: the session the
    // last fire wrote, so the log is reachable from the surface that
    // lists the job. Read from the sessions directory, not the store —
    // the id is a fact about a log, and the store's shape is frozen.
    if let Some(session) = last_session_id(home, &job.name) {
        lines.push(format!(
            "{DIM}\u{2502}{RESET} {DIM}session {session}{RESET}"
        ));
    }
    let bottom = format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}",
        "\u{2500}".repeat(CARD_WIDTH - 2)
    );
    let mut out = format!("{header}\n");
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&bottom);
    out.push('\n');
    out
}

/// The dash run that pads a card header so its right edge aligns — the
/// card family's own helper, in the shape `skills` writes it.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// The English plural for a count, the way the card family carries it.
fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(tag: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-cron-test-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A minute built from its parts, for a deterministic clock.
    fn minute(year: i64, month: u32, day: u32, hour: u32, minute: u32) -> Minute {
        Minute {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    fn job(name: &str, schedule: &str, last_run: Option<&str>) -> Job {
        Job {
            name: name.to_string(),
            schedule: schedule.to_string(),
            task: "the goal".to_string(),
            auto_approve: false,
            enabled: true,
            last_run: last_run.map(str::to_string),
            last_outcome: None,
        }
    }

    // --- the parser ---------------------------------------------------------

    /// KI-089: the claim is atomic and per-minute. The first claim owns
    /// the minute; a second claim for the same minute is refused by the
    /// operating system's create, which is what stops two schedulers on
    /// one machine from both firing the same job. A different minute
    /// claims a fresh file.
    /// KI-093: a `last_run` in the future — a hand-edited or restored
    /// store — used to silence a job forever. The clamp treats it as
    /// "ran just now", so the next window is served.
    /// KI-161: the manifest seeds a fresh home and never rewrites what
    /// the owner already has. A re-run is a no-op; a changed job stays
    /// changed.
    /// KI-092: the store's read-modify-write runs under the lock. The
    /// lock file is created beside the store, the guard releases on
    /// drop, and a mutation takes the lock itself — a caller must not
    /// hold it across a mutation, because the lock is per descriptor
    /// and the mutation would wait on itself.
    #[test]
    fn the_stores_mutations_run_under_the_lock() {
        let home = temp_home("store-lock");
        {
            let _guard = StoreLock::take(&home).expect("the first lock");
            assert!(lock_path(&home).exists(), "the lock file exists");
        }
        // Released on drop: a second take succeeds.
        let _second = StoreLock::take(&home).expect("the lock released");
        drop(_second);
        // A store mutation takes the lock itself and lands whole.
        let mut store = Store::open(&home).unwrap();
        store.add("a-job", "* * * * *", "do it", false).unwrap();
        let again = Store::open(&home).unwrap();
        assert_eq!(again.jobs().len(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_manifest_seeds_a_fresh_home_and_never_rewrites() {
        let home = temp_home("seed-manifest");
        let added = seed_missing(&home).expect("the seed lands");
        assert_eq!(added.len(), SEED_MANIFEST.len(), "{added:?}");
        let store = Store::open(&home).unwrap();
        assert_eq!(store.jobs().len(), SEED_MANIFEST.len());
        // Every seeded job parses and is enabled with auto-approve.
        for job in store.jobs() {
            assert!(job.enabled, "{}", job.name);
            assert!(job.auto_approve, "{}", job.name);
            assert!(job.last_run.is_none(), "{}", job.name);
        }
        // The owner's edit wins: a re-seed adds nothing and changes
        // nothing.
        let before = store.jobs().to_vec();
        let again = seed_missing(&home).expect("the re-seed lands");
        assert!(again.is_empty(), "{again:?}");
        let after = Store::open(&home).unwrap().jobs().to_vec();
        assert_eq!(before, after);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// KI-221/224: the manifest's task names the spawn route, not the
    /// lane tool the driver session does not carry.
    #[test]
    fn the_manifest_task_names_the_route_the_driver_has() {
        let task = seed_task("fleet");
        assert!(!task.contains("mcp__blend1__refresh"), "{task}");
        assert!(task.contains("mcp_server.py"), "{task}");
        assert!(task.contains("refresh"), "{task}");
        assert!(task.contains("fleet"), "{task}");
    }

    #[test]
    fn a_future_last_run_does_not_silence_the_job() {
        let home = temp_home("future-last");
        let mut store = Store::open(&home).unwrap();
        store.add("a-job", "* * * * *", "do it", false).unwrap();
        let now = minute(2026, 10, 14, 7, 0);
        store
            .record_run("a-job", minute(2026, 10, 15, 7, 0), Outcome::Completed)
            .unwrap();
        let jobs = store.jobs().to_vec();
        let mut scheduler = Scheduler::new(home.clone(), scripted_runner("nothing"));
        // The clamp: the future stamp reads as "ran just now", so the
        // catch-up's `missed > last` is true for the current window and
        // the job is served rather than skipped. The scheduler has to
        // have seen the job enabled for the window to be eligible.
        scheduler.note_jobs(&jobs, now);
        let due = scheduler.catch_up_due(&jobs, now);
        // The clamp reads the future stamp as "ran just now", so the
        // window at `now` is newer than the floor and the job serves.
        assert_eq!(due.len(), 1, "{due:?}");
        assert_eq!(due[0].minute, now, "{due:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// KI-094: a clock that steps backward is refused, not served: a
    /// tick whose minute is older than the last one seen would re-fire
    /// minutes the latch already holds.
    #[tokio::test]
    async fn a_backward_clock_is_refused_not_served() {
        let home = temp_home("clock-back");
        let mut scheduler = Scheduler::new(home.clone(), scripted_runner("nothing"));
        let later = minute(2026, 10, 14, 8, 0);
        let earlier = minute(2026, 10, 14, 7, 0);
        let _ = scheduler.tick(later).await;
        let lines = scheduler.tick(earlier).await;
        assert!(
            lines
                .iter()
                .any(|line| line.contains("clock moved backward")),
            "{lines:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// KI-105: the eval cadence's minute rides disk, so a restarted
    /// process does not re-run a corpus that already ran for it.
    #[test]
    fn the_eval_minute_survives_a_restart() {
        let home = temp_home("eval-minute");
        let when = minute(2026, 10, 14, 4, 0);
        write_eval_minute(&home, when);
        let scheduler = Scheduler::new(home.clone(), scripted_runner("nothing"));
        assert_eq!(scheduler.eval_last, Some(when));
        // A second scheduler over the same home reads the same floor.
        let second = Scheduler::new(home.clone(), scripted_runner("nothing"));
        assert_eq!(second.eval_last, Some(when));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_minutes_claim_is_atomic_and_a_second_scheduler_is_refused() {
        let home = temp_home("claim");
        let minute = Minute::parse("2026-10-18T09:00Z").unwrap();
        assert!(
            claim_minute(&home, "daily", minute),
            "the first claim owns the minute"
        );
        assert!(
            !claim_minute(&home, "daily", minute),
            "a second scheduler is refused by the create"
        );
        // A different minute, and a different job, claim fresh.
        let later = Minute::parse("2026-10-18T09:01Z").unwrap();
        assert!(
            claim_minute(&home, "daily", later),
            "the next minute is a fresh claim"
        );
        assert!(
            claim_minute(&home, "other", minute),
            "another job is a fresh claim"
        );
        // The sweep removes the past minute's claim and keeps the current.
        sweep_claims(&home, later);
        assert!(!claims_dir(&home)
            .join(format!("daily-{}", minute.stamp()))
            .exists());
        assert!(claims_dir(&home)
            .join(format!("daily-{}", later.stamp()))
            .exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_five_field_expression_parses_and_matches() {
        let schedule = Schedule::parse("0 7 * * *").unwrap();
        assert_eq!(schedule.expression(), "0 7 * * *");
        assert!(schedule.matches(minute(2026, 10, 14, 7, 0)));
        assert!(!schedule.matches(minute(2026, 10, 14, 7, 1)));
        assert!(!schedule.matches(minute(2026, 10, 14, 8, 0)));
    }

    #[test]
    fn lists_ranges_and_steps_expand() {
        let schedule = Schedule::parse("0,30 8-9 * * *").unwrap();
        assert_eq!(schedule.minutes, vec![0, 30]);
        assert_eq!(schedule.hours, vec![8, 9]);
        assert!(schedule.matches(minute(2026, 10, 14, 9, 30)));
        let stepped = Schedule::parse("*/15 * * * *").unwrap();
        assert_eq!(stepped.minutes, vec![0, 15, 30, 45]);
        let range_step = Schedule::parse("0-30/10 0 * * *").unwrap();
        assert_eq!(range_step.minutes, vec![0, 10, 20, 30]);
    }

    #[test]
    fn a_step_attaches_to_a_star_or_a_range_never_to_a_bare_value() {
        assert!(Schedule::parse("5/2 * * * *").is_err());
        let error = Schedule::parse("5/2 * * * *").unwrap_err();
        assert!(matches!(error, ScheduleError::Element { .. }), "{error}");
    }

    #[test]
    fn the_wrong_field_count_is_a_named_error() {
        let error = Schedule::parse("* * * *").unwrap_err();
        assert!(
            matches!(error, ScheduleError::FieldCount { found: 4 }),
            "{error}"
        );
        assert!(error.to_string().contains("five"), "{error}");
        let error = Schedule::parse("* * * * * *").unwrap_err();
        assert!(
            matches!(error, ScheduleError::FieldCount { found: 6 }),
            "{error}"
        );
    }

    #[test]
    fn out_of_range_values_and_a_zero_step_are_named_errors() {
        let error = Schedule::parse("60 * * * *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Range {
                    field: "minute",
                    value: 60,
                    ..
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("* 24 * * *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Range {
                    field: "hour",
                    value: 24,
                    ..
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("* * 0 * *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Range {
                    field: "day-of-month",
                    value: 0,
                    ..
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("* * * 13 *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Range {
                    field: "month",
                    value: 13,
                    ..
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("* * * * 8").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Range {
                    field: "day-of-week",
                    value: 8,
                    ..
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("*/0 * * * *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Step {
                    field: "minute",
                    ..
                }
            ),
            "{error}"
        );
        assert!(error.to_string().contains("positive integer"), "{error}");
    }

    #[test]
    fn an_inverted_range_and_a_junk_element_are_named_errors() {
        let error = Schedule::parse("30-10 * * * *").unwrap_err();
        assert!(
            matches!(
                error,
                ScheduleError::Inverted {
                    field: "minute",
                    start: 30,
                    end: 10
                }
            ),
            "{error}"
        );
        let error = Schedule::parse("a * * * *").unwrap_err();
        assert!(matches!(error, ScheduleError::Value { .. }), "{error}");
        let error = Schedule::parse("1,,2 * * * *").unwrap_err();
        assert!(matches!(error, ScheduleError::Element { .. }), "{error}");
    }

    #[test]
    fn seven_is_sunday_and_zero_is_sunday() {
        let seven = Schedule::parse("0 0 * * 7").unwrap();
        let zero = Schedule::parse("0 0 * * 0").unwrap();
        // 2026-10-18 is a Sunday.
        let sunday = minute(2026, 10, 18, 0, 0);
        assert!(seven.matches(sunday));
        assert!(zero.matches(sunday));
        assert!(!seven.matches(minute(2026, 10, 19, 0, 0)));
    }

    #[test]
    fn the_day_of_month_and_day_of_week_rule_is_vixies() {
        // Both restricted: either matching suffices.
        let either = Schedule::parse("0 0 13 * 5").unwrap();
        assert!(either.matches(minute(2026, 11, 13, 0, 0))); // a Friday the 13th
        assert!(either.matches(minute(2026, 10, 16, 0, 0))); // a Friday, not the 13th
        assert!(!either.matches(minute(2026, 10, 14, 0, 0))); // neither
                                                              // One star: both must match, which degrades to the other field.
        let fridays = Schedule::parse("0 0 * * 5").unwrap();
        assert!(fridays.matches(minute(2026, 10, 16, 0, 0)));
        assert!(!fridays.matches(minute(2026, 10, 14, 0, 0)));
        let thirteenths = Schedule::parse("0 0 13 * *").unwrap();
        assert!(thirteenths.matches(minute(2026, 11, 13, 0, 0)));
        assert!(!thirteenths.matches(minute(2026, 11, 14, 0, 0)));
    }

    // --- the search ---------------------------------------------------------

    #[test]
    fn next_after_is_the_first_matching_minute_strictly_later() {
        let hourly = Schedule::parse("0 * * * *").unwrap();
        assert_eq!(
            hourly.next_after(minute(2026, 10, 14, 12, 30)),
            Some(minute(2026, 10, 14, 13, 0))
        );
        // The current minute is not its own next fire.
        assert_eq!(
            hourly.next_after(minute(2026, 10, 14, 12, 0)),
            Some(minute(2026, 10, 14, 13, 0))
        );
        let daily = Schedule::parse("0 7 * * *").unwrap();
        assert_eq!(
            daily.next_after(minute(2026, 10, 14, 8, 0)),
            Some(minute(2026, 10, 15, 7, 0))
        );
        // A weekday rule crosses the weekend.
        let weekdays = Schedule::parse("0 9 * * 1-5").unwrap();
        assert_eq!(
            weekdays.next_after(minute(2026, 10, 16, 10, 0)), // a Friday
            Some(minute(2026, 10, 19, 9, 0))                  // the next Monday
        );
    }

    #[test]
    fn previous_at_or_before_is_the_most_recent_matching_minute() {
        let hourly = Schedule::parse("0 * * * *").unwrap();
        assert_eq!(
            hourly.previous_at_or_before(minute(2026, 10, 14, 12, 30)),
            Some(minute(2026, 10, 14, 12, 0))
        );
        assert_eq!(
            hourly.previous_at_or_before(minute(2026, 10, 14, 12, 0)),
            Some(minute(2026, 10, 14, 12, 0))
        );
        let daily = Schedule::parse("0 7 * * *").unwrap();
        assert_eq!(
            daily.previous_at_or_before(minute(2026, 10, 14, 6, 0)),
            Some(minute(2026, 10, 13, 7, 0))
        );
    }

    #[test]
    fn an_unsatisfiable_expression_never_fires() {
        // April has thirty days.
        let never = Schedule::parse("0 0 31 4 *").unwrap();
        assert_eq!(never.next_after(minute(2026, 10, 14, 0, 0)), None);
        assert_eq!(
            never.previous_at_or_before(minute(2026, 10, 14, 0, 0)),
            None
        );
    }

    // --- the minute ---------------------------------------------------------

    #[test]
    fn a_minute_round_trips_through_its_stamp() {
        let minute = minute(2026, 10, 14, 7, 5);
        assert_eq!(minute.stamp(), "2026-10-14T07:05Z");
        assert_eq!(Minute::parse("2026-10-14T07:05Z"), Some(minute));
        // 2026 is not a leap year.
        assert_eq!(Minute::parse("2026-02-29T00:00Z"), None);
        assert_eq!(Minute::parse("2026-13-01T00:00Z"), None);
        assert_eq!(Minute::parse("2026-10-14T24:00Z"), None);
        assert_eq!(Minute::parse("2026-10-14T07:60Z"), None);
        assert_eq!(Minute::parse("2026-10-14 07:05Z"), None);
        assert_eq!(Minute::parse("2026-10-14T07:05"), None);
        assert_eq!(Minute::parse(""), None);
    }

    #[test]
    fn the_weekday_of_known_days_is_right() {
        // 1970-01-01 was a Thursday.
        assert_eq!(minute(1970, 1, 1, 0, 0).day_of_week(), 4);
        assert_eq!(minute(2026, 10, 18, 0, 0).day_of_week(), 0); // a Sunday
        assert_eq!(minute(2026, 10, 19, 0, 0).day_of_week(), 1); // a Monday
    }

    #[test]
    fn previous_minute_carries_the_borrow_back_across_every_boundary() {
        assert_eq!(
            minute(2026, 10, 14, 12, 30).previous_minute(),
            minute(2026, 10, 14, 12, 29)
        );
        assert_eq!(
            minute(2026, 10, 14, 12, 0).previous_minute(),
            minute(2026, 10, 14, 11, 59)
        );
        assert_eq!(
            minute(2026, 10, 14, 0, 0).previous_minute(),
            minute(2026, 10, 13, 23, 59)
        );
        assert_eq!(
            minute(2026, 1, 1, 0, 0).previous_minute(),
            minute(2025, 12, 31, 23, 59)
        );
        // Across a leap day: 2028 is a leap year.
        assert_eq!(
            minute(2028, 3, 1, 0, 0).previous_minute(),
            minute(2028, 2, 29, 23, 59)
        );
        assert_eq!(
            minute(2026, 3, 1, 0, 0).previous_minute(),
            minute(2026, 2, 28, 23, 59)
        );
    }

    // --- the store ----------------------------------------------------------

    #[test]
    fn add_mutates_round_trip_and_the_file_holds_every_field() {
        let home = temp_home("store-round-trip");
        let mut store = Store::open(&home).unwrap();
        assert!(store.jobs().is_empty());
        let stored = store
            .add("daily-digest", "0 7 * * *", "  Summarize the day.  ", false)
            .unwrap();
        assert!(stored.enabled);
        assert_eq!(stored.task, "Summarize the day.");
        assert_eq!(stored.last_run, None);

        let reopened = Store::open(&home).unwrap();
        assert_eq!(reopened.jobs(), store.jobs());
        let text = fs::read_to_string(jobs_path(&home)).unwrap();
        assert!(text.contains("\"name\": \"daily-digest\""), "{text}");
        assert!(text.contains("\"schedule\": \"0 7 * * *\""), "{text}");
        assert!(text.contains("\"last_run\": null"), "{text}");
        assert!(text.contains("\"last_outcome\": null"), "{text}");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn every_mutation_round_trips_through_the_file() {
        let home = temp_home("store-mutations");
        let mut store = Store::open(&home).unwrap();
        store.add("a-job", "* * * * *", "one", true).unwrap();
        store.add("b-job", "0 * * * *", "two", false).unwrap();
        store.set_enabled("a-job", false).unwrap();
        store
            .record_run("a-job", minute(2026, 10, 14, 7, 0), Outcome::Completed)
            .unwrap();
        let removed = store.remove("b-job").unwrap();
        assert_eq!(removed.name, "b-job");

        let reopened = Store::open(&home).unwrap();
        assert_eq!(reopened.jobs().len(), 1);
        let job = reopened.job("a-job").unwrap();
        assert!(!job.enabled);
        assert!(job.auto_approve);
        assert_eq!(job.last_run.as_deref(), Some("2026-10-14T07:00Z"));
        assert_eq!(job.last_outcome, Some(Outcome::Completed));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_bad_name_a_taken_name_and_a_bad_schedule_are_refused() {
        let home = temp_home("store-refusals");
        let mut store = Store::open(&home).unwrap();
        let error = store
            .add("Daily Digest", "* * * * *", "x", false)
            .unwrap_err();
        assert!(matches!(error, CronError::Name(_)), "{error}");
        let error = store.add("../escape", "* * * * *", "x", false).unwrap_err();
        assert!(matches!(error, CronError::Name(_)), "{error}");
        let error = store.add(".hidden", "* * * * *", "x", false).unwrap_err();
        assert!(matches!(error, CronError::Name(_)), "{error}");
        let error = store.add("a-job", "nonsense", "x", false).unwrap_err();
        assert!(matches!(error, CronError::Schedule(_)), "{error}");
        let error = store.add("a-job", "* * * * *", "   ", false).unwrap_err();
        assert!(matches!(error, CronError::Task(_)), "{error}");
        store.add("a-job", "* * * * *", "x", false).unwrap();
        let error = store.add("a-job", "* * * * *", "y", false).unwrap_err();
        assert!(matches!(error, CronError::Exists(_)), "{error}");
        let error = store.set_enabled("nope", false).unwrap_err();
        assert!(matches!(error, CronError::Missing(_)), "{error}");
        let error = store.remove("nope").unwrap_err();
        assert!(matches!(error, CronError::Missing(_)), "{error}");
        let error = store
            .record_run("nope", minute(2026, 10, 14, 7, 0), Outcome::Failed)
            .unwrap_err();
        assert!(matches!(error, CronError::Missing(_)), "{error}");
        // Nothing was written by a refused mutation.
        assert_eq!(Store::open(&home).unwrap().jobs().len(), 1);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_malformed_store_is_a_typed_error_not_a_silent_empty_read() {
        let home = temp_home("store-malformed");
        fs::create_dir_all(jobs_path(&home).parent().unwrap()).unwrap();
        // Not JSON at all.
        fs::write(jobs_path(&home), "not a store").unwrap();
        let error = Store::open(&home).unwrap_err();
        assert!(matches!(error, CronError::Malformed(_)), "{error}");
        // A JSON document that is not a job store.
        fs::write(jobs_path(&home), "[1, 2, 3]").unwrap();
        let error = Store::open(&home).unwrap_err();
        assert!(matches!(error, CronError::Malformed(_)), "{error}");
        // A record with an unparseable schedule.
        fs::write(jobs_path(&home), r#"{"jobs":[{"name":"x","schedule":"bogus","task":"t","auto_approve":false,"enabled":true,"last_run":null,"last_outcome":null}]}"#).unwrap();
        let error = Store::open(&home).unwrap_err();
        let message = error.to_string();
        assert!(matches!(error, CronError::Malformed(_)), "{message}");
        assert!(message.contains("job `x`"), "{message}");
        // A record with a last_run that is not a minute.
        fs::write(jobs_path(&home), r#"{"jobs":[{"name":"x","schedule":"* * * * *","task":"t","auto_approve":false,"enabled":true,"last_run":"yesterday","last_outcome":null}]}"#).unwrap();
        let error = Store::open(&home).unwrap_err();
        assert!(matches!(error, CronError::Malformed(_)), "{error}");
        // A duplicate name.
        fs::write(jobs_path(&home), r#"{"jobs":[{"name":"x","schedule":"* * * * *","task":"t","auto_approve":false,"enabled":true,"last_run":null,"last_outcome":null},{"name":"x","schedule":"* * * * *","task":"t","auto_approve":false,"enabled":true,"last_run":null,"last_outcome":null}]}"#).unwrap();
        let error = Store::open(&home).unwrap_err();
        assert!(matches!(error, CronError::Malformed(_)), "{error}");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_store_with_no_jobs_is_empty_not_an_error() {
        let home = temp_home("store-empty");
        fs::create_dir_all(jobs_path(&home).parent().unwrap()).unwrap();
        fs::write(jobs_path(&home), "{\"jobs\": []}\n").unwrap();
        assert!(Store::open(&home).unwrap().jobs().is_empty());
        fs::remove_dir_all(&home).ok();
    }

    // --- the scheduler ------------------------------------------------------

    fn test_scheduler(home: PathBuf) -> Scheduler {
        // A runner that records nothing and runs nothing: the decision
        // half of a pass is what these tests drive.
        Scheduler::new(
            home,
            Box::new(|_job, _minute, _catch_up| {
                Box::pin(async { Err("the test runner ran".to_string()) })
            }),
        )
    }

    #[test]
    fn a_job_fires_when_its_minute_matches_and_the_latch_holds_the_minute() {
        let home = temp_home("tick-fires");
        let jobs = vec![job("every-minute", "* * * * *", None)];
        let now = minute(2026, 10, 14, 12, 0);

        let mut scheduler = test_scheduler(home.clone());
        let due = scheduler.tick_due(&jobs, now);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].job.name, "every-minute");
        assert_eq!(due[0].minute, now);
        assert!(!due[0].catch_up);
        // The latch: the same minute does not fire twice.
        assert!(scheduler.tick_due(&jobs, now).is_empty());
        // The next minute does.
        let next = minute(2026, 10, 14, 12, 1);
        assert_eq!(scheduler.tick_due(&jobs, next).len(), 1);

        // A job whose minute does not match never fires.
        let hourly = vec![job("hourly", "0 * * * *", None)];
        let mut scheduler = test_scheduler(home.clone());
        assert!(scheduler
            .tick_due(&hourly, minute(2026, 10, 14, 12, 30))
            .is_empty());
        assert_eq!(
            scheduler
                .tick_due(&hourly, minute(2026, 10, 14, 13, 0))
                .len(),
            1
        );
        // A disabled job never fires.
        let mut disabled = job("off", "* * * * *", None);
        disabled.enabled = false;
        assert!(scheduler.tick_due(&[disabled], now).is_empty());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_missed_window_catches_up_exactly_once_at_startup() {
        let home = temp_home("catch-up-missed");
        // The job last ran at 11:00 and its schedule is hourly at :00;
        // the process came up at 12:30, so the 12:00 window was missed.
        let jobs = vec![job("hourly", "0 * * * *", Some("2026-10-14T11:00Z"))];
        let mut scheduler = test_scheduler(home.clone());
        let due = scheduler.catch_up_due(&jobs, minute(2026, 10, 14, 12, 30));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].minute, minute(2026, 10, 14, 12, 0));
        assert!(due[0].catch_up, "the catch-up is named as one");
        // The same startup pass does not decide it twice.
        assert!(scheduler
            .catch_up_due(&jobs, minute(2026, 10, 14, 12, 30))
            .is_empty());
        // And the tick in the missed minute does not fire it again.
        assert!(scheduler
            .tick_due(&jobs, minute(2026, 10, 14, 12, 0))
            .is_empty());

        // A window already served is not a missed window.
        let served = vec![job("hourly", "0 * * * *", Some("2026-10-14T12:00Z"))];
        let mut scheduler = test_scheduler(home.clone());
        assert!(scheduler
            .catch_up_due(&served, minute(2026, 10, 14, 12, 30))
            .is_empty());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_job_that_has_never_run_does_not_catch_up() {
        let home = temp_home("catch-up-never-run");
        // No last_run at all: the job's first fire is its first matching
        // minute going forward, whatever the clock says at startup.
        let jobs = vec![job("every-minute", "* * * * *", None)];
        let mut scheduler = test_scheduler(home.clone());
        assert!(scheduler
            .catch_up_due(&jobs, minute(2026, 10, 14, 12, 30))
            .is_empty());
        // The tick is the path that starts it.
        assert_eq!(
            scheduler
                .tick_due(&jobs, minute(2026, 10, 14, 12, 30))
                .len(),
            1
        );
        fs::remove_dir_all(&home).ok();
    }

    // --- the fires ---------------------------------------------------------

    /// Write one job into the store the way `fleet cron add` leaves it,
    /// so a whole pass can be driven through the real store.
    fn seed_job(home: &Path, name: &str, schedule: &str) {
        Store::open(home)
            .unwrap()
            .add(name, schedule, "the goal", true)
            .unwrap();
    }

    /// One finished fire's outcome, the shape a runner reports when the
    /// run itself went through.
    fn completed(job: &Job) -> RunOutcome {
        RunOutcome {
            outcome: Outcome::Completed,
            session_id: format!("20261014T120000Z-{}", job.name),
            log_path: PathBuf::from("/tmp/scheduled.jsonl"),
            skipped_tool: None,
            detail: "completed in 2 turns".to_string(),
        }
    }

    /// A runner that panics for the job named `panics` and completes
    /// every other one: the seam a panicking fire is driven through.
    fn scripted_runner(panics: &'static str) -> Runner {
        Box::new(move |job, _minute, _catch_up| {
            Box::pin(async move {
                if job.name == panics {
                    panic!("the runner exploded");
                }
                Ok(completed(&job))
            })
        })
    }

    #[test]
    fn the_eval_cadence_is_daily_and_fires_once_per_matching_minute() {
        // Pure: due at its daily minute, not at any other, and not twice
        // for the same minute the latch already holds.
        let due = minute(2026, 10, 14, 4, 0);
        assert!(eval_due(None, due));
        assert!(!eval_due(Some(due), due), "the latch holds the minute");
        assert!(
            !eval_due(None, minute(2026, 10, 14, 5, 0)),
            "only 04:00 matches"
        );
        assert!(
            eval_due(Some(due), minute(2026, 10, 15, 4, 0)),
            "a new day is a new fire"
        );
    }

    #[tokio::test]
    async fn the_scheduler_runs_the_eval_corpus_on_its_daily_cadence() {
        let home = temp_home("eval-cadence");
        let fired = std::sync::Arc::new(AtomicUsize::new(0));
        let seen = std::sync::Arc::clone(&fired);
        let eval: EvalRunner = Box::new(move || {
            let seen = std::sync::Arc::clone(&seen);
            Box::pin(async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Ok("ran \u{2014} 6 passed, 0 failed, score 1.00".to_string())
            })
        });
        let mut scheduler =
            Scheduler::new(home.clone(), scripted_runner("nothing")).with_eval(eval);
        let due = minute(2026, 10, 14, 4, 0);
        // The matching minute fires it once, and the line names the run.
        let lines = scheduler.run_eval_if_due(due).await;
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("eval corpus") && line.contains("score 1.00")),
            "{lines:?}"
        );
        // The same minute does not fire again; a non-matching minute does not.
        assert!(scheduler.run_eval_if_due(due).await.is_empty());
        assert!(scheduler
            .run_eval_if_due(minute(2026, 10, 14, 9, 0))
            .await
            .is_empty());
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        fs::remove_dir_all(&home).ok();
    }

    /// A panic in the eval cadence is the KI-027 spawn/port pattern
    /// promoted into the scheduler's own thread: unguarded, it
    /// unwinds through the tick loop's `block_on` and kills the
    /// thread for the life of the process — silently, one stderr line
    /// over the TUI's alternate screen — and every scheduled job with
    /// it. The cadence now rides the same CatchUnwind guard a job
    /// fire does: one named line, and the loop carries on.
    #[tokio::test]
    async fn a_panicking_eval_cadence_is_named_and_the_loop_carries_on() {
        let home = temp_home("eval-cadence-panics");
        let eval: EvalRunner =
            Box::new(|| Box::pin(async move { panic!("the eval runner exploded") }));
        let mut scheduler =
            Scheduler::new(home.clone(), scripted_runner("nothing")).with_eval(eval);
        let due = minute(2026, 10, 14, 4, 0);

        let lines = scheduler.run_eval_if_due(due).await;
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("eval corpus"), "{}", lines[0]);
        assert!(lines[0].contains("panicked"), "{}", lines[0]);
        assert!(
            lines[0].contains("the eval runner exploded"),
            "{}",
            lines[0]
        );
        assert!(
            lines[0].contains("the minute stays claimed"),
            "{}",
            lines[0]
        );
        // The minute is spent either way: no retry inside it.
        assert!(scheduler.run_eval_if_due(due).await.is_empty());
        // The scheduler is still alive: a later tick decides and runs
        // normally — the panic was one cadence's failure, never the
        // tick loop's death.
        seed_job(&home, "steady", "* * * * *");
        let lines = scheduler.tick(minute(2026, 10, 14, 12, 0)).await;
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("job `steady`"), "{}", lines[0]);
        assert!(lines[0].contains("completed"), "{}", lines[0]);
        fs::remove_dir_all(&home).ok();
    }

    #[tokio::test]
    async fn a_scheduler_without_a_wired_cadence_runs_no_corpus() {
        // The default: no eval cadence wired, so `run_eval_if_due` is a
        // no-op even on the matching minute — the cron-job tests are
        // unchanged by the cadence's presence.
        let home = temp_home("eval-cadence-none");
        let mut scheduler = Scheduler::new(home.clone(), scripted_runner("nothing"));
        assert!(scheduler
            .run_eval_if_due(minute(2026, 10, 14, 4, 0))
            .await
            .is_empty());
        fs::remove_dir_all(&home).ok();
    }

    /// A panic inside a fire used to unwind through the scheduler
    /// thread's `block_on` and kill the thread for the life of the
    /// process, with nothing on any surface but the default hook's
    /// stderr line. It is now one named line — the job, the panic — and
    /// the loop carries on.
    #[tokio::test]
    async fn a_panicking_fire_is_named_and_the_minute_stays_claimed() {
        let home = temp_home("fire-panics");
        seed_job(&home, "boom", "* * * * *");
        let mut scheduler = Scheduler::new(home.clone(), scripted_runner("boom"));
        let now = minute(2026, 10, 14, 12, 0);

        let lines = scheduler.tick(now).await;
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("job `boom`"), "{}", lines[0]);
        assert!(lines[0].contains("panicked"), "{}", lines[0]);
        assert!(lines[0].contains("the runner exploded"), "{}", lines[0]);
        // The latch decision: the minute stays claimed. A retry inside
        // the same minute could spend a window whose run may already
        // have spent, and the next minute is one second away.
        assert!(scheduler.tick(now).await.is_empty());
        // The store recorded the spent minute, as `failed`: its
        // vocabulary has no fourth outcome, and the minute is spent
        // either way.
        let stored = Store::open(&home).unwrap().job("boom").unwrap().clone();
        assert_eq!(stored.last_run.as_deref(), Some("2026-10-14T12:00Z"));
        assert_eq!(stored.last_outcome, Some(Outcome::Failed));
        fs::remove_dir_all(&home).ok();
    }

    /// The fire beside a panicking one still runs, and the pass still
    /// names it: a panic is one fire's failure, never the pass's.
    #[tokio::test]
    async fn a_panicking_fire_does_not_stop_the_fires_beside_it() {
        let home = temp_home("fire-panics-beside");
        seed_job(&home, "boom", "* * * * *");
        seed_job(&home, "steady", "* * * * *");
        let mut scheduler = Scheduler::new(home.clone(), scripted_runner("boom"));
        let now = minute(2026, 10, 14, 12, 0);

        let lines = scheduler.tick(now).await;
        assert_eq!(lines.len(), 2, "{lines:?}");
        let panicked = lines
            .iter()
            .find(|line| line.contains("job `boom`"))
            .expect("the panicking fire's line");
        assert!(panicked.contains("the runner exploded"), "{panicked}");
        let steady = lines
            .iter()
            .find(|line| line.contains("job `steady`"))
            .expect("the steady fire's line");
        assert!(steady.contains("completed"), "{steady}");
        // Both minutes are spent: neither fires again this minute.
        assert!(scheduler.tick(now).await.is_empty());
        fs::remove_dir_all(&home).ok();
    }

    /// A fire inside the TUI rides the interface's event channel and
    /// produces no stderr line: the card carries the job, the minute,
    /// the outcome, and the session (KI-090).
    #[tokio::test]
    async fn a_fire_inside_the_tui_rides_the_event_channel_instead_of_stderr() {
        let home = temp_home("fire-ui");
        seed_job(&home, "every-minute", "* * * * *");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<UiEvent>();
        let mut scheduler =
            Scheduler::new(home.clone(), scripted_runner("nothing")).with_ui(Some(tx));

        let lines = scheduler.tick(minute(2026, 10, 14, 12, 0)).await;
        assert!(
            lines.is_empty(),
            "the card rides the channel, not stderr: {lines:?}"
        );
        let event = rx.try_recv().expect("the fire's card");
        match event {
            UiEvent::CronFire {
                job,
                minute,
                catch_up,
                outcome,
                detail,
                session,
            } => {
                assert_eq!(job, "every-minute");
                assert_eq!(minute, "2026-10-14T12:00Z");
                assert!(!catch_up);
                assert_eq!(outcome, "completed");
                assert_eq!(detail, "completed in 2 turns");
                assert_eq!(session.as_deref(), Some("20261014T120000Z-every-minute"));
            }
            other => panic!("expected a cron fire event, got {other:?}"),
        }
        fs::remove_dir_all(&home).ok();
    }

    /// Two fires due in one minute run at the same time, not one after
    /// the other. The proof is a two-party barrier only two concurrent
    /// fires can clear: a pass that ran them one at a time would wait
    /// forever on the first.
    #[tokio::test]
    async fn two_due_fires_run_concurrently_not_one_after_the_other() {
        let home = temp_home("fire-concurrent");
        seed_job(&home, "first", "* * * * *");
        seed_job(&home, "second", "* * * * *");
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let runner_barrier = barrier.clone();
        let mut scheduler = Scheduler::new(
            home.clone(),
            Box::new(move |job, _minute, _catch_up| {
                let barrier = runner_barrier.clone();
                Box::pin(async move {
                    barrier.wait().await;
                    Ok(completed(&job))
                })
            }),
        );

        let lines = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            scheduler.tick(minute(2026, 10, 14, 12, 0)),
        )
        .await
        .expect("two concurrent fires cleared the barrier; a sequential pass would hang here");
        assert_eq!(lines.len(), 2, "{lines:?}");
        fs::remove_dir_all(&home).ok();
    }

    // --- the passed windows -------------------------------------------------

    /// A window that came due while the pass was in flight is named,
    /// one line per window, and never silently dropped.
    #[tokio::test]
    async fn a_window_that_passes_while_the_scheduler_is_busy_is_named_not_dropped() {
        let home = temp_home("passed-window");
        seed_job(&home, "every-minute", "* * * * *");
        let mut scheduler = test_scheduler(home.clone());
        // The 12:00 tick decides the fire, and its pass runs long: the
        // next tick lands at 12:03.
        assert_eq!(scheduler.tick(minute(2026, 10, 14, 12, 0)).await.len(), 1);

        let lines = scheduler.tick(minute(2026, 10, 14, 12, 3)).await;
        let passed: Vec<&String> = lines
            .iter()
            .filter(|line| line.contains("window passed"))
            .collect();
        assert_eq!(passed.len(), 2, "{lines:?}");
        assert!(passed[0].contains("2026-10-14T12:01Z"), "{}", passed[0]);
        assert!(passed[1].contains("2026-10-14T12:02Z"), "{}", passed[1]);
        // The naming does not repeat: every passed minute is claimed in
        // the latch as it is named.
        let again = scheduler.tick(minute(2026, 10, 14, 12, 4)).await;
        assert!(
            !again.iter().any(|line| line.contains("window passed")),
            "{again:?}"
        );
        fs::remove_dir_all(&home).ok();
    }

    /// A long backlog is bounded on the surface: the first windows are
    /// named, the rest are summed into one trailing count — and every
    /// window's minute is spent whether it was named or not.
    #[tokio::test]
    async fn a_long_backlog_of_passed_windows_is_bounded_on_the_surface() {
        let home = temp_home("passed-window-cap");
        seed_job(&home, "every-minute", "* * * * *");
        let mut scheduler = test_scheduler(home.clone());
        assert_eq!(scheduler.tick(minute(2026, 10, 14, 12, 0)).await.len(), 1);

        let lines = scheduler.tick(minute(2026, 10, 14, 12, 10)).await;
        let named: Vec<&String> = lines
            .iter()
            .filter(|line| line.contains("'s 2026-10-14T"))
            .collect();
        assert_eq!(named.len(), MAX_NAMED_MISSED_WINDOWS, "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|line| line.contains("and 1 more window passed")),
            "{lines:?}"
        );
        // The nine minutes between are spent, named or not.
        let again = scheduler.tick(minute(2026, 10, 14, 12, 11)).await;
        assert!(
            !again.iter().any(|line| line.contains("window passed")),
            "{again:?}"
        );
        fs::remove_dir_all(&home).ok();
    }

    /// A window that came due before the scheduler ever saw the job
    /// enabled — a job that did not exist yet, or one that was
    /// disabled — is nobody's drop and is not named.
    #[tokio::test]
    async fn a_window_from_before_the_job_was_seen_is_not_named_as_a_drop() {
        let home = temp_home("passed-window-floor");
        seed_job(&home, "every-minute", "* * * * *");
        let mut scheduler = test_scheduler(home.clone());
        // The first tick that sees the job is 12:01, so the 12:00
        // window is not a window the scheduler dropped.
        let lines = scheduler.tick(minute(2026, 10, 14, 12, 1)).await;
        assert!(
            !lines.iter().any(|line| line.contains("window passed")),
            "{lines:?}"
        );
        assert_eq!(lines.len(), 1, "{lines:?}");

        // A job that was disabled when its windows passed starts its
        // floor at the minute it comes back.
        let home = temp_home("passed-window-disabled");
        seed_job(&home, "off", "* * * * *");
        Store::open(&home)
            .unwrap()
            .set_enabled("off", false)
            .unwrap();
        let mut scheduler = test_scheduler(home.clone());
        assert!(scheduler.tick(minute(2026, 10, 14, 12, 3)).await.is_empty());
        Store::open(&home)
            .unwrap()
            .set_enabled("off", true)
            .unwrap();
        let lines = scheduler.tick(minute(2026, 10, 14, 12, 3)).await;
        assert!(
            !lines.iter().any(|line| line.contains("window passed")),
            "{lines:?}"
        );
        assert_eq!(lines.len(), 1, "{lines:?}");
        fs::remove_dir_all(&home).ok();
    }

    // --- the store complaint -------------------------------------------------

    /// The tick runs once a second for the life of the process, so an
    /// unreadable store is named once and then silenced until the
    /// reason changes — never one line per second over whatever
    /// surface is watching.
    #[tokio::test]
    async fn an_unreadable_store_is_named_once_not_once_a_second() {
        let home = temp_home("store-complaint");
        fs::create_dir_all(jobs_path(&home).parent().unwrap()).unwrap();
        fs::write(jobs_path(&home), "not a store").unwrap();
        let mut scheduler = test_scheduler(home.clone());

        let first = scheduler.tick(minute(2026, 10, 14, 12, 0)).await;
        assert_eq!(first.len(), 1, "{first:?}");
        assert!(
            first[0].contains("the job store is unreadable"),
            "{}",
            first[0]
        );
        // The same complaint again is silence.
        assert!(scheduler.tick(minute(2026, 10, 14, 12, 1)).await.is_empty());
        assert!(scheduler.tick(minute(2026, 10, 14, 12, 2)).await.is_empty());
        // A different reason is a different complaint, and it is named.
        fs::write(jobs_path(&home), "[1, 2, 3]").unwrap();
        let changed = scheduler.tick(minute(2026, 10, 14, 12, 3)).await;
        assert_eq!(changed.len(), 1, "{changed:?}");
        assert!(changed[0].contains("unreadable"), "{}", changed[0]);
        // And silence again.
        assert!(scheduler.tick(minute(2026, 10, 14, 12, 4)).await.is_empty());
        fs::remove_dir_all(&home).ok();
    }

    // --- the framing --------------------------------------------------------

    /// The unattended gate's denial names the job's policy, not a user
    /// who is guaranteed absent: this gate denies precisely because
    /// nobody is there, so the model-visible text has to say so
    /// (KI-088). A declared-unattended job approves everything and
    /// supplies no denial of its own.
    #[test]
    fn the_unattended_gate_names_the_jobs_policy_not_an_absent_user() {
        let gate = Gate::for_job(&job("careful", "* * * * *", None));
        assert!(gate.unattended);
        let denial = gate
            .denial_reason()
            .expect("the unattended gate names its own reason");
        assert!(denial.contains("not declared unattended"), "{denial}");
        assert!(denial.contains("auto_approve"), "{denial}");
        assert!(!denial.contains("the user did not approve"), "{denial}");

        let mut trusted = job("trusted", "* * * * *", None);
        trusted.auto_approve = true;
        let gate = Gate::for_job(&trusted);
        assert!(!gate.unattended);
        assert_eq!(gate.denial_reason(), None);
    }

    #[test]
    fn the_catch_up_framing_names_the_missed_window() {
        let stored = job("hourly", "0 * * * *", None);
        let on_time = frame_task(&stored, minute(2026, 10, 14, 12, 0), false);
        assert!(
            on_time.starts_with("[scheduled run \u{2014} job `hourly` at 2026-10-14T12:00Z]"),
            "{on_time}"
        );
        assert!(on_time.ends_with("the goal"), "{on_time}");
        let catch_up = frame_task(&stored, minute(2026, 10, 14, 12, 0), true);
        assert!(catch_up.contains("catch-up"), "{catch_up}");
        assert!(catch_up.contains("2026-10-14T12:00Z"), "{catch_up}");
        assert!(
            catch_up.contains("missed while fleet was down"),
            "{catch_up}"
        );
        assert!(catch_up.ends_with("the goal"), "{catch_up}");
    }

    // --- the surface --------------------------------------------------------

    /// One scheduled run's log under `home`: the `start` record marked
    /// `kind: scheduled` and the framed task naming its job, written
    /// the way a real fire leaves them.
    fn seed_scheduled_session(home: &Path, id: &str, job: &str) {
        let sessions = crate::home::sessions_dir(home);
        fs::create_dir_all(&sessions).unwrap();
        let framed = frame_task(&job_fixture(job), minute(2026, 10, 14, 12, 0), false);
        let record = serde_json::json!({
            "type": "start",
            "v": 1,
            "model": "deepseek-flash",
            "created": "2026-10-14T12:00:00Z",
            "kind": session::SCHEDULED_KIND,
        });
        let user = serde_json::json!({"type": "user", "content": framed});
        fs::write(
            sessions.join(format!("{id}.jsonl")),
            format!("{record}\n{user}\n"),
        )
        .unwrap();
    }

    fn job_fixture(name: &str) -> Job {
        job(name, "* * * * *", None)
    }

    #[test]
    fn the_listing_cards_every_job_and_names_the_empty_home() {
        let home = temp_home("listing-cards");
        let now = minute(2026, 10, 14, 12, 30);
        let empty = render(&home, &[], now);
        assert!(empty.contains("no jobs yet"), "{empty}");
        assert!(empty.contains("fleet cron add"), "{empty}");

        let mut ran = job("daily-digest", "0 7 * * *", Some("2026-10-14T07:00Z"));
        ran.last_outcome = Some(Outcome::Completed);
        let mut fresh = job("hourly", "0 * * * *", None);
        fresh.enabled = false;
        let listing = render(&home, &[ran, fresh], now);
        assert!(listing.contains("2 jobs"), "{listing}");
        assert!(listing.contains("daily-digest"), "{listing}");
        assert!(listing.contains("0 7 * * *"), "{listing}");
        assert!(listing.contains("next fire 2026-10-15T07:00Z"), "{listing}");
        assert!(listing.contains("last run 2026-10-14T07:00Z"), "{listing}");
        assert!(listing.contains("completed"), "{listing}");
        assert!(listing.contains("never run"), "{listing}");
        assert!(listing.contains("auto-approve off"), "{listing}");
        assert!(listing.contains("disabled"), "{listing}");
        // The disabled job's card does not promise a fire it will never
        // keep.
        assert!(listing.contains("next fire \u{2014} disabled"), "{listing}");
        assert!(listing.contains("1 enabled"), "{listing}");
        assert!(listing.contains("1 disabled"), "{listing}");
        // No scheduled log yet: no session line on either card.
        assert!(!listing.contains("session "), "{listing}");
        // Cards, never a plain-text table.
        assert!(listing.contains('\u{250c}'), "{listing}");
        assert!(listing.contains('\u{2518}'), "{listing}");
        fs::remove_dir_all(&home).ok();
    }

    /// The retained output is reachable from the surface that lists the
    /// job: the card carries the session the last fire wrote, read from
    /// the sessions directory rather than from the store.
    #[test]
    fn the_listing_carries_the_session_the_last_fire_wrote() {
        let home = temp_home("listing-session");
        // An older scheduled run for the job, then a newer one: the
        // newest is the one the card names.
        seed_scheduled_session(&home, "20261014T110000Z-aaa111", "daily-digest");
        seed_scheduled_session(&home, "20261014T120000Z-bbb222", "daily-digest");
        // Another job's run, which the digest's card must not claim.
        seed_scheduled_session(&home, "20261014T120100Z-ccc333", "hourly");
        // An ordinary session, which no card claims.
        let sessions = crate::home::sessions_dir(&home);
        fs::write(
            sessions.join("20261014T120200Z-ddd444.jsonl"),
            "{\"type\":\"start\",\"v\":1,\"model\":\"deepseek-flash\",\"created\":\"c\"}\n\
             {\"type\":\"user\",\"content\":\"a task the owner typed\"}\n",
        )
        .unwrap();

        let mut ran = job("daily-digest", "0 7 * * *", Some("2026-10-14T07:00Z"));
        ran.last_outcome = Some(Outcome::Completed);
        let other = job("hourly", "0 * * * *", None);
        let listing = render(&home, &[ran, other], minute(2026, 10, 14, 12, 30));
        assert!(
            listing.contains("session 20261014T120000Z-bbb222"),
            "{listing}"
        );
        // The other job's own newest run rides its own card.
        assert!(
            listing.contains("session 20261014T120100Z-ccc333"),
            "{listing}"
        );
        // And the ordinary session is nobody's.
        assert!(!listing.contains("ddd444"), "{listing}");
        fs::remove_dir_all(&home).ok();
    }

    /// The join is the frame itself: a log that is not a scheduled run
    /// of the job is not claimed, and a home with no sessions directory
    /// reads as no session rather than an error.
    #[test]
    fn the_session_join_names_only_scheduled_runs_of_the_job() {
        let home = temp_home("session-join");
        assert_eq!(last_session_id(&home, "daily-digest"), None);
        seed_scheduled_session(&home, "20261014T120000Z-bbb222", "daily-digest");
        assert_eq!(
            last_session_id(&home, "daily-digest").as_deref(),
            Some("20261014T120000Z-bbb222")
        );
        // A different job's log, and a log whose start record is not
        // marked scheduled, both read as not this job's.
        assert_eq!(last_session_id(&home, "hourly"), None);
        let sessions = crate::home::sessions_dir(&home);
        let framed = frame_task(
            &job_fixture("daily-digest"),
            minute(2026, 10, 14, 13, 0),
            false,
        );
        let start = serde_json::json!({
            "type": "start",
            "v": 1,
            "model": "deepseek-flash",
            "created": "c",
        });
        let user = serde_json::json!({"type": "user", "content": framed});
        fs::write(
            sessions.join("20261014T130000Z-eee555.jsonl"),
            format!("{start}\n{user}\n"),
        )
        .unwrap();
        // The newer unmarked log does not win over the marked one.
        assert_eq!(
            last_session_id(&home, "daily-digest").as_deref(),
            Some("20261014T120000Z-bbb222")
        );
        fs::remove_dir_all(&home).ok();
    }
}
