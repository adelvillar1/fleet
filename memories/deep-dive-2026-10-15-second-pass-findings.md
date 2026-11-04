---
name: deep-dive-2026-10-15-second-pass-findings
description: "The 2026-10-15 second deep dive found ~20 defects outside the first dive's scope; the fix-first ones KI-208..213 were FIXED 2026-10-16 (plan complete, gate four-legs green, 1013 tests); the rest wait in the plan's out-of-scope for a later plan"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

On 2026-10-15 a second deep dive verified the first dive's findings plan complete (all six phases landed by HEAD `a2555fe`; the other session's own review round `783b524` caught its two fresh-phase slips — TUI web-leg double-render and the unwired session-recall `record_hit`). The miss is in code the first dive never read line-deep.

Top five severe, each personally verified, now assigned **KI-208 through KI-213** in the filed plan:

1. KI-208 `route.rs:1690` — committed-turn failure record indexes the roster by walk position (`self.resolved[start]` instead of `self.resolved[self.order[start]]`), so failures are recorded against the wrong provider and feed `calib::demoted_providers` (demotion on a streak of 3 by name).
2. KI-209 `approvals.rs:182/248-253` — yes-guarded never strips the `subagent → ` prefix in `bash_command`, so the destructive class is bypassed for forwarded child bash calls under unattended guarded runs.
3. KI-210 `approvals.rs:602-608` + `subagents.rs:1851` — `asks_regardless` strips the prefix once, so a depth-2 grandchild's `computer_use` arrives double-prefixed and loses always-ask protection under `yes`/`yes-guarded`.
4. KI-211 `tools.rs:816-871` — the bash timeout loop only runs while `sh` is alive and both paths `join()` drain threads blocked in `read_to_end`; a background descendant holding the inherited pipes (`nohup … &`) hangs the tool past the timeout in both paths.
5. KI-212 `memory.rs:3793` — `write_summary` discards `write_document`'s error, then consolidation stamps `superseded-by` pointing at a possibly nonexistent file and lands audit rows claiming a fold that didn't land.
6. KI-213 `session.rs:241-256` — `replay` fails the whole log on any unparseable line; one torn trailing JSONL line (SIGKILL mid 64 KiB write) bricks resume and listing.

**Disposition:** the owner asked for a fix plan (`docs/plans/2026-10-15-dogfooding-viability-fixes.md`, `686ed45`), then said "implement the plan as designed" — **all eight phases landed 2026-10-16, plan status complete**. Commits `c7741c9`..`2736a91`; KI-208..213 all closed-fixed with evidence, KI-007/097/104 closed (toolchain repaired). Notable implementation facts: bash fix is a std-only bounded drain with 2s `DRAIN_GRACE` (process-group kill rejected — needs libc/unsafe); the approval gate's `unlabel` strips every `subagent → ` repetition and unwraps the labelled wrapper; the forget half of consolidation follows the fold (a failed summary must not archive unstamped facts); and the KI-068 zero-choice premise was corrected — the streamed client always assembles one choice, so a `[DONE]`-only stream is a completed empty answer, not a zero-choice failure. Gate after: **four legs green, 1013 passed / 2 ignored**.

Next tier (NOT in the plan, recorded in its out-of-scope for later triage): pin silently dropped when sys1 judges an unmapped workload (`fallback()` has no pin filter); `gates.default_workload` unvalidated so a typo reaches the full-roster walk that serves the DeepSeek PAYG reserve; improve.rs revert engine fail-silent in four places; consolidator-vs-run TOCTOU resurrects archived facts; role overlay never logged (hard-rule-5); calib `pending` counter wrap race; nomenclature trio; TUI approval clobbers overlays; cron catch-up mislabels on-time fires; ADMISSIBLE_TOOLS comment contract mismatch. Assumed gaps needing owner ruling: settings-writer cross-process lost updates; TuiApprover ignores the `approvals` config word.

Gate at time of report: 991 passed / 0 failed, exit 0. See [[fleet-gate-exit-code-trap]] and [[fleet-dogfooding-viability-verdict]].
