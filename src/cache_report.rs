//! The cache instrument: what each model's prompt cache actually saved.
//!
//! The owner's ask (2026-11-15): cache performance per model used by the
//! router. The accounting is already on the record — `Usage` normalizes
//! DeepSeek's `prompt_cache_hit_tokens`, OpenAI's nested `cached_tokens`
//! and Anthropic's `cache_read_input_tokens` into one field, and every
//! route row carries it beside `provider`, `model` and `latency_ms`. This
//! module is the arithmetic over those rows and nothing else: no capture,
//! no estimate, no price.
//!
//! Three honesties shape the document:
//!
//! - **Cache-read tokens are a subset of prompt tokens** (the definition
//!   at `api::Usage::cache_read_tokens`), so the share is
//!   `cache_read ÷ prompt` — never a subtraction from a total that never
//!   included them.
//! - **The denominator is the rows that report**, not every row of the
//!   model: a provider that leaves the field out would otherwise dilute a
//!   rate it simply did not mention. `calls`, `accounted` and `reported`
//!   are carried separately so a surface can say which it is showing.
//! - **A missing rate is `null`, never zero** — a model that never
//!   reports cache reads is not a model with a 0% hit rate.

use crate::calib::{ReadRow, OP_ROUTE};
use serde_json::{json, Value};
use std::path::Path;

/// How many day-files the document reads: two weeks is long enough for a
/// collapse and a recovery to both show, short enough that a model the
/// router stopped choosing leaves the panel.
pub const WINDOW_DAYS: usize = 14;

/// The fewest rows a latency side may carry before it is reported — a
/// percentile of three calls is a number, not a measurement.
pub const LATENCY_FLOOR: usize = 4;

/// A caller's resolution of one session to its project: `None` when the log is
/// not in the archive, `Some` with a workspace (or none, for a session born
/// unbound). The bridge supplies [`crate::session::workspace_of`]; tests supply
/// a map.
pub type WorkspaceLookup<'a> = dyn Fn(&str) -> Option<crate::session::WorkspaceFact> + 'a;

/// One model's account across the window.
#[derive(Debug, Default, Clone)]
struct Ledger {
    provider: String,
    model: String,
    /// Route rows for this model, usage or not.
    calls: u64,
    /// Rows carrying a usage block — the token arithmetic's population.
    accounted: u64,
    /// Rows whose provider reported the cache half at all.
    reported: u64,
    /// Prompt tokens over the accounted rows.
    prompt: u64,
    /// Prompt tokens over the reporting rows — the share's denominator.
    reporting_prompt: u64,
    /// Cache-read tokens (a subset of `reporting_prompt`).
    cache_read: u64,
    completion: u64,
    /// Tokens the provider wrote into its cache, over the rows that
    /// reported the field at all — absence is never zero.
    cache_creation: u64,
    creation_reported: u64,
    /// Reasoning tokens, over the rows that reported them.
    reasoning: u64,
    reasoning_reported: u64,
    /// The provider's own `total_tokens`, over the rows that reported a
    /// non-zero one. Carried so a provider whose total disagrees with
    /// prompt + completion is named, never silently reconciled.
    provider_total: u64,
    total_reported: u64,
    /// Reporting rows that actually read from cache.
    shared_calls: u64,
    /// (cached, fresh) route latencies in ms, for the split.
    cached_ms: Vec<u64>,
    fresh_ms: Vec<u64>,
    /// Per-day (day, prompt, cache_read, calls); the scan is chronological,
    /// so the pushes are too.
    days: Vec<(String, u64, u64, u64)>,
    /// The reporting rows' own (prompt, cache_read), for the halves.
    reported_rows: Vec<(u64, u64)>,
    last_at: String,
}

impl Ledger {
    fn share(&self) -> Option<f64> {
        (self.reporting_prompt > 0).then(|| self.cache_read as f64 / self.reporting_prompt as f64)
    }

    fn note_day(&mut self, day: &str, prompt: u64, cache: Option<u64>) {
        match self.days.iter_mut().find(|one| one.0 == day) {
            Some(entry) => {
                entry.1 += prompt;
                entry.2 += cache.unwrap_or(0);
                entry.3 += 1;
            }
            None => self
                .days
                .push((day.to_string(), prompt, cache.unwrap_or(0), 1)),
        }
    }
}

/// The percentile of `values` at `share` (0..1), nearest-rank on the
/// sorted copy. `None` on an empty set.
fn percentile(values: &[u64], share: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((share * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

/// One latency side: its percentiles and how many rows stood behind them.
fn latency_side(values: &[u64]) -> Value {
    if values.len() < LATENCY_FLOOR {
        return json!({"rows": values.len(), "p50": Value::Null, "p90": Value::Null});
    }
    json!({
        "rows": values.len(),
        "p50": percentile(values, 0.5),
        "p90": percentile(values, 0.9),
    })
}

/// The token-weighted share over a slice of a model's reporting rows. A
/// mean of per-call rates would weight a small call like a large one; the
/// sums are the honest reading.
fn weighted(rows: &[(u64, u64)]) -> Option<f64> {
    let prompt: u64 = rows.iter().map(|(prompt, _)| prompt).sum();
    if prompt == 0 {
        return None;
    }
    let cache: u64 = rows.iter().map(|(_, cache)| cache).sum();
    Some(cache as f64 / prompt as f64)
}

/// The running account: one ledger per (provider, model) as the rows
/// arrive. Split from the render so the window can be **streamed** — the
/// cache view wants every route row of two weeks, and a day's file can
/// hold tens of thousands of rows; keeping only the account flat means
/// the read never holds the window in memory.
#[derive(Debug, Default)]
struct Account {
    ledgers: Vec<Ledger>,
    unaccounted: u64,
}

impl Account {
    /// Fold one route row in. Non-route rows never reach here.
    fn add(&mut self, row: &ReadRow) {
        // A route row exists whether or not its call's usage does, so the
        // call is counted first: a model whose provider omits usage still
        // reads its real call count, with an absent rate rather than zero.
        let index = match self
            .ledgers
            .iter()
            .position(|one| one.provider == row.provider && one.model == row.model)
        {
            Some(index) => index,
            None => {
                self.ledgers.push(Ledger {
                    provider: row.provider.clone(),
                    model: row.model.clone(),
                    ..Ledger::default()
                });
                self.ledgers.len() - 1
            }
        };
        let ledgers = &mut self.ledgers;
        let day = row.ts.chars().take(10).collect::<String>();
        let cache = row.usage.as_ref().and_then(|usage| usage.cache_read_tokens);
        let ledger = &mut ledgers[index];
        ledger.calls += 1;
        if row.ts > ledger.last_at {
            ledger.last_at = row.ts.clone();
        }
        let Some(usage) = row.usage.as_ref() else {
            ledger.note_day(&day, 0, None);
            self.unaccounted += 1;
            return;
        };
        ledger.accounted += 1;
        ledger.prompt += usage.prompt_tokens;
        ledger.completion += usage.completion_tokens;
        ledger.note_day(&day, usage.prompt_tokens, cache);
        if let Some(cache) = cache {
            ledger.reported += 1;
            ledger.reporting_prompt += usage.prompt_tokens;
            ledger.cache_read += cache;
            ledger.reported_rows.push((usage.prompt_tokens, cache));
            if cache > 0 {
                ledger.shared_calls += 1;
                ledger.cached_ms.push(row.latency_ms);
            } else {
                ledger.fresh_ms.push(row.latency_ms);
            }
        }
        if let Some(creation) = usage.cache_creation_tokens {
            ledger.creation_reported += 1;
            ledger.cache_creation += creation;
        }
        if let Some(reasoning) = usage.reasoning_tokens {
            ledger.reasoning_reported += 1;
            ledger.reasoning += reasoning;
        }
        if usage.total_tokens > 0 {
            ledger.total_reported += 1;
            ledger.provider_total += usage.total_tokens;
        }
    }

    /// Render the account as the instrument's document.
    fn finish(self, window_days: usize) -> Value {
        let mut ledgers = self.ledgers;

        // Volume first: the model carrying the most prompt tokens is the one
        // whose rate matters most, and the surface draws in that order.
        ledgers.sort_by(|a, b| {
            b.prompt
                .cmp(&a.prompt)
                .then_with(|| a.model.to_lowercase().cmp(&b.model.to_lowercase()))
        });

        let models: Vec<Value> = ledgers
            .iter()
            .map(|ledger| {
                let half = ledger.reported_rows.len() / 2;
                let early = weighted(&ledger.reported_rows[..half]);
                let late = weighted(&ledger.reported_rows[half..]);
                json!({
                    "provider": ledger.provider,
                    "model": ledger.model,
                    "calls": ledger.calls,
                    "accounted": ledger.accounted,
                    "reported": ledger.reported,
                    "shared_calls": ledger.shared_calls,
                    "prompt": ledger.prompt,
                    "reporting_prompt": ledger.reporting_prompt,
                    "cache_read": ledger.cache_read,
                    "fresh": ledger.reporting_prompt.saturating_sub(ledger.cache_read),
                    "completion": ledger.completion,
                    "share": ledger.share(),
                    "latency": {
                        "cached": latency_side(&ledger.cached_ms),
                        "fresh": latency_side(&ledger.fresh_ms),
                    },
                    "series": ledger.days.iter().map(|(day, prompt, cache, calls)| json!({
                        "day": day,
                        "prompt": prompt,
                        "cache_read": cache,
                        "calls": calls,
                        "share": (*prompt > 0).then(|| *cache as f64 / *prompt as f64),
                    })).collect::<Vec<_>>(),
                    "halves": {"early": early, "late": late},
                    "last_at": ledger.last_at,
                })
            })
            .collect();

        let calls: u64 = ledgers.iter().map(|one| one.calls).sum();
        let accounted: u64 = ledgers.iter().map(|one| one.accounted).sum();
        let prompt: u64 = ledgers.iter().map(|one| one.prompt).sum();
        let reporting_prompt: u64 = ledgers.iter().map(|one| one.reporting_prompt).sum();
        let cache_read: u64 = ledgers.iter().map(|one| one.cache_read).sum();
        let reporting_models = ledgers.iter().filter(|one| one.reported > 0).count();

        json!({
            "type": "cache",
            "window_days": window_days,
            "models": models,
            "totals": {
                "calls": calls,
                "accounted": accounted,
                "unaccounted": self.unaccounted,
                "prompt": prompt,
                "reporting_prompt": reporting_prompt,
                "cache_read": cache_read,
                "fresh": reporting_prompt.saturating_sub(cache_read),
                "share": (reporting_prompt > 0)
                    .then(|| cache_read as f64 / reporting_prompt as f64),
                "models": ledgers.len(),
                "reporting_models": reporting_models,
            },
            "read_at": crate::calib::stamp(),
        })
    }
}

/// The project a route row belongs to, as the session archive resolves it —
/// or the honest bucket when it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProjectKey {
    Project {
        path: String,
        inferred: bool,
    },
    /// A row with no session on it: written before rows named their
    /// conversation, or a run prepared without one (cron, a one-shot CLI,
    /// an onboarding probe).
    Unattributed,
    /// The session exists and its Start record (and its own paths) name no
    /// workspace.
    Unbound,
    /// The session id is on the row and the log is no longer in the archive.
    UnknownSession,
}

impl ProjectKey {
    fn bucket(&self) -> Option<&'static str> {
        match self {
            ProjectKey::Project { .. } => None,
            ProjectKey::Unattributed => Some("unattributed"),
            ProjectKey::Unbound => Some("unbound"),
            ProjectKey::UnknownSession => Some("unknown session"),
        }
    }

    /// The host's own sentence for a bucket, so every surface says the same
    /// thing about the calls no project can claim.
    fn says(&self) -> Option<&'static str> {
        match self {
            ProjectKey::Project { .. } => None,
            ProjectKey::Unattributed => Some("rows written before routes named their session"),
            ProjectKey::Unbound => Some("the session exists and has no workspace"),
            ProjectKey::UnknownSession => Some("the session's log is no longer in the archive"),
        }
    }

    fn path(&self) -> Option<&str> {
        match self {
            ProjectKey::Project { path, .. } => Some(path),
            _ => None,
        }
    }

    /// The label a card shows: the project folder's own name.
    fn name(&self) -> Option<String> {
        self.path().map(|path| {
            path.trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .to_string()
        })
    }

    fn inferred(&self) -> bool {
        matches!(self, ProjectKey::Project { inferred: true, .. })
    }

    /// The bucket's fixed place after the real projects — unattributed,
    /// unbound, unknown session, in that order, never ranked against each
    /// other by volume.
    fn order(&self) -> (u8, u8) {
        match self {
            ProjectKey::Project { .. } => (0, 0),
            ProjectKey::Unattributed => (1, 0),
            ProjectKey::Unbound => (1, 1),
            ProjectKey::UnknownSession => (1, 2),
        }
    }
}

impl Account {
    /// One project's token view: its models by billed tokens, and the totals
    /// the card's centre carries. Cache fields stay `null` when no row in the
    /// group reported them — a provider that never reports is not a provider
    /// that reported zero.
    fn project_json(self, key: &ProjectKey) -> Value {
        let mut ledgers = self.ledgers;
        ledgers.sort_by(|a, b| {
            (b.prompt + b.completion)
                .cmp(&(a.prompt + a.completion))
                .then_with(|| a.model.to_lowercase().cmp(&b.model.to_lowercase()))
        });
        let billed = |one: &Ledger| one.prompt.saturating_add(one.completion);
        let models: Vec<Value> = ledgers
            .iter()
            .map(|ledger| {
                let total = ledger.prompt.saturating_add(ledger.completion);
                json!({
                    "provider": ledger.provider,
                    "model": ledger.model,
                    "calls": ledger.calls,
                    "accounted": ledger.accounted,
                    "prompt": ledger.prompt,
                    "completion": ledger.completion,
                    "billed": total,
                    "cache_read": (ledger.reported > 0).then_some(ledger.cache_read),
                    "cache_creation": (ledger.creation_reported > 0)
                        .then_some(ledger.cache_creation),
                    "reasoning": (ledger.reasoning_reported > 0)
                        .then_some(ledger.reasoning),
                    "provider_total": (ledger.total_reported > 0)
                        .then_some(ledger.provider_total),
                    "provider_total_differs": ledger.total_reported > 0
                        && ledger.provider_total != total,
                    "last_at": ledger.last_at,
                })
            })
            .collect();
        let sum = |f: &dyn Fn(&Ledger) -> u64| ledgers.iter().map(f).sum::<u64>();
        let total_calls = sum(&|one| one.calls);
        let total_accounted = sum(&|one| one.accounted);
        let total_billed: u64 = ledgers.iter().map(billed).sum();
        let cache_read = ledgers
            .iter()
            .any(|one| one.reported > 0)
            .then(|| sum(&|one| one.cache_read));
        let cache_creation = ledgers
            .iter()
            .any(|one| one.creation_reported > 0)
            .then(|| sum(&|one| one.cache_creation));
        let reasoning = ledgers
            .iter()
            .any(|one| one.reasoning_reported > 0)
            .then(|| sum(&|one| one.reasoning));
        let provider_total = ledgers
            .iter()
            .any(|one| one.total_reported > 0)
            .then(|| sum(&|one| one.provider_total));
        json!({
            "project": key.path(),
            "name": key.name(),
            "bucket": key.bucket(),
            "says": key.says(),
            "inferred": key.inferred(),
            "models": models,
            "totals": {
                "calls": total_calls,
                "accounted": total_accounted,
                "prompt": sum(&|one| one.prompt),
                "completion": sum(&|one| one.completion),
                "billed": total_billed,
                "cache_read": cache_read,
                "cache_creation": cache_creation,
                "reasoning": reasoning,
                "provider_total": provider_total,
                "provider_total_differs": ledgers.iter().any(|one| {
                    one.total_reported > 0 && one.provider_total != billed(one)
                }),
                "models": ledgers.len(),
            },
        })
    }
}

/// The running document: the model-wide account (the cache plugin's contract),
/// and — when the caller can resolve a session to its project — one account per
/// project. One walk feeds both, so the two views cannot disagree about a row.
struct Builder<'a> {
    account: Account,
    projects: Vec<(ProjectKey, Account)>,
    by_session: std::collections::HashMap<String, ProjectKey>,
    workspace_of: Option<&'a WorkspaceLookup<'a>>,
}

impl<'a> Builder<'a> {
    fn new(workspace_of: Option<&'a WorkspaceLookup<'a>>) -> Self {
        Builder {
            account: Account::default(),
            projects: Vec::new(),
            by_session: std::collections::HashMap::new(),
            workspace_of,
        }
    }

    fn key_for(&mut self, session: &str) -> ProjectKey {
        if session.is_empty() {
            return ProjectKey::Unattributed;
        }
        if let Some(key) = self.by_session.get(session) {
            return key.clone();
        }
        let Some(lookup) = self.workspace_of else {
            return ProjectKey::UnknownSession;
        };
        let key = match lookup(session) {
            None => ProjectKey::UnknownSession,
            Some(fact) => match fact.path {
                Some(path) => ProjectKey::Project {
                    path,
                    inferred: fact.inferred,
                },
                None => ProjectKey::Unbound,
            },
        };
        self.by_session.insert(session.to_string(), key.clone());
        key
    }

    fn add(&mut self, row: &ReadRow) {
        self.account.add(row);
        if self.workspace_of.is_none() {
            return;
        }
        let key = self.key_for(&row.session);
        let index = match self.projects.iter().position(|(one, _)| *one == key) {
            Some(index) => index,
            None => {
                self.projects.push((key, Account::default()));
                self.projects.len() - 1
            }
        };
        self.projects[index].1.add(row);
    }

    fn finish(mut self, window_days: usize) -> Value {
        let mut doc = self.account.finish(window_days);
        let mut projects: Vec<(ProjectKey, Account)> = std::mem::take(&mut self.projects);
        // The projects by billed tokens, the buckets after them in their own
        // fixed order — a bucket is never ranked against a project.
        projects.sort_by(|(a, aa), (b, bb)| {
            a.order().cmp(&b.order()).then_with(|| {
                let billed = |account: &Account| -> u64 {
                    account
                        .ledgers
                        .iter()
                        .map(|one| one.prompt.saturating_add(one.completion))
                        .sum()
                };
                billed(bb)
                    .cmp(&billed(aa))
                    .then_with(|| a.path().unwrap_or("").cmp(b.path().unwrap_or("")))
            })
        });
        let projects: Vec<Value> = projects
            .into_iter()
            .map(|(key, account)| account.project_json(&key))
            .collect();
        if let Value::Object(ref mut map) = doc {
            map.insert("projects".to_string(), Value::Array(projects));
        }
        doc
    }
}

/// The document over a slice of rows — the shape the tests drive.
pub fn document(rows: &[ReadRow], window_days: usize) -> Value {
    document_with(rows, window_days, None)
}

/// The document with the project dimension, over a slice of rows.
pub fn document_with(
    rows: &[ReadRow],
    window_days: usize,
    workspace_of: Option<&WorkspaceLookup<'_>>,
) -> Value {
    let mut builder = Builder::new(workspace_of);
    for row in rows.iter().filter(|row| row.op == OP_ROUTE) {
        builder.add(row);
    }
    builder.finish(window_days)
}

/// The document over the stream's newest `days` day-files, **streamed**:
/// the demotion reader takes a bounded tail per day ([`crate::calib::
/// READ_CAP`]), which is right for a streak and wrong for a volume — the
/// cache view wants every route row of the window, so it reads each file
/// line by line and keeps only the account.
pub fn read_document(root: &Path, days: usize) -> Value {
    read_document_with(root, days, None)
}

/// The document over the stream's newest `days` day-files, with the project
/// dimension resolved through `workspace_of` (one session-log read per distinct
/// session in the window, memoised for the walk).
pub fn read_document_with(
    root: &Path,
    days: usize,
    workspace_of: Option<&WorkspaceLookup<'_>>,
) -> Value {
    use std::io::BufRead;
    let mut builder = Builder::new(workspace_of);
    for file in crate::calib::day_files_recent(root, days) {
        let Ok(handle) = std::fs::File::open(&file) else {
            continue;
        };
        let mut reader = std::io::BufReader::new(handle);
        let mut line = String::new();
        loop {
            line.clear();
            let Ok(read) = reader.read_line(&mut line) else {
                break;
            };
            if read == 0 {
                break;
            }
            let Ok(row) = serde_json::from_str::<ReadRow>(line.trim()) else {
                continue;
            };
            if row.op == OP_ROUTE {
                builder.add(&row);
            }
        }
    }
    builder.finish(days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Usage;

    fn usage(prompt: u64, cache: Option<u64>) -> Usage {
        Usage {
            prompt_tokens: prompt,
            completion_tokens: 10,
            total_tokens: prompt + 10,
            cache_read_tokens: cache,
            ..Usage::default()
        }
    }

    fn row(ts: &str, provider: &str, model: &str, usage: Option<Usage>, ms: u64) -> ReadRow {
        ReadRow {
            ts: ts.to_string(),
            op: OP_ROUTE.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            usage,
            latency_ms: ms,
            ..ReadRow::default()
        }
    }

    /// The tokens-by-project plan's C2: one walk feeds both views; rows are
    /// grouped by the project their session was born in, and the calls no
    /// session can claim are their own named buckets — never guessed, never
    /// merged. The provider's own total is carried, and a disagreement with
    /// prompt + completion is a flag, not a silent reconciliation.
    #[test]
    fn the_projects_group_by_the_sessions_workspace_and_name_their_buckets() {
        fn row_in(
            ts: &str,
            provider: &str,
            model: &str,
            usage: Option<Usage>,
            ms: u64,
            session: &str,
        ) -> ReadRow {
            let mut one = row(ts, provider, model, usage, ms);
            one.session = session.to_string();
            one
        }
        let mut mismatch = usage(200, Some(20));
        mismatch.total_tokens = 999;
        let rows = vec![
            row_in(
                "2026-11-20T10:00:00Z",
                "p1",
                "m1",
                Some(usage(100, Some(10))),
                100,
                "s1",
            ),
            row_in(
                "2026-11-20T11:00:00Z",
                "p1",
                "m1",
                Some(usage(50, None)),
                110,
                "s2",
            ),
            row_in(
                "2026-11-20T12:00:00Z",
                "p2",
                "m2",
                Some(mismatch),
                120,
                "s3",
            ),
            row_in(
                "2026-11-20T13:00:00Z",
                "p1",
                "m1",
                Some(usage(30, None)),
                130,
                "",
            ),
            row_in(
                "2026-11-20T14:00:00Z",
                "p3",
                "m3",
                Some(usage(40, None)),
                140,
                "gone",
            ),
            row_in(
                "2026-11-20T15:00:00Z",
                "p4",
                "m4",
                Some(usage(60, None)),
                150,
                "s4",
            ),
        ];
        let facts = [
            ("s1", Some("/Users/x/Projects/alpha")),
            ("s2", Some("/Users/x/Projects/alpha")),
            ("s3", Some("/Users/x/Projects/beta")),
            ("s4", None),
        ];
        let lookup = |id: &str| {
            facts.iter().find(|(one, _)| *one == id).map(|(_, path)| {
                crate::session::WorkspaceFact {
                    path: path.map(str::to_string),
                    inferred: false,
                }
            })
        };
        let doc = document_with(&rows, 14, Some(&lookup));
        let projects = doc["projects"].as_array().expect("projects");
        assert_eq!(projects.len(), 5, "{projects:?}");
        // The two real projects first (by billed tokens: beta's 210 over
        // alpha's 170); then the buckets in their fixed order.
        assert_eq!(projects[0]["name"], "beta");
        assert_eq!(projects[1]["name"], "alpha");
        assert_eq!(projects[2]["bucket"], "unattributed");
        assert_eq!(
            projects[2]["says"],
            "rows written before routes named their session"
        );
        assert_eq!(projects[3]["bucket"], "unbound");
        assert_eq!(projects[4]["bucket"], "unknown session");
        assert_eq!(projects[2]["project"], Value::Null);
        assert_eq!(projects[2]["name"], Value::Null);

        // One project, two sessions, one model row — a session is never a row.
        let alpha = projects
            .iter()
            .find(|one| one["name"] == "alpha")
            .expect("alpha");
        assert_eq!(alpha["inferred"], false);
        assert_eq!(alpha["models"].as_array().expect("models").len(), 1);
        let model = &alpha["models"][0];
        assert_eq!(model["model"], "m1");
        assert_eq!(model["calls"], 2);
        assert_eq!(model["accounted"], 2);
        assert_eq!(model["prompt"], 150);
        assert_eq!(model["completion"], 20);
        assert_eq!(model["billed"], 170);
        // Cache read: only the reporting row's 10 — absence is not zero, and
        // the unreported fields stay null.
        assert_eq!(model["cache_read"], 10);
        assert_eq!(model["cache_creation"], Value::Null);
        assert_eq!(model["reasoning"], Value::Null);
        assert_eq!(model["provider_total"], 170);
        assert_eq!(model["provider_total_differs"], false);
        assert_eq!(alpha["totals"]["calls"], 2);
        assert_eq!(alpha["totals"]["billed"], 170);
        assert_eq!(alpha["totals"]["cache_read"], 10);

        // The provider's own total disagreeing with prompt + completion is a
        // flag on the row and on the project.
        let beta = projects
            .iter()
            .find(|one| one["name"] == "beta")
            .expect("beta");
        assert_eq!(beta["models"][0]["provider_total"], 999);
        assert_eq!(beta["models"][0]["provider_total_differs"], true);
        assert_eq!(beta["totals"]["provider_total"], 999);
        assert_eq!(beta["totals"]["provider_total_differs"], true);

        // A bucket's model with no cache report anywhere: null, never 0.
        assert_eq!(projects[3]["models"][0]["cache_read"], Value::Null);
        // The model-wide view is untouched: every row is still there.
        let models = doc["models"].as_array().expect("models");
        assert_eq!(models.len(), 4, "{models:?}");
        assert_eq!(doc["totals"]["calls"], 6);

        // Without a lookup the project dimension is simply absent — the cache
        // plugin's own contract.
        let plain = document(&rows, 14);
        assert_eq!(plain["projects"].as_array().expect("projects").len(), 0);
    }

    #[test]
    fn the_models_group_and_order_by_their_prompt_volume() {
        let rows = vec![
            row(
                "2026-11-15T01:00:00Z",
                "p1",
                "small",
                Some(usage(100, Some(10))),
                100,
            ),
            row(
                "2026-11-15T01:01:00Z",
                "p1",
                "big",
                Some(usage(1000, Some(500))),
                100,
            ),
            row(
                "2026-11-15T01:02:00Z",
                "p1",
                "big",
                Some(usage(1000, Some(0))),
                120,
            ),
        ];
        let document = document(&rows, WINDOW_DAYS);
        let models = document["models"].as_array().expect("models");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0]["model"], "big");
        assert_eq!(models[1]["model"], "small");
        assert_eq!(models[0]["calls"].as_u64(), Some(2));
        assert_eq!(models[0]["prompt"].as_u64(), Some(2000));
        assert_eq!(models[0]["cache_read"].as_u64(), Some(500));
        // 500 of 2000 prompt tokens, over the two reporting rows.
        assert_eq!(models[0]["share"].as_f64(), Some(0.25));
        assert_eq!(document["totals"]["share"].as_f64(), Some(510.0 / 2100.0));
    }

    #[test]
    fn a_model_that_never_reports_cache_is_null_not_zero() {
        let rows = vec![row(
            "2026-11-15T01:00:00Z",
            "p1",
            "silent",
            Some(usage(900, None)),
            50,
        )];
        let document = document(&rows, WINDOW_DAYS);
        assert_eq!(document["models"][0]["share"], Value::Null);
        assert_eq!(document["models"][0]["reported"].as_u64(), Some(0));
        assert_eq!(document["totals"]["share"], Value::Null);
        // The volume is still counted; only the rate is absent.
        assert_eq!(document["totals"]["prompt"].as_u64(), Some(900));
    }

    #[test]
    fn the_latency_split_reports_only_when_each_side_has_enough_rows() {
        let mut rows = Vec::new();
        for index in 0..4 {
            rows.push(row(
                "2026-11-15T01:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(0))),
                400 + index,
            ));
        }
        rows.push(row(
            "2026-11-15T01:00:00Z",
            "p1",
            "m",
            Some(usage(100, Some(50))),
            90,
        ));
        let document = document(&rows, WINDOW_DAYS);
        assert_eq!(
            document["models"][0]["latency"]["fresh"]["p50"].as_u64(),
            Some(401)
        );
        assert_eq!(
            document["models"][0]["latency"]["fresh"]["rows"].as_u64(),
            Some(4)
        );
        // One cached row is a number, not a measurement.
        assert_eq!(
            document["models"][0]["latency"]["cached"]["p50"],
            Value::Null
        );
        assert_eq!(
            document["models"][0]["latency"]["cached"]["rows"].as_u64(),
            Some(1)
        );
    }

    #[test]
    fn the_halves_name_a_collapse() {
        let mut rows = Vec::new();
        for _ in 0..4 {
            rows.push(row(
                "2026-11-15T01:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(80))),
                10,
            ));
        }
        for _ in 0..4 {
            rows.push(row(
                "2026-11-15T02:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(0))),
                10,
            ));
        }
        let document = document(&rows, WINDOW_DAYS);
        assert_eq!(document["models"][0]["halves"]["early"].as_f64(), Some(0.8));
        assert_eq!(document["models"][0]["halves"]["late"].as_f64(), Some(0.0));
    }

    #[test]
    fn rows_without_usage_are_counted_not_dropped() {
        let rows = vec![
            row(
                "2026-11-15T01:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(50))),
                10,
            ),
            row("2026-11-15T01:01:00Z", "p1", "m", None, 10),
        ];
        let document = document(&rows, WINDOW_DAYS);
        assert_eq!(document["models"][0]["calls"].as_u64(), Some(2));
        assert_eq!(document["models"][0]["accounted"].as_u64(), Some(1));
        assert_eq!(document["totals"]["unaccounted"].as_u64(), Some(1));
    }

    #[test]
    fn an_empty_window_is_empty_not_invented() {
        let document = document(&[], WINDOW_DAYS);
        assert_eq!(document["models"].as_array().expect("models").len(), 0);
        assert_eq!(document["totals"]["calls"].as_u64(), Some(0));
        assert_eq!(document["totals"]["share"], Value::Null);
        assert_eq!(document["window_days"].as_u64(), Some(WINDOW_DAYS as u64));
    }

    #[test]
    fn the_window_is_streamed_whole_not_a_daily_tail_sample() {
        let root = std::env::temp_dir().join(format!("fleet-cache-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (day, count) in [("2026-11-14", 3u32), ("2026-11-15", 2u32)] {
            let dir = root.join("2026").join("11").join(&day[8..]);
            std::fs::create_dir_all(&dir).expect("dir");
            let mut text = String::new();
            for index in 0..count {
                let row = json!({
                    "ts": format!("{day}T0{index}:00:00Z"),
                    "op": OP_ROUTE,
                    "provider": "p1",
                    "model": "m",
                    "latency_ms": 100,
                    "usage": {
                        "prompt_tokens": 100,
                        "completion_tokens": 5,
                        "total_tokens": 105,
                        "cache_read_input_tokens": 50
                    }
                });
                text.push_str(&row.to_string());
                text.push('\n');
            }
            std::fs::write(dir.join("events.jsonl"), text).expect("write");
        }
        let document = read_document(&root, WINDOW_DAYS);
        // Every route row of both days, not a capped tail.
        assert_eq!(document["totals"]["calls"].as_u64(), Some(5));
        assert_eq!(document["totals"]["cache_read"].as_u64(), Some(250));
        assert_eq!(
            document["models"][0]["series"]
                .as_array()
                .expect("days")
                .len(),
            2
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_daily_series_carries_the_day_share() {
        let rows = vec![
            row(
                "2026-11-14T23:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(90))),
                10,
            ),
            row(
                "2026-11-15T01:00:00Z",
                "p1",
                "m",
                Some(usage(100, Some(10))),
                10,
            ),
        ];
        let document = document(&rows, WINDOW_DAYS);
        let series = document["models"][0]["series"].as_array().expect("series");
        assert_eq!(series.len(), 2);
        assert_eq!(series[0]["day"], "2026-11-14");
        assert_eq!(series[0]["share"].as_f64(), Some(0.9));
        assert_eq!(series[1]["share"].as_f64(), Some(0.1));
    }
}
