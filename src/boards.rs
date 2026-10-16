//! The boards: three read-only surfaces over this machine's own sources,
//! mounted in the TUI's board pane.
//!
//! The three boards and their sources, exactly as the fork's
//! `dsh-router-board`, `dsh-second-brain`, and `dsh-tower` read them:
//!
//! | Board | Source | What it renders |
//! |---|---|---|
//! | router | the router kit's operator API at `127.0.0.1:8300` (`/api/state`, `/api/usage`, `/api/suggest`) | the workload tier chains with their candidates, the judge, the usage totals and per-model bars, the recent calls with their state dots |
//! | second-brain | the six derived state files under the second-brain home (`gates_state`, `harness_state`, `loops_state`, `reports_state`, `maturity_state`, `tower_data`) | the earned-autonomy gate board, the HITL inbox, the quality loops and their graded rows, the maturity sparkline, the doing/conflict counts |
//! | tower | git facts per project under `~/Projects` | per-project cards: branch, dirty dot, head commit, the trailing-thirty-day count, ahead/behind |
//!
//! **The snapshot contract.** Every board keeps the last snapshot that
//! landed. A refused read — a dead port, a bad token, an unreadable or
//! missing state file, a git failure — keeps that snapshot visible and
//! names the error in the pane on one line beside the time of the last
//! good read. A board that has never read successfully shows its empty
//! state naming the reason. Nothing here writes to any source: the reads
//! are loopback HTTP GETs, file reads, and `git` calls that only inspect.
//!
//! **The refresh.** [`spawn_refresher`] owns one thread with its own
//! current-thread runtime — the `spawn_consolidator`/`open_lanes_view`
//! precedent — reads all three boards concurrently at start and every
//! [`BOARD_REFRESH_SECS`], and reads again the moment a
//! [`BoardCommand::Refresh`] arrives (the cycle keybinding). Every read is
//! bounded, so a slow or hung source names itself instead of stalling the
//! interface: the event loop only ever receives finished reads over a
//! channel. The bounds are per-read *and* per-scan —
//! [`TOWER_SCAN_BUDGET_SECS`] caps a whole tower scan and
//! [`BRAIN_READ_BUDGET_SECS`] the whole six-file second-brain read —
//! because a sum of bounded reads is not itself bounded, and a hung `git`
//! on one repository must not cost the whole cadence.
//!
//! **The token.** The router kit's operator API authenticates with its
//! `localToken`, which lives in the kit's own rendered runtime config —
//! `<kit home>/router/config.json`, resolved the way the kit resolves it
//! (`AGNOSTIC_ROUTER_DIR`, then `AGNOSTIC_ROUTER_KIT_HOME/router`, then
//! the kit's default home). The board reads that one file for the port
//! and the token and nothing else. An explicit `AGNOSTIC_ROUTER_TOKEN`
//! or `AGNOSTIC_ROUTER_URL` wins, resolved through the same
//! environment-then-`.env` chain every other credential in this crate
//! uses. The token is never written into a snapshot, a log line, or this
//! repository.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// --- the seam ----------------------------------------------------------------

/// How often the refresher reads all three boards. The fork's boards carry
/// their own cadence (60s for the router and second-brain boards, 30
/// minutes for the tower); one cadence for all three keeps the pane's
/// contract in one place, and the tower scan's cost (~2s of bounded `git`
/// calls over thirty repos) is what sets it.
pub const BOARD_REFRESH_SECS: u64 = 30;

/// The bound on one operator-API read. The fork's carrier uses an 8s
/// abort signal; a kit that has not answered by then is a named refusal,
/// not a hung pane.
pub const READ_TIMEOUT_SECS: u64 = 8;

/// The bound on one `git` call. The fork's portfolio scan uses 5s.
pub const GIT_TIMEOUT_SECS: u64 = 5;

/// The aggregate bound on one whole tower scan. Each `git` call is
/// bounded on its own, but thirty projects × five calls is a sum no
/// single bound covers — up to twelve minutes of wall time on a machine
/// whose `git` always spends its full bound. Once this budget is spent
/// the scan stops: the projects read so far render, and the snapshot
/// names how many it left unread, so a hung `git` on one repository
/// costs the budget rather than the cadence behind it.
pub const TOWER_SCAN_BUDGET_SECS: u64 = 20;

/// The aggregate bound on the second-brain board's six-file read, on the
/// same rule: a sum of bounded reads is not itself bounded. Once it is
/// spent no further file is read, and each unread file names the budget
/// rather than the cycle stalling behind a state file on a hung mount.
pub const BRAIN_READ_BUDGET_SECS: u64 = 10;

/// The bound on one state-file read. A file on a hung mount is a named
/// refusal, not a stalled cycle: the read runs on its own thread and the
/// bound expiring names it, the `git` helper's own drain-thread pattern.
pub const FILE_READ_TIMEOUT_SECS: u64 = 5;

/// The tower scan's bound: at most this many immediate subdirectories of
/// the projects root are read for git facts. A machine with more projects
/// than this names the rest rather than spawning an unbounded number of
/// children on a background thread.
pub const MAX_PROJECTS: usize = 64;

/// The kit's own default port, used when no rendered config resolves.
pub const DEFAULT_ROUTER_PORT: u64 = 8300;

/// The three boards, in the order the cycle keybinding walks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardKind {
    Router,
    Brain,
    Tower,
}

impl BoardKind {
    /// Every board, in cycle order.
    pub const ALL: [BoardKind; 3] = [BoardKind::Router, BoardKind::Brain, BoardKind::Tower];

    /// The board's name, as the pane title and the status bar carry it.
    pub fn label(&self) -> &'static str {
        match self {
            BoardKind::Router => "router",
            BoardKind::Brain => "second-brain",
            BoardKind::Tower => "tower",
        }
    }

    /// The next board in the cycle.
    pub fn next(&self) -> BoardKind {
        match self {
            BoardKind::Router => BoardKind::Brain,
            BoardKind::Brain => BoardKind::Tower,
            BoardKind::Tower => BoardKind::Router,
        }
    }

    /// The board's index in [`BoardKind::ALL`], the slot it owns.
    fn index(&self) -> usize {
        match self {
            BoardKind::Router => 0,
            BoardKind::Brain => 1,
            BoardKind::Tower => 2,
        }
    }
}

/// One board's snapshot: the data its source carried at the instant it
/// was read. Each variant carries its own `read_at`, the minute of the
/// last good read, which is what the pane names beside a refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum BoardData {
    Router(RouterSnapshot),
    Brain(BrainSnapshot),
    Tower(TowerSnapshot),
}

impl BoardData {
    /// The minute this snapshot was read.
    pub fn read_at(&self) -> &str {
        match self {
            BoardData::Router(snapshot) => &snapshot.read_at,
            BoardData::Brain(snapshot) => &snapshot.read_at,
            BoardData::Tower(snapshot) => &snapshot.read_at,
        }
    }
}

/// One read's outcome inside a snapshot: the data that landed, or the
/// named reason it did not. The fork's `BrainRead<T> = T | { reason }`,
/// ported: one read that does not validate names itself and the others
/// still land.
#[derive(Debug, Clone, PartialEq)]
pub enum Section<T> {
    Landed(T),
    Refused(String),
}

impl<T> Section<T> {
    /// The data, when it landed.
    pub fn landed(&self) -> Option<&T> {
        match self {
            Section::Landed(data) => Some(data),
            Section::Refused(_) => None,
        }
    }

    /// The reason, when the read was refused.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Section::Landed(_) => None,
            Section::Refused(reason) => Some(reason),
        }
    }
}

/// One board's pane state: the last snapshot that landed, the named
/// reason the latest read did not, and whether a read is in flight.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Slot {
    pub snapshot: Option<BoardData>,
    pub error: Option<String>,
    pub loading: bool,
}

/// The three boards' pane state and which one is showing. Owned by the
/// TUI's state machine; the refresher thread only ever reports finished
/// reads into it.
#[derive(Debug, Clone, PartialEq)]
pub struct Boards {
    pub active: BoardKind,
    slots: [Slot; 3],
}

impl Default for Boards {
    fn default() -> Self {
        Boards::new()
    }
}

impl Boards {
    /// Three empty slots, the router board active, all three loading:
    /// the refresher's opening read is in flight from the moment the
    /// interface starts.
    pub fn new() -> Boards {
        let loading = Slot {
            snapshot: None,
            error: None,
            loading: true,
        };
        Boards {
            active: BoardKind::Router,
            slots: [loading.clone(), loading.clone(), loading],
        }
    }

    /// One board's slot.
    pub fn slot(&self, kind: BoardKind) -> &Slot {
        &self.slots[kind.index()]
    }

    /// One board's slot, mutably.
    pub fn slot_mut(&mut self, kind: BoardKind) -> &mut Slot {
        &mut self.slots[kind.index()]
    }

    /// Move to the next board and mark it for an immediate read. The
    /// cadence already covers every board, so this is about the one the
    /// user just asked for: it is read now, not at the next tick.
    pub fn cycle(&mut self) {
        self.active = self.active.next();
        self.slot_mut(self.active).loading = true;
    }

    /// Land one finished read. A refusal keeps the last snapshot and
    /// names the reason; a success clears both the reason and the
    /// in-flight flag.
    pub fn apply(&mut self, kind: BoardKind, outcome: Result<BoardData, String>) {
        let slot = self.slot_mut(kind);
        slot.loading = false;
        match outcome {
            Ok(snapshot) => {
                slot.snapshot = Some(snapshot);
                slot.error = None;
            }
            Err(reason) => slot.error = Some(reason),
        }
    }
}

/// Where each board reads from, resolved once when the interface starts.
#[derive(Debug, Clone)]
pub struct Sources {
    pub router: RouterSources,
    pub brain_home: Option<PathBuf>,
    pub tower_root: PathBuf,
    pub bounds: Bounds,
}

/// The router board's source: the operator API's base URL and the bearer
/// it authenticates with. The base URL never carries the token, and the
/// token is never carried anywhere but the request it authorizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterSources {
    pub base_url: String,
    pub token: Option<String>,
}

/// The bounds every bounded read runs under. `git_bin` is a path or a
/// name resolved on `PATH`; the tests point it at a script so a hung or
/// failing git is deterministic without touching the process
/// environment. The three durations are the per-read bounds; the two
/// budgets are the aggregate ones, because a sum of bounded reads is not
/// itself bounded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds {
    pub git_bin: String,
    pub git_timeout: Duration,
    pub max_projects: usize,
    /// The aggregate bound on one whole tower scan.
    pub scan_budget: Duration,
    /// The aggregate bound on the six-file second-brain read.
    pub brain_budget: Duration,
    /// The bound on one second-brain state-file read.
    pub file_read_timeout: Duration,
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            git_bin: "git".to_string(),
            git_timeout: Duration::from_secs(GIT_TIMEOUT_SECS),
            max_projects: MAX_PROJECTS,
            scan_budget: Duration::from_secs(TOWER_SCAN_BUDGET_SECS),
            brain_budget: Duration::from_secs(BRAIN_READ_BUDGET_SECS),
            file_read_timeout: Duration::from_secs(FILE_READ_TIMEOUT_SECS),
        }
    }
}

/// Resolve every board's source. The process environment wins, the fleet
/// home's `.env` is the fallback, and the compiled-in defaults are last —
/// the same resolution every credential in this crate uses. Nothing here
/// reads a key out of a repository.
pub fn resolve_sources(dotenv: &HashMap<String, String>) -> Sources {
    let setting = |name: &str| crate::home::resolve_setting(name, dotenv);
    Sources {
        router: router_sources(&setting),
        brain_home: brain_home(&setting),
        tower_root: tower_root(&setting),
        bounds: Bounds::default(),
    }
}

/// The router board's source. The kit's own rendered runtime config
/// carries both facts the board needs — the port and the local token —
/// so the board reads the same file the kit reads, and an explicit
/// override wins over it.
fn router_sources(setting: &impl Fn(&str) -> Option<String>) -> RouterSources {
    let config = router_config(setting).and_then(|value| value.as_object().cloned());
    let base_url = setting("AGNOSTIC_ROUTER_URL").unwrap_or_else(|| {
        let port = config
            .as_ref()
            .and_then(|config| config.get("port"))
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_ROUTER_PORT);
        format!("http://127.0.0.1:{port}")
    });
    // The token resolves the way every other credential does and is
    // dropped the moment the request it authorizes has been made.
    let token = setting("AGNOSTIC_ROUTER_TOKEN").or_else(|| {
        config
            .as_ref()
            .and_then(|config| config.get("localToken"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|token| !token.is_empty())
    });
    RouterSources { base_url, token }
}

/// The kit's rendered runtime config, `config.json`, resolved the way the
/// kit's own `lib/paths.mjs` resolves it: `AGNOSTIC_ROUTER_DIR` first,
/// then `AGNOSTIC_ROUTER_KIT_HOME/router`, then the kit's default home.
/// The last fallback is where this machine's running router actually
/// renders its runtime — the same file, one directory over from the
/// documented default — and it is named rather than assumed.
fn router_config(setting: &impl Fn(&str) -> Option<String>) -> Option<Value> {
    let path = router_config_path(setting)?;
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

/// The path of the kit's rendered runtime config, when one resolves.
fn router_config_path(setting: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(dir) = setting("AGNOSTIC_ROUTER_DIR") {
        return Some(PathBuf::from(dir).join("config.json"));
    }
    if let Some(home) = setting("AGNOSTIC_ROUTER_KIT_HOME") {
        return Some(PathBuf::from(home).join("router").join("config.json"));
    }
    let home = user_home()?;
    let kit_default = home
        .join(".agnostic-router-kit")
        .join("router")
        .join("config.json");
    if kit_default.is_file() {
        return Some(kit_default);
    }
    let deployed = home.join(".zcode").join("router").join("config.json");
    deployed.is_file().then_some(deployed)
}

/// The second-brain home holding the six derived state files. The fork's
/// default is `~/.second-brain`; the repo itself is the fallback when
/// that link is absent, and `SECOND_BRAIN_HOME` overrides both.
fn brain_home(setting: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(home) = setting("SECOND_BRAIN_HOME") {
        return Some(PathBuf::from(home));
    }
    let home = user_home()?;
    let linked = home.join(".second-brain");
    if linked.is_dir() {
        return Some(linked);
    }
    let repo = home.join("Projects").join("second-brain");
    repo.is_dir().then_some(repo)
}

/// The projects root the tower scans. The tower's own CLI names
/// `TOWER_LOG_PROJECTS_DIR` for exactly this, and `~/Projects` is the
/// default both the tower and the fork's portfolio scan use.
fn tower_root(setting: &impl Fn(&str) -> Option<String>) -> PathBuf {
    if let Some(root) = setting("TOWER_LOG_PROJECTS_DIR") {
        return PathBuf::from(root);
    }
    user_home()
        .map(|home| home.join("Projects"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The user's home directory, when the environment names one.
fn user_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

/// One UTC minute stamp, the crate's one time vocabulary: the time of a
/// board's last good read.
pub fn now_stamp() -> String {
    crate::cron::Minute::now().stamp()
}

// --- the router board --------------------------------------------------------

/// One board read of the router kit's operator API.
#[derive(Debug, Clone, PartialEq)]
pub struct RouterSnapshot {
    pub read_at: String,
    pub base_url: String,
    pub port: Option<u64>,
    /// The judge's mode and the model it runs on, when the kit names them.
    pub judge: Option<(String, Option<String>)>,
    /// Whether pay-as-you-go providers may be routed to, and their names.
    pub payg: Option<(bool, Vec<String>)>,
    /// The workload tier chains, in the kit's own order: each workload's
    /// resolved primary and the candidates behind it.
    pub tiers: Vec<TierChain>,
    pub totals: Option<UsageTotals>,
    /// The per-model rows, busiest first.
    pub models: Vec<ModelRow>,
    /// The recent calls, newest first.
    pub recent: Vec<RecentRow>,
    pub judge_counters: Option<JudgeCounters>,
    pub cascade: Option<CascadeCounters>,
    /// The workload mix, busiest first.
    pub workloads: Vec<(String, u64)>,
    /// The kit's own per-tier guidance sentences.
    pub guidance: Vec<(String, String)>,
    /// The reads that did not land, each named with its path and reason.
    pub refused: Vec<String>,
    /// The named shape drift: `(the route that carried it, what is
    /// absent from it)`. A kit upgrade that renames `resolved.workloads`
    /// or `judge.mode` would otherwise empty a section with no reason
    /// given, the way a state file that is not the shape it claims is
    /// named on the second-brain board.
    pub drift: Vec<(String, String)>,
}

/// One workload's tier chain: what the kit resolves it to and the
/// candidates it walks on failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierChain {
    pub tier: String,
    pub primary: String,
    pub candidates: Vec<String>,
}

/// The usage ledger's six counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageTotals {
    pub calls: u64,
    pub ok: u64,
    pub errors: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl UsageTotals {
    /// The share of calls that succeeded, in percent, when the ledger
    /// has seen any call at all.
    pub fn success_rate(&self) -> Option<u64> {
        (self.calls > 0).then(|| self.ok.saturating_mul(100) / self.calls)
    }
}

/// One model's usage row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRow {
    pub key: String,
    pub provider: String,
    pub model: String,
    pub calls: u64,
    pub errors: u64,
    pub total_tokens: u64,
}

/// One recent call, the board's "what was just served" row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRow {
    pub at: u64,
    pub provider: String,
    pub model: String,
    pub status: u16,
    pub workload: Option<String>,
    pub ms: Option<u64>,
    pub reason: Option<String>,
}

impl RecentRow {
    /// Whether the call succeeded, which is what its state dot says.
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// The judge's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeCounters {
    pub fresh: u64,
    pub cached: u64,
    pub errors: u64,
}

/// The cascade's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CascadeCounters {
    pub escalations: u64,
    pub cold: u64,
    pub failed: u64,
    pub compared: u64,
}

/// Read the router kit's operator API. `/api/state` is the spine: without
/// it there is nothing to render, so its refusal is the whole read's
/// refusal. `/api/usage` and `/api/suggest` are independent reads whose
/// refusals are named on the snapshot that still lands.
pub fn read_router(sources: &RouterSources, timeout: Duration) -> Result<RouterSnapshot, String> {
    let base = sources.base_url.trim_end_matches('/').to_string();
    let state = get_json(
        &format!("{base}/api/state"),
        sources.token.as_deref(),
        timeout,
    )
    .map_err(|reason| format!("the router board could not read the kit at {base}: {reason}"))?;
    let usage = get_json(
        &format!("{base}/api/usage"),
        sources.token.as_deref(),
        timeout,
    );
    let suggest = get_json(
        &format!("{base}/api/suggest"),
        sources.token.as_deref(),
        timeout,
    );

    let mut refused = Vec::new();
    let mut drift = state_drift(&state);
    let usage_read = match usage {
        Ok(usage) => parse_usage(&usage),
        Err(reason) => {
            refused.push(format!("/api/usage {reason}"));
            UsageRead::default()
        }
    };
    let guidance = match suggest {
        Ok(suggest) => parse_suggest(&suggest),
        Err(reason) => {
            refused.push(format!("/api/suggest {reason}"));
            Vec::new()
        }
    };
    drift.extend(usage_read.drift);

    Ok(RouterSnapshot {
        read_at: now_stamp(),
        base_url: base,
        port: number(&state, "port"),
        judge: parse_judge(&state),
        payg: parse_payg(&state),
        tiers: parse_tiers(&state),
        totals: usage_read.totals,
        models: usage_read.models,
        recent: usage_read.recent,
        judge_counters: usage_read.judge_counters,
        cascade: usage_read.cascade,
        workloads: usage_read.workloads,
        guidance,
        refused,
        drift,
    })
}

/// One bounded GET against the kit's operator API. The token rides the
/// `Authorization` header and appears in no error, because an error
/// names the URL and the kit's answer, never the credential.
fn get_json(url: &str, token: Option<&str>, timeout: Duration) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let mut request = agent.get(url).set("accept", "application/json");
    if let Some(token) = token {
        request = request.set("authorization", &format!("Bearer {token}"));
    }
    match request.call() {
        Ok(response) => response
            .into_json::<Value>()
            .map_err(|error| format!("answered a body the board cannot read: {error}")),
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            let named = error_message(&body).unwrap_or_else(|| body.trim().to_string());
            let named = if named.is_empty() {
                "no body".to_string()
            } else {
                named
            };
            Err(format!("answered HTTP {code}: {named}"))
        }
        Err(error) => Err(format!("could not be read: {error}")),
    }
}

/// The kit's own error sentence, out of `{"error":{"message":…}}`.
fn error_message(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("error")?
        .get("message")?
        .as_str()
        .map(str::to_string)
}

/// What the usage ledger's read produced: the sections that landed and
/// the shape drift it named on the way. The fork parses this payload
/// into totals, model rows, day rows, recent calls, and the counter
/// block; the board renders the totals, the models, the recent calls,
/// and the counters.
#[derive(Debug, Default)]
struct UsageRead {
    totals: Option<UsageTotals>,
    models: Vec<ModelRow>,
    recent: Vec<RecentRow>,
    judge_counters: Option<JudgeCounters>,
    cascade: Option<CascadeCounters>,
    workloads: Vec<(String, u64)>,
    /// The named shape drift: `(the route that carried it, what is
    /// absent from it)`.
    drift: Vec<(String, String)>,
}

fn parse_usage(usage: &Value) -> UsageRead {
    let route = "/api/usage".to_string();
    let mut drift = Vec::new();
    let totals = usage.get("totals").and_then(parse_totals);
    if totals.is_none() {
        drift.push((route.clone(), "totals is absent".to_string()));
    }
    // A row the board cannot read is named rather than dropped in
    // silence: a payload whose model rows changed shape is an upgrade
    // announcing itself, not an empty ledger.
    let offered = rows(usage.get("models"))
        .map(|rows| rows.len())
        .unwrap_or(0);
    let mut models: Vec<ModelRow> = rows(usage.get("models"))
        .into_iter()
        .flatten()
        .filter_map(parse_model)
        .collect();
    models.sort_by(|left, right| right.calls.cmp(&left.calls).then(left.key.cmp(&right.key)));
    if models.len() < offered {
        drift.push((
            route.clone(),
            format!("{} of {offered} model rows could not be read", models.len()),
        ));
    }
    let offered_recent = rows(usage.get("recent"))
        .map(|rows| rows.len())
        .unwrap_or(0);
    let parsed_recent: Vec<RecentRow> = rows(usage.get("recent"))
        .into_iter()
        .flatten()
        .filter_map(parse_recent)
        .collect();
    if parsed_recent.len() < offered_recent {
        drift.push((
            route.clone(),
            format!(
                "{} of {offered_recent} recent rows could not be read",
                parsed_recent.len()
            ),
        ));
    }
    let recent: Vec<RecentRow> = parsed_recent.into_iter().take(12).collect();
    let judge_counters = parse_judge_counters(usage.get("judge"));
    let cascade = parse_cascade_counters(usage.get("cascade"));
    let workloads = number_map(usage.get("workloads"));
    UsageRead {
        totals,
        models,
        recent,
        judge_counters,
        cascade,
        workloads,
        drift,
    }
}

/// The `/api/state` payload's shape, named where a member the board
/// renders is absent from it. An empty section with no reason given is
/// the failure this names: a kit upgrade that renames `resolved.workloads`
/// or `judge.mode` would otherwise blank part of the board in silence,
/// which is the drift the second-brain board already names for a state
/// file that is not the shape it claims.
fn state_drift(state: &Value) -> Vec<(String, String)> {
    let route = "/api/state".to_string();
    let named = |field: &str| (route.clone(), format!("{field} is absent"));
    let mut drift = Vec::new();
    if state
        .get("judge")
        .and_then(|judge| text(judge, "mode"))
        .is_none()
    {
        drift.push(named("judge.mode"));
    }
    if state
        .get("guard")
        .and_then(|guard| guard.get("allowPayg"))
        .is_none()
    {
        drift.push(named("guard.allowPayg"));
    }
    let workloads = state
        .get("resolved")
        .and_then(|resolved| resolved.get("workloads"));
    match workloads {
        None => drift.push(named("resolved.workloads")),
        // Present but not the kit's own name-to-chain map: the tier
        // chains cannot be read out of it at all.
        Some(workloads) if workloads.as_object().is_none() => {
            drift.push((
                route,
                "resolved.workloads is not the kit's own map".to_string(),
            ));
        }
        Some(_) => {}
    }
    drift
}

fn parse_totals(value: &Value) -> Option<UsageTotals> {
    Some(UsageTotals {
        calls: number(value, "calls")?,
        ok: number(value, "ok")?,
        errors: number(value, "errors")?,
        prompt_tokens: number(value, "promptTokens")?,
        completion_tokens: number(value, "completionTokens")?,
        total_tokens: number(value, "totalTokens")?,
    })
}

fn parse_model(value: &Value) -> Option<ModelRow> {
    Some(ModelRow {
        key: text(value, "key")?,
        provider: text(value, "providerId")?,
        model: text(value, "model")?,
        calls: number(value, "calls")?,
        errors: number(value, "errors")?,
        total_tokens: number(value, "totalTokens")?,
    })
}

fn parse_recent(value: &Value) -> Option<RecentRow> {
    Some(RecentRow {
        at: number(value, "at")?,
        provider: text(value, "providerId")?,
        model: text(value, "model")?,
        status: number(value, "status")?.min(u16::MAX as u64) as u16,
        workload: text(value, "workload"),
        ms: number(value, "ms"),
        reason: text(value, "reason"),
    })
}

/// The judge block: its mode and the model it runs on.
fn parse_judge(state: &Value) -> Option<(String, Option<String>)> {
    let judge = state.get("judge")?;
    Some((text(judge, "mode")?, text(judge, "model")))
}

/// The guard block: whether payg is allowed and which providers are payg.
fn parse_payg(state: &Value) -> Option<(bool, Vec<String>)> {
    let guard = state.get("guard")?;
    let allowed = guard.get("allowPayg").and_then(Value::as_bool)?;
    let providers = rows(guard.get("paygProviders"))
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    Some((allowed, providers))
}

/// `resolved.workloads` into the tier chains, keeping the kit's own
/// order. A workload whose resolution is not its candidate list's head
/// leads the chain, the way the fork's `parseTiers` reads it.
fn parse_tiers(state: &Value) -> Vec<TierChain> {
    let Some(workloads) = state
        .get("resolved")
        .and_then(|resolved| resolved.get("workloads"))
    else {
        return Vec::new();
    };
    let Some(workloads) = workloads.as_object() else {
        return Vec::new();
    };
    let mut chains = Vec::new();
    for (tier, chain) in workloads {
        let Some(chain) = chain.as_object() else {
            continue;
        };
        let primary = hop(chain);
        let mut candidates: Vec<String> = rows(chain.get("candidates"))
            .into_iter()
            .flatten()
            .filter_map(|candidate| candidate.as_object().and_then(hop))
            .collect();
        if let Some(primary) = &primary {
            let leads = candidates
                .first()
                .map(|head| head != primary)
                .unwrap_or(true);
            if leads {
                candidates.insert(0, primary.clone());
            }
        }
        let Some(primary) = candidates.first().cloned() else {
            continue;
        };
        chains.push(TierChain {
            tier: tier.clone(),
            primary,
            candidates,
        });
    }
    chains
}

/// One hop as `provider/model`, the way both the kit's own payload and
/// the board's rows name it.
fn hop(value: &Map<String, Value>) -> Option<String> {
    let provider = value.get("providerId").and_then(Value::as_str)?;
    let model = value.get("model").and_then(Value::as_str)?;
    Some(format!("{provider}/{model}"))
}

/// The kit's own per-tier guidance: `rationale` keyed `tiers.<tier>`, its
/// non-tier entries left out.
fn parse_suggest(suggest: &Value) -> Vec<(String, String)> {
    let Some(rationale) = suggest.get("rationale").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut guidance: Vec<(String, String)> = rationale
        .iter()
        .filter_map(|(key, sentence)| {
            let tier = key.strip_prefix("tiers.")?;
            let sentence = sentence.as_str()?;
            Some((tier.to_string(), sentence.to_string()))
        })
        .collect();
    guidance.sort_by(|left, right| left.0.cmp(&right.0));
    guidance
}

// --- the second-brain board --------------------------------------------------

/// One board read of the second-brain's derived state files. Every file
/// is its own read: a missing or unreadable file names itself and the
/// other five still land, which is what makes the board partial-data
/// honest while the pipeline has not produced every file yet.
#[derive(Debug, Clone, PartialEq)]
pub struct BrainSnapshot {
    pub read_at: String,
    pub home: PathBuf,
    pub gates: Section<GatesRead>,
    pub harness: Section<HarnessRead>,
    pub loops: Section<LoopsRead>,
    pub reports: Section<ReportsRead>,
    pub maturity: Section<MaturityRead>,
    pub tower: Section<TowerDataRead>,
}

/// The gates state: the earned-autonomy board, the HITL inbox, and the
/// daily verdict counts.
#[derive(Debug, Clone, PartialEq)]
pub struct GatesRead {
    pub generated: String,
    pub board: Vec<GateRow>,
    pub inbox: Vec<InboxRow>,
    pub totals: GateTotals,
    pub daily: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateTotals {
    pub events: u64,
    pub verdicts: u64,
    pub dispositions: u64,
}

/// One gate's earned-autonomy row.
#[derive(Debug, Clone, PartialEq)]
pub struct GateRow {
    pub gate: String,
    pub verdicts: u64,
    pub dispositions: u64,
    /// The graded-progress phrase as the state carries it, `2/20`.
    pub graded_progress: String,
    pub graded_rows: u64,
    pub fit_ready: bool,
    pub override_rate: f64,
}

/// One HITL inbox row: a negative verdict no later disposition closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxRow {
    pub op: String,
    pub verdict: String,
    pub ts: String,
    /// The command's first line, trimmed: the payload's command can be a
    /// whole script, and a card carries its head.
    pub command: String,
}

/// The harness state: the helmsman turn and its loop-run outcomes.
#[derive(Debug, Clone, PartialEq)]
pub struct HarnessRead {
    pub generated: String,
    pub loops: Vec<HarnessLoop>,
    pub recent: Vec<HarnessEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessLoop {
    pub name: String,
    pub runs: u64,
    pub converged: u64,
    pub last_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessEvent {
    pub ts: String,
    pub kind: String,
    pub label: String,
    pub outcome: String,
}

/// The loops state: quality-loop convergence and the graded rows the
/// loops feed into the calibration stores.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopsRead {
    pub derived_at: String,
    pub loops: Vec<LoopRow>,
    pub graded: Vec<(String, GradedRow)>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopRow {
    pub name: String,
    pub family: String,
    pub verdict: String,
    pub iterations: u64,
    pub pending_review: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradedRow {
    pub n: u64,
    pub confirmed: u64,
}

/// The maturity state: weekly methodology signals and the milestones.
#[derive(Debug, Clone, PartialEq)]
pub struct MaturityRead {
    pub generated: String,
    pub weeks: Vec<String>,
    /// The named series, in the payload's key order (serde_json's map is
    /// sorted, so this is alphabetical: `continuity`, `graded`, `infra`,
    /// `instrumented`, `output`).
    pub series: Vec<(String, Vec<u64>)>,
    pub milestones: Vec<(String, String)>,
}

/// The reports state: what is being done, what awaits verification, and
/// the conflict catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportsRead {
    pub generated_at: String,
    pub doing: Vec<DoingRow>,
    pub pending_verification: u64,
    pub conflicts: Vec<ConflictRow>,
    pub open_conflicts: u64,
    pub trust: Vec<TrustRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoingRow {
    pub id: String,
    pub project: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRow {
    pub task: String,
    pub class: String,
    pub severity: String,
    pub claim: String,
    pub reality: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrustRow {
    pub harness: String,
    pub claimed: u64,
    pub evidenced: u64,
    pub precision: f64,
}

/// The tower's built aggregates, as the second-brain board reads them.
#[derive(Debug, Clone, PartialEq)]
pub struct TowerDataRead {
    pub generated_at: String,
    pub projects: Vec<TowerProject>,
    pub timeline: Vec<TimelineRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TowerProject {
    pub name: String,
    pub lifecycle: String,
    pub branch: Option<String>,
    pub last_commit: String,
    pub commits_30d: u64,
    pub dirty: u64,
    /// The plan-ledger counts by status, done first.
    pub plans: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRow {
    pub date: String,
    pub subject: String,
    pub project: String,
}

/// Read the six derived state files under `home`. The home itself being
/// unreadable is the whole read's refusal; each file inside it is its
/// own, and the whole six-file read runs under an aggregate budget: a
/// sum of bounded reads is not itself bounded, so once the budget is
/// spent no further file is read and each unread file names the budget
/// rather than the cycle stalling behind it.
pub fn read_brain(home: &Path, bounds: &Bounds) -> Result<BrainSnapshot, String> {
    if !home.is_dir() {
        return Err(format!(
            "the second-brain board could not read {}: the directory does not exist",
            home.display()
        ));
    }
    let deadline = Instant::now() + bounds.brain_budget;
    let read = |name: &str| -> Section<Value> {
        if Instant::now() >= deadline {
            return Section::Refused(format!(
                "{name}.json was not read — the read budget was spent"
            ));
        }
        read_state_file(&home.join(format!("{name}.json")), bounds.file_read_timeout)
    };
    Ok(BrainSnapshot {
        read_at: now_stamp(),
        home: home.to_path_buf(),
        gates: parse_gates(read("gates_state")),
        harness: parse_harness(read("harness_state")),
        loops: parse_loops(read("loops_state")),
        reports: parse_reports(read("reports_state")),
        maturity: parse_maturity(read("maturity_state")),
        tower: parse_tower_data(read("tower_data")),
    })
}

/// One state file, read and named. The read runs on its own thread and
/// the bound expiring names it — a state file on a hung mount is a named
/// refusal, not a stalled cycle. The `git` helper's own drain pattern,
/// without a child to kill: the reader thread is left to end on its own
/// when the mount answers, which is the one case where an unbounded
/// wait is the cheaper of two evils.
fn read_state_file(path: &Path, timeout: Duration) -> Section<Value> {
    let named = path.display().to_string();
    let owned = path.to_path_buf();
    let (tx, rx) = mpsc::channel::<Section<Value>>();
    thread::spawn(move || {
        let _ = tx.send(read_state_file_now(&owned));
    });
    match rx.recv_timeout(timeout) {
        Ok(section) => section,
        Err(_) => Section::Refused(format!(
            "{named} was not read — the read did not finish inside its bound"
        )),
    }
}

/// One state file, read and named on the reader thread: a missing file,
/// an unreadable one, and a payload that does not validate are three
/// different refusals and each names which.
fn read_state_file_now(path: &Path) -> Section<Value> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Section::Refused(format!("{} does not exist", path.display()));
        }
        Err(error) => {
            return Section::Refused(format!("{} could not be read: {error}", path.display()));
        }
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Section::Landed(value),
        Err(error) => Section::Refused(format!(
            "{} returned a payload the board cannot read: {error}",
            path.display()
        )),
    }
}

/// Validate one payload into its read, naming the file when it does not.
fn validated<T>(
    section: Section<Value>,
    path: &str,
    parse: impl FnOnce(&Value) -> Option<T>,
) -> Section<T> {
    match section {
        Section::Landed(value) => {
            // A payload that is not an object is not this state file, even
            // when it parsed as JSON: the board says so rather than
            // rendering an empty card as though the file were empty.
            if !value.is_object() {
                return Section::Refused(format!(
                    "{path} returned a payload the board cannot read"
                ));
            }
            match parse(&value) {
                Some(read) => Section::Landed(read),
                None => {
                    Section::Refused(format!("{path} returned a payload the board cannot read"))
                }
            }
        }
        Section::Refused(reason) => Section::Refused(reason),
    }
}

fn parse_gates(section: Section<Value>) -> Section<GatesRead> {
    validated(section, "gates_state", |value| {
        Some(GatesRead {
            generated: text(value, "generated").unwrap_or_default(),
            board: board_rows(value.get("board")),
            inbox: inbox_rows(value.get("inbox")),
            totals: value
                .get("totals")
                .map(|totals| GateTotals {
                    events: number(totals, "events").unwrap_or(0),
                    verdicts: number(totals, "verdicts").unwrap_or(0),
                    dispositions: number(totals, "dispositions").unwrap_or(0),
                })
                .unwrap_or(GateTotals {
                    events: 0,
                    verdicts: 0,
                    dispositions: 0,
                }),
            daily: daily_rows(value.get("daily")),
        })
    })
}

fn board_rows(value: Option<&Value>) -> Vec<GateRow> {
    let Some(board) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut rows: Vec<GateRow> = board
        .iter()
        .filter_map(|(gate, row)| {
            Some(GateRow {
                gate: gate.clone(),
                verdicts: number(row, "verdicts").unwrap_or(0),
                dispositions: number(row, "dispositions").unwrap_or(0),
                graded_progress: text(row, "graded_progress").unwrap_or_default(),
                graded_rows: number(row, "graded_rows").unwrap_or(0),
                fit_ready: row
                    .get("fit_ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                override_rate: row
                    .get("override_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            })
        })
        .collect();
    rows.sort_by(|left, right| {
        right
            .verdicts
            .cmp(&left.verdicts)
            .then(left.gate.cmp(&right.gate))
    });
    rows
}

fn inbox_rows(value: Option<&Value>) -> Vec<InboxRow> {
    rows(value)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some(InboxRow {
                op: text(row, "op")?,
                verdict: text(row, "verdict")?,
                ts: text(row, "ts").unwrap_or_default(),
                command: text(row, "command")
                    .unwrap_or_default()
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .chars()
                    .take(60)
                    .collect(),
            })
        })
        .collect()
}

fn daily_rows(value: Option<&Value>) -> Vec<(String, u64)> {
    rows(value)
        .into_iter()
        .flatten()
        .filter_map(|row| Some((text(row, "date")?, number(row, "count").unwrap_or(0))))
        .collect()
}

fn parse_harness(section: Section<Value>) -> Section<HarnessRead> {
    validated(section, "harness_state", |value| {
        Some(HarnessRead {
            generated: text(value, "generated").unwrap_or_default(),
            loops: rows(value.get("loops"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(HarnessLoop {
                        name: text(row, "loop")?,
                        runs: number(row, "runs").unwrap_or(0),
                        converged: number(row, "converged").unwrap_or(0),
                        last_status: text(row, "last_status").unwrap_or_default(),
                    })
                })
                .collect(),
            recent: rows(value.get("recent"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(HarnessEvent {
                        ts: text(row, "ts").unwrap_or_default(),
                        kind: text(row, "kind").unwrap_or_default(),
                        label: text(row, "label").unwrap_or_default(),
                        outcome: text(row, "outcome").unwrap_or_default(),
                    })
                })
                .take(8)
                .collect(),
        })
    })
}

fn parse_loops(section: Section<Value>) -> Section<LoopsRead> {
    validated(section, "loops_state", |value| {
        Some(LoopsRead {
            derived_at: text(value, "derived_at").unwrap_or_default(),
            loops: rows(value.get("loops"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(LoopRow {
                        name: text(row, "loop")?,
                        family: text(row, "family").unwrap_or_default(),
                        verdict: text(row, "verdict").unwrap_or_default(),
                        iterations: number(row, "iterations").unwrap_or(0),
                        pending_review: number(row, "pending_review").unwrap_or(0),
                    })
                })
                .collect(),
            graded: value
                .get("graded_rows")
                .and_then(Value::as_object)
                .map(|graded| {
                    let mut rows: Vec<(String, GradedRow)> = graded
                        .iter()
                        .map(|(name, row)| {
                            (
                                name.clone(),
                                GradedRow {
                                    n: number(row, "n").unwrap_or(0),
                                    confirmed: number(row, "confirmed").unwrap_or(0),
                                },
                            )
                        })
                        .collect();
                    rows.sort_by(|left, right| right.1.n.cmp(&left.1.n).then(left.0.cmp(&right.0)));
                    rows
                })
                .unwrap_or_default(),
            findings: rows(value.get("findings"))
                .into_iter()
                .flatten()
                .filter_map(|finding| finding.as_str().map(str::to_string))
                .take(3)
                .collect(),
        })
    })
}

fn parse_maturity(section: Section<Value>) -> Section<MaturityRead> {
    validated(section, "maturity_state", |value| {
        let weeks: Vec<String> = rows(value.get("weeks"))
            .into_iter()
            .flatten()
            .filter_map(|week| week.as_str().map(str::to_string))
            .collect();
        let series = value
            .get("series")
            .and_then(Value::as_object)
            .map(|series| {
                series
                    .iter()
                    .map(|(name, values)| {
                        let values: Vec<u64> = rows(Some(values))
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_u64)
                            .collect();
                        (name.clone(), values)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let milestones = value
            .get("milestones")
            .and_then(Value::as_object)
            .map(|milestones| {
                milestones
                    .iter()
                    .filter_map(|(name, at)| Some((name.clone(), at.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Some(MaturityRead {
            generated: text(value, "generated").unwrap_or_default(),
            weeks,
            series,
            milestones,
        })
    })
}

fn parse_reports(section: Section<Value>) -> Section<ReportsRead> {
    validated(section, "reports_state", |value| {
        Some(ReportsRead {
            generated_at: text(value, "generated_at").unwrap_or_default(),
            doing: rows(value.get("doing"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(DoingRow {
                        id: text(row, "id")?,
                        project: text(row, "project").unwrap_or_default(),
                        title: text(row, "title").unwrap_or_default(),
                    })
                })
                .take(5)
                .collect(),
            pending_verification: rows(value.get("pending_verification"))
                .map(|rows| rows.len() as u64)
                .unwrap_or(0),
            conflicts: rows(value.get("conflicts"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(ConflictRow {
                        task: text(row, "task").unwrap_or_default(),
                        class: text(row, "type").unwrap_or_default(),
                        severity: text(row, "severity").unwrap_or_default(),
                        claim: text(row, "claim").unwrap_or_default(),
                        reality: text(row, "reality").unwrap_or_default(),
                    })
                })
                .take(6)
                .collect(),
            open_conflicts: number(value, "open_conflict_count").unwrap_or(0),
            trust: rows(value.get("trust"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(TrustRow {
                        harness: text(row, "harness")?,
                        claimed: number(row, "claimed").unwrap_or(0),
                        evidenced: number(row, "evidenced").unwrap_or(0),
                        precision: row.get("precision").and_then(Value::as_f64).unwrap_or(0.0),
                    })
                })
                .collect(),
        })
    })
}

fn parse_tower_data(section: Section<Value>) -> Section<TowerDataRead> {
    validated(section, "tower_data", |value| {
        Some(TowerDataRead {
            generated_at: text(value, "generated_at").unwrap_or_default(),
            projects: rows(value.get("projects"))
                .into_iter()
                .flatten()
                .filter_map(parse_tower_project)
                .collect(),
            timeline: rows(value.get("timeline"))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    Some(TimelineRow {
                        date: text(row, "date")?,
                        subject: text(row, "subject").unwrap_or_default(),
                        project: text(row, "project").unwrap_or_default(),
                    })
                })
                .take(8)
                .collect(),
        })
    })
}

fn parse_tower_project(value: &Value) -> Option<TowerProject> {
    let plans = value
        .get("plans")
        .and_then(Value::as_object)
        .map(|plans| {
            let mut counts: Vec<(String, u64)> = plans
                .iter()
                .map(|(status, count)| (status.clone(), count.as_u64().unwrap_or(0)))
                .collect();
            counts.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
            counts
        })
        .unwrap_or_default();
    Some(TowerProject {
        name: text(value, "name")?,
        lifecycle: text(value, "lifecycle").unwrap_or_default(),
        branch: text(value, "branch"),
        last_commit: text(value, "last_commit").unwrap_or_default(),
        commits_30d: number(value, "commits_30d").unwrap_or(0),
        dirty: number(value, "dirty").unwrap_or(0),
        plans,
    })
}

// --- the tower ---------------------------------------------------------------

/// One board read of the projects root: git facts per project, read-only.
/// The root itself being unlistable is the whole read's refusal; one
/// project whose git calls fail lands with its facts named as unanswered
/// and the rest still report.
#[derive(Debug, Clone, PartialEq)]
pub struct TowerSnapshot {
    pub read_at: String,
    pub root: PathBuf,
    /// The projects that are git worktrees, dirtiest first.
    pub projects: Vec<ProjectFacts>,
    /// The scanned directories that are not worktrees, with the reason.
    pub skipped: Vec<(String, String)>,
    /// How many directories the scan's bound left unread.
    pub beyond_bound: usize,
    /// How many directories the scan's aggregate budget left unread.
    /// Non-zero names a truncated scan: the projects read so far are
    /// real and the rest were never read, which is a different fact
    /// from a project having no git facts.
    pub unread: usize,
}

/// One project's git facts. A `None` names what git could not answer; a
/// note says why when the calls failed outright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFacts {
    pub name: String,
    pub branch: Option<String>,
    /// The head commit's short sha.
    pub head: Option<String>,
    pub subject: Option<String>,
    /// The head commit's date, `YYYY-MM-DD`.
    pub when: Option<String>,
    pub commits_30d: u64,
    /// The working-tree entries `git status --porcelain` reports.
    pub dirty: u64,
    /// Commits the local head is ahead of / behind its upstream, when the
    /// project has one. A local comparison, never a fetch.
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub notes: Vec<String>,
}

/// Read the projects root's git facts. Every `git` call is bounded by
/// `bounds.git_timeout`, and the whole scan is bounded by
/// `bounds.scan_budget`: thirty projects × five bounded calls is a sum
/// no single bound covers, so once the budget is spent the scan stops,
/// the projects read so far land, and the snapshot names how many
/// directories it left unread. A call that does not answer inside its
/// bound is killed and named rather than stalling the scan.
pub fn read_tower(root: &Path, bounds: &Bounds) -> Result<TowerSnapshot, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(format!(
                "the tower could not list {}: {error}",
                root.display()
            ));
        }
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        // Followed, not the dirent's own type: a symlinked project
        // directory is scanned and named rather than silently skipped.
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    let beyond_bound = names.len().saturating_sub(bounds.max_projects);
    names.truncate(bounds.max_projects);
    // The aggregate deadline, checked between projects and before every
    // `git` call: a hung git on one repository costs the budget, not the
    // cadence behind the scan.
    let deadline = Instant::now() + bounds.scan_budget;

    let found = names.len();
    let mut projects = Vec::new();
    let mut skipped = Vec::new();
    let mut unread = 0;
    for (index, name) in names.into_iter().enumerate() {
        if Instant::now() >= deadline {
            unread = found - index;
            break;
        }
        let path = root.join(&name);
        if !path.join(".git").exists() {
            skipped.push((name, "no .git".to_string()));
            continue;
        }
        projects.push(project_facts(&path, &name, bounds, deadline));
    }
    // The dirtiest projects first: a scan of thirty repos shows a dozen
    // rows, and the ones with uncommitted work are the ones worth seeing.
    projects.sort_by(|left, right| {
        right
            .dirty
            .cmp(&left.dirty)
            .then(right.commits_30d.cmp(&left.commits_30d))
            .then(left.name.cmp(&right.name))
    });
    Ok(TowerSnapshot {
        read_at: now_stamp(),
        root: root.to_path_buf(),
        projects,
        skipped,
        beyond_bound,
        unread,
    })
}

/// One project's git facts, each call bounded and each failure named.
/// The scan's aggregate deadline is checked before each call as well as
/// between projects: a project whose every call spends its full bound
/// would otherwise cost five bounds on its own, and the budget is what
/// stops the scan rather than what merely bounds it.
fn project_facts(path: &Path, name: &str, bounds: &Bounds, deadline: Instant) -> ProjectFacts {
    let mut notes = Vec::new();
    let mut spent = false;
    let mut call = |args: &[&str]| {
        if Instant::now() >= deadline {
            spent = true;
            return None;
        }
        git(path, &bounds.git_bin, args, bounds.git_timeout)
    };
    let branch = call(&["symbolic-ref", "--short", "HEAD"])
        .map(|text| text.trim().to_string())
        .filter(|branch| !branch.is_empty());
    if branch.is_none() {
        notes.push("git could not name the branch".to_string());
    }
    let head = match call(&["log", "-1", "--format=%h%x1f%s%x1f%cI"]) {
        Some(line) => parse_head(&line),
        None => {
            notes.push("git could not read the head commit".to_string());
            None
        }
    };
    let commits_30d = call(&["rev-list", "--count", "HEAD", "--since=30.days"])
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let dirty = call(&["status", "--porcelain"])
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count() as u64)
        .unwrap_or(0);
    let (ahead, behind) = match call(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
    {
        Some(text) => parse_ahead_behind(&text),
        None => (None, None),
    };
    if spent {
        notes.push("the scan budget was spent".to_string());
    }
    ProjectFacts {
        name: name.to_string(),
        branch,
        head: head.as_ref().map(|(sha, _, _)| sha.clone()),
        subject: head.as_ref().map(|(_, subject, _)| subject.clone()),
        when: head.as_ref().map(|(_, _, when)| when.clone()),
        commits_30d,
        dirty,
        ahead,
        behind,
        notes,
    }
}

/// `%h%x1f%s%x1f%cI` into the short sha, the subject, and the date. The
/// separators are git's own format guarantee; a line without them reads
/// as unanswered rather than being sliced into.
fn parse_head(line: &str) -> Option<(String, String, String)> {
    let line = line.trim_end_matches(['\n', '\r']);
    let first = line.find('\u{1f}')?;
    let last = line.rfind('\u{1f}')?;
    if last <= first {
        return None;
    }
    let sha = line[..first].to_string();
    let subject = line[first + 1..last].to_string();
    let when = line[last + 1..].chars().take(10).collect();
    Some((sha, subject, when))
}

/// `--left-right --count` output: left is the local head's own commits,
/// right is the upstream's.
fn parse_ahead_behind(text: &str) -> (Option<u64>, Option<u64>) {
    let mut counts = text
        .split_whitespace()
        .filter_map(|part| part.parse::<u64>().ok());
    (counts.next(), counts.next())
}

/// One bounded `git` call. The child's stdout is drained on a reader
/// thread so a chatty child cannot deadlock the wait, and the bound
/// expiring kills the child: a hung git is a named failure, not a stalled
/// scan (the eval probe's own pattern). stderr is discarded — a call that
/// fails is "git did not answer", and git's own prose is not the board's
/// to render.
fn git(directory: &Path, bin: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut command = Command::new(bin);
    command.arg("-C").arg(directory).args(args);
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take();
    let (tx, rx) = mpsc::channel::<std::io::Result<Vec<u8>>>();
    thread::spawn(move || {
        use std::io::Read as _;
        let mut out = Vec::new();
        let drained = stdout
            .as_mut()
            .map(|pipe| pipe.read_to_end(&mut out))
            .transpose()
            .map(|_| out);
        let _ = tx.send(drained);
    });
    let output = match rx.recv_timeout(timeout) {
        Ok(Ok(out)) => Some(String::from_utf8_lossy(&out).to_string()),
        _ => None,
    };
    if output.is_none() {
        // The bound expired: kill the child, which closes its pipe and
        // ends the reader thread.
        let _ = child.kill();
    }
    let _ = child.wait();
    output
}

// --- the refresher -----------------------------------------------------------

/// One instruction to the refresher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardCommand {
    /// Read all three boards now, whatever the cadence says.
    Refresh,
}

/// One finished read, on its way to the interface.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardUpdate {
    pub kind: BoardKind,
    pub outcome: Result<BoardData, String>,
}

/// Spawn the board refresher: one thread, its own current-thread runtime,
/// an opening read of all three boards and one every
/// [`BOARD_REFRESH_SECS`], plus one immediately per
/// [`BoardCommand::Refresh`]. A refresh asked for while a read is already
/// running is satisfied by that read rather than queued behind it, so N
/// presses of the cycle key cost one read, not N. The reads run on the
/// runtime's blocking pool, so the three boards are read concurrently and
/// the slowest source sets a refresh's cost rather than the sum of all
/// three. Every read is bounded, so a hung source names itself instead of
/// stalling the interface: the event loop only ever receives finished
/// reads.
pub fn spawn_refresher(
    sources: Sources,
) -> (
    UnboundedSender<BoardCommand>,
    UnboundedReceiver<BoardUpdate>,
) {
    let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel::<BoardCommand>();
    let (update_tx, update_rx) = tokio::sync::mpsc::unbounded_channel::<BoardUpdate>();
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                for kind in BoardKind::ALL {
                    let _ = update_tx.send(BoardUpdate {
                        kind,
                        outcome: Err(format!("the board runtime failed to start: {error}")),
                    });
                }
                return;
            }
        };
        runtime.block_on(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(BOARD_REFRESH_SECS));
            loop {
                // The cadence or a refresh command, whichever comes
                // first. No missed-tick behavior is set, because the
                // reset below supersedes it: whatever woke this
                // iteration, the next read is one period after the read
                // it triggered has finished.
                tokio::select! {
                    _ = ticker.tick() => {}
                    command = command_rx.recv() => {
                        // The sender is gone: the interface has ended.
                        if command.is_none() {
                            return;
                        }
                    }
                }
                for update in read_all(&sources).await {
                    if update_tx.send(update).is_err() {
                        return;
                    }
                }
                // A refresh asked for while that read was running is
                // satisfied by it, so the channel is drained here rather
                // than read again: N presses of the cycle key cost one
                // read, not N. Without this, a Ctrl+B mash queues a full
                // tower scan behind every press.
                while command_rx.try_recv().is_ok() {}
                // The cadence restarts from the end of the read that just
                // finished, and a tick that was still pending when a
                // command woke this iteration is discarded with it: a
                // read that overruns its period stretches the cadence
                // rather than firing a second read the moment the first
                // lands, which is what made one Ctrl+B cost two scans.
                ticker.reset();
            }
        });
    });
    (command_tx, update_rx)
}

/// One read of all three boards, concurrently.
async fn read_all(sources: &Sources) -> Vec<BoardUpdate> {
    let router = sources.router.clone();
    let brain_home = sources.brain_home.clone();
    let tower_root = sources.tower_root.clone();
    let brain_bounds = sources.bounds.clone();
    let tower_bounds = sources.bounds.clone();
    let timeout = Duration::from_secs(READ_TIMEOUT_SECS);
    let (router, brain, tower) = tokio::join!(
        tokio::task::spawn_blocking(move || read_router(&router, timeout)),
        tokio::task::spawn_blocking(move || read_brain_source(&brain_home, &brain_bounds)),
        tokio::task::spawn_blocking(move || read_tower(&tower_root, &tower_bounds)),
    );
    // A read that panicked on the blocking pool is named, never swallowed:
    // the board keeps its last snapshot and the pane says the read died.
    let landed = |joined: Result<Result<BoardData, String>, tokio::task::JoinError>,
                  board: &str| {
        joined.unwrap_or_else(|error| Err(format!("the {board} read could not run: {error}")))
    };
    vec![
        BoardUpdate {
            kind: BoardKind::Router,
            outcome: landed(
                router.map(|inner| inner.map(BoardData::Router)),
                "router board",
            ),
        },
        BoardUpdate {
            kind: BoardKind::Brain,
            outcome: landed(
                brain.map(|inner| inner.map(BoardData::Brain)),
                "second-brain board",
            ),
        },
        BoardUpdate {
            kind: BoardKind::Tower,
            outcome: landed(tower.map(|inner| inner.map(BoardData::Tower)), "tower"),
        },
    ]
}

/// The second-brain read as the refresher drives it: a home that does not
/// resolve is a named refusal, not a panic.
fn read_brain_source(home: &Option<PathBuf>, bounds: &Bounds) -> Result<BrainSnapshot, String> {
    match home {
        Some(home) => read_brain(home, bounds),
        None => Err(
            "the second-brain board could not read its state files: no second-brain home resolved; set SECOND_BRAIN_HOME"
                .to_string(),
        ),
    }
}

// --- the payload readers -----------------------------------------------------

/// One string member.
fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}

/// One unsigned integer member. A count that arrives as a float (a JSON
/// writer with no integer type) reads as its floor rather than refusing
/// the row.
fn number(value: &Value, key: &str) -> Option<u64> {
    let member = value.get(key)?;
    member
        .as_u64()
        .or_else(|| member.as_f64().map(|value| value.max(0.0) as u64))
}

/// One array member.
fn rows(value: Option<&Value>) -> Option<&Vec<Value>> {
    value?.as_array()
}

/// A name-to-count map, busiest first.
fn number_map(value: Option<&Value>) -> Vec<(String, u64)> {
    let Some(map) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut counts: Vec<(String, u64)> = map
        .iter()
        .filter_map(|(name, count)| Some((name.clone(), count.as_u64()?)))
        .collect();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    counts
}

/// The judge's counters; `None` when the block is absent, because a
/// payload that names no block is not read as one with zero counters.
fn parse_judge_counters(value: Option<&Value>) -> Option<JudgeCounters> {
    let value = value?;
    Some(JudgeCounters {
        fresh: number(value, "fresh")?,
        cached: number(value, "cached")?,
        errors: number(value, "errors")?,
    })
}

/// The cascade's counters, on the same rule.
fn parse_cascade_counters(value: Option<&Value>) -> Option<CascadeCounters> {
    let value = value?;
    Some(CascadeCounters {
        escalations: number(value, "escalations")?,
        cold: number(value, "cold")?,
        failed: number(value, "failed")?,
        compared: number(value, "compared")?,
    })
}

// --- the render vocabulary ---------------------------------------------------

/// One count in the board's compact spelling: grouped past a thousand,
/// abbreviated past ten million, so a token count keeps its column.
pub fn compact(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}B", value as f64 / 1e9)
    } else if value >= 10_000_000 {
        format!("{:.1}M", value as f64 / 1e6)
    } else {
        grouped(value)
    }
}

/// One count with its thousands grouped: `19,873`.
pub fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(character);
    }
    out
}

/// A proportional bar in eighths: `value` against `max`, drawn `width`
/// cells wide with the Unicode block elements, so a comparison reads at a
/// glance instead of being counted. A zero max draws blanks rather than a
/// full bar.
pub fn bar(value: u64, max: u64, width: usize) -> String {
    const PARTIALS: [char; 7] = [
        '\u{258f}', '\u{258e}', '\u{258d}', '\u{258c}', '\u{258b}', '\u{258a}', '\u{2589}',
    ];
    const FULL: char = '\u{2588}';
    if width == 0 {
        return String::new();
    }
    if max == 0 || value == 0 {
        return " ".repeat(width);
    }
    let eighths = (value as u128 * (width as u128 * 8) / max as u128) as usize;
    let full = (eighths / 8).min(width);
    let part = eighths % 8;
    let mut out: String = (0..full).map(|_| FULL).collect();
    if full < width && part > 0 {
        out.push(PARTIALS[part - 1]);
    }
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// A sparkline over `values`, one cell per value (the last `width` of
/// them), drawn with the lower block elements. A flat or empty series
/// draws its floor rather than nothing.
pub fn sparkline(values: &[u64], width: usize) -> String {
    const STEPS: [char; 8] = [
        '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
        '\u{2588}',
    ];
    if width == 0 {
        return String::new();
    }
    let shown: Vec<u64> = values.iter().rev().take(width).rev().copied().collect();
    if shown.is_empty() {
        return " ".repeat(width);
    }
    let max = shown.iter().copied().max().unwrap_or(0);
    let pad = width.saturating_sub(shown.len());
    let mut out: String = " ".repeat(pad);
    for value in &shown {
        let step = if max == 0 {
            0
        } else {
            ((*value as u128 * 7 / max as u128) as usize).min(7)
        };
        out.push(STEPS[step]);
    }
    out
}

/// Trim `text` to `width` cells, marking a cut with an ellipsis so a
/// truncated name never reads as the whole name.
pub fn trim_to(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width <= 1 {
        return text.chars().take(width).collect();
    }
    let kept: String = text.chars().take(width - 1).collect();
    format!("{kept}\u{2026}")
}

/// Pad `text` to exactly `width` cells, trimming it when it is longer.
pub fn cell(text: &str, width: usize) -> String {
    let mut out = trim_to(text, width);
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(base_url: &str) -> RouterSources {
        RouterSources {
            base_url: base_url.to_string(),
            token: Some("test-token".to_string()),
        }
    }

    /// A scratch directory that is removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let path =
                std::env::temp_dir().join(format!("fleet-boards-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the scratch directory");
            Scratch(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // --- the scripted operator API -------------------------------------------

    /// A scripted stand-in for the router kit's operator API: one loopback
    /// listener answering each route from a fixture, so the board's read
    /// is driven by a deterministic payload instead of the live kit. An
    /// unregistered path answers 404 with the kit's own unknown-path
    /// sentence, the way the real router does.
    struct ScriptedKit {
        port: u16,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl ScriptedKit {
        fn start(routes: &[(&str, u16, &'static str)]) -> ScriptedKit {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            let port = listener.local_addr().expect("the bound port").port();
            let routes: Vec<(String, u16, String)> = routes
                .iter()
                .map(|(path, status, body)| ((*path).to_string(), *status, (*body).to_string()))
                .collect();
            let handle = thread::spawn(move || {
                use std::io::{BufRead, BufReader, Write};
                // Polled rather than blocked: a route the board never asks
                // for must not leave the server waiting on `accept` when
                // the test's own `Drop` joins this thread.
                let _ = listener.set_nonblocking(true);
                let deadline = std::time::Instant::now() + Duration::from_secs(10);
                loop {
                    let Ok((mut stream, _)) = listener.accept() else {
                        if std::time::Instant::now() >= deadline {
                            return;
                        }
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    };
                    // A socket accepted from a non-blocking listener
                    // inherits the flag on this platform, and a
                    // non-blocking stream turns the response's own read
                    // into `InvalidInput`: the connection goes back to
                    // blocking before a word is written to it.
                    let _ = stream.set_nonblocking(false);
                    let peer = stream.try_clone().expect("the connection");
                    let request = BufReader::new(peer)
                        .lines()
                        .map_while(Result::ok)
                        .take_while(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_string();
                    let (status, body) = routes
                        .iter()
                        .find(|(route, _, _)| route == &path)
                        .map(|(_, status, body)| (*status, body.clone()))
                        .unwrap_or((
                            404,
                            "{\"error\":{\"message\":\"router: unknown api path\"}}".to_string(),
                        ));
                    let reason = if status == 200 { "OK" } else { "Unauthorized" };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            });
            ScriptedKit {
                port,
                handle: Some(handle),
            }
        }

        fn url(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }
    }

    impl Drop for ScriptedKit {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    /// A listener that accepts and never answers: the hung source.
    struct DeafKit(u16);

    impl DeafKit {
        fn start() -> DeafKit {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            let port = listener.local_addr().expect("the bound port").port();
            thread::spawn(move || {
                for stream in listener.incoming().take(4) {
                    // Hold the connection open without a word: the read
                    // must name its own bound rather than wait forever.
                    if let Ok(stream) = stream {
                        thread::spawn(move || {
                            thread::sleep(Duration::from_secs(30));
                            drop(stream);
                        });
                    }
                }
            });
            DeafKit(port)
        }

        fn url(&self) -> String {
            format!("http://127.0.0.1:{}", self.0)
        }
    }

    /// A port with nothing listening on it: the dead source.
    fn dead_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let url = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("the bound port").port()
        );
        drop(listener);
        url
    }

    const STATE_FIXTURE: &str = r#"{
      "ok": true,
      "port": 8300,
      "kitRoot": "/Users/owner/Projects/agnostic-router-kit",
      "judge": { "mode": "cascade", "model": "fastino/gliner2.5-multi-v1" },
      "guard": { "allowPayg": false, "paygProviders": ["deepseek"] },
      "resolved": {
        "defaultWorkload": "standard_code",
        "workloads": {
          "quick": { "providerId": "stepfun", "model": "step-3.7-flash",
                     "candidates": [ { "providerId": "stepfun", "model": "step-3.7-flash" },
                                     { "providerId": "xiaomi-mimo", "model": "mimo-v2.6-flash" },
                                     { "providerId": "zai-coding-plan", "model": "GLM-5.3-Flash" } ] },
          "standard_code": { "providerId": "stepfun", "model": "step-5-preview",
                     "candidates": [ { "providerId": "stepfun", "model": "step-5-preview" },
                                     { "providerId": "zai-coding-plan", "model": "GLM-5.3-Flash" } ] }
        }
      }
    }"#;

    const USAGE_FIXTURE: &str = r#"{
      "ok": true,
      "totals": { "calls": 19873, "ok": 19646, "errors": 227,
                  "promptTokens": 88197905869, "completionTokens": 3961204277, "totalTokens": 92159110146 },
      "models": [
        { "key": "stepfun/step-5-preview", "providerId": "stepfun", "model": "step-5-preview",
          "calls": 1791, "ok": 1791, "errors": 0, "totalTokens": 61234540459, "lastUsedAt": 1791565120539 },
        { "key": "stepfun/step-3.7-flash", "providerId": "stepfun", "model": "step-3.7-flash",
          "calls": 3720, "ok": 3715, "errors": 5, "totalTokens": 29604087234, "lastUsedAt": 1791918037872 }
      ],
      "days": [ { "day": "2026-09-29", "calls": 2902, "promptTokens": 1, "completionTokens": 2 } ],
      "recent": [
        { "at": 1791918037872, "requested": "auto", "workload": "standard_code", "providerId": "stepfun",
          "model": "step-3.7-flash", "status": 200, "ms": 2000, "promptTokens": 10279, "completionTokens": 509 },
        { "at": 1791916699901, "requested": "auto", "workload": "standard_code", "providerId": "zai-coding-plan",
          "model": "GLM-5.3-Flash", "status": 500, "ms": 812, "promptTokens": 13, "completionTokens": 5,
          "reason": "upstream refused" }
      ],
      "judge": { "fresh": 581, "cached": 15738, "errors": 321 },
      "cascade": { "escalations": 787, "cold": 0, "failed": 570, "compared": 217 },
      "workloads": { "quick": 4726, "standard_code": 10557, "hard": 3520 }
    }"#;

    const SUGGEST_FIXTURE: &str = r#"{
      "ok": true,
      "suggested": { "tiers": { "quick": { "target": "xiaomi-mimo/mimo-v2.6-pro" } } },
      "rationale": { "tiers.quick": "the quick tier's head is exhausted; move it",
                     "general": "not a tier entry" },
      "basis": { "recentRequests": 512, "modelsScored": 5 }
    }"#;

    // --- the router board ----------------------------------------------------

    /// The three routes a full router-board read makes.
    fn scripted_kit() -> ScriptedKit {
        ScriptedKit::start(&[
            ("/api/state", 200, STATE_FIXTURE),
            ("/api/usage", 200, USAGE_FIXTURE),
            ("/api/suggest", 200, SUGGEST_FIXTURE),
        ])
    }

    #[test]
    fn the_router_board_reads_the_kits_operator_api_into_its_snapshot() {
        let kit = scripted_kit();
        let snapshot = read_router(&sources(&kit.url()), Duration::from_secs(5))
            .expect("the scripted kit answers");

        assert_eq!(snapshot.port, Some(8300));
        assert_eq!(
            snapshot.judge,
            Some((
                "cascade".to_string(),
                Some("fastino/gliner2.5-multi-v1".to_string())
            ))
        );
        assert_eq!(snapshot.payg, Some((false, vec!["deepseek".to_string()])));
        // The tier chains keep the kit's own order and name the resolved
        // primary as the chain's head.
        let tiers: Vec<&str> = snapshot
            .tiers
            .iter()
            .map(|chain| chain.tier.as_str())
            .collect();
        assert_eq!(tiers, vec!["quick", "standard_code"]);
        assert_eq!(snapshot.tiers[0].primary, "stepfun/step-3.7-flash");
        assert_eq!(snapshot.tiers[0].candidates.len(), 3);
        // The usage ledger lands with its totals, its busiest-first
        // models, and its recent calls.
        let totals = snapshot.totals.expect("the usage totals");
        assert_eq!(totals.calls, 19_873);
        assert_eq!(totals.success_rate(), Some(98));
        assert_eq!(snapshot.models[0].model, "step-3.7-flash");
        assert_eq!(snapshot.recent.len(), 2);
        assert!(snapshot.recent[0].ok());
        assert!(!snapshot.recent[1].ok());
        // The counters and the workload mix.
        assert_eq!(snapshot.judge_counters.map(|c| c.cached), Some(15_738));
        assert_eq!(snapshot.cascade.map(|c| c.escalations), Some(787));
        assert_eq!(snapshot.workloads[0], ("standard_code".to_string(), 10_557));
        // The guidance keeps only the kit's per-tier sentences.
        assert_eq!(
            snapshot.guidance,
            vec![(
                "quick".to_string(),
                "the quick tier's head is exhausted; move it".to_string()
            )]
        );
        assert!(snapshot.refused.is_empty(), "{:?}", snapshot.refused);
    }

    #[test]
    fn a_dead_kit_port_is_a_named_refusal_not_a_panic() {
        let error = read_router(&sources(&dead_url()), Duration::from_secs(2))
            .expect_err("nothing is listening");
        assert!(
            error.contains("the router board could not read the kit"),
            "{error}"
        );
    }

    #[test]
    fn a_bad_token_is_named_with_the_kits_own_reason() {
        let kit = ScriptedKit::start(&[(
            "/api/state",
            401,
            "{\"error\":{\"message\":\"router: bad local token\"}}",
        )]);
        let source = RouterSources {
            base_url: kit.url(),
            token: Some("wrong".to_string()),
        };
        let error = read_router(&source, Duration::from_secs(5)).expect_err("the kit refuses");
        assert!(error.contains("router: bad local token"), "{error}");
    }

    #[test]
    fn a_hung_kit_read_is_bounded_by_its_own_timeout() {
        let kit = DeafKit::start();
        let started = std::time::Instant::now();
        let error = read_router(&sources(&kit.url()), Duration::from_millis(300))
            .expect_err("the kit never answers");
        let elapsed = started.elapsed();
        assert!(error.contains("could not be read"), "{error}");
        // The bound held: the read named itself instead of waiting.
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    }

    #[test]
    fn one_refused_usage_read_names_itself_and_the_rest_lands() {
        let kit = ScriptedKit::start(&[
            ("/api/state", 200, STATE_FIXTURE),
            ("/api/suggest", 200, SUGGEST_FIXTURE),
        ]);
        let snapshot = read_router(&sources(&kit.url()), Duration::from_secs(5))
            .expect("the state read is the spine");
        assert!(snapshot.totals.is_none());
        assert_eq!(snapshot.refused.len(), 1);
        assert!(
            snapshot.refused[0].contains("/api/usage"),
            "{}",
            snapshot.refused[0]
        );
    }

    #[test]
    fn the_router_board_names_the_kits_shape_drift() {
        // A kit whose payload renamed the members the board reads. The
        // sections that cannot be read come back empty, and each one
        // names what it looked for: an upgrade announces itself instead
        // of blanking part of the board.
        let kit = ScriptedKit::start(&[
            (
                "/api/state",
                200,
                r#"{"port":8300,"judge":{"strategy":"cascade"},"guard":{"providers":["deepseek"]},"resolved":{"chains":{}}}"#,
            ),
            (
                "/api/usage",
                200,
                r#"{"models":[{"key":"stepfun/step-3.7-flash","calls":1}],"recent":[]}"#,
            ),
        ]);
        let snapshot = read_router(&sources(&kit.url()), Duration::from_secs(5))
            .expect("the state read is the spine");

        assert!(snapshot.judge.is_none());
        assert!(snapshot.payg.is_none());
        assert!(snapshot.tiers.is_empty());
        assert!(snapshot.totals.is_none());
        let routes: Vec<&str> = snapshot
            .drift
            .iter()
            .map(|(route, _)| route.as_str())
            .collect();
        assert_eq!(
            routes,
            vec![
                "/api/state",
                "/api/state",
                "/api/state",
                "/api/usage",
                "/api/usage"
            ]
        );
        assert_eq!(snapshot.drift[0].1, "judge.mode is absent");
        assert_eq!(snapshot.drift[1].1, "guard.allowPayg is absent");
        assert_eq!(snapshot.drift[2].1, "resolved.workloads is absent");
        assert_eq!(snapshot.drift[3].1, "totals is absent");
        // A row the board cannot read is named too, rather than dropped.
        assert_eq!(snapshot.drift[4].1, "0 of 1 model rows could not be read");
    }

    #[test]
    fn the_kit_token_never_enters_a_snapshot_or_a_named_error() {
        const TOKEN: &str = "kit-local-token-4f9c2b7e";
        let kit = scripted_kit();
        let source = RouterSources {
            base_url: kit.url(),
            token: Some(TOKEN.to_string()),
        };
        let snapshot =
            read_router(&source, Duration::from_secs(5)).expect("the scripted kit answers");
        // The snapshot's serialization carries no credential...
        assert!(
            !format!("{snapshot:?}").contains(TOKEN),
            "the snapshot carries the token"
        );
        // ...and neither does a read that refused: a hung kit names its
        // URL and never the header it was sent.
        let deaf = DeafKit::start();
        let error = read_router(
            &RouterSources {
                base_url: deaf.url(),
                token: Some(TOKEN.to_string()),
            },
            Duration::from_millis(300),
        )
        .expect_err("the kit never answers");
        assert!(!error.contains(TOKEN), "{error}");
    }

    // --- the second-brain board ----------------------------------------------

    fn write_state(home: &Path, name: &str, body: &str) {
        std::fs::write(home.join(format!("{name}.json")), body).expect("the fixture file");
    }

    const GATES_FIXTURE: &str = r#"{
      "generated": "2026-10-04T16:13:35Z",
      "inbox": [ { "op": "zcode-gate", "ts": "2026-10-02T20:09:10Z", "verdict": "block",
                   "command": "cd /repo && python3 - <<'PYEOF'\nmore lines" } ],
      "board": {
        "arch-gate": { "verdicts": 10, "dispositions": 2, "fixed": 0, "waived": 2, "overridden": 0,
                       "override_rate": 0.0, "graded_progress": "2/20", "graded_rows": 0,
                       "fit_ready": false, "latency_median_h": 2.23, "latency_p90_h": 2.23,
                       "n_timed": 1, "oldest_open_h": null },
        "plan-gate": { "verdicts": 24, "dispositions": 9, "graded_progress": "9/20", "graded_rows": 9,
                       "fit_ready": true, "override_rate": 0.125 }
      },
      "daily": [ { "date": "2026-09-21", "count": 0 }, { "date": "2026-09-22", "count": 4 } ],
      "totals": { "events": 107, "verdicts": 80, "dispositions": 16 }
    }"#;

    const HARNESS_FIXTURE: &str = r#"{
      "generated": "2026-10-09T01:52:49Z",
      "totals": { "turns": 12, "ok": 11 },
      "loops": [ { "loop": "nightly", "runs": 3, "converged": 3, "last_status": "converged",
                   "last_run": "2026-10-09", "last_quality": [0.9], "last_iterations": 2,
                   "quality_series": [[0.9]] } ],
      "recent": [ { "ts": "2026-10-09T01:52:49Z", "kind": "turn", "session": "s1", "label": "run",
                    "outcome": "ok", "ms": 1200, "rounds": 3, "toolCalls": 4 } ]
    }"#;

    const LOOPS_FIXTURE: &str = r#"{
      "loops": [
        { "loop": "audio-clean", "family": "audio", "adapter": "AudioAdapter", "verdict": "passed",
          "iterations": 1, "adjuster_models": [], "flags_fixed": 0, "flags_new": 0,
          "pending_review": 0, "escalations": [] },
        { "loop": "ui-capture", "family": "ui", "adapter": "UiAdapter", "verdict": "failed",
          "iterations": 3, "flags_fixed": 2, "flags_new": 1, "pending_review": 1, "escalations": [] }
      ],
      "signal": { "glm-5.3-flash": { "loops": 2, "passed": 1, "failed": 1, "adjustments": 3,
                                     "flags_fixed": 2, "flags_new": 1, "adjuster_errors": 0,
                                     "families": ["audio"], "avg_iterations": 2, "max_iterations": 3 } },
      "graded_rows": { "ux_capture": { "n": 127, "confirmed": 45, "last7": 127 },
                       "audio_capture": { "n": 57, "confirmed": 54, "last7": 57 } },
      "findings": ["one finding", "another finding"],
      "derived_at": "2026-10-04T11:13:42"
    }"#;

    const MATURITY_FIXTURE: &str = r#"{
      "generated": "2026-10-04T11:13:42",
      "weeks": ["2026-09-14", "2026-09-21", "2026-09-28"],
      "series": { "output": [79, 119, 173], "continuity": [14, 21, 48] },
      "milestones": { "first plan": "2026-09-21", "tower online": "2026-10-01" }
    }"#;

    const REPORTS_FIXTURE: &str = r#"{
      "generated_at": "2026-10-04 16:18:56",
      "now": "2026-10-04 16:18:56",
      "malformed": 0,
      "events_total": 119,
      "doing": [ { "id": "t1", "project": "fleet", "title": "slice 14", "harness": "hermes",
                   "opened": "2026-10-04", "last": "2026-10-04" } ],
      "pending_verification": [ { "id": "t0", "project": "fleet", "title": "slice 13" } ],
      "conflicts": [ { "task": "t9", "title": "x", "claim": "done", "reality": "not",
                       "window": "7d", "detected_at": "2026-10-04", "severity": "high",
                       "project": "fleet", "type": "C1", "status": "open" } ],
      "open_conflict_count": 10,
      "trust": [ { "harness": "hermes", "claimed": 12, "evidenced": 11, "precision": 0.916 } ]
    }"#;

    const TOWER_DATA_FIXTURE: &str = r#"{
      "generated_at": "2026-10-09 01:52",
      "projects": [
        { "name": "fleet", "role": "own", "lifecycle": "active", "branch": "master",
          "last_commit": "2026-10-14", "commits_30d": 12, "dirty": 0,
          "plans": { "done": 9, "in_progress": 1, "planned": 2, "cancelled": 0, "unparsed": 1 } },
        { "name": "second-brain", "role": "own", "lifecycle": "active", "branch": "main",
          "last_commit": "2026-10-09", "commits_30d": 40, "dirty": 3,
          "plans": { "done": 4 } }
      ],
      "timeline": [ { "date": "2026-10-14", "subject": "slice 14", "project": "fleet" } ],
      "reports": {}, "harness": [], "harness_runs": {}
    }"#;

    #[test]
    fn the_second_brain_board_reads_the_six_derived_state_files() {
        let scratch = Scratch::new("brain");
        let home = scratch.path();
        write_state(home, "gates_state", GATES_FIXTURE);
        write_state(home, "harness_state", HARNESS_FIXTURE);
        write_state(home, "loops_state", LOOPS_FIXTURE);
        write_state(home, "maturity_state", MATURITY_FIXTURE);
        write_state(home, "reports_state", REPORTS_FIXTURE);
        write_state(home, "tower_data", TOWER_DATA_FIXTURE);

        let snapshot = read_brain(home, &Bounds::default()).expect("the home is readable");

        let gates = snapshot.gates.landed().expect("the gates state landed");
        assert_eq!(
            gates.totals,
            GateTotals {
                events: 107,
                verdicts: 80,
                dispositions: 16
            }
        );
        // The gate board is ordered by verdicts, busiest first.
        assert_eq!(gates.board[0].gate, "plan-gate");
        assert_eq!(gates.board[0].graded_progress, "9/20");
        assert!(gates.board[0].fit_ready);
        assert_eq!(gates.inbox.len(), 1);
        assert_eq!(gates.inbox[0].op, "zcode-gate");
        // A command payload's later lines stay off the card.
        assert_eq!(gates.inbox[0].command, "cd /repo && python3 - <<'PYEOF'");
        assert_eq!(
            gates.daily,
            vec![("2026-09-21".to_string(), 0), ("2026-09-22".to_string(), 4)]
        );

        let harness = snapshot.harness.landed().expect("the harness state landed");
        assert_eq!(harness.loops[0].name, "nightly");
        assert_eq!(harness.loops[0].converged, 3);
        assert_eq!(harness.recent[0].outcome, "ok");

        let loops = snapshot.loops.landed().expect("the loops state landed");
        assert_eq!(loops.loops.len(), 2);
        assert_eq!(loops.loops[1].verdict, "failed");
        // The graded rows are ordered by volume.
        assert_eq!(
            loops.graded[0],
            (
                "ux_capture".to_string(),
                GradedRow {
                    n: 127,
                    confirmed: 45
                }
            )
        );

        let maturity = snapshot
            .maturity
            .landed()
            .expect("the maturity state landed");
        assert_eq!(maturity.weeks.len(), 3);
        assert_eq!(
            maturity.series[0],
            ("continuity".to_string(), vec![14, 21, 48])
        );
        assert_eq!(
            maturity.milestones[0],
            ("first plan".to_string(), "2026-09-21".to_string())
        );

        let reports = snapshot.reports.landed().expect("the reports state landed");
        assert_eq!(reports.doing[0].title, "slice 14");
        assert_eq!(reports.pending_verification, 1);
        assert_eq!(reports.open_conflicts, 10);
        assert_eq!(reports.conflicts[0].class, "C1");
        assert_eq!(reports.trust[0].harness, "hermes");

        let tower = snapshot.tower.landed().expect("the tower data landed");
        assert_eq!(tower.projects.len(), 2);
        assert_eq!(tower.projects[0].name, "fleet");
        assert_eq!(tower.projects[0].plans[0], ("done".to_string(), 9));
    }

    #[test]
    fn a_missing_state_file_names_itself_and_the_other_five_land() {
        let scratch = Scratch::new("brain-missing");
        let home = scratch.path();
        for (name, body) in [
            ("gates_state", GATES_FIXTURE),
            ("harness_state", HARNESS_FIXTURE),
            ("loops_state", LOOPS_FIXTURE),
            ("reports_state", REPORTS_FIXTURE),
            ("tower_data", TOWER_DATA_FIXTURE),
        ] {
            write_state(home, name, body);
        }

        let snapshot = read_brain(home, &Bounds::default()).expect("the home is readable");
        assert_eq!(
            snapshot
                .maturity
                .reason()
                .map(|reason| reason.contains("maturity_state.json does not exist")),
            Some(true)
        );
        assert!(snapshot.gates.landed().is_some());
    }

    #[test]
    fn a_state_file_that_is_not_the_shape_it_claims_is_refused() {
        let scratch = Scratch::new("brain-shape");
        let home = scratch.path();
        // Valid JSON, wrong shape: the gates state is an object, not a list.
        write_state(home, "gates_state", "[1, 2, 3]");
        write_state(home, "loops_state", LOOPS_FIXTURE);

        let snapshot = read_brain(home, &Bounds::default()).expect("the home is readable");
        assert_eq!(
            snapshot
                .gates
                .reason()
                .map(|reason| reason.contains("gates_state")),
            Some(true)
        );
        assert!(snapshot.loops.landed().is_some());
    }

    #[test]
    fn an_unreadable_state_home_is_the_reads_refusal() {
        let missing =
            std::env::temp_dir().join(format!("fleet-boards-absent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&missing);
        let error = read_brain(&missing, &Bounds::default()).expect_err("the home does not exist");
        assert!(
            error.contains("the second-brain board could not read"),
            "{error}"
        );
        assert!(error.contains("does not exist"), "{error}");
    }

    #[test]
    fn the_second_brain_read_is_bounded_by_its_budget_and_names_the_unread_files() {
        let scratch = Scratch::new("brain-budget");
        let home = scratch.path();
        for (name, body) in [
            ("gates_state", GATES_FIXTURE),
            ("harness_state", HARNESS_FIXTURE),
            ("loops_state", LOOPS_FIXTURE),
            ("reports_state", REPORTS_FIXTURE),
            ("maturity_state", MATURITY_FIXTURE),
            ("tower_data", TOWER_DATA_FIXTURE),
        ] {
            write_state(home, name, body);
        }
        // A named pipe with no writer: reading it blocks in `open` until
        // a writer appears, which is what a state file on a hung mount
        // does. It stands in for the unbounded read the module doc used
        // to claim was bounded.
        let hung = home.join("gates_state.json");
        let _ = std::fs::remove_file(&hung);
        let made = Command::new("mkfifo")
            .arg(&hung)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "the named pipe was created");

        // The per-file bound outlasts the aggregate budget, so the first
        // file's bound is what spends it: the budget is a deadline
        // checked before each read, and a read already running finishes
        // against its own bound.
        let bounds = Bounds {
            file_read_timeout: Duration::from_millis(300),
            brain_budget: Duration::from_millis(100),
            ..Bounds::default()
        };
        let started = std::time::Instant::now();
        let snapshot = read_brain(home, &bounds).expect("the home is readable");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );

        // The hung file names its own bound...
        assert!(
            snapshot
                .gates
                .reason()
                .is_some_and(|reason| reason.contains("was not read")),
            "{:?}",
            snapshot.gates.reason()
        );
        // ...and the five behind it were never read, each naming the
        // budget rather than the cycle stalling behind them.
        let spent = |reason: Option<&str>| {
            reason.map(|reason| reason.contains("the read budget was spent"))
        };
        assert_eq!(
            spent(snapshot.harness.reason()),
            Some(true),
            "{:?}",
            snapshot.harness.reason()
        );
        assert_eq!(
            spent(snapshot.loops.reason()),
            Some(true),
            "{:?}",
            snapshot.loops.reason()
        );
        assert_eq!(
            spent(snapshot.reports.reason()),
            Some(true),
            "{:?}",
            snapshot.reports.reason()
        );
        assert_eq!(
            spent(snapshot.maturity.reason()),
            Some(true),
            "{:?}",
            snapshot.maturity.reason()
        );
        assert_eq!(
            spent(snapshot.tower.reason()),
            Some(true),
            "{:?}",
            snapshot.tower.reason()
        );
        // Release the reader the bounded read left behind: a writer
        // unblocks its `open`, so the thread ends instead of waiting for
        // the process's whole life.
        let release = hung.clone();
        thread::spawn(move || {
            let _ = std::fs::OpenOptions::new().write(true).open(release);
        });
    }

    // --- the tower ------------------------------------------------------------

    /// A scratch git repo with known facts: two commits on `main`, one
    /// dirty file, and no upstream.
    fn scratch_repo(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        std::fs::create_dir_all(&path).expect("the project directory");
        for args in [
            vec!["init", "--quiet"],
            vec!["config", "user.email", "board@example.invalid"],
            vec!["config", "user.name", "Board Test"],
            vec!["checkout", "--quiet", "-b", "main"],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(&args)
                .status()
                .expect("git runs");
            assert!(status.success(), "{args:?}");
        }
        for (index, message) in ["first commit", "second commit"].into_iter().enumerate() {
            // One file per commit: a second commit with nothing new to
            // land is refused by git, which is not the fact under test.
            std::fs::write(path.join(format!("file-{index}.txt")), format!("{index}\n"))
                .expect("the commit's file");
            for args in [vec!["add", "-A"], vec!["commit", "--quiet", "-m", message]] {
                let status = Command::new("git")
                    .arg("-C")
                    .arg(&path)
                    .args(&args)
                    .status()
                    .expect("git runs");
                assert!(status.success(), "{args:?}");
            }
        }
        // One dirty entry, and the head commit's subject to assert on.
        std::fs::write(path.join("dirty.txt"), "uncommitted\n").expect("the dirty file");
        path
    }

    #[test]
    fn the_tower_reads_git_facts_per_project() {
        let scratch = Scratch::new("tower");
        let root = scratch.path();
        scratch_repo(root, "alpha");
        scratch_repo(root, "beta");
        std::fs::create_dir_all(root.join("not-a-repo")).expect("the skipped directory");

        let snapshot = read_tower(root, &Bounds::default()).expect("the root is listable");
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(
            snapshot.skipped,
            vec![("not-a-repo".to_string(), "no .git".to_string())]
        );
        assert_eq!(snapshot.beyond_bound, 0);

        let alpha = &snapshot.projects[0];
        // Both repos are equally dirty, so the name breaks the tie.
        assert_eq!(alpha.name, "alpha");
        assert_eq!(alpha.branch.as_deref(), Some("main"));
        assert_eq!(alpha.dirty, 1);
        assert_eq!(alpha.commits_30d, 2);
        assert_eq!(alpha.subject.as_deref(), Some("second commit"));
        assert_eq!(alpha.head.as_deref().map(|sha| sha.len()), Some(7));
        assert!(
            alpha.when.as_deref().is_some_and(|when| when.len() == 10),
            "{:?}",
            alpha.when
        );
        // No upstream: ahead and behind read as unanswered rather than 0.
        assert_eq!(alpha.ahead, None);
        assert_eq!(alpha.behind, None);
        assert!(alpha.notes.is_empty(), "{:?}", alpha.notes);
    }

    #[test]
    fn the_tower_names_a_project_whose_git_calls_fail() {
        let scratch = Scratch::new("tower-broken");
        let root = scratch.path();
        scratch_repo(root, "healthy");
        // A directory with a `.git` that is not a gitdir: every git call
        // fails, and the project lands with its facts unanswered.
        let broken = root.join("broken");
        std::fs::create_dir_all(broken.join(".git")).expect("the broken gitdir");

        let snapshot = read_tower(root, &Bounds::default()).expect("the root is listable");
        assert_eq!(snapshot.projects.len(), 2);
        let broken = snapshot
            .projects
            .iter()
            .find(|project| project.name == "broken")
            .expect("the broken project still reports");
        assert_eq!(broken.branch, None);
        assert_eq!(broken.dirty, 0);
        assert!(!broken.notes.is_empty(), "{:?}", broken.notes);
        // The healthy project is unaffected by its neighbour's failure.
        assert!(snapshot
            .projects
            .iter()
            .any(|project| project.name == "healthy"));
    }

    #[test]
    fn an_unlistable_projects_root_is_the_reads_refusal() {
        let missing =
            std::env::temp_dir().join(format!("fleet-boards-no-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&missing);
        let error = read_tower(&missing, &Bounds::default()).expect_err("the root does not exist");
        assert!(error.contains("the tower could not list"), "{error}");
    }

    #[test]
    fn the_tower_scan_is_bounded_by_its_project_cap() {
        let scratch = Scratch::new("tower-bound");
        let root = scratch.path();
        for index in 0..4 {
            std::fs::create_dir_all(root.join(format!("repo-{index}")).join(".git"))
                .expect("the project directory");
        }
        let bounds = Bounds {
            max_projects: 2,
            ..Bounds::default()
        };
        let snapshot = read_tower(root, &bounds).expect("the root is listable");
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(snapshot.beyond_bound, 2);
        assert_eq!(snapshot.skipped.len(), 0);
    }

    /// A `git` that accepts its arguments and never answers, written into
    /// `root` and made executable: the hung source the per-call bound and
    /// the aggregate scan budget are both proven against.
    fn hung_git(root: &Path) -> PathBuf {
        let bin = root.join("git");
        std::fs::write(&bin, "#!/bin/sh\nsleep 30\n").expect("the stand-in git");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&bin).expect("the stand-in").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&bin, permissions).expect("the mode");
        }
        bin
    }

    #[test]
    fn a_hung_git_call_is_bounded_by_its_own_timeout() {
        let scratch = Scratch::new("tower-hung");
        let root = scratch.path();
        let project = root.join("hung");
        std::fs::create_dir_all(project.join(".git")).expect("the project directory");
        let bin = hung_git(root);
        let bounds = Bounds {
            git_bin: bin.display().to_string(),
            git_timeout: Duration::from_millis(200),
            ..Bounds::default()
        };
        let started = std::time::Instant::now();
        let snapshot = read_tower(root, &bounds).expect("the scan still lands");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            started.elapsed()
        );
        let project = &snapshot.projects[0];
        assert_eq!(project.branch, None);
        assert!(
            project
                .notes
                .iter()
                .any(|note| note.contains("git could not")),
            "{:?}",
            project.notes
        );
    }

    #[test]
    fn the_tower_scan_is_bounded_by_its_aggregate_budget() {
        let scratch = Scratch::new("tower-budget");
        let root = scratch.path();
        for index in 0..4 {
            std::fs::create_dir_all(root.join(format!("repo-{index}")).join(".git"))
                .expect("the project directory");
        }
        let bin = hung_git(root);
        // Every call spends its whole bound, so the only thing that can
        // stop this scan is the aggregate budget.
        let bounds = Bounds {
            git_bin: bin.display().to_string(),
            git_timeout: Duration::from_millis(200),
            scan_budget: Duration::from_millis(400),
            ..Bounds::default()
        };
        let started = std::time::Instant::now();
        let snapshot = read_tower(root, &bounds).expect("the scan still lands");
        let elapsed = started.elapsed();
        // The bound held: four projects of five bounded calls each would
        // have cost four seconds, and the scan stopped inside the budget.
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.unread, 3);
        assert!(snapshot.skipped.is_empty(), "{:?}", snapshot.skipped);
        // The project the budget cut short says so, so its unanswered
        // facts are not read as a project with no facts.
        let project = &snapshot.projects[0];
        assert!(
            project
                .notes
                .iter()
                .any(|note| note.contains("the scan budget was spent")),
            "{:?}",
            project.notes
        );
    }

    // --- the snapshot contract ------------------------------------------------

    #[test]
    fn a_refused_read_keeps_the_last_snapshot_and_names_the_error() {
        let mut boards = Boards::new();
        let kit = scripted_kit();
        let good = read_router(&sources(&kit.url()), Duration::from_secs(5))
            .expect("the scripted kit answers");
        let read_at = good.read_at.clone();
        boards.apply(BoardKind::Router, Ok(BoardData::Router(good)));
        assert!(boards.slot(BoardKind::Router).error.is_none());

        // The kit goes away: the read is refused.
        let refused = read_router(&sources(&dead_url()), Duration::from_secs(2))
            .expect_err("nothing is listening");
        boards.apply(BoardKind::Router, Err(refused));

        let slot = boards.slot(BoardKind::Router);
        let snapshot = slot
            .snapshot
            .as_ref()
            .expect("the last snapshot stays visible");
        assert_eq!(snapshot.read_at(), read_at);
        match snapshot {
            BoardData::Router(snapshot) => {
                assert_eq!(snapshot.totals.map(|t| t.calls), Some(19_873))
            }
            other => panic!("unexpected board data: {other:?}"),
        }
        let error = slot.error.as_deref().expect("the error is named");
        assert!(
            error.contains("the router board could not read the kit"),
            "{error}"
        );
    }

    #[test]
    fn a_never_read_board_names_its_reason_instead_of_showing_nothing() {
        let mut boards = Boards::new();
        boards.apply(
            BoardKind::Tower,
            Err("the tower could not list /nowhere: no such file or directory".to_string()),
        );
        let slot = boards.slot(BoardKind::Tower);
        assert!(slot.snapshot.is_none());
        assert!(!slot.loading);
        assert!(slot
            .error
            .as_deref()
            .is_some_and(|error| error.contains("the tower could not list")));
    }

    #[test]
    fn the_cycle_walks_the_three_boards_in_order() {
        let mut boards = Boards::new();
        assert_eq!(boards.active, BoardKind::Router);
        assert_eq!(boards.active.label(), "router");
        boards.cycle();
        assert_eq!(boards.active, BoardKind::Brain);
        assert_eq!(boards.slot(BoardKind::Brain).loading, true);
        boards.cycle();
        assert_eq!(boards.active, BoardKind::Tower);
        boards.cycle();
        assert_eq!(boards.active, BoardKind::Router);
        // Every slot is independent: one board's refusal is not another's.
        boards.apply(BoardKind::Tower, Err("named".to_string()));
        assert!(boards.slot(BoardKind::Router).error.is_none());
    }

    // --- the read-only contract -----------------------------------------------

    #[test]
    fn the_reads_never_write_to_their_sources() {
        let scratch = Scratch::new("readonly");
        let root = scratch.path();
        let project = scratch_repo(root, "alpha");
        let brain = root.join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        let kit = scripted_kit();

        let fingerprint = |path: &Path| -> Vec<(String, u64)> {
            fn walk(path: &Path, out: &mut Vec<(String, u64)>) {
                let Ok(entries) = std::fs::read_dir(path) else {
                    return;
                };
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        // `.git` is left out: a `git status` refreshes the
                        // index on its own, and that is git's housekeeping,
                        // not this board writing to the source.
                        if entry.file_name() != ".git" {
                            walk(&path, out);
                        }
                    } else if let Ok(metadata) = entry.metadata() {
                        out.push((path.display().to_string(), metadata.len()));
                    }
                }
            }
            let mut out = Vec::new();
            walk(path, &mut out);
            out.sort();
            out
        };

        let before_projects = fingerprint(root);
        let before_brain = fingerprint(&brain);

        let _ = read_router(&sources(&kit.url()), Duration::from_secs(5)).expect("the router read");
        let _ = read_brain(&brain, &Bounds::default()).expect("the brain read");
        let _ = read_tower(root, &Bounds::default()).expect("the tower read");

        assert_eq!(
            fingerprint(root),
            before_projects,
            "the tower's sources are untouched"
        );
        assert_eq!(
            fingerprint(&brain),
            before_brain,
            "the state files are untouched"
        );
        // The scratch repo is still on its own branch with its own dirty file.
        let branch = git(
            &project,
            "git",
            &["symbolic-ref", "--short", "HEAD"],
            Duration::from_secs(5),
        );
        assert_eq!(branch.as_deref().map(str::trim), Some("main"));
    }

    // --- the refresher -------------------------------------------------------

    #[test]
    fn the_refresher_lands_all_three_boards_from_its_own_thread() {
        let kit = scripted_kit();
        let scratch = Scratch::new("refresher");
        let brain = scratch.path().join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        write_state(&brain, "loops_state", LOOPS_FIXTURE);
        // One git repo in the projects root, and one directory that is not
        // one: the tower scan reports both.
        scratch_repo(scratch.path(), "alpha");

        let sources = Sources {
            router: sources(&kit.url()),
            brain_home: Some(brain),
            tower_root: scratch.path().to_path_buf(),
            bounds: Bounds::default(),
        };
        // The sender is dropped at the end of the test, which is what ends
        // the refresher's loop.
        let (_tx, mut rx) = spawn_refresher(sources);

        let mut landed = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while landed.len() < 3 && std::time::Instant::now() < deadline {
            // `None` is the refresher ending: its sender is gone.
            let Some(update) = rx.blocking_recv() else {
                break;
            };
            update
                .outcome
                .unwrap_or_else(|reason| panic!("{}: {reason}", update.kind.label()));
            landed.push(update.kind);
        }
        landed.sort_by_key(|kind| kind.index());
        assert_eq!(landed, BoardKind::ALL.to_vec(), "all three boards landed");
    }

    /// A std channel fed by the refresher's own, so a test can wait for
    /// an update that must not arrive rather than blocking on one that
    /// never comes.
    fn bridge(mut rx: UnboundedReceiver<BoardUpdate>) -> std::sync::mpsc::Receiver<BoardUpdate> {
        let (tx, bridged) = std::sync::mpsc::channel::<BoardUpdate>();
        thread::spawn(move || {
            while let Some(update) = rx.blocking_recv() {
                if tx.send(update).is_err() {
                    return;
                }
            }
        });
        bridged
    }

    #[test]
    fn the_refresher_survives_a_read_that_panics_and_keeps_reading() {
        let kit = scripted_kit();
        let scratch = Scratch::new("refresher-panic");
        let brain = scratch.path().join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        scratch_repo(scratch.path(), "alpha");
        // A scan budget no instant can hold makes the tower read's own
        // deadline computation panic: the stand-in for any read that dies
        // on the blocking pool, which is what the JoinError guard exists
        // for. The crate's hard-won CatchUnwind lesson from cron/eval,
        // pinned here rather than assumed.
        let sources = Sources {
            router: sources(&kit.url()),
            brain_home: Some(brain),
            tower_root: scratch.path().to_path_buf(),
            bounds: Bounds {
                scan_budget: Duration::MAX,
                ..Bounds::default()
            },
        };
        let (tx, rx) = spawn_refresher(sources);
        let bridged = bridge(rx);

        let mut landed = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while landed.len() < 3 && std::time::Instant::now() < deadline {
            let Ok(update) = bridged.recv() else { break };
            match update.outcome {
                Ok(_) => landed.push(update.kind),
                Err(reason) => {
                    // The panicking read is named, never swallowed.
                    assert_eq!(update.kind, BoardKind::Tower, "{reason}");
                    assert!(reason.contains("the tower read could not run"), "{reason}");
                    landed.push(update.kind);
                }
            }
        }
        assert_eq!(landed.len(), 3, "{landed:?}");
        assert!(landed.contains(&BoardKind::Router), "{landed:?}");
        assert!(landed.contains(&BoardKind::Brain), "{landed:?}");

        // The refresher's own thread is still alive: a refresh asked for
        // now reads again instead of the loop being dead.
        let _ = tx.send(BoardCommand::Refresh);
        let mut again = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while again.len() < 3 && std::time::Instant::now() < deadline {
            let Ok(update) = bridged.recv() else { break };
            again.push(update.kind);
        }
        assert_eq!(again.len(), 3, "the refresher kept reading: {again:?}");
    }

    #[test]
    fn refreshes_asked_for_during_a_read_are_not_queued_behind_it() {
        let kit = scripted_kit();
        let scratch = Scratch::new("refresher-dedupe");
        let brain = scratch.path().join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        scratch_repo(scratch.path(), "alpha");
        // A `git` that never answers: the opening read is still running
        // when the refreshes arrive, which is the case the dedupe is for.
        let bin = hung_git(scratch.path());
        let sources = Sources {
            router: sources(&kit.url()),
            brain_home: Some(brain),
            tower_root: scratch.path().to_path_buf(),
            bounds: Bounds {
                git_bin: bin.display().to_string(),
                git_timeout: Duration::from_millis(100),
                ..Bounds::default()
            },
        };
        let (tx, rx) = spawn_refresher(sources);
        let bridged = bridge(rx);
        // Three presses of the cycle key, all inside one read's window.
        for _ in 0..3 {
            let _ = tx.send(BoardCommand::Refresh);
        }

        let mut landed = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while landed.len() < 3 && std::time::Instant::now() < deadline {
            let Ok(update) = bridged.recv() else { break };
            landed.push(update.kind);
        }
        assert_eq!(landed.len(), 3, "{landed:?}");
        // Those three refreshes were satisfied by the read that was
        // already running: nothing else arrives, where a queued refresh
        // would have cost another full scan inside this window.
        let queued = bridged.recv_timeout(Duration::from_millis(1500));
        assert!(
            queued.is_err(),
            "a refresh asked for during a read ran anyway: {queued:?}"
        );
    }

    // --- the render vocabulary ------------------------------------------------

    #[test]
    fn compact_counts_group_and_abbreviate() {
        assert_eq!(compact(999), "999");
        assert_eq!(compact(19_873), "19,873");
        assert_eq!(compact(92_159_110_146), "92.2B");
        assert_eq!(compact(12_000_000), "12.0M");
    }

    /// The three boards against the machine's REAL sources: the router
    /// kit's operator API at 127.0.0.1:8300, the second-brain's derived
    /// state files, and the git facts per project under `~/Projects`.
    ///
    /// Ignored like the repo's live-API e2e, because it reads live local
    /// state that the deterministic gate must not depend on — the kit may
    /// be down, the pipeline may not have run. Run it with
    /// `cargo test --lib boards::tests::the_boards_read_the_machines_real_sources -- --ignored`.
    ///
    /// Every read is read-only: the loopback GETs, the file reads, and the
    /// `git` calls inspect and write nothing.
    #[ignore = "reads live local sources; run it explicitly to verify the boards against the real kit, second-brain, and projects root"]
    #[test]
    fn the_boards_read_the_machines_real_sources() {
        let sources = resolve_sources(&HashMap::new());
        let router = read_router(&sources.router, Duration::from_secs(READ_TIMEOUT_SECS));
        match &router {
            Ok(snapshot) => {
                eprintln!(
                    "router: {} tiers, judge {:?}, totals {:?}, {} model(s), {} recent",
                    snapshot.tiers.len(),
                    snapshot.judge,
                    snapshot.totals.map(|totals| totals.calls),
                    snapshot.models.len(),
                    snapshot.recent.len()
                );
                // The kit's own port, out of the kit's own rendered config.
                assert_eq!(sources.router.base_url, "http://127.0.0.1:8300");
                assert!(!snapshot.tiers.is_empty(), "the kit named no workload");
                // The live canary for a kit upgrade: the payload still
                // carries every member the board reads. An upgrade that
                // renames one fails here, naming the drift, instead of
                // quietly emptying a section on the board.
                assert!(
                    snapshot.drift.is_empty(),
                    "the kit's shape drifted: {:?}",
                    snapshot.drift
                );
            }
            // A kit that is not answering is the contract's named refusal,
            // not a failing test: the pane keeps its last snapshot.
            Err(reason) => eprintln!("router: refused \u{b7} {reason}"),
        }

        match sources.brain_home.as_ref() {
            Some(home) => {
                let brain = read_brain(home, &Bounds::default())
                    .expect("the second-brain home is readable");
                let landed = [
                    brain.gates.landed().is_some(),
                    brain.harness.landed().is_some(),
                    brain.loops.landed().is_some(),
                    brain.reports.landed().is_some(),
                    brain.maturity.landed().is_some(),
                    brain.tower.landed().is_some(),
                ];
                eprintln!(
                    "second-brain: {} \u{b7} {} of six state files landed",
                    home.display(),
                    landed.iter().filter(|landed| **landed).count()
                );
                assert!(landed.iter().any(|landed| *landed), "no state file landed");
            }
            None => eprintln!("second-brain: no home resolved"),
        }

        let tower = read_tower(&sources.tower_root, &sources.bounds);
        match &tower {
            Ok(snapshot) => {
                let dirty = snapshot
                    .projects
                    .iter()
                    .filter(|project| project.dirty > 0)
                    .count();
                eprintln!(
                    "tower: {} \u{b7} {} repos, {} dirty, {} skipped",
                    snapshot.root.display(),
                    snapshot.projects.len(),
                    dirty,
                    snapshot.skipped.len()
                );
                assert!(
                    !snapshot.projects.is_empty(),
                    "no project reported git facts"
                );
            }
            Err(reason) => eprintln!("tower: refused \u{b7} {reason}"),
        }
    }

    #[test]
    fn bars_and_sparklines_are_proportional() {
        // A half bar of eight cells is four full blocks.
        assert_eq!(bar(50, 100, 8), "\u{2588}\u{2588}\u{2588}\u{2588}    ");
        // A zero max draws blanks, never a full bar.
        assert_eq!(bar(5, 0, 4), "    ");
        assert_eq!(bar(0, 100, 4), "    ");
        // A quarter of eight is two full blocks and a half block.
        assert_eq!(bar(30, 100, 8), "\u{2588}\u{2588}\u{258d}     ");
        // The sparkline scales its series to its own maximum and pads the
        // front when the series is shorter than the width.
        assert_eq!(sparkline(&[1, 2, 4], 3), "\u{2582}\u{2584}\u{2588}");
        assert_eq!(sparkline(&[4], 3), "  \u{2588}");
        assert_eq!(sparkline(&[], 3), "   ");
    }

    #[test]
    fn cells_trim_and_pad_to_their_width() {
        assert_eq!(cell("router", 8), "router  ");
        assert_eq!(cell("second-brain-board", 8), "second-\u{2026}");
        assert_eq!(cell("abc", 2), "a\u{2026}");
    }
}
