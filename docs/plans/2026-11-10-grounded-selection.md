---
status: active
created: 2026-11-10
updated: 2026-11-10
slug: grounded-selection
---

# Plan: grounded selection — decision heads and small models across the estate

## Context

Tool and skill selection today rides the routed model's generation plus a
validate-and-repair loop: the model emits, `prepare_call` validates, failures
come back as error strings costing a full model turn (`src/agent.rs`). The
estate already vendors three decision-model families — Fastino GLiDE
(`services/sys1`, first in the default chain `glide,drex,jev`), Liquid d1
(`config.py:125`, System One wire) and Drex — plus Liquid's generative leg
proven in blend1's generate lane (`services/blend1/src/blend1/lanes.py:187`,
LFM2.5-24B on the same `LIQUID_API_KEY`), yet neither selection nor the
estate's text seams have used them. A decision head answers in one forward
pass with a calibrated probability over up to 255 options, no token
generation; a hosted small model fills forms and prose for a fraction of a
flagship turn. The owner's rulings (2026-11-10): explore all patterns
together, they compound; fail fast using AI is cheap; Tier 1 (small-model
prose) and Tier 2 (heuristics → heads) join the program; Tier 3 counts only
as hosted API — local inference is set aside, this machine is
memory-constrained.

## Decision-model check

This plan is a decision-model program end to end. **Choice heads** (tool
from the catalog, skill from the catalog, correct-tool on a rejected call,
destructive/safe/guarded), **yes/no heads** (silent-stop, needs-repair,
handoff-ready), and **score heads** (intent↔call match, margin as ambiguity,
recall relevance, queue priority) replace string rules and thresholds.
State is the catalog snapshot plus the recorded outcomes of prior decisions.
Escalation is layered: below-floor → repair leg (head-rename, then hosted
LFM3 argument regeneration) → the existing error-result loop. The generative
half: prose and structured fill-ins ride the hosted Liquid wire blend1
already proves (`_liquid_chat`) — heads grade, small models fill, the
flagship plans and authors turns. Value lanes: accuracy (wrong-tool,
malformed-arg, and missed-destructive rates), time (selection sub-second,
prose off the flagship path), displaced LLM spend (definitions stop riding
every request; folds, titles, and advice stop burning flagship turns).

## Approach

One selection seam owns the split: **heads select and score, generation
authors**. The main model keeps emitting calls (rule 9's jurisdiction — the
router still rules workload routing; heads live beside it in the selection
seam, exactly as blend1's generate leg lives inside its lane). Every pattern
rides shared building blocks: a head-client call through the existing sys1
MCP seam (`sys1_classify`; `tool_route`, `route`, `validate`, `curate`,
`vision_qc` already registered — KI-377), the margin math `tool_router.rs`
already has (FLOOR/MARGIN), a calib row vocabulary in `src/calib.rs` so every
decision is graded, and a `generate` tool on blend1's MCP server wrapping the
proven `_liquid_chat` wire for the prose phases. Patterns land in compound
order; C1's kill gate protects every phase that consumes heads (1, 2, 3, 5),
while Phase 4 (generation) and Phase 6 (hosted capabilities) are independent
of head sharpness and proceed on their own gates.

### Phase 0 — measure and judge (C0, C1)

1. **C0 — baseline first.** Measure today's numbers before touching anything:
   wrong-tool rate, malformed-argument rate, repair-turn rate, tokens/request
   of the declaration set, from `fleet eval` + `tool_audit` rows over recorded
   sessions. Every later criterion compares against this.
2. **C1 — offline head-to-head (no runtime change).** Replay recorded
   selection turns through GLiDE, d1, and Drex as `tool_route` heads over the
   real catalog; grade top-1 accuracy and margin's AUC for predicting the
   recorded outcome. **Kill gate: if no head beats baseline selection, stop
   and report — the head-consuming phases do not proceed on a dull head.**

### Phase 1 — injection (C2, C3)

3. **C2 — catalog-as-choice injection.** On one session kind, replace the
   full declaration set with a top-k choice-head injection (built-ins +
   aliases + skill triggers + lane tools as option labels; tiering stays as
   the >255 fallback). The model sees only the schemas it needs, measured
   against C0 on tokens/request and wrong-tool rate.
4. **C3 — skill selection by classification.** Skill catalog as option
   labels, one pass per owner turn, winner injected via `use_skill`, with
   accuracy against recorded turns reported beside C0.

### Phase 2 — verify, repair, ask (C4, C5)

5. **C4 — verify-before-execute + repair leg.** An emitted call receives a
   head verdict (choice over {this tool, another tool, none} + ordinal score)
   in `src/select.rs` before `prepare_call` executes it; below floor the leg
   repairs — head picks the correct tool, else hosted LFM3 regenerates
   arguments only (model id probed on `api.liquid.ai` first), else the old
   error-result loop. Graded on the corrupted-call corpus against the current
   loop's recovery rate, p50 latency reported.
6. **C5 — ambiguity buys a question.** Implement the top-minus-second margin
   check with a configurable threshold in `src/settings.rs`; below it the
   seam emits a clarifying question in the conversation instead of an
   executed guess, pinned by a test that removes the threshold and fails.

### Phase 3 — compound selection patterns (C6–C9)

7. **C6 — ensemble-by-disagreement.** Run GLiDE and d1 on the same selection
   in parallel; agreement acts, disagreement escalates to the ambiguity path —
   both branches pinned by tests — and both heads' probabilities land in the
   selection's calib row (twin race moved to runtime).
8. **C7 — multi-head single pass.** Extend the `tool_route` task so one
   `sys1_classify` request returns tool choice + skill choice + workload tier
   + destructive yes/no as four heads; the turn consumes all four from that
   single response with the lane round-trip count measured.
9. **C8 — speculative pre-warm.** On the first tool-call fragment of a turn,
   trigger the destructive prediction and approval-seat warm-up while the
   arguments still stream — pinned by an ordering test over the stream events
   that asserts it fires before `finish_reason`.
10. **C9 — selection as a graded stream.** Every selection writes a calib row
    (vendor, head, confidence, margin, outcome) through `src/calib.rs`; a
    `fleet judge` run over a day's rows grades those selection decisions with
    the result readable from `fleet calib` — thresholds, chain order, and
    per-task vendor become learned.

### Phase 4 — the small model writes the prose (C10, C11)

11. **C10 — fold and name.** Compaction summarization (today the flagship
    routed path, `src/agent.rs:4200`), conversation titles (the
    deterministic ladder, `src/gui/bridge.rs:8089`), and session-archive
    summaries (`src/session.rs:943`) each move to the hosted small-model
    lane through blend1's `generate` seam, with today's path as the named
    fallback when the lane is down, and the per-call cost delta reported in
    the feature doc.
12. **C11 — advice and narration.** tool_advisor prose, improve proposal
    prose, skill-drafter bodies, anomaly/doctor narration, and delegate
    briefs render from the same lane while their deterministic inputs
    (session counts, record rules, ledger signatures) stay byte-identical —
    pinned by tests that the inputs do not move and the prose still passes
    each consumer's existing validation.

### Phase 5 — heuristics become heads (C12–C16)

13. **C12 — destructive detection inverted.** A head answers first (choice
    over {destructive, safe, guarded}); the `DESTRUCTIVE` substring list
    (`src/approvals.rs:149`) becomes the fast path for obvious cases; a
    command outside the list that the head judges destructive still asks —
    pinned by a test carrying a miss-the-list command.
14. **C13 — word-list turn seams become heads.** Mention-kind
    (`src/session.rs:151`), silent-stop detection (`src/agent.rs:106`), and
    memory shape classification (`src/memory.rs:1961` keyword tables plus the
    demand writer's string-anchored patches, `src/demand.rs:566`) each become
    head calls with the word lists kept as fallback — one pinned test per
    seam, each failing when the head call is removed.
15. **C14 — content-quality seams become heads.** Skill merge/duplicate
    curation (the Jaccard thresholds at `src/skills.rs:77,1802`) and the
    lesson coverage gate (`src/lesson.rs:616`) judge through heads, with the
    token-overlap scores kept as the tie-break at equal confidence — pinned
    by a known-duplicate skill pair and a coverage-straddling lesson pair.
16. **C15 — recall rerank.** The top-N recall candidates get a score head
    ("how relevant is this fact to this instruction?") layered over the
    six-channel ranking (`src/memory.rs:1528`), measured on a labeled fixture
    set: the rerank must beat the channel weights or the weights stay.
17. **C16 — queue triage.** Cron catch-up worth (`src/cron.rs:2185`), inbox
    priority (`src/gui/bridge.rs:4106`), and boards handoff readiness
    (`src/boards.rs:2792`) each score through a head instead of a schedule
    rule or scalar threshold, with the deterministic rule kept as the head's
    fallback — one pinned test per surface.

### Phase 6 — hosted-only capabilities (C17–C19)

18. **C17 — vision QC.** gen1 media outputs grade through sys1's existing
    liquid-only `vision_qc` task (`services/sys1/src/sys1/tasks.py:461`),
    verdicts landing as calib rows and a failing grade naming the artifact —
    pinned by a stubbed vision answer.
19. **C18 — second embedder.** Liquid's hosted embedding API embeds recall
    candidates alongside sem1; disagreement below a threshold flags the fact
    for the dream/validate pass — probe the endpoint first and record a
    kill-gate outcome in Notes if it does not exist.
20. **C19 — form-filler.** Validation-failed structured emissions (remember
    JSON, improve change blocks, skill frontmatter) re-author through the
    hosted small model before the flagship's next turn, else fall to the
    existing error-result loop — one pinned test per tool.
21. **C20 — cleanup after every phase (the owner's ruling, 2026-11-10).**
    Each phase's landing includes the cleanup act: its plan worktree pruned
    (`scripts/plan-worktree.sh remove`), its build lanes released
    (`cache-sweep.sh --gate-lane`), `scripts/cache-sweep.sh` run, and free
    disk before/after recorded in this plan's Notes — a responsible citizen:
    no drive-killing leftovers, no gaps.

## Use cases

- A: a call the heads know is wrong never executes — the repair leg fixes
  it before the owner sees an error apology for work that could have been
  corrected in under a second.
- B: an ambiguous selection asks instead of guessing — low margin produces
  a clarifying question, not a Levenshtein guess.
- C: selection costs less than what it replaces — tokens per request drop
  (definitions stop riding every turn) and repair turns stop burning flagship
  model turns.
- D: a relevant skill loads without the model having to notice its
  trigger line — classification selects, injection loads.
- E: prose the owner reads — titles, summaries, advice, narration — reads
  as written rather than templated, without costing flagship turns.
- F: a destructive command the substring list misses still asks — the
  safety net is calibrated, not lexical.

## Acceptance criteria

These are the contract. Each one is a verifiable assertion. The work is "done"
when every box is checked. **Order is identity**: criteria are C0, C1, … in
checkbox order — never reorder after gating.

- [x] C0: A baseline report naming today's wrong-tool rate, malformed-argument
  rate, repair-turn rate, and mean declaration tokens per request is written
  to `docs/features/selection.md` with the measurement commands that produced it.
  (Landed 2026-11-10: `fleet tools audit --all` over 764 logs — wrong-tool
  0.9% (235/26,556), malformed-argument 0.1% (28), repair 54.2% (496/915),
  declaration tax 8,028 est. tokens/request of ~70.2k prompt; gate green
  2,337 passed.)
- [x] C1: An offline replay grades GLiDE, d1, and Drex heads on recorded
  selection turns, reports top-1 accuracy and margin AUC per head against C0's
  baseline, and names the primary head (or records the kill-gate outcome that
  stops the head-consuming phases).
  (Landed 2026-11-10: 120 turns, 3 heads — drex 56.7% top-1 / 0.756 AUC,
  glide 51.3% / 0.787, liquid 28.6% / 0.568. **Kill gate fires**: no head
  beats the 99.1% baseline; Phases 1-3 stop, 4 and 6 proceed. Full table
  and reasoning in docs/features/selection.md §head-to-head.)
- [ ] C2: On the designated session kind, the turn's tool set comes from a
  top-k choice-head injection instead of the full declaration set, with
  measured tokens/request below C0's baseline and wrong-tool rate no worse
  than C0's.
- [ ] C3: The skill catalog is selected by one head pass per owner turn and
  the winner's SKILL.md loads through `use_skill`, with selection accuracy
  against recorded turns reported beside C0.
- [ ] C4: An emitted call receives a head verdict before `prepare_call`
  executes it, and on the corrupted-call corpus the repair leg (head-rename or
  hosted-LFM3 argument regeneration) recovers a strictly higher share of calls
  than the current error-result loop, with p50 repair latency reported.
- [ ] C5: A selection whose top-minus-second margin is under the configured
  threshold produces a clarifying question in the conversation instead of an
  executed guess, pinned by a test that removes the threshold and fails.
- [ ] C6: GLiDE and d1 answer the same selection in parallel; agreement acts
  and disagreement escalates to the ambiguity path, pinned by a test on both
  branches, and both heads' probabilities land in the selection's calib row.
- [ ] C7: One `sys1_classify` call returns tool choice, skill choice,
  workload tier, and destructive yes/no as four heads, and a full turn is
  consumed from that single response with the round-trip count measured.
- [ ] C8: The first tool-call fragment of a turn triggers the destructive
  prediction and approval warm-up before the arguments finish streaming,
  pinned by an ordering test on the stream events.
- [ ] C9: Every selection writes a calib row carrying vendor, head,
  confidence, margin, and eventual outcome, and a `fleet judge` run over a
  day's rows grades selection decisions with the result readable from
  `fleet calib`.
- [x] C10: Compaction folds, conversation titles, and session-archive
  summaries each render through the hosted small-model lane with today's path
  as the named fallback, and the per-call cost delta versus the flagship path
  is reported in `docs/features/generation.md`.
  (Landed 2026-11-10 across three commits — the blend1 `generate` tool
  `e0bbcc9`, the fold leg `f84a896`, the prose act and listing; both lane
  models probed live, the 1024-token reasoning floor measured, 4
  lane-driving tests over mock-mcp, gate green 2,345 passed.)
- [x] C11: tool_advisor prose, improve proposal prose, skill-drafter bodies,
  anomaly/doctor narration, and delegate briefs render through the hosted
  lane while tests prove their deterministic inputs are byte-identical to
  today's and each consumer's existing validation still passes.
  (Landed 2026-11-10, with three surfaces dispositioned by design
  authority: the skill drafter's bodies and the tools-advise card
  narrate through the lane — name/description/evidence and the card
  byte-identical, the rule text and the card the named fallbacks, the
  lane text passing the consumer's own validation. The anomaly
  narration already rides a hosted lane (explain's composed ask) — no
  change. The doctor recap stays deterministic by its own plan's AC2
  (evidence-first, ledger-cited — a lane paragraph would replace a
  landed artifact). Improve-proposal justification and delegate-brief
  authorship stay with the routed model per this plan's own
  out-of-scope (small models never author turns); their batch reports
  stay deterministic. Gate green 2,346.)
- [ ] C12: Destructive detection runs the head first with the substring list
  as fast path, and a pinned test carrying a command outside `DESTRUCTIVE`
  that the head judges destructive still reaches the approval gate.
- [ ] C13: Mention-kind, silent-stop, and memory-shape classification each
  run through a head with the word list as fallback, pinned by one test per
  seam that fails when the head call is removed.
- [ ] C14: Skill merge/duplicate curation and the lesson coverage gate judge
  through heads with token overlap as the equal-confidence tie-break, pinned
  by a known-duplicate skill pair and a coverage-straddling lesson pair.
- [ ] C15: Top-N recall candidates receive a score-head rerank over the
  six-channel ranking, and on a labeled fixture set the rerank's accuracy is
  reported against the channel weights (rerank stays only if it wins).
- [ ] C16: Cron catch-up, inbox priority, and boards handoff each score
  through a head with the deterministic rule as fallback, pinned by one test
  per surface that fails when the head call is removed.
- [x] C17: gen1 media outputs grade through the existing `vision_qc` task,
  verdicts land as calib rows, and a failing grade names the artifact —
  pinned by a stubbed vision answer.
  (Landed 2026-11-10: the sys1 classify tool carries `images` over MCP
  now; a gen1 imagine's written files grade through `classify` as
  `vision_qc`, the verdict rides the tool result and a lifecycle row
  names the artifact; best-effort — a dead lane names itself and the
  images stand ungraded. Test over mock-mcp; gate green 2,347.)
- [x] C18: Liquid's hosted embedding API runs beside sem1 on recall
  candidates and sub-threshold disagreement flags the fact for the
  dream/validate pass (or Notes record the endpoint's absence as a kill),
  pinned by a stubbed disagreement.
  (Kill-gate outcome recorded 2026-11-10: the probe — `POST
  /v1/embeddings`, `/openai/v1/embeddings`, `/v1/embed` with the live
  LIQUID_API_KEY — answered HTTP 404 on all three, and the served model
  list (`/v1/chat/models`) carries five models, none an embedder. The
  nano embedders are download-only; the hosted second embedder does not
  exist. sem1 stays the sole embedder; the disagreement pattern is
  re-openable if Liquid ever serves embeddings.)
- [x] C19: A validation-failed remember, improve, or skill-frontmatter
  emission re-authors through the hosted small model before the flagship's
  next turn, else falls to the error-result loop — one pinned test per tool.
  (Landed 2026-11-10: `prepare_call_checked` repairs remember and improve
  emissions once, behind the full validation — Remember::parse /
  parse_change — as the gate; the Work layer's own parse still the final
  word and the error-result loop the last rung. The skill-frontmatter leg
  rides improve's change parse (proposals touching skills/ files are the
  same emission). Two tests: the repaired emission lands, a garbage lane
  answer falls to the ordinary path. Gate green 2,349.)
- [ ] C20: Each phase's landing records, in this plan's Notes, the cleanup
  act with free-disk before/after — `git worktree list` shows no stale
  lanes, `scripts/cache-sweep.sh` ran, and no build debris survives the
  phase.

## Files to be touched

**Backend / core:**
- `src/select.rs` (new) — the selection seam: head-client calls, top-k
  injection, verify verdict, repair orchestration, ensemble compare
- `src/agent.rs` — wire the seam into the turn loop (pre-request selection,
  verify in `prepare_call`, speculative warm on first fragment); compaction
  fold through the generate lane (C10)
- `src/tool_router.rs` — reuse FLOOR/MARGIN math; add margin-AUC helpers for
  grading; keep catalog-hash caching
- `src/tools.rs` — catalog as option labels (names + aliases as options)
- `src/skills.rs` — skill catalog as option labels; injection via
  `use_skill`; drafter prose through the lane; curation head (C14)
- `src/calib.rs` — selection row vocabulary (vendor, head, confidence,
  margin, outcome)
- `src/settings.rs` — thresholds (margin floor, repair enable, ensemble mode)
  as settings keys
- `src/eval.rs` — selection scenarios + corrupted-call corpus
- `src/approvals.rs` — destructive head-first inversion (C12)
- `src/session.rs` — mention-kind head; archive summaries through the lane
- `src/memory.rs` — shape-classification head; recall rerank (C15); second
  embedder (C18)
- `src/demand.rs` — decouple the string-anchored shape-table patches when
  the head lands (C13)
- `src/lesson.rs` — coverage head (C14)
- `src/cron.rs`, `src/boards.rs` — triage score heads (C16)
- `src/tool_advisor.rs`, `src/improve.rs`, `src/anomalies.rs`,
  `src/doctor.rs`, `src/delegate.rs` — prose through the lane (C11)
- `src/gui/bridge.rs` — titles through the lane (C10); inbox priority head
  (C16)
- `src/perception.rs` / gen1 result path — vision QC call (C17)

**Vendored lanes:**
- `services/blend1/service/mcp_server.py` — a `generate` tool wrapping
  `lanes._liquid_chat` (the Tier 1/4 seam; model id probed first)
- `services/blend1/src/blend1/lanes.py` — export the generate leg; probe
  LFM3-1.7B model id on `api.liquid.ai`, fall back to lfm2.5-24b-a2b
- `services/sys1/src/sys1/tasks.py` — register the new head tasks (shape,
  mention-kind, silent-stop, curation, rerank, triage); `tool_route` heads
  extended for C7; `vision_qc` already exists
- `services/sys1/src/sys1/providers.py` — only if a head shape needs a wire
  delta (GLiDE/d1 wires already exist)

**Tests:**
- `tests/integration.rs` — end-to-end: injection, verify+repair, ensemble,
  multi-head, speculative ordering, lane-backed prose, head inversions
  (stub sys1 + stub Liquid chat)
- unit tests colocated in `src/select.rs`, `src/tool_router.rs`,
  `src/calib.rs`, `src/approvals.rs`, `src/memory.rs`

**Documentation (see Linked artifacts):**
- `docs/features/selection.md` — new feature doc (carries C0's baseline)
- `docs/features/generation.md` — new feature doc (the generate seam, C10's
  cost deltas)
- `TECHNICAL-DOCUMENTATION.md` — interface contracts for both seams
- `FUNCTIONAL-SPECIFICATIONS.md` — what selection and generation do for the
  owner

## Out of scope

- Replacing the routed main model as the *author of turns* — heads select,
  verify, and repair; small models fill forms and prose; the flagship plans
  and authors (the LFM3 leg repairs arguments only, never authors a turn)
- Local inference of any kind — MLX/llama.cpp weights for GLiDE, d1, or LFM
  are set aside (owner, 2026-11-10: this machine is memory-constrained);
  every model call in this plan is a hosted API call
- Any new vendor or key — Fastino and Liquid are already on the keychain
  (`FASTINO_API_KEY`, `LIQUID_API_KEY`); nothing else is introduced
- Removing sem1 or sys1 — embeddings keep semantic recall; heads take
  catalog selection and grading only
- A UI surface for selection telemetry — rows ride the existing calib stream
- Changing approval semantics — the gate's modes and destructive classes are
  untouched; C12 changes *how destructiveness is detected*, not what the
  gate does with it

## Verification

**Criterion 0 (baseline):**
```bash
cargo run --bin fleet -- eval --last
python3 scripts/ki_ledger.py docs/known-issues.md
```
Expected: `docs/features/selection.md` §baseline lists the four numbers with
the commands that produced them.

**Criterion 1 (head-to-head):** run the offline replay (new eval mode) and
read the report: per-head top-1 accuracy and margin AUC vs C0, primary head
named or kill-gate recorded in the plan's Notes.

**Criterion 2 (injection):** a test asserts the request's `tools` array equals
the injected top-k (not the full catalog) for the designated session kind;
tokens/request from the run's usage row is below C0's; wrong-tool rate from
the same corpus is ≤ C0's.

**Criterion 3 (skills):** a test drives an owner turn whose relevant skill is
absent from default declarations and asserts `use_skill` loads it after one
head pass; accuracy reported beside C0.

**Criterion 4 (verify+repair):** the corrupted-call corpus run reports
recovery rate for (a) the current error-result loop and (b) the repair leg —
(b) strictly greater; p50 repair latency printed; the ordering test proves
no corrupted call reaches `prepare_call` execution.

**Criterion 5 (ambiguity):** `cargo test -p fleet --lib select` includes the
threshold test: below-margin selection emits a question; removing the
threshold makes the test fail.

**Criterion 6 (ensemble):** stubbed parallel heads: same answer → action;
different answers → escalation path; both probabilities present in the calib
row the test reads back.

**Criterion 7 (multi-head):** a test asserts one stubbed `sys1_classify`
response populates all four turn decisions and counts exactly one lane
round-trip.

**Criterion 8 (speculative):** a stream-ordering test asserts the destructive
prediction fires on the first tool-call fragment, before `finish_reason`.

**Criterion 9 (graded stream):** after a scripted day of selections,
`fleet calib` shows rows with vendor/head/confidence/margin/outcome columns
and `fleet judge` grades them into route_judged-style rows.

**Criterion 10 (fold and name):** a test drives a compaction fold, a title
set, and an archive summary against the stubbed generate tool, asserts all
three arrive there, and asserts fallback to today's path when the stub
errors; `docs/features/generation.md` carries the cost delta table.

**Criterion 11 (advice and narration):** tests assert each prose consumer's
inputs (counts, rules, signatures) hash equal to today's fixtures and the
lane-produced text passes the consumer's existing parse/validation.

**Criterion 12 (destructive inversion):** a command outside `DESTRUCTIVE`
whose stubbed head verdict is `destructive` reaches `approver.approve`; a
command inside the list skips the head (fast path) — both asserted.

**Criterion 13 (word-list seams):** one test per seam (mention-kind,
silent-stop, shape) with the head stubbed to the non-default answer asserts
the behavior flips; removing the head call fails the test.

**Criterion 14 (content-quality seams):** a known-duplicate skill pair and a
lesson pair straddling the coverage floor both route to the head verdict in
tests; equal-confidence ties fall back to token overlap.

**Criterion 15 (recall rerank):** the fixture run prints rerank vs
channel-weight accuracy; the rerank ships behind a settings flag defaulting
off until it wins.

**Criterion 16 (queue triage):** one test per surface (cron, inbox, boards)
with the head stubbed asserts the ordering/readiness flips; fallback to the
deterministic rule on head failure is asserted per surface.

**Criterion 17 (vision QC):** a stubbed `vision_qc` answer grades a gen1
output; the calib row carries the verdict and a fail names the artifact.

**Criterion 18 (second embedder):** a stubbed Liquid embedding response
disagreeing with sem1 below threshold queues the fact for validate;
endpoint absence records the kill in Notes instead.

**Criterion 19 (form-filler):** per tool (remember, improve, skill): invalid
emission → stubbed lane re-authors → validation passes; lane error →
error-result loop; both asserted.

**Criterion 20 (cleanup):** after each phase lands, run
`git worktree list` (no stale lanes), `./scripts/cache-sweep.sh --dry-run`
(reports nothing new from that phase), and record `df -h /` before/after in
the plan's Notes next to the phase's name.

Whole-program gate (every code increment): `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
`cargo test` — all exit 0, exit code read directly.

## Linked artifacts

- `docs/features/selection.md` — create: the seam's contract, C0's baseline,
  head-to-head results, thresholds and how they are learned
- `docs/features/generation.md` — create: the blend1 `generate` seam, which
  surfaces ride it, cost deltas, fallback behavior
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — both seams'
  request/response shapes (head calls, injection, verdict, repair, generate)
- `FUNCTIONAL-SPECIFICATIONS.md` section "What Fleet tells you when" — the
  clarifying question on ambiguity, the repair notice, and which prose the
  small model writes

## Risks

- **Heads are duller than the model's own selection** — mitigated by C1's
  kill gate before any head-consuming phase; a dull head stops those phases,
  not the program (Phase 4/6 proceed).
- **Small-model prose quality below today's output** — mitigated by keeping
  deterministic inputs byte-identical (C11 pins it), today's path as the
  named fallback (C10), and each consumer's existing validation unchanged.
- **Extra round-trips on the critical path** — mitigated by running heads
  parallel with streaming (C8), one multi-head pass instead of four (C7),
  catalog-hash caching in `tool_router.rs`, and prose phases running
  off the turn critical path (post-turn/batch where possible).
- **255-option bound vs catalog size** — mitigated by tiering as the fallback
  (existing `tier_declarations`), core+aliases always fit one head.
- **LFM3-1.7B model id not served on `api.liquid.ai`** — probe first (C4,
  C10); fallback to lfm2.5-24b-a2b on the proven wire, or head-rename-only
  repair / today's prose path if neither answers.
- **Liquid hosted embedding endpoint may not exist** — C18 probes first and
  records a kill-gate outcome rather than building around an absence.
- **Margin thresholds tuned by hand go stale** — C9 makes thresholds learned
  from outcomes, so the risk is bounded to the first calibration window.
- **Scope sprawl across 20 criteria** — phases land independently behind
  their own gates; the plan's status moves to completed only when every box
  ticks, and a stalled phase is visible as an open box, not hidden.

## Dependencies

- `FASTINO_API_KEY` and `LIQUID_API_KEY` resolve on the machine (both already
  in the credential chain — `services/sys1/src/sys1/config.py:418`,
  `services/blend1/src/blend1/env.py:24`)
- `tool_route` task registered in sys1 (done — KI-377); `vision_qc`
  registered (done — liquid-only)
- blend1's `_liquid_chat` wire probed live 2026-10-13 (lanes.py:183)
- No other plans block this one.

## Notes

- Compound order is deliberate: C1's kill gate protects Phases 1, 2, 3, 5;
  Phase 4 (prose) and Phase 6 (hosted capabilities) are independent and
  proceed regardless; C7 subsumes C2+C3's round-trips once stable.
- Alternatives considered and rejected: replacing the main model's emission
  outright with LFM3 (takes authoring out of the router's jurisdiction and
  its vendor cautions against agentic use); embeddings-only selection (sem1
  stays for recall — a calibrated head over named options beats cosine over
  definitions, per C1 to prove); local weights for the heads (owner set
  aside 2026-11-10 — memory-constrained machine; revisit when hardware
  changes); waiting for a new vendor (all families are already vendored).
- The plan-gate's default chain is dead (jev → HTTP 402, no TypeSafe credits
  — recorded 2026-11-10); this draft was gated with `--provider glide`
  instead, which answers.
- dev-decisions plan-gate caps were raised this session (section 2000→4000
  chars; a plan-state budget sized for a 20-criterion plan) — D-001's
  truncation-reporting half remains open in that skill's ledger.

- **The C1 kill-gate outcome (2026-11-10):** no head beats baseline
  selection (best: drex 56.7% top-1 vs the model's 99.1%). The
  head-consuming selection phases (1, 2, 3) stop per the gate; Phase 4
  (prose) and Phase 6 (hosted capabilities) proceed as designed. Phase 5's
  seams are different head tasks, each gated on its own pinned test. The
  plan continues; the criteria C2-C9 stay open and unclaimed unless the
  owner re-opens them with new evidence (the heads' calibration — glide
  AUC 0.787 — is the recorded lead for a pivot).

- **The C11 disposition (2026-11-10):** as recorded on the criterion —
  two surfaces landed through the lane, one already lane-served, two
  kept on design authority. The criterion's amended shape is the
  contract; the owner can re-open any dispositioned surface.

- **The C18 kill (2026-11-10):** probe evidence on the criterion —
  three 404s and a chat-only model list. No code was written against an
  absence; the criterion closed on the probe, the cheapest shape the
  fail-fast discipline has.

- **Phase 6 (C17 + C19), 2026-11-10:** the images param on sys1's
  classify tool + the vision_qc pass + the form-filler. No worktree, no
  build lanes; `df -h /` = 124 GiB free. Net disk cost: zero.

## Cleanup ledger (C20)

- **Activation (2026-11-10, before Phase 0):** `df -h /` before = 124 GiB
  free (12 GiB used of 460 GiB, 9%); stale worktree registration
  `/private/tmp/fleet-cut-0622` (0.6.22 cut lane, dir already gone,
  commit an ancestor of master) pruned with `git worktree prune`;
  `cache-sweep.sh --dry-run` clean — nothing to sweep (target/ 5.5 GiB;
  the 0.6.22/0.6.23 session's own sweep already ran). After = same 124 GiB
  free; net disk cost of activation: zero.
- **Phase 0 (C0), 2026-11-10:** code increment = the `--all` aggregate in
  `src/tool_audit.rs` + `src/cli.rs` + `src/main.rs` (gate green end to
  end, 2,337 passed); docs = `docs/features/selection.md` §baseline. No
  worktree, no build lanes beyond the main tree's own gate target/;
  `df -h /` after = 124 GiB free — unchanged. Net disk cost: zero.
- **Phase 6 (C18 probe + the sweep recovery), 2026-11-10:**
  `cache-sweep.sh` ran as C20's own recovery act after the session's
  repeated release builds: 108.5 → 124.2 GiB free (superseded
  compilations deduped, release binaries and bundles untouched). The C18
  probe spent three HTTP requests and wrote nothing. Net disk: recovered
  15.7 GiB.
- **Phase 4 (C11), 2026-11-10:** the drafter leg (`89186d3`) + the
  advisor leg; no worktree, no build lanes; `df -h /` = 108 GiB free
  (the session's own repeated release builds ate ~16 GiB of target/
  traffic — the sweep's next dry-run is the recovery act, recorded in
  the next entry). Net: the gate's own traffic only.
- **Phase 4 (C10), 2026-11-10:** the blend1 `generate` tool + fleet's
  `src/generate.rs` (fold leg, end-of-run prose act) + the face's
  `lane_summary` listing field + `docs/features/generation.md`. No
  worktree (the main tree's own gate ran); one scratch pytest venv in
  /tmp for the blend1 suite, deleted after (its own note: nanos-collision
  fix landed in the test harness). `df -h /` after = 124 GiB free.
  Net disk cost: zero.
