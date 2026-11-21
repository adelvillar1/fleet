//! The learning seam: the owner's confirmed judgments teach Laya's Learner.
//!
//! Slice 5 records every routing decision, the judge beat grades them
//! ([`crate::judge`]), and the owner's disposition — `agreed`, `disagreed`,
//! `waived` — rides the same `route_judged` row as ground truth. This module
//! turns the CONFIRMED ones into training rows for the laya provider's own
//! continuous learner, so a decision the owner agreed with today makes the
//! same decision cheaper to get right tomorrow.
//!
//! ## The discipline
//!
//! - **batch-only.** No synchronous path references this module: the routing
//!   walk, the tool loop, the recall path and the judge itself never name it.
//!   The model load is the expensive part (measured 4.3 GiB peak, ~4 s), so
//!   this is a day's rows applied at once, the shape the vendor's own guide
//!   describes — never a call per decision.
//! - **the text never leaves the stream.** The rows are redacted by design
//!   and carry no task text; the state the learner is taught is
//!   [`crate::judge::evidence_record`], the SAME redacted record the judge was
//!   asked about. Nothing here re-renders a task prompt or stores a text.
//! - **`agreed` only.** An `agreed` row confirms the lane's own label, which
//!   is a gold answer. A `disagreed` row says the label was wrong without
//!   saying what was right — a learner cannot be taught "not this" — and a
//!   `waived` row says nothing at all. Both are counted and named, never sent.
//! - **a batch with nothing to teach is refused by name**, not sent as an
//!   empty list: an empty run that reports success is how a broken feed looks
//!   healthy.
//! - **idempotent.** Every record carries its `route_key`; the applier keeps
//!   the ids it has applied and skips them, so re-running a day is a no-op.
//!   [`crate::laya::Report`] reports applied / skipped / refused apart.
//!
//! ## What it does not do
//!
//! It never enables laya, never names it in the route chain, and never writes
//! a floor: the tier stays `candidate` and the learned state only makes a
//! provider the owner already chose to measure better at what it is asked.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;
use serde_json::json;

use crate::calib::{self, ReadRow, OP_ROUTE, OP_ROUTE_JUDGED};
use crate::judge;

/// The applier, inside the sys1 lane's own service directory: it runs in the
/// LAYA venv (so `import laya` resolves) with sys1's `src` on the path (so the
/// question spec is built by the same code the wire uses).
pub const LEARN_SCRIPT: &str = "laya_learn.py";

/// How long a batch may take. The learner loads the reader (~4 s measured) and
/// then learns from the rows; the bound is generous because a batch that
/// overruns is a batch that should be retried, not killed mid-write.
pub const LEARN_TIMEOUT: Duration = Duration::from_secs(900);

/// The provider whose learner this is.
pub const PROVIDER: &str = "laya";

/// One row the batch carries: the judged route's identity, the redacted
/// evidence record it was judged on, and the owner-confirmed answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Record {
    /// The judged route row's key — the idempotence handle.
    pub id: String,
    /// The evidence record, redacted: never the task's text.
    pub state: String,
    /// `{"workload": "<label>"}` — the shape the learner's question takes.
    pub gold: BTreeMap<String, String>,
}

/// What a day's stream yields: the teachable records and the counts of what
/// could not be taught, each named.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Batch {
    pub records: Vec<Record>,
    /// Judged rows carrying `agreed`.
    pub agreed: usize,
    /// Judged rows carrying `disagreed` — an inversion with no gold label.
    pub disagreed: usize,
    /// Judged rows carrying `waived` — set aside, nothing to learn.
    pub waived: usize,
    /// Judged rows with no disposition at all: the lane's own verdict stands
    /// and the owner never graded it, so there is nothing to confirm.
    pub ungraded: usize,
    /// Agreed rows that could not become a record, with the reason.
    pub unteachable: Vec<String>,
    /// Why the batch is not sendable, when it is not.
    pub refusal: Option<String>,
}

impl Batch {
    /// Whether there is anything to send.
    pub fn is_sendable(&self) -> bool {
        self.refusal.is_none() && !self.records.is_empty()
    }
}

/// Read a day's rows into the batch this seam would send.
///
/// Pure over the rows the caller already read: the join is the route row's own
/// identity ([`calib::route_key`]), the state is
/// [`judge::evidence_record`], and the gold label is the workload the judged
/// row carries — which, for an `agreed` row, is exactly the answer the owner
/// confirmed.
pub fn batch(rows: &[ReadRow]) -> Batch {
    let mut out = Batch::default();
    // The judged rows, freshest per key: a disposition is a LATER row for the
    // same key, so the freshest one is the one carrying the owner's word.
    let judged = calib::judged_rows(rows);
    // The route rows by their own identity, for the state and the turn join.
    let mut routes: BTreeMap<String, &ReadRow> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.op == OP_ROUTE) {
        routes.insert(calib::route_key(row), row);
    }

    let mut graded = 0usize;
    for row in judged.iter().filter(|row| row.op == OP_ROUTE_JUDGED) {
        match row.disposition.as_str() {
            calib::DISPOSITION_AGREED => {
                graded += 1;
                out.agreed += 1;
            }
            calib::DISPOSITION_DISAGREED => {
                graded += 1;
                out.disagreed += 1;
            }
            calib::DISPOSITION_WAIVED => {
                graded += 1;
                out.waived += 1;
            }
            _ => {
                out.ungraded += 1;
                continue;
            }
        }
        if row.disposition != calib::DISPOSITION_AGREED {
            continue;
        }
        let Some(route) = routes.get(&row.route_key) else {
            out.unteachable.push(format!(
                "{}: agreed, but no `route` row carries that key — the evidence record \
                 cannot be re-rendered, and a learner taught on nothing learns nothing",
                row.route_key
            ));
            continue;
        };
        let label = row.workload.trim();
        if label.is_empty() {
            out.unteachable.push(format!(
                "{}: agreed, but the judged row carries no workload label to teach",
                row.route_key
            ));
            continue;
        }
        let turn = judge::joined_turn_outcome(rows, route);
        let state = judge::evidence_record(route, turn);
        if state.trim().is_empty() {
            out.unteachable.push(format!(
                "{}: agreed, but its evidence record renders empty",
                row.route_key
            ));
            continue;
        }
        let mut gold = BTreeMap::new();
        gold.insert("workload".to_string(), label.to_string());
        out.records.push(Record {
            id: row.route_key.clone(),
            state,
            gold,
        });
    }

    out.refusal = if graded == 0 {
        Some(
            "no `route_judged` row in this day carries a disposition — \
             `fleet judge --agree/--disagree/--waive <key>` writes one, and only a \
             confirmed answer can teach"
                .to_string(),
        )
    } else if out.records.is_empty() {
        Some(format!(
            "{} disposition(s) on this day, none of them `agreed` ({} disagreed, {} waived) — \
             a disagreement names no gold answer, so there is nothing to learn",
            graded, out.disagreed, out.waived
        ))
    } else {
        None
    };
    out
}

/// What the applier reported: the counts, apart, plus the learner's own word.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub applied: usize,
    pub skipped: usize,
    pub refused: usize,
    pub learned: bool,
    pub workdir: String,
    /// The learner's stored rows after the batch, when it reported them.
    pub rows: Option<u64>,
}

/// The learner's state directory: `<home>/laya-learn`.
pub fn workdir(home: &Path) -> PathBuf {
    home.join("laya-learn")
}

/// The laya venv's python, when the install built it.
///
/// Resolved through the installer's own registry rather than by parsing
/// sys1's config: the press built `<home>/venvs/laya`, and that is the path
/// sys1's `providers.laya_venv` defaults to.
pub fn venv_python(home: &Path) -> Option<PathBuf> {
    let provider = crate::install::provider_venv(PROVIDER)?;
    let python = crate::install::venv_dir(home, provider)
        .join("bin")
        .join("python");
    python.exists().then_some(python)
}

/// Apply `records` to the learner in-process, through the sys1 lane's service
/// script. Batch-only: one spawn, one batch, one report.
///
/// The script runs in the LAYA venv (it is the interpreter that can
/// `import laya`) with `SYS1_PYTHONPATH` pointing at the lane's `src` (so the
/// question spec comes from `providers.laya_questions`, the same builder the
/// wire uses). Both paths are the lane's own, resolved the way the launch
/// spec resolves them — never from the caller's environment.
pub fn apply(home: &Path, sys1_home: &Path, records: &[Record]) -> Result<Report, String> {
    let python = venv_python(home).ok_or_else(|| {
        format!(
            "the laya venv is not built at {} — `fleet install --provider laya` builds it",
            crate::install::venv_dir(
                home,
                crate::install::provider_venv(PROVIDER).expect("registered")
            )
            .display()
        )
    })?;
    let script = sys1_home.join("service").join(LEARN_SCRIPT);
    if !script.is_file() {
        return Err(format!(
            "the applier is missing at {} — the sys1 lane's copy is what runs it",
            script.display()
        ));
    }
    let payload = json!({ "records": records }).to_string();
    let mut child = Command::new(&python)
        .arg("-u")
        .arg(&script)
        .arg("--workdir")
        .arg(workdir(home))
        .env("SYS1_PYTHONPATH", sys1_home.join("src"))
        // The lane's own base environment is not inherited wholesale (KI-031);
        // the applier needs no ambient name but its own two.
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("the applier did not start ({}): {error}", python.display()))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(payload.as_bytes())
            .map_err(|error| format!("the batch could not be handed to the applier: {error}"))?;
    }
    // Closing stdin is the applier's EOF: it reads one document and answers.
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("the applier could not be waited on: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let named = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no output");
        return Err(format!("the applier refused the batch: {named}"));
    }
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|_| format!("the applier answered something that is not a report: {stdout}"))?;
    Ok(Report {
        applied: value.get("applied").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        skipped: value.get("skipped").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        refused: value.get("refused").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
        learned: value
            .get("learned")
            .and_then(|flag| flag.as_bool())
            .unwrap_or(false),
        workdir: value
            .get("workdir")
            .and_then(|path| path.as_str())
            .unwrap_or_default()
            .to_string(),
        rows: value.get("rows").and_then(|n| n.as_u64()),
    })
}

/// The card `fleet judge --apply-laya` prints: what the day held, what was
/// sent, and what the learner did with it.
///
/// The house frame — `┌─┐ │ └─┘`, the ANSI palette, `cards::CARD_WIDTH` —
/// built from `cards`' primitives rather than a second copy of the frame
/// (KI-140: body lines wrap, they never truncate).
pub fn render(day: &str, batch: &Batch, report: Option<&Report>, refusal: Option<&str>) -> String {
    const ACCENT: &str = "\u{1b}[36m";
    const DIM: &str = "\u{1b}[90m";
    const TEXT: &str = "\u{1b}[37m";
    const BOLD: &str = "\u{1b}[1m";
    const RESET: &str = "\u{1b}[0m";
    let width = crate::cards::CARD_WIDTH;
    let title = crate::cards::clip(&format!("laya learner — {day}"), width / 2);
    let right = "the day's confirmed judgments";
    let used = title.chars().count() + right.chars().count() + 2;
    let dashes = crate::cards::rule(width.saturating_sub(used + 5).max(1));
    let mut out = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{title}{RESET} {DIM}{dashes} {TEXT}{right}{RESET} {DIM}\u{2510}{RESET}\n"
    );
    let mut body: Vec<String> = vec![
        format!(
            "judged rows: {} agreed · {} disagreed · {} waived · {} ungraded",
            batch.agreed, batch.disagreed, batch.waived, batch.ungraded
        ),
        format!("teachable records: {}", batch.records.len()),
    ];
    for said in &batch.unteachable {
        body.push(format!("not teachable: {said}"));
    }
    match (refusal, report) {
        (Some(said), _) => body.push(format!("refused: {said}")),
        (None, Some(report)) => {
            body.push(format!(
                "applied {} · skipped {} (already learned) · refused {}",
                report.applied, report.skipped, report.refused
            ));
            body.push(format!(
                "learned: {} · state: {}{}",
                if report.learned { "yes" } else { "no" },
                report.workdir,
                report
                    .rows
                    .map(|rows| format!(" · {rows} rows"))
                    .unwrap_or_default()
            ));
        }
        (None, None) => body.push("nothing was sent".to_string()),
    }
    for line in body {
        for piece in crate::cards::fit(&line, width - 4) {
            out.push_str(&format!("{DIM}\u{2502}{RESET} {piece}\n"));
        }
    }
    out.push_str(&format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}\n",
        crate::cards::rule(width - 2)
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calib::ReadAttempt;

    fn route_row(ts: &str, workload: &str, reason: &str) -> ReadRow {
        ReadRow {
            ts: ts.to_string(),
            op: OP_ROUTE.to_string(),
            provider: "glide".to_string(),
            workload: workload.to_string(),
            how: "judged".to_string(),
            verdict: "pass".to_string(),
            reason: reason.to_string(),
            model: "glide".to_string(),
            providers_used: vec!["glide".to_string()],
            attempts: vec![ReadAttempt {
                provider: "glide".to_string(),
                outcome: "served".to_string(),
                class: None,
            }],
            input_chars: 412,
            input_hash: "8f2c1a9b4d5e6f70".to_string(),
            latency_ms: 812,
            ..ReadRow::default()
        }
    }

    fn judged_row(route: &ReadRow, disposition: Option<&str>) -> ReadRow {
        ReadRow {
            ts: "2026-11-21T10:00:00Z".to_string(),
            op: OP_ROUTE_JUDGED.to_string(),
            route_key: calib::route_key(route),
            provider: route.provider.clone(),
            workload: route.workload.clone(),
            verdict: calib::VERDICT_RIGHT.to_string(),
            confidence: 0.8,
            reason: "the lane judged it right".to_string(),
            disposition: disposition.unwrap_or_default().to_string(),
            lane: "sys1".to_string(),
            ..ReadRow::default()
        }
    }

    /// The whole seam, pure: an `agreed` row becomes one record whose state is
    /// the evidence record the judge read and whose gold is the confirmed
    /// label; the other dispositions teach nothing and are counted apart.
    #[test]
    fn an_agreed_row_becomes_a_record_and_the_rest_are_counted_apart() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let other = route_row("2026-11-21T09:05:00Z", "hard", "a cross-cutting change");
        let rows = vec![
            route.clone(),
            other.clone(),
            judged_row(&route, Some("agreed")),
            judged_row(&other, Some("disagreed")),
        ];
        let batch = batch(&rows);
        assert_eq!(batch.records.len(), 1, "{batch:?}");
        assert_eq!(batch.agreed, 1);
        assert_eq!(batch.disagreed, 1);
        assert_eq!(batch.waived, 0);
        assert!(batch.is_sendable());
        let record = &batch.records[0];
        assert_eq!(record.id, calib::route_key(&route));
        assert_eq!(
            record.gold.get("workload").map(String::as_str),
            Some("quick")
        );
        // The state is the judge's own redacted record — never the task text,
        // and it carries the fields the router recorded.
        assert!(record
            .state
            .contains("the workload shape the router judged: quick"));
        assert!(record.state.contains("input hash: 8f2c1a9b4d5e6f70"));
        assert_eq!(
            record.state,
            judge::evidence_record(&route, None),
            "the learner reads exactly the record the judge read"
        );
    }

    /// A day with no dispositions is refused by name — an empty batch that
    /// reports success is how a broken feed looks healthy.
    #[test]
    fn a_day_with_no_dispositions_is_refused_by_name() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let batch = batch(&[route.clone(), judged_row(&route, None)]);
        assert!(!batch.is_sendable());
        assert!(batch.records.is_empty());
        assert_eq!(batch.ungraded, 1);
        let refusal = batch.refusal.expect("a named refusal");
        assert!(refusal.contains("no `route_judged` row"), "{refusal}");
        assert!(refusal.contains("--agree"), "{refusal}");
    }

    /// Dispositions that are all negative are refused too: a disagreement
    /// names no gold answer, and a learner cannot be taught "not this".
    #[test]
    fn a_day_of_only_negative_dispositions_names_what_it_cannot_teach() {
        let wrong = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let aside = route_row("2026-11-21T09:05:00Z", "hard", "a cross-cutting change");
        let batch = batch(&[
            wrong.clone(),
            aside.clone(),
            judged_row(&wrong, Some("disagreed")),
            judged_row(&aside, Some("waived")),
        ]);
        assert!(!batch.is_sendable());
        let refusal = batch.refusal.expect("a named refusal");
        assert!(refusal.contains("none of them `agreed`"), "{refusal}");
        assert!(refusal.contains("1 disagreed"), "{refusal}");
        assert!(refusal.contains("1 waived"), "{refusal}");
    }

    /// An agreed row whose route row is missing cannot be taught: the evidence
    /// record cannot be re-rendered, and the card says which key.
    #[test]
    fn an_agreed_row_with_no_route_row_is_named_not_guessed() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let orphan = judged_row(&route, Some("agreed"));
        let batch = batch(&[orphan]);
        assert!(batch.records.is_empty());
        assert_eq!(batch.unteachable.len(), 1);
        assert!(batch.unteachable[0].contains("no `route` row carries that key"));
    }

    /// The freshest judged row per key is the one that counts: a disposition
    /// appended after the judgment replaces it, exactly as the stream reads.
    #[test]
    fn the_freshest_disposition_per_key_decides() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let mut later = judged_row(&route, Some("agreed"));
        later.ts = "2026-11-21T11:00:00Z".to_string();
        let batch = batch(&[route.clone(), judged_row(&route, None), later]);
        assert_eq!(batch.records.len(), 1);
        assert_eq!(batch.agreed, 1);
        assert_eq!(batch.ungraded, 0);
    }

    /// The seam is batch-only: the synchronous paths — the routing walk, the
    /// tool loop, the session log, the calibration stream — never name it. The
    /// same socket guard the judgment beat carries, in the census's own set.
    #[test]
    fn no_synchronous_path_references_the_learning_seam() {
        // The modules that may name it: this one, the CLI's two dispatchers,
        // and the judgment beat (whose evidence renderer it borrows).
        const DRIVERS: [&str; 4] = ["laya.rs", "main.rs", "cli.rs", "judge.rs"];
        for &(name, source) in crate::testcensus::MODULES {
            if DRIVERS.contains(&name) {
                continue;
            }
            assert!(
                !source.contains("crate::laya") && !source.contains("laya::batch"),
                "{name} references the learning seam, which is batch-only"
            );
        }
    }

    /// The provider stays a candidate: this seam teaches a model the owner
    /// already chose to measure, and never promotes it or edits the chain.
    #[test]
    fn the_seam_never_touches_the_chain() {
        let source = include_str!("laya.rs");
        let body = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            !body.contains("default_chain"),
            "the chain is not this module's"
        );
        assert!(
            !body.contains("providers.enabled"),
            "enabling is not this module's"
        );
    }

    /// The record's id is the route key, so the applier's ledger and
    /// `fleet judge --agree <key>` address the same row.
    #[test]
    fn the_record_id_is_the_key_the_owner_grades_with() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let key = calib::route_key(&route);
        let batch = batch(&[route.clone(), judged_row(&route, Some("agreed"))]);
        assert_eq!(batch.records[0].id, key);
        assert_eq!(key.len(), 16, "the 16-hex key the CLI prints");
    }

    /// The gold carries the workload alone: one question, one confirmed
    /// answer. The execution head is the router's business, not the learner's.
    #[test]
    fn the_gold_carries_the_workload_alone() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let batch = batch(&[route.clone(), judged_row(&route, Some("agreed"))]);
        let gold = &batch.records[0].gold;
        assert_eq!(gold.len(), 1);
        assert!(gold.contains_key("workload"));
    }

    /// The card names every count and the refusal, and never a raw record.
    #[test]
    fn the_card_names_the_counts_and_the_refusal() {
        let route = route_row("2026-11-21T09:00:00Z", "quick", "a one-line answer");
        let batch = batch(&[route.clone(), judged_row(&route, Some("agreed"))]);
        let said = render("2026-11-21", &batch, None, batch.refusal.as_deref());
        assert!(said.contains("1 agreed"), "{said}");
        assert!(said.contains("teachable records: 1"), "{said}");
        let report = Report {
            applied: 1,
            skipped: 0,
            refused: 0,
            learned: true,
            workdir: "/tmp/x".to_string(),
            rows: Some(2),
        };
        let after = render("2026-11-21", &batch, Some(&report), None);
        assert!(after.contains("applied 1"), "{after}");
        assert!(after.contains("learned: yes"), "{after}");
        let refused = render("2026-11-21", &batch, None, Some("no dispositions"));
        assert!(refused.contains("refused: no dispositions"), "{refused}");
    }
}
