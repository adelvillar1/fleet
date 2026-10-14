//! `fleet doctor` checks: credentials, home, API reachability, shell.
//! Evaluators take facts the caller gathered, so every evaluator runs
//! hermetically in tests.

use std::fs;
use std::path::Path;

use crate::api::ClientError;
use crate::session::Record;

/// One probe's outcome.
pub struct Check {
    /// Short check name, e.g. `credentials`.
    pub name: &'static str,
    /// Whether the check passed.
    pub ok: bool,
    /// Human-readable detail, e.g. the masked key.
    pub detail: String,
}

/// Render one check as a report line.
pub fn render(check: &Check) -> String {
    let verdict = if check.ok { "ok  " } else { "FAIL" };
    format!("{:12} {}  {}", check.name, verdict, check.detail)
}

/// The credentials check from a resolved `DEEPSEEK_API_KEY` (after the
/// `.env` and Harness credential-store fallbacks). The detail masks
/// everything but the last four characters.
pub fn evaluate_credentials(key: Option<&str>) -> Check {
    match key {
        None => Check {
            name: "credentials",
            ok: false,
            detail: "DEEPSEEK_API_KEY is not set; export it, add it to ~/.fleet/.env, or store it in ~/.dsh/.credentials.yaml".to_string(),
        },
        Some(key) => Check {
            name: "credentials",
            ok: true,
            detail: format!("set ({})", mask(key)),
        },
    }
}

/// `…last4` for anything longer than a plausible short prefix; short values
/// are masked entirely. Operates on chars, not bytes: a key is arbitrary text
/// until the API says otherwise, and byte-slicing 4-from-the-end panics when
/// the index straddles a multi-byte char (logged as KI-010).
fn mask(key: &str) -> String {
    if key.chars().count() > 8 {
        let last_four: String =
            key.chars().rev().take(4).collect::<Vec<char>>().into_iter().rev().collect();
        format!("\u{2026}{last_four}")
    } else {
        "\u{2026}".to_string()
    }
}

/// The home check from an `ensure_sessions_dir` outcome with its error
/// already rendered.
pub fn evaluate_home(home: Option<&Path>, outcome: Result<(), String>) -> Check {
    match (home, outcome) {
        (None, _) => Check {
            name: "home",
            ok: false,
            detail: "no home directory known; set FLEET_HOME or HOME".to_string(),
        },
        (Some(home), Ok(())) => Check {
            name: "home",
            ok: true,
            detail: format!("writable ({})", home.display()),
        },
        (Some(home), Err(error)) => Check {
            name: "home",
            ok: false,
            detail: format!("{}: {error}", home.display()),
        },
    }
}

/// The API reachability check from a probe outcome against the parsed
/// endpoint.
pub fn evaluate_api(endpoint: Option<(&str, u16)>, reachable: bool) -> Check {
    match (endpoint, reachable) {
        (None, _) => Check {
            name: "api",
            ok: false,
            detail: "DEEPSEEK_BASE_URL is not a host[:port] URL".to_string(),
        },
        (Some((host, port)), true) => Check {
            name: "api",
            ok: true,
            detail: format!("reachable ({host}:{port})"),
        },
        (Some((host, port)), false) => Check {
            name: "api",
            ok: false,
            detail: format!("{host}:{port} unreachable; check network or DEEPSEEK_BASE_URL"),
        },
    }
}

/// Parse an API base URL into `(host, port)`, defaulting the port by scheme.
pub fn api_endpoint(base_url: &str) -> Option<(&str, u16)> {
    let (scheme, rest) = base_url.split_once("://")?;
    let authority = rest.split(['/']).next()?;
    if authority.is_empty() {
        return None;
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host, port.parse().ok()?)),
        None => match scheme {
            "https" => Some((authority, 443)),
            "http" => Some((authority, 80)),
            _ => None,
        },
    }
}

/// The shell check from a probe outcome.
pub fn evaluate_shell(probe: Result<&'static str, String>) -> Check {
    match probe {
        Ok(shell) => Check { name: "shell", ok: true, detail: format!("{shell} available") },
        Err(error) => Check { name: "shell", ok: false, detail: error },
    }
}

/// The calibration-stream line, informational and never a check: the
/// resolved root and whether today's file exists. It is the cheapest
/// guard against dev-decisions moving its `LOG_DIR` literal and silently
/// re-routing Fleet's stream — the root is named where an operator looks.
/// It never touches the exit code.
pub fn render_calib(root: Option<&Path>, today_file_exists: bool) -> String {
    let detail = match root {
        Some(root) => {
            let status = if today_file_exists { "present" } else { "absent" };
            format!("{} (today's file: {})", root.display(), status)
        }
        None => "no home directory known; set FLEET_HOME or HOME".to_string(),
    };
    format!("{:12} {}  {}", "calib", "info", detail)
}

/// The recall-path check: which retrieval path is live. It passes in
/// **both** states — a keyword-only recall is a working configuration,
/// not a failure, and a clean machine has no local embedding model — and
/// its detail names the path. When the vector path is absent, the one
/// named line says so: that is the live system's failure this check
/// exists to fix, where an absent embedding stack silently degraded
/// recall over 11k entries. The line is never silent.
///
/// `vector` is `(endpoint, dims)` when the probe found a live local
/// model, `None` for the keyword-only degradation.
pub fn evaluate_recall(vector: Option<(&str, usize)>) -> Check {
    let detail = match vector {
        Some((endpoint, dims)) => {
            format!("vector path live \u{2014} {endpoint} ({dims}-dim embeddings serve recall)")
        }
        None => "embeddings unavailable \u{2014} recall is keyword-only".to_string(),
    };
    Check { name: "recall", ok: true, detail }
}

// --- slice 13: the per-layer diagnosis -------------------------------------

/// How a failure is classified, in the plan's own five words plus the
/// honest sixth. The classes are the crate's existing typed failures
/// named at the diagnosis seam: a [`ClientError`] maps to network, auth,
/// quota, or decode; a tool result that errored maps to tool; and a
/// failure that fits none of them is `unknown` rather than forced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Network,
    Auth,
    Quota,
    Decode,
    Tool,
    Unknown,
}

impl FailureClass {
    /// The word the diagnosis and the ledger row carry.
    pub fn label(self) -> &'static str {
        match self {
            FailureClass::Network => "network",
            FailureClass::Auth => "auth",
            FailureClass::Quota => "quota",
            FailureClass::Decode => "decode",
            FailureClass::Tool => "tool",
            FailureClass::Unknown => "unknown",
        }
    }
}

/// Classify a client failure into the plan's classes. The mapping is the
/// typed error's own shape: a transport failure is network, a malformed
/// 2xx is decode, and an answered status splits by its code — 401/403
/// are auth, 402/429 are quota, and any other answered status is a
/// server-side (network-class) failure rather than the caller's.
pub fn classify_error(error: &ClientError) -> FailureClass {
    match error {
        ClientError::Network(_) => FailureClass::Network,
        ClientError::Decode(_) => FailureClass::Decode,
        ClientError::Api(api) => match api.status {
            401 | 403 => FailureClass::Auth,
            402 | 429 => FailureClass::Quota,
            _ => FailureClass::Network,
        },
    }
}

/// Classify a tool result: a result that opens `error:` is a tool-class
/// failure, and anything else (including a `denied:` policy outcome, which
/// is a decision, not a failure) is not. `None` names "not a failure".
pub fn classify_tool(output: &str) -> Option<FailureClass> {
    output.starts_with("error:").then_some(FailureClass::Tool)
}

/// The roster layer: how many routing candidates are ready to serve. A
/// roster with no ready candidate is the broken layer — a keyless
/// machine — and the check names the count and, when none are ready, the
/// reason a caller can act on.
pub fn evaluate_roster(ready: usize, total: usize) -> Check {
    if total == 0 {
        return Check {
            name: "roster",
            ok: false,
            detail: "no routing candidates configured \u{2014} add a provider to config.json".to_string(),
        };
    }
    if ready == 0 {
        return Check {
            name: "roster",
            ok: false,
            detail: format!("0 of {total} candidates ready \u{2014} every provider is missing its key"),
        };
    }
    Check { name: "roster", ok: true, detail: format!("{ready} of {total} candidates ready") }
}

/// The ledger layer: the calibration stream's root and whether it is
/// writable. An unwritable stream is the broken layer — the audit spine
/// every self-improvement loop writes to — and the check names the root
/// and the reason rather than staying silent about it.
pub fn evaluate_ledger(root: Option<&Path>, writable: bool) -> Check {
    let Some(root) = root else {
        return Check {
            name: "ledger",
            ok: false,
            detail: "no calibration root known; set FLEET_HOME or HOME".to_string(),
        };
    };
    if writable {
        Check { name: "ledger", ok: true, detail: format!("writable ({})", root.display()) }
    } else {
        Check {
            name: "ledger",
            ok: false,
            detail: format!("{} is not writable \u{2014} the calibration stream cannot record", root.display()),
        }
    }
}

/// One MCP core's layer: whether its home resolves. `resolved` is the
/// lane's home on success, or the named reason it did not resolve. A
/// core that does not resolve is named with its reason, so `doctor`
/// says which of the four is unavailable and why, not just that one is.
pub fn evaluate_mcp_core(name: &'static str, resolved: Result<&Path, String>) -> Check {
    match resolved {
        Ok(home) => {
            Check { name, ok: true, detail: format!("resolves ({})", home.display()) }
        }
        Err(reason) => Check { name, ok: false, detail: reason },
    }
}

// --- slice 13: the evidence-first diagnosis and the ledger filing ----------

/// One diagnosed failure: its class, the layer it sits in, a one-line
/// summary, and the evidence that produced it (the session log's records,
/// the probe's finding). This is the unit the drafting and the filing
/// both work from — composed from evidence, never from narrative.
#[derive(Debug, Clone)]
pub struct Diagnosis {
    pub class: FailureClass,
    pub layer: String,
    pub summary: String,
    pub evidence: String,
}

/// One ledger row a diagnosis files: the severity it carries and the
/// diagnosis behind it. The severity is the ledger's own vocabulary
/// (`defect`, `gap`, `improvement`, `risk`, `environmental`).
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub severity: &'static str,
    pub diagnosis: Diagnosis,
}

/// The stable key a ledger row is filed or updated by: a 16-hex hash of
/// the failure's class, layer, and summary. Two diagnoses of the same
/// failure carry the same signature, so re-diagnosing updates the row
/// rather than appending a duplicate.
pub fn signature(diagnosis: &Diagnosis) -> String {
    crate::calib::hash16(&format!(
        "{}:{}:{}",
        diagnosis.class.label(),
        diagnosis.layer,
        diagnosis.summary
    ))
}

/// What one filing did: appended a new row, or updated the existing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filed {
    Appended { id: String },
    Updated { id: String },
}

/// The next `KI-\d+` id in a ledger's text: one past the largest present,
/// or `KI-001` when the ledger holds none.
fn next_ki_id(text: &str) -> String {
    let max = text
        .lines()
        .filter_map(|line| {
            let cell = line.trim_start().strip_prefix("| KI-")?;
            let digits: String = cell.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0);
    format!("KI-{:03}", max + 1)
}

/// The id a ledger row leads with (`| KI-104 |` -> `KI-104`).
fn row_id(line: &str) -> Option<String> {
    let cell = line.trim_start().strip_prefix("| ")?;
    let id: String = cell.chars().take_while(|c| !c.is_whitespace()).collect();
    (!id.is_empty()).then_some(id)
}

/// One ledger row's cells: the six `|`-separated fields (id, severity,
/// issue, evidence, status, next action), trimmed. `None` when the
/// line is not a six-cell row — a header, a separator, prose.
fn row_cells(line: &str) -> Option<Vec<&str>> {
    let mut cells: Vec<&str> = line.split('|').map(str::trim).collect();
    // A row is `| a | b | … |`: the split opens and closes on the
    // pipes, so the first and last cells are empty.
    if cells.first().is_some_and(|cell| cell.is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|cell| cell.is_empty()) {
        cells.pop();
    }
    (cells.len() == 6).then_some(cells)
}

/// One ledger row's text, in the ledger's own six-column shape, with
/// the signature riding the evidence cell so a later filing can find
/// it. The status and the next action are the row owner's cells: a
/// new row opens with `open`/`triage`, an update carries the
/// existing row's own.
fn render_ki_row(
    id: &str,
    entry: &LedgerEntry,
    sig: &str,
    status: &str,
    next_action: &str,
) -> String {
    format!(
        "| {id} | {severity} | {class} failure in {layer}: {summary} | {evidence} [sig:{sig}] | {status} | {next_action} |",
        severity = entry.severity,
        class = entry.diagnosis.class.label(),
        layer = entry.diagnosis.layer,
        summary = entry.diagnosis.summary,
        evidence = entry.diagnosis.evidence,
    )
}

/// File one diagnosis into a ledger: append a new row, or update the
/// existing row that carries the same signature. The update re-renders
/// only the matched row; every other line is preserved byte for byte, so
/// a filing never rewrites an unrelated row. The write goes through a
/// temp file and a rename, so a reader never sees a half-written ledger.
///
/// An update **preserves the row owner's cells**: the status and the
/// next action stay exactly as the human left them — a re-diagnosis
/// is evidence gathering, not triage, and silently reopening a closed
/// row (or discarding its next action) is the named review finding.
/// Only the issue and the evidence refresh. A row whose status or
/// next-action cell cannot be read back (a malformed row) opens as a
/// new row would rather than guessing.
///
/// The ledger is an LF file: the update path rebuilds the text with
/// `lines().join("\n")` plus one trailing newline, so a CRLF ledger
/// normalizes to LF and a file without a final newline gains one.
/// The append path adds the trailing newline the same way. This is
/// the documented assumption — `docs/known-issues.md` is LF.
///
/// The caller decides the file — a test points it at a scratch ledger,
/// production names the repo's `docs/known-issues.md`. This function
/// writes wherever it is pointed and nowhere else.
pub fn file_ledger_row(path: &Path, entry: &LedgerEntry) -> Result<Filed, String> {
    let sig = signature(&entry.diagnosis);
    let marker = format!("[sig:{sig}]");
    let text = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    if let Some(index) = text.lines().position(|line| line.contains(&marker)) {
        // Update the matched row in place, keeping its id, its status,
        // and its next action: the human's cells are not the
        // automation's to overwrite.
        let id = text
            .lines()
            .nth(index)
            .and_then(row_id)
            .unwrap_or_else(|| next_ki_id(&text));
        let preserved = text.lines().nth(index).and_then(row_cells);
        let (status, next_action) = match preserved {
            Some(cells) => (cells[4].to_string(), cells[5].to_string()),
            None => ("open".to_string(), "triage".to_string()),
        };
        let updated = render_ki_row(&id, entry, &sig, &status, &next_action);
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        lines[index] = updated;
        let mut out = lines.join("\n");
        out.push('\n');
        write_atomic(path, &out)?;
        return Ok(Filed::Updated { id });
    }
    // Append a new row under the next id.
    let id = next_ki_id(&text);
    let row = render_ki_row(&id, entry, &sig, "open", "triage");
    let mut out = text;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&row);
    out.push('\n');
    write_atomic(path, &out)?;
    Ok(Filed::Appended { id })
}

/// Write `text` to `path` through a temp file and a rename.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".{}.tmp", std::process::id()));
    let temp = std::path::PathBuf::from(temp);
    fs::write(&temp, text)
        .map_err(|error| format!("could not write {}: {error}", temp.display()))?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("could not replace {}: {error}", path.display())
    })
}

/// Diagnose one session log into a [`Diagnosis`]: replay the log, read
/// its turns, its tool results, and its ending, and classify what is
/// deterministically there. A tool result that errored is a tool-class
/// failure in that tool's layer; an orderly completion is **not a
/// failure** — `Ok(None)`, so nothing is drafted and nothing is filed
/// for a healthy run; anything else that did not complete is an
/// `unknown`-class failure in the agent layer, named honestly rather
/// than forced into a class the log cannot support.
pub fn diagnose_session(path: &Path) -> Result<Option<Diagnosis>, String> {
    let records = crate::session::replay(path)
        .map_err(|error| format!("could not replay {}: {error}", path.display()))?;
    let id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
    let turns = records
        .iter()
        .filter(|record| matches!(record, Record::Assistant { .. }))
        .count();
    let tool_results: Vec<&Record> = records
        .iter()
        .filter(|record| matches!(record, Record::ToolResult { .. }))
        .collect();
    let ending = records.iter().find_map(|record| match record {
        Record::End { stop_reason, .. } => Some(stop_reason.clone()),
        _ => None,
    });
    let ending_word = ending.as_deref().unwrap_or("interrupted");
    let tool_failure = tool_results.iter().find_map(|record| match record {
        Record::ToolResult { name, output, .. } if output.starts_with("error:") => {
            Some(name.clone())
        }
        _ => None,
    });
    let (class, layer, summary) = match (&tool_failure, ending.as_deref()) {
        (Some(name), _) => (
            FailureClass::Tool,
            name.clone(),
            format!("the run hit a tool failure in `{name}` after {turns} turns"),
        ),
        // An orderly completion is not a failure: nothing to classify,
        // and nothing for a filing to file. The caller says so.
        (None, Some("completed")) => return Ok(None),
        (None, other) => (
            FailureClass::Unknown,
            "agent".to_string(),
            format!(
                "the run did not complete (ending {}) after {turns} turns",
                other.unwrap_or("interrupted")
            ),
        ),
    };
    let evidence = format!(
        "session {id}: {} records, {} tool results, ending {ending_word}",
        records.len(),
        tool_results.len(),
    );
    Ok(Some(Diagnosis { class, layer, summary, evidence }))
}

/// The evidence-first draft: a recap composed from the diagnosis and the
/// ledger rather than from narrative. It names the class and the layer,
/// carries the summary and the evidence verbatim, and cites the ledger
/// rows that already hold this failure's signature — or names the row a
/// filing would add. This is the drafted recap AC2 asks for, and it is
/// assembled from the ledger and the session log, not written by hand.
pub fn render_diagnosis(diagnosis: &Diagnosis, ledger_path: &Path) -> String {
    let sig = signature(diagnosis);
    let text = fs::read_to_string(ledger_path).unwrap_or_default();
    let cited: Vec<String> = text
        .lines()
        .filter(|line| line.contains(&format!("[sig:{sig}]")))
        .filter_map(row_id)
        .collect();
    let ledger_note = if cited.is_empty() {
        format!(
            "no ledger row for this failure yet; a filing would add {} in {}",
            next_ki_id(&text),
            ledger_path.display()
        )
    } else {
        format!("ledger row(s) {} already carry this failure's evidence", cited.join(", "))
    };
    format!(
        "## Diagnosis \u{2014} {class} failure in {layer}\n\n{summary}\n\nEvidence: {evidence}\nLedger: {ledger_note}\nSignature: {sig}\n",
        class = diagnosis.class.label(),
        layer = diagnosis.layer,
        summary = diagnosis.summary,
        evidence = diagnosis.evidence,
    )
}

// --- slice 13: the open-action-items read-back ------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, RESET, TEXT, WARN};

const CARD_WIDTH: usize = 72;

/// One open ledger row, as the read-back surfaces it: the id, the
/// severity, the issue, and the next action that stays visible until
/// the row's status leaves `open`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenItem {
    pub id: String,
    pub severity: String,
    pub issue: String,
    pub next_action: String,
}

/// The ledger's open rows, in file order: every six-cell row whose id
/// leads with `KI-` and whose status cell is `open`, with its next
/// action. This is the read-back the plan's Diagnosis clause asks for
/// — an unclosed action item stays visible until it is closed. A row
/// whose status is anything else (`closed`, a deferral, an owner's
/// own word) is not surfaced.
pub fn open_items(text: &str) -> Vec<OpenItem> {
    text.lines()
        .filter_map(row_cells)
        .filter(|cells| cells[0].starts_with("KI-"))
        .filter(|cells| cells[4].eq_ignore_ascii_case("open"))
        .map(|cells| OpenItem {
            id: cells[0].to_string(),
            severity: cells[1].to_string(),
            issue: cells[2].to_string(),
            next_action: cells[5].to_string(),
        })
        .collect()
}

/// Render the open action items as house cards: one card per open row
/// with its severity, its issue, and the next action that keeps it
/// visible. A ledger with no open rows says so; an absent or
/// unreadable ledger names itself. The surface is read-only wherever
/// it is pointed — it writes nothing.
pub fn render_open_items(ledger_path: &Path) -> String {
    let text = match fs::read_to_string(ledger_path) {
        Ok(text) => text,
        Err(error) => {
            return format!(
                "{DIM}the ledger {} could not be read: {error}{RESET}\n",
                ledger_path.display()
            );
        }
    };
    let items = open_items(&text);
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet doctor --open{RESET} {DIM}\u{2014} {} open action item{} in {}{RESET}\n\n",
        items.len(),
        plural(items.len()),
        ledger_path.display(),
    ));
    if items.is_empty() {
        out.push_str(&format!(
            "{DIM}no open rows \u{b7} every action item this ledger holds is closed{RESET}\n"
        ));
        return out;
    }
    for item in &items {
        out.push_str(&open_card(item));
    }
    out.push_str(&format!(
        "{DIM}each row stays here until its status leaves `open` \u{b7} the ledger is the human-owned system of record{RESET}\n"
    ));
    out
}

/// One open row as a card: the id and severity on the header, the
/// issue wrapped beneath, and the next action — the cell that keeps
/// the item visible.
fn open_card(item: &OpenItem) -> String {
    let mut card = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{id}{RESET} {DIM}{dashes} {WARN}{severity}{RESET} {DIM}\u{2510}{RESET}\n",
        id = item.id,
        dashes = dashes_for(&format!(" {} ", item.id), &item.severity),
        severity = item.severity,
    );
    for line in wrap(&item.issue, CARD_WIDTH - 4) {
        card.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{line}{RESET}\n"));
    }
    for line in wrap(&format!("next: {}", item.next_action), CARD_WIDTH - 4) {
        card.push_str(&format!("{DIM}\u{2502}{RESET} {WARN}{line}{RESET}\n"));
    }
    card.push_str(&format!("{DIM}\u{2514}{}\u{2518}{RESET}\n", "\u{2500}".repeat(CARD_WIDTH - 2)));
    card
}

/// Wrap `text` to `width` columns on word boundaries; a word longer
/// than the width rides its own line. A ledger's issue cell is a
/// paragraph, and a card stays readable.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// The English plural for a count.
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

    #[test]
    fn credentials_masks_but_never_hides_presence() {
        let ok = evaluate_credentials(Some("sk-1234567890abcdef"));
        assert!(ok.ok);
        assert!(ok.detail.starts_with("set (\u{2026}"));
        assert!(!ok.detail.contains("sk-1234567890"));
        let short = evaluate_credentials(Some("short"));
        assert_eq!(short.detail, "set (\u{2026})");
        let missing = evaluate_credentials(None);
        assert!(!missing.ok);
        assert!(missing.detail.contains("DEEPSEEK_API_KEY"));
    }

    #[test]
    fn credentials_mask_never_panics_on_multibyte_keys() {
        // The byte-slicing mask panicked when the 4-from-end index straddled a
        // multi-byte char (KI-010); a key is arbitrary text until the API says
        // otherwise, and doctor is the last place a panic belongs.
        let crafted = "aaaa\u{1d54f}\u{e9}";
        let check = evaluate_credentials(Some(crafted));
        assert!(check.ok);
        assert!(!check.detail.contains('\u{1d54f}'), "{}", check.detail);
        assert!(check.detail.starts_with("set (\u{2026}"));
        let long = "sk-1234567890\u{e9}";
        let check = evaluate_credentials(Some(long));
        // The wrapper is `set (…)`, so the masked tail sits just inside the paren.
        assert!(check.detail.ends_with("890\u{e9})"), "{}", check.detail);
        assert!(!check.detail.contains("sk-1234567890"));
    }

    #[test]
    fn api_endpoint_parses_hosts_and_ports() {
        assert_eq!(api_endpoint("https://api.deepseek.com"), Some(("api.deepseek.com", 443)));
        assert_eq!(api_endpoint("http://127.0.0.1:4141/v1"), Some(("127.0.0.1", 4141)));
        assert_eq!(api_endpoint("https://host:notaport"), None);
        assert_eq!(api_endpoint("ftp://host"), None);
    }

    #[test]
    fn render_aligns_names_and_verdicts() {
        let line = render(&evaluate_shell(Ok("sh")));
        assert!(line.starts_with("shell        ok    sh available"), "{line}");
        let line = render(&evaluate_shell(Err("no shell".to_string())));
        assert!(line.contains("FAIL"), "{line}");
    }

    #[test]
    fn the_calib_line_names_the_resolved_root() {
        // A scratch home with no config.json and no user home resolves to
        // the <FLEET_HOME>/logs fallback; the informational line names it
        // and says today's file is absent, without touching any check.
        let home = std::path::Path::new("/scratch/fleet-home");
        let root = crate::calib::resolve_root(home, None);
        assert_eq!(root, std::path::Path::new("/scratch/fleet-home/logs"));
        let line = render_calib(Some(&root), false);
        assert!(line.contains("calib"), "{line}");
        assert!(line.contains("/scratch/fleet-home/logs"), "{line}");
        assert!(line.contains("absent"), "{line}");
        // A present file is named present; a missing home says so plainly.
        assert!(render_calib(Some(&root), true).contains("present"));
        assert!(render_calib(None, false).contains("no home directory known"));
    }

    #[test]
    fn the_recall_check_names_the_live_path_both_ways() {
        // The vector path: the endpoint and the dimensionality the probe
        // found are named where an operator looks.
        let live = evaluate_recall(Some(("http://127.0.0.1:8901/v1/embeddings", 768)));
        assert!(live.ok);
        assert!(live.detail.contains("vector path live"), "{}", live.detail);
        assert!(live.detail.contains("768"), "{}", live.detail);
        // The degradation: one named line, never silent.
        let degraded = evaluate_recall(None);
        assert!(degraded.ok);
        assert_eq!(degraded.detail, "embeddings unavailable \u{2014} recall is keyword-only");
        // Both states pass: keyword-only recall is a working
        // configuration, and doctor is the install gate — a clean
        // machine with no local model must stay green.
        assert!(render(&degraded).starts_with("recall"), "{}", render(&degraded));
    }

    // --- slice 13: the classification ---

    fn api_error(status: u16) -> ClientError {
        ClientError::Api(crate::api::ApiError { status, message: "x".to_string() })
    }

    #[test]
    fn the_failure_classes_name_the_plans_five_words() {
        assert_eq!(FailureClass::Network.label(), "network");
        assert_eq!(FailureClass::Auth.label(), "auth");
        assert_eq!(FailureClass::Quota.label(), "quota");
        assert_eq!(FailureClass::Decode.label(), "decode");
        assert_eq!(FailureClass::Tool.label(), "tool");
    }

    #[test]
    fn a_client_error_classifies_by_its_typed_shape() {
        // Transport is network; a malformed 2xx is decode; the answered
        // statuses split by code — auth, quota, and server-side network.
        assert_eq!(classify_error(&ClientError::Network("refused".into())), FailureClass::Network);
        assert_eq!(classify_error(&ClientError::Decode("bad chunk".into())), FailureClass::Decode);
        assert_eq!(classify_error(&api_error(401)), FailureClass::Auth);
        assert_eq!(classify_error(&api_error(403)), FailureClass::Auth);
        assert_eq!(classify_error(&api_error(402)), FailureClass::Quota);
        assert_eq!(classify_error(&api_error(429)), FailureClass::Quota);
        assert_eq!(classify_error(&api_error(500)), FailureClass::Network);
    }

    #[test]
    fn a_tool_result_classifies_only_when_it_errored() {
        assert_eq!(classify_tool("error: /missing: no such file"), Some(FailureClass::Tool));
        // A denial is a decision, not a failure; a success is neither.
        assert_eq!(classify_tool("denied: the user did not approve this call."), None);
        assert_eq!(classify_tool("exit 0"), None);
    }

    // --- slice 13: the per-layer probes ---

    #[test]
    fn the_roster_layer_names_a_keyless_machine() {
        let ready = evaluate_roster(1, 6);
        assert!(ready.ok);
        assert!(ready.detail.contains("1 of 6"));
        let keyless = evaluate_roster(0, 6);
        assert!(!keyless.ok);
        assert!(keyless.detail.contains("missing its key"), "{}", keyless.detail);
        let empty = evaluate_roster(0, 0);
        assert!(!empty.ok);
        assert!(empty.detail.contains("no routing candidates"), "{}", empty.detail);
    }

    #[test]
    fn the_ledger_layer_names_an_unwritable_stream() {
        let ok = evaluate_ledger(Some(Path::new("/scratch/logs")), true);
        assert!(ok.ok);
        assert!(ok.detail.contains("writable"));
        let broken = evaluate_ledger(Some(Path::new("/scratch/logs")), false);
        assert!(!broken.ok);
        assert!(broken.detail.contains("not writable"), "{}", broken.detail);
        assert!(broken.detail.contains("/scratch/logs"), "{}", broken.detail);
        let none = evaluate_ledger(None, false);
        assert!(!none.ok);
        assert!(none.detail.contains("no calibration root"), "{}", none.detail);
    }

    #[test]
    fn an_mcp_core_layer_names_the_reason_it_did_not_resolve() {
        let ok = evaluate_mcp_core("sem1", Ok(Path::new("/services/sem1")));
        assert!(ok.ok);
        assert!(ok.name == "sem1");
        let broken = evaluate_mcp_core("gen1", Err("no mcp_server.py under /wrong".to_string()));
        assert!(!broken.ok);
        assert_eq!(broken.name, "gen1");
        assert!(broken.detail.contains("no mcp_server.py"), "{}", broken.detail);
    }

    // --- slice 13: the ledger filing ---

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-doctor-{tag}-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn diagnosis(class: FailureClass, layer: &str, summary: &str) -> Diagnosis {
        Diagnosis {
            class,
            layer: layer.to_string(),
            summary: summary.to_string(),
            evidence: "session abc: 6 records, 1 tool result".to_string(),
        }
    }

    #[test]
    fn a_diagnosis_files_then_updates_by_its_signature_and_touches_nothing_else() {
        let dir = scratch("ledger");
        let ledger = dir.join("known-issues.md");
        // A scratch ledger with a header and one unrelated human row.
        std::fs::write(
            &ledger,
            "# Known issues\n\n| ID | Severity | Issue | Evidence | Status | Next action |\n|---|---|---|---|---|---|\n| KI-001 | defect | a human row | evidence | open | keep |\n",
        )
        .unwrap();
        let entry =
            LedgerEntry { severity: "defect", diagnosis: diagnosis(FailureClass::Tool, "bash", "a tool failed") };

        // First filing appends KI-002.
        let first = file_ledger_row(&ledger, &entry).unwrap();
        assert_eq!(first, Filed::Appended { id: "KI-002".to_string() });
        let after_first = std::fs::read_to_string(&ledger).unwrap();
        assert!(after_first.contains("| KI-002 |"), "{after_first}");
        assert!(after_first.contains(&format!("[sig:{}]", signature(&entry.diagnosis))));

        // The same failure re-diagnosed updates KI-002, never a KI-003.
        let second = file_ledger_row(&ledger, &entry).unwrap();
        assert_eq!(second, Filed::Updated { id: "KI-002".to_string() });
        let after_second = std::fs::read_to_string(&ledger).unwrap();
        assert_eq!(after_second.matches("| KI-002 |").count(), 1, "{after_second}");
        assert!(!after_second.contains("| KI-003 |"), "{after_second}");

        // A different failure appends KI-003 beside the untouched human row.
        let other = LedgerEntry {
            severity: "gap",
            diagnosis: diagnosis(FailureClass::Auth, "roster", "a key is missing"),
        };
        let third = file_ledger_row(&ledger, &other).unwrap();
        assert_eq!(third, Filed::Appended { id: "KI-003".to_string() });
        let after_third = std::fs::read_to_string(&ledger).unwrap();
        // KI-001 is byte-for-byte the human's row, untouched.
        assert!(
            after_third.contains("| KI-001 | defect | a human row | evidence | open | keep |"),
            "{after_third}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_signature_is_stable_for_the_same_failure() {
        let a = diagnosis(FailureClass::Network, "api", "the endpoint is unreachable");
        let b = diagnosis(FailureClass::Network, "api", "the endpoint is unreachable");
        let c = diagnosis(FailureClass::Auth, "api", "the endpoint is unreachable");
        assert_eq!(signature(&a), signature(&b));
        assert_ne!(signature(&a), signature(&c));
    }

    #[test]
    fn a_session_log_diagnoses_its_tool_failure_by_class_and_layer() {
        // A failed run's log: a tool call whose result errored, and no
        // `end` (interrupted). The diagnosis names the class (tool), the
        // layer (the failing tool), and cites the log's records — the
        // classified diagnosis AC2 asks for, composed from the log.
        let dir = scratch("diagnose-session");
        let log = dir.join("20261014T120000Z-1.jsonl");
        std::fs::write(
            &log,
            concat!(
                "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"2026-10-14T12:00:00Z\"}\n",
                "{\"type\":\"user\",\"content\":\"read the missing file\"}\n",
                "{\"type\":\"assistant\",\"tool_calls\":[{\"id\":\"c1\",\"type\":\"function\",\"function\":{\"name\":\"read_file\",\"arguments\":\"{}\"}}]}\n",
                "{\"type\":\"tool_result\",\"call_id\":\"c1\",\"name\":\"read_file\",\"output\":\"error: /x: no such file\",\"truncated\":false,\"approved\":true}\n",
            ),
        )
        .unwrap();
        let diagnosis = diagnose_session(&log).unwrap().expect("a tool failure diagnoses");
        assert_eq!(diagnosis.class, FailureClass::Tool);
        assert_eq!(diagnosis.layer, "read_file");
        assert!(diagnosis.summary.contains("read_file"), "{}", diagnosis.summary);
        assert!(diagnosis.evidence.contains("ending interrupted"), "{}", diagnosis.evidence);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_completed_session_is_not_a_failure_and_files_nothing() {
        // The named finding: `diagnose_session` used to return an
        // `unknown`-class Diagnosis for a completed run ("the run
        // completed in N turns"), so `--diagnose` on a healthy session
        // filed a nonsense row into the human-owned ledger. An orderly
        // completion is now `Ok(None)` — nothing classified, nothing
        // filed, and the surface says so.
        let dir = scratch("diagnose-healthy");
        let log = dir.join("20261014T120000Z-2.jsonl");
        std::fs::write(
            &log,
            concat!(
                "{\"type\":\"start\",\"v\":1,\"model\":\"m\",\"created\":\"2026-10-14T12:00:00Z\"}\n",
                "{\"type\":\"user\",\"content\":\"say hello\"}\n",
                "{\"type\":\"assistant\",\"content\":\"hello\"}\n",
                "{\"type\":\"end\",\"stop_reason\":\"completed\",\"turns\":1}\n",
            ),
        )
        .unwrap();
        let diagnosed = diagnose_session(&log).unwrap();
        assert!(diagnosed.is_none(), "a completed run is not a failure: {diagnosed:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_update_preserves_a_human_closed_rows_status_and_next_action() {
        // The named finding: the update path re-rendered the matched
        // row with status hardcoded `open` and next-action hardcoded
        // `triage`, so re-diagnosing a row a human closed silently
        // reopened it and discarded the human's next action. The
        // update now refreshes the issue and the evidence and leaves
        // the owner's two cells exactly as they are.
        let dir = scratch("ledger-closed-row");
        let ledger = dir.join("known-issues.md");
        std::fs::write(
            &ledger,
            "# Known issues\n\n| ID | Severity | Issue | Evidence | Status | Next action |\n|---|---|---|---|---|---|\n| KI-001 | defect | a human row | evidence | open | keep |\n",
        )
        .unwrap();
        let entry =
            LedgerEntry { severity: "defect", diagnosis: diagnosis(FailureClass::Tool, "bash", "a tool failed") };
        // File it once: KI-002, open, triage.
        assert_eq!(
            file_ledger_row(&ledger, &entry).unwrap(),
            Filed::Appended { id: "KI-002".to_string() }
        );
        // The human closes the row and writes their own next action.
        let filed = std::fs::read_to_string(&ledger).unwrap();
        let closed = close_row(&filed, "KI-002", "closed", "fixed in cbc5213");
        std::fs::write(&ledger, closed).unwrap();

        // The same failure re-diagnosed: the row updates, and the
        // human's status and next action survive.
        let second = file_ledger_row(&ledger, &entry).unwrap();
        assert_eq!(second, Filed::Updated { id: "KI-002".to_string() });
        let after = std::fs::read_to_string(&ledger).unwrap();
        assert!(after.contains("| KI-002 |"), "{after}");
        assert!(!after.contains("| KI-003 |"), "{after}");
        assert!(after.contains("| closed |"), "{after}");
        assert!(after.contains("| fixed in cbc5213 |"), "{after}");
        // The evidence cell refreshed with the new diagnosis's own.
        assert!(after.contains("session abc: 6 records, 1 tool result"), "{after}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Close one row's status and next-action cells, the way a human
    /// editing the ledger would: the line carrying `id` is re-split,
    /// the two cells replaced, and the line re-joined.
    fn close_row(text: &str, id: &str, status: &str, next_action: &str) -> String {
        text.lines()
            .map(|line| {
                let Some(mut cells) = row_cells(line) else { return line.to_string() };
                if cells[0] != id {
                    return line.to_string();
                }
                cells[4] = status;
                cells[5] = next_action;
                format!("| {} | {} | {} | {} | {} | {} |", cells[0], cells[1], cells[2], cells[3], cells[4], cells[5])
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    #[test]
    fn the_drafted_recap_cites_the_ledger_row_it_would_file() {
        let dir = scratch("draft");
        let ledger = dir.join("known-issues.md");
        std::fs::write(
            &ledger,
            "# Known issues\n\n| ID | Severity | Issue | Evidence | Status | Next action |\n|---|---|---|---|---|---|\n| KI-007 | environmental | a row | e | open | a |\n",
        )
        .unwrap();
        let d = diagnosis(FailureClass::Decode, "api", "a bad SSE chunk");
        let before = render_diagnosis(&d, &ledger);
        assert!(before.contains("decode failure in api"), "{before}");
        assert!(before.contains("a bad SSE chunk"), "{before}");
        // No row yet: the draft names the row a filing would add (KI-008,
        // one past the KI-007 the scratch ledger already holds).
        assert!(before.contains("a filing would add KI-008"), "{before}");
        // After filing, the draft cites the row that now carries it.
        let entry = LedgerEntry { severity: "defect", diagnosis: d.clone() };
        file_ledger_row(&ledger, &entry).unwrap();
        let after = render_diagnosis(&d, &ledger);
        assert!(after.contains("KI-008 already carry"), "{after}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- slice 13: the open-action-items read-back ---

    /// A scratch ledger holding two open rows, one closed row, and the
    /// header and separator lines that are not rows.
    fn scratch_ledger(tag: &str) -> std::path::PathBuf {
        let dir = scratch(tag);
        let ledger = dir.join("known-issues.md");
        std::fs::write(
            &ledger,
            concat!(
                "# Known issues\n\n",
                "| ID | Severity | Issue | Evidence | Status | Next action |\n",
                "|---|---|---|---|---|---|\n",
                "| KI-104 | defect | the cadence fire lacked the panic guard | `src/cron.rs run_eval_if_due`, 2026-10-14 | open | wrap the fire in CatchUnwind |\n",
                "| KI-105 | risk | the eval latch is in-memory | `src/cron.rs run_eval_if_due`, 2026-10-14 | closed | persist the minute beside jobs.json |\n",
                "| KI-106 | gap | the read-back was unimplemented | the plan's slice-13 clause, 2026-10-14 | open | ship `fleet doctor --open` |\n",
            ),
        )
        .unwrap();
        ledger
    }

    #[test]
    fn the_read_back_surfaces_the_open_rows_with_their_next_actions() {
        let ledger = scratch_ledger("open-items");
        let items = open_items(&std::fs::read_to_string(&ledger).unwrap());
        // Two open rows in file order; the closed one is not
        // surfaced, and the header and separator are not rows.
        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(items[0].id, "KI-104");
        assert_eq!(items[0].severity, "defect");
        assert_eq!(items[0].next_action, "wrap the fire in CatchUnwind");
        assert_eq!(items[1].id, "KI-106");
        assert_eq!(items[1].next_action, "ship `fleet doctor --open`");

        // The cards carry the id, the issue, and the next action — the
        // cell that keeps the item visible — and never the closed row.
        let rendered = render_open_items(&ledger);
        assert!(rendered.contains("fleet doctor --open"), "{rendered}");
        assert!(rendered.contains("2 open action items"), "{rendered}");
        assert!(rendered.contains("KI-104"), "{rendered}");
        assert!(rendered.contains("wrap the fire in CatchUnwind"), "{rendered}");
        assert!(rendered.contains("KI-106"), "{rendered}");
        assert!(!rendered.contains("KI-105"), "{rendered}");
        let _ = std::fs::remove_file(&ledger);
    }

    #[test]
    fn a_ledger_with_no_open_rows_or_no_ledger_names_itself() {
        let dir = scratch("open-items-none");
        let ledger = dir.join("known-issues.md");
        std::fs::write(
            &ledger,
            "# Known issues\n\n| ID | Severity | Issue | Evidence | Status | Next action |\n|---|---|---|---|---|---|\n| KI-001 | defect | a row | e | closed | done |\n",
        )
        .unwrap();
        let rendered = render_open_items(&ledger);
        assert!(rendered.contains("no open rows"), "{rendered}");
        // An absent ledger is named, never an empty silence.
        let absent = dir.join("absent.md");
        let rendered = render_open_items(&absent);
        assert!(rendered.contains("could not be read"), "{rendered}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
