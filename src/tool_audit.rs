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
    /// The recovered and failed counts behind `recovery`, carried so an
    /// aggregate can sum rates instead of re-deriving them from shares.
    pub recovered: usize,
    pub failures: usize,
    /// Resolutions by method.
    pub methods: BTreeMap<String, usize>,
    /// Resolutions by tier.
    pub tiers: BTreeMap<String, usize>,
    /// Calls by argument source: `model` when the call's own keys
    /// supplied at least half the arguments, `default` when the schema
    /// defaults carried it. C1's grounding, read as one number.
    pub argument_source: BTreeMap<String, usize>,
    /// `Declarations` records seen — the requests whose tool set the log
    /// named. The grounded-selection baseline's denominator for the
    /// declaration tax.
    pub decl_requests: usize,
    /// Sum of declared tool names across those requests.
    pub decl_names_sum: usize,
    /// Sum of estimated declaration tokens (chars/4, the house
    /// convention) across those requests. Lane tools, whose schemas are
    /// not compiled in, count at the known specs' mean so a
    /// lane-heavy session is not silently undercounted.
    pub decl_tokens_sum: u64,
    /// `Usage` records seen — the requests the API answered.
    pub usage_requests: usize,
    pub prompt_tokens_sum: u64,
    pub completion_tokens_sum: u64,
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
            Record::Declarations { names, .. } => {
                audit.decl_requests += 1;
                audit.decl_names_sum += names.len();
                audit.decl_tokens_sum += declaration_tokens(names);
            }
            Record::Usage {
                prompt_tokens,
                completion_tokens,
                ..
            } => {
                audit.usage_requests += 1;
                audit.prompt_tokens_sum += prompt_tokens;
                audit.completion_tokens_sum += completion_tokens;
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
        audit.recovered = recovered;
        audit.failures = failures.len();
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

/// Estimated declaration tokens for one request's name set: chars/4 per
/// compiled spec, with names whose schema is not compiled in (the lane
/// tools) counted at the known specs' mean. The grounded-selection
/// baseline's fourth number; the estimate's shape is stated in the
/// render so a reader knows what it is.
fn declaration_tokens(names: &[String]) -> u64 {
    let table = spec_char_table();
    let known: Vec<usize> = table.values().copied().collect();
    let mean = if known.is_empty() {
        0.0
    } else {
        known.iter().sum::<usize>() as f64 / known.len() as f64
    };
    names
        .iter()
        .map(|name| {
            let chars = table
                .get(name.as_str())
                .copied()
                .unwrap_or(mean.max(1.0) as usize);
            (chars as u64).div_ceil(4)
        })
        .sum()
}

/// Compiled schema sizes, in chars, by tool name — the built-ins and
/// the free-function specials. Lane tools are absent by construction.
fn spec_char_table() -> HashMap<&'static str, usize> {
    let mut table = HashMap::new();
    for tool in crate::tools::ALL_TOOLS.iter() {
        let chars = serde_json::to_string(&tool.spec())
            .map(|json| json.len())
            .unwrap_or(0);
        table.insert(tool.name(), chars);
    }
    for (name, spec) in [
        ("remember", crate::tools::remember_spec()),
        ("improve", crate::tools::improve_spec()),
        ("process", crate::tools::process_spec()),
        ("tool_search", crate::tools::tool_search_spec()),
    ] {
        let chars = serde_json::to_string(&spec)
            .map(|json| json.len())
            .unwrap_or(0);
        table.insert(name, chars);
    }
    table
}

/// The archive's tool numbers: every session's audit summed. The
/// grounded-selection baseline reads its four rates from here — one
/// command, re-runnable after each phase, the number every later
/// criterion compares against.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Aggregate {
    pub sessions: usize,
    /// Logs that failed to replay (torn beyond tolerance), named by
    /// count only — a measurement must not die on one bad file.
    pub skipped: usize,
    pub calls: usize,
    pub errors: usize,
    pub classes: BTreeMap<String, usize>,
    pub methods: BTreeMap<String, usize>,
    pub recovered: usize,
    pub failures: usize,
    pub decl_requests: usize,
    pub decl_names_sum: usize,
    pub decl_tokens_sum: u64,
    pub usage_requests: usize,
    pub prompt_tokens_sum: u64,
    pub completion_tokens_sum: u64,
}

impl Aggregate {
    /// Calls whose resolution was `refused` — the model named a tool
    /// that does not exist (or an unresolvable name).
    pub fn wrong_tool_rate(&self) -> Option<f64> {
        let refused = self.methods.get("refused").copied().unwrap_or(0);
        rate(refused, self.calls)
    }
    /// Calls that failed schema or JSON validation.
    pub fn malformed_argument_rate(&self) -> Option<f64> {
        let validation = self.classes.get("validation").copied().unwrap_or(0);
        rate(validation, self.calls)
    }
    /// Of the failed calls, the share followed by a later success of
    /// the same tool — the repair-turn rate the plan's baseline names.
    pub fn repair_rate(&self) -> Option<f64> {
        rate(self.recovered, self.failures)
    }
    /// Mean estimated declaration tokens per request that declared a
    /// tool set.
    pub fn mean_decl_tokens(&self) -> Option<u64> {
        if self.decl_requests == 0 {
            return None;
        }
        Some(self.decl_tokens_sum / self.decl_requests as u64)
    }
    /// Mean declared tool names per request that declared a tool set.
    pub fn mean_decl_names(&self) -> Option<f64> {
        if self.decl_requests == 0 {
            return None;
        }
        Some(self.decl_names_sum as f64 / self.decl_requests as f64)
    }
    /// Mean prompt tokens per API-answered request.
    pub fn mean_prompt_tokens(&self) -> Option<u64> {
        if self.usage_requests == 0 {
            return None;
        }
        Some(self.prompt_tokens_sum / self.usage_requests as u64)
    }
}

fn rate(part: usize, whole: usize) -> Option<f64> {
    if whole == 0 {
        return None;
    }
    Some(part as f64 / whole as f64)
}

/// Audit every log in `paths`, merging into one archive-wide number.
pub fn aggregate(paths: &[std::path::PathBuf]) -> Aggregate {
    let mut aggregate = Aggregate::default();
    for path in paths {
        match audit(path) {
            Ok(one) => {
                aggregate.sessions += 1;
                aggregate.calls += one.calls;
                aggregate.errors += one.errors;
                for (class, count) in &one.classes {
                    *aggregate.classes.entry(class.clone()).or_insert(0) += count;
                }
                for (method, count) in &one.methods {
                    *aggregate.methods.entry(method.clone()).or_insert(0) += count;
                }
                aggregate.recovered += one.recovered;
                aggregate.failures += one.failures;
                aggregate.decl_requests += one.decl_requests;
                aggregate.decl_names_sum += one.decl_names_sum;
                aggregate.decl_tokens_sum += one.decl_tokens_sum;
                aggregate.usage_requests += one.usage_requests;
                aggregate.prompt_tokens_sum += one.prompt_tokens_sum;
                aggregate.completion_tokens_sum += one.completion_tokens_sum;
            }
            Err(_) => aggregate.skipped += 1,
        }
    }
    aggregate
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

/// The archive-wide card the grounded-selection baseline reads. Every
/// rate prints with its denominator — a share without its denominator
/// reads as precision it does not have (tool-surface C2's rule).
pub fn render_aggregate(aggregate: &Aggregate) -> String {
    let mut out = format!(
        "fleet tools audit \u{2014} all sessions: {} log(s){}\n",
        aggregate.sessions,
        if aggregate.skipped > 0 {
            format!(" ({} skipped: unplayable)", aggregate.skipped)
        } else {
            String::new()
        }
    );
    let pct = |part: usize, whole: usize| {
        rate(part, whole)
            .map(|share| format!("{:.1}% ({})", share * 100.0, part))
            .unwrap_or_else(|| "n/a".to_string())
    };
    out.push_str(&format!(
        "  calls: {} \u{b7} failures: {} \u{b7} recovery after a failure: {}\n",
        aggregate.calls,
        aggregate.failures,
        pct(aggregate.recovered, aggregate.failures),
    ));
    out.push_str(&format!(
        "  wrong tool (refused resolutions): {} \u{b7} malformed arguments (error[validation]): {}\n",
        pct(
            aggregate.methods.get("refused").copied().unwrap_or(0),
            aggregate.calls
        ),
        pct(
            aggregate.classes.get("validation").copied().unwrap_or(0),
            aggregate.calls
        ),
    ));
    if aggregate.decl_requests > 0 {
        out.push_str(&format!(
            "  declarations per request: {:.1} tool(s) \u{b7} est. {} tokens (chars/4; lane tools at the known-spec mean)\n",
            aggregate.mean_decl_names().unwrap_or(0.0),
            aggregate.mean_decl_tokens().unwrap_or(0),
        ));
    }
    if aggregate.usage_requests > 0 {
        out.push_str(&format!(
            "  usage per request: {} prompt \u{b7} {} completion tokens ({} requests)\n",
            aggregate.mean_prompt_tokens().unwrap_or(0),
            aggregate.completion_tokens_sum / aggregate.usage_requests.max(1) as u64,
            aggregate.usage_requests,
        ));
    }
    if !aggregate.classes.is_empty() {
        let classes: Vec<String> = aggregate
            .classes
            .iter()
            .map(|(class, count)| format!("{class} {count}"))
            .collect();
        out.push_str(&format!("  error classes: {}\n", classes.join(" \u{b7} ")));
    }
    let methods: Vec<String> = aggregate
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

    #[test]
    fn the_aggregate_sums_sessions_and_carries_every_rate() {
        // The grounded-selection baseline's shape: two sessions merged,
        // the four rates readable with their denominators, the
        // declaration and usage sums carried from the records.
        let home = scratch("aggregate");
        let mut session = Session::create(&home, "test-model").unwrap();
        session
            .record(&Record::Declarations {
                names: vec![
                    "read_file".into(),
                    "edit".into(),
                    "mcp__sys1__sys1_classify".into(),
                ],
                tier: "core".into(),
                withheld: Vec::new(),
            })
            .unwrap();
        session
            .record(&Record::Usage {
                turn: 1,
                prompt_tokens: 4000,
                completion_tokens: 200,
            })
            .unwrap();
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: vec![
                    call("a1", "read_file", r#"{"path":"a"}"#),
                    call("a2", "ghost", "{}"),
                ],
                persona: None,
            })
            .unwrap();
        session
            .record(&Record::ToolResolution {
                call_id: "a2".into(),
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
                call_id: "a2".into(),
                name: "ghost".into(),
                output: "error[validation]: unknown tool `ghost`".into(),
                truncated: false,
                approved: true,
            })
            .unwrap();
        let mut second = Session::create(&home, "test-model").unwrap();
        second
            .record(&Record::Usage {
                turn: 1,
                prompt_tokens: 2000,
                completion_tokens: 100,
            })
            .unwrap();
        second
            .record(&Record::Assistant {
                content: None,
                tool_calls: vec![call("b1", "bash", r#"{"command":"ls"}"#)],
                persona: None,
            })
            .unwrap();
        second
            .record(&Record::ToolResult {
                call_id: "b1".into(),
                name: "bash".into(),
                output: "a\nb\n".into(),
                truncated: false,
                approved: true,
            })
            .unwrap();
        let paths: Vec<std::path::PathBuf> = crate::session::list(&home)
            .unwrap()
            .into_iter()
            .map(|s| s.path)
            .collect();
        assert_eq!(paths.len(), 2);
        let aggregate = aggregate(&paths);
        assert_eq!(aggregate.sessions, 2);
        assert_eq!(aggregate.calls, 3);
        assert_eq!(aggregate.failures, 1);
        assert_eq!(aggregate.methods["refused"], 1);
        assert_eq!(aggregate.classes["validation"], 1);
        assert_eq!(
            aggregate.wrong_tool_rate(),
            Some(1.0 / 3.0),
            "one refused call of three"
        );
        assert_eq!(aggregate.malformed_argument_rate(), Some(1.0 / 3.0));
        assert_eq!(
            aggregate.repair_rate(),
            Some(0.0),
            "the one failure recovered never"
        );
        assert_eq!(aggregate.decl_requests, 1);
        assert_eq!(aggregate.decl_names_sum, 3);
        assert!(
            aggregate.decl_tokens_sum > 0,
            "the three schemas render to tokens"
        );
        assert_eq!(aggregate.usage_requests, 2);
        assert_eq!(aggregate.prompt_tokens_sum, 6000);
        assert_eq!(aggregate.mean_prompt_tokens(), Some(3000));
        let card = render_aggregate(&aggregate);
        assert!(card.contains("all sessions: 2 log(s)"), "{card}");
        assert!(
            card.contains("wrong tool (refused resolutions): 33.3% (1)"),
            "{card}"
        );
        assert!(
            card.contains("malformed arguments (error[validation]): 33.3% (1)"),
            "{card}"
        );
        assert!(
            card.contains("declarations per request: 3.0 tool(s)"),
            "{card}"
        );
        assert!(card.contains("usage per request: 3000 prompt"), "{card}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_aggregate_counts_skipped_logs_without_dying() {
        // One unplayable file must not take the measurement down: it is
        // a named skip, and the rest of the archive still reads.
        let home = scratch("aggregate-skip");
        let mut session = Session::create(&home, "test-model").unwrap();
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: vec![call("k1", "bash", r#"{"command":"ls"}"#)],
                persona: None,
            })
            .unwrap();
        let torn = home.join("torn.jsonl");
        // A torn *trailing* record is tolerated (KI-213) — the skip
        // path needs a mid-file break, the shape that is corruption
        // rather than an interrupted write.
        std::fs::write(
            &torn,
            "{\"torn\"\n{\"v\":1,\"model\":\"m\",\"created\":\"t\"}\n",
        )
        .unwrap();
        let paths = vec![session.path().to_path_buf(), torn];
        let aggregate = aggregate(&paths);
        assert_eq!(aggregate.sessions, 1, "the good log still reads");
        assert_eq!(aggregate.skipped, 1, "the torn log is a named skip");
        assert_eq!(aggregate.calls, 1);
        assert!(
            render_aggregate(&aggregate).contains("1 skipped"),
            "the skip is named"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
