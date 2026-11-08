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
    self, Row, Stream, OP_DELEGATION, OP_EVAL_CORPUS, OP_LEARN_CHECK, OP_ROUTE_JUDGED, OP_VALIDATE,
};
use crate::improve::{self, Basis, Change, Target};

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
            OP_LEARN_CHECK => {
                // The learning feed: the class is the proposition the
                // step taught, and a row is bad when the learner named
                // a confusion — the map reads which propositions the
                // learner struggles with, and the trend says whether
                // the struggle is easing. An empty confusion (the step
                // held first) is a clean row, not a demand.
                if row.node.trim().is_empty() {
                    continue;
                }
                classes("learn", &row.node, !row.confusion.trim().is_empty());
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

// --- the clustering beat (C1) ----------------------------------------------

/// How many bad rows a class needs before the beat files a shape
/// proposal for it. The article's rule — a cluster that repeats is a
/// failure mode worth naming; a single bad row is noise.
pub const CLUSTER_MIN_BAD: u64 = 5;

/// The clusters at or over the filing threshold — the beat's
/// candidate set. A cluster under the threshold is noise; the beat
/// files nothing for it.
pub fn filing_clusters(clusters: &[Cluster]) -> Vec<&Cluster> {
    clusters
        .iter()
        .filter(|cluster| cluster.bad >= CLUSTER_MIN_BAD)
        .collect()
}

/// How many examples a cluster surfaces, and how long each may be.
const CLUSTER_EXAMPLES: usize = 3;
const CLUSTER_EXAMPLE_CHARS: usize = 80;

/// One failure cluster: a demand class's bad rows over the window,
/// with the examples that name the mode.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub family: String,
    pub class: String,
    pub bad: u64,
    pub rows: u64,
    pub rate: f64,
    /// The cluster's examples, deduplicated, bounded — the rows a
    /// reader names the failure mode from.
    pub examples: Vec<String>,
}

/// Group the window's failures by class. Every bad row joins its
/// class's cluster; the examples surface the rows' own detail.
pub fn clusters(root: &Path, day: &str, days: usize) -> Vec<Cluster> {
    use std::collections::HashMap;
    #[derive(Default)]
    struct Accum {
        bad: u64,
        rows: u64,
        examples: Vec<String>,
    }
    let mut accums: HashMap<(String, String), Accum> = HashMap::new();
    let mut add = |family: &str, class: &str, bad: bool, example: Option<String>| {
        let entry = accums
            .entry((family.to_string(), class.to_string()))
            .or_default();
        entry.rows += 1;
        if bad {
            entry.bad += 1;
            if let Some(example) = example {
                let example = bounded(&example, CLUSTER_EXAMPLE_CHARS);
                if !entry.examples.contains(&example) && entry.examples.len() < CLUSTER_EXAMPLES {
                    entry.examples.push(example);
                }
            }
        }
    };
    for row in calib::read_days_recent(root, day, days) {
        match row.op.as_str() {
            OP_ROUTE_JUDGED => {
                if row.workload.trim().is_empty() || row.provider.trim().is_empty() {
                    continue;
                }
                let bad = row.verdict != "right";
                let example = if row.reason.trim().is_empty() {
                    format!("{} {} {}", row.workload, row.provider, row.verdict)
                } else {
                    row.reason.clone()
                };
                add(
                    "route",
                    &format!("{}/{}", row.workload, row.provider),
                    bad,
                    Some(example),
                );
            }
            OP_EVAL_CORPUS => {
                for scenario in &row.scenarios {
                    let example = if scenario.failure.trim().is_empty() {
                        scenario.scenario.clone()
                    } else {
                        scenario.failure.clone()
                    };
                    add("eval", &scenario.scenario, !scenario.passed, Some(example));
                }
            }
            OP_DELEGATION => {
                if row.outcome.trim().is_empty() {
                    continue;
                }
                let example = format!("{}: {}", row.outcome, row.session);
                add(
                    "delegate",
                    &row.outcome,
                    row.outcome != "completed",
                    Some(example),
                );
            }
            OP_VALIDATE => {
                if row.verdict.trim().is_empty() {
                    continue;
                }
                let example = format!("{}: {}", row.verdict, row.fact);
                add(
                    "validate",
                    &row.verdict,
                    row.verdict == "contradicted",
                    Some(example),
                );
            }
            OP_LEARN_CHECK => {
                if row.node.trim().is_empty() {
                    continue;
                }
                // The example is the learner's own confusion — the
                // cluster's evidence is what they said, not a summary.
                add(
                    "learn",
                    &row.node,
                    !row.confusion.trim().is_empty(),
                    Some(row.confusion.clone()),
                );
            }
            _ => {}
        }
    }
    let mut out: Vec<Cluster> = accums
        .into_iter()
        .map(|((family, class), accum)| Cluster {
            family,
            class,
            bad: accum.bad,
            rows: accum.rows,
            rate: if accum.rows == 0 {
                0.0
            } else {
                accum.bad as f64 / accum.rows as f64
            },
            examples: accum.examples,
        })
        .collect();
    out.sort_by(|left, right| {
        right
            .bad
            .cmp(&left.bad)
            .then_with(|| left.class.cmp(&right.class))
    });
    out
}

/// The clustering view's rendering: per cluster, its bad count, rate,
/// and the examples that name the mode.
pub fn render_clusters(clusters: &[Cluster], days: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "failure clusters — {days} day{} of the stream, {} class{} with failures\n",
        if days == 1 { "" } else { "s" },
        clusters.len(),
        if clusters.len() == 1 { "" } else { "es" }
    ));
    let filing: Vec<&Cluster> = clusters
        .iter()
        .filter(|cluster| cluster.bad >= CLUSTER_MIN_BAD)
        .collect();
    out.push_str(&format!(
        "{} cluster{} at or over the {}-bad filing threshold\n",
        filing.len(),
        if filing.len() == 1 { "" } else { "s" },
        CLUSTER_MIN_BAD
    ));
    for cluster in clusters {
        let mark = if cluster.bad >= CLUSTER_MIN_BAD {
            "file"
        } else {
            "    "
        };
        out.push_str(&format!(
            "\n[{}] {:<28} {:>3} bad of {:>3}  {:.0}%\n",
            mark,
            bounded(&cluster.class, 28),
            cluster.bad,
            cluster.rows,
            cluster.rate * 100.0
        ));
        for example in &cluster.examples {
            out.push_str(&format!("      · {}\n", example));
        }
    }
    out
}

/// A class name as a taxonomy word: lowercase, hyphens, bounded — the
/// shape vocabulary's own word shape. The beat proposes with the
/// class-derived word; the owner's apply is the naming gate.
fn taxonomy_word(class: &str) -> String {
    let mut word = String::new();
    let mut last_hyphen = false;
    for character in class.chars() {
        if character.is_ascii_alphanumeric() {
            word.push(character.to_ascii_lowercase());
            last_hyphen = false;
        } else if !last_hyphen && !word.is_empty() {
            word.push('-');
            last_hyphen = true;
        }
        if word.chars().count() >= 32 {
            break;
        }
    }
    word.trim_matches('-').to_string()
}

/// The trigger keywords for a proposed tag: the class's own words,
/// deduplicated, bounded — the taxonomy's keyword-table shape.
fn taxonomy_keywords(class: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in class.split(|character: char| !character.is_alphanumeric()) {
        let word = word.to_ascii_lowercase();
        if word.is_empty() || word.chars().count() > 24 || words.contains(&word) {
            continue;
        }
        words.push(word);
        if words.len() >= 6 {
            break;
        }
    }
    words
}

/// The taxonomy patch: the full new content of `src/memory.rs` with
/// the tag joined to `FAILURE_SHAPES` and `FAILURE_SHAPE_KEYWORDS`.
/// A patch, not a diff — the improve store's Patch change carries the
/// file's whole new content and the gate guards it.
fn taxonomy_patch(repo_root: &Path, tag: &str, keywords: &[String]) -> Result<String, String> {
    let path = repo_root.join("src/memory.rs");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    // The failure-shape array: bump the count, insert before its close.
    let shapes_anchor = "pub const FAILURE_SHAPES: [&str; 6] = [";
    let shapes_start = text
        .find(shapes_anchor)
        .ok_or_else(|| "the FAILURE_SHAPES array moved".to_string())?;
    let shapes_close = text[shapes_start..]
        .find("];")
        .map(|offset| shapes_start + offset)
        .ok_or_else(|| "the FAILURE_SHAPES close moved".to_string())?;
    let mut out = String::new();
    out.push_str(&text[..shapes_start]);
    out.push_str("pub const FAILURE_SHAPES: [&str; 7] = [");
    out.push_str(&text[shapes_anchor.len() + shapes_start..shapes_close]);
    out.push_str(&format!("    \"{tag}\",\n"));
    out.push_str(&text[shapes_close..]);
    // The keyword table: the same shape.
    let text = out;
    let keywords_anchor = "pub const FAILURE_SHAPE_KEYWORDS: [(&str, &[&str]); 6] = [";
    let keywords_start = text
        .find(keywords_anchor)
        .ok_or_else(|| "the FAILURE_SHAPE_KEYWORDS table moved".to_string())?;
    let keywords_close = text[keywords_start..]
        .find("];")
        .map(|offset| keywords_start + offset)
        .ok_or_else(|| "the FAILURE_SHAPE_KEYWORDS close moved".to_string())?;
    let mut entry = format!("    (\n        \"{tag}\",\n        &[\n");
    for keyword in keywords {
        entry.push_str(&format!("            \"{keyword}\",\n"));
    }
    entry.push_str("        ],\n    ),\n");
    let mut out = String::new();
    out.push_str(&text[..keywords_start]);
    out.push_str("pub const FAILURE_SHAPE_KEYWORDS: [(&str, &[&str]); 7] = [");
    out.push_str(&text[keywords_anchor.len() + keywords_start..keywords_close]);
    out.push_str(&entry);
    out.push_str(&text[keywords_close..]);
    Ok(out)
}

/// File one shape proposal for a cluster into `<home>/proposals/` —
/// the improve store's own format, a source Patch that joins the tag
/// to the taxonomy, owner-applied. The proposal is a draft: the
/// class-derived word is the beat's proposal, and the owner's apply
/// is the naming gate.
pub fn file_shape_proposal(
    home: &Path,
    repo_root: &Path,
    cluster: &Cluster,
) -> Result<std::path::PathBuf, String> {
    let tag = taxonomy_word(&cluster.class);
    if tag.is_empty() {
        return Err(format!("`{}` yields no taxonomy word", cluster.class));
    }
    let keywords = taxonomy_keywords(&cluster.class);
    let content = taxonomy_patch(repo_root, &tag, &keywords)?;
    let change = Change::Patch {
        path: "src/memory.rs".to_string(),
        content,
    };
    let shas = improve::stamp_target_shas(repo_root, &change);
    let name = format!("shape-{tag}-{}", calib::today());
    let justification = format!(
        "the demand map's {}-day window carries {} bad row{} in the {} class `{}` ({:.0}% of {} row{}). Examples: {}",
        DEMAND_WINDOW_DAYS,
        cluster.bad,
        if cluster.bad == 1 { "" } else { "s" },
        cluster.family,
        cluster.class,
        cluster.rate * 100.0,
        cluster.rows,
        if cluster.rows == 1 { "" } else { "s" },
        if cluster.examples.is_empty() {
            "(no examples recorded)".to_string()
        } else {
            cluster.examples.join(" | ")
        }
    );
    let acceptance = format!(
        "the tag `{tag}` is joined to FAILURE_SHAPES and FAILURE_SHAPE_KEYWORDS in src/memory.rs; the writer accepts `{tag}` as a failure shape; the gate is green"
    );
    let effect = format!(
        "facts carrying `{tag}` become recallable through the failure-shape partition; the mode the cluster names stops living only in the ledger"
    );
    let change_json = serde_json::json!({
        "op": "patch",
        "path": "src/memory.rs",
        "content": match &change {
            Change::Patch { content, .. } => content.clone(),
            _ => unreachable!("a patch change"),
        },
    })
    .to_string();
    let text = improve::render_proposal(
        &name,
        Target::Source,
        "src/memory.rs",
        Basis::ConfirmedBy,
        &justification,
        &acceptance,
        &effect,
        &change_json,
        &shas,
    );
    let dir = improve::proposals_dir(home);
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let path = dir.join(format!("{name}.md"));
    std::fs::write(&path, text)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(path)
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

    fn learn_check_row(ts: &str, node: &str, confusion: &str) -> Row {
        Row::LearnCheck(calib::LearnCheckRow {
            ts: ts.to_string(),
            session: "2026-11-08-abc".to_string(),
            node: node.to_string(),
            confusion: confusion.to_string(),
            held: confusion.is_empty(),
        })
    }

    /// The learning feed (the learning plan's C8): the lesson's own
    /// rows are demand classes. A fixture lesson's repeated confusion
    /// about one proposition is a class the map reads, the clustering
    /// beat files it, and the proposal lands in the improve store.
    #[test]
    fn the_lesson_confusions_are_demand_classes_and_file_a_proposal() {
        let root = scratch("learn-demand");
        let stream = Stream::open(&root);
        // A fixture lesson: the step on `blocks` confused six times.
        for index in 0..6 {
            stream.record(learn_check_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "blocks",
                "the handler returning and the request finishing are the same",
            ));
        }
        // A clean step: held first, no confusion — a clean row.
        stream.record(learn_check_row("2026-11-07T00:00:00Z", "holds", ""));
        stream.flush();
        let demands = collect(&root, "2026-11-20", 30);
        let learn = demands
            .iter()
            .find(|demand| demand.family == "learn")
            .expect("the learning family");
        assert_eq!(learn.class, "blocks");
        assert_eq!(learn.rows, 6, "the confused step's rows");
        assert_eq!(learn.bad, 6);
        // The clean step is counted as a class with no bad rows — the
        // map's denominator, not a target.
        let clean = demands
            .iter()
            .find(|demand| demand.family == "learn" && demand.class == "holds")
            .expect("the clean step's class");
        assert_eq!(clean.bad, 0, "a held-first step is a clean row");
        assert_eq!(clean.rows, 1);
        // The clustering beat files the repeating confusion.
        let clusters = clusters(&root, "2026-11-20", 30);
        let filing = filing_clusters(&clusters);
        assert_eq!(filing.len(), 1, "only the repeating confusion files");
        assert_eq!(filing[0].family, "learn");
        assert_eq!(filing[0].class, "blocks");
        assert!(
            !filing[0].examples.is_empty(),
            "the learner's own words surface"
        );
        // The proposal lands in the improve store and parses through
        // the store's own loader.
        let home = scratch("learn-home");
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = file_shape_proposal(&home, &repo_root, filing[0]).expect("the proposal files");
        assert!(path.exists(), "the proposal is on disk");
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("the proposal's own name");
        let parsed = crate::improve::load_one(&home, name)
            .expect("the proposal parses through the store's loader");
        assert_eq!(parsed.name, name);
        assert!(
            parsed.justification.contains("learn"),
            "the justification names the family: {}",
            parsed.justification
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&home);
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

    /// The clustering beat: a repeating class surfaces its examples,
    /// the filing set is the clusters at or over the threshold, and
    /// the filed proposal carries the taxonomy patch.
    #[test]
    fn a_repeating_cluster_files_a_shape_proposal_with_the_taxonomy_patch() {
        let root = scratch("cluster");
        let stream = Stream::open(&root);
        for index in 0..6 {
            stream.record(route_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "deploy",
                "glide",
                "wrong",
            ));
        }
        // A second class with two bad rows — under the threshold.
        for index in 0..2 {
            stream.record(route_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "build",
                "drex",
                "wrong",
            ));
        }
        stream.flush();
        let clusters = clusters(&root, "2026-11-20", 30);
        let deploy = clusters
            .iter()
            .find(|cluster| cluster.class == "deploy/glide")
            .expect("the deploy cluster");
        assert_eq!(deploy.bad, 6);
        assert_eq!(deploy.rows, 6);
        assert!(!deploy.examples.is_empty(), "the examples surface");
        let filing = filing_clusters(&clusters);
        assert_eq!(filing.len(), 1, "only the repeating cluster files");
        assert_eq!(filing[0].class, "deploy/glide");
        // The proposal files with the taxonomy patch.
        let home = scratch("cluster-home");
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = file_shape_proposal(&home, &repo_root, filing[0]).expect("the proposal files");
        let text = std::fs::read_to_string(&path).expect("the proposal reads back");
        assert!(text.contains("target: source"), "{text}");
        assert!(text.contains("name: shape-deploy-glide-"), "{text}");
        // The proposal parses through the improve store's own reader, and
        // the patch it carries joins the tag to both taxonomy arrays.
        let name = format!("shape-deploy-glide-{}", calib::today());
        let parsed = crate::improve::load_one(&home, &name)
            .expect("the proposal parses through the store's loader");
        assert_eq!(parsed.name, name);
        let content = match &parsed.change {
            crate::improve::Change::Patch { content, .. } => content,
            other => panic!("a patch change was filed, not {other:?}"),
        };
        assert!(
            content.contains("pub const FAILURE_SHAPES: [&str; 7] = ["),
            "the array count is bumped"
        );
        assert!(
            content.contains(
                "\"deploy-glide\",
"
            ),
            "the tag is joined"
        );
        assert!(
            content.contains("pub const FAILURE_SHAPE_KEYWORDS: [(&str, &[&str]); 7] = ["),
            "the keyword table is bumped"
        );
        assert!(
            content.contains(
                "\"deploy-glide\",
        &[
            \"deploy\",
            \"glide\",
"
            ),
            "the keywords ride"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The cluster view renders the classes, the marks, and the
    /// threshold count.
    #[test]
    fn the_cluster_view_names_the_classes_and_the_filing_set() {
        let root = scratch("cluster-render");
        let stream = Stream::open(&root);
        for index in 0..5 {
            stream.record(route_row(
                &format!("2026-11-0{index}T00:00:00Z"),
                "deploy",
                "glide",
                "wrong",
            ));
        }
        stream.flush();
        let clusters = clusters(&root, "2026-11-20", 30);
        let rendered = render_clusters(&clusters, 30);
        assert!(rendered.contains("deploy/glide"), "{rendered}");
        assert!(rendered.contains("[file]"), "{rendered}");
        assert!(rendered.contains("5-bad filing threshold"), "{rendered}");
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
