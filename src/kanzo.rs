//! The kanzo lane: the owner's intent backlog, read and answered over
//! kanzo's own MCP server.
//!
//! Kanzo is not a vendored python core — it is one binary the owner
//! installs (`kanzo`, 4.6 MB) that serves a board at `localhost:4242` and
//! an MCP server on stdio (`kanzo mcp`) in the same process. Fleet is a
//! **client** of that server: it reads the board, comments, and moves
//! cards through kanzo's own tools, and never writes a task's wording.
//!
//! Everything below is measured from the installed build (0.4.2) rather
//! than guessed:
//!
//! - **Transport** — stdio, one JSON-RPC object per line, no
//!   `Content-Length` framing (the same shape `crate::mcp` already
//!   speaks, KI-387). `kanzo mcp` answers `initialize` with
//!   `serverInfo {name: "kanzo", version}` and protocol `2025-06-18`.
//! - **Tools** — `list_projects`, `create_project`, `link_repo`,
//!   `list_tasks`, `get_task`, `create_task`, `update_task`,
//!   `add_comment`, `next_task`, `delete_task`. This lane calls four of
//!   them (`list_tasks`, `get_task`, `add_comment`, `update_task`) and
//!   creates nothing: `create_task` is deliberately unused.
//! - **Columns** — `todo`, `doing`, `review`, `done`, plus `backlog` when
//!   the owner turns the backlog setting on. The column this lane reads
//!   is a lane setting (`KANZO_COLUMN`, default `todo`).
//! - **Human tasks** — `list_tasks` omits them and counts them in a
//!   trailing note; `get_task` marks them `human: for the user, not for
//!   agents`. The lane refuses one by name whenever it holds its id.
//! - **Authorship** — a comment posted from the board's own UI is
//!   authored `me`; an MCP caller names its author. The owner's accept is
//!   therefore a comment authored `me`, after the synthesis, carrying the
//!   accept word — a signal no persona can forge by naming itself.
//! - **Live updates** — the board polls `/api/changes`; there is no SSE
//!   or websocket on the board's own routes (measured 0.4.2). The relay
//!   phase carries that fact forward.
//!
//! The loop is one pass at a time, and the plan's contract is law:
//! READ → PERSPECTIVES → SYNTHESIS → ACCEPT → RUN → REPORT. A pass that
//! crashes re-runs as a no-op because every step it took is in the
//! ledger, keyed by task id.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::mcp::{McpServer, ServerSpec};

/// The lane's registry name.
pub const LANE: &str = "kanzo";
/// The environment variable naming the binary, when it is not on PATH.
pub const BIN_ENV: &str = "FLEET_KANZO_BIN";
/// Kanzo's own data-folder variable (the store, not a service tree).
pub const HOME_ENV: &str = "KANZO_HOME";
/// The board's port — kanzo's own default, and the one the plan names.
pub const PORT: u16 = 4242;
/// The author the board writes for the owner's own comments (measured:
/// `kanzo-web`'s `add_comment` posts `"me"`). This is what makes an
/// accept unforgeable by a persona that names itself anything else.
pub const OWNER_AUTHOR: &str = "me";
/// The word an accept carries. Case-insensitive, at the start of the
/// owner's comment.
pub const ACCEPT_WORD: &str = "accept";
/// The lane's ledger, under `<FLEET_HOME>/kanzo/`.
pub const LEDGER_DIR: &str = "kanzo";
pub const LEDGER_FILE: &str = "ledger.jsonl";

/// How much of one perspective rides the board. A perspective is three
/// lines — approach, needs, worries — not a document; the bound is named
/// rather than discovered.
pub const PERSPECTIVE_CAP: usize = 700;
/// How much of a synthesis rides the board: one line per perspective,
/// the disagreements, and a numbered plan.
pub const SYNTHESIS_CAP: usize = 1_600;

/// The ledger's event words. They are the card's rows and the ledger's
/// own vocabulary, so a reader that names one names them all.
pub const EVENT_PROBE: &str = "probe";
pub const EVENT_READ: &str = "read";
pub const EVENT_PERSPECTIVE: &str = "perspective";
pub const EVENT_SYNTHESIS: &str = "synthesis";
pub const EVENT_OFFER: &str = "offer";
pub const EVENT_ACCEPT: &str = "accept";
pub const EVENT_CLAIM: &str = "claim";
pub const EVENT_REPORT: &str = "report";
pub const EVENT_HELPED: &str = "helped";
pub const EVENT_REFUSED: &str = "refused";
pub const EVENT_FAILED: &str = "failed";

// ---------------------------------------------------------------------------
// the binary
// ---------------------------------------------------------------------------

/// Resolve kanzo's binary: the `FLEET_KANZO_BIN` override (absolute, and
/// it must exist — a mistyped override is a named refusal, never a silent
/// fall-through), then a `kanzo` on PATH, then the installer's own home
/// (`~/.cargo/bin`, where the signed installer puts it).
pub fn binary() -> Result<PathBuf, String> {
    if let Some(named) = std::env::var_os(BIN_ENV) {
        let named = PathBuf::from(named);
        if !named.is_absolute() {
            return Err(format!(
                "{BIN_ENV}={} is a relative path; an absolute one is required",
                named.display()
            ));
        }
        return if named.is_file() {
            Ok(named)
        } else {
            Err(format!(
                "{BIN_ENV}={} is not a file — kanzo is not installed there",
                named.display()
            ))
        };
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("kanzo");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let candidate = PathBuf::from(home).join(".cargo").join("bin").join("kanzo");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "kanzo is not installed — press install (the signed installer at \
         https://github.com/emanuele-em/kanzo/releases/latest/download/kanzo-installer.sh), \
         or name the binary with {BIN_ENV}"
    ))
}

/// The version kanzo reports (`kanzo 0.4.2` → `0.4.2`). Run beside the
/// probe, never on a read path: a card that spawned a process per poll
/// would be a card that costs more than the board.
pub fn version(bin: &Path) -> Result<String, String> {
    let output = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .map_err(|error| format!("{} --version did not run: {error}", bin.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} --version exited {}",
            bin.display(),
            output.status
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let word = text
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("kanzo --version printed no version: {:?}", text.trim()))?;
    Ok(word.to_string())
}

/// Whether the board answers on its own port. A GET of `/api/board` is
/// the read that proves the store opened, not just that the port is
/// bound — a listening socket with a broken store is not a board.
pub fn board_answers() -> bool {
    let url = format!("http://127.0.0.1:{PORT}/api/board");
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(400))
        .timeout(Duration::from_millis(1_200))
        .build();
    match agent.get(&url).call() {
        Ok(reply) => reply.status() == 200,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// the probe
// ---------------------------------------------------------------------------

/// What one probe of the kanzo lane found. `present` is the binary,
/// `mcp` the handshake and the four tools this lane calls, `board` the
/// running board — the plan's own four facts, each named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    pub present: bool,
    pub binary: Option<PathBuf>,
    pub version: Option<String>,
    pub mcp: bool,
    pub tools: Vec<String>,
    pub missing_tools: Vec<String>,
    pub board: bool,
    /// The launch, or the reason it cannot be launched — the lane card's
    /// detail line, exactly as every other lane's is.
    pub detail: String,
    pub elapsed_ms: u128,
}

impl Health {
    /// Whether kanzo can be worked through: the binary and its MCP
    /// server. A board that is not running is not fatal to the lane (the
    /// MCP server opens the store itself), but the card says so by name.
    pub fn usable(&self) -> bool {
        self.present && self.mcp && self.missing_tools.is_empty()
    }
}

/// The tools this lane needs from kanzo. Measured from 0.4.2's
/// `tools/list`; a build that drops one is named, never guessed around.
pub const REQUIRED_TOOLS: &[&str] = &["list_tasks", "get_task", "add_comment", "update_task"];

/// Probe the lane with the machine's own kanzo.
pub async fn probe() -> Health {
    probe_with(None).await
}

/// Probe against an explicit data folder — the live tests' own board, so
/// a test never writes into the owner's `~/kanzo`.
pub async fn probe_with(home: Option<&Path>) -> Health {
    let started = std::time::Instant::now();
    let bin = match binary() {
        Ok(bin) => bin,
        Err(reason) => {
            return Health {
                present: false,
                binary: None,
                version: None,
                mcp: false,
                tools: Vec::new(),
                missing_tools: Vec::new(),
                board: board_answers(),
                detail: reason,
                elapsed_ms: started.elapsed().as_millis(),
            }
        }
    };
    let version = version(&bin).ok();
    let spec = spec(&bin, home);
    let launch = format!("{} mcp", bin.display());
    let server = match McpServer::spawn(&spec) {
        Ok(server) => server,
        Err(error) => {
            return Health {
                present: true,
                binary: Some(bin),
                version,
                mcp: false,
                tools: Vec::new(),
                missing_tools: Vec::new(),
                board: board_answers(),
                detail: format!("{launch}: {error}"),
                elapsed_ms: started.elapsed().as_millis(),
            }
        }
    };
    let outcome = async {
        let initialize = server.initialize().await?;
        let tools = server.list_tools().await?;
        Ok::<_, crate::mcp::McpError>((initialize, tools))
    }
    .await;
    let _ = server.close().await;
    let board = board_answers();
    match outcome {
        Ok((initialize, tools)) => {
            let names: Vec<String> = tools.iter().map(|tool| tool.name.clone()).collect();
            let missing: Vec<String> = REQUIRED_TOOLS
                .iter()
                .filter(|wanted| !names.iter().any(|name| name == *wanted))
                .map(|name| name.to_string())
                .collect();
            let detail = if missing.is_empty() {
                launch
            } else {
                format!("{launch} — missing tools: {}", missing.join(", "))
            };
            Health {
                present: true,
                binary: Some(bin),
                version: version.or(Some(initialize.server_version)),
                mcp: true,
                tools: names,
                missing_tools: missing,
                board,
                detail,
                elapsed_ms: started.elapsed().as_millis(),
            }
        }
        Err(error) => Health {
            present: true,
            binary: Some(bin),
            version,
            mcp: false,
            tools: Vec::new(),
            missing_tools: Vec::new(),
            board,
            detail: format!("{launch}: {error}"),
            elapsed_ms: started.elapsed().as_millis(),
        },
    }
}

/// The launch spec: the binary, the `mcp` subcommand, and nothing else.
/// `KANZO_HOME` is the one locator a child needs when the owner keeps
/// the board's data somewhere other than `~/kanzo`.
pub fn spec(bin: &Path, home: Option<&Path>) -> ServerSpec {
    ServerSpec {
        program: bin.to_string_lossy().into_owned(),
        args: vec!["mcp".to_string()],
        env: home
            .map(|home| vec![(HOME_ENV.to_string(), home.to_string_lossy().into_owned())])
            .unwrap_or_default(),
        // Kanzo resolves its store through its own `KANZO_HOME`, then
        // `~/kanzo`; `HOME` already rides `mcp::BASE_ENV`.
        passthrough: vec![HOME_ENV.to_string()],
        cwd: None,
    }
}

// ---------------------------------------------------------------------------
// the board client
// ---------------------------------------------------------------------------

/// One task as `list_tasks` lists it: the key (`project#N`), the column,
/// the title, its tags and its comment count. Nothing else is parsed —
/// the list's own line is the whole of what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub key: String,
    pub status: String,
    pub title: String,
    pub tags: Vec<String>,
    pub comments: usize,
}

/// One comment as `get_task` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub author: String,
    pub at: String,
    pub body: String,
}

/// One task as `get_task` shows it: the list's facts plus the human
/// mark, the URL, the description and the comments — the four things the
/// loop reads (the guard, the board link, the words to answer, and the
/// owner's accept).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDetail {
    pub key: String,
    pub title: String,
    pub tags: Vec<String>,
    pub status: String,
    pub human: bool,
    pub url: String,
    pub description: String,
    pub comments: Vec<Comment>,
}

impl TaskDetail {
    /// The words a persona is matched against: the title, the tags and
    /// the description. Never the comments — an offer is matched to what
    /// the owner asked for, not to what a previous pass said about it.
    pub fn words(&self) -> Vec<String> {
        let mut text = self.title.clone();
        text.push(' ');
        text.push_str(&self.tags.join(" "));
        text.push(' ');
        text.push_str(&self.description);
        tokens(&text)
    }
}

/// A live kanzo MCP session. One board read or one pass holds one of
/// these; it is closed when the pass ends, so no child outlives it.
pub struct Board {
    server: McpServer,
}

impl Board {
    /// Open the machine's kanzo against the owner's own data folder.
    pub async fn open() -> Result<Board, String> {
        Self::open_with(None).await
    }

    /// Open against an explicit data folder (the tests' scratch board).
    pub async fn open_with(home: Option<&Path>) -> Result<Board, String> {
        let bin = binary()?;
        let spec = spec(&bin, home);
        let server =
            McpServer::spawn(&spec).map_err(|error| format!("kanzo mcp did not start: {error}"))?;
        server
            .initialize()
            .await
            .map_err(|error| format!("kanzo mcp did not answer initialize: {error}"))?;
        Ok(Board { server })
    }

    async fn call(&self, tool: &str, arguments: Value) -> Result<String, String> {
        let result = self
            .server
            .call_tool(tool, arguments)
            .await
            .map_err(|error| format!("kanzo `{tool}` did not answer: {error}"))?;
        let text = result.text();
        if result.is_error {
            return Err(format!("kanzo `{tool}` refused: {text}"));
        }
        Ok(text)
    }

    /// The tasks in one column, in board order. A human task is never in
    /// this list: kanzo omits them and counts them in a trailing note.
    pub async fn tasks(&self, project: Option<&str>, column: &str) -> Result<Vec<Task>, String> {
        let mut arguments = json!({ "status": column });
        if let Some(project) = project {
            arguments["project"] = json!(project);
        }
        let text = self.call("list_tasks", arguments).await?;
        Ok(parse_tasks(&text))
    }

    /// One task in full: the human mark, the URL, the description, the
    /// comments.
    pub async fn task(&self, id: &str) -> Result<TaskDetail, String> {
        let text = self.call("get_task", json!({ "id": id })).await?;
        parse_detail(id, &text)
    }

    /// Post one comment in `author`'s name. This is the whole of the
    /// lane's authorship: each persona speaks for itself, and the owner's
    /// own comments stay `me`.
    pub async fn comment(&self, id: &str, author: &str, body: &str) -> Result<(), String> {
        self.call(
            "add_comment",
            json!({ "id": id, "author": author, "body": body }),
        )
        .await
        .map(|_| ())
    }

    /// Move a card to a column. The only board write besides a comment.
    pub async fn move_to(&self, id: &str, status: &str) -> Result<(), String> {
        self.call("update_task", json!({ "id": id, "status": status }))
            .await
            .map(|_| ())
    }

    /// Point a task at the run's artifact, when the owner left the URL
    /// empty. The owner's own URL is never overwritten.
    pub async fn set_url(&self, id: &str, url: &str) -> Result<(), String> {
        self.call("update_task", json!({ "id": id, "url": url }))
            .await
            .map(|_| ())
    }

    /// Create a project — the seed's own write, used by the live tests and
    /// never by a pass (a pass creates nothing).
    pub async fn create_project(&self, slug: &str, name: &str) -> Result<(), String> {
        self.call("create_project", json!({ "slug": slug, "name": name }))
            .await
            .map(|_| ())
    }

    /// Add a task — the seed's own write, used by the live tests and
    /// never by a pass.
    pub async fn create_task(&self, arguments: Value) -> Result<(), String> {
        self.call("create_task", arguments).await.map(|_| ())
    }

    pub async fn close(self) {
        let _ = self.server.close().await;
    }
}

// ---------------------------------------------------------------------------
// parsing kanzo's own text
// ---------------------------------------------------------------------------

/// Parse one `list_tasks` line: `probe#1 [todo] Ship it {ui, export} (1
/// comment)`. The trailing human note (`(1 human task not shown…)`) and
/// the empty-column sentence carry no key and parse to nothing.
pub fn parse_task_line(line: &str) -> Option<Task> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('(') {
        return None;
    }
    let (key, rest) = line.split_once(' ')?;
    if !key.contains('#') || key.contains('[') {
        return None;
    }
    let rest = rest.strip_prefix('[')?;
    let (status, rest) = rest.split_once(']')?;
    let mut body = rest.trim_start();
    let mut comments = 0;
    if body.ends_with(')') {
        if let Some(open) = body.rfind(" (") {
            let inner = &body[open + 2..body.len() - 1];
            if inner.ends_with("comment") || inner.ends_with("comments") {
                comments = inner
                    .split_whitespace()
                    .next()
                    .and_then(|count| count.parse().ok())
                    .unwrap_or(0);
                body = &body[..open];
            }
        }
    }
    let (title, tags) = split_tags(body);
    Some(Task {
        key: key.to_string(),
        status: status.to_string(),
        title,
        tags,
        comments,
    })
}

/// Split a trailing `{a, b}` tag run off a title.
fn split_tags(body: &str) -> (String, Vec<String>) {
    if body.ends_with('}') {
        if let Some(open) = body.rfind(" {") {
            let tags = body[open + 2..body.len() - 1]
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect();
            return (body[..open].trim().to_string(), tags);
        }
    }
    (body.trim().to_string(), Vec::new())
}

/// Every task line in a `list_tasks` answer.
pub fn parse_tasks(text: &str) -> Vec<Task> {
    text.lines().filter_map(parse_task_line).collect()
}

/// Parse a `get_task` answer:
///
/// ```text
/// probe#1 Ship the export CSV {ui, export}
/// status: todo, human: for the user, not for agents, created 2026-…
/// url: https://github.com/acme/probe/pull/1
///
/// Add a CSV export to the reports page.
///
/// Comments (1):
/// - me, 2026-11-22 04:37 UTC: accept — go ahead
/// ```
pub fn parse_detail(id: &str, text: &str) -> Result<TaskDetail, String> {
    let mut header: Vec<&str> = Vec::new();
    let mut body: Vec<&str> = Vec::new();
    let mut in_body = false;
    for line in text.lines() {
        if !in_body && line.trim().is_empty() {
            in_body = true;
            continue;
        }
        if in_body {
            body.push(line);
        } else {
            header.push(line);
        }
    }
    let head = header
        .first()
        .copied()
        .ok_or_else(|| format!("kanzo answered nothing for `{id}`"))?;
    let (key, rest) = head
        .split_once(' ')
        .ok_or_else(|| format!("kanzo's answer for `{id}` carries no title: {head:?}"))?;
    if !key.contains('#') {
        return Err(format!("kanzo's answer for `{id}` is not a task: {head:?}"));
    }
    let (title, tags) = split_tags(rest);
    let mut detail = TaskDetail {
        key: key.to_string(),
        title,
        tags,
        status: String::new(),
        human: false,
        url: String::new(),
        description: String::new(),
        comments: Vec::new(),
    };
    for line in header.iter().skip(1) {
        if let Some(rest) = line.strip_prefix("status: ") {
            let mut parts = rest.split(", ");
            detail.status = parts.next().unwrap_or_default().trim().to_string();
            detail.human = rest.contains("human: for the user");
        } else if let Some(rest) = line.strip_prefix("url: ") {
            detail.url = rest.trim().to_string();
        }
    }
    if detail.status.is_empty() {
        return Err(format!(
            "kanzo's answer for `{id}` carries no status line: {text:?}"
        ));
    }
    // The body: the description, then the comments section when there is
    // one. A comment's own body may carry newlines; kanzo prints one
    // comment per `- author, stamp: body` line and the body's newlines
    // arrive as `  ` continuation lines, which are kept.
    let mut description: Vec<&str> = Vec::new();
    let mut comments: Vec<Comment> = Vec::new();
    let mut in_comments = false;
    for line in body {
        if !in_comments && line.starts_with("Comments (") {
            in_comments = true;
            continue;
        }
        if in_comments {
            if let Some(rest) = line.strip_prefix("- ") {
                if let Some((who, said)) = rest.split_once(": ") {
                    let (author, at) = who
                        .split_once(", ")
                        .map(|(author, at)| (author.to_string(), at.to_string()))
                        .unwrap_or((who.to_string(), String::new()));
                    comments.push(Comment {
                        author,
                        at,
                        body: said.trim().to_string(),
                    });
                }
            } else if let Some(comment) = comments.last_mut() {
                let line = line.trim();
                if !line.is_empty() {
                    comment.body.push('\n');
                    comment.body.push_str(line);
                }
            }
        } else {
            description.push(line);
        }
    }
    detail.description = description.join("\n").trim().to_string();
    detail.comments = comments;
    Ok(detail)
}

/// The lowercase words a roster entry and a task are matched on.
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .map(|word| word.trim_matches('-').to_lowercase())
        .filter(|word| word.len() >= 4 && !STOP_WORDS.contains(&word.as_str()))
        .collect()
}

/// The words that carry no capability. A roster entry is a description,
/// and half of a description is grammar.
const STOP_WORDS: &[&str] = &[
    "about", "after", "also", "always", "another", "anything", "because", "before", "being",
    "between", "both", "could", "does", "doing", "down", "each", "every", "from", "have", "here",
    "into", "itself", "just", "like", "make", "many", "more", "most", "much", "must", "never",
    "only", "other", "over", "same", "should", "some", "such", "than", "that", "their", "them",
    "then", "there", "these", "they", "thing", "things", "this", "those", "through", "under",
    "until", "very", "want", "well", "what", "when", "where", "which", "while", "will", "with",
    "work", "would", "your", "yours", "task", "tasks", "user", "users", "help", "helps", "owner",
    "board", "fleet", "persona", "personas", "role", "roles",
];

// ---------------------------------------------------------------------------
// the roster
// ---------------------------------------------------------------------------

/// One persona as the lane matches it: its name and the capability words
/// its own role file declares. The roster is `roles::load_all` — the same
/// files the bots surface and a delegation run under; the lane adds no
/// second roster and no scores it cannot show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persona {
    pub name: String,
    pub description: String,
    /// The role's own declared vocabulary: its name, description, skills
    /// and tools, tokenised. A role that narrows nothing declares its
    /// description only — which is exactly what it says about itself.
    pub words: BTreeSet<String>,
}

impl Persona {
    pub fn from_role(role: &crate::roles::Role) -> Persona {
        let mut words: BTreeSet<String> = BTreeSet::new();
        for word in tokens(&role.name) {
            words.insert(word);
        }
        for word in tokens(&role.description) {
            words.insert(word);
        }
        for skill in &role.skills {
            for word in tokens(skill) {
                words.insert(word);
            }
        }
        for tool in &role.tools {
            for word in tokens(tool) {
                words.insert(word);
            }
        }
        Persona {
            name: role.name.clone(),
            description: role.description.clone(),
            words,
        }
    }
}

/// The home's roster, in the order the roles directory lists it.
pub fn roster(home: &Path) -> Vec<Persona> {
    crate::roles::load_all(home)
        .iter()
        .map(Persona::from_role)
        .collect()
}

/// How many of a persona's declared words the task carries. A count, not
/// a probability: the card and the ledger show the same number the match
/// used, and a task no persona scores on is named rather than assigned.
///
/// The match is on words, with the plural/inflection allowance every
/// roster needs (`exporter` answers `export`, `migrations` answers
/// `migration`) — a four-letter stem and at most three trailing
/// characters, so `art` never answers `article`.
pub fn score(persona: &Persona, detail: &TaskDetail) -> usize {
    let words = detail.words();
    words
        .iter()
        .filter(|word| {
            persona
                .words
                .iter()
                .any(|declared| same_word(declared, word))
        })
        .count()
}

/// Whether two lowercase words are the same word, allowing a short
/// inflection: equal, or the shorter is a prefix of the longer with at
/// most three characters between them and a four-letter stem.
fn same_word(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let (short, long) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    short.len() >= 4 && long.starts_with(short) && long.len() - short.len() <= 3
}

/// Whether this persona can help with this task at all.
pub fn capable(persona: &Persona, detail: &TaskDetail) -> bool {
    score(persona, detail) > 0
}

/// The pass's voice pick: the capable personas in score order (name
/// breaking ties, so the pick is stable), the convener, and whether the
/// convener was the owner's own designation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    pub perspectives: Vec<Persona>,
    pub convener: Persona,
    /// `Some(reason)` when the owner named a convener the roster does not
    /// carry — the pass still runs, and the report says the fallback by
    /// name rather than pretending the designation held.
    pub convener_fallback: Option<String>,
}

/// Pick the voices for one task. The budget bounds how many personas
/// speak (0 = every capable one, the default); the convener is the
/// owner's designation when it resolves, else the top-scoring persona.
/// A task no persona scores on picks nothing — the caller names it.
pub fn pick(
    detail: &TaskDetail,
    roster: &[Persona],
    convener: Option<&str>,
    budget: usize,
) -> Option<Picked> {
    let mut scored: Vec<(usize, &Persona)> = roster
        .iter()
        .map(|persona| (score(persona, detail), persona))
        .filter(|(score, _)| *score > 0)
        .collect();
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.name.cmp(&right.1.name)));
    let mut fallback = None;
    let convener = match convener {
        Some(named) => match scored.iter().find(|(_, persona)| persona.name == named) {
            Some((_, persona)) => (*persona).clone(),
            None => {
                let chosen = scored[0].1.clone();
                fallback = Some(format!(
                    "the convener `{named}` is not a persona this home carries — the top-scoring \
                     persona `{}` convened instead",
                    chosen.name
                ));
                chosen
            }
        },
        None => scored[0].1.clone(),
    };
    let keep = if budget == 0 {
        scored.len()
    } else {
        budget.min(scored.len())
    };
    let mut perspectives: Vec<Persona> = scored
        .iter()
        .take(keep)
        .map(|(_, persona)| (*persona).clone())
        .collect();
    // The convener speaks as a perspective too, unless the budget cut it
    // off — a synthesis over voices the owner's own budget silenced would
    // be the invented consensus this lane refuses.
    if !perspectives.iter().any(|one| one.name == convener.name) {
        perspectives.push(convener.clone());
    }
    Some(Picked {
        perspectives,
        convener,
        convener_fallback: fallback,
    })
}

// ---------------------------------------------------------------------------
// the law: human tasks, consent, synthesis coverage
// ---------------------------------------------------------------------------

/// A task the owner marked human is intent, never permission. Named
/// exactly as kanzo marks it, so a refusal on the card reads as the
/// board's own sentence.
pub fn eligible(detail: &TaskDetail) -> Result<(), String> {
    if detail.human {
        return Err(format!(
            "{} is marked human — for the user, not for agents; never offered to",
            detail.key
        ));
    }
    Ok(())
}

/// The owner's consent, read off the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    pub at: String,
    pub body: String,
}

/// Read the owner's accept. The accept is a comment authored `me` (the
/// board's own author for the owner) that carries the accept word and
/// lands **after** the synthesis (`after` is the number of comments the
/// board held when the synthesis was posted). An offer that is never
/// accepted decays by name; an agent's own comment never counts, whatever
/// it says.
///
/// `accept_wait` is the law's own switch, and it exists only so the
/// red-proof can be run: with it off the lane has no way to read consent,
/// and it refuses by name rather than running an offer.
pub fn consent(detail: &TaskDetail, after: usize, accept_wait: bool) -> Result<Consent, String> {
    if !accept_wait {
        return Err(format!(
            "{}: the accept-wait is off, so no consent can be read — an offer is not a job, and \
             the pass refuses to run without the owner's accept",
            detail.key
        ));
    }
    for comment in detail.comments.iter().skip(after) {
        if comment.author != OWNER_AUTHOR {
            continue;
        }
        let said = comment.body.trim().to_lowercase();
        if said.starts_with(ACCEPT_WORD) {
            return Ok(Consent {
                at: comment.at.clone(),
                body: comment.body.clone(),
            });
        }
    }
    Err(format!(
        "{} has no accept from the owner after the offer — the offer decays; an offer is not a \
         job",
        detail.key
    ))
}

/// The synthesis's coverage rule, checked mechanically over the comments
/// actually posted: every perspective's author must be cited **as an
/// author** — a line of the synthesis that starts with the persona's
/// name — and a plan over more than one voice must name what they
/// disagreed on (`none` is an answer). A name that merely appears inside
/// a plan step is not a citation of that persona's perspective: a
/// synthesis that cites nobody speaks for personas that did not speak,
/// and is refused by name.
pub fn synthesis_covers(synthesis: &str, perspectives: &[Perspective]) -> Result<(), String> {
    if perspectives.is_empty() {
        return Err(
            "the synthesis has no perspective to cite — a plan over no voice is not a synthesis"
                .to_string(),
        );
    }
    let said = synthesis.to_lowercase();
    let missing: Vec<String> = perspectives
        .iter()
        .filter(|perspective| !cited(&said, &perspective.persona))
        .map(|perspective| perspective.persona.clone())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "the synthesis spoke for personas that did not speak: {}",
            missing.join(", ")
        ));
    }
    if perspectives.len() > 1 && !said.contains("disagreement") {
        return Err(format!(
            "the synthesis over {} voices names no disagreements — a plan must say what they \
             disagreed on (write `Disagreements: none` when they agree)",
            perspectives.len()
        ));
    }
    Ok(())
}

/// Whether the synthesis carries a digest line for this author: a line
/// that opens with the persona's name and then a separator. The
/// attribution the contract asks for is a line, not a mention.
fn cited(synthesis_lower: &str, persona: &str) -> bool {
    let name = persona.to_lowercase();
    if name.is_empty() {
        return false;
    }
    synthesis_lower.lines().any(|line| {
        let line = line.trim_start();
        match line.strip_prefix(&name) {
            Some(rest) => {
                rest.is_empty()
                    || rest.starts_with(':')
                    || rest.starts_with(" -")
                    || rest.starts_with('\u{2014}')
                    || rest.starts_with(',')
                    || rest.starts_with(" (")
            }
            None => false,
        }
    })
}

// ---------------------------------------------------------------------------
// the ledger
// ---------------------------------------------------------------------------

/// One ledger line. `n` is the board's comment count when the line was
/// written, which is what makes an accept readable as "after the
/// synthesis" on a re-run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub at: String,
    pub task: String,
    #[serde(default)]
    pub url: String,
    pub event: String,
    #[serde(default)]
    pub persona: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub n: usize,
}

/// The lane's action log, one JSON line per step taken, keyed by task id
/// — the `applied.json` idempotence pattern, in the open so the card and
/// the pass read the same file.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    /// Lines that did not parse. A pass that died mid-write leaves one,
    /// and the next append closes it rather than doubling it; a file
    /// someone edited leaves more, and the count is what makes that
    /// visible instead of silent. Zero is the ordinary state.
    pub torn: usize,
}

impl Ledger {
    pub fn path_for(home: &Path) -> PathBuf {
        home.join(LEDGER_DIR).join(LEDGER_FILE)
    }

    /// Read the ledger. A missing file is an empty ledger. A line that
    /// does not parse is counted in `torn` and skipped — the receipt log
    /// must not be blocked by its own torn write, and the count is what
    /// keeps the loss from being silent.
    pub fn load(home: &Path) -> Result<Ledger, String> {
        let path = Self::path_for(home);
        let mut ledger = Ledger {
            path,
            entries: Vec::new(),
            torn: 0,
        };
        let text = match std::fs::read_to_string(&ledger.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(ledger),
            Err(error) => {
                return Err(format!("{} is unreadable: {error}", ledger.path.display()));
            }
        };
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<Entry>(line) {
                Ok(entry) => ledger.entries.push(entry),
                Err(_) => ledger.torn += 1,
            }
        }
        Ok(ledger)
    }

    /// Append one line, creating the directory on first use. A file that
    /// does not end in a newline is a torn tail from a crashed pass: the
    /// newline is written first, so the new receipt is its own line and
    /// the torn one stays counted rather than swallowing it.
    pub fn append(&mut self, entry: Entry) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        }
        let mut line = serde_json::to_string(&entry)
            .map_err(|error| format!("the ledger line did not encode: {error}"))?;
        line.push('\n');
        let mut prefix = "";
        if let Ok(existing) = std::fs::read(&self.path) {
            if !existing.is_empty() && !existing.ends_with(b"\n") {
                prefix = "\n";
            }
        }
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("{}: {error}", self.path.display()))?;
        file.write_all(format!("{prefix}{line}").as_bytes())
            .map_err(|error| format!("{}: {error}", self.path.display()))?;
        self.entries.push(entry);
        Ok(())
    }

    /// Record one step, stamped now.
    pub fn record(
        &mut self,
        event: &str,
        task: &TaskDetail,
        persona: &str,
        detail: &str,
        n: usize,
    ) -> Result<(), String> {
        self.append(Entry {
            at: crate::boards::now_stamp(),
            task: task.key.clone(),
            url: task.url.clone(),
            event: event.to_string(),
            persona: persona.to_string(),
            detail: detail.to_string(),
            n,
        })
    }

    /// Whether this step was already taken — the idempotence read. A
    /// `persona` of `None` asks whether the step was taken at all.
    pub fn saw(&self, task: &str, event: &str, persona: Option<&str>) -> bool {
        self.entries.iter().any(|entry| {
            entry.task == task
                && entry.event == event
                && persona.is_none_or(|persona| entry.persona == persona)
        })
    }

    /// The newest line for a task and an event.
    pub fn newest(&self, task: &str, event: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.task == task && entry.event == event)
    }

    /// Every perspective this lane posted for a task, in posting order —
    /// the synthesis's citation set, rebuilt from the ledger so a re-run
    /// cites the voices the board already carries.
    pub fn perspectives_for(&self, task: &str) -> Vec<Perspective> {
        self.entries
            .iter()
            .filter(|entry| entry.task == task && entry.event == EVENT_PERSPECTIVE)
            .map(|entry| Perspective {
                persona: entry.persona.clone(),
                body: entry.detail.clone(),
            })
            .collect()
    }

    /// Every task this lane has offered on, oldest offer first.
    pub fn offered_tasks(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for entry in &self.entries {
            if entry.event == EVENT_OFFER && !seen.contains(&entry.task) {
                seen.push(entry.task.clone());
            }
        }
        seen
    }

    /// The five counts the card answers with, and the newest entry behind
    /// each — never a number without its receipt.
    pub fn counts(&self) -> Vec<Receipt> {
        [
            ("offers made", EVENT_OFFER),
            ("accepted", EVENT_ACCEPT),
            ("claimed", EVENT_CLAIM),
            ("helped", EVENT_HELPED),
            ("refused", EVENT_REFUSED),
        ]
        .iter()
        .map(|(label, event)| {
            let count = self.entries.iter().filter(|e| e.event == *event).count();
            let newest = self.entries.iter().rev().find(|e| e.event == *event);
            Receipt {
                label: label.to_string(),
                event: event.to_string(),
                count,
                task: newest.map(|e| e.task.clone()).unwrap_or_default(),
                persona: newest.map(|e| e.persona.clone()).unwrap_or_default(),
                url: newest.map(|e| e.url.clone()).unwrap_or_default(),
                at: newest.map(|e| e.at.clone()).unwrap_or_default(),
                detail: newest.map(|e| e.detail.clone()).unwrap_or_default(),
            }
        })
        .collect()
    }
}

/// One card row: the count and the newest receipt behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub label: String,
    pub event: String,
    pub count: usize,
    pub task: String,
    pub persona: String,
    pub url: String,
    pub at: String,
    pub detail: String,
}

// ---------------------------------------------------------------------------
// the settings
// ---------------------------------------------------------------------------

/// The owner's lane settings, read from `<FLEET_HOME>/lanes.json`'s
/// `lanes.kanzo.env` — the same file every lane's configuration lives in,
/// so the column the owner uses is a setting, not a constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The column the pass reads for unclaimed work.
    pub column: String,
    /// The column a helped task is moved to.
    pub done: String,
    /// The persona that convenes, when the owner designates one.
    pub convener: Option<String>,
    /// The project to read, when the owner scopes the pass to one.
    pub project: Option<String>,
    /// How many personas speak per task; 0 is every capable one.
    pub budget: usize,
    /// Whether the pass may read the owner's accept at all. `true` always
    /// in an owner's config; the red-proof is the only caller that turns
    /// it off.
    pub accept_wait: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            column: "todo".to_string(),
            done: "done".to_string(),
            convener: None,
            project: None,
            budget: 0,
            accept_wait: true,
        }
    }
}

/// Read the settings out of a lane's env map. Pure, so the mapping is
/// testable without a home.
pub fn settings_from(env: &BTreeMap<String, String>) -> Settings {
    let mut settings = Settings::default();
    let get = |name: &str| {
        env.get(name)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
    };
    if let Some(column) = get("KANZO_COLUMN") {
        settings.column = column.to_string();
    }
    if let Some(done) = get("KANZO_DONE_COLUMN") {
        settings.done = done.to_string();
    }
    settings.convener = get("KANZO_CONVENER").map(str::to_string);
    settings.project = get("KANZO_PROJECT").map(str::to_string);
    if let Some(budget) = get("KANZO_BUDGET").and_then(|value| value.parse().ok()) {
        settings.budget = budget;
    }
    if let Some(wait) = get("KANZO_ACCEPT_WAIT") {
        settings.accept_wait = !matches!(
            wait.to_ascii_lowercase().as_str(),
            "off" | "no" | "false" | "0"
        );
    }
    settings
}

/// The home's settings. A broken `lanes.json` is a named refusal — a pass
/// that silently read the wrong column would offer on the wrong cards.
pub fn settings(home: &Path) -> Result<Settings, String> {
    match crate::lanes::read_lane(home, LANE)? {
        Some(config) => Ok(settings_from(&config.env)),
        None => Ok(Settings::default()),
    }
}

// ---------------------------------------------------------------------------
// voices
// ---------------------------------------------------------------------------

/// One persona's perspective, as posted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perspective {
    pub persona: String,
    pub body: String,
}

/// Who speaks for a persona, and who convenes. The pass is generic over
/// this so the live tests can script the voices and the owner's own run
/// uses the models — one loop, two mouths.
#[allow(async_fn_in_trait)]
pub trait Voice {
    /// One bounded perspective in this persona's own name: how it would
    /// approach the task, what it would need, what it worries about.
    async fn perspective(&self, persona: &Persona, task: &TaskDetail) -> Result<String, String>;
    /// The synthesis: each perspective attributed, the disagreements
    /// named, and a numbered plan with an owner per step and the accept
    /// affordance.
    async fn synthesis(
        &self,
        convener: &Persona,
        task: &TaskDetail,
        perspectives: &[Perspective],
    ) -> Result<String, String>;
}

/// The personas' own models, through fleet's normal route: each voice is
/// one bounded completion with the persona's overlay, routed like every
/// other request (the same `route::prepare` a run walks). A persona whose
/// route cannot be prepared is a named failure, never a silent empty
/// comment.
pub struct ModelVoice<'a> {
    pub home: &'a Path,
}

impl ModelVoice<'_> {
    async fn say(&self, system: &str, user: &str, cap: usize) -> Result<String, String> {
        let passphrase = crate::home::vault_passphrase();
        let dotenv = crate::home::read_dotenv_with_vault(self.home, passphrase.as_deref());
        let prepared = crate::route::prepare(
            self.home,
            &dotenv,
            "a perspective on the owner's board",
            &std::collections::HashMap::new(),
            None,
        )
        .await
        .map_err(|error| format!("the route could not be prepared: {error}"))?;
        let client = crate::api::Client::routed(prepared.router.clone());
        let messages = vec![
            crate::api::Message::text(crate::api::Role::System, system),
            crate::api::Message::text(crate::api::Role::User, user),
        ];
        let request = crate::api::ChatRequest {
            model: &prepared.decision.model,
            messages: &messages,
            tools: Vec::new(),
            max_tokens: None,
        };
        let response = client
            .chat_stream(&request, |_| {})
            .await
            .map_err(|error| format!("the model did not answer: {error}"))?;
        let text = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();
        if text.trim().is_empty() {
            return Err("the model answered nothing".to_string());
        }
        Ok(bounded(&text, cap))
    }
}

impl Voice for ModelVoice<'_> {
    async fn perspective(&self, persona: &Persona, task: &TaskDetail) -> Result<String, String> {
        let system = format!(
            "You are `{}`, a persona on the owner's fleet. {} Reply with exactly three lines, no \
             preamble: `Approach: …`, `Needs: …`, `Worries: …` — at most 60 words in total, about \
             this task and nothing else. You are not deciding whether to do it; the owner decides.",
            persona.name, persona.description
        );
        let user = format!(
            "The owner's kanzo task {}: {}\n\n{}",
            task.key, task.title, task.description
        );
        self.say(&system, &user, PERSPECTIVE_CAP).await
    }

    async fn synthesis(
        &self,
        convener: &Persona,
        task: &TaskDetail,
        perspectives: &[Perspective],
    ) -> Result<String, String> {
        let system = format!(
            "You are `{}`, convening on the owner's kanzo task {}. You digest the perspectives \
             below, never invent one. Reply with exactly: one line per perspective, each starting \
             with the author's name; then `Disagreements: <what they disagreed on, or none>`; then \
             `Plan:` and numbered steps, each `N. <step> — <persona who owns it>`; then a last \
             line `Accept: comment `accept` on this task to start the plan.` Cite every author \
             listed. At most 200 words.",
            convener.name, task.key
        );
        let mut user = format!(
            "Task {}: {}\n\n{}\n\nPerspectives:\n",
            task.key, task.title, task.description
        );
        for perspective in perspectives {
            user.push_str(&format!(
                "\n[{}]\n{}\n",
                perspective.persona, perspective.body
            ));
        }
        self.say(&system, &user, SYNTHESIS_CAP).await
    }
}

/// Bound a comment at a word boundary. A bound that cut mid-word would
/// post a comment that reads as broken rather than short.
pub fn bounded(text: &str, cap: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= cap {
        return text.to_string();
    }
    let cut: String = text.chars().take(cap).collect();
    let cut = match cut.rfind(char::is_whitespace) {
        Some(at) => cut[..at].to_string(),
        None => cut,
    };
    format!("{}\u{2026}", cut.trim_end())
}

// ---------------------------------------------------------------------------
// the pass
// ---------------------------------------------------------------------------

/// One named skip or refusal: the task, and the sentence that says why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub task: String,
    pub reason: String,
}

/// One task's offer: who spoke, who convened, and the board's own URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferMade {
    pub task: String,
    pub title: String,
    pub url: String,
    pub personas: Vec<String>,
    pub convener: String,
    pub synthesis: String,
}

/// One accept read off the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub task: String,
    pub persona: String,
    pub session: String,
    pub said: String,
}

/// One report posted back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub task: String,
    pub session: String,
    pub record: String,
}

/// What one pass did, in the ledger's own words.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassReport {
    /// The tasks the read found in the owner's column.
    pub read: Vec<String>,
    pub offers: Vec<OfferMade>,
    pub skipped: Vec<Named>,
    pub refused: Vec<Named>,
    pub accepted: Vec<Accepted>,
    pub reported: Vec<Reported>,
}

impl PassReport {
    pub fn is_empty(&self) -> bool {
        self.read.is_empty()
            && self.offers.is_empty()
            && self.skipped.is_empty()
            && self.refused.is_empty()
            && self.accepted.is_empty()
            && self.reported.is_empty()
    }
}

/// Run one pass: read the owner's column, offer on what no persona can
/// help with (named), convene, then read the accepts the board carries
/// and claim them.
pub async fn pass<V: Voice>(
    home: &Path,
    board: &Board,
    voices: &V,
    settings: &Settings,
) -> Result<PassReport, String> {
    let mut ledger = Ledger::load(home)?;
    let mut report = PassReport::default();
    let tasks = board
        .tasks(settings.project.as_deref(), &settings.column)
        .await?;
    for task in &tasks {
        report.read.push(task.key.clone());
    }
    // The offer half runs on the column's own order; the accept half then
    // reads every task this lane has offered on, including ones the owner
    // has since moved out of the column.
    for task in &tasks {
        offer_one(
            home,
            board,
            voices,
            settings,
            &mut ledger,
            &mut report,
            &task.key,
        )
        .await?;
    }
    accept_all(home, board, settings, &mut ledger, &mut report).await?;
    Ok(report)
}

/// Offer on one named task. Public so the human-refusal and red-proof
/// paths run the same code the column read runs.
pub async fn offer_one<V: Voice>(
    home: &Path,
    board: &Board,
    voices: &V,
    settings: &Settings,
    ledger: &mut Ledger,
    report: &mut PassReport,
    id: &str,
) -> Result<(), String> {
    let detail = board.task(id).await?;
    if let Err(reason) = eligible(&detail) {
        ledger.record(EVENT_REFUSED, &detail, "", &reason, detail.comments.len())?;
        report.refused.push(Named {
            task: detail.key.clone(),
            reason,
        });
        return Ok(());
    }
    if ledger.saw(&detail.key, EVENT_CLAIM, None) {
        report.skipped.push(Named {
            task: detail.key.clone(),
            reason: format!(
                "{} is already claimed by this lane — a claim is not offered twice",
                detail.key
            ),
        });
        return Ok(());
    }
    if ledger.saw(&detail.key, EVENT_OFFER, None) {
        report.skipped.push(Named {
            task: detail.key.clone(),
            reason: format!(
                "{} already carries this lane's offer — the ledger makes the re-run a no-op",
                detail.key
            ),
        });
        return Ok(());
    }
    let roster = roster(home);
    let Some(picked) = pick(
        &detail,
        &roster,
        settings.convener.as_deref(),
        settings.budget,
    ) else {
        let reason = format!(
            "no persona on this home's roster can help with {} — it keeps waiting, and it is not \
             reassigned to whatever fits",
            detail.key
        );
        ledger.record(EVENT_REFUSED, &detail, "", &reason, detail.comments.len())?;
        report.refused.push(Named {
            task: detail.key.clone(),
            reason,
        });
        return Ok(());
    };
    if let Some(reason) = &picked.convener_fallback {
        ledger.record(
            EVENT_REFUSED,
            &detail,
            &picked.convener.name,
            reason,
            detail.comments.len(),
        )?;
        report.refused.push(Named {
            task: detail.key.clone(),
            reason: reason.clone(),
        });
    }
    // The perspectives the board already carries (a crashed pass's own
    // comments), rebuilt from the ledger so the synthesis cites them.
    let mut posted = ledger.perspectives_for(&detail.key);
    for persona in &picked.perspectives {
        if posted.iter().any(|one| one.persona == persona.name) {
            continue;
        }
        let body = match voices.perspective(persona, &detail).await {
            Ok(body) => body,
            Err(reason) => {
                let said = format!("{}: {} could not speak: {reason}", detail.key, persona.name);
                ledger.record(
                    EVENT_REFUSED,
                    &detail,
                    &persona.name,
                    &said,
                    detail.comments.len(),
                )?;
                report.refused.push(Named {
                    task: detail.key.clone(),
                    reason: said,
                });
                continue;
            }
        };
        board.comment(&detail.key, &persona.name, &body).await?;
        let n = detail.comments.len() + posted.len() + 1;
        ledger.record(EVENT_PERSPECTIVE, &detail, &persona.name, &body, n)?;
        posted.push(Perspective {
            persona: persona.name.clone(),
            body,
        });
    }
    if posted.is_empty() {
        return Ok(());
    }
    if ledger.saw(&detail.key, EVENT_SYNTHESIS, None) {
        report.skipped.push(Named {
            task: detail.key.clone(),
            reason: format!(
                "{} already carries this lane's synthesis — the ledger makes the re-run a no-op",
                detail.key
            ),
        });
        return Ok(());
    }
    let synthesis = match voices.synthesis(&picked.convener, &detail, &posted).await {
        Ok(synthesis) => synthesis,
        Err(reason) => {
            let said = format!("{}: the synthesis did not land: {reason}", detail.key);
            ledger.record(
                EVENT_REFUSED,
                &detail,
                &picked.convener.name,
                &said,
                detail.comments.len(),
            )?;
            report.refused.push(Named {
                task: detail.key.clone(),
                reason: said,
            });
            return Ok(());
        }
    };
    // The law's mechanical check, over the perspectives actually posted:
    // a synthesis that cites nobody speaks for personas that did not
    // speak, and is refused by name without touching the board.
    if let Err(reason) = synthesis_covers(&synthesis, &posted) {
        ledger.record(
            EVENT_REFUSED,
            &detail,
            &picked.convener.name,
            &reason,
            detail.comments.len(),
        )?;
        report.refused.push(Named {
            task: detail.key.clone(),
            reason,
        });
        return Ok(());
    }
    board
        .comment(&detail.key, &picked.convener.name, &synthesis)
        .await?;
    let n = detail.comments.len() + posted.len() + 1;
    ledger.record(
        EVENT_SYNTHESIS,
        &detail,
        &picked.convener.name,
        &synthesis,
        n,
    )?;
    let plan = synthesis
        .lines()
        .find(|line| line.trim_start().starts_with("Plan:"))
        .unwrap_or_else(|| synthesis.lines().last().unwrap_or_default())
        .trim()
        .to_string();
    ledger.record(EVENT_OFFER, &detail, &picked.convener.name, &plan, n)?;
    report.offers.push(OfferMade {
        task: detail.key.clone(),
        title: detail.title.clone(),
        url: detail.url.clone(),
        personas: posted.iter().map(|one| one.persona.clone()).collect(),
        convener: picked.convener.name.clone(),
        synthesis,
    });
    Ok(())
}

/// Read the accepts the board carries for every task this lane offered
/// on, and claim each one the owner accepted. A task the owner marked
/// human since the offer is refused by name; a task with no accept
/// decays by name.
async fn accept_all(
    home: &Path,
    board: &Board,
    settings: &Settings,
    ledger: &mut Ledger,
    report: &mut PassReport,
) -> Result<(), String> {
    for task in ledger.offered_tasks() {
        if ledger.saw(&task, EVENT_CLAIM, None) {
            continue;
        }
        let detail = board.task(&task).await?;
        if let Err(reason) = eligible(&detail) {
            ledger.record(EVENT_REFUSED, &detail, "", &reason, detail.comments.len())?;
            report.refused.push(Named { task, reason });
            continue;
        }
        let synthesis = ledger.newest(&task, EVENT_SYNTHESIS);
        let after = synthesis
            .map(|entry| entry.n)
            .unwrap_or(detail.comments.len());
        let consent = match consent(&detail, after, settings.accept_wait) {
            Ok(consent) => consent,
            Err(reason) => {
                report.skipped.push(Named { task, reason });
                continue;
            }
        };
        let convener = synthesis
            .map(|entry| entry.persona.clone())
            .unwrap_or_default();
        let model = leader_model(home)?;
        let mut session = crate::session::Session::create_with_persona(
            home,
            &model,
            Some(vec![convener.clone()]),
        )
        .map_err(|error| format!("the claim's session could not be created: {error}"))?;
        let id = session.id().to_string();
        // The claim's own receipt, on the session: the board task's id and
        // URL ride the log's standing objective, so the run's prompt
        // carries them and a reader of the log sees where the work came
        // from (hard rule 5).
        let objective = format!(
            "kanzo {} — {}\nboard: http://127.0.0.1:{PORT}/ · task: {}",
            detail.key,
            detail.title,
            if detail.url.is_empty() {
                format!("http://127.0.0.1:{PORT}/ (no URL on the card)")
            } else {
                detail.url.clone()
            }
        );
        session
            .record(&crate::session::Record::Goal { text: objective })
            .map_err(|error| format!("the claim's goal did not record: {error}"))?;
        ledger.record(
            EVENT_ACCEPT,
            &detail,
            &convener,
            &consent.body,
            detail.comments.len(),
        )?;
        ledger.record(EVENT_CLAIM, &detail, &convener, &id, detail.comments.len())?;
        report.accepted.push(Accepted {
            task: detail.key.clone(),
            persona: convener,
            session: id,
            said: consent.said(),
        });
    }
    Ok(())
}

impl Consent {
    fn said(&self) -> String {
        format!("{} — {}", self.at, self.body)
    }
}

/// Report every claimed task whose session has finished: comment the
/// session's own record line and move the card to the done column. A
/// session still running is left alone, named on the report as pending —
/// a claim is not a completion.
pub async fn report_finished(
    home: &Path,
    board: &Board,
    settings: &Settings,
    ledger: &mut Ledger,
) -> Result<Vec<Reported>, String> {
    let mut reported = Vec::new();
    for task in ledger.offered_tasks() {
        if ledger.saw(&task, EVENT_HELPED, None) {
            continue;
        }
        let Some(claim) = ledger.newest(&task, EVENT_CLAIM) else {
            continue;
        };
        let session = claim.detail.clone();
        let record = match session_record(home, &session) {
            Ok(record) => record,
            Err(_) => continue,
        };
        let detail = board.task(&task).await?;
        let persona = claim.persona.clone();
        board.comment(&task, &persona, &record).await?;
        ledger.record(
            EVENT_REPORT,
            &detail,
            &persona,
            &record,
            detail.comments.len(),
        )?;
        board.move_to(&task, &settings.done).await?;
        ledger.record(
            EVENT_HELPED,
            &detail,
            &persona,
            &session,
            detail.comments.len(),
        )?;
        reported.push(Reported {
            task,
            session,
            record,
        });
    }
    Ok(reported)
}

/// A finished session's own record line: the log's `end` record, as the
/// board's comment. Nothing is composed about the work beyond what the
/// log says — a helped task is reported from its record, never from a
/// summary of what the bot "would have" done.
pub fn session_record(home: &Path, session: &str) -> Result<String, String> {
    let path = crate::session::session_path(home, session);
    let records =
        crate::session::replay(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut end = None;
    for record in &records {
        if let crate::session::Record::End {
            stop_reason,
            turns,
            served_model,
        } = record
        {
            end = Some((stop_reason.clone(), *turns, served_model.clone()));
        }
    }
    let Some((reason, turns, model)) = end else {
        return Err(format!(
            "session {session} has no end record yet — the work is still in flight"
        ));
    };
    Ok(format!(
        "done — session {session}: {turns} turns, stopped {reason}, served by {} · log: {}",
        model.unwrap_or_else(|| "unrecorded".to_string()),
        path.display()
    ))
}

/// The model a claim's session starts on: the roster's first resolved
/// candidate. No judgment call is made here — a claim is a receipt, not a
/// request, and the run's own route decides what serves its turns.
pub fn leader_model(home: &Path) -> Result<String, String> {
    let roster = crate::route::load_roster(home)
        .map_err(|error| format!("this home's roster could not be read: {error}"))?;
    let passphrase = crate::home::vault_passphrase();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let resolved = crate::route::resolve(&roster.candidates, &dotenv);
    resolved
        .iter()
        .find_map(|one| one.candidate.models.first().cloned())
        .ok_or_else(|| {
            "no provider on this home's roster carries a model — a claim's session needs one"
                .to_string()
        })
}

// ---------------------------------------------------------------------------
// the card's read
// ---------------------------------------------------------------------------

/// The bots surface's kanzo document: the ledger's five receipts, the
/// lane's last probe facts, and whether the binary is there. Zero draws
/// empty — a row with no receipt says so instead of standing a zero in
/// for an unread board.
pub fn card(home: &Path) -> Value {
    let ledger = Ledger::load(home).unwrap_or_default();
    let receipts = ledger.counts();
    let probe = ledger
        .entries
        .iter()
        .rev()
        .find(|entry| entry.event == EVENT_PROBE);
    let present = binary().is_ok();
    let rows: Vec<Value> = receipts
        .iter()
        .map(|receipt| {
            json!({
                "label": receipt.label,
                "event": receipt.event,
                "count": receipt.count,
                "task": receipt.task,
                "persona": receipt.persona,
                "url": receipt.url,
                "at": receipt.at,
                "detail": receipt.detail,
            })
        })
        .collect();
    // The newest refusals, by name: the card's refused sentence is the
    // ledger's own words, never a generic apology.
    let refused: Vec<Value> = ledger
        .entries
        .iter()
        .rev()
        .filter(|entry| entry.event == EVENT_REFUSED)
        .take(4)
        .map(|entry| {
            json!({
                "task": entry.task,
                "persona": entry.persona,
                "reason": entry.detail,
                "at": entry.at,
            })
        })
        .collect();
    let offers: Vec<Value> = ledger
        .entries
        .iter()
        .rev()
        .filter(|entry| entry.event == EVENT_OFFER)
        .take(4)
        .map(|entry| {
            json!({
                "task": entry.task,
                "persona": entry.persona,
                "url": entry.url,
                "plan": entry.detail,
                "at": entry.at,
            })
        })
        .collect();
    json!({
        "installed": present,
        "version": probe.and_then(|entry| {
            entry.detail.split(" · ").next().map(str::to_string)
        }),
        "probed_at": probe.map(|entry| entry.at.clone()),
        "mcp": probe.map(|entry| entry.detail.contains("mcp ok")).unwrap_or(false),
        "board": probe.map(|entry| entry.detail.contains("board ok")).unwrap_or(false),
        "rows": rows,
        "offers": offers,
        "refused": refused,
        "entries": ledger.entries.len(),
        "torn": ledger.torn,
        "ledger": ledger.path.display().to_string(),
        // The press: a pass is on demand, never a cron the owner did not
        // ask for. The card names the verb the owner presses.
        "press": "fleet kanzo pass",
    })
}

/// Record a probe's facts in the ledger, so the card answers from the
/// lane's own action log rather than spawning a process per poll.
pub fn record_probe(home: &Path, health: &Health) -> Result<(), String> {
    let mut ledger = Ledger::load(home)?;
    let mut detail = String::new();
    if let Some(version) = &health.version {
        detail.push_str(version);
    } else {
        detail.push_str("version unknown");
    }
    detail.push_str(if health.mcp {
        " · mcp ok"
    } else {
        " · mcp down"
    });
    detail.push_str(if health.board {
        " · board ok"
    } else {
        " · board not running"
    });
    detail.push_str(&format!(" · {}", health.detail));
    ledger.append(Entry {
        at: crate::boards::now_stamp(),
        task: String::new(),
        url: format!("http://127.0.0.1:{PORT}/"),
        event: EVENT_PROBE.to_string(),
        persona: String::new(),
        detail,
        n: 0,
    })
}

// ---------------------------------------------------------------------------
// the surfaces the CLI renders
// ---------------------------------------------------------------------------

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const ERROR: &str = "\u{1b}[31m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, ERROR, OK, RESET, TEXT};

/// The probe as one card, in the lane cards' own idiom: the four facts
/// the plan names, each on its own line, and the launch or the reason.
pub fn render_health(health: &Health) -> String {
    let width = crate::cards::CARD_WIDTH;
    let (dot, color) = if health.usable() {
        ("\u{2713}", OK)
    } else if health.present {
        ("\u{25b3}", ERROR)
    } else {
        ("\u{25cb}", DIM)
    };
    let state = if health.usable() {
        "ok"
    } else if health.present {
        "error"
    } else {
        "unavailable"
    };
    let header_right = format!("{dot} {state}");
    let used = " kanzo ".chars().count() + header_right.chars().count() + 2;
    let mut out = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}kanzo{RESET} {DIM}{dashes} {color}{header_right}{RESET} {DIM}\u{2510}{RESET}\n",
        dashes = crate::cards::rule(width.saturating_sub(used + 4).max(1)),
    );
    let facts = [
        (
            "installed",
            health.present,
            health
                .binary
                .as_ref()
                .map(|bin| bin.display().to_string())
                .unwrap_or_else(|| "no binary — kanzo is not installed".to_string()),
        ),
        (
            "version",
            health.version.is_some(),
            health
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        (
            "MCP",
            health.mcp && health.missing_tools.is_empty(),
            if health.mcp {
                format!("{} tools · {}", health.tools.len(), health.detail)
            } else {
                health.detail.clone()
            },
        ),
        (
            "board",
            health.board,
            if health.board {
                format!("http://127.0.0.1:{PORT} answers")
            } else {
                format!(
                    "not running — start it with `kanzo`, the board answers at 127.0.0.1:{PORT}"
                )
            },
        ),
    ];
    for (label, ok, text) in facts {
        let (dot, color) = if ok {
            ("\u{2713}", OK)
        } else {
            ("\u{25cb}", DIM)
        };
        for (index, chunk) in crate::skills::wrap_detail(&text, width - 18)
            .iter()
            .enumerate()
        {
            let lead = if index == 0 {
                format!("{color}{dot}{RESET} {TEXT}{label:<9}{RESET}")
            } else {
                "              ".to_string()
            };
            out.push_str(&format!("{DIM}\u{2502}{RESET} {lead} {chunk}\n"));
        }
    }
    out.push_str(&format!(
        "{DIM}\u{2502}{RESET} {DIM}{} ms{RESET}\n",
        health.elapsed_ms
    ));
    out.push_str(&format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}\n",
        crate::cards::rule(width - 2)
    ));
    out
}

/// One pass as one card: what was read, offered, skipped and refused,
/// each named — the same sentences the ledger carries.
pub fn render_pass(report: &PassReport) -> String {
    let width = crate::cards::CARD_WIDTH;
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}kanzo pass{RESET} {DIM}\u{2014} read {}, offered {}, skipped {}, refused {}{RESET}\n\n",
        report.read.len(),
        report.offers.len(),
        report.skipped.len(),
        report.refused.len()
    ));
    let line = |out: &mut String, dot: &str, color: &str, text: &str| {
        for (index, chunk) in crate::skills::wrap_detail(text, width - 6)
            .iter()
            .enumerate()
        {
            let lead = if index == 0 {
                format!("{color}{dot}{RESET}")
            } else {
                " ".to_string()
            };
            out.push_str(&format!("{lead} {chunk}\n"));
        }
    };
    for offer in &report.offers {
        line(
            &mut out,
            "\u{2713}",
            OK,
            &format!(
                "offered {} — {} · convened by {} · {}",
                offer.task,
                offer.personas.join(", "),
                offer.convener,
                if offer.url.is_empty() {
                    "no URL on the card"
                } else {
                    &offer.url
                }
            ),
        );
    }
    for accepted in &report.accepted {
        line(
            &mut out,
            "\u{2713}",
            OK,
            &format!(
                "accepted {} by {} — session {} ({})",
                accepted.task, accepted.persona, accepted.session, accepted.said
            ),
        );
    }
    for reported in &report.reported {
        line(
            &mut out,
            "\u{2713}",
            OK,
            &format!("reported {} — {}", reported.task, reported.record),
        );
    }
    for skipped in &report.skipped {
        line(&mut out, "\u{00b7}", DIM, &skipped.reason);
    }
    for refused in &report.refused {
        line(&mut out, "\u{2715}", ERROR, &refused.reason);
    }
    if report.is_empty() {
        out.push_str(&format!(
            "{DIM}the board holds nothing in the column, and nothing of this lane's is \
             waiting{RESET}\n"
        ));
    }
    out
}

/// The bots-surface card as text, for the CLI: the five receipts, the
/// lane's last probe facts, and the press. The face renders the same
/// document; this is the terminal's own read of it.
pub fn render_card(document: &Value) -> String {
    let width = crate::cards::CARD_WIDTH;
    let installed = document["installed"].as_bool().unwrap_or(false);
    let version = document["version"].as_str().unwrap_or("unknown");
    let mcp = document["mcp"].as_bool().unwrap_or(false);
    let board = document["board"].as_bool().unwrap_or(false);
    let mut out = String::new();
    let (dot, color) = if installed && mcp {
        ("\u{2713}", OK)
    } else if installed {
        ("\u{25b3}", ERROR)
    } else {
        ("\u{25cb}", DIM)
    };
    let state = if installed && mcp {
        "ok"
    } else if installed {
        "error"
    } else {
        "unavailable"
    };
    let header_right = format!("{dot} {state}");
    let used = " kanzo board ".chars().count() + header_right.chars().count() + 2;
    out.push_str(&format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}kanzo board{RESET} {DIM}{dashes} {color}{header_right}{RESET} {DIM}\u{2510}{RESET}\n",
        dashes = crate::cards::rule(width.saturating_sub(used + 4).max(1)),
    ));
    let facts = format!(
        "v{version} · MCP {} · board {}",
        if mcp { "answers" } else { "did not answer" },
        if board {
            "answers".to_string()
        } else {
            format!("not running (127.0.0.1:{PORT})")
        }
    );
    for chunk in crate::skills::wrap_detail(&facts, width - 4) {
        out.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{chunk}{RESET}\n"));
    }
    for row in document["rows"].as_array().into_iter().flatten() {
        let label = row["label"].as_str().unwrap_or_default();
        let count = row["count"].as_u64().unwrap_or(0);
        let task = row["task"].as_str().unwrap_or_default();
        let persona = row["persona"].as_str().unwrap_or_default();
        let url = row["url"].as_str().unwrap_or_default();
        let text = if count == 0 {
            format!("{label:<12} 0 — no receipt yet")
        } else {
            let mut line = format!("{label:<12} {count} — {task}");
            if !persona.is_empty() {
                line.push_str(&format!(" · {persona}"));
            }
            if !url.is_empty() {
                line.push_str(&format!(" · {url}"));
            }
            line
        };
        let color = if count == 0 { DIM } else { TEXT };
        for (index, chunk) in crate::skills::wrap_detail(&text, width - 4)
            .iter()
            .enumerate()
        {
            let indent = if index == 0 { "" } else { "  " };
            out.push_str(&format!(
                "{DIM}\u{2502}{RESET} {color}{indent}{chunk}{RESET}\n"
            ));
        }
    }
    let press = document["press"].as_str().unwrap_or("fleet kanzo pass");
    let torn = document["torn"].as_u64().unwrap_or(0);
    if torn > 0 {
        out.push_str(&format!(
            "{DIM}\u{2502}{RESET} {ERROR}{torn} ledger line(s) did not parse — a crashed pass's torn \
             tail, or an edit{RESET}\n"
        ));
    }
    out.push_str(&format!(
        "{DIM}\u{2502}{RESET} {DIM}press: {press} — a pass is on demand, never a cron{RESET}\n"
    ));
    out.push_str(&format!(
        "{DIM}\u{2514}{}\u{2518}{RESET}\n",
        crate::cards::rule(width - 2)
    ));
    out
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(key: &str, title: &str, description: &str, tags: &[&str]) -> TaskDetail {
        TaskDetail {
            key: key.to_string(),
            title: title.to_string(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            status: "todo".to_string(),
            human: false,
            url: String::new(),
            description: description.to_string(),
            comments: Vec::new(),
        }
    }

    fn persona(name: &str, description: &str) -> Persona {
        Persona {
            name: name.to_string(),
            description: description.to_string(),
            words: tokens(description).into_iter().collect(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-kanzo-{name}-{}-{}",
            std::process::id(),
            crate::boards::now_stamp().replace([' ', ':'], "-")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the scratch home");
        dir
    }

    // --- parsing: kanzo's own text, measured -----------------------------

    #[test]
    fn a_task_line_parses_key_status_title_tags_and_comment_count() {
        let task = parse_task_line("probe#1 [todo] Ship the export CSV {ui, export} (1 comment)")
            .expect("a task line");
        assert_eq!(task.key, "probe#1");
        assert_eq!(task.status, "todo");
        assert_eq!(task.title, "Ship the export CSV");
        assert_eq!(task.tags, vec!["ui", "export"]);
        assert_eq!(task.comments, 1);
        let plain = parse_task_line("probe#2 [doing] Refactor the router").expect("a plain line");
        assert_eq!(plain.title, "Refactor the router");
        assert!(plain.tags.is_empty());
        assert_eq!(plain.comments, 0);
        // The human note and the empty column are not tasks.
        assert!(
            parse_task_line("(1 human task not shown: for the user, not for agents)").is_none()
        );
        assert!(parse_task_line("No tasks in probe.").is_none());
        assert!(parse_task_line("").is_none());
    }

    #[test]
    fn a_task_detail_parses_the_human_mark_url_and_comments() {
        let text = "probe#1 Ship the export CSV {ui, export}\n\
                    status: todo, created 2026-11-22 04:37 UTC\n\
                    url: https://github.com/acme/probe/pull/1\n\
                    \n\
                    Add a CSV export to the reports page.\n\
                    \n\
                    Comments (2):\n\
                    - me, 2026-11-22 04:38 UTC: accept — go ahead\n\
                    - writer, 2026-11-22 04:39 UTC: the CSV lands behind the export flag";
        let detail = parse_detail("probe#1", text).expect("a detail");
        assert_eq!(detail.key, "probe#1");
        assert_eq!(detail.title, "Ship the export CSV");
        assert_eq!(detail.tags, vec!["ui", "export"]);
        assert_eq!(detail.status, "todo");
        assert!(!detail.human);
        assert_eq!(detail.url, "https://github.com/acme/probe/pull/1");
        assert!(detail.description.contains("CSV export"));
        assert_eq!(detail.comments.len(), 2);
        assert_eq!(detail.comments[0].author, "me");
        assert!(detail.comments[0].body.starts_with("accept"));
        assert_eq!(detail.comments[1].author, "writer");
        // The human mark is kanzo's own sentence, read as the guard.
        let human = "probe#3 Sign the vendor contract\n\
                     status: todo, human: for the user, not for agents, created 2026-11-22 04:37 UTC";
        let human = parse_detail("probe#3", human).expect("a human detail");
        assert!(human.human);
        assert_eq!(human.comments.len(), 0);
        // A non-task answer is a named refusal, not an empty task.
        assert!(parse_detail("x", "There are no projects yet.").is_err());
    }

    // --- the law ---------------------------------------------------------

    #[test]
    fn a_human_task_is_refused_by_name_and_never_offered_to() {
        let mut human = detail("probe#3", "Sign the vendor contract", "legal", &[]);
        human.human = true;
        let reason = eligible(&human).expect_err("a human task is refused");
        assert!(reason.contains("probe#3"), "{reason}");
        assert!(
            reason.contains("for the user, not for agents"),
            "the refusal is kanzo's own sentence: {reason}"
        );
        assert!(eligible(&detail("probe#1", "x", "", &[])).is_ok());
    }

    #[test]
    fn consent_needs_the_owners_accept_after_the_synthesis() {
        let mut task = detail("probe#1", "Ship the export CSV", "", &[]);
        task.comments = vec![
            Comment {
                author: "writer".into(),
                at: "2026-11-22 04:37 UTC".into(),
                body: "I can help with this by writing the exporter".into(),
            },
            Comment {
                author: "planner".into(),
                at: "2026-11-22 04:38 UTC".into(),
                body: "Plan: 1. write the exporter — writer. Accept: comment `accept`".into(),
            },
        ];
        // Two comments existed when the synthesis posted; the accept must
        // land after them.
        let reason = consent(&task, 2, true).expect_err("no accept yet");
        assert!(reason.contains("no accept from the owner"), "{reason}");
        assert!(reason.contains("an offer is not a job"), "{reason}");
        // An agent that names itself "accept" is not the owner.
        task.comments.push(Comment {
            author: "writer".into(),
            at: "2026-11-22 04:39 UTC".into(),
            body: "accept — starting now".into(),
        });
        assert!(
            consent(&task, 2, true).is_err(),
            "a persona's own comment never counts as the owner's consent"
        );
        // The owner's own comment, authored `me`, is the accept.
        task.comments.push(Comment {
            author: OWNER_AUTHOR.into(),
            at: "2026-11-22 04:40 UTC".into(),
            body: "accept — go ahead".into(),
        });
        let accepted = consent(&task, 2, true).expect("the owner's accept");
        assert!(accepted.body.starts_with("accept"));
        // An accept BEFORE the synthesis is stale: it cannot consent to an
        // offer it predates.
        let mut stale = detail("probe#1", "Ship it", "", &[]);
        stale.comments = vec![Comment {
            author: OWNER_AUTHOR.into(),
            at: "2026-11-22 04:30 UTC".into(),
            body: "accept".into(),
        }];
        assert!(consent(&stale, 1, true).is_err());
        // The red-proof: with the accept-wait off there is no consent to
        // read, and the refusal says so by name.
        let reason = consent(&stale, 0, false).expect_err("the accept-wait is off");
        assert!(reason.contains("the accept-wait is off"), "{reason}");
        assert!(reason.contains("an offer is not a job"), "{reason}");
        assert!(reason.contains("probe#1"), "{reason}");
    }

    #[test]
    fn a_synthesis_must_cite_every_voice_and_name_the_disagreements() {
        let posted = vec![
            Perspective {
                persona: "writer".into(),
                body: "Approach: write the exporter".into(),
            },
            Perspective {
                persona: "reviewer".into(),
                body: "Worries: the CSV needs a schema".into(),
            },
        ];
        // The red-proof: a synthesis that speaks for a persona that did
        // not speak is refused by name.
        let invented = "Plan:\n1. write the exporter — writer\nAccept: comment `accept`";
        let reason = synthesis_covers(invented, &posted).expect_err("a missing citation");
        assert!(
            reason.contains("spoke for personas that did not speak"),
            "{reason}"
        );
        assert!(reason.contains("reviewer"), "{reason}");
        // Citing both (one line per author) but naming no disagreement is
        // still a failed synthesis: the plan may not invent a consensus.
        let agreed = "writer: write it.\nreviewer: add a schema.\nPlan:\n1. write — writer";
        let reason = synthesis_covers(agreed, &posted).expect_err("no disagreements line");
        assert!(reason.contains("names no disagreements"), "{reason}");
        // A name that only appears inside a plan step is not a citation:
        // the attribution the contract asks for is a line.
        let mention_only = "writer: write the exporter. reviewer: add a schema.\n\
                            Disagreements: reviewer wants a schema first.\n\
                            Plan:\n1. write — writer\n2. schema — reviewer";
        let reason = synthesis_covers(mention_only, &posted).expect_err("a mention is not a line");
        assert!(
            reason.contains("spoke for personas that did not speak"),
            "{reason}"
        );
        assert!(reason.contains("reviewer"), "{reason}");
        // The whole contract, satisfied: one digest line per author, the
        // disagreements named, and the plan's steps attributed.
        let whole = "writer: write the exporter.\n\
                     reviewer: add a schema first.\n\
                     Disagreements: reviewer wants a schema first; writer does not.\n\
                     Plan:\n1. write the exporter — writer\n2. add the schema — reviewer\n\
                     Accept: comment `accept` on this task to start the plan.";
        assert!(synthesis_covers(whole, &posted).is_ok());
        // One voice needs no disagreement line — the degenerate flow does
        // not fork.
        let one = vec![Perspective {
            persona: "writer".into(),
            body: "Approach: write it".into(),
        }];
        assert!(synthesis_covers("writer: write it.\nPlan:\n1. write — writer", &one).is_ok());
        // No perspective at all is not a synthesis.
        assert!(synthesis_covers("anything", &[]).is_err());
    }

    // --- the roster ------------------------------------------------------

    #[test]
    fn the_roster_matches_on_its_own_declared_words_and_names_a_miss() {
        let roster = vec![
            persona("writer", "writes prose, documentation and copy"),
            persona("engineer", "builds code, migrations and tests"),
        ];
        let docs = detail(
            "probe#2",
            "Write the migration guide",
            "docs for the migration",
            &["docs"],
        );
        let scores: Vec<usize> = roster.iter().map(|one| score(one, &docs)).collect();
        assert!(scores[0] > 0, "the writer scores on docs: {scores:?}");
        assert!(
            scores[1] > 0,
            "the engineer scores on migration: {scores:?}"
        );
        let picked = pick(&docs, &roster, None, 0).expect("two capable voices");
        assert_eq!(picked.perspectives.len(), 2);
        // The convener defaults to the top scorer, and a budget cuts the
        // voices without silencing the convener.
        let capped = pick(&docs, &roster, None, 1).expect("a capped pick");
        assert_eq!(capped.perspectives.len(), 1);
        assert_eq!(capped.perspectives[0].name, picked.convener.name);
        // A task no persona declares a word for is named, never assigned.
        let ledger_work = detail("probe#9", "Reconcile the ledger", "", &["finance"]);
        assert!(pick(&ledger_work, &roster, None, 0).is_none());
        // An owner-named convener that does not resolve falls back by name.
        let fallback = pick(&docs, &roster, Some("ghost"), 0).expect("a pick");
        assert!(fallback.convener_fallback.is_some());
        assert!(fallback
            .convener_fallback
            .as_deref()
            .unwrap()
            .contains("ghost"));
    }

    // --- the ledger ------------------------------------------------------

    #[test]
    fn the_ledger_is_idempotent_by_task_id_and_names_a_torn_tail() {
        let home = scratch("ledger");
        let task = detail("probe#1", "Ship the export CSV", "", &[]);
        let mut ledger = Ledger::load(&home).expect("an empty ledger");
        assert!(!ledger.saw("probe#1", EVENT_PERSPECTIVE, Some("writer")));
        ledger
            .record(EVENT_PERSPECTIVE, &task, "writer", "Approach: write it", 0)
            .unwrap();
        assert!(ledger.saw("probe#1", EVENT_PERSPECTIVE, Some("writer")));
        assert!(!ledger.saw("probe#1", EVENT_PERSPECTIVE, Some("reviewer")));
        // A fresh read sees the same facts — the file is the record.
        let reloaded = Ledger::load(&home).expect("the ledger reloads");
        assert_eq!(reloaded.entries.len(), 1);
        assert_eq!(reloaded.entries[0].persona, "writer");
        // A torn last line is skipped and counted; the next append closes
        // the tail rather than being swallowed by it.
        let mut text = std::fs::read_to_string(Ledger::path_for(&home)).unwrap();
        text.push_str("{\"at\":\"torn");
        std::fs::write(Ledger::path_for(&home), text).unwrap();
        let mut torn = Ledger::load(&home).expect("a torn tail is not fatal");
        assert_eq!(torn.entries.len(), 1);
        assert_eq!(torn.torn, 1, "the torn line is counted, not silent");
        torn.record(EVENT_OFFER, &task, "writer", "Plan: 1. write it", 1)
            .unwrap();
        let after = Ledger::load(&home).unwrap();
        assert_eq!(after.entries.len(), 2, "the append landed on its own line");
        assert_eq!(after.torn, 1, "the torn line stays counted");
        // A file someone edited is counted too — visible, never dropped.
        std::fs::write(
            Ledger::path_for(&home),
            "not json\n{\"at\":\"x\",\"task\":\"probe#1\",\"event\":\"read\"}\n",
        )
        .unwrap();
        let edited = Ledger::load(&home).unwrap();
        assert_eq!(edited.torn, 1);
        assert_eq!(edited.entries.len(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_cards_five_rows_carry_a_receipt_or_say_they_have_none() {
        let home = scratch("card");
        let task = detail("probe#1", "Ship the export CSV", "", &[]);
        let mut ledger = Ledger::load(&home).unwrap();
        ledger
            .record(
                EVENT_OFFER,
                &task,
                "planner",
                "Plan: 1. write it — writer",
                2,
            )
            .unwrap();
        ledger
            .record(EVENT_ACCEPT, &task, "planner", "me — accept", 3)
            .unwrap();
        let card = card(&home);
        let rows = card["rows"].as_array().expect("the five rows");
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0]["label"], "offers made");
        assert_eq!(rows[0]["count"], 1);
        assert_eq!(rows[0]["task"], "probe#1");
        assert_eq!(rows[0]["persona"], "planner");
        assert_eq!(rows[1]["label"], "accepted");
        assert_eq!(rows[1]["count"], 1);
        // Zero draws empty: a row with no receipt says so by its own zero
        // and no task, never an invented one.
        assert_eq!(rows[3]["label"], "helped");
        assert_eq!(rows[3]["count"], 0);
        assert_eq!(rows[3]["task"], "");
        assert_eq!(card["press"], "fleet kanzo pass");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_settings_are_the_owners_lane_keys_with_named_defaults() {
        let defaults = settings_from(&BTreeMap::new());
        assert_eq!(defaults.column, "todo");
        assert_eq!(defaults.done, "done");
        assert_eq!(defaults.budget, 0);
        assert!(defaults.accept_wait);
        let mut env = BTreeMap::new();
        env.insert("KANZO_COLUMN".to_string(), "backlog".to_string());
        env.insert("KANZO_DONE_COLUMN".to_string(), "review".to_string());
        env.insert("KANZO_CONVENER".to_string(), "planner".to_string());
        env.insert("KANZO_BUDGET".to_string(), "2".to_string());
        env.insert("KANZO_ACCEPT_WAIT".to_string(), "off".to_string());
        let read = settings_from(&env);
        assert_eq!(read.column, "backlog");
        assert_eq!(read.done, "review");
        assert_eq!(read.convener.as_deref(), Some("planner"));
        assert_eq!(read.budget, 2);
        assert!(!read.accept_wait);
    }

    #[test]
    fn a_comment_is_bounded_at_a_word_boundary() {
        let long = "word ".repeat(400);
        let cut = bounded(&long, 50);
        assert!(cut.chars().count() <= 51, "{cut}");
        assert!(cut.ends_with('\u{2026}'));
        assert!(!cut.contains("wor\u{2026}"), "no mid-word cut: {cut}");
        assert_eq!(bounded("short", 50), "short");
    }

    // --- the live lane: a real kanzo on this machine ---------------------

    /// The live half's gate: no kanzo binary is a named skip, never a
    /// failure — the same shape every machine-dependent check has.
    fn live_bin() -> Option<PathBuf> {
        match binary() {
            Ok(bin) => Some(bin),
            Err(reason) => {
                eprintln!("kanzo lane: skipped — {reason}");
                None
            }
        }
    }

    /// A scripted voice: the live tests' mouths. It speaks the contract's
    /// shape and never invents a citation it was not given.
    struct Scripted {
        /// When set, the synthesis omits this persona's name — the
        /// coverage red-proof.
        omit: Option<String>,
    }

    impl Voice for Scripted {
        async fn perspective(
            &self,
            persona: &Persona,
            task: &TaskDetail,
        ) -> Result<String, String> {
            Ok(format!(
                "Approach: {} would take {} by writing the exporter.\nNeeds: the report's schema.\nWorries: the CSV's encoding.",
                persona.name, task.key
            ))
        }

        async fn synthesis(
            &self,
            convener: &Persona,
            task: &TaskDetail,
            perspectives: &[Perspective],
        ) -> Result<String, String> {
            let mut out = String::new();
            for perspective in perspectives {
                if self.omit.as_deref() == Some(perspective.persona.as_str()) {
                    continue;
                }
                out.push_str(&format!(
                    "{}: {}\n",
                    perspective.persona,
                    perspective.body.lines().next().unwrap_or_default()
                ));
            }
            if perspectives.len() > 1 {
                out.push_str("Disagreements: the encoding is unsettled.\n");
            }
            out.push_str(&format!(
                "Plan:\n1. write the exporter — {}\nAccept: comment `accept` on this task to start the plan.",
                convener.name
            ));
            let _ = task;
            Ok(out)
        }
    }

    async fn seed(bin: &Path, home: &Path, slug: &str) -> Board {
        let spec = spec(bin, Some(home));
        let server = McpServer::spawn(&spec).expect("kanzo mcp starts");
        server.initialize().await.expect("initialize");
        let board = Board { server };
        board
            .create_project(slug, "Kanzo lane live")
            .await
            .expect("the project");
        board
            .create_task(json!({
                "title": "Ship the export CSV",
                "description": "Add a CSV export to the reports page.",
                "project": slug,
                "tags": "ui, export",
            }))
            .await
            .expect("a task");
        board
            .create_task(json!({
                "title": "Sign the vendor contract",
                "project": slug,
                "human": true,
            }))
            .await
            .expect("a human task");
        board
    }

    /// A home carrying the two roles the pass matches on, and a roster
    /// for the claim's session model.
    fn persona_home(name: &str) -> PathBuf {
        let home = scratch(name);
        std::fs::create_dir_all(crate::roles::roles_dir(&home)).unwrap();
        for (role, description) in [
            (
                "writer",
                "writes the exporter, the CSV and its documentation",
            ),
            ("reviewer", "reviews the exporter and the CSV's encoding"),
        ] {
            crate::roles::write(
                &home,
                role,
                &crate::roles::RoleFields {
                    description: description.to_string(),
                    tools: Vec::new(),
                    skills: Vec::new(),
                    shapes: Vec::new(),
                    workload: None,
                    overlay: String::new(),
                },
            )
            .expect("the role");
        }
        home
    }

    #[tokio::test]
    async fn the_live_probe_reports_the_four_facts_it_measured() {
        let Some(bin) = live_bin() else { return };
        let home = scratch("probe-home");
        let health = probe_with(Some(&home)).await;
        assert!(health.present, "the binary resolved: {health:?}");
        assert_eq!(health.binary.as_deref(), Some(bin.as_path()));
        assert!(
            health.version.is_some(),
            "the version was measured: {health:?}"
        );
        assert!(health.mcp, "kanzo mcp answered: {health:?}");
        assert!(
            health.missing_tools.is_empty(),
            "every required tool is there: {health:?}"
        );
        assert!(health.tools.contains(&"add_comment".to_string()));
        assert!(health.usable());
        // The board half is named either way — a board that is not running
        // is not a broken lane, and the detail says which it is.
        if !health.board {
            assert!(health.usable(), "MCP alone is usable: {health:?}");
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_seeded_board_round_trips_and_a_second_pass_is_a_no_op() {
        let Some(bin) = live_bin() else { return };
        let board_home = scratch("board");
        let home = persona_home("roundtrip-home");
        let board = seed(&bin, &board_home, "lane").await;
        let settings = Settings {
            project: Some("lane".to_string()),
            ..Settings::default()
        };
        let voices = Scripted { omit: None };

        // The first pass: two perspectives in their own names, one
        // synthesis by the convener, one offer.
        let mut report = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        let tasks = board.tasks(Some("lane"), "todo").await.unwrap();
        assert_eq!(
            tasks.len(),
            1,
            "the human task is not in the column: {tasks:?}"
        );
        for task in &tasks {
            report.read.push(task.key.clone());
            offer_one(
                &home,
                &board,
                &voices,
                &settings,
                &mut ledger,
                &mut report,
                &task.key,
            )
            .await
            .unwrap();
        }
        assert_eq!(report.offers.len(), 1, "{report:?}");
        let offer = &report.offers[0];
        assert_eq!(offer.task, "lane#1");
        assert_eq!(offer.personas.len(), 2, "both personas spoke: {offer:?}");
        let detail = board.task("lane#1").await.unwrap();
        assert_eq!(detail.comments.len(), 3, "two perspectives + one synthesis");
        let authors: Vec<&str> = detail
            .comments
            .iter()
            .map(|comment| comment.author.as_str())
            .collect();
        assert!(authors.contains(&"writer") && authors.contains(&"reviewer"));
        // Every perspective is attributed, and the synthesis cites both.
        let synthesis = detail.comments.last().unwrap();
        assert!(synthesis.body.contains("writer") && synthesis.body.contains("reviewer"));
        assert!(synthesis.body.contains("Accept"));

        // The second pass is a no-op: the ledger makes it so, by name.
        let mut again = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        offer_one(
            &home,
            &board,
            &voices,
            &settings,
            &mut ledger,
            &mut again,
            "lane#1",
        )
        .await
        .unwrap();
        assert!(again.offers.is_empty(), "{again:?}");
        assert!(again
            .skipped
            .iter()
            .any(|skip| skip.reason.contains("already carries this lane's offer")));
        let after = board.task("lane#1").await.unwrap();
        assert_eq!(after.comments.len(), 3, "no double-offering");

        // The human task is refused by name when the lane holds its id.
        let mut human = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        offer_one(
            &home,
            &board,
            &voices,
            &settings,
            &mut ledger,
            &mut human,
            "lane#2",
        )
        .await
        .unwrap();
        assert_eq!(human.refused.len(), 1, "{human:?}");
        assert!(human.refused[0]
            .reason
            .contains("for the user, not for agents"));
        let untouched = board.task("lane#2").await.unwrap();
        assert_eq!(
            untouched.comments.len(),
            0,
            "a human task is never commented on"
        );

        board.close().await;
        let _ = std::fs::remove_dir_all(&board_home);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn the_owners_accept_is_the_only_trigger_and_the_record_reports_it() {
        let Some(bin) = live_bin() else { return };
        let board_home = scratch("board-accept");
        let home = persona_home("accept-home");
        let board = seed(&bin, &board_home, "accept").await;
        let settings = Settings {
            project: Some("accept".to_string()),
            ..Settings::default()
        };
        let voices = Scripted { omit: None };
        let mut ledger = Ledger::load(&home).unwrap();
        let mut report = PassReport::default();
        offer_one(
            &home,
            &board,
            &voices,
            &settings,
            &mut ledger,
            &mut report,
            "accept#1",
        )
        .await
        .unwrap();
        assert_eq!(report.offers.len(), 1, "{report:?}");

        // RED-PROOF, the whole law: with the accept-wait off the lane
        // cannot read consent, so the offer does not run — named.
        let mut off = settings.clone();
        off.accept_wait = false;
        let mut refused = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        accept_all(&home, &board, &off, &mut ledger, &mut refused)
            .await
            .unwrap();
        assert!(refused.accepted.is_empty(), "{refused:?}");
        assert!(
            refused
                .skipped
                .iter()
                .any(|one| one.reason.contains("the accept-wait is off")),
            "{refused:?}"
        );
        assert!(
            !ledger.saw("accept#1", EVENT_CLAIM, None),
            "nothing is claimed without the owner's consent"
        );

        // The owner accepts on the board — the board's own author, `me`.
        board
            .comment("accept#1", OWNER_AUTHOR, "accept — go ahead")
            .await
            .unwrap();
        let mut accepted = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        accept_all(&home, &board, &settings, &mut ledger, &mut accepted)
            .await
            .unwrap();
        assert_eq!(accepted.accepted.len(), 1, "{accepted:?}");
        let claim = &accepted.accepted[0];
        assert_eq!(claim.task, "accept#1");
        // The claimed persona's session carries the task's id and URL on
        // its own standing objective — the log says where the work came
        // from, so a reader of the log does not have to ask.
        let path = crate::session::session_path(&home, &claim.session);
        let log = std::fs::read_to_string(&path).expect("the claim's log");
        assert!(log.contains("kanzo accept#1"), "{log}");
        assert!(log.contains("board: http://127.0.0.1"), "{log}");

        // A claim is not a completion: with the session still open there
        // is nothing to report, and the card keeps the card where it is.
        let mut ledger = Ledger::load(&home).unwrap();
        let early = report_finished(&home, &board, &settings, &mut ledger)
            .await
            .unwrap();
        assert!(early.is_empty(), "{early:?}");

        // The session ends; the next pass reports its own record line and
        // moves the card.
        let mut log = crate::session::Session::open(&path).unwrap();
        log.record(&crate::session::Record::End {
            stop_reason: "completed".to_string(),
            turns: 3,
            served_model: Some("test-model".to_string()),
        })
        .unwrap();
        let mut ledger = Ledger::load(&home).unwrap();
        let reported = report_finished(&home, &board, &settings, &mut ledger)
            .await
            .unwrap();
        assert_eq!(reported.len(), 1, "{reported:?}");
        assert!(reported[0].record.contains("3 turns"), "{:?}", reported[0]);
        assert!(reported[0].record.contains("test-model"));
        let detail = board.task("accept#1").await.unwrap();
        assert_eq!(detail.status, "done", "the card moved to the done column");
        assert!(
            detail
                .comments
                .iter()
                .any(|comment| comment.body.starts_with("done — session")),
            "{:?}",
            detail.comments
        );
        // Reporting once is reporting forever: the ledger makes the second
        // pass a no-op.
        let mut ledger = Ledger::load(&home).unwrap();
        let again = report_finished(&home, &board, &settings, &mut ledger)
            .await
            .unwrap();
        assert!(again.is_empty(), "a helped task is not re-reported");

        board.close().await;
        let _ = std::fs::remove_dir_all(&board_home);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_synthesis_that_invents_a_voice_is_refused_and_nothing_posts() {
        let Some(bin) = live_bin() else { return };
        let board_home = scratch("board-cover");
        let home = persona_home("cover-home");
        let board = seed(&bin, &board_home, "cover").await;
        let settings = Settings {
            project: Some("cover".to_string()),
            ..Settings::default()
        };
        // The synthesis omits the reviewer: it speaks for a persona that
        // did not speak.
        let voices = Scripted {
            omit: Some("reviewer".to_string()),
        };
        let mut report = PassReport::default();
        let mut ledger = Ledger::load(&home).unwrap();
        offer_one(
            &home,
            &board,
            &voices,
            &settings,
            &mut ledger,
            &mut report,
            "cover#1",
        )
        .await
        .unwrap();
        assert!(report.offers.is_empty(), "{report:?}");
        assert!(
            report
                .refused
                .iter()
                .any(|one| one.reason.contains("spoke for personas that did not speak")),
            "{report:?}"
        );
        // The perspectives posted; the synthesis did not.
        let detail = board.task("cover#1").await.unwrap();
        assert_eq!(detail.comments.len(), 2, "two perspectives, no synthesis");
        assert!(!detail
            .comments
            .iter()
            .any(|comment| comment.body.contains("Plan:")));
        board.close().await;
        let _ = std::fs::remove_dir_all(&board_home);
        let _ = std::fs::remove_dir_all(&home);
    }
}
