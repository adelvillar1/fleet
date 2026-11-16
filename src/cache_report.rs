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

/// The document over a slice of rows — the shape the tests drive.
pub fn document(rows: &[ReadRow], window_days: usize) -> Value {
    let mut account = Account::default();
    for row in rows.iter().filter(|row| row.op == OP_ROUTE) {
        account.add(row);
    }
    account.finish(window_days)
}

/// The document over the stream's newest `days` day-files, **streamed**:
/// the demotion reader takes a bounded tail per day ([`crate::calib::
/// READ_CAP`]), which is right for a streak and wrong for a volume — the
/// cache view wants every route row of the window, so it reads each file
/// line by line and keeps only the account.
pub fn read_document(root: &Path, days: usize) -> Value {
    use std::io::BufRead;
    let mut account = Account::default();
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
                account.add(&row);
            }
        }
    }
    account.finish(days)
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
