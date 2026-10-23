//! The MCP server registry: the fleet home's `mcp.json`, the file that
//! carries the MCP servers the owner configured — imported from another
//! harness's config or written by hand — beside the vendored lanes.
//!
//! The split is D1 of the registry plan: `lanes.json` is the fleet-owned
//! lane configuration and refuses credential-shaped names outright
//! (`set_lane`), while a server registry's whole job is to record which
//! env names a server needs. So this file's schema inverts the guard:
//! **a name is what is stored, a value has no field to live in** (D1a).
//! At spawn each name resolves through the chain the lanes use — process
//! env, then the fleet home's dotenv/vault — and an unresolved name is a
//! named probe failure, never a silent empty value.
//!
//! The registry never spawns anything: it is a document with validation.
//! The run wiring pre-joins its enabled servers (increment 3), the probe
//! writes `last_probe` (increment 4), and the settings card reads it
//! through the bridge (increment 5).
//!
//! Reserved names (D8): the six lanes, because the model-facing
//! namespace is `mcp__<name>__<tool>` and a lane-named server would be
//! indistinguishable from the lane; and the four built-in tools, because
//! a tool name must never shadow one. Inside one server, short-name
//! collisions resolve through `tools::unique_name` at declaration time,
//! so no two declarations ever share a name.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

/// The registry file's name, beside `lanes.json` in the fleet home.
pub const MCP_FILE: &str = "mcp.json";

/// The path the registry lives at.
pub fn servers_path(home: &Path) -> PathBuf {
    home.join(MCP_FILE)
}

/// A server the registry may not be named: the lanes (the model calls
/// `mcp__<lane>__<tool>` and a lane-named server would collide with the
/// lane's own namespace) and the built-in tools (a declaration must
/// never shadow one).
pub fn reserved_names() -> Vec<String> {
    let mut names: Vec<String> = crate::lanes::LANES
        .iter()
        .map(|lane| lane.name.to_string())
        .collect();
    names.extend(
        crate::tools::ALL_TOOLS
            .iter()
            .map(|tool| tool.name().to_string()),
    );
    names
}

/// One configured server, validated on the way in and out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEntry {
    /// The program to run, resolved through PATH.
    pub program: String,
    /// The program's arguments.
    pub args: Vec<String>,
    /// The env names the server needs, never their values.
    pub env_names: Vec<String>,
    /// Whether a run pre-joins this server. A configured-but-disabled
    /// server stays in the registry and out of every run.
    pub enabled: bool,
    /// Where an imported server came from, when it was imported.
    pub source: Option<Source>,
    /// The last probe's outcome, when one ran (increment 4 writes it).
    pub last_probe: Option<Probe>,
}

/// Where an imported server came from. The harness is one of the four
/// known foreign config files; the date is the import's, not a file
/// timestamp's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub harness: String,
    pub imported: String,
}

/// One probe's outcome, as the health path wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub at: String,
    pub ok: bool,
    pub server_name: String,
    pub server_version: String,
    pub tools: usize,
    /// The failure's reason when `ok` is false — the child's stderr
    /// tail, the lane probe's way.
    pub reason: String,
}

/// The registry document, read whole. A missing file is an empty
/// registry, the way `read_lanes` reads a missing `lanes.json`; an
/// unreadable or malformed one is a named error the caller surfaces.
pub fn read_registry(home: &Path) -> Result<Value, String> {
    let path = servers_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({}));
        }
        Err(error) => return Err(format!("{} is unreadable: {error}", path.display())),
    };
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }
    Ok(value)
}

/// The configured servers, in name order: one `(name, entry)` per entry
/// the document carries. A malformed entry is a named error — the
/// registry is owner-written or imported, never hand-edited mid-run, so
/// a bad shape is reported rather than skipped.
pub fn read_servers(home: &Path) -> Result<Vec<(String, ServerEntry)>, String> {
    let document = read_registry(home)?;
    let mut servers = Vec::new();
    let empty = Map::new();
    let entries = document
        .get("servers")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for (name, config) in entries {
        servers.push((name.clone(), parse_server(name, config)?));
    }
    servers.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(servers)
}

/// One server's entry, or `None` when the registry does not name it.
pub fn read_server(home: &Path, name: &str) -> Result<Option<ServerEntry>, String> {
    let document = read_registry(home)?;
    match document
        .get("servers")
        .and_then(|servers| servers.get(name))
    {
        Some(config) => Ok(Some(parse_server(name, config)?)),
        None => Ok(None),
    }
}

/// The enabled servers' launch specs, keyed by name: what a run seeds
/// into `AgentOptions.lane_specs` so the roster pre-joins them before
/// the first request. Each env name resolves through the chain the
/// lanes use — the process env first, then the home's dotenv — and an
/// unresolved name fails the run start by name, the probe's way: a
/// server whose env is missing must not half-join.
///
/// `dotenv` carries the home's own `.env` as name → value, already
/// loaded by the caller; the vault's names resolve through it too, the
/// way `settings::dotenv` is assembled.
pub fn enabled_specs(
    home: &Path,
    dotenv: &std::collections::BTreeMap<String, String>,
) -> Result<std::collections::HashMap<String, crate::mcp::ServerSpec>, String> {
    let mut specs = std::collections::HashMap::new();
    for (name, entry) in read_servers(home)? {
        if !entry.enabled {
            continue;
        }
        let mut env = Vec::new();
        for env_name in &entry.env_names {
            let value = std::env::var(env_name)
                .ok()
                .filter(|value| !value.is_empty())
                .or_else(|| dotenv.get(env_name).cloned())
                .ok_or_else(|| {
                    format!(
                        "server `{name}` names env `{env_name}`, which resolves to nothing: \
                         set it in the environment or the fleet home's .env"
                    )
                })?;
            env.push((env_name.clone(), value));
        }
        specs.insert(
            name.clone(),
            crate::mcp::ServerSpec {
                program: entry.program,
                args: entry.args,
                env,
                // No passthrough: the child gets the base plus the
                // resolved env entries and nothing else, the same
                // minimal child KI-031 already gives a lane.
                passthrough: Vec::new(),
                cwd: None,
            },
        );
    }
    Ok(specs)
}

/// Validate one registry entry and return it parsed. The write path and
/// the read path share this, so a file a writer produced always reads
/// back, and a hand-edited file that lost the shape is named at read.
pub(crate) fn parse_server(name: &str, config: &Value) -> Result<ServerEntry, String> {
    if name_reserved(name) {
        return Err(format!(
            "`{name}` is reserved: the registry cannot carry a lane's or a built-in \
             tool's name, because the model-facing namespace is mcp__<server>__<tool>"
        ));
    }
    if !name_valid(name) {
        return Err(format!(
            "`{name}` is not a valid server name: lowercase letters, digits, `-` and `_`"
        ));
    }
    let Some(object) = config.as_object() else {
        return Err(format!("server `{name}` carries an object"));
    };
    let program = object
        .get("program")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|program| !program.is_empty())
        .ok_or_else(|| format!("server `{name}` names its program"))?
        .to_string();
    let mut args = Vec::new();
    if let Some(list) = object.get("args") {
        let Some(list) = list.as_array() else {
            return Err(format!("server `{name}`: `args` is a list of strings"));
        };
        for arg in list {
            let Some(arg) = arg.as_str() else {
                return Err(format!("server `{name}`: `args` is a list of strings"));
            };
            args.push(arg.to_string());
        }
    }
    let mut env_names = Vec::new();
    if let Some(list) = object.get("env_names") {
        let Some(list) = list.as_array() else {
            return Err(format!("server `{name}`: `env_names` is a list of strings"));
        };
        for env_name in list {
            let Some(env_name) = env_name.as_str() else {
                return Err(format!("server `{name}`: `env_names` is a list of strings"));
            };
            if !env_name_valid(env_name) {
                return Err(format!(
                    "server `{name}`: `{env_name}` is not a valid env name; and a value has \
                     no field in this registry — names resolve from the environment at spawn"
                ));
            }
            env_names.push(env_name.to_string());
        }
    }
    if object.get("env").is_some() {
        return Err(format!(
            "server `{name}`: there is no `env` field — the registry carries env NAMES \
             only; write `env_names` and let the values resolve at spawn"
        ));
    }
    let enabled = object
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let source = match object.get("source") {
        Some(source) => {
            let harness = source
                .get("harness")
                .and_then(Value::as_str)
                .filter(|harness| !harness.is_empty())
                .ok_or_else(|| format!("server `{name}`: `source.harness` is a string"))?;
            let imported = source
                .get("imported")
                .and_then(Value::as_str)
                .filter(|imported| !imported.is_empty())
                .ok_or_else(|| format!("server `{name}`: `source.imported` is a string"))?;
            Some(Source {
                harness: harness.to_string(),
                imported: imported.to_string(),
            })
        }
        None => None,
    };
    let last_probe = match object.get("last_probe") {
        Some(probe) => {
            let at = probe
                .get("at")
                .and_then(Value::as_str)
                .filter(|at| !at.is_empty())
                .ok_or_else(|| format!("server `{name}`: `last_probe.at` is a string"))?;
            let ok = probe
                .get("ok")
                .and_then(Value::as_bool)
                .ok_or_else(|| format!("server `{name}`: `last_probe.ok` is a boolean"))?;
            let server_name = probe
                .get("server_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let server_version = probe
                .get("server_version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let tools = probe.get("tools").and_then(Value::as_u64).unwrap_or(0) as usize;
            let reason = probe
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            Some(Probe {
                at: at.to_string(),
                ok,
                server_name,
                server_version,
                tools,
                reason,
            })
        }
        None => None,
    };
    Ok(ServerEntry {
        program,
        args,
        env_names,
        enabled,
        source,
        last_probe,
    })
}

/// The registry to disk: one line of JSON, the way the lane writer
/// keeps it, through the file lock the home's writers share.
pub(crate) fn write_registry(home: &Path, document: &Value) -> Result<(), String> {
    let path = servers_path(home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} is unwritable: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(document).expect("a registry document serializes");
    let _lock = crate::filelock::FileLock::take(&crate::filelock::FileLock::beside(&path));
    std::fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("{} is unwritable: {error}", path.display()))
}

/// The entry as the document stores it. `env_names` is the only env
/// field the shape has — there is nowhere for a value to go (D1a).
pub(crate) fn entry_value(entry: &ServerEntry) -> Value {
    let mut object = Map::new();
    object.insert("program".to_string(), Value::String(entry.program.clone()));
    object.insert(
        "args".to_string(),
        Value::Array(entry.args.iter().cloned().map(Value::String).collect()),
    );
    object.insert(
        "env_names".to_string(),
        Value::Array(entry.env_names.iter().cloned().map(Value::String).collect()),
    );
    object.insert("enabled".to_string(), Value::Bool(entry.enabled));
    if let Some(source) = &entry.source {
        object.insert(
            "source".to_string(),
            json!({"harness": source.harness, "imported": source.imported}),
        );
    }
    if let Some(probe) = &entry.last_probe {
        object.insert(
            "last_probe".to_string(),
            json!({
                "at": probe.at,
                "ok": probe.ok,
                "server_name": probe.server_name,
                "server_version": probe.server_version,
                "tools": probe.tools,
                "reason": probe.reason,
            }),
        );
    }
    Value::Object(object)
}

/// Whether the name is one the registry may not carry.
fn name_reserved(name: &str) -> bool {
    reserved_names().iter().any(|reserved| reserved == name)
}

/// A server name is lowercase-kebab-or-snake: it rides the
/// `mcp__<server>__<tool>` namespace, so it must not carry characters
/// that would make one server's name ambiguous with another's tools.
fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// An env name is the usual shell shape: uppercase letters, digits,
/// underscores, not starting with a digit.
fn env_name_valid(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

// ------------------------------------------------------------- the cards

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m"; // cyan
    pub const OK: &str = "\u{1b}[32m"; // green
    pub const ERROR: &str = "\u{1b}[31m"; // red
    pub const DIM: &str = "\u{1b}[90m"; // dark gray
    pub const TEXT: &str = "\u{1b}[37m"; // gray
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

/// The card width, in characters — the lanes' card width, so the two
/// surfaces read as one family.
const CARD_WIDTH: usize = 72;

/// Render the configured servers as cards: a status dot, the shape, the
/// enabled state, the tool count from the last probe (or "never
/// probed"), the launch command, and — for an imported server — the
/// harness it came from. Never a plain-text table. `now` stamps the
/// footer, and `scan_rows` (the discovery increment) appends nothing
/// here: the list is the registry, not the machine.
pub fn render(servers: &[(String, ServerEntry)]) -> String {
    use palette::*;
    let mut out = String::new();
    if servers.is_empty() {
        out.push_str(&format!(
            "{BOLD}fleet mcp{RESET} {DIM}— the MCP server registry{RESET}\n\n"
        ));
        out.push_str(&render_empty());
        return out;
    }
    out.push_str(&format!(
        "{BOLD}fleet mcp{RESET} {DIM}— the {} configured server{}, stdio only{RESET}\n\n",
        servers.len(),
        plural(servers.len()),
    ));
    for (name, entry) in servers {
        out.push_str(&render_card(name, entry));
    }
    let enabled = servers.iter().filter(|(_, e)| e.enabled).count();
    out.push_str(&format!(
        "\n{DIM}{} server{} · {enabled} enabled · a run pre-joins the enabled \
         ones before its first request{RESET}\n",
        servers.len(),
        plural(servers.len()),
    ));
    out
}

/// The empty state is a card, not a bare table, and it offers the way
/// in (C1).
fn render_empty() -> String {
    use palette::*;
    let mut out = String::new();
    out.push_str(&format!(
        "{DIM}┌─{RESET} {}{DIM} ┐{RESET}\n",
        "no MCP servers configured yet"
    ));
    out.push_str(&format!(
        "{DIM}│{RESET} {TEXT}The registry is the fleet home's mcp.json.{RESET}\n"
    ));
    out.push_str(&format!(
        "{DIM}│{RESET} {TEXT}Run `fleet mcp scan` to discover this machine's{RESET}\n"
    ));
    out.push_str(&format!(
        "{DIM}│{RESET} {TEXT}other harnesses' servers, or write one by hand.{RESET}\n"
    ));
    out.push_str(&format!("{DIM}└───{RESET}\n"));
    out
}

/// One server's card, in the lanes' visual grammar.
fn render_card(name: &str, entry: &ServerEntry) -> String {
    use palette::*;
    let (dot_color, state) = match (&entry.last_probe, entry.enabled) {
        (Some(probe), _) if probe.ok => (OK, "probed ok"),
        (Some(_), _) => (ERROR, "last probe failed"),
        (None, true) => (DIM, "enabled · never probed"),
        (None, false) => (DIM, "disabled"),
    };
    let header_right = format!("{dot_color}{state}{RESET}");
    let header = format!(
        "{DIM}┌─{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {header_right} {DIM}┐{RESET}",
        dashes = dashes_for(name, &format!(" {state} ")),
    );
    let mut lines = vec![format!(
        "{DIM}│{RESET} {TEXT}stdio · {} · {} env name{}{RESET}",
        if entry.enabled { "enabled" } else { "disabled" },
        entry.env_names.len(),
        plural(entry.env_names.len()),
    )];
    if let Some(probe) = &entry.last_probe {
        let line = if probe.ok {
            format!(
                "{} v{} · {} tool{} · probed {}",
                probe.server_name,
                probe.server_version,
                probe.tools,
                plural(probe.tools),
                probe.at,
            )
        } else {
            format!("probe failed {}: {}", probe.at, probe.reason)
        };
        for (index, chunk) in wrap_detail(&line, CARD_WIDTH - 4).into_iter().enumerate() {
            let indent = if index == 0 { "" } else { "  " };
            lines.push(format!("{DIM}│{RESET} {DIM}{indent}{chunk}{RESET}"));
        }
    } else {
        lines.push(format!("{DIM}│{RESET} {DIM}never probed{RESET}"));
    }
    let launch = if entry.args.is_empty() {
        entry.program.clone()
    } else {
        format!("{} {}", entry.program, entry.args.join(" "))
    };
    for chunk in wrap_detail(&launch, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}│{RESET} {DIM}{chunk}{RESET}"));
    }
    for env_name in &entry.env_names {
        lines.push(format!(
            "{DIM}│{RESET} {TEXT}env {env_name}{RESET} {DIM}(resolves at spawn){RESET}"
        ));
    }
    if let Some(source) = &entry.source {
        lines.push(format!(
            "{DIM}│{RESET} {TEXT}imported from {}{}, {imported}{RESET}",
            source.harness,
            if entry.enabled { "" } else { " · disabled" },
            imported = source.imported,
        ));
    }
    let mut out = header;
    out.push('\n');
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&format!("{DIM}└───{RESET}\n"));
    out
}

/// The dash run that pads a card header so its right edge aligns, the
/// lanes' own helper.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "─".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// Hard-wrap into the card width, word-aware — the shared wrapper.
fn wrap_detail(text: &str, width: usize) -> Vec<String> {
    crate::skills::wrap_detail(text, width)
}

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
    use crate::settings::{remove_server, set_server, set_server_enabled};

    /// The env-mutating tests serialize on this, the lanes' own pattern:
    /// `cargo test` runs a module's tests on threads, and a `set_var` in
    /// one must not be visible to another mid-flight.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Set `name` for the duration of the guard, restoring whatever was
    /// there before — including its absence — on the way out.
    struct EnvVar {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVar {
        fn set(name: &str, value: &str) -> EnvVar {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            EnvVar {
                name: name.to_string(),
                previous,
            }
        }
    }

    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(&self.name, value),
                None => std::env::remove_var(&self.name),
            }
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-mcp-registry-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry_json() -> Value {
        json!({
            "program": "npx",
            "args": ["-y", "@modelcontextprotocol/server-time"],
            "env_names": [],
            "enabled": true,
        })
    }

    #[test]
    fn a_server_writes_and_reads_back_round_trip() {
        let home = scratch("round-trip");
        set_server(&home, "time", &entry_json()).expect("writes");
        let servers = read_servers(&home).expect("reads");
        assert_eq!(servers.len(), 1);
        let (name, entry) = &servers[0];
        assert_eq!(name, "time");
        assert_eq!(entry.program, "npx");
        assert_eq!(entry.args, vec!["-y", "@modelcontextprotocol/server-time"]);
        assert!(entry.enabled);
        assert!(entry.source.is_none());
        assert!(entry.last_probe.is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_lanes_and_the_builtins_are_reserved() {
        let home = scratch("reserved");
        for name in reserved_names() {
            let error = set_server(&home, &name, &entry_json()).expect_err("refused");
            assert!(error.contains("reserved"), "{error}");
        }
        // The refusal is about the name, not the shape: a valid entry
        // under a free name writes beside the refusals.
        set_server(&home, "time", &entry_json()).expect("a free name writes");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_name_with_uppercase_or_a_slash_is_refused() {
        let home = scratch("name-shape");
        for name in ["GitHub", "my server", "a/b", ""] {
            let error = set_server(&home, name, &entry_json()).expect_err("refused");
            assert!(error.contains("not a valid server name"), "{error}");
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_env_value_has_nowhere_to_go() {
        let home = scratch("no-values");
        let with_env = json!({
            "program": "python3",
            "args": ["-m", "weather"],
            "env": {"OWM_API_KEY": "sk-marker-value"},
            "enabled": true,
        });
        let error = set_server(&home, "weather", &with_env).expect_err("refused");
        assert!(error.contains("env NAMES only"), "{error}");
        // The file was never written.
        assert!(!servers_path(&home).exists(), "nothing landed");
        // Even a hand-edited file with a value is refused at read.
        std::fs::write(
            servers_path(&home),
            r#"{"servers": {"weather": {"program": "python3", "env": {"OWM_API_KEY": "x"}}}}"#,
        )
        .unwrap();
        let error = read_servers(&home).expect_err("refused at read");
        assert!(error.contains("env NAMES only"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_env_name_must_be_a_name() {
        let home = scratch("env-names");
        let bad = json!({
            "program": "python3",
            "env_names": ["not a name"],
            "enabled": true,
        });
        let error = set_server(&home, "weather", &bad).expect_err("refused");
        assert!(error.contains("not a valid env name"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn removing_an_unknown_server_names_what_is_configured() {
        let home = scratch("remove");
        set_server(&home, "time", &entry_json()).expect("writes");
        let error = remove_server(&home, "weather").expect_err("refused");
        assert!(error.contains("`time`"), "{error}");
        assert!(error.contains("not configured"), "{error}");
        remove_server(&home, "time").expect("the known one removes");
        assert!(read_servers(&home).expect("reads").is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn enable_and_disable_flip_in_place_and_keep_the_rest() {
        let home = scratch("enable");
        let full = json!({
            "program": "npx",
            "args": ["-y", "server-weather"],
            "env_names": ["OWM_API_KEY"],
            "enabled": true,
            "source": {"harness": "claude-desktop", "imported": "2026-10-23"},
        });
        set_server(&home, "weather", &full).expect("writes");
        set_server_enabled(&home, "weather", false).expect("disables");
        let (_, entry) = &read_servers(&home).expect("reads")[0];
        assert!(!entry.enabled, "flipped off");
        assert_eq!(entry.env_names, vec!["OWM_API_KEY"], "the rest is kept");
        assert_eq!(
            entry.source.as_ref().expect("kept").harness,
            "claude-desktop"
        );
        set_server_enabled(&home, "weather", true).expect("re-enables");
        let (_, entry) = &read_servers(&home).expect("reads")[0];
        assert!(entry.enabled);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_missing_file_is_an_empty_registry() {
        let home = scratch("empty");
        assert!(read_servers(&home).expect("reads").is_empty());
        assert!(read_server(&home, "time").expect("reads").is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn enabled_specs_resolve_names_through_the_env_and_the_dotenv() {
        let home = scratch("specs");
        let _held = ENV_LOCK.lock().unwrap();
        let _own = EnvVar::set("FLEET_TEST_MARKER_KEY", "from-env");
        let weather = json!({
            "program": "python3",
            "args": ["-m", "weather"],
            "env_names": ["FLEET_TEST_MARKER_KEY", "FLEET_TEST_MARKER_MISSING"],
            "enabled": true,
        });
        set_server(&home, "weather", &weather).expect("writes");
        let mut dotenv = std::collections::BTreeMap::new();
        dotenv.insert(
            "FLEET_TEST_MARKER_MISSING".to_string(),
            "from-dotenv".to_string(),
        );
        let specs = enabled_specs(&home, &dotenv).expect("resolves");
        let spec = &specs["weather"];
        assert_eq!(spec.program, "python3");
        assert_eq!(spec.env[0].0, "FLEET_TEST_MARKER_KEY");
        assert_eq!(spec.env[0].1, "from-env", "the process env wins");
        assert_eq!(spec.env[1].1, "from-dotenv", "the home's dotenv answers");
        // An unresolved name fails by name.
        let specs = enabled_specs(&home, &std::collections::BTreeMap::new());
        let error = specs.expect_err("refused");
        assert!(error.contains("FLEET_TEST_MARKER_MISSING"), "{error}");
        assert!(error.contains("resolves to nothing"), "{error}");
        // A disabled server is absent from the specs.
        set_server_enabled(&home, "weather", false).expect("disables");
        let specs = enabled_specs(&home, &dotenv).expect("resolves");
        assert!(specs.is_empty(), "disabled means absent");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn servers_come_back_in_name_order() {
        let home = scratch("order");
        set_server(&home, "zeta", &entry_json()).expect("writes");
        set_server(&home, "alpha", &entry_json()).expect("writes");
        let servers = read_servers(&home).expect("reads");
        let names: Vec<&str> = servers.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "zeta"]);
        let _ = std::fs::remove_dir_all(&home);
    }
}
