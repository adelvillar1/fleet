---
status: active
created: 2026-11-01
updated: 2026-11-01
slug: open-calls
---

# Plan: the open calls — seven decisions the ledger is waiting on

## Context

Seven open rows are not waiting on code: they are waiting on the
owner. Each names a fork — two honest directions, both defensible —
and the row cannot close until one is chosen, because the follow-through
is different work under each. This plan is the decision record: each
call carries its evidence, its options, and a recommendation, so a
ruling is one line per call and the follow-through lands as its own
pinned increment.

Two neighbouring rows (KI-249, KI-252) are also windows, but they
already have a contract — the active embed-bounds plan
(`2026-10-24-embed-bounds-from-measured-tail.md`) — and are listed
here only so the reader knows they were considered and deliberately
not re-ruled.

## The calls

### O1 — KI-367: the relay's join endpoint is ungated

**Evidence**: a deployment probe joined the relay with
`{"sid":"probe","hash":"x"}` and was served — any caller at the
origin can open a session slot.

**Options**:
- **(a) Gate the join** — require the pairing secret (or a short
  minted token) at the join, refusing an unauthenticated join by
  name. The paired-device flow already carries a secret; the join
  is the one hole.
- **(b) Accept it** — the relay sits behind the origin's TLS and
  loopback boundary; an open join costs a slot, not data.

**Recommendation**: (a). The relay is a door with a lock everywhere
except this one hinge; the pairing secret already exists to be
checked here.

### O2 — KI-378 (the open half): the task's send bound

**Evidence**: the goal writer refuses past 2,000 chars (fixed,
pinned); the task's send path has no named bound of its own — a
100 KB task is refused only by the wire's 64 KB cap, which is a
transport fact, not a product one. The row's own note calls the send
bound "triage for the owner (a one-shot spend, not a standing tax)".

**Options**:
- **(a) Bound the send** — a named ceiling on the task at the send
  path (the composer's own scale, like the goal's 2,000), the refusal
  naming the bound and why.
- **(b) Leave it** — the wire cap already refuses the absurd; a
  one-shot giant task is the owner's to spend.

**Recommendation**: (a), with a generous ceiling. The goal's bound
already set the pattern (a named refusal beats a transport error),
and the composer's own draft scale is the honest measure of what a
task should carry.

### O3 — KI-391: the severity vocabulary

**Evidence**: the preamble names six severities; the table carries
`note` (14), `warning` (4), `flake` (2), `decision` (2),
`environment` (2), `constraint` (1), `hang` (1), and four rows put
`**fixed**` in the severity cell beside a `closed` status.

**Options**:
- **(a) The preamble gains the words in use** — document `note`,
  `warning`, `flake`, `decision`, `constraint`, `hang` (and the
  `environment`→`environmental` spelling), move the four `**fixed**`
  markers out of the severity cell (their status already says
  `closed`), and have the checker police severity beside status.
- **(b) The rows normalize** — rewrite ~30 rows to the six documented
  words.

**Recommendation**: (a). The words in use carry distinctions the six
do not (`flake` vs `defect`, `note` vs `gap`); documenting reality
costs one preamble edit and one checker rule, where normalizing costs
thirty row rewrites and flattens real signal.

### O4 — KI-328: the live-endpoint test's flake class

**Evidence**: a live-endpoint test reds under parallel load; the
instance is recorded, the class undecided.

**Options**:
- **(a) Hermeticize** — the contract the test asserts is ours, so
  drive it against a stub; the live probe (if any survives) runs
  `#[ignore]`d with an on-demand flag, like the live e2e.
- **(b) `#[ignore]` the class** — every live-endpoint test joins the
  on-demand set; nothing hermetic is built.
- **(c) Delete** — the assertion no longer earns its flake.

**Recommendation**: (a). The repo already has the stub-server pattern
(the extract leg, the mock API); a flaky test that asserts a contract
we own should own a deterministic double, and true liveness checks
belong to the on-demand lane.

### O5 — KI-386: the corpus snapshot ignore rule

**Evidence**: `.gitignore`'s rule requires `-corpus-` before an
internally-dashed date; snapshots the runs themselves produce escape
in two shapes (the id precedes the token; the date loses its dashes),
so what gets committed is decided by name shape rather than by the
owner's ruling ("dated corpus state is runtime state").

**Options**:
- **(a) Widen the rule** — one glob over dated corpus state however
  spelled, and untrack the already-tracked escapes (the owner's
  agreement required — the ruling's substance, not its spelling, is
  what the rule encodes).
- **(b) Normalize the names** — the runs write snapshots under the
  rule's spelling; the tracked escapes are re-spelled and stay
  tracked history.

**Recommendation**: (a). The ruling is about substance — dated state
is state — and a rule that splits the class by spelling is deciding
by accident. Untracking is reversible (the files stay in git history).

### O6 — KI-332: what `remember`'s promise means

**Evidence**: `remember` documents "one file, read back on later
turns", but the corpus and its vector index are a task-start snapshot
— a fact written mid-task is not a recall candidate until the next
session, and the superseded fact keeps the vector entry the new one
never gets.

**Options**:
- **(a) Write-time index entry** — `remember` knows the file it just
  wrote; give it the same immediate-index treatment the nomenclature
  cache got (KI-232's argument), so the promise reads true.
- **(b) Wording** — the promise says "later session", and the
  mid-task gap is documented rather than closed.

**Recommendation**: (a). The tool's own words are the contract the
caller reads; making them true is cheaper than retraining every
caller on a subtler promise, and the precedent (KI-232) already paid
this exact seam.

### O7 — KI-369: a durable spawn route for lane jobs

**Evidence**: the hand-driven stdio JSON-RPC path works — and keeps
paying: KI-221/KI-224 (the job could not execute as written, "fixed"
by pointing the task string at a hand-built driver), every 10-28
catch-up hand-driving the same session, and KI-387 itself — the first
run through the spawn route took a Content-Length-framed driver that
dead-read 90s, a relaunched child killed as a leftover, a bytes/str
TypeError, and a fourth attempt before it ran clean. The route has
bitten more than twice; the hand path re-derives a contract the repo
already implements in `src/mcp.rs` on every nightly run.

**Options**:
- **(a) Build it** — a one-shot lane-call verb on the existing
  surface: `fleet mcp call <server> <tool> '<args-json>'` spawns the
  lane through the registry's launch spec, drives initialize (the
  300s cold window) and the `tools/call` over the client that already
  speaks the framing, prints the reply, tears down, and exits with
  the outcome. Job task strings then name one command; KI-387's
  contract lives in the code that runs it instead of a docstring.
- **(b) Keep the hand-driven path** — C-plan W3 writes the transport
  where drivers read it; the driver stays a runbook step.

**Recommendation**: (a). The bite count is the ledger's own — three
rows plus today's KI-387 — and the increment is wiring around an
existing client, not a new subsystem. W3's contract work stays
valuable (the docstring and broken-pipe fixes) but stops being the
load-bearing answer.

## Acceptance criteria

- [ ] **O1–O7 each carry the owner's ruling**, recorded in the
      call's own row (the status or next-action cell names the
      choice and its date) — one line per call is enough.
- [ ] **Each ruling's follow-through lands** as its own increment
      with its pin, in the order the rows' own severity puts them
      (crashers and data-loss first — none here are; O1 and O2
      before the cosmetic calls).
- [ ] **KI-249 / KI-252 stay with their plan** — the embed-bounds
      plan's window criterion is their gate; nothing here re-opens
      them.
- [ ] **Close-out.** The full gate green by exit code; the ledger
      checker green; every called row closed or explicitly deferred
      with the ruling recorded.

## Files to be touched

Each follow-through lands where its row lives (O1: the relay's join
handler; O2: the send path's bound; O3: the preamble + the checker;
O4: the flake's test + its stub; O5: `.gitignore` + the tracked
escapes; O6: `remember`'s index write; O7: the lane-call verb in
`src/cli.rs`/`src/main.rs` over the existing MCP client, and the job
task strings naming it).
This plan file records the rulings.

## Out of scope

- **Deciding here** — this plan's job is to make the decisions
  one-line each; the work they unlock is the follow-throughs.

## Verification

The gate per follow-through; the ledger checker after every row
update; the rulings themselves verified by reading each row's cell
back.

## Linked artifacts

- `docs/known-issues.md` — KI-367, KI-378, KI-391, KI-328, KI-386,
  KI-332, KI-369 (and the considered-and-deferred KI-249, KI-252)
- `docs/plans/2026-10-24-embed-bounds-from-measured-tail.md` — the
  window rows' own contract
