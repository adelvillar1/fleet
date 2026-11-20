---
status: completed
created: 2026-11-20
updated: 2026-11-20
slug: omh-borrowings
---

# Plan: the OMH borrowings — five enhancements from the oh-my-hermes review

## Context

`docs/architecture/deep-dive-2026-11-20-oh-my-hermes.md` read
[rlaope/oh-my-hermes](https://github.com/rlaope/oh-my-hermes) (MIT) against
Fleet and ranked four findings, each one verified at file:line on both sides.
This plan is the build half of that dive: five increments, ordered by
capability gained per unit of risk.

Two of the four findings are **capability losses Fleet is living with right
now** — they are not polish:

- **Skill selection is by file mtime.** `src/agent.rs:319` caps the declared
  index at 60 and `src/agent.rs:388` (`declared_skills_priority`) sorts the
  whole library newest-first. Measured 2026-11-20: 467 directories, 464 with a
  `SKILL.md`, so **404 skills are held back from every run's prompt** — and
  because `use_skill_result` (`src/skills.rs:860`) searches only the run's
  declared slice, those 404 are **unloadable by name**, not merely
  unmentioned. The newest 60 are dominated by the HyperFrames video family
  (ranks 55-62), which no ordinary coding turn selects.
- **A plan node's declared paths are a dispatch lock, not a boundary.**
  `Graph::dispatch` refuses a node while a running sibling holds one of its
  paths (`src/graph.rs:738`), but nothing compares a `write_file` target
  against the *writing* child's declaration. A grep for `.with_paths(` finds
  three constructive sites and no fourth consult.

The other three are honesty increments: a number that drifted from its own
document, a "done" tick that is the model's word, and the two-axis status
vocabulary that makes "what does the system know" readable.

**Decision-model check.** Two decisions here are repeated per item and belong
to sys1, not to a language model: which skills a brief wants (a classification
over a closed set with a confidence — the same shape `src/tool_router.rs`
already runs for tools), and whether a write is inside a node's scope (a
closed-set verdict per call). Neither needs a new head: the tool router's
floor-and-margin geometry is the pattern, and the scope check is arithmetic on
paths, not a judgment. So this plan adds **no** sys1 task and no model call on
either path — both stay deterministic, which is also what makes them testable
without a lane.

## Approach

Five increments. C1 and C2 are the ones that return capability; the rest are
what stop the count from lying. Each ships with its tests and its doc update in
the same increment (hard rule 3), and the gate runs between increments.

### Increment 1 — the loader reaches the whole library (fixes C1's hard half)

Smallest possible change with the largest effect, and it lands **before** any
ranking work so a bad ranker can never be a *ceiling*. `use_skill_result`
gains a fallback: a name the run did not declare but the library holds is
loaded from the library and the result says so ("this skill was not declared in
this run's index; loaded from the library"). The retired-skill message stays as
it is. This turns a hard ceiling into a soft one in one function.

### Increment 2 — rank the declared list by the brief, with a floor

Replace the mtime sort with the geometry Fleet already owns. `src/tool_router.rs`
carries `FLOOR = 0.68`, `MARGIN = 0.02`, `ACTIVATE_MAX = 5` and
`Selection::{Confident, Ambiguous, None}` — pure functions, no lane, no
network. The skills path calls the same `select` over a ranking of
name + description + trigger words against the brief, and fills a **token**
budget rather than a count:

- Replace `SKILLS_PROMPT_CAP: usize = 60` with `SKILL_INDEX_TOKENS: usize`,
  sized from the measurement the dive recorded (~3,940 tokens for 60 skills at
  a 400-char clip). The count then falls out of the budget instead of being an
  independent number that can drift from it.
- Ranked selection fills the budget best-first; when nothing clears the floor,
  the run declares the room's skills (if any) and nothing else, and the
  existing growth line (`src/agent.rs:415`) names what was held back.
- The `.skills` session-log record already pins what was declared, so hard
  rule 5 holds by construction — whatever selects, the list is logged.

### Increment 3 — a child cannot write outside what its node declared

The seam already exists: every tool call passes `self.approver.approve(call)`
at `src/agent.rs:5538`, and `src/approvals.rs` already classifies calls
(`Rule::for_call`, `:528`) and reads their arguments (`is_destructive`, `:349`).
A scope rule joins that vocabulary rather than opening a new path:

- `ScopeRule::for_call(call, node)` compares the **resolved** target against
  the enclosing node's `paths`, reusing `resolve_write_in_root` /
  `resolve_in_root` (`src/tools.rs:4299`, `:4251`) — the literal-then-normalized
  comparison OMH hand-rolls.
- The child's node identity rides `AgentOptions` from `src/plan_graph.rs:1049`,
  where the child's `Node` (with its `paths`) is already constructed.
- Refusal text follows OMH's shape: name the file, the node, the declared
  scope, and the escape hatch ("finish what your scope covers and name the file
  in your report"), and close the obvious workaround in the same sentence.
- A node declared read-only blocks every write in the folder except its own
  report — the guarantee `src/verify.rs` benefits from directly: a verification
  that cannot mutate what it verifies is worth the name.

Fails open on an unresolvable lookup (warn once), as OMH does: a guard that
wedges a run is worse than the write it missed.

### Increment 4 — one registry for the numbers that must agree

`docs/architecture/structure-guards.md:59` says `MAX_DEFINITION_TOKENS` is
10,050; `src/agent.rs:216` says 10,800. The doc is the one the repo tells a
session to read before its first compile, and it is stale by five crossings.
OMH's `src/maintenance/drift.py` answers exactly this, and its shape is worth
copying:

- `src/drift.rs` with two row types: a metric (name, a `live` closure that
  **calls the real producer**, the expected value, and `sites` as file paths
  **never line numbers**) and a budget (limit, limit site, `ratchet` vs
  `ceiling`, a named `reviewed_exception`).
- Seed with the budgets Fleet actually has: `MAX_VISIBLE_TOOLS` (36),
  `MAX_DEFINITION_TOKENS` (10,800), `SKILL_INDEX_TOKENS` (new in increment 2),
  `DELEGATION_CONCURRENCY` (3), `DEFAULT_MAX_TURNS` (100), `FLOOR` / `MARGIN` /
  `ACTIVATE_MAX`.
- `fleet drift` beside `fleet doctor`, reporting every invalidated value and
  its sites in one pass.
- A text-scan test in the house style (`include_str!` the guard doc, assert the
  constant's current value appears in it) so the instance cannot recur.

### Increment 5 — a tick that says who closed it

`run_todo` (`src/tools.rs:3566`) renders `[x]` on the model's word alone. The
session log is already the evidence base (hard rule 5), so this needs no new
record type:

- Bind each `completed` item to the nearest evidence-capable tool result in its
  own window (a `bash` result with exit 0, a successful `write_file`/`edit`),
  the way `todo_evidence.py` does — and bind it **from the loop's own records,
  ignoring any reference the model sends**.
- Render `[x] step (verified)` vs `[x] step (unverified — no passing command)`.
  The mark stays; the word stops being free. An item that cannot resolve stays
  explicitly unverified rather than silently becoming done.

**Ordering rationale.** 1 → 2 is forced (the loader must stop being a ceiling
before ranking matters). 3 is independent of both and touches the approval
path, so it ships alone. 4 depends on 2 (it seeds `SKILL_INDEX_TOKENS`) and
therefore follows it, though it could be built earlier if the drift is urgent.
5 is last because it is the smallest.

## Acceptance Criteria

- [x] A skill outside the 60 declared in a run can be loaded by `use_skill` and
      the result names the fact that it was not in the declared index.
- [x] With a 464-skill library and no configuration, the declared index fits a
      stated token budget and the run records which skills it held back.
- [x] A brief naming a domain ("fix the flaky embedder test") declares the
      skills that mention that domain, ranked above the newest-by-mtime ones,
      asserted against a fixed library fixture.
- [x] A delegated child whose node declares `paths: ["src/a.rs"]` has a
      `write_file` to `src/b.rs` refused by name, naming the node and the
      declared scope, and the refusal does not reach the file.
- [x] A node declared read-only refuses every write in the folder except its
      own report.
- [x] A scope guard that cannot resolve its input lets the write through and
      says so once, rather than wedging the run (`warn_unresolvable_once`).
- [x] `fleet drift` reports every seeded budget with its sites, and a text-scan
      test fails when a documented bound disagrees with its constant (the
      current `structure-guards.md` discrepancy is fixed or the test names it).
- [x] A `todo` item marked completed with no evidence-capable call behind it
      renders as unverified, and one behind a passing command renders verified.
- [x] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` all exit 0 at every increment.

## Files to be touched

**Increment 1 — loader reach**
- `src/skills.rs` — `use_skill_result` gains the library fallback and its message.
- `src/agent.rs` — the `Work::UseSkill` arm passes the library, not only the slice.
- `src/tools.rs` — `use_skill_spec`'s description currently reads "an unknown name
  is an error result, not a guess" (`:227`). After this increment that sentence
  is false for a name the library holds, so it must say the library is searched
  and only a name the library does not hold is an error. A schema that lies to
  the model is the failure the `subagent` spec's own comment names.

**Increment 2 — ranked selection**
- `src/agent.rs` — `SKILLS_PROMPT_CAP` → `SKILL_INDEX_TOKENS`; `declared_skills_priority` becomes budget-filling rank.
- `src/tool_router.rs` — a skill-ranking entry point beside the tool one, reusing `select`.
- `src/skills.rs` — the ranking input (name, description, trigger words) as a `Candidate`.
- `src/tools.rs` — the `use_skill` spec text if it names the cap.

**Increment 3 — write scope**
- `src/approvals.rs` — `ScopeRule`, its refusal text, and the fail-open warn.
- `src/agent.rs` — the consultation beside `self.approver.approve(call)` (:5538) and carrying the node into `AgentOptions`.
- `src/subagents.rs` — pass the node binding down.
- `src/plan_graph.rs` — expose the child's `paths` with the node id.

**Increment 4 — drift registry**
- `src/drift.rs` (new) — the two row types and `report()`.
- `src/main.rs`, `src/cli.rs` — the `drift` verb.
- `docs/architecture/structure-guards.md` — the corrected 10,800 and the registry's pointer.
- `docs/known-issues.md` — the KIs this closes.

**Increment 5 — todo evidence**
- `src/tools.rs` — `run_todo` renders the verified/unverified marks.
- `src/agent.rs` — hand `run_todo` the turn's own tool results.
- `src/session.rs` — read only; no new record type.

**Docs (all increments)**
- `docs/features/skills.md`, `docs/features/subagents.md`,
  `docs/features/memory.md` where the numbers move.

## Out of scope

- **OMH's two-axis phase·confidence label** across every Fleet surface. The
  dive names it as an adaptation worth one map that fails closed, not a
  vocabulary migration; it is not in this plan.
- **OMH's routing prep chain** (`RoutingQuery`). Its value arrives with
  increment 2 (ranking and tool selection sharing a prepared input); folding it
  into the increment would double the surface for no extra capability.
- **OMH's reasoning-effort axis.** Fleet's `ChatRequest` (`src/api.rs`) has no
  effort parameter, and `reasoning_content` is deliberately display-only.
- **OMH's test sharding.** Measured 2026-11-19: 2,410 lib tests in 58s. Revisit
  when the test phase alone crosses ten minutes.
- **Raising the per-session checkpoint keep** or adding an idle-age rule for
  `~/.fleet/checkpoints` — that is KI-575's neighbourhood and the owner's call
  on rewind depth, not this plan's.

## Verification

1. Per increment, the touched module's tests first: `cargo test --lib
   skills::`, `cargo test --lib tool_router::`, `cargo test --lib approvals::`,
   `cargo test --lib drift::`, `cargo test --lib tools::`.
2. Increment 1 and 2 measured against the live library: a probe that builds
   `TurnCaps`-style resolved state from `~/.agents/skills` and asserts the
   declared count, the token total, and that a rank-61+ skill is now reachable
   by name. The probe is scratch, run and deleted, the way the child-cap probe
   was.
3. Increment 3 against a scratch plan graph: declare a two-node plan with
   disjoint `paths`, dispatch, attempt an out-of-scope write, assert the
   refusal text and that the file is untouched on disk.
4. Increment 4 by construction: `fleet drift` before and after deliberately
   moving a constant, asserting the report names it and its sites.
5. The full gate (`cargo fmt --all --check`, `cargo clippy --all-targets -- -D
   warnings`, `cargo build --release`, `cargo test`), read by exit code, at
   every increment's landing.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-oh-my-hermes.md` — the source of every
  finding; each increment cites its section.
- `docs/features/skills.md` — the index cap, the selection rule, and the loader's
  reach.
- `docs/features/subagents.md` — the scope rule and the read-only node class.
- `docs/architecture/structure-guards.md` — the corrected `MAX_DEFINITION_TOKENS`
  and a pointer to `fleet drift` as the one home for the bounds.
- `docs/known-issues.md` — the structure-guards drift and the loader ceiling
  close here; each row's status flips in the increment that fixes it.
- `TECHNICAL-DOCUMENTATION.md` — the skills and delegation sections, if they
  restate the cap or the scope claim.
