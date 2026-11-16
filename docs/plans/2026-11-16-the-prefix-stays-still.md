---
title: the prefix stays still — keeping the request head cacheable
date: 2026-11-16
status: completed
---

# The prefix stays still

The cache instrument measured the problem (2026-11-15/16, live stream):
187 route calls carry cache accounting, 34 hit — **8.7% of prompt tokens**.
The shape is not size and not expiry: hit rate is flat across gaps
(<2m 41%, <10m 15% on the 109-call bulk, <1h 32%, ≥6h 0%), and **5 of 5
within-run continuations missed**. What moves is the prefix.

Prompt caching is an exact-prefix match on the **rendered prompt**. Every
provider fleet speaks to templates the system message and the tool block
ahead of the messages (ChatML-shaped), so the head is `system → tools →
messages` in the token stream — a byte change in either of the first two
invalidates everything after it. (The JSON body's field order is not the
mechanism; `ChatRequest` serializes `messages` before `tools`, and the
provider's template re-orders it. The plan therefore treats the two as
one head and fingerprints them together.)

Two parts of that head are dynamic today:

- `compose_system_prompt` (`src/agent.rs:2215-2300`) appends `## This
  request`, `## Goal`, `## Skills`, `## Memory`, the role overlay, plan
  mode and the execution line after the constant block — differing per run
  and per session.
- `declaration_set` (`src/agent.rs:2476-2545`) appends a lane's tools as
  it joins the run. 38% of the runs that log declarations change the set
  mid-run, and there are 474 `lane_tools` records across 200 sessions.
  Tools precede messages, so every join invalidates every message behind
  it — the transcript is re-read at full price.

And the effect is unmeasurable today: `self.usage` accumulates prompt and
completion only (`src/agent.rs:3506`, `:4508`), so turn rows carry no
cache field (0 of 2,665), and nothing records whether the head was
byte-identical when a miss happened.

## The increments

### I0 — the instrument first

- `self.usage` sums `cache_read_tokens` (Option-aware: an absent value
  stays absent) at both accumulation sites, so the **turn row** carries the
  run's cache read beside its prompt/completion.
- A `prefix` session record per run: the sha256 and byte length of the
  assembled head — the `system` message's bytes and the serialized `tools`
  block. This is the line that separates "the head changed" from "the
  provider dropped it".
- *Met when:* a test drives two turns and asserts the run usage sums the
  cache half; the prefix record is written once per run, hashes
  deterministically, and is reconstructable (hard rule 5).

### I1 — the declaration set is frozen per run (delivered, re-scoped by measurement)

The plan guessed the churn came from lane joins. The logs say otherwise:
**535 declaration records, 0 with a withheld tool** — `pre_join` already
declares every reachable lane before the first request, and the tier
never withholds. So the set was never short; the **order** moved:
`profile_order` re-sorted the native declarations by the workload the
router judged, and a workload change rewrote the bytes ahead of the
messages — the head, from the cache's point of view.

Delivered instead, same intent:

- the declarations keep the compiled order, always
  (`src/agent.rs`, `declarations`); `profile_order` is gone;
- the profile's steering rides the brief as words — `workload_line`
  names the three tools the workload reaches for first, and every
  declared tool stays available (`## Order of reach`);
- a test pins that the declarations are identical whatever the workload,
  and that the line still names the workload.

*Met when:* the declarations are byte-identical across workloads (test),
the profile still steers (test), and the head's fingerprint records what
first request carried (I0).

### I2 — the volatile brief rides the tail (dropped — see "Dropped")

The cache stops at the first differing byte, not at the first differing
message: what the provider can hold across runs is the **leading
constant**, and `head_prompt()` now produces exactly those bytes first
(`default_prompt()` + nothing else). Moving goal/memory/skills/overlay to
the tail would lengthen the cacheable span only by the *sections between*
the constant and the first varying one — which is nothing, because the
goal section is the first varying section. So the tail move buys no
prefix the current order does not already have, while it does change what
the model weighs (recency) on every run. Dropped after I6: the constant head alone moved the share, and the move
would not have added a byte.

### I2 (original text, kept for the record) — the volatile brief rides the tail

- The system message becomes `SYSTEM_PROMPT + REPORTING_CONTRACT + catalog`
  and nothing else. Goal, skills, memory block, plan mode, execution line,
  gui prompt and file access move into one **brief** system note placed
  after the transcript — the pattern the health and curation notes already
  use (`src/agent.rs:3400-3418`, with the comment explaining why).
- The brief is logged as its own record, so the request stays
  reconstructable.
- *Met when:* two runs with different goals, memory and skills produce
  **identical system bytes** (a test asserts the equality), the brief's
  text is present in the request's messages, and the existing prompt
  assertions (`prompt.starts_with(SYSTEM_PROMPT)` and friends) stay green.

### I3 — a speaker switch costs the tail (dropped — see "Dropped")

The overlay lives in the variable section, *after* the constant head, so
a mention switch changes bytes the cache was never going to hold across
runs anyway. It costs nothing the ordering does not already cost.

- `turn_system_prompt` stays byte-stable; a mention that switches the
  speaker carries their overlay in the brief note for that turn instead of
  rewriting the system message.
- *Met when:* a mention switch leaves the head hash unchanged and the
  overlay still reaches that turn's request.

### I4 — the session's model is sticky (dropped here — now KI-548)

Routing is prepared **once per run** (`route::prepare_with_pin` at run
start; a mid-run pin is the only re-route), so a run cannot change models
between turns — the cold-cache-by-provider-switch case is *between the
runs of one session*, and making that sticky changes routing policy. That
needs its own measurement (how many sessions re-judge to a different
model) and its own slice, not a mechanical edit inside this one.

- Routing prefers the session's last serving model when it is viable
  (healthy, not demoted, no explicit pin elsewhere) and walks the chain
  only otherwise. A provider switch mid-conversation is a cold cache by
  construction.
- *Met when:* a healthy last-served model is chosen again; a failed one
  demotes and the walk proceeds; the per-session model-switch count is
  measured before and after.

### I5 — the fold fires later (85%)

The owner's addition (2026-11-16): compaction at **85% of the context
window**, not the three fifths it used to take.

- `compaction::budget_for` returns `window × 85%` for a declared window;
  a window nobody declared keeps the floor (`COMPACTION_TOKEN_BUDGET =
  100_000`, ~80% of DeepSeek's 128k). The old `window − 32k` reserve is
  gone with the fraction it existed to protect — at 85% it would have
  capped every window under ~213k *below* the ruling (128k would fold at
  96k, i.e. 75%).
- The 15% left is the headroom: the request that crosses the line, the
  fold's own summary request and its answer.
- Named behaviour change: a *declared small* window now folds at its own
  85% instead of being dragged down to the floor (65,536 → 55,705, was
  33,536). The floor is the undeclared-window policy, not a ceiling.
- *Met when:* the budget tests pin 85% across the roster's windows, the
  bridge's gauge reads the same number, and no caller re-derives a
  fraction.

### I6 — verified on the live stream (measured 2026-11-16)

Two pairs of tiny runs with the new binary, each pair carrying a different
task, so a cache read could only come from the head:

| runs | head | provider's cache read |
|---|---|---|
| 2 × `opencode/deepseek-v4.1-flash` | `690708de9c16144e` / 31,271 bytes both | **0**, then **0** |
| 2 × `zai-coding-plan/glm-4.5` | same head, both | **0**, then **0** |
| 2 × `deepseek/deepseek-flash` | same head, both | **0**, then **5,248 of 12,833** (41%) |

The harness side is proven: an identical 31,271-byte head (`head_bytes`,
≈7.8k tokens of a ~12.8k first call) across different tasks. Direct
DeepSeek cached it on the second run; **the gateways report zero** — the
provider-side remainder this plan said to name rather than chase, now
filed as KI-547. The turn row also carries the cache half for the first
time (`cache 0` / `cache 5248`), so the panel can read per-run.

### I6 (original text, kept for the record) — verified on the live stream

- Gate green, deployed, a few real sessions run.
- The cache query shows the move: **fresh-run** cache share from ~0 to
  roughly the head's size (the `prefix` record names that size), the
  fleet-wide token share from 8.7% upward, and per-session model switches
  down.
- If the head hash is identical run-to-run and the provider still reports
  zero, the remainder is provider/gateway behaviour (`opencode`,
  `ollama-cloud`) — filed in the ledger by name, not chased.

## Dropped (not deferred)

The owner's ruling (2026-11-16): a plan does not stay active for no
reason. Three criteria were removed rather than parked, because nothing
pending can decide them:

- **C3 — the brief rides the tail.** Dropped: the provider's prefix cache
  stops at the first differing byte. The constant head leads and the first
  varying section (`## This request` / `## Goal`) follows it immediately,
  so the tail move lengthens the cacheable span by zero bytes — while it
  changes what the model weighs on every run. I6 measured the head
  identical across runs and direct DeepSeek cached 5,248 of 12,833 tokens
  from it, which is the whole prize.
- **C4 — a speaker switch costs the tail.** Dropped for the same reason:
  the overlay already sits after the constant head, so a mention switch
  never touches the cacheable prefix. There is nothing to pay.
- **C5 — sticky cross-run routing.** Dropped from this plan and filed as
  **KI-548**: routing is prepared once per run, so this is a cross-run
  policy with its own measurement and its own risks — it does not belong
  to a cache plan, and the ledger is where it is tracked.

What would reopen the cache plan: a provider whose cache key includes the
messages (none fleet speaks to does), or a measured regression in the
head after a declarations change — both are the instrument's job to
report, not this plan's to predict.

## The shape of the win

The constant head is the system prompt, the reporting contract, the
component catalog and the tool block — plausibly 4–6k tokens against the
~10.5k first call the stream shows. Stabilising it should make a *fresh*
call read roughly that much from cache before any transcript reuse, and
I1/I2 together make the head identical across sessions, which is what
turns the cache from per-session luck into a standing asset.

## Non-goals

- Prices: fleet holds no table; the measure stays rate and volume.
- Gateway retention and sharding: not ours to fix — separated by the
  prefix fingerprint in I0.
- Fold *policy beyond I5*: a fold still rewrites the head by design —
  one named miss per fold. I6 reduces how many folds happen; it does not
  make a fold cheap.
- The cache panel's layout: shipped, unchanged.

## Risks

- Moving the brief out of the system message changes what the model
  weighs. Mitigation: it stays visible every turn at the tail, the
  existing suite stays green, and I5 runs real tasks before any claim.
- Freezing the tools grows the block; the budget guard in I1 bounds it and
  names the fallback.
- A gateway can ignore the prefix contract entirely; I0 makes that
  visible instead of guessed.

## Acceptance criteria

- [x] C1: the run's usage sums `cache_read_tokens` (both accumulator
  sites, Option-aware, pinned by a test), so the turn row carries the
  run's cache read; the `Declarations` record carries the head's
  fingerprint — `head_sha256` (16 hex) and `head_bytes` over the
  constant head plus the serialized declarations — pinned by a test.
  (The plan said "a `prefix` record"; the fingerprint rides the existing
  record instead of a new log type, one write, one place.)
- [x] C2: the declarations are byte-identical whatever workload the
  router judged — the per-workload reorder is gone and its steering
  rides the brief's words; pinned by a test.
- [x] C7: compaction fires at 85% of a declared window (the floor for an
  undeclared one), with the budget tests and the bridge gauge updated.
- [x] C6: gate green, deployed, and measured live: an identical head
  reads 5,248 of 12,833 prompt tokens from cache on direct DeepSeek (a
  fresh run whose task differed), while opencode and zai report zero —
  the remainder is named in KI-547, not chased.
