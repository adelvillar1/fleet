//! The stream's own anomaly pass (the usage-in-calibration plan's
//! Increment B): a deterministic pre-pass over a day's calibration
//! rows that names what the day did unusually, in the row vocabulary,
//! with the evidence and no spend. Detection never blocks, delays, or
//! fails a run — the stream is advisory by construction.
//!
//! Six classes, each with its threshold as a named `const` beside its
//! detector. Only when a deterministic class has fired may the pass
//! ask the lanes — `triage` for a severity, `explain` for the fired
//! cluster's narration, at most two asks for the whole day, and zero
//! on a dry run.

use crate::calib::{
    read_day, ReadRow, OP_DELEGATION, OP_EMBED_TIMING, OP_ROUTE, OP_TOOL_SCREEN, OP_TURN,
};
use crate::explain::{ask_lane, LaneSource as ExplainSource};
use crate::judge::{LaneSession, LaneSource, JUDGE_LANE};

/// The consecutive-route-rows threshold for `fallback-walk`: three
/// rows whose walks all end the same way is a day the roster kept
/// falling through, not a day of ordinary retries.
const FALLBACK_WALK_MIN: usize = 3;
/// The shared-(tool, input) threshold for `tool-failure-loop`.
const TOOL_LOOP_MIN: usize = 3;
/// The lane-failure threshold for `lanes-down`.
const LANE_FAILURE_MIN: usize = 2;
/// The embed path's slowness bound, in milliseconds — the
/// embed-bounds plan's measured-tail discipline: a row past this is
/// the path degrading, the signal `lanes-down` pairs with lane
/// failures.
const EMBED_SLOW_MS: u128 = 5_000;
/// The delegation threshold for `delegation-maxed`: two children in
/// one day that both ate their turn cap.
const DELEGATION_MAXED_MIN: usize = 2;
/// The turn-failure threshold for `run-failure-cluster`.
const RUN_FAILURE_MIN: usize = 3;
/// The window for `run-failure-cluster`, in minutes.
const RUN_FAILURE_WINDOW_MIN: u64 = 15;
/// The cache-hit share a provider's earlier rows must show before a
/// later zero counts as `cache-collapse` (a share at or above this,
/// then a window with none).
const CACHE_COLLAPSE_HIT_SHARE: f64 = 0.5;

/// One finding: the class, the window it covers, the rows it keys on,
/// and the evidence line. The severity and narration ride after the
/// lane's asks — `None` until then, and `None` forever on a dry run.
#[derive(Debug, Clone)]
pub struct Finding {
    pub class: &'static str,
    pub window: String,
    pub rows: usize,
    pub evidence: String,
    pub severity: Option<String>,
    pub narration: Option<String>,
}

/// The day's report: the findings, and how many lane asks the pass
/// made (the counting seam reads this).
#[derive(Debug, Clone, Default)]
pub struct AnomalyReport {
    pub findings: Vec<Finding>,
    pub lane_asks: usize,
}

/// The day's day-rows, filtered to one op, oldest first.
fn op_rows<'a>(rows: &'a [ReadRow], op: &str) -> Vec<&'a ReadRow> {
    rows.iter().filter(|row| row.op == op).collect()
}

/// `2026-11-02T14:02:00Z` → minutes since the epoch, for window math.
fn ts_minutes(ts: &str) -> Option<u64> {
    let bytes = ts.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let number = |from: usize, to: usize| -> Option<u64> { ts.get(from..to)?.parse::<u64>().ok() };
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    // Days from civil (Howard Hinnant's algorithm), then to minutes.
    let (year, month) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let era = year / 400;
    let yoe = year - era * 400;
    let doy = (153 * (month - 3) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 24 * 60 + hour * 60 + minute + second / 60)
}

/// `fallback-walk`: consecutive route rows whose walks all end in the
/// same fallback — the roster falling through on one task shape.
fn fallback_walk(rows: &[ReadRow]) -> Vec<Finding> {
    let routes = op_rows(rows, OP_ROUTE);
    let mut findings = Vec::new();
    let mut run: Vec<&ReadRow> = Vec::new();
    let flush = |run: &mut Vec<&ReadRow>, findings: &mut Vec<Finding>| {
        if run.len() >= FALLBACK_WALK_MIN {
            let first = run.first().expect("a non-empty run");
            let last = run.last().expect("a non-empty run");
            let fallback = last
                .providers_used
                .last()
                .cloned()
                .unwrap_or_else(|| last.provider.clone());
            findings.push(Finding {
                class: "fallback-walk",
                window: format!("{}–{}", first.ts, last.ts),
                rows: run.len(),
                evidence: format!(
                    "the walk: {}, every row ending `fallback`; the same `input_hash` in all {}",
                    run.iter()
                        .filter_map(|row| row.providers_used.last())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" → "),
                    run.len()
                ),
                severity: None,
                narration: None,
            });
            let _ = fallback;
        }
        run.clear();
    };
    for row in routes {
        let is_fallback = row.how == "fallback";
        if is_fallback {
            run.push(row);
        } else {
            flush(&mut run, &mut findings);
        }
    }
    flush(&mut run, &mut findings);
    findings
}

/// `tool-failure-loop`: one tool screened the same way (same input
/// hash) three times or more — a loop the run did not escape.
fn tool_failure_loop(rows: &[ReadRow]) -> Vec<Finding> {
    let screens = op_rows(rows, OP_TOOL_SCREEN);
    let mut keys: Vec<(String, String, usize, Vec<&ReadRow>)> = Vec::new();
    for row in screens {
        let key = (row.tool.clone(), row.input_hash.clone());
        if let Some(entry) = keys
            .iter_mut()
            .find(|(tool, hash, _, _)| *tool == key.0 && *hash == key.1)
        {
            entry.2 += 1;
            entry.3.push(row);
        } else {
            keys.push((key.0, key.1, 1, vec![row]));
        }
    }
    keys.into_iter()
        .filter(|(_, _, count, _)| *count >= TOOL_LOOP_MIN)
        .map(|(tool, hash, count, rows)| Finding {
            class: "tool-failure-loop",
            window: format!("{}–{}", rows[0].ts, rows[rows.len() - 1].ts),
            rows: count,
            evidence: format!("`{tool}` screened {count} times with the same input `{hash}`"),
            severity: None,
            narration: None,
        })
        .collect()
}

/// `lanes-down`: lane calls failing and the embed path degrading in
/// the same day — each degrades silently by design, and both at once
/// has no other name.
fn lanes_down(rows: &[ReadRow]) -> Vec<Finding> {
    let lane_failures = rows
        .iter()
        .filter(|row| row.op == OP_TOOL_SCREEN && row.verdict == "denied")
        .count();
    let slow_embeds = rows
        .iter()
        .filter(|row| row.op == OP_EMBED_TIMING)
        .filter(|row| row.ms.unwrap_or(0) >= EMBED_SLOW_MS)
        .count();
    if lane_failures >= LANE_FAILURE_MIN && slow_embeds > 0 {
        vec![Finding {
            class: "lanes-down",
            window: "the day".to_string(),
            rows: lane_failures + slow_embeds,
            evidence: format!(
                "{lane_failures} lane tool call{} denied \u{b7} {slow_embeds} embed timing{} past \
                 {EMBED_SLOW_MS} ms",
                if lane_failures == 1 { "" } else { "s" },
                if slow_embeds == 1 { "" } else { "s" }
            ),
            severity: None,
            narration: None,
        }]
    } else {
        Vec::new()
    }
}

/// `delegation-maxed`: two or more children in one day that both ate
/// their turn cap — the delegation shape is not working that day.
fn delegation_maxed(rows: &[ReadRow]) -> Vec<Finding> {
    let maxed = op_rows(rows, OP_DELEGATION)
        .into_iter()
        .filter(|row| row.outcome.contains("max-turns") || row.reason.contains("max-turns"))
        .collect::<Vec<_>>();
    if maxed.len() >= DELEGATION_MAXED_MIN {
        vec![Finding {
            class: "delegation-maxed",
            window: format!("{}–{}", maxed[0].ts, maxed[maxed.len() - 1].ts),
            rows: maxed.len(),
            evidence: format!(
                "{} delegations ended at the turn cap: {}",
                maxed.len(),
                maxed
                    .iter()
                    .map(|row| row.session.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            severity: None,
            narration: None,
        }]
    } else {
        Vec::new()
    }
}

/// `run-failure-cluster`: three or more failed turns inside the
/// window — the day's runs are failing together, not independently.
fn run_failure_cluster(rows: &[ReadRow]) -> Vec<Finding> {
    let failures: Vec<&ReadRow> = op_rows(rows, OP_TURN)
        .into_iter()
        .filter(|row| row.outcome == "failed")
        .collect();
    let mut findings = Vec::new();
    let mut start = 0;
    while start < failures.len() {
        let Some(first_minutes) = failures[start]
            .ts
            .parse::<String>()
            .ok()
            .and_then(|_| ts_minutes(&failures[start].ts))
        else {
            start += 1;
            continue;
        };
        let mut end = start;
        while end + 1 < failures.len() {
            let Some(minutes) = ts_minutes(&failures[end + 1].ts) else {
                break;
            };
            if minutes > first_minutes + RUN_FAILURE_WINDOW_MIN {
                break;
            }
            end += 1;
        }
        let count = end - start + 1;
        if count >= RUN_FAILURE_MIN {
            findings.push(Finding {
                class: "run-failure-cluster",
                window: format!("{}–{}", failures[start].ts, failures[end].ts),
                rows: count,
                evidence: format!(
                    "{count} failed turns inside {RUN_FAILURE_WINDOW_MIN} minutes, reasons: {}",
                    failures[start..=end]
                        .iter()
                        .map(|row| row.reason.clone())
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                severity: None,
                narration: None,
            });
            start = end + 1;
        } else {
            start += 1;
        }
    }
    findings
}

/// `cache-collapse`: a provider's cache-read share was healthy in the
/// day's first rows and zero in its later ones — the cache the spend
/// card counts on stopped answering.
fn cache_collapse(rows: &[ReadRow]) -> Vec<Finding> {
    let routes = op_rows(rows, OP_ROUTE);
    let with_usage: Vec<&ReadRow> = routes
        .iter()
        .copied()
        .filter(|row| row.usage.is_some())
        .collect();
    if with_usage.len() < 4 {
        return Vec::new();
    }
    let half = with_usage.len() / 2;
    let mut findings = Vec::new();
    let mut providers: Vec<String> = Vec::new();
    for row in &with_usage {
        if !providers.contains(&row.provider) {
            providers.push(row.provider.clone());
        }
    }
    for provider in providers {
        let share = |slice: &[&ReadRow]| -> Option<f64> {
            let billed: u64 = slice
                .iter()
                .filter(|row| row.provider == provider)
                .filter_map(|row| row.usage.as_ref().map(|usage| usage.billed_tokens()))
                .sum();
            if billed == 0 {
                return None;
            }
            let cache: u64 = slice
                .iter()
                .filter(|row| row.provider == provider)
                .filter_map(|row| row.usage.as_ref().and_then(|usage| usage.cache_read_tokens))
                .sum();
            Some(cache as f64 / billed as f64)
        };
        let early = share(&with_usage[..half]);
        let late = share(&with_usage[half..]);
        if let (Some(early), Some(late)) = (early, late) {
            if early >= CACHE_COLLAPSE_HIT_SHARE && late == 0.0 {
                findings.push(Finding {
                    class: "cache-collapse",
                    window: format!(
                        "{}–{}",
                        with_usage[half].ts,
                        with_usage[with_usage.len() - 1].ts
                    ),
                    rows: with_usage.len() - half,
                    evidence: format!(
                        "{provider}: cache-read share {early:.0}% in the day's first rows, zero \
                         across the later ones"
                    ),
                    severity: None,
                    narration: None,
                });
            }
        }
    }
    findings
}

/// The deterministic pass: every detector over the day's rows. No lane
/// ask, no spend — the thresholds are the contract.
pub fn detect(rows: &[ReadRow]) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(fallback_walk(rows));
    findings.extend(tool_failure_loop(rows));
    findings.extend(lanes_down(rows));
    findings.extend(delegation_maxed(rows));
    findings.extend(run_failure_cluster(rows));
    findings.extend(cache_collapse(rows));
    findings
}

/// The full pass: the deterministic detection, then — only when a
/// class fired and this is not a dry run — at most two lane asks:
/// `triage` for the severity, `explain` for the narration. A dry run
/// returns the findings with both asks unmade (the judge's rehearse
/// shape).
pub async fn pass(
    root: &std::path::Path,
    day: &str,
    source: &LaneSource,
    explain_source: &ExplainSource,
    dry_run: bool,
) -> AnomalyReport {
    // The day's tail — the stream's own read bound; a day whose
    // shape lives beyond it is a follow-up, not a silent miss.
    let rows = read_day(root, day, crate::calib::READ_CAP);
    let mut findings = detect(&rows);
    let mut lane_asks = 0;
    if !findings.is_empty() && !dry_run {
        fn summary(findings: &[Finding]) -> String {
            findings
                .iter()
                .map(|finding| format!("{}: {}", finding.class, finding.evidence))
                .collect::<Vec<_>>()
                .join("\n")
        }
        // One ask for the cluster's severity (the judge's sys1 lane,
        // its own session seam).
        let triage = LaneSession::open(source, JUDGE_LANE, "sys1_classify", "anomaly triage").await;
        if let Ok(mut session) = triage {
            if let Ok(answer) = session
                .ask(&format!(
                    "grade the severity of these calibration findings (low/medium/high) with a \
                     one-line reason:\n{}",
                    summary(&findings)
                ))
                .await
            {
                lane_asks += 1;
                let severity = answer
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .or_else(|| answer.get("verdict").and_then(|value| value.as_str()))
                    .unwrap_or("the lane answered without a severity")
                    .trim()
                    .to_string();
                for finding in &mut findings {
                    finding.severity = Some(severity.clone());
                }
            }
            session.close().await;
        }
        // One ask for the narration (the explain lane).
        if let Ok(reply) = ask_lane(
            explain_source,
            &format!(
                "narrate these calibration findings for the owner, in one paragraph:\n{}",
                summary(&findings)
            ),
        )
        .await
        {
            lane_asks += 1;
            for finding in &mut findings {
                finding.narration = Some(reply.final_answer.clone());
            }
        }
    }
    AnomalyReport {
        findings,
        lane_asks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// One ReadRow from a minimal JSON line — the reader's own parse
    /// path fills every defaulted field, exactly as the stream does.
    fn row(json: &str) -> ReadRow {
        serde_json::from_str(json).expect("a row parses")
    }

    fn day_file(root: &std::path::Path, day: &str, rows: &[&str]) {
        let path = crate::calib::day_path(root, day);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, rows.join("\n") + "\n").unwrap();
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("fleet-anomalies-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn route(provider: &str, how: &str, hash: &str) -> String {
        format!(
            r#"{{"ts":"2026-11-02T12:00:00Z","op":"route","provider":"{provider}","model":"m","how":"{how}","reason":"t","providers_used":["{provider}"],"attempts":[{{"provider":"{provider}","outcome":"served","class":null}}],"input_chars":4,"input_hash":"{hash}","latency_ms":10,"judged_from":"fresh"}}"#
        )
    }

    fn screen(tool: &str, hash: &str, verdict: &str) -> String {
        format!(
            r#"{{"ts":"2026-11-02T12:00:00Z","op":"tool_screen","tool":"{tool}","verdict":"{verdict}","input_chars":4,"input_hash":"{hash}","tier":"standard"}}"#
        )
    }

    fn turn(outcome: &str, ts: &str, reason: &str) -> String {
        format!(
            r#"{{"ts":"{ts}","op":"turn","outcome":"{outcome}","stop_reason":"stop","turns":2,"usage":{{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15,"finish_reason":"stop"}},"input_chars":4,"input_hash":"h","reason":"{reason}"}}"#
        )
    }

    #[test]
    fn fallback_walk_fires_on_three_and_silents_on_two() {
        let rows = vec![
            row(&route("a", "fallback", "same")),
            row(&route("b", "fallback", "same")),
            row(&route("c", "fallback", "same")),
        ];
        assert_eq!(fallback_walk(&rows).len(), 1);
        let two = vec![
            row(&route("a", "fallback", "same")),
            row(&route("b", "fallback", "same")),
        ];
        assert!(fallback_walk(&two).is_empty());
    }

    #[test]
    fn tool_loop_fires_on_three_shared_inputs() {
        let rows = vec![
            row(&screen("bash", "same", "denied")),
            row(&screen("bash", "same", "denied")),
            row(&screen("bash", "same", "denied")),
        ];
        assert_eq!(tool_failure_loop(&rows).len(), 1);
        let two = vec![
            row(&screen("bash", "same", "denied")),
            row(&screen("bash", "same", "denied")),
        ];
        assert!(tool_failure_loop(&two).is_empty());
    }

    #[test]
    fn lanes_down_needs_both_signals() {
        let rows = vec![
            row(&screen("mcp__sys1__classify", "h1", "denied")),
            row(&screen("mcp__blend1__ask", "h2", "denied")),
            row(
                r#"{"ts":"2026-11-02T12:00:00Z","op":"embed_timing","kind":"embed","ms":9000,"facts":4}"#,
            ),
        ];
        assert_eq!(lanes_down(&rows).len(), 1);
        let lanes_only = vec![
            row(&screen("mcp__sys1__classify", "h1", "denied")),
            row(&screen("mcp__blend1__ask", "h2", "denied")),
        ];
        assert!(lanes_down(&lanes_only).is_empty());
    }

    #[test]
    fn delegation_maxed_fires_on_two_capped_children() {
        let rows = vec![
            row(
                r#"{"ts":"2026-11-02T12:00:00Z","op":"delegation","session":"child-1","outcome":"max-turns","reason":"the turn cap"}"#,
            ),
            row(
                r#"{"ts":"2026-11-02T12:01:00Z","op":"delegation","session":"child-2","outcome":"max-turns","reason":"the turn cap"}"#,
            ),
        ];
        assert_eq!(delegation_maxed(&rows).len(), 1);
        let one = vec![row(
            r#"{"ts":"2026-11-02T12:00:00Z","op":"delegation","session":"child-1","outcome":"max-turns","reason":"the turn cap"}"#,
        )];
        assert!(delegation_maxed(&one).is_empty());
    }

    #[test]
    fn run_failure_cluster_fires_inside_the_window() {
        let rows = vec![
            row(&turn("failed", "2026-11-02T12:00:00Z", "api")),
            row(&turn("failed", "2026-11-02T12:05:00Z", "api")),
            row(&turn("failed", "2026-11-02T12:10:00Z", "api")),
        ];
        assert_eq!(run_failure_cluster(&rows).len(), 1);
        let spread = vec![
            row(&turn("failed", "2026-11-02T12:00:00Z", "api")),
            row(&turn("failed", "2026-11-02T12:20:00Z", "api")),
            row(&turn("failed", "2026-11-02T12:40:00Z", "api")),
        ];
        assert!(run_failure_cluster(&spread).is_empty());
    }

    #[test]
    fn cache_collapse_fires_on_a_zero_second_half() {
        let with_cache = r#"{"ts":"2026-11-02T12:00:00Z","op":"route","provider":"p","model":"m","how":"judged","reason":"t","providers_used":["p"],"attempts":[{"provider":"p","outcome":"served","class":null}],"input_chars":4,"input_hash":"h","latency_ms":10,"judged_from":"fresh","usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"cache_read_input_tokens":80,"finish_reason":"stop"}}"#;
        let no_cache = r#"{"ts":"2026-11-02T12:01:00Z","op":"route","provider":"p","model":"m","how":"judged","reason":"t","providers_used":["p"],"attempts":[{"provider":"p","outcome":"served","class":null}],"input_chars":4,"input_hash":"h","latency_ms":10,"judged_from":"fresh","usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"finish_reason":"stop"}}"#;
        let rows = vec![
            row(with_cache),
            row(with_cache),
            row(no_cache),
            row(no_cache),
        ];
        assert_eq!(cache_collapse(&rows).len(), 1);
        let always = vec![
            row(with_cache),
            row(with_cache),
            row(with_cache),
            row(with_cache),
        ];
        assert!(cache_collapse(&always).is_empty());
    }

    #[test]
    fn a_quiet_day_finds_nothing() {
        let rows = vec![
            row(&route("a", "judged", "one")),
            row(&turn("ok", "2026-11-02T12:00:00Z", "")),
            row(&screen("bash", "different", "approved")),
        ];
        assert!(detect(&rows).is_empty());
    }

    #[tokio::test]
    async fn the_pass_asks_at_most_twice_and_never_on_a_dry_run() {
        let root = scratch("pass");
        day_file(
            &root,
            "2026-11-02",
            &[
                &route("a", "fallback", "same"),
                &route("b", "fallback", "same"),
                &route("c", "fallback", "same"),
            ],
        );
        let judge_log = Arc::new(Mutex::new(crate::lanes::CountingLog {
            events: Vec::new(),
            fail_open: false,
        }));
        let explain_log = Arc::new(Mutex::new(crate::lanes::CountingLog {
            events: Vec::new(),
            fail_open: false,
        }));
        let report = pass(
            &root,
            "2026-11-02",
            &LaneSource::Counting(Arc::clone(&judge_log)),
            &ExplainSource::Counting(Arc::clone(&explain_log)),
            false,
        )
        .await;
        assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
        assert_eq!(report.lane_asks, 2, "one triage ask and one explain ask");
        assert!(report.findings[0].severity.is_some());
        assert!(report.findings[0].narration.is_some());
        let dry = pass(
            &root,
            "2026-11-02",
            &LaneSource::Counting(Arc::clone(&judge_log)),
            &ExplainSource::Counting(Arc::clone(&explain_log)),
            true,
        )
        .await;
        assert_eq!(dry.lane_asks, 0, "a dry run spends nothing");
        assert!(dry.findings[0].severity.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_surface_renders_a_quiet_day_and_a_finding() {
        let quiet = crate::calib::render_anomalies("2026-11-02", &AnomalyReport::default());
        assert!(quiet.contains("the day is quiet"), "{quiet}");
        let report = AnomalyReport {
            findings: vec![Finding {
                class: "fallback-walk",
                window: "12:00–12:10".to_string(),
                rows: 3,
                evidence: "the walk: a → b → c, every row ending `fallback`".to_string(),
                severity: Some("high".to_string()),
                narration: Some("the roster kept falling through".to_string()),
            }],
            lane_asks: 2,
        };
        let rendered = crate::calib::render_anomalies("2026-11-02", &report);
        assert!(rendered.contains("fallback-walk"), "{rendered}");
        assert!(rendered.contains("severity:"), "{rendered}");
        assert!(rendered.contains("high"), "{rendered}");
        assert!(
            rendered.contains("the roster kept falling through"),
            "{rendered}"
        );
        assert!(rendered.contains("2 lane asks"), "{rendered}");
    }
}
