---
title: the prefix stays still — keeping the request head cacheable
date: 2026-11-16
status: draft
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

### I1 — the declaration set is frozen per run

- The set is computed **once at run start** and never mutated: the
  scope-filtered built-ins, `use_skill` when the run carries skills, the
  capability specs, and **every lane's tools** (the union), in a
  deterministic order. A lane that joins later changes nothing the model
  sees; `lane_tools` records what it *used*, never what it saw.
- *Budget guard:* the serialized union is measured; a test asserts it
  under a cap. If the union is too large, the fallback is a stable generic
  lane entry point with one-line per-lane docs — decided by the
  measurement, recorded on this plan, not assumed.
- *Met when:* the declaration set is byte-identical across every turn of a
  run, including the turn after a lane join; the run-start record lists
  the union; the join path has no mutation left in it.

### I2 — the volatile brief rides the tail

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

### I3 — a speaker switch costs the tail

- `turn_system_prompt` stays byte-stable; a mention that switches the
  speaker carries their overlay in the brief note for that turn instead of
  rewriting the system message.
- *Met when:* a mention switch leaves the head hash unchanged and the
  overlay still reaches that turn's request.

### I4 — the session's model is sticky

- Routing prefers the session's last serving model when it is viable
  (healthy, not demoted, no explicit pin elsewhere) and walks the chain
  only otherwise. A provider switch mid-conversation is a cold cache by
  construction.
- *Met when:* a healthy last-served model is chosen again; a failed one
  demotes and the walk proceeds; the per-session model-switch count is
  measured before and after.

### I5 — verified on the live stream

- Gate green, deployed, a few real sessions run.
- The cache query shows the move: **fresh-run** cache share from ~0 to
  roughly the head's size (the `prefix` record names that size), the
  fleet-wide token share from 8.7% upward, and per-session model switches
  down.
- If the head hash is identical run-to-run and the provider still reports
  zero, the remainder is provider/gateway behaviour (`opencode`,
  `ollama-cloud`) — filed in the ledger by name, not chased.

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
- Fold policy: a fold rewrites the head by design. One named miss per
  fold, accepted.
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

- [ ] C1: turn rows carry `cache_read`/`prompt`; `prefix` records name the
  head's hash and bytes per run; tests pin both.
- [ ] C2: the declaration set is byte-identical across a run, join or no
  join; the union is recorded and under its size cap.
- [ ] C3: two different runs assemble identical system bytes; the brief
  note carries the volatile half and is logged.
- [ ] C4: a mention switch leaves the head unchanged.
- [ ] C5: a session re-uses its last serving model unless it failed; the
  switch count drops.
- [ ] C6: gate green, deployed, and the live cache document shows the
  fresh-run and fleet-wide shares move; the provider-side remainder is
  named if it stays.
