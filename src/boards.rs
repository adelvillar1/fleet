//! The boards: three read-only surfaces over this machine's own sources,
//! mounted in the TUI's board pane.
//!
//! The three boards and their sources:
//!
//! | Board | Source | What it renders |
//! |---|---|---|
//! | router | Fleet's own routing state: the declared chain, the workload preference table, the gates, and the route rows Fleet writes for every run it serves | the workload tier chains with their candidates, what Fleet just served, and the day's counts |
//! | second-brain | the six derived state files under the second-brain home (`gates_state`, `harness_state`, `loops_state`, `reports_state`, `maturity_state`, `tower_data`) | the earned-autonomy gate board, the HITL inbox, the quality loops and their graded rows, the maturity sparkline, the doing/conflict counts |
//! | tower | git facts per project under `~/Projects` | per-project cards: branch, dirty dot, head commit, the trailing-thirty-day count, ahead/behind |
//!
//! **The router board reads nothing over the network.** Fleet's router is
//! this crate's own [`crate::route::select`] — sys1 asks for a workload
//! verdict, the workload preference table picks the provider, and
//! failover walks the declared chain — so there is no router process to
//! dial, no port, and no operator API. The owner's 2026-10-16 ruling is
//! that Fleet is a self-contained app with a router it owns; the
//! `agnostic-router-kit` this crate supersedes was a separate process,
//! and the router on 8300 belongs to the zcode harness. This board reads
//! the same three things the router reads and nothing else.
//!
//! **The snapshot contract.** Every board keeps the last snapshot that
//! landed. A refused read — an unreadable or missing state file, a git
//! failure — keeps that snapshot visible and names the error in the pane
//! on one line beside the time of the last good read. A board that has
//! never read successfully shows its empty state naming the reason.
//! Nothing here writes to any source: the reads are file reads and `git`
//! calls that only inspect.
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
//! **No credential reaches this module.** The router board needs none:
//! it reads Fleet's own routing state, which is this crate's data. The
//! second-brain and tower boards read files and git facts. Nothing here
//! authenticates to anything, and nothing here carries a key value.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// --- the seam ----------------------------------------------------------------

/// How often the refresher reads all three boards. The fork's boards carry
/// their own cadence (60s for the router and second-brain boards, 30
/// minutes for the tower); one cadence for all three keeps the pane's
/// contract in one place, and the tower scan's cost (~2s of bounded `git`
/// calls over thirty repos) is what sets it.
pub const BOARD_REFRESH_SECS: u64 = 30;

/// How many of a day's route rows the router board reads. The day file is
/// an audit trail, not a training set: the board shows what was just
/// served, so a bounded tail is the whole point rather than a compromise.
pub const ROUTER_ROW_CAP: usize = 64;

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
///
/// All three variants are boxed. A snapshot is a message, not a value the
/// caller holds: it is built once per read, carried over the update
/// channel, matched by reference, and dropped — so the box costs one
/// allocation per refresh and buys an enum the size of a pointer rather
/// than the size of its largest variant. The brain snapshot's six
/// `Section<T>` reads inline their reads into it and run to 616 bytes,
/// which is more than five times the tower's; without the boxes the enum
/// would be that large whichever board is actually being carried.
#[derive(Debug, Clone, PartialEq)]
pub enum BoardData {
    Router(Box<RouterSnapshot>),
    Brain(Box<BrainSnapshot>),
    Tower(Box<TowerSnapshot>),
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
    pub router: RouterState,
    pub brain_home: Option<PathBuf>,
    pub tower_root: PathBuf,
    pub bounds: Bounds,
}

/// The router board's source: Fleet's own routing state, resolved once
/// when the interface starts. There is no URL, no port and no token —
/// Fleet's router is this crate's own, so the board reads what the router
/// reads. A roster too broken to load would already have failed startup
/// loud, so the compiled-in defaults are the fallback here.
#[derive(Debug, Clone)]
pub struct RouterState {
    /// The declared chain, each candidate's key resolved the way the
    /// router resolves it.
    pub resolved: Vec<crate::route::Resolved>,
    /// Each workload's own candidates, in the table's order.
    pub preferences: Vec<crate::route::Preference>,
    pub gates: crate::route::Gates,
    pub pin: Option<String>,
    /// The calibration log root Fleet writes its route rows under.
    pub root: PathBuf,
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
pub fn resolve_sources(home: &Path, dotenv: &HashMap<String, String>) -> Sources {
    let setting = |name: &str| crate::home::resolve_setting(name, dotenv);
    Sources {
        router: router_state(home, dotenv),
        brain_home: brain_home(&setting),
        tower_root: tower_root(&setting),
        bounds: Bounds::default(),
    }
}

/// The router board's source: Fleet's own routing state. The roster is
/// the one the router itself runs on, loaded from `<home>/config.json`
/// exactly as [`crate::route::load_roster`] loads it, and the calibration
/// root is the one Fleet writes its route rows under. A config too broken
/// to parse has already failed startup loud, so the compiled-in defaults
/// are this reader's fallback rather than a silent second opinion.
fn router_state(home: &Path, dotenv: &HashMap<String, String>) -> RouterState {
    let roster = crate::route::load_roster(home).unwrap_or_else(|_| crate::route::Roster {
        candidates: crate::route::default_roster(),
        pin: None,
        preferences: crate::route::default_preferences(),
        gates: crate::route::Gates::default(),
    });
    RouterState {
        resolved: crate::route::resolve(&roster.candidates, dotenv),
        preferences: roster.preferences,
        gates: roster.gates,
        pin: roster.pin,
        root: crate::calib::resolve_root(home, std::env::var("HOME").ok().as_deref()),
    }
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

/// One board read of Fleet's own routing state.
#[derive(Debug, Clone, PartialEq)]
pub struct RouterSnapshot {
    pub read_at: String,
    /// The judgment source and the workload an unmapped ask defaults to.
    pub judge: Option<(String, Option<String>)>,
    /// The declared pin, when the owner set one.
    pub pin: Option<String>,
    /// Whether pay-as-you-go providers may be routed to, and their names.
    pub payg: Option<(bool, Vec<String>)>,
    /// The workload tier chains, in the table's own order: each workload's
    /// own candidates, the first of which is what a route selects.
    pub tiers: Vec<TierChain>,
    pub totals: Option<UsageTotals>,
    /// The per-model rows, busiest first.
    pub models: Vec<ModelRow>,
    /// The recent route rows, newest first.
    pub recent: Vec<RecentRow>,
    /// The day's calls bucketed by their own UTC hour, in clock order,
    /// with the hours that saw no call left out. A day column chart is
    /// this and nothing else: the shape of the day the ledger recorded.
    pub hours: Vec<HourColumn>,
    pub judge_counters: Option<JudgeCounters>,
    /// The workload mix, busiest first.
    pub workloads: Vec<(String, u64)>,
    /// Fleet's own failover feedback: the candidates the calibration is
    /// currently demoting, and the clause that says why.
    pub guidance: Vec<(String, String)>,
    /// The reads that did not land, each named with its reason.
    pub refused: Vec<String>,
}

/// One workload's tier chain: the candidates the workload names, in
/// order, the first of which is what a route selects.
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

/// One UTC hour of the day: the calls the ledger saw in it, and how many
/// of those ended in a walk that served. The chart's own column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HourColumn {
    pub hour: u32,
    pub calls: u64,
    pub errors: u64,
}

/// One recent route row, the board's "what Fleet just served" row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRow {
    /// The row's own stamp, exactly as the stream wrote it.
    pub at: String,
    pub provider: String,
    pub model: String,
    /// The walk's own outcome word: `served`, `failed`, or `skipped`.
    pub outcome: String,
    /// The candidates the walk actually attempted, in the order it
    /// attempted them. This is the failover history: a row whose walk
    /// took more than one candidate served because an earlier one did
    /// not, and the row says which.
    pub providers_used: Vec<String>,
    pub workload: Option<String>,
    pub ms: Option<u64>,
    pub reason: Option<String>,
}

impl RecentRow {
    /// Whether the row's walk served, which is what its state dot says.
    pub fn ok(&self) -> bool {
        self.outcome == "served"
    }
}

/// The judge's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeCounters {
    pub fresh: u64,
    pub cached: u64,
    pub errors: u64,
}

/// Read Fleet's own routing state. Nothing is dialed: the router is this
/// crate's own [`crate::route::select`], so the board reads the same three
/// things the router reads — the declared chain, the workload preference
/// table, and the route rows Fleet already wrote for the runs it served.
/// A day with no rows is not a refusal: the chain renders and the ledger
/// says so, the way a fresh install reads.
pub fn read_router(state: &RouterState, day: &str) -> RouterSnapshot {
    // The route rows are read by op, not as a tail of the file: a day's
    // stream is mixed-op, and on a busy day the last route row sits above
    // hundreds of later rows — a whole-stream cap reads none of them and
    // the board renders an empty day over a day that routed.
    let rows = crate::calib::read_op_day(&state.root, day, crate::calib::OP_ROUTE, ROUTER_ROW_CAP);
    let route_rows: Vec<&crate::calib::ReadRow> = rows.iter().collect();
    RouterSnapshot {
        read_at: now_stamp(),
        judge: Some((
            "sys1".to_string(),
            Some(state.gates.default_workload.to_string()),
        )),
        pin: state.pin.clone(),
        payg: Some(payg_guard(
            &state.resolved,
            &state.preferences,
            state.pin.as_deref(),
        )),
        tiers: tier_chains(&state.preferences),
        totals: route_totals(&route_rows),
        models: model_rows(&route_rows),
        recent: recent_rows(&route_rows),
        hours: hour_columns(&route_rows),
        judge_counters: judge_counters(&route_rows),
        workloads: workload_mix(&route_rows),
        guidance: demotion_guidance(state),
        refused: Vec::new(),
    }
}

/// The workload tier chains, in the table's own order. Each workload's
/// first candidate is what a route selects for it; the rest are what the
/// workload's own failover walks.
fn tier_chains(preferences: &[crate::route::Preference]) -> Vec<TierChain> {
    preferences
        .iter()
        .map(|preference| {
            let candidates: Vec<String> = preference
                .candidates
                .iter()
                .map(|(provider, model)| format!("{provider}/{model}"))
                .collect();
            TierChain {
                tier: preference.workload.to_string(),
                primary: candidates.first().cloned().unwrap_or_default(),
                candidates,
            }
        })
        .collect()
}

/// The pay-as-you-go guard, Fleet's own: a payg candidate may not serve
/// unless the owner pinned it or a workload names it, which is what makes
/// the reserve unable to serve silently.
fn payg_guard(
    resolved: &[crate::route::Resolved],
    preferences: &[crate::route::Preference],
    pin: Option<&str>,
) -> (bool, Vec<String>) {
    use crate::route::Billing;
    let providers: Vec<String> = resolved
        .iter()
        .filter(|entry| entry.candidate.billing == Billing::Payg)
        .map(|entry| entry.id().to_string())
        .collect();
    // A workload's own chain is the walk, so a payg candidate a workload
    // names may serve; a pin is the owner naming it directly. Neither
    // names it, and the reserve is walked last by nothing at all.
    let allowed = preferences
        .iter()
        .flat_map(|preference| preference.candidates.iter())
        .any(|(provider, _)| providers.iter().any(|name| name == provider))
        || pin.is_some_and(|pinned| providers.iter().any(|name| name == pinned));
    (allowed, providers)
}

/// The day's route-row counts: one call per row, a row whose walk ended
/// in a `served` attempt counts as ok, and every other row counts as an
/// error. Fleet's route rows carry no token counts, so the token fields
/// stay zero rather than being estimated from a character count.
///
/// "Ended in" is the row's last attempt, not any attempt it walked
/// through: a session's attempts accumulate across the calls it made, so
/// a row whose first call served and whose second failed ended failed —
/// the same reading the row's own state dot gives, and the reason the
/// ledger and the row list cannot disagree about one run.
fn route_totals(rows: &[&crate::calib::ReadRow]) -> Option<UsageTotals> {
    if rows.is_empty() {
        return None;
    }
    let ok = rows
        .iter()
        .filter(|row| {
            row.attempts
                .last()
                .is_some_and(|attempt| attempt.outcome == "served")
        })
        .count() as u64;
    Some(UsageTotals {
        calls: rows.len() as u64,
        ok,
        errors: rows.len() as u64 - ok,
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    })
}

/// The per-model rows, busiest first: how often each served model was the
/// one a route selected.
fn model_rows(rows: &[&crate::calib::ReadRow]) -> Vec<ModelRow> {
    let mut counts: Vec<(String, u64)> = Vec::new();
    for row in rows {
        if let Some(entry) = counts.iter_mut().find(|(key, _)| *key == row.model) {
            entry.1 += 1;
        } else {
            counts.push((row.model.clone(), 1));
        }
    }
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts
        .into_iter()
        .map(|(model, calls)| {
            let provider = model.split('/').next().unwrap_or(&model).to_string();
            ModelRow {
                key: model.clone(),
                provider,
                model,
                calls,
                errors: 0,
                total_tokens: 0,
            }
        })
        .collect()
}

/// The recent route rows, newest first. The day file is an append-only
/// audit trail, so its tail is what was just served.
fn recent_rows(rows: &[&crate::calib::ReadRow]) -> Vec<RecentRow> {
    let mut recent: Vec<RecentRow> = rows
        .iter()
        .rev()
        .take(6)
        .map(|row| {
            let outcome = row
                .attempts
                .last()
                .map(|attempt| attempt.outcome.clone())
                .unwrap_or_else(|| "never attempted".to_string());
            RecentRow {
                at: row.ts.clone(),
                provider: row.provider.clone(),
                model: row.model.clone(),
                outcome,
                providers_used: row.providers_used.clone(),
                workload: (!row.workload.is_empty()).then(|| row.workload.clone()),
                ms: Some(row.latency_ms),
                reason: (!row.reason.is_empty()).then(|| row.reason.clone()),
            }
        })
        .collect();
    recent.truncate(6);
    recent
}

/// The day's calls bucketed by the hour their own stamp says, in clock
/// order. An hour with no calls is not a zero-height column: the ledger
/// has nothing to say about it, and a chart that drew it would be
/// inventing the shape of a day it never saw. The stamp is the crate's
/// one time vocabulary, so a row that carries no hour is simply not
/// counted rather than guessed at.
fn hour_columns(rows: &[&crate::calib::ReadRow]) -> Vec<HourColumn> {
    let mut hours: Vec<HourColumn> = Vec::new();
    for row in rows {
        let hour = row
            .ts
            .as_bytes()
            .get(11..13)
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| digits.parse::<u32>().ok());
        let Some(hour) = hour.filter(|hour| *hour < 24) else {
            continue;
        };
        let served = row
            .attempts
            .iter()
            .any(|attempt| attempt.outcome == "served");
        if let Some(column) = hours.iter_mut().find(|column| column.hour == hour) {
            column.calls += 1;
            if !served {
                column.errors += 1;
            }
        } else {
            hours.push(HourColumn {
                hour,
                calls: 1,
                errors: u64::from(!served),
            });
        }
    }
    hours.sort_by_key(|column| column.hour);
    hours
}

/// The judgment split: a judgment made this run, one served from the
/// route-decision cache, or one never obtained. A cached judgment is a
/// recorded one, so the counts must not pretend otherwise.
fn judge_counters(rows: &[&crate::calib::ReadRow]) -> Option<JudgeCounters> {
    if rows.is_empty() {
        return None;
    }
    let fresh = rows.iter().filter(|row| row.judged_from == "fresh").count() as u64;
    let cached = rows
        .iter()
        .filter(|row| row.judged_from == "cached")
        .count() as u64;
    let errors = rows.iter().filter(|row| row.judged_from == "none").count() as u64;
    Some(JudgeCounters {
        fresh,
        cached,
        errors,
    })
}

/// The workload mix, busiest first: where the day's routes actually went.
fn workload_mix(rows: &[&crate::calib::ReadRow]) -> Vec<(String, u64)> {
    let mut counts: Vec<(String, u64)> = Vec::new();
    for row in rows {
        let workload = if row.workload.is_empty() {
            "\u{2014}".to_string()
        } else {
            row.workload.clone()
        };
        if let Some(entry) = counts.iter_mut().find(|(name, _)| *name == workload) {
            entry.1 += 1;
        } else {
            counts.push((workload, 1));
        }
    }
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts
}

/// Fleet's own failover feedback: the candidates the calibration is
/// demoting for this day, and the clause that says why. This is the
/// precursor's per-tier suggestion, read out of Fleet's own stream rather
/// than another process's operator API.
fn demotion_guidance(state: &RouterState) -> Vec<(String, String)> {
    let ids: Vec<&str> = state.resolved.iter().map(|entry| entry.id()).collect();
    let demoted = crate::calib::demotions(&state.root, &crate::calib::today(), &ids);
    demoted
        .into_iter()
        .map(|id| (id.clone(), "demoted \u{2014} walked last".to_string()))
        .collect()
}

// --- the delegation panel ----------------------------------------------------

/// How many session logs one delegation read considers. The logs are
/// the tree's only source, so the bound is on the scan rather than on
/// the tree: the newest ids are read first, and a home with more logs
/// than this names the rest rather than reading without end.
pub const DELEGATION_LOG_CAP: usize = 256;

/// How many past trees the panel carries. The unit is one orchestrator
/// run, so eight is eight runs of history — enough to see a pattern and
/// few enough that the panel stays a picture.
pub const DELEGATION_TREE_CAP: usize = 8;

/// The aggregate bound on one delegation read, on the tower scan's own
/// rule: a sum of bounded reads is not itself bounded. Once it is spent
/// the tree stops where it stands and the snapshot names that it did.
pub const DELEGATION_SCAN_BUDGET_SECS: u64 = 5;

/// The bound on one log's opening read. A `start` record carries the
/// version, the model, the stamp and at most two names, so a window this
/// size reads it whole and cannot be made to read a model's whole first
/// answer.
const DELEGATION_START_BYTES: u64 = 2048;

/// The bound on one log's head read, which is where the goal lives: the
/// first user record, the brief the run was handed. A goal that does not
/// fit the window is absent rather than the read growing to find it.
const DELEGATION_HEAD_BYTES: u64 = 64 * 1024;

/// The bound on one log's tail read, which is where an orderly stop
/// lives: the `end` record is the last line, so a window this size reads
/// it whole however large the run grew.
const DELEGATION_TAIL_BYTES: u64 = 4096;

/// One read of the delegation surface: the live run's tree, the past
/// runs' trees, and the declared dependency graph with the unlock
/// conditions the code actually requires.
///
/// The tree comes from the session logs and nothing else. Every
/// `start` record names the session that delegated to it, which is what
/// makes the chain reconstructable from the logs alone (hard rule 5) —
/// the same walk [`crate::subagents::Chain::resolve`] walks, run from
/// the other end. The depth cap is the code's own
/// [`crate::subagents::SUBAGENT_MAX_DEPTH`], read from the constant
/// rather than restated, and it rides the snapshot so the panel shows
/// the wall rather than a tree that merely stops.
///
/// The graph comes from `<home>/graph.json` through
/// [`crate::graph::Graph::load`], which names every failure rather than
/// defaulting. A refused graph read leaves `graph` `None` and names the
/// refusal, because an empty graph silently substituted for a missing
/// one would render as "nothing is blocking anything" — the one reading
/// this surface must never produce by accident.
#[derive(Debug, Clone, PartialEq)]
pub struct DelegationSnapshot {
    /// When this read landed, the same stamp every board carries.
    pub read_at: String,
    /// The depth cap the tree is drawn against, which is the code's own
    /// constant rather than this module's idea of one.
    pub depth_cap: usize,
    /// The live run's tree, rooted at the session this face is serving.
    /// `None` when that session's log cannot be read, which names
    /// itself rather than drawing an empty tree that reads as "this run
    /// delegated nothing".
    pub live: Option<DelegationNode>,
    /// The past runs' trees, newest root first: one tree per
    /// orchestrator run.
    pub history: Vec<DelegationNode>,
    /// The declared dependency graph, or `None` with a named refusal.
    pub graph: Option<GraphSnapshot>,
    /// Everything this read could not do, named. An empty list means
    /// every source answered.
    pub refused: Vec<String>,
}

/// One session in a tree: what it ran, where it sits, and what it was
/// handed.
#[derive(Debug, Clone, PartialEq)]
pub struct DelegationNode {
    pub id: String,
    pub model: String,
    /// The `kind` the log's `start` record carries, when it carries one:
    /// `subagent` for a delegated run, `scheduled` for a job's.
    pub kind: Option<String>,
    /// How many parents stand above this node. The orchestrator's own
    /// run is 0, its children 1, its grandchildren 2.
    pub depth: usize,
    /// The task it was handed: its first user record's brief.
    pub goal: Option<String>,
    /// `running` while the log carries no `end` record, `stopped` once
    /// it does — the distinction the session module itself draws.
    pub status: String,
    /// The code's own stop word, verbatim: `completed`, `max-turns`,
    /// `failed`, an owner's stop.
    pub stop_reason: Option<String>,
    /// The turns the stop recorded.
    pub turns: Option<u64>,
    pub children: Vec<DelegationNode>,
}

/// The declared dependency graph, flattened for the browser: the nodes
/// in declaration order, the edges as index pairs into that order, and
/// the graph's own aggregates.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphSnapshot {
    pub nodes: Vec<GraphNode>,
    /// `(prerequisite index, node index)`. A prerequisite that is not a
    /// node is not an edge — it is named in that node's `unresolved`,
    /// which is where the code puts it.
    pub edges: Vec<(usize, usize)>,
    pub critical_path: Vec<String>,
    pub blocks_most_work: Option<String>,
    pub revisions: usize,
    pub diff_score: u64,
    pub replan: bool,
}

/// One graph node with the conditions the code actually requires, read
/// off [`crate::graph::Graph::unlock`] rather than derived a second time.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub cost: u64,
    /// The remaining work in this node's dependency closure.
    pub score: u64,
    /// The highest that score has been: the readiness bar's denominator.
    pub peak: u64,
    /// The status the file carries.
    pub status: String,
    /// The state this node stands in, named by the same function the
    /// terminal render uses, so the two cannot disagree.
    pub state: String,
    pub approach: Option<String>,
    pub paths: Vec<String>,
    /// The capability declaration, as the phrase the graph renders.
    pub capabilities: String,
    pub prereqs: Vec<String>,
    pub unlock: UnlockShape,
    /// The named reason this node is held, or `None` when it is not.
    pub reason: Option<String>,
    pub delta: Option<DeltaShape>,
}

/// The three unlock conditions, reported separately and all necessary —
/// the code's own reading, not a summary of it.
#[derive(Debug, Clone, PartialEq)]
pub struct UnlockShape {
    pub score_zero: bool,
    pub resource_free: bool,
    pub deps_verified: bool,
    pub unlocked: bool,
    pub waiting: Vec<String>,
    pub failed: Vec<String>,
    pub unresolved: Vec<String>,
    /// `(holder, path)`: the sibling running this node's admitted path.
    pub held: Vec<(String, String)>,
}

impl UnlockShape {
    fn of(unlock: &crate::graph::Unlock) -> Self {
        Self {
            score_zero: unlock.score_zero,
            resource_free: unlock.resource_free,
            deps_verified: unlock.deps_verified,
            unlocked: unlock.is_unlocked(),
            waiting: unlock.waiting.clone(),
            failed: unlock.failed.clone(),
            unresolved: unlock.unresolved.clone(),
            held: unlock
                .held
                .iter()
                .map(|hold| (hold.holder.clone(), hold.path.clone()))
                .collect(),
        }
    }
}

/// The angle between the approach a node was handed and the one it ran,
/// as the categories the delta's own vocabulary names.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaShape {
    pub state: String,
    pub reason: String,
}

/// One log's opening record: what it runs, the `kind` that marks a
/// delegated or scheduled run, and the session that delegated to it.
#[derive(Debug, Clone)]
struct StartShape {
    model: String,
    kind: Option<String>,
    parent: Option<String>,
}

/// One log's shape, once its head and tail have been read.
#[derive(Debug, Clone)]
struct SessionShape {
    id: String,
    model: String,
    kind: Option<String>,
    goal: Option<String>,
    stop_reason: Option<String>,
    turns: Option<u64>,
}

/// One read of the delegation panel's two pictures.
///
/// The scan is bounded three ways — a cap on the logs, a byte window per
/// read, and a wall-clock budget across the whole thing — because the
/// panel refreshes while a run is in flight and a read that grows with
/// the run it is watching is a read that eventually stops landing.
pub fn read_delegation(home: &Path, session: &str) -> DelegationSnapshot {
    let sessions = crate::home::sessions_dir(home);
    let deadline = Instant::now() + Duration::from_secs(DELEGATION_SCAN_BUDGET_SECS);
    let mut refused: Vec<String> = Vec::new();
    let mut starts: HashMap<String, StartShape> = HashMap::new();
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();

    let mut names: Vec<String> = match std::fs::read_dir(&sessions) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    // The ids are timestamp-shaped, so ascending is oldest first: reading
    // the tail of that order reads the recent runs, which is the half of
    // the history a tree is for.
    names.sort();
    let dropped = names.len().saturating_sub(DELEGATION_LOG_CAP);
    if dropped > 0 {
        refused.push(format!(
            "{dropped} session logs are older than the newest {DELEGATION_LOG_CAP}"
        ));
    }
    for id in names.iter().rev().take(DELEGATION_LOG_CAP) {
        if Instant::now() >= deadline {
            refused.push("the session scan spent its budget before it finished".to_string());
            break;
        }
        let Some(start) = start_shape(&sessions.join(format!("{id}.jsonl"))) else {
            continue;
        };
        if let Some(parent) = start.parent.clone().filter(|parent| !parent.is_empty()) {
            children_of.entry(parent).or_default().push(id.clone());
        }
        starts.insert(id.clone(), start);
    }
    // The live tree's root is read even when the cap or the budget
    // dropped it: a tree without its root is not a tree.
    if !starts.contains_key(session) {
        if let Some(start) = start_shape(&sessions.join(format!("{session}.jsonl"))) {
            starts.insert(session.to_string(), start);
        }
    }
    for children in children_of.values_mut() {
        children.sort();
    }

    let depth = depth_of(&starts, session);
    let live = tree_of(
        &sessions,
        &starts,
        &children_of,
        session,
        depth,
        deadline,
        &mut refused,
    );
    if live.is_none() {
        refused.push(format!("{session} has no readable session log"));
    }

    // A past tree is rooted at a session that delegated to somebody and
    // was itself delegated to by nobody — an orchestrator's own run. A
    // mid-tree node's subtree already sits inside its root's tree, so
    // listing it again would draw one run twice.
    let mut roots: Vec<String> = children_of
        .keys()
        .filter(|id| id.as_str() != session)
        .filter(|id| {
            starts
                .get(id.as_str())
                .and_then(|start| start.parent.clone())
                .is_none_or(|parent| parent.is_empty())
        })
        .cloned()
        .collect();
    roots.sort_by(|left, right| right.cmp(left));
    roots.truncate(DELEGATION_TREE_CAP);
    let mut history: Vec<DelegationNode> = Vec::new();
    for root in &roots {
        if let Some(tree) = tree_of(
            &sessions,
            &starts,
            &children_of,
            root,
            depth_of(&starts, root),
            deadline,
            &mut refused,
        ) {
            history.push(tree);
        }
    }

    let graph = graph_snapshot(home, &mut refused);
    DelegationSnapshot {
        read_at: now_stamp(),
        depth_cap: crate::subagents::SUBAGENT_MAX_DEPTH,
        live,
        history,
        graph,
        refused,
    }
}

/// How many parents stand above `id`, walked off the logs' own `start`
/// records — the walk [`crate::subagents::Chain::resolve`] walks, from
/// the top down. The walk stops at a log that names no parent, at a log
/// that was not read, or at the cap, whichever comes first.
fn depth_of(starts: &HashMap<String, StartShape>, id: &str) -> usize {
    let mut depth = 0;
    let mut cursor = id.to_string();
    while depth < crate::subagents::SUBAGENT_MAX_DEPTH {
        match starts
            .get(&cursor)
            .and_then(|start| start.parent.clone())
            .filter(|parent| !parent.is_empty())
        {
            Some(parent) => {
                depth += 1;
                cursor = parent;
            }
            None => break,
        }
    }
    depth
}

/// One tree, grown from `id` downward.
///
/// A node at the depth cap grows no children, because the code refuses
/// to make them: the tree stops at the wall rather than drawing a
/// subtree that could not have run. A node whose own log cannot be read
/// is absent, and the tree it would have rooted is absent with it.
fn tree_of(
    sessions: &Path,
    starts: &HashMap<String, StartShape>,
    children_of: &HashMap<String, Vec<String>>,
    id: &str,
    depth: usize,
    deadline: Instant,
    refused: &mut Vec<String>,
) -> Option<DelegationNode> {
    let start = starts.get(id)?;
    let shape = session_shape(sessions, id, start);
    let mut node = DelegationNode {
        id: shape.id,
        model: shape.model,
        kind: shape.kind,
        depth,
        goal: shape.goal,
        status: if shape.stop_reason.is_some() {
            "stopped".to_string()
        } else {
            "running".to_string()
        },
        stop_reason: shape.stop_reason,
        turns: shape.turns,
        children: Vec::new(),
    };
    if depth < crate::subagents::SUBAGENT_MAX_DEPTH {
        if let Some(children) = children_of.get(id) {
            for child in children {
                if Instant::now() >= deadline {
                    refused.push(format!(
                        "the tree stopped at {id} \u{2014} the read spent its budget"
                    ));
                    break;
                }
                if let Some(grown) = tree_of(
                    sessions,
                    starts,
                    children_of,
                    child,
                    depth + 1,
                    deadline,
                    refused,
                ) {
                    node.children.push(grown);
                }
            }
        }
    }
    Some(node)
}

/// One log's opening record, read inside its byte window. `None` for a
/// log that cannot be opened or whose first line is not a start record.
fn start_shape(path: &Path) -> Option<StartShape> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file).take(DELEGATION_START_BYTES);
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    match serde_json::from_str::<crate::session::Record>(&line).ok()? {
        crate::session::Record::Start {
            model,
            kind,
            parent,
            ..
        } => Some(StartShape {
            model,
            kind,
            parent,
        }),
        _ => None,
    }
}

/// One log's shape: its opening record, the goal it was handed, and the
/// stop it recorded. The head read is where the goal lives and the tail
/// read is where an orderly stop lives, and neither is read past its
/// window, so a log that has grown to hundreds of kilobytes costs what a
/// fresh one costs.
///
/// The sessions directory and the id, rather than a path: a caller that
/// held a path could hold the directory's, and reading a directory for a
/// goal reads nothing while looking like a log that carried none.
fn session_shape(sessions: &Path, id: &str, start: &StartShape) -> SessionShape {
    let path = sessions.join(format!("{id}.jsonl"));
    let (stop_reason, turns) = read_stop(&path);
    SessionShape {
        id: id.to_string(),
        model: start.model.clone(),
        kind: start.kind.clone(),
        goal: read_goal(&path),
        stop_reason,
        turns,
    }
}

/// The task a log was handed: its first user record, which is the brief.
/// `None` for a log whose head window holds no user record.
fn read_goal(path: &Path) -> Option<String> {
    let mut text = String::new();
    File::open(path)
        .ok()?
        .take(DELEGATION_HEAD_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    for line in text.lines() {
        if let Ok(crate::session::Record::User { content }) =
            serde_json::from_str::<crate::session::Record>(line)
        {
            if !content.trim().is_empty() {
                return Some(crate::subagents::Brief::parse(&content).task);
            }
        }
    }
    None
}

/// The stop a log recorded: the `end` record, which is the last line of
/// an orderly log. A log without one was interrupted, which reads as
/// `(None, None)` rather than as a stop that never happened.
fn read_stop(path: &Path) -> (Option<String>, Option<u64>) {
    let Ok(mut file) = File::open(path) else {
        return (None, None);
    };
    let Ok(len) = file.seek(SeekFrom::End(0)) else {
        return (None, None);
    };
    // Clamped, because a seek backwards past the start of a short log is
    // an error rather than a read from zero.
    let from = len.saturating_sub(DELEGATION_TAIL_BYTES);
    if file.seek(SeekFrom::Start(from)).is_err() {
        return (None, None);
    }
    let mut bytes = Vec::new();
    if file
        .take(DELEGATION_TAIL_BYTES)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return (None, None);
    }
    // Read as bytes and decoded lossily: the window's first byte can
    // land inside a multi-byte character, and the record this read is
    // for is the last one, not the first.
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines().rev() {
        if let Ok(crate::session::Record::End { stop_reason, turns }) =
            serde_json::from_str::<crate::session::Record>(line)
        {
            return (Some(stop_reason), Some(turns));
        }
    }
    (None, None)
}

/// The declared graph, with every node's actual conditions. A missing
/// file is not a failure — a home that has declared no graph is the
/// ordinary state — but it is named, because an empty graph rendered in
/// its place would read as "nothing is blocking anything".
fn graph_snapshot(home: &Path, refused: &mut Vec<String>) -> Option<GraphSnapshot> {
    let path = crate::graph::path_in(home);
    if !path.exists() {
        refused.push(format!("{} holds no declared graph", path.display()));
        return None;
    }
    let graph = match crate::graph::Graph::load(&path) {
        Ok(graph) => graph,
        Err(error) => {
            refused.push(error);
            return None;
        }
    };
    let nodes: Vec<GraphNode> = graph
        .nodes()
        .iter()
        .map(|node| {
            let unlock = graph.unlock(&node.id).unwrap_or_default();
            GraphNode {
                id: node.id.clone(),
                cost: node.cost,
                score: graph.score(&node.id).unwrap_or(0),
                peak: node.peak(),
                status: node.status.as_str().to_string(),
                state: crate::graph::state_word(node, &unlock).to_string(),
                approach: (!node.approach.trim().is_empty()).then(|| node.approach.clone()),
                paths: node.paths.clone(),
                capabilities: node.capabilities.phrase(),
                prereqs: node.prereqs.clone(),
                unlock: UnlockShape::of(&unlock),
                reason: unlock.reason(),
                delta: node.delta.as_ref().map(|delta| DeltaShape {
                    state: delta.state.label().to_string(),
                    reason: delta.reason.label().to_string(),
                }),
            }
        })
        .collect();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (index, node) in graph.nodes().iter().enumerate() {
        for prereq in &node.prereqs {
            if let Some(from) = graph.nodes().iter().position(|other| &other.id == prereq) {
                edges.push((from, index));
            }
        }
    }
    Some(GraphSnapshot {
        nodes,
        edges,
        critical_path: graph.critical_path(),
        blocks_most_work: graph.blocks_most_work(),
        revisions: graph.revisions().len(),
        diff_score: graph.diff_score(),
        replan: graph.should_replan(),
    })
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
        .map(|(gate, row)| GateRow {
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
                .map(|row| HarnessEvent {
                    ts: text(row, "ts").unwrap_or_default(),
                    kind: text(row, "kind").unwrap_or_default(),
                    label: text(row, "label").unwrap_or_default(),
                    outcome: text(row, "outcome").unwrap_or_default(),
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
                .map(|row| ConflictRow {
                    task: text(row, "task").unwrap_or_default(),
                    class: text(row, "type").unwrap_or_default(),
                    severity: text(row, "severity").unwrap_or_default(),
                    claim: text(row, "claim").unwrap_or_default(),
                    reality: text(row, "reality").unwrap_or_default(),
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
    // The router board reads Fleet's own stream, so its day is today's —
    // the same day the router's own rows are stamped with.
    let day = crate::calib::today();
    let (router, brain, tower) = tokio::join!(
        tokio::task::spawn_blocking(move || Ok(read_router(&router, &day))),
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
                router.map(|inner| inner.map(|snapshot| BoardData::Router(Box::new(snapshot)))),
                "router board",
            ),
        },
        BoardUpdate {
            kind: BoardKind::Brain,
            outcome: landed(
                brain.map(|inner| inner.map(|snapshot| BoardData::Brain(Box::new(snapshot)))),
                "second-brain board",
            ),
        },
        BoardUpdate {
            kind: BoardKind::Tower,
            outcome: landed(
                tower.map(|inner| inner.map(|snapshot| BoardData::Tower(Box::new(snapshot)))),
                "tower",
            ),
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

    /// Fleet's own routing state, the way the board reads it: the
    /// compiled-in chain and table, and a scratch calibration root so the
    /// rows a test writes are the rows it reads back.
    fn state(scratch: &Scratch) -> RouterState {
        RouterState {
            resolved: crate::route::resolve(&crate::route::default_roster(), &HashMap::new()),
            preferences: crate::route::default_preferences(),
            gates: crate::route::Gates::default(),
            pin: None,
            root: scratch.path().to_path_buf(),
        }
    }

    /// One `route` row, the shape the stream writes. A test names the
    /// provider that served, the workload sys1 judged, where the judgment
    /// came from, and the walk's own attempts.
    fn route_row(
        ts: &str,
        provider: &str,
        model: &str,
        workload: &str,
        judged_from: &str,
        attempts: &[(&str, &str)],
        latency_ms: u64,
    ) -> String {
        serde_json::json!({
            "op": crate::calib::OP_ROUTE,
            "ts": ts,
            "provider": provider,
            "how": "judged",
            "workload": workload,
            "reason": "the lane judged the workload",
            "model": model,
            "providers_used": attempts.iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            "attempts": attempts
                .iter()
                .map(|(p, outcome)| serde_json::json!({ "provider": p, "outcome": outcome }))
                .collect::<Vec<_>>(),
            "input_chars": 420,
            "input_hash": "0123456789abcdef",
            "latency_ms": latency_ms,
            "judged_from": judged_from,
        })
        .to_string()
    }

    /// One `tool_screen` row, the shape the stream writes. A busy day's
    /// screens are what bury the route rows, so the regression test
    /// writes a day of them.
    fn screen_row(ts: &str) -> String {
        serde_json::json!({
            "op": crate::calib::OP_TOOL_SCREEN,
            "ts": ts,
            "tool": "read",
            "verdict": "allow",
        })
        .to_string()
    }

    /// Write `lines` as one day's stream file under `root`.
    fn write_day(root: &Path, day: &str, lines: &[String]) {
        let path = crate::calib::day_path(root, day);
        std::fs::create_dir_all(path.parent().expect("the day file's parent"))
            .expect("the log directory");
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("the day file");
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

    // --- the router board ----------------------------------------------------

    /// A day of route rows: two served, one served after a failover, one
    /// whose walk failed outright. The mix is what lets the board prove
    /// each counter reads its own column.
    fn a_days_rows() -> Vec<String> {
        vec![
            route_row(
                "2026-10-16T09:14:02Z",
                "stepfun",
                "stepfun/step-3.7-flash",
                "quick",
                "fresh",
                &[("stepfun", "served")],
                812,
            ),
            route_row(
                "2026-10-16T09:15:41Z",
                "zai-coding-plan",
                "zai-coding-plan/GLM-5.3-Flash",
                "standard_code",
                "cached",
                &[("stepfun", "failed"), ("zai-coding-plan", "served")],
                1_904,
            ),
            route_row(
                "2026-10-16T09:22:10Z",
                "stepfun",
                "stepfun/step-5-preview",
                "hard",
                "none",
                &[("stepfun", "failed")],
                3_311,
            ),
            route_row(
                "2026-10-16T09:31:57Z",
                "stepfun",
                "stepfun/step-3.7-flash",
                "quick",
                "fresh",
                &[("stepfun", "served")],
                744,
            ),
        ]
    }

    #[test]
    fn the_router_board_reads_fleets_own_routing_state_into_its_snapshot() {
        let scratch = Scratch::new("router-state");
        let day = "2026-10-16";
        write_day(scratch.path(), day, &a_days_rows());
        let state = state(&scratch);

        let snapshot = read_router(&state, day);

        // The judge and the gates are Fleet's own: sys1 asked the workload,
        // and the workload table says what an unmapped ask defaults to.
        assert_eq!(
            snapshot.judge,
            Some(("sys1".to_string(), Some("standard_code".to_string())))
        );
        assert!(snapshot.pin.is_none());
        // The tier chains are the preference table, in its own order, and
        // each workload's first candidate is what a route selects.
        let tiers: Vec<&str> = snapshot
            .tiers
            .iter()
            .map(|chain| chain.tier.as_str())
            .collect();
        assert_eq!(
            tiers,
            vec![
                "quick",
                "standard_code",
                "hard",
                "prose",
                "deep_context",
                "composed"
            ]
        );
        assert_eq!(snapshot.tiers[0].primary, "stepfun/step-3.7-flash");
        assert_eq!(snapshot.tiers[0].candidates.len(), 3);
        assert_eq!(
            snapshot.tiers[1].candidates,
            vec![
                "stepfun/step-3.7-flash".to_string(),
                "zai-coding-plan/GLM-5.3-Flash".to_string(),
                "xiaomi-mimo/mimo-v2.6-flash".to_string(),
            ]
        );
        // The ledger's totals count the day's route rows.
        let totals = snapshot.totals.expect("the day's totals");
        assert_eq!(totals.calls, 4);
        // Three rows ended in a `served` attempt — two directly, one after
        // the walk failed over — and one row's walk failed outright.
        assert_eq!(totals.ok, 3);
        assert_eq!(totals.errors, 1);
        assert_eq!(totals.success_rate(), Some(75));
        // The route row carries no token counts, and the board says so
        // rather than estimating them from the task's length.
        assert_eq!((totals.prompt_tokens, totals.completion_tokens), (0, 0));
        // The busiest model is the one the day served most.
        assert_eq!(snapshot.models[0].model, "stepfun/step-3.7-flash");
        assert_eq!(snapshot.models[0].calls, 2);
        assert_eq!(snapshot.models[0].provider, "stepfun");
        // The recent rows are newest first, and the walk's own outcome
        // is what a row says.
        assert_eq!(snapshot.recent.len(), 4);
        assert_eq!(snapshot.recent[0].at, "2026-10-16T09:31:57Z");
        assert!(snapshot.recent[0].ok());
        assert_eq!(snapshot.recent[1].outcome, "failed");
        assert_eq!(
            snapshot.recent[1].reason.as_deref(),
            Some("the lane judged the workload")
        );
        assert_eq!(snapshot.recent[1].ms, Some(3_311));
        // The walk the row actually took, in the order it took it. This
        // is the failover history: the row that served after a failure
        // names both candidates, and the order says which one fell.
        assert_eq!(
            snapshot.recent[2].providers_used,
            vec!["stepfun".to_string(), "zai-coding-plan".to_string()]
        );
        assert_eq!(
            snapshot.recent[1].providers_used,
            vec!["stepfun".to_string()],
            "a row whose walk failed outright walked the one candidate it named"
        );
        assert_eq!(
            snapshot.recent[0].providers_used,
            vec!["stepfun".to_string()],
            "a row that served first try walked one candidate"
        );
        assert_eq!(
            snapshot.recent[0].workload.as_deref(),
            Some("quick"),
            "the workload sys1 judged rides the row"
        );
        // The judge split, out of `judged_from`.
        let judge = snapshot.judge_counters.expect("the judge counters");
        assert_eq!((judge.fresh, judge.cached, judge.errors), (2, 1, 1));
        // The workload mix, busiest first.
        assert_eq!(
            snapshot.workloads,
            vec![
                ("quick".to_string(), 2),
                ("hard".to_string(), 1),
                ("standard_code".to_string(), 1),
            ]
        );
        // Nothing was refused: every read is Fleet's own file.
        assert!(snapshot.refused.is_empty(), "{:?}", snapshot.refused);
    }

    /// The defect the board shipped with: a day's stream is mixed-op, and
    /// the route rows were read as a tail of the whole file. On a busy day
    /// the last route row sits above hundreds of later rows — here the
    /// afternoon's screens, which is a real day's shape — so the whole
    /// stream fell outside its cap and the ledger read an empty day over
    /// a day that routed. The rows are read by op now.
    #[test]
    fn the_router_board_reads_a_busy_days_route_rows_past_the_other_ops() {
        let scratch = Scratch::new("router-busy-day");
        let day = "2026-10-16";
        let mut lines = a_days_rows();
        for index in 0..(ROUTER_ROW_CAP * 4) {
            lines.push(screen_row(&format!(
                "2026-10-16T14:{:02}:{:02}Z",
                (index / 60) % 60,
                index % 60
            )));
        }
        write_day(scratch.path(), day, &lines);
        let state = state(&scratch);

        let snapshot = read_router(&state, day);

        // The morning's four route rows survive the afternoon's screens.
        assert_eq!(snapshot.recent.len(), 4);
        assert_eq!(snapshot.recent[0].at, "2026-10-16T09:31:57Z");
        let totals = snapshot.totals.expect("the day's totals");
        assert_eq!(totals.calls, 4);
        assert_eq!(totals.ok, 3);
        assert_eq!(totals.errors, 1);
        assert_eq!(totals.success_rate(), Some(75));
        // The hour columns, the model mix and the workload mix all read
        // off the same rows, so they are the day's rather than nothing.
        assert_eq!(snapshot.models[0].calls, 2);
        assert_eq!(snapshot.workloads.len(), 3);
        assert!(
            !snapshot.hours.is_empty(),
            "the day's hour columns came from the route rows"
        );
        assert!(snapshot.refused.is_empty(), "{:?}", snapshot.refused);
    }

    /// The other half of the same day: a session's attempts accumulate
    /// across the calls it made, so a row can walk through a served
    /// attempt and still end failed. The ledger reads the attempt the
    /// walk ended on — the same one the row's own state dot reads — or
    /// the panel shows one run succeeding and failing at the same time.
    #[test]
    fn the_ledger_counts_a_row_by_the_attempt_its_walk_ended_on() {
        let scratch = Scratch::new("router-ended");
        let day = "2026-10-16";
        write_day(
            scratch.path(),
            day,
            &[
                route_row(
                    "2026-10-16T09:14:02Z",
                    "stepfun",
                    "stepfun/step-3.7-flash",
                    "quick",
                    "fresh",
                    &[("stepfun", "served")],
                    812,
                ),
                route_row(
                    "2026-10-16T09:15:41Z",
                    "stepfun",
                    "stepfun/step-3.7-flash",
                    "quick",
                    "fresh",
                    &[
                        ("stepfun", "served"),
                        ("stepfun", "failed"),
                        ("stepfun", "failed"),
                    ],
                    1_223,
                ),
            ],
        );
        let state = state(&scratch);

        let snapshot = read_router(&state, day);

        let totals = snapshot.totals.expect("the day's totals");
        assert_eq!((totals.calls, totals.ok, totals.errors), (2, 1, 1));
        assert_eq!(totals.success_rate(), Some(50));
        // The row list says the same thing about the same row, newest
        // first: the 09:15 row is the one that walked through a serve and
        // ended failed, and its dot says so.
        assert!(!snapshot.recent[0].ok(), "the newest row ended failed");
        assert_eq!(snapshot.recent[0].outcome, "failed");
        assert_eq!(snapshot.recent[0].providers_used.len(), 3);
        assert!(snapshot.recent[1].ok(), "the older row served first try");
    }

    #[test]
    fn a_day_with_no_route_rows_is_an_empty_ledger_not_a_refusal() {
        // A fresh install, or a day Fleet has not routed on: the chain
        // still renders and the ledger says it has seen nothing. There
        // is nothing to dial, so there is nothing that can refuse.
        let scratch = Scratch::new("router-empty");
        let state = state(&scratch);

        let snapshot = read_router(&state, "2026-10-16");

        assert!(!snapshot.tiers.is_empty(), "the chain still renders");
        assert!(snapshot.totals.is_none());
        assert!(snapshot.recent.is_empty());
        assert!(
            snapshot.hours.is_empty(),
            "a day with no rows has no columns, and the chart draws nothing rather than a row of zeroes"
        );
        assert!(snapshot.judge_counters.is_none());
        assert!(snapshot.models.is_empty());
        assert!(snapshot.workloads.is_empty());
        assert!(snapshot.refused.is_empty(), "{:?}", snapshot.refused);
    }

    #[test]
    fn the_day_columns_are_the_days_own_hour_shape() {
        // A day's calls bucketed by the hour their own stamp says, in
        // clock order, with the errors the walks made in each. The chart
        // is this and nothing else — an hour the ledger never saw is
        // left out rather than drawn as a zero, because a column for a
        // quiet hour would be the board inventing the shape of a day.
        let scratch = Scratch::new("router-hours");
        let day = "2026-10-16";
        let rows = vec![
            route_row(
                "2026-10-16T09:14:02Z",
                "stepfun",
                "stepfun/step-3.7-flash",
                "quick",
                "fresh",
                &[("stepfun", "served")],
                812,
            ),
            route_row(
                "2026-10-16T09:31:57Z",
                "stepfun",
                "stepfun/step-3.7-flash",
                "quick",
                "fresh",
                &[("stepfun", "served")],
                744,
            ),
            route_row(
                "2026-10-16T09:22:10Z",
                "stepfun",
                "stepfun/step-5-preview",
                "hard",
                "none",
                &[("stepfun", "failed")],
                3_311,
            ),
            route_row(
                "2026-10-16T14:02:00Z",
                "stepfun",
                "stepfun/step-5-preview",
                "hard",
                "fresh",
                &[("stepfun", "served")],
                2_010,
            ),
        ];
        write_day(scratch.path(), day, &rows);
        let state = state(&scratch);

        let snapshot = read_router(&state, day);

        assert_eq!(
            snapshot.hours,
            vec![
                HourColumn {
                    hour: 9,
                    calls: 3,
                    errors: 1
                },
                HourColumn {
                    hour: 14,
                    calls: 1,
                    errors: 0
                },
            ]
        );
        // The columns agree with the ledger they came out of.
        let totals = snapshot.totals.expect("the day's totals");
        let summed: u64 = snapshot.hours.iter().map(|column| column.calls).sum();
        assert_eq!(summed, totals.calls);
    }

    #[test]
    fn the_payg_guard_names_the_reserve_and_what_may_serve() {
        // The reserve is payg: it may not serve unless the owner pinned it
        // or a workload names it. No workload in the compiled-in table
        // names deepseek, and nothing is pinned, so the guard says payg is
        // off and names deepseek as the payg candidate.
        let scratch = Scratch::new("router-payg");
        let state = state(&scratch);

        let snapshot = read_router(&state, "2026-10-16");

        let (allowed, providers) = snapshot.payg.expect("the guard reads");
        assert!(!allowed, "no workload names a payg candidate");
        assert_eq!(providers, vec!["deepseek".to_string()]);
    }

    #[test]
    fn a_pinned_payg_candidate_may_serve() {
        let scratch = Scratch::new("router-pin");
        let mut state = state(&scratch);
        state.pin = Some("deepseek".to_string());

        let snapshot = read_router(&state, "2026-10-16");

        let (allowed, _) = snapshot.payg.expect("the guard reads");
        assert!(allowed, "the owner pinned the payg candidate");
    }

    #[test]
    fn the_boards_guidance_reads_the_calibrations_own_demotions() {
        // Fleet's own failover feedback, read out of the day's route rows:
        // a candidate whose trailing attempts are all failed is demoted,
        // and the board names it with the clause that says why.
        let scratch = Scratch::new("router-demote");
        // The calibration grades today's rows, so the fixture rides today.
        let day = crate::calib::today();
        let rows: Vec<String> = (0..crate::calib::DEMOTION_THRESHOLD)
            .map(|index| {
                route_row(
                    &format!("2026-10-16T1{index}:00:00Z"),
                    "stepfun",
                    "stepfun/step-3.7-flash",
                    "quick",
                    "fresh",
                    &[("stepfun", "failed")],
                    500,
                )
            })
            .collect();
        write_day(scratch.path(), &day, &rows);
        let state = state(&scratch);

        let snapshot = read_router(&state, &day);

        let demoted: Vec<&str> = snapshot
            .guidance
            .iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(demoted, vec!["stepfun"]);
        assert_eq!(snapshot.guidance[0].1, "demoted \u{2014} walked last");
    }

    #[test]
    fn no_calibration_demotion_is_no_guidance() {
        let scratch = Scratch::new("router-no-demote");
        let day = "2026-10-16";
        write_day(scratch.path(), day, &a_days_rows());
        let state = state(&scratch);

        let snapshot = read_router(&state, day);

        assert!(snapshot.guidance.is_empty(), "{:?}", snapshot.guidance);
    }

    #[test]
    fn the_boards_read_the_machines_real_sources() {
        // The three boards against the machine's REAL sources: Fleet's own
        // routing state under `~/.fleet`, the second-brain's derived state
        // files, and the git facts per project under `~/Projects`.
        //
        // Ignored like the repo's live-API e2e, because it reads live local
        // state the deterministic gate must not depend on. Run it with
        // `cargo test --lib boards::tests::the_boards_read_the_machines_real_sources -- --ignored`.
        //
        // Every read is read-only: the file reads and the `git` calls
        // inspect and write nothing.
        let dotenv = HashMap::new();
        let home = crate::home::fleet_home(
            std::env::var("FLEET_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        )
        .expect("a fleet home is known");
        let sources = resolve_sources(&home, &dotenv);
        let snapshot = read_router(&sources.router, &crate::calib::today());
        eprintln!(
            "router: {} tiers, judge {:?}, totals {:?}, {} model(s), {} recent",
            snapshot.tiers.len(),
            snapshot.judge,
            snapshot.totals.map(|totals| totals.calls),
            snapshot.models.len(),
            snapshot.recent.len()
        );
        // The chain Fleet itself routes on, out of the compiled-in roster.
        assert!(!snapshot.tiers.is_empty(), "no workload was named");
        assert_eq!(snapshot.tiers[0].tier, "quick");
        assert!(snapshot.refused.is_empty(), "{:?}", snapshot.refused);

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
        let scratch = Scratch::new("contract");
        let mut boards = Boards::new();
        // Fleet's own router read cannot refuse: there is nothing to dial,
        // so the router slot's snapshot lands and holds.
        let good = read_router(&state(&scratch), "2026-10-16");
        let read_at = good.read_at.clone();
        boards.apply(BoardKind::Router, Ok(BoardData::Router(Box::new(good))));
        assert!(boards.slot(BoardKind::Router).error.is_none());

        // A board whose read can still fail — the tower scan — refuses,
        // and its refusal names itself without touching the router slot.
        boards.apply(
            BoardKind::Tower,
            Err("the tower could not list /nowhere: no such file or directory".to_string()),
        );

        let slot = boards.slot(BoardKind::Router);
        let snapshot = slot
            .snapshot
            .as_ref()
            .expect("the last snapshot stays visible");
        assert_eq!(snapshot.read_at(), read_at);
        match snapshot {
            BoardData::Router(snapshot) => {
                assert!(
                    !snapshot.tiers.is_empty(),
                    "the chain renders whatever the day's ledger says"
                );
            }
            other => panic!("unexpected board data: {other:?}"),
        }
        assert!(
            slot.error.is_none(),
            "one board's refusal is not another's: {:?}",
            slot.error
        );
        let error = boards
            .slot(BoardKind::Tower)
            .error
            .as_deref()
            .expect("the tower's error is named");
        assert!(error.contains("the tower could not list"), "{error}");
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
        assert!(boards.slot(BoardKind::Brain).loading);
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
        // The router board's own source: a calibration root under the
        // scratch, read the way the board reads it.
        let router = state(&Scratch::new("readonly-router"));

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

        let _ = read_router(&router, "2026-10-16");
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

    // --- the delegation panel ------------------------------------------------

    /// A session log in a scratch home: the `start` record the tree is
    /// built off, the brief it was handed, and the stop it recorded. A
    /// test writes the log directly rather than through `Session`,
    /// because the panel's read is of the file on disk.
    fn write_log(
        sessions: &Path,
        id: &str,
        model: &str,
        kind: Option<&str>,
        parent: Option<&str>,
        goal: &str,
        stop: Option<(&str, u64)>,
    ) {
        let mut start = serde_json::json!({
            "type": "start",
            "v": crate::session::SESSION_FORMAT_VERSION,
            "model": model,
            "created": "2026-10-16T09:00:00Z",
        });
        if let Some(kind) = kind {
            start["kind"] = serde_json::json!(kind);
        }
        if let Some(parent) = parent {
            start["parent"] = serde_json::json!(parent);
        }
        let mut lines = vec![
            start.to_string(),
            serde_json::json!({ "type": "user", "content": goal }).to_string(),
        ];
        if let Some((reason, turns)) = stop {
            lines.push(
                serde_json::json!({ "type": "end", "stop_reason": reason, "turns": turns })
                    .to_string(),
            );
        }
        std::fs::write(
            sessions.join(format!("{id}.jsonl")),
            format!("{}\n", lines.join("\n")),
        )
        .expect("the session log");
    }

    /// A home whose sessions directory holds the logs, and whose
    /// `graph.json` declares three nodes in a chain.
    fn delegation_home(name: &str) -> (Scratch, String) {
        let scratch = Scratch::new(name);
        let sessions = scratch.path().join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        let root = "20261016T090000Z-root";
        write_log(
            &sessions,
            root,
            "deepseek-chat",
            None,
            None,
            "revamp the delegation panel",
            None,
        );
        write_log(
            &sessions,
            "20261016T090100Z-child",
            "stepfun/step-3.7-flash",
            Some(crate::session::SUBAGENT_KIND),
            Some(root),
            "read the session logs",
            Some(("completed", 3)),
        );
        write_log(
            &sessions,
            "20261016T090200Z-grandchild",
            "zai-coding-plan/GLM-5.3-Flash",
            Some(crate::session::SUBAGENT_KIND),
            Some("20261016T090100Z-child"),
            "read the graph file",
            None,
        );
        // A great-grandchild: the depth cap means the code never made
        // this one, so the tree must stop at the grandchild.
        write_log(
            &sessions,
            "20261016T090300Z-great",
            "deepseek-chat",
            Some(crate::session::SUBAGENT_KIND),
            Some("20261016T090200Z-grandchild"),
            "a run that could not have happened",
            None,
        );
        write_graph(scratch.path());
        (scratch, root.to_string())
    }

    /// A declared graph file: six nodes that between them stand in every
    /// state the code names, so one read proves the panel reports the
    /// conditions rather than a summary of them.
    ///
    /// Written as the file the panel reads, because the file is the
    /// contract: a fixture built through the in-memory API would be one
    /// transition away from a graph the surface never sees.
    fn write_graph(home: &Path) {
        let node = |id: &str, cost: u64, prereqs: &[&str], paths: &[&str], status: &str| {
            serde_json::json!({
                "id": id,
                "cost": cost,
                "prereqs": prereqs,
                "paths": paths,
                "capabilities": { "tools": ["read"], "skills": [], "workflows": [], "loops": [] },
                "approach": format!("the approach for {id}"),
                "status": status,
            })
        };
        let file = serde_json::json!({
            "nodes": [
                // Verified: its closure is empty, so the score-zero
                // condition holds and it is unlocked.
                node("probe", 2, &[], &[], "verified"),
                // Pending behind a verified node: a verified prereq
                // carries no cost into the score, so this one's closure
                // is zero and all three conditions hold. It is `ready`,
                // which is the row that keeps `ready` from reading as
                // "nothing is left to do".
                node("plan", 3, &["probe"], &[], "pending"),
                // Pending behind a pending node: it waits on `plan` by
                // name, the deps condition fails, and the score carries
                // what is left behind it. The graph names the dep rather
                // than hiding it.
                node("build", 5, &["plan"], &[], "pending"),
                // Held: `run` is in flight on this node's admitted path,
                // so the resource condition is the one that fails. Its
                // own score is zero and its deps are verified, so a
                // summary of the three would read this node as ready.
                node("audit", 1, &[], &["src/boards.rs"], "pending"),
                node("run", 1, &[], &["src/boards.rs"], "running"),
                // A prerequisite that is not a node: it is named in
                // `unresolved`, it is not an edge, and it is the only
                // thing separating this node from `plan`.
                node("ghost", 1, &["missing"], &[], "pending"),
            ],
            "revisions": [],
        });
        std::fs::write(
            crate::graph::path_in(home),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&file).expect("the graph json")
            ),
        )
        .expect("the declared graph");
    }

    #[test]
    fn the_delegation_read_builds_the_live_tree_from_the_logs_own_start_records() {
        let (scratch, root) = delegation_home("delegation-live");
        let snapshot = read_delegation(scratch.path(), &root);

        let live = snapshot.live.as_ref().expect("the live tree");
        assert_eq!(live.id, root);
        assert_eq!(live.model, "deepseek-chat");
        assert_eq!(live.depth, 0);
        // The root's log has no `end` record, so it is still running: the
        // distinction the session module itself draws.
        assert_eq!(live.status, "running");
        assert_eq!(live.stop_reason, None);
        assert_eq!(live.goal.as_deref(), Some("revamp the delegation panel"));

        assert_eq!(live.children.len(), 1, "{live:?}");
        let child = &live.children[0];
        assert_eq!(child.kind.as_deref(), Some(crate::session::SUBAGENT_KIND));
        assert_eq!(child.depth, 1);
        assert_eq!(child.model, "stepfun/step-3.7-flash");
        assert_eq!(child.status, "stopped");
        assert_eq!(child.stop_reason.as_deref(), Some("completed"));
        assert_eq!(child.turns, Some(3));

        // The grandchild is at the cap: the code refuses to make it a
        // parent, so the tree stops there even though its log exists.
        assert_eq!(child.children.len(), 1, "{child:?}");
        let grandchild = &child.children[0];
        assert_eq!(grandchild.depth, 2);
        assert_eq!(grandchild.status, "running");
        assert!(
            grandchild.children.is_empty(),
            "a node at the cap grew children: {:?}",
            grandchild.children
        );
    }

    #[test]
    fn the_depth_cap_is_the_codes_own() {
        let (scratch, root) = delegation_home("delegation-cap");
        let snapshot = read_delegation(scratch.path(), &root);
        // The cap is read off the code, not written into the panel.
        assert_eq!(
            snapshot.depth_cap,
            crate::subagents::SUBAGENT_MAX_DEPTH,
            "the panel's cap drifted from the code's"
        );
        // A great-grandchild's log is on disk and still not in the tree.
        assert!(
            snapshot
                .refused
                .iter()
                .all(|reason| !reason.contains("great-grandchild")),
            "{:?}",
            snapshot.refused
        );
        let text = format!("{snapshot:?}");
        assert!(
            !text.contains("a run that could not have happened"),
            "a run past the cap was drawn: {text}"
        );
    }

    #[test]
    fn the_dependency_graph_carries_the_conditions_the_code_actually_requires() {
        let (scratch, root) = delegation_home("delegation-graph");
        let snapshot = read_delegation(scratch.path(), &root);
        let graph = snapshot.graph.as_ref().expect("the declared graph");

        assert_eq!(graph.nodes.len(), 6);
        // One edge per declared prerequisite, as index pairs — and the
        // prerequisite that is not a node is not an edge, because the
        // code puts it in `unresolved` instead.
        assert_eq!(graph.edges, vec![(0, 1), (1, 2)]);
        // The critical path is the graph's own: the head is the highest
        // remaining score, and the walk stops at a verified prerequisite.
        assert_eq!(graph.critical_path, vec!["build", "plan"]);

        // A verified node: its closure is empty, so the score-zero
        // condition holds and it is unlocked.
        let probe = &graph.nodes[0];
        assert_eq!(probe.status, "verified");
        assert_eq!(probe.state, "verified");
        assert_eq!(probe.score, 0, "{probe:?}");
        assert!(probe.unlock.score_zero, "{probe:?}");
        assert!(probe.unlock.resource_free, "{probe:?}");
        assert!(probe.unlock.deps_verified, "{probe:?}");
        assert!(probe.unlock.unlocked, "{probe:?}");
        assert!(probe.unlock.waiting.is_empty(), "{probe:?}");

        // A pending node behind a verified one: the score counts what is
        // left in the closure, and a verified prerequisite carries none,
        // so all three conditions hold and it reads `ready`. This is the
        // row that proves the three are not one summary — `plan` and
        // `build` are both pending with pending-only dependencies, and
        // the difference between them is which condition fails.
        let plan = &graph.nodes[1];
        assert_eq!(plan.state, "ready");
        assert_eq!(plan.score, 0, "{plan:?}");
        assert!(plan.unlock.score_zero, "{plan:?}");
        assert!(plan.unlock.resource_free, "{plan:?}");
        assert!(plan.unlock.deps_verified, "{plan:?}");
        assert!(plan.unlock.unlocked, "{plan:?}");
        assert!(plan.unlock.waiting.is_empty(), "{plan:?}");
        assert_eq!(plan.approach.as_deref(), Some("the approach for plan"));
        assert_eq!(plan.capabilities, "tools: read");

        // A pending node behind a pending one: the dep it waits on is
        // named, and it is not verified — so the deps condition fails
        // and the score carries what is left behind it.
        let build = &graph.nodes[2];
        assert_eq!(build.state, "waiting");
        assert_eq!(build.score, 3, "{build:?}");
        assert!(!build.unlock.score_zero, "{build:?}");
        assert!(build.unlock.resource_free, "{build:?}");
        assert!(!build.unlock.deps_verified, "{build:?}");
        assert_eq!(build.unlock.waiting, vec!["plan".to_string()]);
        assert!(build.unlock.failed.is_empty(), "{build:?}");
        assert!(build.unlock.unresolved.is_empty(), "{build:?}");

        // The held node: the resource condition is the one that fails,
        // and the sibling holding the path is named. Its own score is
        // zero and its deps are verified, so a one-word summary of the
        // three would read this node as ready.
        let audit = &graph.nodes[3];
        assert_eq!(audit.state, "held");
        assert_eq!(audit.score, 0, "{audit:?}");
        assert!(audit.unlock.score_zero, "{audit:?}");
        assert!(audit.unlock.deps_verified, "{audit:?}");
        assert!(!audit.unlock.resource_free, "{audit:?}");
        assert_eq!(
            audit.unlock.held,
            vec![("run".to_string(), "src/boards.rs".to_string())]
        );
        assert!(!audit.unlock.unlocked, "{audit:?}");

        // The running sibling: in flight, so it is the holder, and a
        // running node holds whether or not it is unlocked.
        let run = &graph.nodes[4];
        assert_eq!(run.state, "running");
        assert_eq!(run.score, 0, "{run:?}");

        // The unresolved prerequisite is named, not drawn as an edge:
        // its score is zero and its resource free, so only the deps
        // condition separates it from `plan`.
        let ghost = &graph.nodes[5];
        assert_eq!(ghost.state, "waiting");
        assert_eq!(ghost.score, 0, "{ghost:?}");
        assert!(ghost.unlock.score_zero, "{ghost:?}");
        assert!(ghost.unlock.resource_free, "{ghost:?}");
        assert!(!ghost.unlock.deps_verified, "{ghost:?}");
        assert_eq!(ghost.unlock.unresolved, vec!["missing".to_string()]);
        assert!(ghost.unlock.waiting.is_empty(), "{ghost:?}");
    }

    #[test]
    fn a_home_with_no_declared_graph_says_so_rather_than_rendering_an_empty_one() {
        let scratch = Scratch::new("delegation-no-graph");
        let sessions = scratch.path().join("sessions");
        std::fs::create_dir_all(&sessions).expect("the sessions directory");
        write_log(
            &sessions,
            "20261016T090000Z-root",
            "deepseek-chat",
            None,
            None,
            "a run that delegated nothing",
            None,
        );
        let snapshot = read_delegation(scratch.path(), "20261016T090000Z-root");
        assert!(snapshot.graph.is_none(), "{:?}", snapshot.graph);
        assert!(
            snapshot
                .refused
                .iter()
                .any(|reason| reason.contains("holds no declared graph")
                    && reason.contains("graph.json")),
            "{:?}",
            snapshot.refused
        );
    }

    #[test]
    fn a_malformed_graph_file_is_named_never_substituted() {
        let (scratch, root) = delegation_home("delegation-bad-graph");
        std::fs::write(crate::graph::path_in(scratch.path()), "{ not json").expect("the file");
        let snapshot = read_delegation(scratch.path(), &root);
        assert!(snapshot.graph.is_none(), "{:?}", snapshot.graph);
        assert!(
            snapshot
                .refused
                .iter()
                .any(|reason| reason.contains("graph.json")),
            "{:?}",
            snapshot.refused
        );
    }

    #[test]
    fn a_session_with_no_log_is_named_rather_than_drawn_as_an_empty_tree() {
        let scratch = Scratch::new("delegation-no-log");
        let snapshot = read_delegation(scratch.path(), "20261016T090000Z-absent");
        assert!(snapshot.live.is_none());
        assert!(
            snapshot
                .refused
                .iter()
                .any(|reason| reason.contains("has no readable session log")),
            "{:?}",
            snapshot.refused
        );
    }

    #[test]
    fn a_past_runs_tree_is_in_the_history_without_being_drawn_twice() {
        let (scratch, root) = delegation_home("delegation-history");
        // An earlier orchestrator run that delegated once: the history
        // root a tree is built for.
        let sessions = scratch.path().join("sessions");
        let past = "20261015T080000Z-past";
        write_log(
            &sessions,
            past,
            "deepseek-chat",
            None,
            None,
            "the run before this one",
            Some(("completed", 4)),
        );
        write_log(
            &sessions,
            "20261015T080100Z-past-child",
            "stepfun/step-3.7-flash",
            Some(crate::session::SUBAGENT_KIND),
            Some(past),
            "the earlier delegation",
            Some(("completed", 2)),
        );

        let snapshot = read_delegation(scratch.path(), &root);
        assert_eq!(snapshot.history.len(), 1, "{:?}", snapshot.history);
        assert_eq!(snapshot.history[0].id, past);
        assert_eq!(snapshot.history[0].status, "stopped");
        assert_eq!(snapshot.history[0].children.len(), 1);
        // The past run's child is not a history root of its own: one run
        // is never drawn twice.
        assert!(
            !snapshot
                .history
                .iter()
                .any(|tree| tree.id == "20261015T080100Z-past-child"),
            "{:?}",
            snapshot.history
        );
        // And neither is the live root.
        assert!(
            !snapshot.history.iter().any(|tree| tree.id == root),
            "{:?}",
            snapshot.history
        );
    }

    // --- the refresher -------------------------------------------------------

    #[test]
    fn the_refresher_lands_all_three_boards_from_its_own_thread() {
        let scratch = Scratch::new("refresher");
        let brain = scratch.path().join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        write_state(&brain, "loops_state", LOOPS_FIXTURE);
        // One git repo in the projects root, and one directory that is not
        // one: the tower scan reports both.
        scratch_repo(scratch.path(), "alpha");

        let sources = Sources {
            router: state(&scratch),
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
            router: state(&scratch),
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
        let scratch = Scratch::new("refresher-dedupe");
        let brain = scratch.path().join("brain");
        std::fs::create_dir_all(&brain).expect("the brain home");
        write_state(&brain, "gates_state", GATES_FIXTURE);
        scratch_repo(scratch.path(), "alpha");
        // A `git` that never answers: the opening read is still running
        // when the refreshes arrive, which is the case the dedupe is for.
        let bin = hung_git(scratch.path());
        let sources = Sources {
            router: state(&scratch),
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
