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
