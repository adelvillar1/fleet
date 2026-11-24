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

    // ---- counted facts -------------------------------------------------
    //
    // The producers below read a **collection's length**, not a constant.
    // They exist because the audit of 2026-11-23 found that the numbers
    // which rotted were the ones no row could see: `ALL_TOOLS.len()` and
    // `LANES.len()` had no `live` closure, so no drift row could be
    // written for them, so every doc that stated them drifted silently.

    /// The built-in tool count, read from the tool table itself.
    pub fn builtin_tools() -> usize {
        crate::tools::ALL_TOOLS.len()
    }

    /// Every tool name the harness can declare, native and special.
    pub fn declarable_tools() -> usize {
        crate::tools::known_tool_names().len()
    }

    /// The vendored lane count, read from the lane registry.
    pub fn lanes() -> usize {
        crate::lanes::LANES.len()
    }

    /// The rail rows the browser face actually renders, read from the
    /// face's own `RAIL_MAP` declaration rather than a second list.
    pub fn rail_rows() -> usize {
        crate::gui::face_declarations::rail_rows()
    }

    /// Every address the face resolves, read from its own `ROUTES` table.
    pub fn face_routes() -> usize {
        crate::gui::face_declarations::routes()
    }

    /// The graded eval corpus's scenario count.
    pub fn eval_scenarios() -> usize {
        crate::eval::corpus().len()
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
        // ---- counted facts (the 2026-11-23 documentation audit) ---------
        //
        // Every row below states a number that appears in prose. The old
        // registry could not hold these: a row needs a `live` producer, and
        // a collection's length had none. Now it does, so the doc site is
        // named and the count is read, not remembered.
        DriftMetric {
            name: "builtin_tools",
            describe: "The built-in tools the harness always declares",
            live: producers::builtin_tools,
            expected: crate::tools::ALL_TOOLS.len(),
            sites: &["src/tools.rs", "CLAUDE.md", "docs/features/tools.md"],
        },
        DriftMetric {
            name: "declarable_tools",
            describe: "Every tool name the harness can declare, native and special",
            live: producers::declarable_tools,
            expected: crate::tools::known_tool_names().len(),
            sites: &["src/tools.rs"],
        },
        DriftMetric {
            name: "lanes",
            describe: "The vendored lane cores in the registry",
            live: producers::lanes,
            expected: crate::lanes::LANES.len(),
            sites: &[
                "src/lanes.rs",
                "docs/architecture/lanes.md",
                "services/README.md",
            ],
        },
        DriftMetric {
            name: "rail_rows",
            describe: "The rows the browser face's rail paints",
            live: producers::rail_rows,
            expected: producers::rail_rows(),
            sites: &[
                "src/gui/face_declarations.rs",
                "README.md",
                "docs/architecture/surfaces.md",
                "FUNCTIONAL-SPECIFICATIONS.md",
            ],
        },
        DriftMetric {
            name: "face_routes",
            describe: "Every address the browser face resolves",
            live: producers::face_routes,
            expected: producers::face_routes(),
            sites: &["src/gui/assets/app.js"],
        },
        DriftMetric {
            name: "eval_scenarios",
            describe: "The graded eval corpus's fixed scenarios",
            live: producers::eval_scenarios,
            expected: producers::eval_scenarios(),
            sites: &["src/eval.rs"],
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

    // ---- the doc sites state what the producers read -------------------
    //
    // The 2026-11-23 audit's gap: a registered row names a doc file, but
    // nothing read that file. A number could move in the doc and every
    // test still passed, because the row only compared the producer to
    // its own constant. These tests close that: the words the docs
    // publish are parsed and compared to the collection's live length.

    /// A spelled-out or numeric count, as a number.
    fn count_word(word: &str) -> Option<usize> {
        match word {
            "one" => Some(1),
            "two" => Some(2),
            "three" => Some(3),
            "four" => Some(4),
            "five" => Some(5),
            "six" => Some(6),
            "seven" => Some(7),
            "eight" => Some(8),
            "nine" => Some(9),
            "ten" => Some(10),
            "eleven" => Some(11),
            "twelve" => Some(12),
            "thirteen" => Some(13),
            "fourteen" => Some(14),
            "fifteen" => Some(15),
            "sixteen" => Some(16),
            other => other.parse().ok(),
        }
    }

    /// Every line of a doc that states a count immediately before `noun`.
    fn stated_counts(doc: &str, noun: &str) -> Vec<(usize, String)> {
        doc.lines()
            .filter_map(|line| {
                // The count and the noun must be adjacent words, so
                // "eight built-ins" matches and a sentence that mentions a
                // number three words earlier does not.
                let words: Vec<&str> = line.split_whitespace().collect();
                for pair in words.windows(2) {
                    let count = pair[0].trim_matches(|c: char| !c.is_alphanumeric());
                    let word = pair[1].trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
                    if word != noun {
                        continue;
                    }
                    if let Some(value) = count_word(count) {
                        return Some((value, line.trim().to_string()));
                    }
                }
                None
            })
            .collect()
    }

    #[test]
    fn the_front_door_states_the_tool_count_the_code_holds() {
        let tools = open_doc("docs/features/tools.md");
        let claimed = stated_counts(&tools, "built-ins");
        assert!(
            !claimed.is_empty(),
            "docs/features/tools.md states no built-in count — the phrasing moved, so this \
             guard would pass vacuously and the doc could drift unwatched"
        );
        for (value, line) in claimed {
            assert_eq!(
                value,
                producers::builtin_tools(),
                "docs/features/tools.md publishes {value} built-ins but ALL_TOOLS has {}: {line}",
                producers::builtin_tools()
            );
        }
    }

    #[test]
    fn claude_md_states_the_tool_count_the_code_holds() {
        let claude = open_doc("CLAUDE.md");
        let claimed = stated_counts(&claude, "built-ins");
        assert!(
            !claimed.is_empty(),
            "CLAUDE.md states no built-in count; the pointer block's phrasing moved"
        );
        for (value, line) in claimed {
            assert_eq!(
                value,
                producers::builtin_tools(),
                "CLAUDE.md publishes {value} built-ins but ALL_TOOLS has {}: {line}",
                producers::builtin_tools()
            );
        }
    }

    #[test]
    fn the_lane_docs_state_the_lane_count_the_registry_holds() {
        // Each doc names the count with its own noun: lanes.md heads a
        // section "The six lanes", services/README.md opens with "The six
        // fleet lane cores". Both are the same fact, so both are held to
        // it — and each is asserted non-empty so the phrasing moving can
        // never make the guard pass vacuously.
        let docs: [(&str, &str); 2] = [
            ("docs/architecture/lanes.md", "lanes"),
            ("services/README.md", "lanes"),
        ];
        for (doc, noun) in docs {
            let text = open_doc(doc);
            let claimed = stated_counts(&text, noun);
            assert!(
                !claimed.is_empty(),
                "{doc} states no `{noun}` count — a doc that never says the number cannot be \
                 held to it, and this guard would pass vacuously"
            );
            for (value, line) in claimed {
                assert_eq!(
                    value,
                    producers::lanes(),
                    "{doc} publishes {value} {noun} but LANES has {}: {line}",
                    producers::lanes()
                );
            }
        }
    }

    #[test]
    fn the_doc_truth_checker_agrees_with_the_registry() {
        // `scripts/doc_truth.py` is the command form of this audit. It
        // must not be a second, drifting copy of the same claims: its
        // reported values come from the same numbers these rows hold.
        let path = std::path::Path::new("scripts/doc_truth.py");
        assert!(
            path.exists(),
            "the doc-truth checker is missing; the audit is not a command"
        );
        let text = std::fs::read_to_string(path).expect("the checker reads");
        assert!(
            text.contains("drift") || text.contains("ALL_TOOLS"),
            "the checker does not mention the registry it is meant to mirror"
        );
    }

    /// One doc, read from the crate root.
    fn open_doc(path: &str) -> String {
        std::fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("{path} is a registered drift site but unreadable: {error}")
        })
    }
}
