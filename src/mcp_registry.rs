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
        // No passthrough: the child gets the base plus the resolved env
        // entries and nothing else, the same minimal child KI-031
        // already gives a lane.
        let mut spec = spec_from_entry(&entry, dotenv)
            .map_err(|reason| format!("server `{name}`: {reason}"))?;
        spec.program = entry.program.clone();
        spec.args = entry.args.clone();
        specs.insert(name.clone(), spec);
    }
    Ok(specs)
}

/// Validate one registry entry and return it parsed. The write path and
/// the read path share this, so a file a writer produced always reads
/// back, and a hand-edited file that lost the shape is named at read.
/// One probe of one configured server: spawn, initialize, list its
/// tools, report the server's self-reported name and version, tear down.
/// Every failure is named, and a spawn or handshake failure carries the
/// child's stderr tail the way the lane probe does (C6) — the reason is
/// where a Python import traceback lands, at the end.
pub async fn probe_server(
    home: &Path,
    name: &str,
    dotenv: &std::collections::BTreeMap<String, String>,
) -> Result<Probe, String> {
    let entry = read_server(home, name)?.ok_or_else(|| format!("`{name}` is not configured"))?;
    let spec = match spec_from_entry(&entry, dotenv) {
        Ok(spec) => spec,
        Err(reason) => {
            return Ok(Probe {
                at: crate::boards::now_stamp(),
                ok: false,
                server_name: String::new(),
                server_version: String::new(),
                tools: 0,
                reason,
            });
        }
    };
    let server = match crate::mcp::McpServer::spawn(&spec) {
        Ok(server) => server,
        Err(error) => {
            return Ok(Probe {
                at: crate::boards::now_stamp(),
                ok: false,
                server_name: String::new(),
                server_version: String::new(),
                tools: 0,
                reason: format!("could not start: {error}"),
            });
        }
    };
    let info = match server.initialize().await {
        Ok(info) => info,
        Err(error) => {
            let _ = server.close().await;
            return Ok(Probe {
                at: crate::boards::now_stamp(),
                ok: false,
                server_name: String::new(),
                server_version: String::new(),
                tools: 0,
                reason: format!("could not initialize: {error}"),
            });
        }
    };
    let tools = match server.list_tools().await {
        Ok(tools) => tools.len(),
        Err(error) => {
            let _ = server.close().await;
            return Ok(Probe {
                at: crate::boards::now_stamp(),
                ok: false,
                server_name: info.server_name,
                server_version: info.server_version,
                tools: 0,
                reason: format!("could not list its tools: {error}"),
            });
        }
    };
    let _ = server.close().await;
    Ok(Probe {
        at: crate::boards::now_stamp(),
        ok: true,
        server_name: info.server_name,
        server_version: info.server_version,
        tools,
        reason: String::new(),
    })
}

/// The launch spec one entry builds, with its env names resolved the way
/// `enabled_specs` resolves them — the probe proves the whole chain.
fn spec_from_entry(
    entry: &ServerEntry,
    dotenv: &std::collections::BTreeMap<String, String>,
) -> Result<crate::mcp::ServerSpec, String> {
    let mut env = Vec::new();
    for env_name in &entry.env_names {
        let value = std::env::var(env_name)
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| dotenv.get(env_name).cloned())
            .ok_or_else(|| {
                format!(
                    "env `{env_name}` resolves to nothing: set it in the environment or the \
                     fleet home's .env"
                )
            })?;
        env.push((env_name.clone(), value));
    }
    Ok(crate::mcp::ServerSpec {
        program: entry.program.clone(),
        args: entry.args.clone(),
        env,
        passthrough: Vec::new(),
        cwd: None,
    })
}

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

// ------------------------------------------------------------ discovery

/// The machine's foreign harness configs, as `(harness, user-home
/// relative path)` pairs. The scan reads them and never writes them:
/// import is one-way, into the fleet home.
pub const FOREIGN_HARNESSES: &[(&str, &str)] = &[
    (
        "claude-desktop",
        "Library/Application Support/Claude/claude_desktop_config.json",
    ),
    ("claude-code", ".claude.json"),
    ("cursor", ".cursor/mcp.json"),
    ("codex", ".codex/config.toml"),
];

/// The shape one scanned entry reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanShape {
    /// Spawnable by the client Fleet already has.
    Stdio {
        program: String,
        args: Vec<String>,
        env_names: Vec<String>,
        /// The source's own enabled flag, when it declared one (Codex
        /// does): an entry the other harness had disabled imports
        /// disabled, so importing never turns something on the owner
        /// had turned off.
        enabled: bool,
    },
    /// Reported by shape, not disappeared (C3): the reason names what
    /// the entry is.
    Unsupported { reason: String },
}

/// The advisorial badge: what kind of launch the entry is, derived from
/// its own shape (D6). The four labels are facts the config declares —
/// the program is docker, or a package runner, or a bare interpreter, or
/// the entry carries a url — so the badge is read off the entry rather
/// than asked of a lane, which would spend the judgment budget per row
/// on a fact in hand. `Unclassified` covers an exotic program; the
/// owner decides imports either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Containerized,
    PinnedPackage,
    BareInterpreter,
    RemoteEndpoint,
    Unclassified,
}

impl Badge {
    pub fn label(self) -> &'static str {
        match self {
            Badge::Containerized => "containerized",
            Badge::PinnedPackage => "pinned package",
            Badge::BareInterpreter => "bare interpreter",
            Badge::RemoteEndpoint => "remote endpoint",
            Badge::Unclassified => "unclassified",
        }
    }

    /// The review list's order: containerized and pinned first (their
    /// surface is declared), the bare interpreter flagged (it runs the
    /// machine's own runtime), then the remote and the unknown.
    fn rank(self) -> usize {
        match self {
            Badge::Containerized | Badge::PinnedPackage => 0,
            Badge::BareInterpreter => 1,
            Badge::RemoteEndpoint => 2,
            Badge::Unclassified => 3,
        }
    }
}

/// One scanned entry: who it is, which file named it, its shape, its
/// badge, and whether the registry already carries the name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRow {
    pub name: String,
    pub harness: &'static str,
    pub file: String,
    pub shape: ScanShape,
    pub badge: Badge,
    pub configured: bool,
}

/// The scan's whole answer: the rows in review order, and the named
/// skips — the files that exist but could not be read (C3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanReport {
    pub rows: Vec<ScanRow>,
    pub skips: Vec<String>,
}

/// The four foreign files under the USER's home — the other harnesses'
/// configs, not the fleet home's — scanned against the registry at
/// `home`. Absent files are skipped silently — a machine without Cursor
/// has no Cursor servers — and a present-but-unreadable file is a named
/// skip. No user home is an empty scan: there is nothing to discover.
pub fn scan(home: &Path) -> ScanReport {
    let configured: Vec<String> = read_servers(home)
        .map(|servers| servers.into_iter().map(|(name, _)| name).collect())
        .unwrap_or_default();
    let Some(user_home) = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_dir())
    else {
        return ScanReport::default();
    };
    let mut files = Vec::new();
    for (harness, relative) in FOREIGN_HARNESSES {
        let path = user_home.join(relative);
        if path.exists() {
            files.push((*harness, path));
        }
    }
    scan_files(&files, &configured)
}

/// The scan over explicit files — the testable core, and the shape the
/// bridge's scan op reuses. Every entry present is reported: configured
/// / available / unsupported (C3), its source file named.
pub fn scan_files(
    files: &[(&'static str, std::path::PathBuf)],
    configured: &[String],
) -> ScanReport {
    let mut report = ScanReport::default();
    for (harness, path) in files {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            // An absent file is skipped silently (C3): a machine
            // without Cursor has no Cursor servers. A present file
            // that cannot be read is a named skip.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                report.skips.push(format!(
                    "{} ({}) is unreadable: {error}",
                    path.display(),
                    harness
                ));
                continue;
            }
        };
        let entries = match *harness {
            "codex" => parse_toml_mcp_servers(&text),
            _ => parse_json_mcp_servers(&text),
        };
        let entries = match entries {
            Ok(entries) => entries,
            Err(error) => {
                report.skips.push(format!(
                    "{} ({harness}) could not be read: {error}",
                    path.display()
                ));
                continue;
            }
        };
        for (name, shape) in entries {
            // The credential-shaped `-e NAME=VALUE` pairs a desktop
            // config carries are extracted HERE, at the read: the row's
            // args become `-e NAME`, the NAME joins env_names, and no
            // field of the row carries the value — so nothing
            // downstream (a card, the registry, the face's document)
            // can repeat what it never received (D3).
            let shape = match shape {
                ScanShape::Stdio {
                    program,
                    args,
                    env_names,
                    enabled,
                } => {
                    let (args, extracted) = extract_env_args(&args);
                    let mut env_names = env_names;
                    for name in extracted {
                        if !env_names.contains(&name) {
                            env_names.push(name);
                        }
                    }
                    ScanShape::Stdio {
                        program,
                        args,
                        env_names,
                        enabled,
                    }
                }
                other => other,
            };
            let badge = badge_for(&shape);
            report.rows.push(ScanRow {
                badge,
                configured: configured.iter().any(|carried| carried == &name),
                file: path.display().to_string(),
                harness,
                name,
                shape,
            });
        }
    }
    report.rows.sort_by(|left, right| {
        left.badge
            .rank()
            .cmp(&right.badge.rank())
            .then_with(|| left.name.cmp(&right.name))
    });
    report
}

/// The credential-shaped args a foreign config can carry, extracted at
/// the import: `docker run -e NAME=VALUE image` is how the desktop
/// configs pass credentials, and the value rides the arg verbatim. The
/// import rewrites the pair to `-e NAME`, records NAME in env_names,
/// and lets the value resolve at spawn through the chain the lanes use
/// (D3) — a bare `-e NAME` is recorded too. The transformation returns
/// the args to store and the names it extracted.
fn extract_env_args(args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut clean: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "-e" || arg == "--env" {
            match args.get(index + 1) {
                Some(pair) if pair_contains_env_value(pair) => {
                    // `-e NAME=VALUE` becomes `-e NAME`; the value
                    // resolves at spawn, through the child's own env.
                    let name = pair.split('=').next().unwrap_or(pair).to_string();
                    clean.push(arg.clone());
                    clean.push(name.clone());
                    names.push(name);
                    index += 2;
                    continue;
                }
                Some(name) => {
                    // A bare `-e NAME` already says "pass this through":
                    // the name is recorded so the spawn resolves it.
                    clean.push(arg.clone());
                    clean.push(name.clone());
                    names.push(name.clone());
                    index += 2;
                    continue;
                }
                None => {
                    clean.push(arg.clone());
                    index += 1;
                    continue;
                }
            }
        }
        clean.push(arg.clone());
        index += 1;
    }
    (clean, names)
}

/// Whether one arg is the `NAME=VALUE` shape with an env-shaped NAME —
/// the shape docker's `-e` carries a credential in.
fn pair_contains_env_value(arg: &str) -> bool {
    let Some((name, value)) = arg.split_once('=') else {
        return false;
    };
    !value.is_empty()
        && !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The args as the cards print them: a `NAME=VALUE` arg's value is the
/// owner's credential or their machine's path, and a card has no
/// reason to repeat either — the NAME and the fact that it resolves at
/// spawn is what a reader needs. A URL's credential-shaped query
/// parameter is redacted the same way. One display rule for both
/// renders.
pub fn display_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|arg| {
            if pair_contains_env_value(arg) {
                let name = arg.split('=').next().unwrap_or(arg);
                return format!("{name}=<resolves at spawn>");
            }
            redact_query(arg)
        })
        .collect()
}

/// The keys a query parameter must not show a value for on a card.
const CREDENTIAL_QUERY_KEYS: &[&str] = &[
    "apikey",
    "api_key",
    "token",
    "access_token",
    "key",
    "password",
    "secret",
];

/// `https://host/mcp?apiKey=x` prints as
/// `https://host/mcp?apiKey=<redacted>`: the address stays readable,
/// the key does not ride a card.
fn redact_query(arg: &str) -> String {
    let Some((base, query)) = arg.split_once('?') else {
        return arg.to_string();
    };
    let parts: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _))
                if CREDENTIAL_QUERY_KEYS.contains(&key.to_ascii_lowercase().as_str()) =>
            {
                format!("{key}=<redacted>")
            }
            Some((key, value)) => format!("{key}={value}"),
            None => pair.to_string(),
        })
        .collect();
    format!("{base}?{}", parts.join("&"))
}

/// The badge an entry's shape declares.
fn badge_for(shape: &ScanShape) -> Badge {
    match shape {
        ScanShape::Unsupported { reason } if reason.starts_with("remote endpoint") => {
            Badge::RemoteEndpoint
        }
        ScanShape::Unsupported { .. } => Badge::Unclassified,
        ScanShape::Stdio { program, .. } => {
            let runner = program
                .rsplit('/')
                .next()
                .unwrap_or(program)
                .to_ascii_lowercase();
            match runner.as_str() {
                "docker" | "podman" => Badge::Containerized,
                "npx" | "uvx" | "pipx" | "bunx" => Badge::PinnedPackage,
                "python" | "python3" | "node" | "deno" | "ruby" | "perl" => Badge::BareInterpreter,
                _ => Badge::Unclassified,
            }
        }
    }
}

/// The `mcpServers` object of a JSON config, as `(name, shape)` pairs.
/// Values are never kept: an `env` object contributes its KEYS only,
/// which is the C4 rule at the read, before anything could carry one
/// into a report.
fn parse_json_mcp_servers(text: &str) -> Result<Vec<(String, ScanShape)>, String> {
    let document: Value =
        serde_json::from_str(text).map_err(|error| format!("not valid JSON: {error}"))?;
    let empty = Map::new();
    let servers = document
        .get("mcpServers")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut entries = Vec::new();
    for (name, spec) in servers {
        entries.push((name.clone(), json_shape(name, spec)));
    }
    Ok(entries)
}

/// One JSON entry's shape, defensively: whatever the config carries,
/// the scan reports a shape or a named unsupported — never a guess.
fn json_shape(name: &str, spec: &Value) -> ScanShape {
    let Some(object) = spec.as_object() else {
        return ScanShape::Unsupported {
            reason: "the entry is not an object".to_string(),
        };
    };
    if let Some(url) = object.get("url").and_then(Value::as_str) {
        return ScanShape::Unsupported {
            reason: format!("remote endpoint: {url} needs a transport this slice does not carry"),
        };
    }
    if let Some(kind) = object.get("type").and_then(Value::as_str) {
        if kind == "http" || kind == "sse" {
            return ScanShape::Unsupported {
                reason: format!(
                    "remote endpoint: a {kind} server needs a transport this slice does not carry"
                ),
            };
        }
    }
    let Some(program) = object.get("command").and_then(Value::as_str) else {
        return ScanShape::Unsupported {
            reason: "no command and no url: the entry's shape is not one this reader takes"
                .to_string(),
        };
    };
    let args = object
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let env_names = object
        .get("env")
        .and_then(Value::as_object)
        .map(|env| env.keys().cloned().collect())
        .unwrap_or_default();
    let _ = name;
    ScanShape::Stdio {
        program: program.to_string(),
        args,
        env_names,
        enabled: true,
    }
}

/// The `mcp_servers` tables of a TOML config (Codex), read by a
/// purpose-built subset reader — the file is one harness's config, not
/// a format this repo takes a dependency on. What the reader takes:
/// `[table.name]` headers (bare or quoted segments), `key = value` with
/// basic and literal strings, arrays of strings, booleans, integers,
/// inline string tables, and comments. Anything else is a named skip
/// for that file, not a silent misread.
fn parse_toml_mcp_servers(text: &str) -> Result<Vec<(String, ScanShape)>, String> {
    let mut entries: Vec<(String, ScanShape)> = Vec::new();
    let mut current: Option<String> = None;
    let mut in_env_table = false;
    let mut raw_values: Vec<(String, String)> = Vec::new();
    let mut env_names: Vec<String> = Vec::new();
    let flush = |current: &mut Option<String>,
                 raw_values: &mut Vec<(String, String)>,
                 env_names: &mut Vec<String>,
                 in_env_table: &mut bool,
                 entries: &mut Vec<(String, ScanShape)>| {
        if let Some(name) = current.take() {
            let mut program = String::new();
            let mut args = Vec::new();
            let mut enabled = true;
            let mut url: Option<String> = None;
            for (key, value) in raw_values.drain(..) {
                match key.as_str() {
                    "command" => program = value,
                    "args" => {
                        args = value
                            .strip_prefix('[')
                            .and_then(|rest| rest.strip_suffix(']'))
                            .map(|inner| {
                                inner
                                    .split(',')
                                    .map(|part| part.trim().trim_matches('"').to_string())
                                    .filter(|part| !part.is_empty())
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                    "enabled" => enabled = value == "true",
                    "url" => url = Some(value),
                    _ => {}
                }
            }
            let shape = if let Some(url) = url {
                ScanShape::Unsupported {
                    reason: format!(
                        "remote endpoint: {url} needs a transport this slice does not carry"
                    ),
                }
            } else if program.is_empty() {
                ScanShape::Unsupported {
                    reason: "no command and no url: the entry's shape is not one this reader takes"
                        .to_string(),
                }
            } else {
                ScanShape::Stdio {
                    program,
                    args,
                    env_names: std::mem::take(env_names),
                    enabled,
                }
            };
            entries.push((name, shape));
            *in_env_table = false;
        }
    };
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            let Some(end) = line.strip_suffix(']') else {
                return Err(format!(
                    "line {}: `{line}` is not a table header",
                    number + 1
                ));
            };
            let path = end.strip_prefix('[').unwrap_or(end); // array-of-tables is not ours
            let segments = split_toml_key(path);
            let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
            match segments.as_slice() {
                ["mcp_servers", name] => {
                    flush(
                        &mut current,
                        &mut raw_values,
                        &mut env_names,
                        &mut in_env_table,
                        &mut entries,
                    );
                    current = Some(name.to_string());
                }
                ["mcp_servers", _, "env"] => in_env_table = current.is_some(),
                _ => {
                    flush(
                        &mut current,
                        &mut raw_values,
                        &mut env_names,
                        &mut in_env_table,
                        &mut entries,
                    );
                }
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "line {}: `{line}` is not a key = value",
                number + 1
            ));
        };
        let key = key.trim();
        let value = value.trim();
        if in_env_table {
            if let Some(name) = env_name_of_toml(key) {
                env_names.push(name);
            }
            continue;
        }
        if let Some(name) = env_name_of_toml(key) {
            if current.is_some() && key.ends_with("_env_var") {
                // A token's env NAME on an unsupported (HTTP) entry: the
                // shape refusal covers the entry, and the name never
                // carries a value, so there is nothing to record here.
                continue;
            }
            let _ = name;
        }
        let parsed =
            parse_toml_value(value).map_err(|error| format!("line {}: {error}", number + 1))?;
        raw_values.push((key.to_string(), parsed));
    }
    flush(
        &mut current,
        &mut raw_values,
        &mut env_names,
        &mut in_env_table,
        &mut entries,
    );
    Ok(entries)
}

/// A quoted or bare TOML key segment, unquoted.
fn env_name_of_toml(key: &str) -> Option<String> {
    let key = key.trim();
    let unquoted = if (key.starts_with('"') && key.ends_with('"') && key.len() >= 2)
        || (key.starts_with('\'') && key.ends_with('\'') && key.len() >= 2)
    {
        &key[1..key.len() - 1]
    } else {
        key
    };
    if unquoted.is_empty() {
        None
    } else {
        Some(unquoted.to_string())
    }
}

/// The dotted path of a table header, honoring quoted segments
/// (`[projects."/Users/a b"]` is two segments, not five).
fn split_toml_key(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for character in path.chars() {
        match quote {
            Some(q) if character == q => quote = None,
            Some(_) => current.push(character),
            None if character == '"' || character == '\'' => quote = Some(character),
            None if character == '.' => {
                if !current.trim().is_empty() {
                    segments.push(current.trim().to_string());
                }
                current.clear();
            }
            None => current.push(character),
        }
    }
    if !current.trim().is_empty() {
        segments.push(current.trim().to_string());
    }
    segments
}

/// One TOML scalar: a basic string, a literal string, a boolean, an
/// integer, or an array of strings kept as its bracketed text (the args
/// reader strips it). Anything else is a named error.
fn parse_toml_value(value: &str) -> Result<String, String> {
    if let Some(rest) = value.strip_prefix('"') {
        let Some(end) = rest.rfind('"') else {
            return Err("the string never closes".to_string());
        };
        return Ok(rest[..end].replace("\\\"", "\"").replace("\\\\", "\\"));
    }
    if let Some(rest) = value.strip_prefix('\'') {
        let Some(end) = rest.rfind('\'') else {
            return Err("the string never closes".to_string());
        };
        return Ok(rest[..end].to_string());
    }
    if value == "true" || value == "false" {
        return Ok(value.to_string());
    }
    if value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '-')
    {
        // An integer, optionally signed: one leading dash and digits.
        // A date (`1979-05-27`) carries dashes between digits and is
        // not a value this reader takes — a named error, not a misread.
        let digits = value.strip_prefix('-').unwrap_or(value);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            return Ok(value.to_string());
        }
    }
    if value.starts_with('[') {
        return Ok(value.to_string());
    }
    Err(format!("`{value}` is a value this reader does not take"))
}

/// Import one scanned entry into the registry: the write executes
/// nothing and carries the entry's shape — program, args, env NAMES,
/// the source's own enabled flag — plus the source badge naming the
/// harness and today's date. An unsupported shape is a named refusal
/// (C3): the scan reported it, the import will not write it.
///
/// `harness: Some(...)` disambiguates a name discovered under more than
/// one harness; the CLI's bare-name path resolves that first.
pub fn import(home: &Path, row: &ScanRow, today: &str) -> Result<String, String> {
    let (program, args, env_names, enabled) = match &row.shape {
        ScanShape::Stdio {
            program,
            args,
            env_names,
            enabled,
        } => (program.clone(), args.clone(), env_names.clone(), *enabled),
        ScanShape::Unsupported { reason } => {
            return Err(format!("`{}` cannot be imported: {reason}", row.name));
        }
    };
    crate::settings::set_server(
        home,
        &row.name,
        &serde_json::json!({
            "program": program,
            "args": args,
            "env_names": env_names,
            "enabled": enabled,
            "source": {"harness": row.harness, "imported": today},
        }),
    )?;
    Ok(row.name.clone())
}

/// Render the scan as cards: one card per harness that answered, its
/// entries as dot-and-badge lines in review order, and the named skips
/// at the end. Never a plain-text table, and never an env value — the
/// rows carry names, which is all the scan ever read.
pub fn render_scan(report: &ScanReport) -> String {
    use palette::*;
    let mut out = String::new();
    if report.rows.is_empty() && report.skips.is_empty() {
        out.push_str(&format!(
            "{BOLD}fleet mcp scan{RESET} {DIM}— this machine's other harnesses{RESET}\n\n"
        ));
        out.push_str(&format!(
            "{DIM}│{RESET} {TEXT}No foreign config file is present: nothing to discover.{RESET}\n"
        ));
        return out;
    }
    out.push_str(&format!(
        "{BOLD}fleet mcp scan{RESET} {DIM}— {} entr{} across this machine's other harnesses{RESET}\n\n",
        report.rows.len(),
        if report.rows.len() == 1 { "y" } else { "ies" },
    ));
    let mut harnesses: Vec<&str> = report.rows.iter().map(|row| row.harness).collect();
    harnesses.sort();
    harnesses.dedup();
    for harness in harnesses {
        let rows: Vec<&ScanRow> = report
            .rows
            .iter()
            .filter(|row| row.harness == harness)
            .collect();
        let file = rows[0].file.clone();
        let header_right = format!(
            "{DIM}{} {}{RESET}",
            rows.len(),
            if rows.len() == 1 { "entry" } else { "entries" },
        );
        out.push_str(&format!(
            "{DIM}┌─{RESET} {BOLD}{ACCENT}{harness}{RESET} {DIM}{dashes} {header_right} {DIM}┐{RESET}\n",
            dashes = dashes_for(harness, " entries "),
        ));
        out.push_str(&format!("{DIM}│{RESET} {DIM}{}{RESET}\n", file));
        for row in rows {
            let (mark, color) = if row.configured {
                ("configured", TEXT)
            } else {
                match &row.shape {
                    ScanShape::Stdio { .. } => ("available", OK),
                    ScanShape::Unsupported { .. } => ("unsupported", DIM),
                }
            };
            let detail = match &row.shape {
                ScanShape::Stdio {
                    program,
                    args,
                    env_names,
                    ..
                } => {
                    let shown = display_args(args);
                    let launch = if shown.is_empty() {
                        program.clone()
                    } else {
                        format!("{program} {}", shown.join(" "))
                    };
                    if env_names.is_empty() {
                        launch
                    } else {
                        format!("{launch} · env {}", env_names.join(","))
                    }
                }
                ScanShape::Unsupported { reason } => reason.clone(),
            };
            let body = format!("{} · {} · {}", row.name, row.badge.label(), detail);
            for (index, chunk) in wrap_detail(&body, CARD_WIDTH - 8).into_iter().enumerate() {
                let lead = if index == 0 {
                    format!("{DIM}│{RESET} {color}{mark}{RESET}")
                } else {
                    format!("{DIM}│{RESET}       ")
                };
                out.push_str(&format!("{lead} {chunk}\n"));
            }
        }
        out.push_str(&format!("{DIM}└───{RESET}\n"));
    }
    if !report.skips.is_empty() {
        out.push_str(&format!("\n{DIM}{} skip{RESET}\n", report.skips.len(),));
        for skip in &report.skips {
            out.push_str(&format!("{DIM}·{RESET} {skip}\n"));
        }
    }
    let available = report
        .rows
        .iter()
        .filter(|row| !row.configured && matches!(row.shape, ScanShape::Stdio { .. }))
        .count();
    out.push_str(&format!(
        "\n{DIM}{} available to import · the rest configured or unsupported by shape{RESET}\n",
        available,
    ));
    out
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

/// The launch line as the face shows it: the program and the args
/// through the display redaction, so no value the file carries reaches
/// the page.
pub fn display_entry(entry: &ServerEntry) -> (String, Vec<String>) {
    let shown = display_args(&entry.args);
    (entry.program.clone(), shown)
}

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
    let shown = display_args(&entry.args);
    let launch = if shown.is_empty() {
        entry.program.clone()
    } else {
        format!("{} {}", entry.program, shown.join(" "))
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

    // ------------------------------------------------- the scan's tests

    /// A JSON config fixture: one stdio entry with an env name, one
    /// whose env value carries a marker, one HTTP entry.
    fn json_fixture(marker: &str) -> String {
        format!(
            r#"{{"mcpServers": {{
                "time": {{"command": "npx", "args": ["-y", "server-time"]}},
                "weather": {{"command": "python3", "args": ["-m", "weather"], "env": {{"OWM_API_KEY": "{marker}"}}}},
                "tinyfish": {{"type": "http", "url": "https://agent.tinyfish.example/mcp"}},
                "github": {{"command": "docker", "args": ["run", "-i", "--rm", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN={marker}", "ghcr.io/github/github-mcp-server"]}}
            }}}}"#
        )
    }

    /// A TOML config fixture in Codex's own shape: one stdio entry with
    /// an env sub-table, one disabled stdio entry, one HTTP entry.
    fn toml_fixture() -> String {
        r#"
model = "antigravity/gemini-3.8-flash-high"

[mcp_servers.node_repl]
args = []
command = "/Applications/Codex.app/bin/node_repl"
startup_timeout_sec = 120

[mcp_servers.node_repl.env]
NODE_REPL_NODE_PATH = "/Applications/Codex.app/bin/node"
BROWSER_USE_AVAILABLE_BACKENDS = "chrome,iab,mcpapps"

[mcp_servers.computer-use]
command = "./SkyComputerUseClient"
args = ["mcp"]
cwd = "."
enabled = false

[mcp_servers.tinyfish]
url = "https://agent.tinyfish.example/mcp"
bearer_token_env_var = "TINYFISH_API_KEY"

[projects."/Users/a b/Resource Allocation"]
trust_level = "trusted"
"#
        .to_string()
    }

    #[test]
    fn the_scan_reports_each_entry_with_its_shape_and_source_named() {
        let dir = scratch("scan");
        let json_path = dir.join("claude_desktop_config.json");
        let toml_path = dir.join("config.toml");
        std::fs::write(&json_path, json_fixture("sk-marker-1")).unwrap();
        std::fs::write(&toml_path, toml_fixture()).unwrap();
        let report = scan_files(
            &[
                ("claude-desktop", json_path.clone()),
                ("codex", toml_path.clone()),
            ],
            &[],
        );
        // Every entry is reported, each with its source file named (C3).
        assert_eq!(report.rows.len(), 7, "{:?}", report.rows);
        assert!(report
            .rows
            .iter()
            .all(|row| row.file == json_path.display().to_string()
                || row.file == toml_path.display().to_string()));
        // The shapes: stdio where there is a command, unsupported where
        // there is a url.
        let weather = &report
            .rows
            .iter()
            .find(|r| r.name == "weather")
            .unwrap()
            .shape;
        match weather {
            ScanShape::Stdio {
                program, env_names, ..
            } => {
                assert_eq!(program, "python3");
                assert_eq!(env_names, &vec!["OWM_API_KEY".to_string()]);
            }
            other => panic!("weather is not stdio: {other:?}"),
        }
        let tinyfish = &report.rows.iter().find(|r| r.name == "tinyfish").unwrap();
        match &tinyfish.shape {
            ScanShape::Unsupported { reason } => {
                assert!(reason.starts_with("remote endpoint"), "{reason}");
            }
            other => panic!("tinyfish is not unsupported: {other:?}"),
        }
        // Codex's own enabled flag rides the shape: computer-use was
        // disabled there, and the scan says so.
        let computer = &report
            .rows
            .iter()
            .find(|r| r.name == "computer-use")
            .unwrap()
            .shape;
        match computer {
            ScanShape::Stdio { enabled, .. } => assert!(!enabled, "the source said disabled"),
            other => panic!("computer-use is not stdio: {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_scan_carries_names_and_never_values() {
        let marker = "sk-MARKER-VALUE-9f2c";
        let dir = scratch("marker");
        let json_path = dir.join("claude_desktop_config.json");
        std::fs::write(&json_path, json_fixture(marker)).unwrap();
        let report = scan_files(&[("claude-desktop", json_path)], &[]);
        // The whole report — every row's every field — is free of the
        // marker value, and carries the env NAME (C4).
        let rendered = format!("{:?}", report);
        assert!(!rendered.contains(marker), "the marker leaked: {rendered}");
        let weather = report.rows.iter().find(|r| r.name == "weather").unwrap();
        match &weather.shape {
            ScanShape::Stdio { env_names, .. } => {
                assert_eq!(env_names, &vec!["OWM_API_KEY".to_string()]);
            }
            other => panic!("weather is not stdio: {other:?}"),
        }
        // The same holds of the rendered cards and of the imported
        // registry file: the import writes, then both are grepped.
        let rendered = render_scan(&report);
        assert!(!rendered.contains(marker), "the cards leaked the marker");
        assert!(rendered.contains("OWM_API_KEY"), "the name is on the cards");
        let home = scratch("marker-home");
        let row = report.rows.iter().find(|r| r.name == "weather").unwrap();
        import(&home, row, "2026-10-23").expect("imports");
        let file = std::fs::read_to_string(servers_path(&home)).unwrap();
        assert!(
            !file.contains(marker),
            "the registry leaked the marker: {file}"
        );
        assert!(file.contains("OWM_API_KEY"));
        assert!(
            file.contains("claude-desktop"),
            "the source badge rides the import"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_import_carries_the_source_disabled_flag_and_refuses_unsupported() {
        let dir = scratch("import");
        let toml_path = dir.join("config.toml");
        std::fs::write(&toml_path, toml_fixture()).unwrap();
        let report = scan_files(&[("codex", toml_path)], &[]);
        let home = scratch("import-home");
        let computer = report
            .rows
            .iter()
            .find(|r| r.name == "computer-use")
            .unwrap();
        import(&home, computer, "2026-10-23").expect("imports");
        let (_, entry) = &read_servers(&home).expect("reads")[0];
        assert!(!entry.enabled, "the source's disabled flag imports");
        assert_eq!(entry.source.as_ref().expect("badged").harness, "codex");
        // An unsupported shape is refused, by its reason.
        let tinyfish = report.rows.iter().find(|r| r.name == "tinyfish").unwrap();
        let error = import(&home, tinyfish, "2026-10-23").expect_err("refused");
        assert!(error.contains("cannot be imported"), "{error}");
        assert!(error.contains("remote endpoint"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn configured_names_are_marked_and_the_review_list_orders_by_badge() {
        let dir = scratch("order");
        let json_path = dir.join("claude_desktop_config.json");
        std::fs::write(&json_path, json_fixture("sk-marker-2")).unwrap();
        let report = scan_files(&[("claude-desktop", json_path)], &["time".to_string()]);
        let time = report.rows.iter().find(|r| r.name == "time").unwrap();
        assert!(time.configured, "the registry carries it");
        let weather = report.rows.iter().find(|r| r.name == "weather").unwrap();
        assert!(!weather.configured);
        // The order: the declared-surface launches first — the pinned
        // package and the containerized one — then the bare interpreter
        // flagged, then the remote (D6's rank).
        let names: Vec<&str> = report.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["github", "time", "weather", "tinyfish"],
            "{names:?}"
        );
        assert_eq!(report.rows[0].badge, Badge::Containerized);
        assert_eq!(report.rows[1].badge, Badge::PinnedPackage);
        assert_eq!(report.rows[2].badge, Badge::BareInterpreter);
        assert_eq!(report.rows[3].badge, Badge::RemoteEndpoint);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unparseable_file_is_a_named_skip_and_an_absent_one_silence() {
        let dir = scratch("skips");
        let bad = dir.join("bad.json");
        std::fs::write(&bad, "{not json").unwrap();
        let absent = dir.join("absent.json");
        let report = scan_files(&[("claude-desktop", bad), ("cursor", absent)], &[]);
        assert!(report.rows.is_empty());
        assert_eq!(report.skips.len(), 1, "{:?}", report.skips);
        assert!(
            report.skips[0].contains("not valid JSON"),
            "{}",
            report.skips[0]
        );
        assert!(
            report.skips[0].contains("claude-desktop"),
            "{}",
            report.skips[0]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_toml_reader_takes_the_codex_shapes_and_names_what_it_cannot() {
        let entries = parse_toml_mcp_servers(&toml_fixture()).expect("reads");
        assert_eq!(entries.len(), 3);
        let node = &entries
            .iter()
            .find(|(name, _)| name == "node_repl")
            .unwrap()
            .1;
        match node {
            ScanShape::Stdio {
                program, env_names, ..
            } => {
                assert_eq!(program, "/Applications/Codex.app/bin/node_repl");
                assert!(env_names.contains(&"NODE_REPL_NODE_PATH".to_string()));
                assert!(env_names.contains(&"BROWSER_USE_AVAILABLE_BACKENDS".to_string()));
            }
            other => panic!("node_repl is not stdio: {other:?}"),
        }
        // A feature the reader does not take is a named error, not a
        // misread.
        let exotic = "[mcp_servers.odd]\ncommand = \"x\"\nvalue = 1979-05-27\n";
        let error = parse_toml_mcp_servers(exotic).expect_err("refused");
        assert!(error.contains("this reader does not take"), "{error}");
    }

    #[test]
    fn the_scan_render_names_the_harness_and_never_a_value() {
        let dir = scratch("render");
        let json_path = dir.join("claude_desktop_config.json");
        std::fs::write(&json_path, json_fixture("sk-MARKER-3")).unwrap();
        let report = scan_files(&[("claude-desktop", json_path)], &["time".to_string()]);
        let out = render_scan(&report);
        assert!(out.contains("claude-desktop"), "{out}");
        assert!(out.contains("configured"), "{out}");
        assert!(out.contains("available"), "{out}");
        assert!(out.contains("unsupported"), "{out}");
        assert!(out.contains("pinned package"), "{out}");
        assert!(!out.contains("sk-MARKER-3"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_env_pair_in_the_args_is_extracted_and_never_stored_or_shown() {
        let marker = "ghp-MARKER-PAT-4f2b";
        let dir = scratch("env-pair");
        let json_path = dir.join("claude_desktop_config.json");
        std::fs::write(&json_path, json_fixture(marker)).unwrap();
        let report = scan_files(&[("claude-desktop", json_path)], &[]);
        // The scan's rows carry the args: the display rule is the
        // cards', but the shape keeps what the import needs.
        let github = report.rows.iter().find(|r| r.name == "github").unwrap();
        let home = scratch("env-pair-home");
        import(&home, github, "2026-10-23").expect("imports");
        let (_, entry) = &read_servers(&home).expect("reads")[0];
        // The pair became `-e NAME` plus a recorded env name; the value
        // never reached the file (C4, the args half).
        assert!(entry.args.contains(&"-e".to_string()));
        assert!(entry
            .args
            .contains(&"GITHUB_PERSONAL_ACCESS_TOKEN".to_string()));
        assert!(
            !entry.args.iter().any(|arg| arg.contains(marker)),
            "{:?}",
            entry.args
        );
        assert!(entry
            .env_names
            .contains(&"GITHUB_PERSONAL_ACCESS_TOKEN".to_string()));
        let file = std::fs::read_to_string(servers_path(&home)).unwrap();
        assert!(
            !file.contains(marker),
            "the registry leaked the PAT: {file}"
        );
        // The cards show the NAME and the fact it resolves, never a value.
        let out = render(&read_servers(&home).expect("reads"));
        assert!(out.contains("env GITHUB_PERSONAL_ACCESS_TOKEN"), "{out}");
        assert!(out.contains("resolves at spawn"), "{out}");
        assert!(!out.contains(marker), "{out}");
        let scan_out = render_scan(&report);
        assert!(!scan_out.contains(marker), "{scan_out}");
        assert!(
            scan_out.contains("env GITHUB_PERSONAL_ACCESS_TOKEN"),
            "{scan_out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_probe_with_an_unresolved_env_name_fails_by_name() {
        // The async body is the probe's; the env resolution is the
        // registry's, and its refusal is the named one.
        let home = scratch("probe-env");
        set_server(
            &home,
            "weather",
            &serde_json::json!({
                "program": "python3",
                "env_names": ["FLEET_TEST_MISSING_KEY"],
                "enabled": true,
            }),
        )
        .unwrap();
        let entry = read_server(&home, "weather").unwrap().unwrap();
        let error =
            spec_from_entry(&entry, &std::collections::BTreeMap::new()).expect_err("refused");
        assert!(error.contains("FLEET_TEST_MISSING_KEY"), "{error}");
        assert!(error.contains("resolves to nothing"), "{error}");
        let _ = std::fs::remove_dir_all(&home);
    }
}
