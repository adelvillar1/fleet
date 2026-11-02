//! The tool audit: one session's tool calls measured, not described.
//!
//! The numbers are read from the log alone — the calls the model made,
//! the results it read, and the resolution rows written beside them. No
//! model, no lane, no network: the same log that answers a replay
//! answers an audit.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::session::Record;

/// One session's tool numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Audit {
    pub calls: usize,
    /// Results whose text begins `error` or `denied`.
    pub errors: usize,
    /// Counts keyed by class: `validation`, `denied`, `error`, …
    pub classes: BTreeMap<String, usize>,
    /// Results the cap cut.
    pub truncated: usize,
    /// The 95th-percentile result size, in bytes.
    pub p95_bytes: usize,
    /// Repeated (name, arguments) calls within the session.
    pub redundant: usize,
    /// Of the calls that failed, the share followed by a later success
    /// of the same tool. `None` when nothing failed.
    pub recovery: Option<f64>,
    /// Resolutions by method.
    pub methods: BTreeMap<String, usize>,
    /// Resolutions by tier.
    pub tiers: BTreeMap<String, usize>,
    /// Calls by argument source: `model` when the call's own keys
    /// supplied at least half the arguments, `default` when the schema
    /// defaults carried it. C1's grounding, read as one number.
    pub argument_source: BTreeMap<String, usize>,
}

impl Audit {
    /// The share of calls that carried no validation error.
    pub fn valid_share(&self) -> Option<f64> {
        if self.calls == 0 {
            return None;
        }
        let invalid = self.classes.get("validation").copied().unwrap_or(0);
        Some((self.calls - invalid) as f64 / self.calls as f64)
    }
}

/// Read one session log.
pub fn audit(path: &Path) -> Result<Audit, String> {
    let records = crate::session::replay(path).map_err(|error| error.to_string())?;
    let mut audit = Audit::default();
    let mut results: HashMap<String, (String, bool, usize)> = HashMap::new();
    for record in &records {
        match record {
            Record::Assistant { tool_calls, .. } => audit.calls += tool_calls.len(),
            Record::ToolResult {
                call_id,
                name,
                output,
                truncated,
                ..
            } => {
                let failed = output.starts_with("error") || output.starts_with("denied");
                if failed {
                    audit.errors += 1;
                    *audit
                        .classes
                        .entry(class_of(output).to_string())
                        .or_insert(0) += 1;
                }
                if *truncated {
                    audit.truncated += 1;
                }
                results.insert(call_id.clone(), (name.clone(), failed, output.len()));
            }
            Record::ToolResolution {
                method,
                tier,
                argument_sources,
                ..
            } => {
                *audit.methods.entry(method.clone()).or_insert(0) += 1;
                *audit.tiers.entry(tier.clone()).or_insert(0) += 1;
                // C1: where the arguments came from — the model's own
                // keys against the schema defaults that filled the rest.
                // A grounded session reads high on `model`; a session
                // leaning on defaults says so.
                let model_supplied = argument_sources
                    .values()
                    .filter(|source| source.as_str() == "model")
                    .count();
                let total = argument_sources.len();
                if total > 0 {
                    *audit
                        .argument_source
                        .entry(if model_supplied * 2 >= total {
                            "model".to_string()
                        } else {
                            "default".to_string()
                        })
                        .or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }
    let mut sizes: Vec<usize> = results.values().map(|(_, _, bytes)| *bytes).collect();
    if !sizes.is_empty() {
        sizes.sort_unstable();
        let index = ((sizes.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sizes.len() - 1);
        audit.p95_bytes = sizes[index];
    }
    // Recovery: for each failed result, a later result of the same name
    // that did not fail.
    let mut recovered = 0usize;
    let failures: Vec<(usize, String)> = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match record {
            Record::ToolResult { name, output, .. }
                if output.starts_with("error") || output.starts_with("denied") =>
            {
                Some((index, name.clone()))
            }
            _ => None,
        })
        .collect();
    for (index, name) in &failures {
        let later = records.iter().skip(index + 1).any(|record| match record {
            Record::ToolResult {
                name: later,
                output,
                ..
            } => later == name && !output.starts_with("error") && !output.starts_with("denied"),
            _ => false,
        });
        if later {
            recovered += 1;
        }
    }
    if !failures.is_empty() {
        audit.recovery = Some(recovered as f64 / failures.len() as f64);
    }
    // Redundant: the same name and arguments twice in the session.
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for record in &records {
        if let Record::Assistant { tool_calls, .. } = record {
            for call in tool_calls {
                let count = seen
                    .entry((call.function.name.clone(), call.function.arguments.clone()))
                    .or_insert(0);
                *count += 1;
                if *count == 2 {
                    audit.redundant += 1;
                }
            }
        }
    }
    Ok(audit)
}

fn class_of(output: &str) -> &str {
    if let Some(rest) = output.strip_prefix("error[") {
        return rest.split(']').next().unwrap_or("error");
    }
    if output.starts_with("denied") {
        return "denied";
    }
    "error"
}

/// The audit as a card.
pub fn render(session: &str, audit: &Audit) -> String {
    let mut out = format!(
        "fleet tools audit \u{2014} {session}: {} call(s)\n",
        audit.calls
    );
    if audit.calls == 0 {
        out.push_str("  no tool calls in this session\n");
        return out;
    }
    let valid = audit
        .valid_share()
        .map(|share| format!("{:.0}%", share * 100.0))
        .unwrap_or_else(|| "n/a".to_string());
    // C2: a count is not a rate. Truncation and redundancy print as
    // shares of the calls — the number the owner compares across
    // sessions — with the count beside it, because a share without its
    // denominator reads as precision it does not have.
    let truncation = if audit.calls == 0 {
        "n/a".to_string()
    } else {
        format!(
            "{:.0}% ({})",
            audit.truncated as f64 * 100.0 / audit.calls as f64,
            audit.truncated
        )
    };
    let redundancy = if audit.calls == 0 {
        "n/a".to_string()
    } else {
        format!(
            "{:.0}% ({})",
            audit.redundant as f64 * 100.0 / audit.calls as f64,
            audit.redundant
        )
    };
    out.push_str(&format!(
        "  schema-valid: {valid} \u{b7} errors: {} \u{b7} truncated: {truncation}\n",
        audit.errors
    ));
    out.push_str(&format!(
        "  p95 result: {} bytes \u{b7} redundant calls: {redundancy}\n",
        audit.p95_bytes
    ));
    match audit.recovery {
        Some(share) => out.push_str(&format!(
            "  recovery after a failure: {:.0}%\n",
            share * 100.0
        )),
        None => out.push_str("  recovery after a failure: no failures\n"),
    }
    if !audit.classes.is_empty() {
        let classes: Vec<String> = audit
            .classes
            .iter()
            .map(|(class, count)| format!("{class} {count}"))
            .collect();
        out.push_str(&format!("  error classes: {}\n", classes.join(" \u{b7} ")));
    }
    if !audit.argument_source.is_empty() {
        let sources: Vec<String> = audit
            .argument_source
            .iter()
            .map(|(source, count)| format!("{source} {count}"))
            .collect();
        out.push_str(&format!(
            "  argument sources: {}\n",
            sources.join(" \u{b7} ")
        ));
    }
    let methods: Vec<String> = audit
        .methods
        .iter()
        .map(|(method, count)| format!("{method} {count}"))
        .collect();
    out.push_str(&format!(
        "  resolutions: {}\n",
        if methods.is_empty() {
            "none recorded".to_string()
        } else {
            methods.join(" \u{b7} ")
        }
    ));
    let tiers: Vec<String> = audit
        .tiers
        .iter()
        .map(|(tier, count)| format!("{tier} {count}"))
        .collect();
    if !tiers.is_empty() {
        out.push_str(&format!("  tiers: {}\n", tiers.join(" \u{b7} ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-audit-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn call(id: &str, name: &str, arguments: &str) -> crate::api::ToolCall {
        crate::api::ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: crate::api::FunctionCall {
                name: name.to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    #[test]
    fn the_audit_counts_calls_classes_recovery_and_methods() {
        let home = scratch("counts");
        let mut session = Session::create(&home, "test-model").unwrap();
        let calls = vec![
            call("c1", "read_file", r#"{"path":"a"}"#),
            call(
                "c2",
                "edit",
                r#"{"path":"a","old_string":"x","new_string":"y"}"#,
            ),
            call(
                "c3",
                "edit",
                r#"{"path":"a","old_string":"x","new_string":"y"}"#,
            ),
            call("c4", "ghost", "{}"),
        ];
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: calls,
                persona: None,
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "c1".into(),
                requested: "read_file".into(),
                resolved: "read_file".into(),
                method: "exact".into(),
                reason: "declared".into(),
                catalog: 20,
                tier: "core".into(),
                argument_sources: std::collections::BTreeMap::new(),
            })
            .unwrap();
        session
            .record(&Record::ToolResult {
                call_id: "c1".into(),
                name: "read_file".into(),
                output: "a (1 lines)\n     1\ta\n".into(),
                truncated: false,
                approved: true,
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "c2".into(),
                requested: "edit".into(),
                resolved: "edit".into(),
                method: "exact".into(),
                reason: "declared".into(),
                catalog: 20,
                tier: "core".into(),
                argument_sources: std::collections::BTreeMap::new(),
            })
            .unwrap();
        session
            .record(&Record::ToolResult {
                call_id: "c2".into(),
                name: "edit".into(),
                output: "error: read `a` before editing it".into(),
                truncated: false,
                approved: true,
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "c3".into(),
                requested: "edit".into(),
                resolved: "edit".into(),
                method: "retrieved".into(),
                reason: "activated by tool_search".into(),
                catalog: 20,
                tier: "deferred".into(),
                argument_sources: std::collections::BTreeMap::new(),
            })
            .unwrap();
        session
            .record(&Record::ToolResult {
                call_id: "c3".into(),
                name: "edit".into(),
                output: "edited a: 1 occurrence".into(),
                truncated: false,
                approved: true,
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "c4".into(),
                requested: "ghost".into(),
                resolved: String::new(),
                method: "refused".into(),
                reason: "error: unknown tool `ghost`".into(),
                catalog: 20,
                tier: "core".into(),
                argument_sources: std::collections::BTreeMap::new(),
            })
            .unwrap();
        session
            .record(&Record::ToolResult {
                call_id: "c4".into(),
                name: "ghost".into(),
                output: "error[validation]: unknown tool".into(),
                truncated: true,
                approved: true,
            })
            .unwrap();
        let audit = audit(session.path()).unwrap();
        assert_eq!(audit.calls, 4);
        assert_eq!(audit.errors, 2);
        assert_eq!(audit.classes["validation"], 1);
        assert_eq!(audit.classes["error"], 1);
        assert_eq!(audit.truncated, 1);
        assert_eq!(audit.redundant, 1);
        assert_eq!(audit.recovery, Some(0.5));
        assert_eq!(audit.methods["exact"], 2);
        assert_eq!(audit.methods["retrieved"], 1);
        assert_eq!(audit.methods["refused"], 1);
        assert_eq!(audit.tiers["deferred"], 1);
    }

    #[test]
    fn the_audit_reads_the_argument_sources() {
        // C1's grounding, read as one number: a record whose own keys
        // supplied at least half the arguments counts as `model`, and
        // one leaning on schema defaults counts as `default`.
        let home = scratch("sources");
        let mut session = Session::create(&home, "test-model").unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "s1".into(),
                requested: "read_file".into(),
                resolved: "read_file".into(),
                method: "exact".into(),
                reason: "declared".into(),
                catalog: 20,
                tier: "core".into(),
                argument_sources: std::collections::BTreeMap::from([
                    ("path".to_string(), "model".to_string()),
                    ("offset".to_string(), "default".to_string()),
                ]),
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "s2".into(),
                requested: "grep".into(),
                resolved: "grep".into(),
                method: "exact".into(),
                reason: "declared".into(),
                catalog: 20,
                tier: "core".into(),
                argument_sources: std::collections::BTreeMap::from([
                    ("pattern".to_string(), "default".to_string()),
                    ("path".to_string(), "default".to_string()),
                ]),
            })
            .unwrap();
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: vec![call("s1", "read_file", "{}"), call("s2", "grep", "{}")],
                persona: None,
            })
            .unwrap();
        let audit = audit(session.path()).unwrap();
        assert_eq!(audit.calls, 2);
        assert_eq!(audit.argument_source["model"], 1);
        assert_eq!(audit.argument_source["default"], 1);
        let card = render("sources", &audit);
        assert!(
            card.contains("argument sources: default 1 \u{b7} model 1"),
            "{card}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_audit_prints_the_shares_with_their_counts() {
        // Tool-surface C2: truncation and redundancy print as shares of
        // the calls — the number the owner compares across sessions —
        // with the count beside it. The fixture: 4 calls, 1 truncated,
        // 1 redundant, so 25% and 25% exactly, not approximations.
        let home = scratch("rates");
        let mut session = Session::create(&home, "test-model").unwrap();
        let calls = vec![
            call("r1", "read_file", r#"{"path":"a"}"#),
            call(
                "r2",
                "edit",
                r#"{"path":"a","old_string":"x","new_string":"y"}"#,
            ),
            call(
                "r3",
                "edit",
                r#"{"path":"a","old_string":"x","new_string":"y"}"#,
            ),
            call("r4", "ghost", "{}"),
        ];
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: calls,
                persona: None,
            })
            .unwrap();
        for (id, name, output, truncated) in [
            ("r1", "read_file", "a\n", false),
            ("r2", "edit", "error: read `a` first", false),
            ("r3", "edit", "edited a", false),
            ("r4", "ghost", "error[validation]: unknown tool", true),
        ] {
            session
                .record(&Record::ToolResult {
                    call_id: id.into(),
                    name: name.into(),
                    output: output.into(),
                    truncated,
                    approved: true,
                })
                .unwrap();
        }
        let card = render("rates", &audit(session.path()).unwrap());
        assert!(card.contains("truncated: 25% (1)"), "{card}");
        assert!(card.contains("redundant calls: 25% (1)"), "{card}");
        // An empty session prints n/a, not 0% — a zero of nothing is not
        // a rate.
        let empty = Session::create(&scratch("empty"), "test-model").unwrap();
        let blank = render("empty", &audit(empty.path()).unwrap());
        assert!(blank.contains("no tool calls"), "{blank}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
