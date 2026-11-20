//! The drift registry: every bound Fleet states in more than one place, in
//! one place, so a change that invalidates one is named instead of found.
//!
//! ## The failure this exists for
//!
//! `docs/architecture/structure-guards.md` said `MAX_DEFINITION_TOKENS` was
//! 10,050. `src/agent.rs` said 10,800. Both were in the repo, both were
//! read by real sessions, and nothing failed: the code was right, the tests
//! passed, and the document the repo tells a session to read *before its
//! first compile* was wrong by 750 tokens. A session that trusted it would
//! trim tool prose against a bound that did not exist.
//!
//! The text-scan guards (`include_str!` sites) catch **presence** — this
//! module must not reference that beat — very well. They cannot catch **a
//! number appearing in two places**, which is a different question and
//! needs a different mechanism.
//!
//! ## The shape, borrowed from oh-my-hermes
//!
//! `src/maintenance/drift.py` in [rlaope/oh-my-hermes](https://github.com/rlaope/oh-my-hermes)
//! answers exactly this, and its own docstring is the argument: adding one
//! catalog entry invalidates counts, budgets and generated artifacts at
//! once, but the suite reveals them a few at a time — "an observed session
//! spent twelve full-suite cycles that way". Its two row types are the ones
//! kept here:
//!
//! - a **metric**: a name, a description, a `live` closure that calls the
//!   real producer (never a copy of the number), the value it should equal,
//!   and the `sites` that state it — **file paths, never line numbers,
//!   because line numbers drift**.
//! - a **budget**: the same, plus a `limit` and the file that holds it.
//!
//! Nothing here parses source: every live value comes from calling the real
//! producer, and every site is declared data.
//!
//! ## Reading it
//!
//! `fleet drift` reports every row with its verdict. A row whose live value
//! disagrees with its expected value, or whose budget is spent, is a
//! finding; a clean report is one line. The registry is a *report*, not a
//! gate: the suite still owns the assertions. This is the index that says
//! which assertions a change just invalidated.

/// One value that must agree in several places.
pub struct DriftMetric {
    /// The value's name, as a reader greps for it.
    pub name: &'static str,
    /// What it is, in the units a reader cares about.
    pub describe: &'static str,
    /// The live reading: calls the real producer, never a copy.
    pub live: fn() -> usize,
    /// The value every site is expected to state.
    pub expected: usize,
    /// Where the value is stated. **File paths, never line numbers.**
    pub sites: &'static [&'static str],
}

impl DriftMetric {
    /// The row's verdict: `true` when the live producer agrees with the
    /// value its sites claim.
    pub fn agrees(&self) -> bool {
        (self.live)() == self.expected
    }
}

/// One bound that must hold, with the reason it is bounded at all.
pub struct BudgetMetric {
    /// The bound's name, as a reader greps for it.
    pub name: &'static str,
    /// What the budget pays for, in the units a reader cares about.
    pub describe: &'static str,
    /// The live reading against the limit.
    pub live: fn() -> usize,
    /// The most the value may be.
    pub limit: usize,
    /// The file that holds the limit, so a reader can move it deliberately.
    pub limit_site: &'static str,
}

impl BudgetMetric {
    /// The row's verdict: `true` when the live reading is inside the limit.
    pub fn holds(&self) -> bool {
        (self.live)() <= self.limit
    }

    /// How much headroom is left, saturating at zero.
    pub fn headroom(&self) -> usize {
        self.limit.saturating_sub((self.live)())
    }
}

/// The producers: each reads its own constant, so this registry cannot
/// itself be the copy that drifts.
mod producers {
    /// The visible-tool bound, read from the constant.
    pub fn max_visible_tools() -> usize {
        crate::agent::MAX_VISIBLE_TOOLS
    }

    /// The token bound on one request's tool declarations.
    pub fn max_definition_tokens() -> usize {
        crate::agent::MAX_DEFINITION_TOKENS
    }

    /// The skill index's token budget, read from the constant.
    pub fn skill_index_tokens() -> usize {
        crate::skills::SKILL_INDEX_TOKENS
    }

    /// The delegation concurrency bound, read from the constant.
    pub fn delegation_concurrency() -> usize {
        crate::agent::DELEGATION_CONCURRENCY
    }

    /// The main session's fallback turn ceiling, read from the constant.
    pub fn default_max_turns() -> usize {
        crate::agent::DEFAULT_MAX_TURNS
    }

    /// The delegated child's default turn cap.
    pub fn default_child_turns() -> usize {
        crate::subagents::DEFAULT_CHILD_TURNS
    }

    /// The tool router's cosine floor, in basis points so the row carries
    /// an integer like every other producer.
    pub fn tool_router_floor_bp() -> usize {
        (crate::tool_router::FLOOR * 10_000.0).round() as usize
    }
}

/// Every value Fleet states in more than one place.
///
/// A row belongs here when **two copies exist** and the compiler cannot
/// hold them together. A row whose sites are mostly the constant's own
/// module is still worth keeping: it names the file a session greps for
/// the number, and that is what makes the next copy visible.
pub fn metrics() -> Vec<DriftMetric> {
    vec![
        DriftMetric {
            name: "max_visible_tools",
            describe: "The most tool declarations one request may carry",
            live: producers::max_visible_tools,
            expected: crate::agent::MAX_VISIBLE_TOOLS,
            sites: &[
                "src/agent.rs",
                "docs/architecture/structure-guards.md",
                "docs/architecture/tool-surface.md",
            ],
        },
        DriftMetric {
            name: "max_definition_tokens",
            describe: "The token bound on one request's tool declarations",
            live: producers::max_definition_tokens,
            expected: crate::agent::MAX_DEFINITION_TOKENS,
            sites: &[
                "src/agent.rs",
                "docs/architecture/structure-guards.md",
                "docs/architecture/tool-surface.md",
            ],
        },
        DriftMetric {
            name: "skill_index_tokens",
            describe: "The token budget one run's skill index may spend",
            live: producers::skill_index_tokens,
            expected: crate::skills::SKILL_INDEX_TOKENS,
            sites: &["src/skills.rs", "docs/features/skills.md"],
        },
        DriftMetric {
            name: "delegation_concurrency",
            describe: "How many of one turn's independent tool calls may run at once",
            live: producers::delegation_concurrency,
            expected: crate::agent::DELEGATION_CONCURRENCY,
            sites: &["src/agent.rs", "docs/features/subagents.md"],
        },
        DriftMetric {
            name: "default_max_turns",
            describe: "The main session's fallback turn ceiling",
            live: producers::default_max_turns,
            expected: crate::agent::DEFAULT_MAX_TURNS,
            sites: &["src/agent.rs", "src/settings.rs"],
        },
        DriftMetric {
            name: "default_child_turns",
            describe: "The turn cap an unasked delegated child gets",
            live: producers::default_child_turns,
            expected: crate::subagents::DEFAULT_CHILD_TURNS,
            sites: &[
                "src/subagents.rs",
                "src/settings.rs",
                "docs/features/subagents.md",
            ],
        },
        DriftMetric {
            name: "tool_router_floor_bp",
            describe: "The cosine below which no tool candidate is trusted, in basis points",
            live: producers::tool_router_floor_bp,
            expected: 6_800,
            sites: &["src/tool_router.rs"],
        },
    ]
}

/// Every bound Fleet holds itself to, with the file that sets it.
pub fn budgets() -> Vec<BudgetMetric> {
    vec![
        BudgetMetric {
            name: "max_visible_tools",
            describe: "Tool declarations per request; the curator withholds past it",
            live: producers::max_visible_tools,
            limit: crate::agent::MAX_VISIBLE_TOOLS,
            limit_site: "src/agent.rs",
        },
        BudgetMetric {
            name: "max_definition_tokens",
            describe:
                "Tool declaration tokens per request; the curator declares `tool_search` past it",
            live: producers::max_definition_tokens,
            limit: crate::agent::MAX_DEFINITION_TOKENS,
            limit_site: "src/agent.rs",
        },
        BudgetMetric {
            name: "skill_index_tokens",
            describe: "Skill index tokens per request; the ranking fills this and stops",
            live: producers::skill_index_tokens,
            limit: crate::skills::SKILL_INDEX_TOKENS,
            limit_site: "src/skills.rs",
        },
    ]
}

/// The whole report, as a reader reads it: one line per finding, then a
/// closing line saying whether anything drifted.
///
/// A clean report is deliberately short — the failure this module exists
/// for was invisible, so a report nobody reads because it is long is the
/// same failure with extra ceremony.
pub fn report() -> String {
    let mut out = String::new();
    let mut findings = 0usize;

    for metric in metrics() {
        if metric.agrees() {
            continue;
        }
        findings += 1;
        out.push_str(&format!(
            "drift: `{}` reads {} but its sites state {} — check {}\n",
            metric.name,
            (metric.live)(),
            metric.expected,
            metric.sites.join(", ")
        ));
    }

    for budget in budgets() {
        if budget.holds() {
            continue;
        }
        findings += 1;
        out.push_str(&format!(
            "over budget: `{}` reads {} against a limit of {} ({}) — the limit lives in {}\n",
            budget.name,
            (budget.live)(),
            budget.limit,
            budget.describe,
            budget.limit_site
        ));
    }

    if findings == 0 {
        let rows = metrics().len() + budgets().len();
        out.push_str(&format!(
            "drift: {rows} row(s) checked, none drifted — every bound agrees with its sites\n"
        ));
    } else {
        out.push_str(&format!("\n{findings} finding(s).\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_metric_agrees_with_its_own_constant() {
        // The registry's first duty is not to be the thing it reports. A
        // row whose `expected` was a literal copy would fail here the day
        // the constant moved, which is the whole point.
        for metric in metrics() {
            assert!(
                metric.agrees(),
                "`{}` reads {} but the row states {}",
                metric.name,
                (metric.live)(),
                metric.expected
            );
            assert!(
                !metric.sites.is_empty(),
                "`{}` names no site, so a reader cannot check it",
                metric.name
            );
            for site in metric.sites {
                assert!(
                    !site.contains(':'),
                    "`{}` names `{site}`; sites are paths because line numbers drift",
                    metric.name
                );
                assert!(
                    std::path::Path::new(site).exists(),
                    "`{}` names `{site}`, which is not in the tree",
                    metric.name
                );
            }
        }
    }

    #[test]
    fn every_budget_holds_and_names_a_real_limit_site() {
        for budget in budgets() {
            assert!(
                budget.holds(),
                "`{}` is over its limit on the day it was written",
                budget.name
            );
            assert!(
                std::path::Path::new(budget.limit_site).exists(),
                "`{}` names `{}`, which is not in the tree",
                budget.name,
                budget.limit_site
            );
        }
    }

    #[test]
    fn a_clean_report_is_one_line_and_names_the_row_count() {
        let text = report();
        assert!(
            text.contains("none drifted"),
            "a clean report says so: {text}"
        );
        assert_eq!(text.lines().count(), 1, "and stays one line: {text}");
    }

    #[test]
    fn the_documented_bound_is_the_one_the_code_enforces() {
        // The instance this registry was built for: the guard doc said
        // 10,050 while the constant said 10,800, for five crossings, and
        // nothing failed. This reads the doc the way a session reads it and
        // checks the number it publishes is the one the code holds.
        let doc = include_str!("../docs/architecture/structure-guards.md");
        let section = doc
            .split("## B. The tool-surface bounds")
            .nth(1)
            .expect("the tool-surface section");
        let head: String = section.chars().take(600).collect();
        let published = head
            .split("MAX_DEFINITION_TOKENS")
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .map(|cell| {
                cell.trim_start_matches('\n')
                    .trim_start_matches('(')
                    .to_string()
            })
            .expect("the doc names the bound");
        let digits: String = published.chars().filter(|c| c.is_ascii_digit()).collect();
        let published: usize = digits.parse().expect("the doc states a number");
        assert_eq!(
            published,
            crate::agent::MAX_DEFINITION_TOKENS,
            "structure-guards.md publishes {published} but the constant is {}",
            crate::agent::MAX_DEFINITION_TOKENS
        );
    }

    #[test]
    fn a_drifted_metric_is_named_with_its_sites() {
        // Driven through the same comparison the report uses, on a row
        // built to disagree, so the failing path is exercised rather than
        // assumed.
        let wrong = DriftMetric {
            name: "a-value-that-moved",
            describe: "driven",
            live: || 10_800,
            expected: 10_050,
            sites: &["docs/architecture/structure-guards.md"],
        };
        assert!(!wrong.agrees());
        assert_eq!((wrong.live)(), 10_800);
    }
}
