//! The tool advisor: what the sessions actually asked for, and what that
//! says about the tool set.
//!
//! The scan is the skills usage scan's shape applied to tools — the most
//! recent session logs, a count per name, no model. The advice is a set
//! of deterministic rules over those counts: a name that matched nothing,
//! a tool that failed a quarter of the time, a shell command that does
//! work a native tool already covers, an enabled server nobody called.
//! Nothing here changes the harness; the owner reads the card.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::session::Record;

/// How many sessions the scan reads unless told otherwise.
pub const DEFAULT_WINDOW: usize = 20;

/// One tool's tally across the window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolStat {
    pub calls: usize,
    pub errors: usize,
    pub denied: usize,
}

impl ToolStat {
    fn error_rate(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.errors as f64 / self.calls as f64
        }
    }
}

/// What the window said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolScan {
    pub sessions: usize,
    pub window: usize,
    /// Every name the model called, including lane and unknown names.
    pub per_tool: BTreeMap<String, ToolStat>,
    /// Names no declared tool and no alias answered.
    pub unknown: BTreeMap<String, usize>,
    /// Names another harness taught the model that an alias resolved.
    pub alias_attempts: BTreeMap<String, usize>,
    /// The first word of every `bash` command that does work a native
    /// tool covers, with its count.
    pub shell_shapes: BTreeMap<String, usize>,
    /// `tool_search` results that said nothing fits.
    pub search_misses: usize,
    /// Identical (name, arguments) calls repeated within one session.
    pub repeated: usize,
    /// Enabled servers, and how often their tools were called.
    pub enabled_servers: Vec<String>,
    pub server_calls: BTreeMap<String, usize>,
}

/// Scan the most recent `window` session logs.
pub fn scan(home: &Path, window: usize) -> ToolScan {
    let window = window.max(1);
    let mut scan = ToolScan {
        window,
        ..ToolScan::default()
    };
    scan.enabled_servers = crate::mcp_registry::read_servers(home)
        .map(|servers| servers.into_iter().map(|(name, _)| name).collect())
        .unwrap_or_default();
    let Ok(summaries) = crate::session::list(home) else {
        return scan;
    };
    for summary in summaries.iter().rev().take(window) {
        scan.sessions += 1;
        let Ok(records) = crate::session::replay(&summary.path) else {
            continue;
        };
        let mut results: HashMap<String, (bool, String)> = HashMap::new();
        for record in &records {
            if let Record::ToolResult {
                call_id,
                output,
                approved,
                ..
            } = record
            {
                results.insert(call_id.clone(), (*approved, output.clone()));
            }
        }
        let known = crate::tools::known_tool_names();
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        for record in &records {
            let Record::Assistant { tool_calls, .. } = record else {
                continue;
            };
            for call in tool_calls {
                let name = call.function.name.as_str();
                let stat = scan.per_tool.entry(name.to_string()).or_default();
                stat.calls += 1;
                let (approved, output) = results
                    .get(&call.id)
                    .cloned()
                    .unwrap_or((true, String::new()));
                if !approved {
                    stat.denied += 1;
                }
                if output.starts_with("error") || output.starts_with("denied") {
                    stat.errors += 1;
                }
                match crate::tools::resolve_name(name) {
                    Some((_, true)) => {
                        *scan.alias_attempts.entry(name.to_string()).or_insert(0) += 1;
                    }
                    Some((_, false)) => {}
                    None if !known.contains(&name)
                        && crate::tools::lane_of(name).is_none()
                        && name != crate::tools::TOOL_SEARCH_TOOL =>
                    {
                        *scan.unknown.entry(name.to_string()).or_insert(0) += 1;
                    }
                    None => {}
                }
                if name == crate::tools::Tool::Bash.name() {
                    if let Ok(arguments) =
                        serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                    {
                        if let Some(command) =
                            arguments.get("command").and_then(|value| value.as_str())
                        {
                            if let Some(shape) = shell_shape(command) {
                                *scan.shell_shapes.entry(shape.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                }
                if name == crate::tools::TOOL_SEARCH_TOOL
                    && (output.contains("no tool clears the floor")
                        || output.contains("no tool is needed"))
                {
                    scan.search_misses += 1;
                }
                let key = (name.to_string(), call.function.arguments.clone());
                let count = seen.entry(key).or_insert(0);
                *count += 1;
                if *count == 2 {
                    scan.repeated += 1;
                }
                if let Some((server, _)) = name
                    .strip_prefix(crate::tools::MCP_NAMESPACE)
                    .and_then(|rest| rest.split_once("__"))
                {
                    *scan.server_calls.entry(server.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
    scan
}

/// What the advice is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdviceKind {
    /// Build a tool that does not exist.
    Build,
    /// Fix a tool whose calls fail.
    Fix,
    /// Adopt a tool or alias that already exists.
    Adopt,
    /// Withhold something nobody calls.
    Retire,
}

impl AdviceKind {
    pub fn label(self) -> &'static str {
        match self {
            AdviceKind::Build => "build",
            AdviceKind::Fix => "fix",
            AdviceKind::Adopt => "adopt",
            AdviceKind::Retire => "retire",
        }
    }
}

/// One evidence-backed suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    pub kind: AdviceKind,
    pub title: String,
    pub evidence: String,
    pub action: String,
}

/// Turn a scan into suggestions. Pure, so the rules are a test.
pub fn advise(scan: &ToolScan) -> Vec<Advice> {
    let mut advice: Vec<Advice> = Vec::new();
    // A name another harness taught the model that an alias now answers.
    for (name, count) in &scan.alias_attempts {
        let tool = crate::tools::resolve_name(name)
            .map(|(tool, _)| tool.name())
            .unwrap_or("");
        advice.push(Advice {
            kind: AdviceKind::Adopt,
            title: format!("`{name}` resolves to `{tool}`"),
            evidence: format!(
                "{count} attempts at `{name}`, an alias of `{tool}`, across {} sessions",
                scan.sessions
            ),
            action: "the alias is live; if the model still reaches for the old spelling, put it in the prompt".to_string(),
        });
    }
    // A name the model reached for and nothing answered.
    for (name, count) in &scan.unknown {
        advice.push(Advice {
            kind: AdviceKind::Build,
            title: format!("`{name}` was called but no tool answers it"),
            evidence: format!(
                "{count} attempts at `{name}` across {} sessions",
                scan.sessions
            ),
            action: "build the tool, or add an alias if it names an existing one".to_string(),
        });
    }
    // A tool that fails a meaningful share of its calls.
    for (name, stat) in &scan.per_tool {
        if stat.calls >= 3 && stat.error_rate() >= 0.25 {
            advice.push(Advice {
                kind: AdviceKind::Fix,
                title: format!(
                    "`{name}` fails {:.0}% of its calls",
                    stat.error_rate() * 100.0
                ),
                evidence: format!(
                    "{} errors in {} calls across {} sessions",
                    stat.errors, stat.calls, scan.sessions
                ),
                action: "read the failing results and tighten the description or the schema"
                    .to_string(),
            });
        }
    }
    // A search that found nothing.
    if scan.search_misses > 0 {
        advice.push(Advice {
            kind: AdviceKind::Build,
            title: "tool_search found no tool".to_string(),
            evidence: format!(
                "{} searches answered `no tool fits` across {} sessions",
                scan.search_misses, scan.sessions
            ),
            action: "read those queries: they name a capability the catalog does not carry"
                .to_string(),
        });
    }
    // Shell work a native tool already covers.
    for (shape, count) in &scan.shell_shapes {
        if *count >= 3 {
            advice.push(Advice {
                kind: AdviceKind::Adopt,
                title: format!("{count} `{shape}` calls went through bash"),
                evidence: format!(
                    "{count} bash commands ran `{shape}`, which `grep`, `glob`, and `edit` cover natively"
                ),
                action: "point the prompt at the native tools, or add this spelling as an alias".to_string(),
            });
        }
    }
    // Enabled and never called: one line, because thirteen retire rows
    // bury the build and fix advice that actually changes the harness.
    let idle: Vec<&str> = scan
        .enabled_servers
        .iter()
        .map(String::as_str)
        .filter(|server| scan.server_calls.get(*server).copied().unwrap_or(0) == 0)
        .collect();
    if !idle.is_empty() {
        advice.push(Advice {
            kind: AdviceKind::Retire,
            title: format!("{} enabled server(s) were never called", idle.len()),
            evidence: idle.join(", "),
            action: "probe them with `fleet mcp health`, or disable the ones you do not want"
                .to_string(),
        });
    }
    // The same call twice in one session.
    if scan.repeated > 0 {
        advice.push(Advice {
            kind: AdviceKind::Fix,
            title: "the same call repeated within a session".to_string(),
            evidence: format!("{} duplicate (name, arguments) calls", scan.repeated),
            action: "batch independent calls into one turn; a second identical read is a waste"
                .to_string(),
        });
    }
    advice.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| right.evidence.cmp(&left.evidence))
    });
    advice
}

/// The shell command's first word, when a native tool covers it.
fn shell_shape(command: &str) -> Option<&'static str> {
    let first = command
        .split_whitespace()
        .find(|word| !word.is_empty() && !word.starts_with('-'))?;
    let word = first.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
    Some(match word {
        "rg" | "ripgrep" | "grep" | "egrep" | "fgrep" => "grep",
        "find" | "fd" | "bfs" => "glob",
        "sed" | "awk" => "edit",
        _ => return None,
    })
}

/// The scan as a card.
pub fn render_scan(scan: &ToolScan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "fleet tools scan \u{2014} {} session(s) of the last {}\n",
        scan.sessions, scan.window
    ));
    if scan.per_tool.is_empty() {
        out.push_str("no tool calls in the window\n");
        return out;
    }
    out.push_str("\n  calls  errors  tool\n");
    let mut rows: Vec<(&String, &ToolStat)> = scan.per_tool.iter().collect();
    rows.sort_by(|left, right| right.1.calls.cmp(&left.1.calls));
    for (name, stat) in rows {
        out.push_str(&format!(
            "  {:>5}  {:>6}  {}{}\n",
            stat.calls,
            stat.errors,
            name,
            if stat.denied > 0 {
                format!(" ({} denied)", stat.denied)
            } else {
                String::new()
            }
        ));
    }
    if !scan.alias_attempts.is_empty() {
        out.push_str("\n  alias recoveries\n");
        for (name, count) in &scan.alias_attempts {
            out.push_str(&format!("  {count:>5}          {name}\n"));
        }
    }
    if !scan.unknown.is_empty() {
        out.push_str("\n  unknown names\n");
        for (name, count) in &scan.unknown {
            out.push_str(&format!("  {count:>5}          {name}\n"));
        }
    }
    if !scan.shell_shapes.is_empty() {
        out.push_str("\n  bash work a native tool covers\n");
        for (shape, count) in &scan.shell_shapes {
            out.push_str(&format!("  {count:>5}          {shape}\n"));
        }
    }
    if scan.search_misses > 0 {
        out.push_str(&format!(
            "\n  {} search(es) found no tool\n",
            scan.search_misses
        ));
    }
    if scan.repeated > 0 {
        out.push_str(&format!(
            "\n  {} duplicate call(s) within a session\n",
            scan.repeated
        ));
    }
    if !scan.enabled_servers.is_empty() {
        out.push_str(&format!(
            "\n  enabled servers: {}\n",
            scan.enabled_servers.join(", ")
        ));
    }
    out
}

/// The advice as a card.
pub fn render_advice(advice: &[Advice], scan: &ToolScan) -> String {
    if advice.is_empty() {
        return format!(
            "fleet tools advise \u{2014} {} session(s): nothing to change\n",
            scan.sessions
        );
    }
    let mut out = format!(
        "fleet tools advise \u{2014} {} suggestion(s) over {} session(s)\n",
        advice.len(),
        scan.sessions
    );
    for item in advice {
        out.push_str(&format!(
            "\n  [{}] {}\n        {}\n        next: {}\n",
            item.kind.label(),
            item.title,
            item.evidence,
            item.action
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    fn log(home: &Path, calls: &[(&str, &str, &str)]) {
        let mut session = Session::create(home, "test-model").unwrap();
        let tool_calls: Vec<crate::api::ToolCall> = calls
            .iter()
            .enumerate()
            .map(|(index, (name, arguments, _))| crate::api::ToolCall {
                id: format!("c{index}"),
                kind: "function".to_string(),
                function: crate::api::FunctionCall {
                    name: (*name).to_string(),
                    arguments: (*arguments).to_string(),
                },
            })
            .collect();
        session
            .record(&Record::Assistant {
                content: None,
                tool_calls: tool_calls.clone(),
            })
            .unwrap();
        for (index, (name, _, output)) in calls.iter().enumerate() {
            session
                .record(&Record::ToolResult {
                    call_id: format!("c{index}"),
                    name: (*name).to_string(),
                    output: (*output).to_string(),
                    truncated: false,
                    approved: true,
                })
                .unwrap();
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-advisor-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_scan_counts_calls_errors_and_unknown_names() {
        let home = scratch("counts");
        log(
            &home,
            &[
                ("read_file", r#"{"path":"a"}"#, "a"),
                ("str_replace", r#"{"path":"a"}"#, "error: unknown tool"),
                ("bash", r#"{"command":"rg needle ."}"#, "a:1: needle"),
            ],
        );
        let scan = scan(&home, 5);
        assert_eq!(scan.sessions, 1);
        assert_eq!(scan.per_tool["read_file"].calls, 1);
        assert_eq!(scan.per_tool["str_replace"].errors, 1);
        assert_eq!(scan.alias_attempts["str_replace"], 1);
        assert!(scan.unknown.is_empty());
        assert_eq!(scan.shell_shapes["grep"], 1);
    }

    #[test]
    fn the_window_is_the_most_recent_sessions() {
        let home = scratch("window");
        log(&home, &[("read_file", "{}", "old")]);
        std::thread::sleep(std::time::Duration::from_millis(20));
        log(&home, &[("read_file", "{}", "new")]);
        assert_eq!(scan(&home, 1).sessions, 1);
        assert_eq!(scan(&home, 10).sessions, 2);
        assert_eq!(scan(&home, 10).per_tool["read_file"].calls, 2);
    }

    #[test]
    fn the_advice_names_a_build_a_fix_and_an_alias() {
        let home = scratch("advice");
        let repeated = r#"{"command":"ls"}"#;
        log(
            &home,
            &[
                ("ghost_tool", "{}", "error: unknown tool `ghost_tool`"),
                ("bash", repeated, "a"),
                ("bash", repeated, "a"),
                ("bash", r#"{"command":"bash"}"#, "error: boom"),
                ("bash", r#"{"command":"bash"}"#, "error: boom"),
                ("bash", r#"{"command":"bash"}"#, "error: boom"),
                ("str_replace", "{}", "error: unknown tool"),
            ],
        );
        let scan = scan(&home, 5);
        let advice = advise(&scan);
        assert!(advice
            .iter()
            .any(|item| item.kind == AdviceKind::Build && item.title.contains("ghost_tool")));
        assert!(advice
            .iter()
            .any(|item| item.kind == AdviceKind::Adopt && item.title.contains("str_replace")));
        assert!(advice
            .iter()
            .any(|item| item.kind == AdviceKind::Fix && item.title.contains("bash")));
        assert!(advice.iter().any(|item| item.title.contains("repeated")));
    }

    #[test]
    fn an_enabled_server_nobody_called_is_one_retire_row() {
        let home = scratch("retire");
        log(&home, &[("read_file", "{}", "a")]);
        let mut scan = scan(&home, 5);
        scan.enabled_servers = vec!["filesystem".to_string(), "github".to_string()];
        let advice = advise(&scan);
        let retire: Vec<&Advice> = advice
            .iter()
            .filter(|item| item.kind == AdviceKind::Retire)
            .collect();
        assert_eq!(retire.len(), 1, "{advice:?}");
        assert!(
            retire[0].title.contains("2 enabled server"),
            "{:?}",
            retire[0]
        );
        assert!(retire[0].evidence.contains("filesystem"));
    }

    #[test]
    fn shell_shapes_only_cover_what_a_native_tool_owns() {
        assert_eq!(shell_shape("rg -n needle src"), Some("grep"));
        assert_eq!(shell_shape("find . -name '*.rs'"), Some("glob"));
        assert_eq!(shell_shape("sed -i '' 's/a/b/' x"), Some("edit"));
        assert_eq!(shell_shape("cargo test"), None);
    }
}
