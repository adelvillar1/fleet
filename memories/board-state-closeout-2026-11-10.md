---
name: board-state-closeout-2026-11-10
description: "project 2026-11-10 — the board's exact close-out state after the triage + the cut-verify/relaunch pass: 0.6.26 confirmed live (feed names it, face relaunched, folder granted); seat-decision COMPLETED (C3 verified), KI-515 CLOSED, KI-516 refined; leaving 1 active plan (beta-shakeout B5 = the owner's pass-three, now UNBLOCKED), resource-contract in progress (C10–C12), 4 open KIs (459/476/494/516) with exact fix targets; fixture tooling shipped (extract + score scripts, 130 rows in ~/.fleet/fixtures)"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

After the triage close (see
[[grounded-selection-expanded-tier123-2026-11-10]]), the board was
audited and two boxes closed on evidence:

- **seat-decision-label-duplication plan → COMPLETED.** Both render
  sites already build the button with no text (`node('button',
  'decision')` at src/gui/assets/app.js:3530 and :3578); C3's own
  command (`cargo fmt --all --check && cargo build --release`) exits 0,
  and the suite ran green the same day (2,356). Tick C3 with the two
  line numbers as the evidence.
- **KI-515 → CLOSED.** The ledger row's `| closed |` cell needs a
  `| next action |` cell AFTER it or `ki_ledger.py` reports "missing
  next action" — the row format is id|severity|description|evidence|
  status|next-action.

**SUPERSEDED LATER THE SAME NIGHT** — every "me" row below was
executed; see [[the-board-is-closed-kis-and-fixtures-2026-11-10]]:
KI-516/494/476/459 all CLOSED (with the real causes found: KI-516 was a
whole-log `find` matching the parent's pre-child reads, not the
override; KI-459 had a SECOND cause, `probed_embedder()` answering
keyword-only on its first call), the contract-doc debt paid, the fixture
campaign run (all five seams beat their rule on MY labels — which are
not ground truth), and the per-item score variant abandoned unrun after
the owner rejected cost framing.

**What remained open at this point (the owner asked for specifics, not generalities):**

| item | exact target | who |
|---|---|---|
| KI-516 | scope `set_project_for_test`'s thread-local PROJECT_OVERRIDE (src/layerfs_tools.rs:750) to the workspace fs tools only, so recall/corpus reads never route through the stub; proof = the coherence test passes ALONE | me |
| KI-494 | `fleet mcp` registry: `computer-use` carries no command; `fli` dies on import — point comp-use at the doctor-resolved driver or remove it | me |
| KI-476 | src/gui/themes.rs — darken each DARK theme's gradient until the sweep clears 4.5:1 vs #fff (light already 4.81, contrast 9.92) | me |
| KI-459 | the ranking benchmark times the EMBED not the rank; embed once outside the timed region and rank against that vector (a vector-taking seam), then `memory.vectors.two_stage` can be decided | me |
| C15 per-item score variant | scripts/c-score-measure.py → N SEPARATE calls (top-5) — the batched shape was a wire artifact; bar = beat the noul re-rank 54.6%/79.0% | me |
| RC C10 | src/footprint.rs — no environment profile exists; feed from footprint-history.jsonl, expose on `fleet footprint`; C11/C12 build on it | me (~2h) |
| live grounding | one real gen1 imagine → vision_qc row; one real fold on a prose run; one real repair with `experiments.repair_leg: "on"` | me |
| doc debt | TECHNICAL-DOCUMENTATION §Interface contracts for selection/generation/experiments/health; FUNCTIONAL-SPECIFICATIONS for the question/repair notice/prose/experiment gate; **CLAUDE.md's Today's state still says 1,013 tests and 0.5.0-beta.1 (now 2,356)** | me |
| B5 | **NOT an exit-beta milestone** — re-scoped 2026-11-10 to "the shakeout PASS closes" (see [[beta-until-a-whole-project-runs]]); the face is relaunched on 0.6.26 (`fleet reload`, pid 86100→21082) with the folder re-granted, and the owner walks pass three during normal use — no crasher → the pass ticks, the beta stays | owner |

**The jev/TypeSafe credit item was DOWNGRADED after a live check**:
`dev-decisions plan-gate docs/plans/2026-11-06-seat-decision-label-
duplication.md` answered `provider=drex, 4/4 covered, PASS` — the
interim pin at `~/.config/sys1/config.toml:9` (`plan_gate =
"drex,jev"`) covers plan-gate; only the chain's tail (jev) is dead.
Do not present the plan-gate as blocked.

**The cut-verify + relaunch pass (the owner: "build and make sure that
the cut is done for the github release that's a pre-req for auto update,
relaunch after that"):** the 0.6.26 release was verified complete
(`gh release view` — not draft, not prerelease, all 6 assets;
`curl -sL .../releases/latest/download/latest.json` → version 0.6.26,
darwin-aarch64; the plain no-`-L` curl returns EMPTY because of the
redirect). The face was still pid 86100 on **0.6.24**; `fleet reload`
re-execed it → **pid 21082**, `/api/version` = 0.6.26, app.js 200. It
then reported **`folder: None`** — KI-492's silent refusal shape, every
fs tool would refuse — **fixed with `POST /api/folder`
`{"folder":"/Users/alejandrodelvillar/Projects/fleet"}`** (never
hand-edit sessions-workdirs.json); `/api/state` now carries the folder
object. **The face's port is 3080, not 8419** (the `DEFAULT_PORT` on
system 3080; 8419 was my guess and answers nothing).

**The plan-gate downgrade phrasing matters:** the interim pin at
`~/.config/sys1/config.toml:9` (`plan_gate = "drex,jev"`) MATCHES the
single-line format `by_task` expects; a live run answered
`provider=drex, 4/4 covered, 1/1 in scope, PASS in 1.6s`. Two stale
"gate is dead" lines were corrected (plan Notes + selection.md), and a
substrate fact `plan-gate-answers-through-drex` landed so no future
session re-asserts the claim. Say "only the jev TAIL is dead", never
"the plan-gate is dead".

**The fixture tooling (C13–C16, the owner said "10 go ahead"):**
`scripts/extract-fixtures.py` builds five labeled tables into
**`~/.fleet/fixtures/` (the HOME, never the repo — real session text +
the repo is public)**: c13-mention 30, c13-silent 20, c14-skills 10,
c15-recall 50, c16-queues 20. `scripts/score-fixtures.py` grades a head
against the owner's filled last column beside the string rule and
prints per-seam BUILD/KILL. Two parser lessons baked in: the **recall
card shape is `- <name> — <description> [score kind date]`** (a
name-only regex finds nothing), and the **fleet verbs render box cards
`┌─ name ─── status ┐`** (strip ANSI, match the corner) — a
plain-text-lines parser yields zero rows. Pilot label verified the
path (head 1/1 vs rule 0/1 → BUILD) then reverted.

Traps re-hit this pass: the zcode-gate blocks compound `rm -rf`
commands (split the cleanup from the commit); `git status --short`
keeps showing `memories/` + `events.jsonl` churn (the substrate's own
recall counts — commit them under `chore(memories)` before a cut, never
by hand otherwise); a generated/ image reappears from the C17 test run;
`2026/11/11/events.jsonl` appears at midnight (the live calib stream's
day rollover — the daily cycle commits it, not debris).

Related: [[grounded-selection-expanded-tier123-2026-11-10]],
[[experimentation-over-refusal]],
[[self-healing-footprint-built-contract-half]].
