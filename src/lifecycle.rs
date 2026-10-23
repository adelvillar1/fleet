//! The tool lifecycle: measure, then disable what the evidence says is
//! idle — and only disable, never delete.
//!
//! Three seams meet here. The advisor measures (`tool_advisor`), the
//! thresholds and the disabled list live in config (`settings`), and
//! this module owns the two decisions between them: which state a
//! tool's measurements put it in, and which states a sweep may act on.
//! Both are pure, so the gate is a test rather than a policy experiment.
//!
//! The gate is deliberately a window, not a threshold: a tool must be
//! out of trial (enough declared runs AND enough days), out of the
//! protected set, and idle (a success rate at or below `idle_rate`) or
//! outright broken (enough calls, mostly errors). Every disable is a
//! config write the owner reverses with one verb; no file and no code
//! is ever removed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::settings::LifecycleThresholds;

/// The tools the sweep never disables and the manual verb guards with
/// `--force`. The working core plus the memory writers and delegation:
/// each is either load-bearing for every run or decisive when it is
/// used at all.
pub const PROTECTED_TOOLS: &[&str] = &[
    "bash",
    "read_file",
    "write_file",
    "edit",
    "grep",
    "glob",
    "todo",
    "remember",
    "supersede",
    "subagent",
];

/// When each native tool was introduced, backfilled from the git
/// history (read 2026-10-23). Backfill-only: the exact denominator is
/// the `Declarations` record, and this table exists so the first pass
/// can tell a new tool from an unused one before any record exists.
pub fn introduced(name: &str) -> &'static str {
    match name {
        "bash" | "read_file" | "write_file" | "list_dir" => "2026-10-12",
        "use_skill" | "remember" | "supersede" | "subagent" => "2026-10-13",
        "improve" => "2026-10-14",
        "convention" | "web" | "computer_use" => "2026-10-15",
        "deck" | "vision" => "2026-10-17",
        "plan" => "2026-10-19",
        "edit" | "grep" | "glob" | "todo" | "tool_search" => "2026-10-23",
        _ => "2026-10-23",
    }
}

/// Days between two `YYYY-MM-DD` stamps. `None` when either does not
/// parse; callers treat unknown dates as "not old enough", which is the
/// conservative direction for both trial and candidacy.
pub fn days_between(earlier: &str, later: &str) -> Option<u64> {
    let parse = |stamp: &str| -> Option<(i32, u32, u32)> {
        let mut parts = stamp.split('-');
        let year: i32 = parts.next()?.parse().ok()?;
        let month: u32 = parts.next()?.parse().ok()?;
        let day: u32 = parts.next()?.parse().ok()?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        Some((year, month, day))
    };
    // A days-since-epoch over the civil calendar; the arithmetic is
    // the standard Howard Hinnant shift, small enough to hold here.
    let days = |(year, month, day): (i32, u32, u32)| -> i64 {
        let (y, m) = (year as i64, month as i64);
        let shifted = if m <= 2 { y - 1 } else { y };
        let era = if shifted >= 0 { shifted } else { shifted - 399 } / 400;
        let year_of_era = shifted - era * 400;
        let month_shifted = if m > 2 { m } else { m + 12 };
        let day_of_year = (153 * month_shifted - 457) / 5 + day as i64 - 1;
        era * 146_097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year
    };
    let difference = days(parse(earlier)?) - days(parse(later)?);
    Some(difference.unsigned_abs())
}

/// One lifecycle candidate the sweep found, in whatever kind.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceCandidate {
    /// `tools`, `skills`, `loops`, or `workflows`.
    pub kind: &'static str,
    pub name: String,
    /// The evidence sentence the gate read.
    pub reason: String,
}

/// The skills the sweep never disables: a skill any role names is
/// protected, because the role's contract lists it.
pub fn protected_skills(home: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(home.join("roles")) else {
        return names;
    };
    for entry in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("skills:") {
                names.extend(
                    rest.split(',')
                        .map(|name| name.trim().trim_matches('"').to_string()),
                );
            }
        }
    }
    names.retain(|name| !name.is_empty());
    names
}

/// One skill's lifecycle metrics, as the scan computes them.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillMetrics {
    pub name: String,
    pub declared_runs: usize,
    pub uses: usize,
    pub first_seen: String,
}

/// The skills a sweep may disable: not retired, not role-named, and
/// idle over the window (zero `use_skill` calls in enough declared
/// runs and days).
pub fn skill_candidates(
    home: &Path,
    metrics: &[SkillMetrics],
    thresholds: &LifecycleThresholds,
) -> Vec<SurfaceCandidate> {
    let retired = crate::skills::retired(home);
    let protected = protected_skills(home);
    let today = today_stamp();
    let mut candidates = Vec::new();
    for entry in metrics {
        if retired.iter().any(|name| name == &entry.name)
            || protected.iter().any(|name| name == &entry.name)
        {
            continue;
        }
        let age = days_between(&entry.first_seen, &today).unwrap_or(0);
        if entry.declared_runs < thresholds.disable_runs || age < thresholds.disable_days {
            continue;
        }
        if entry.uses == 0 {
            candidates.push(SurfaceCandidate {
                kind: "skills",
                name: entry.name.clone(),
                reason: format!(
                    "idle \u{2014} no use in {} declared run(s)",
                    entry.declared_runs
                ),
            });
        }
    }
    candidates
}

/// The loops a sweep may disable: jobs with enough expected fires and
/// no success over the window. The maintenance jobs
/// (`lifecycle-sweep` and the corpus refreshes) are protected.
pub const PROTECTED_LOOPS: &[&str] = &["lifecycle-sweep"];

/// The loops a sweep may disable, from the cron store: at least
/// `min_fires` runs and zero successes.
pub fn loop_candidates(home: &Path, min_fires: u64) -> Result<Vec<SurfaceCandidate>, String> {
    let store = crate::cron::Store::open(home).map_err(|error| error.to_string())?;
    let mut candidates = Vec::new();
    for job in store.jobs() {
        if !job.enabled
            || PROTECTED_LOOPS.contains(&job.name.as_str())
            || job.name.starts_with("corpus-refresh-")
        {
            continue;
        }
        if job.runs < min_fires {
            continue;
        }
        if job.successes == 0 {
            candidates.push(SurfaceCandidate {
                kind: "loops",
                name: job.name.clone(),
                reason: format!("idle \u{2014} no success in {} run(s)", job.runs),
            });
        }
    }
    Ok(candidates)
}

/// The workflows a sweep may mask: the router's workload chains, judged
/// from the calibration stream's route rows. The default and the two
/// tool-dependent workloads never mask.
pub fn workflow_candidates(
    home: &Path,
    thresholds: &LifecycleThresholds,
    min_runs: u64,
) -> Vec<SurfaceCandidate> {
    let masked = crate::settings::workflows_disabled(home);
    let by_workload = workload_selections(home, thresholds.window);
    let mut candidates = Vec::new();
    for (workload, runs) in &by_workload {
        if masked.iter().any(|name| name == workload)
            || *workload == "vision"
            || *workload == "composed"
            || *runs < min_runs
        {
            continue;
        }
        candidates.push(SurfaceCandidate {
            kind: "workflows",
            name: workload.clone(),
            reason: format!(
                "idle \u{2014} routed {runs} time(s) in the window, never beyond the default's own walk"
            ),
        });
    }
    candidates
}

/// The route selections one window holds, from the calibration stream's
/// daily files: how often each workload was judged.
fn workload_selections(home: &Path, window: usize) -> BTreeMap<String, u64> {
    let mut by_workload = BTreeMap::new();
    let user_home = std::env::var("HOME").ok();
    let root = crate::calib::resolve_root(home, user_home.as_deref());
    let mut days: Vec<PathBuf> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return by_workload;
    };
    for entry in entries.flatten() {
        days.push(entry.path());
    }
    days.sort();
    for day in days.iter().rev().take(window.max(1)) {
        let Ok(text) = std::fs::read_to_string(day) else {
            continue;
        };
        for line in text.lines() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("op").and_then(|op| op.as_str()) == Some(crate::calib::OP_ROUTE) {
                if let Some(workload) = value.get("workload").and_then(|w| w.as_str()) {
                    *by_workload.entry(workload.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
    by_workload
}

/// Today's `YYYY-MM-DD`.
fn today_stamp() -> String {
    crate::boards::now_stamp().chars().take(10).collect()
}

/// One tool's lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Disabled by the owner or the sweep; one `enable` away.
    Disabled,
    /// In the protected set; flagged, never disabled.
    Protected,
    /// Too new to judge, whatever its calls.
    Trial,
    /// Above the idle rate over the window.
    Active,
    /// Idle past the window: a disable candidate.
    Candidate,
    /// Enough calls, mostly errors: broken, not unused.
    Broken,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::Disabled => "disabled",
            State::Protected => "protected",
            State::Trial => "trial",
            State::Active => "active",
            State::Candidate => "candidate",
            State::Broken => "broken",
        }
    }

    /// Whether the sweep may disable this state.
    pub fn sweepable(self) -> bool {
        matches!(self, State::Candidate | State::Broken)
    }
}

/// The measurements the gate reads, as the advisor computes them.
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub name: String,
    pub declared_runs: usize,
    pub calls: usize,
    pub successes: usize,
    pub errors: usize,
    /// `YYYY-MM-DD`, from the backfill table or the first record.
    pub first_declared: String,
    /// `YYYY-MM-DD`, the day the state is judged on.
    pub today: String,
}

/// The state one tool's measurements put it in. Pure: the fixture pins
/// every boundary.
pub fn state_of(metrics: &Metrics, thresholds: &LifecycleThresholds, disabled: &[String]) -> State {
    if disabled.iter().any(|name| name == &metrics.name) {
        return State::Disabled;
    }
    if PROTECTED_TOOLS.contains(&metrics.name.as_str()) {
        return State::Protected;
    }
    let age = days_between(&metrics.first_declared, &metrics.today).unwrap_or(0);
    let out_of_trial =
        metrics.declared_runs >= thresholds.trial_runs && age >= thresholds.trial_days;
    if !out_of_trial {
        return State::Trial;
    }
    // Broken is judged before idle: ten calls at an 80% error rate is a
    // tool that fails, not one nobody reaches for.
    if metrics.calls >= 10 && metrics.errors as f64 >= 0.8 * metrics.calls as f64 {
        return State::Broken;
    }
    let eligible =
        metrics.declared_runs >= thresholds.disable_runs && age >= thresholds.disable_days;
    if !eligible {
        return State::Active;
    }
    let total = metrics.calls + metrics.successes;
    let success_rate = if total == 0 {
        0.0
    } else {
        metrics.successes as f64 / total as f64
    };
    if success_rate <= thresholds.idle_rate {
        State::Candidate
    } else {
        State::Active
    }
}

/// One sweep's outcome, rendered by the CLI and recorded in the
/// calibration stream.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SweepOutcome {
    /// (tool, reason) pairs the sweep disabled.
    pub disabled: Vec<(String, String)>,
    /// (tool, reason) pairs the gate held back.
    pub skipped: Vec<(String, String)>,
}

/// The candidates a sweep would disable, capped. Pure over the states:
/// the caller supplies them in any order; the output is deterministic
/// (candidates by name, then broken by name), so two runs over the same
/// measurements disable the same tools.
pub fn sweep_plan(
    metrics: &[Metrics],
    thresholds: &LifecycleThresholds,
    disabled: &[String],
) -> SweepOutcome {
    let mut outcome = SweepOutcome::default();
    let mut candidates: Vec<&Metrics> = Vec::new();
    for entry in metrics {
        match state_of(entry, thresholds, disabled) {
            State::Candidate => candidates.push(entry),
            State::Broken => outcome.skipped.push((
                entry.name.clone(),
                "broken \u{2014} fix it, the gate does not auto-disable tools that fail \
                 with calls to answer for"
                    .to_string(),
            )),
            State::Protected => outcome
                .skipped
                .push((entry.name.clone(), "protected".to_string())),
            State::Trial => outcome.skipped.push((
                entry.name.clone(),
                format!(
                    "trial \u{2014} {} of {} runs",
                    entry.declared_runs, thresholds.trial_runs
                ),
            )),
            _ => {}
        }
    }
    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    for entry in candidates {
        if outcome.disabled.len() >= thresholds.max_disable_per_sweep {
            outcome.skipped.push((
                entry.name.clone(),
                "cap \u{2014} the next sweep takes it".to_string(),
            ));
            continue;
        }
        let total = entry.calls + entry.successes;
        outcome.disabled.push((
            entry.name.clone(),
            format!(
                "idle \u{2014} {} success in {} call over {} declared runs",
                entry.successes, entry.calls, entry.declared_runs
            ),
        ));
        let _ = total;
    }
    outcome
}

/// One kind's sweep result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KindSweep {
    pub applied: Vec<SurfaceCandidate>,
    pub skipped: Vec<(String, String)>,
}

/// The full sweep over the four surfaces, dry or applied. Tools ride
/// the caller's metrics (the advisor's), the other three read their own
/// stores. Each kind applies at most `max_disable_per_sweep`; the
/// write is the kind's own config entry, and every applied disable
/// lands in the calibration stream by the caller.
pub fn sweep(
    home: &Path,
    thresholds: &LifecycleThresholds,
    tool_metrics: &[Metrics],
    skill_metrics: &[SkillMetrics],
    apply: bool,
    calib: Option<&crate::calib::Stream>,
) -> Vec<(&'static str, KindSweep)> {
    let mut results: Vec<(&'static str, KindSweep)> = Vec::new();
    // Every applied disable lands in the calibration stream beside its
    // config write: the audit trail for "why is this disabled".
    let record = |calib: Option<&crate::calib::Stream>, kind: &str, name: &str, reason: &str| {
        if let Some(stream) = calib {
            crate::calib::record_lifecycle(stream, kind, name, "disable", reason);
        }
    };
    // Tools.
    let disabled = crate::settings::tools_disabled(home);
    let tool_plan = sweep_plan(tool_metrics, thresholds, &disabled);
    let mut tools = KindSweep::default();
    for (name, reason) in &tool_plan.disabled {
        if apply {
            if crate::settings::set_tool_disabled(home, name, true).is_ok() {
                record(calib, "tools", name, reason);
                tools.applied.push(SurfaceCandidate {
                    kind: "tools",
                    name: name.clone(),
                    reason: reason.clone(),
                });
            } else {
                tools
                    .skipped
                    .push((name.clone(), "the config write failed".to_string()));
            }
        } else {
            tools
                .skipped
                .push((name.clone(), format!("{reason} (dry-run)")));
        }
    }
    for (name, reason) in &tool_plan.skipped {
        tools.skipped.push((name.clone(), reason.clone()));
    }
    results.push(("tools", tools));
    // Skills.
    let mut skills = KindSweep::default();
    for candidate in skill_candidates(home, skill_metrics, thresholds)
        .into_iter()
        .take(thresholds.max_disable_per_sweep)
    {
        if apply {
            if crate::settings::retire_skill(home, &candidate.name).is_ok() {
                record(calib, "skills", &candidate.name, &candidate.reason);
                skills.applied.push(candidate);
            } else {
                skills
                    .skipped
                    .push((candidate.name, "the config write failed".to_string()));
            }
        } else {
            skills
                .skipped
                .push((candidate.name, format!("{} (dry-run)", candidate.reason)));
        }
    }
    results.push(("skills", skills));
    // Loops.
    let mut loops = KindSweep::default();
    if let Ok(candidates) = loop_candidates(home, 3) {
        for candidate in candidates
            .into_iter()
            .take(thresholds.max_disable_per_sweep)
        {
            if apply {
                let outcome = crate::cron::Store::open(home)
                    .map_err(|error| error.to_string())
                    .and_then(|mut store| {
                        store
                            .set_enabled(&candidate.name, false)
                            .map_err(|error| error.to_string())
                    });
                match outcome {
                    Ok(_) => {
                        record(calib, "loops", &candidate.name, &candidate.reason);
                        loops.applied.push(candidate);
                    }
                    Err(error) => loops
                        .skipped
                        .push((candidate.name, format!("the write failed: {error}"))),
                }
            } else {
                loops
                    .skipped
                    .push((candidate.name, format!("{} (dry-run)", candidate.reason)));
            }
        }
    }
    results.push(("loops", loops));
    // Workflows.
    let mut workflows = KindSweep::default();
    for candidate in workflow_candidates(home, thresholds, 10)
        .into_iter()
        .take(thresholds.max_disable_per_sweep)
    {
        if apply {
            if crate::settings::set_workflow_disabled(home, &candidate.name, true).is_ok() {
                record(calib, "workflows", &candidate.name, &candidate.reason);
                workflows.applied.push(candidate);
            } else {
                workflows
                    .skipped
                    .push((candidate.name, "the config write failed".to_string()));
            }
        } else {
            workflows
                .skipped
                .push((candidate.name, format!("{} (dry-run)", candidate.reason)));
        }
    }
    results.push(("workflows", workflows));
    results
}

/// The promotion pass: repeated work patterns become suggestions for a
/// loop, skill, workflow, or tool. The evidence is the archive's own
/// session tasks — clusters of near-identical first turns.
#[derive(Debug, Clone, PartialEq)]
pub struct Promotion {
    pub kind: &'static str,
    /// The pattern, stated as the cluster's shared task.
    pub pattern: String,
    pub sessions: usize,
    /// The exact action the owner runs; nothing is auto-created.
    pub action: String,
}

/// Cluster the archive's first turns by token Jaccard and propose the
/// kind each cluster's evidence supports. Deterministic: the clusters
/// sort by size, and a cluster below the minimum is never proposed.
pub fn promote(
    home: &Path,
    thresholds: &LifecycleThresholds,
    min_sessions: usize,
    similarity: f64,
    cap: usize,
) -> Vec<Promotion> {
    let tasks = session_tasks(home, thresholds.window);
    if tasks.len() < min_sessions {
        return Vec::new();
    }
    // Greedy clustering in first-seen order: a task joins the first
    // cluster whose seed it is similar enough to.
    let mut clusters: Vec<(String, usize, Vec<String>)> = Vec::new();
    for (session, task) in &tasks {
        let tokens = token_set(task);
        let mut placed = false;
        for cluster in clusters.iter_mut() {
            if similarity_of(&token_set(&cluster.0), &tokens) >= similarity {
                cluster.1 += 1;
                cluster.2.push(session.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            clusters.push((task.clone(), 1, vec![session.clone()]));
        }
    }
    clusters.sort_by(|left, right| right.1.cmp(&left.1));
    let mut promotions = Vec::new();
    for (task, count, members) in clusters {
        if promotions.len() >= cap || count < min_sessions {
            continue;
        }
        // The kind the evidence supports: a shared multi-call sequence
        // proposes a skill; a same-hour recurrence proposes a loop;
        // otherwise the pattern is a workflow-shaped repetition and the
        // suggestion names the config it would need. One session's
        // repeated tool calls do not make a pattern.
        let sequence = shared_sequence(home, &members);
        let kind = if sequence.len() >= 2 {
            "skill"
        } else {
            "workflow"
        };
        let action = match kind {
            "skill" => "file a `skill` proposal through `fleet improve` with this pattern as the body".to_string(),
            _ => "declare the pattern as a routing workload or plan-graph template; the config change is named in the proposal".to_string(),
        };
        promotions.push(Promotion {
            kind,
            pattern: task,
            sessions: count,
            action,
        });
    }
    promotions
}

/// The first user turn of every session in the window.
fn session_tasks(home: &Path, window: usize) -> Vec<(String, String)> {
    let mut tasks = Vec::new();
    let Ok(summaries) = crate::session::list(home) else {
        return tasks;
    };
    for summary in summaries.iter().rev().take(window.max(1)) {
        let Ok(records) = crate::session::replay(&summary.path) else {
            continue;
        };
        for record in &records {
            if let crate::session::Record::User { content } = record {
                if !content.trim().is_empty() {
                    tasks.push((summary.id.clone(), content.clone()));
                    break;
                }
            }
        }
    }
    tasks
}

/// The longest tool sequence every named session shares, capped.
fn shared_sequence(home: &Path, sessions: &[String]) -> Vec<String> {
    let mut sequences: Vec<Vec<String>> = Vec::new();
    let Ok(summaries) = crate::session::list(home) else {
        return Vec::new();
    };
    for summary in &summaries {
        if !sessions.iter().any(|id| id == &summary.id) {
            continue;
        }
        let Ok(records) = crate::session::replay(&summary.path) else {
            continue;
        };
        let mut sequence = Vec::new();
        for record in &records {
            if let crate::session::Record::Assistant { tool_calls, .. } = record {
                for call in tool_calls {
                    sequence.push(call.function.name.clone());
                }
            }
        }
        sequences.push(sequence);
    }
    if sequences.len() < 2 {
        return Vec::new();
    }
    let _ = &sequences;
    // The common prefix of every sequence: the shared opening pattern.
    let first = sequences[0].clone();
    let mut prefix = Vec::new();
    for (index, call) in first.iter().enumerate() {
        if sequences
            .iter()
            .all(|sequence| sequence.get(index) == Some(call))
        {
            prefix.push(call.clone());
        } else {
            break;
        }
    }
    prefix.into_iter().take(5).collect()
}

/// Token Jaccard over lowercased words of four characters or more.
fn similarity_of(
    left: &std::collections::BTreeSet<String>,
    right: &std::collections::BTreeSet<String>,
) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let shared = left.intersection(right).count();
    let union = left.union(right).count();
    shared as f64 / union as f64
}

fn token_set(text: &str) -> std::collections::BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() >= 4)
        .map(|word| word.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> LifecycleThresholds {
        LifecycleThresholds::default()
    }

    fn metrics(
        name: &str,
        declared: usize,
        calls: usize,
        successes: usize,
        errors: usize,
    ) -> Metrics {
        Metrics {
            name: name.to_string(),
            declared_runs: declared,
            calls,
            successes,
            errors,
            first_declared: "2026-10-01".to_string(),
            today: "2026-10-23".to_string(),
        }
    }

    #[test]
    fn a_disabled_tool_is_disabled_whatever_its_numbers() {
        let state = state_of(
            &metrics("deck", 100, 50, 50, 0),
            &thresholds(),
            &["deck".to_string()],
        );
        assert_eq!(state, State::Disabled);
    }

    #[test]
    fn a_protected_tool_is_flagged_never_judged() {
        let state = state_of(&metrics("grep", 100, 0, 0, 0), &thresholds(), &[]);
        assert_eq!(state, State::Protected);
    }

    #[test]
    fn trial_is_a_window_of_runs_and_days() {
        // Enough runs but not enough days.
        let young = Metrics {
            first_declared: "2026-10-20".to_string(),
            ..metrics("deck", 20, 0, 0, 0)
        };
        assert_eq!(state_of(&young, &thresholds(), &[]), State::Trial);
        // Enough days but not enough runs.
        let sparse = metrics("deck", 5, 0, 0, 0);
        assert_eq!(state_of(&sparse, &thresholds(), &[]), State::Trial);
        // Both boundaries met: a zero-use tool is a candidate.
        assert_eq!(
            state_of(&metrics("deck", 10, 0, 0, 0), &thresholds(), &[]),
            State::Candidate
        );
    }

    #[test]
    fn one_success_keeps_a_tool_active() {
        assert_eq!(
            state_of(&metrics("deck", 30, 3, 1, 2), &thresholds(), &[]),
            State::Active
        );
    }

    #[test]
    fn a_broken_tool_is_broken_not_candidate() {
        let state = state_of(&metrics("deck", 30, 10, 0, 9), &thresholds(), &[]);
        assert_eq!(state, State::Broken);
        // Seven errors with one success misses the 80% line and clears
        // the idle rate: active, flagged by neither rule.
        assert_eq!(
            state_of(&metrics("deck", 30, 10, 1, 7), &thresholds(), &[]),
            State::Active
        );
        // Ten failing calls with no success are idle by the rate, so
        // they are a candidate even under the 80% line.
        assert_eq!(
            state_of(&metrics("deck", 30, 10, 0, 7), &thresholds(), &[]),
            State::Candidate
        );
    }

    #[test]
    fn the_sweep_caps_and_names_what_it_holds_back() {
        let list: Vec<Metrics> = ["b", "a", "c", "d"]
            .iter()
            .map(|name| metrics(name, 30, 0, 0, 0))
            .collect();
        let outcome = sweep_plan(&list, &thresholds(), &[]);
        assert_eq!(outcome.disabled.len(), 3, "{outcome:?}");
        let names: Vec<&str> = outcome
            .disabled
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert!(outcome.skipped.iter().any(|(name, _)| name == "d"));
    }

    #[test]
    fn a_broken_tool_is_skipped_not_disabled() {
        let list = vec![metrics("deck", 30, 10, 0, 9)];
        let outcome = sweep_plan(&list, &thresholds(), &[]);
        assert!(outcome.disabled.is_empty());
        assert!(outcome.skipped[0].1.contains("broken"));
    }

    #[test]
    fn days_are_counted_across_months_and_years() {
        assert_eq!(days_between("2026-10-01", "2026-10-23"), Some(22));
        assert_eq!(days_between("2026-09-30", "2026-10-01"), Some(1));
        assert_eq!(days_between("2025-12-31", "2026-01-01"), Some(1));
        assert_eq!(days_between("nonsense", "2026-10-23"), None);
    }

    #[test]
    fn every_native_tool_has_an_introduction_date() {
        for tool in crate::tools::known_tool_names() {
            assert!(
                introduced(tool).len() == 10,
                "{tool} has no introduction date"
            );
        }
    }
}
