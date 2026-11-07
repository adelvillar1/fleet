//! The demand map (the self-improvement plan's C0): the phantom
//! gradient's fleet instrument.
//!
//! The article's sharpest tool is a per-cell, per-checkpoint map of
//! *unmet demand* — where the system is failing to learn, so
//! interventions aim at the cells that stay high and flat while the
//! improving ones are left alone. Fleet's cells are the classes its
//! calibration stream already records: the routing judgment's
//! workload × provider, the eval corpus's scenarios, the delegation
//! outcomes, the validate verdicts. This module reads those rows over
//! a window and answers the demand question per class: the bad-rate,
//! the row count, and the trend.
//!
//! The trend is the article's rule made mechanical: a class whose
//! bad-rate is falling across the window is *improving* (left to
//! ordinary work); one that holds or rises is *flat* or *worsening* —
//! the intervention targets. A class with too few rows reads *thin*,
//! never zero: a small sample is not evidence of health, and the row
//! count rides every line so a reader can weigh it.
//!
//! Nothing here decides or applies anything. The map is a view; the
//! beats and the owner act on it.

use std::path::Path;

use crate::calib::{
    self, Row, Stream, OP_DELEGATION, OP_EVAL_CORPUS, OP_ROUTE_JUDGED, OP_VALIDATE,
};

/// The default window a demand view reads, in days.
pub const DEMAND_WINDOW_DAYS: usize = 14;

/// How many rows a class needs before its rate and trend mean
/// anything. Below this the class reads [`Trend::Thin`].
pub const DEMAND_MIN_ROWS: u64 = 6;

/// The bad-rate change between the window's halves that counts as a
/// trend rather than noise.
pub const DEMAND_TREND_EPSILON: f64 = 0.05;

/// The class's direction across the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// The bad-rate is falling — ordinary work is closing the gap.
    Improving,
    /// The bad-rate holds — the intervention target.
    Flat,
    /// The bad-rate is rising.
    Worsening,
    /// Too few rows to read a direction.
    Thin,
}

impl Trend {
    /// The word the surface prints.
    pub fn label(self) -> &'static str {
        match self {
            Trend::Improving => "improving",
            Trend::Flat => "flat",
            Trend::Worsening => "worsening",
            Trend::Thin => "thin",
        }
    }

    /// The marker: an arrow for a direction, a dash for thin.
    pub fn marker(self) -> &'static str {
        match self {
            Trend::Improving => "↓",
            Trend::Flat => "→",
            Trend::Worsening => "↑",
            Trend::Thin => "·",
        }
    }
}

/// One class's demand over the window.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDemand {
    /// The family the class belongs to: `route`, `eval`, `delegate`,
    /// or `validate`.
    pub family: String,
    /// The class itself: `deploy/glide`, a scenario's name, an
    /// outcome, a verdict.
    pub class: String,
    /// How many rows the class collected.
    pub rows: u64,
    /// How many of them were bad.
    pub bad: u64,
    /// `bad / rows`, in `[0, 1]`.
    pub rate: f64,
    pub trend: Trend,
}

/// Accumulator: the class key, its rows in chronological order with
/// their bad flags.
#[derive(Default)]
struct Accum {
    bad: u64,
    rows: u64,
    /// The bad-flags in row order (oldest first), for the halves.
    history: Vec<bool>,
}

impl Accum {
    fn add(&mut self, bad: bool) {
        self.rows += 1;
        self.bad += bad as u64;
        self.history.push(bad);
    }

    fn rate(&self) -> f64 {
        if self.rows == 0 {
            0.0
        } else {
            self.bad as f64 / self.rows as f64
        }
    }

    /// The trend across the window's halves: the older half's rate
    /// against the recent half's. The article's rule — cells that are
    /// still improving are left; high-flat cells are the targets.
    fn trend(&self) -> Trend {
        if self.rows < DEMAND_MIN_ROWS {
            return Trend::Thin;
        }
        let half = self.history.len() / 2;
        let rate_of = |slice: &[bool]| -> f64 {
            if slice.is_empty() {
                0.0
            } else {
                slice.iter().filter(|bad| **bad).count() as f64 / slice.len() as f64
            }
        };
        let older = rate_of(&self.history[..half]);
        let recent = rate_of(&self.history[half..]);
        if recent < older - DEMAND_TREND_EPSILON {
            Trend::Improving
        } else if recent > older + DEMAND_TREND_EPSILON {
            Trend::Worsening
        } else {
            Trend::Flat
        }
    }
}

/// Collect the demand map over `days` days of the stream under
/// `root`, ending at `day`. The families: the routing judgment's
/// workload × provider, the eval corpus's scenarios, the delegation
/// outcomes, the validate verdicts.
pub fn collect(root: &Path, day: &str, days: usize) -> Vec<ClassDemand> {
    let mut accums: std::collections::HashMap<(String, String), Accum> = Default::default();
    let mut classes = |family: &str, class: &str, bad: bool| {
        accums
            .entry((family.to_string(), class.to_string()))
            .or_default()
            .add(bad);
    };
    for row in calib::read_days_recent(root, day, days) {
        match row.op.as_str() {
            OP_ROUTE_JUDGED => {
                if row.workload.trim().is_empty() || row.provider.trim().is_empty() {
                    continue;
                }
                classes(
                    "route",
                    &format!("{}/{}", row.workload, row.provider),
                    row.verdict != "right",
                );
            }
            OP_EVAL_CORPUS => {
                for scenario in &row.scenarios {
                    classes("eval", &scenario.scenario, !scenario.passed);
                }
            }
            OP_DELEGATION => {
                if row.outcome.trim().is_empty() {
                    continue;
                }
                classes("delegate", &row.outcome, row.outcome != "completed");
            }
            OP_VALIDATE => {
                if row.verdict.trim().is_empty() {
                    continue;
                }
                classes("validate", &row.verdict, row.verdict == "contradicted");
            }
            _ => {}
        }
    }
    let mut out: Vec<ClassDemand> = accums
        .into_iter()
        .map(|((family, class), accum)| ClassDemand {
            family,
            class,
            rows: accum.rows,
            bad: accum.bad,
            rate: accum.rate(),
            trend: accum.trend(),
        })
        .collect();
    // The intervention targets first: the highest bad-rate, then the
    // most rows, then the name — a stable order a reader can scan.
    out.sort_by(|left, right| {
        right
            .rate
            .partial_cmp(&left.rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.rows.cmp(&left.rows))
            .then_with(|| left.class.cmp(&right.class))
    });
    out
}

/// The strongest demand in the map: the highest bad-rate among the
/// classes with enough rows to mean anything, the row count the
/// tie-break (equal rates: the better-evidenced class wins).
/// `None` when every class is thin — the honest "nothing measurable"
/// answer.
pub fn top(demands: &[ClassDemand]) -> Option<&ClassDemand> {
    demands
        .iter()
        .filter(|demand| demand.rows >= DEMAND_MIN_ROWS)
        .max_by(|left, right| {
            left.rate
                .partial_cmp(&right.rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.rows.cmp(&right.rows))
        })
}

/// Record one demand view on the stream — the audit trail that a view
/// ran and what it saw.
pub fn record_view(stream: &Stream, window_days: usize, demands: &[ClassDemand]) {
    let top = top(demands);
    stream.record(Row::Demand(calib::DemandRow {
        ts: calib::stamp(),
        window_days,
        classes: demands.len(),
        top_class: top.map(|demand| demand.class.clone()).unwrap_or_default(),
        top_rate: top.map(|demand| demand.rate).unwrap_or(0.0),
    }));
}

/// The surface's card width.
const CARD_WIDTH: usize = 72;

/// Render the demand map as the CLI's cards: per family, the classes
/// strongest first, each with its rate, trend, and row count.
pub fn render(demands: &[ClassDemand], days: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "the demand map — {days} day{} of the stream, {} class{}\n",
        if days == 1 { "" } else { "s" },
        demands.len(),
        if demands.len() == 1 { "" } else { "es" }
    ));
    if demands.is_empty() {
        out.push_str("\nno graded rows in the window — nothing to read\n");
        return out;
    }
    let top = top(demands);
    match top {
        Some(demand) => out.push_str(&format!(
            "top demand: {} — {:.0}% bad over {} row{}\n",
            demand.class,
            demand.rate * 100.0,
            demand.rows,
            if demand.rows == 1 { "" } else { "s" }
        )),
        None => out.push_str("top demand: none — every class is thin under the row floor\n"),
    }
    for family in ["route", "eval", "delegate", "validate"] {
        let members: Vec<&ClassDemand> = demands
            .iter()
            .filter(|demand| demand.family == family)
            .collect();
        if members.is_empty() {
            continue;
        }
        let heading = match family {
            "route" => "route (workload/provider)",
            "eval" => "eval (scenario)",
            "delegate" => "delegate (outcome)",
            _ => "validate (verdict)",
        };
        out.push_str(&format!("\n{heading}\n"));
        for demand in members {
            let class = bounded(&demand.class, 26);
            out.push_str(&format!(
                "  {} {:<26} {:>5.0}% bad  {:>4} row{}  {}\n",
                demand.trend.marker(),
                class,
                demand.rate * 100.0,
                demand.rows,
                if demand.rows == 1 { "" } else { "s" },
                demand.trend.label()
            ));
        }
    }
    out.push_str(&format!(
        "\n{}-char classes are cut with the mark; the row count weighs every line\n",
        CARD_WIDTH
    ));
    out
}

/// A bounded class name: cut at `bound` with the ellipsis mark, the
/// writer's own bound style — a fixed-width surface must not clip
/// mid-token.
fn bounded(text: &str, bound: usize) -> String {
    if text.chars().count() > bound {
        let cut: String = text.chars().take(bound.saturating_sub(1)).collect();
        format!("{cut}\u{2026}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calib::ReadRow;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-demand-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn route_row(ts: &str, workload: &str, provider: &str, verdict: &str) -> Row {
        Row::RouteJudged(calib::RouteJudgedRow {
            ts: ts.to_string(),
            route_key: format!("{provider}-{workload}-{verdict}"),
            provider: provider.to_string(),
            workload: workload.to_string(),
            verdict: verdict.to_string(),
            confidence: 0.9,
            reason: String::new(),
            disposition: None,
            cost_usd: None,
            latency_ms: None,
            lane: "sys1".to_string(),
        })
    }

    /// The map reads the four families' rates and trends over a
    /// fixture stream: an improving route class, a flat one, a
    /// worsening eval scenario, and a thin class that must read thin.
    #[test]
    fn the_map_reads_rates_and_trends_over_the_window() {
        let root = scratch("rates");
        let stream = Stream::open(&root);
        // deploy/glide: 3 bad of 10 older, 2 bad of 10 recent — improving.
        for index in 0..10 {
            stream.record(route_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "deploy",
                "glide",
                if index < 3 { "wrong" } else { "right" },
            ));
        }
        for index in 0..10 {
            stream.record(route_row(
                &format!("2026-11-1{index}T00:00:00Z"),
                "deploy",
                "glide",
                if index < 2 { "wrong" } else { "right" },
            ));
        }
        // build/drex: half bad throughout — flat, and the top demand.
        for index in 0..8 {
            stream.record(route_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "build",
                "drex",
                if index % 2 == 0 { "wrong" } else { "right" },
            ));
        }
        // eval: insertion-sort fails the recent runs — worsening.
        for index in 0..6 {
            let passed = index < 3;
            stream.record(Row::EvalCorpus(calib::EvalCorpusRow {
                ts: format!("2026-11-0{index}T00:00:00Z"),
                passed: if passed { 1 } else { 0 },
                failed: if passed { 0 } else { 1 },
                score: if passed { 1.0 } else { 0.0 },
                scenarios: vec![calib::EvalScenarioRow {
                    scenario: "insertion-sort".to_string(),
                    family: "clrs".to_string(),
                    passed,
                    failure: if passed {
                        None
                    } else {
                        Some("jagged".to_string())
                    },
                }],
                input_sha256: None,
            }));
        }
        // validate: one contradicted row — thin, never a rate.
        stream.record(Row::Validate(calib::ValidateRow {
            ts: "2026-11-05T00:00:00Z".to_string(),
            fact: "a-fact".to_string(),
            input_hash: "h".to_string(),
            verdict: "contradicted".to_string(),
            confidence: 0.9,
            decision: "superseded".to_string(),
            lane: "sys1".to_string(),
        }));
        stream.flush();
        let demands = collect(&root, "2026-11-20", 30);
        let find = |class: &str| {
            demands
                .iter()
                .find(|demand| demand.class == class)
                .unwrap_or_else(|| panic!("{class} missing from {demands:?}"))
        };
        let improving = find("deploy/glide");
        assert_eq!(improving.rows, 20);
        assert_eq!(improving.bad, 5);
        assert_eq!(
            improving.trend,
            Trend::Improving,
            "the rate falls across the halves"
        );
        let flat = find("build/drex");
        assert_eq!(flat.trend, Trend::Flat, "half bad throughout holds");
        let worsening = find("insertion-sort");
        assert_eq!(worsening.trend, Trend::Worsening, "the recent runs fail");
        let thin = find("contradicted");
        assert_eq!(thin.trend, Trend::Thin, "one row is not a rate");
        assert_eq!(thin.rows, 1);
        // The top demand is the flat class with the highest rate.
        let top = top(&demands).expect("a class over the floor");
        assert_eq!(top.class, "build/drex");
        assert!((top.rate - 0.5).abs() < 1e-9);
        // The render names the families and the top.
        let rendered = render(&demands, 30);
        assert!(rendered.contains("route (workload/provider)"), "{rendered}");
        assert!(rendered.contains("eval (scenario)"), "{rendered}");
        assert!(rendered.contains("validate (verdict)"), "{rendered}");
        assert!(rendered.contains("top demand: build/drex"), "{rendered}");
        // The view records exactly one demand row with the top class.
        record_view(&stream, 30, &demands);
        stream.flush();
        let rows = calib::read_days_recent(&root, "2026-11-20", 30);
        let demand_rows: Vec<&ReadRow> = rows
            .iter()
            .filter(|row| row.op == calib::OP_DEMAND)
            .collect();
        assert_eq!(demand_rows.len(), 1, "one view, one row");
        assert_eq!(demand_rows[0].op, "demand");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An empty window says so — no fabricated classes, and the row
    /// records zero.
    #[test]
    fn an_empty_window_reads_as_empty() {
        let root = scratch("empty");
        let demands = collect(&root, "2026-11-20", 14);
        assert!(demands.is_empty());
        assert!(top(&demands).is_none());
        let rendered = render(&demands, 14);
        assert!(rendered.contains("no graded rows"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
