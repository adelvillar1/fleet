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
- **(a) Validate the join against what the relay minted** — the
  relay already persists its sessions (`relay-sessions.json`, the
  KI-370 fix); the join refuses any `sid`/`hash` pair that does not
  match a session the relay itself minted, by name. The check is
  against the relay's own store — not a length rule, not a rate
  limit: those look like gating and validate nothing.
- **(b) Accept it** — the relay sits behind the origin's TLS and
  loopback boundary, and payloads are sealed end-to-end (KI-371), so
  an open join can neither read nor forge frames it does not hold
  the key for; its cost is a slot.

**Recommendation**: (a). The store already exists — the check is
wiring it into the one endpoint that never consults it. (b) is the
easy answer and it is wrong for a reason the row's own probe
demonstrated: `sid:"probe"` is not a slot-exhaustion curiosity, it
is proof that session identity at the join is currently
self-asserted.

### O2 — KI-378 (the open half): the task's send bound

**Evidence**: the goal writer refuses past 2,000 chars (fixed,
pinned); the task's send path has no named bound of its own — a
100 KB task is refused only by the wire's 64 KB cap, which is a
transport fact, not a product one. The row's own note calls the send
bound "triage for the owner (a one-shot spend, not a standing tax)".

**Options**:
- **(a) Make the honest cap speak the product's words** — the send
  path refuses at the cap the request actually has (the body limit
  the provider accepts), in product words naming the bound and why,
  instead of the wire's raw `a body of 156751 bytes; the cap is
  65536`. No new invented number: the composer already holds a
  131 KB draft (pass one's S1), so a smaller product ceiling would
  contradict the surface that composes the task.
- **(b) Invent a product ceiling** — copy the goal's pattern with
  some larger number. Coherent-looking, arbitrary, and it fights the
  composer's own capacity: the easy fix.
- **(c) Leave it** — the wire refuses; transport words stay.

**Recommendation**: (a). The goal's 2,000 is right FOR THE GOAL
because the goal is a standing prompt tax riding every run; the task
is one-shot input and its honest boundary is the transport's. The
fix is that boundary learning to speak, not a new number to obey —
and (b), my earlier framing of "a generous ceiling," was the easy
fix wearing the goal's pattern as a costume.

### O3 — KI-391: the severity vocabulary

**Evidence**: the preamble names six severities; the table carries
`note` (14), `warning` (4), `flake` (2), `decision` (2),
`environment` (2), `constraint` (1), `hang` (1), and four rows put
`**fixed**` in the severity cell beside a `closed` status.

**Options**:
- **(a) The rows normalize to the six** — the preamble's list is a
  taxonomy with meanings (blocker stops the build; defect is wrong
  behavior; gap is something expected missing; …), and every drift
  word is expressible in it: `flake`→`defect` (the issue text keeps
  "intermittent"), `hang`→`defect`, `decision`→`gap` (the status
  already says open, the next cell IS the ruling), `note`→per-row
  `risk`/`improvement`/`gap`, `environment`→`environmental`,
  `constraint`→per-row; the four `**fixed**` markers move to the
  status cell that already says `closed`. Then the checker polices
  severity beside status — the KI-322 pattern completed, and
  severity becomes machine-checkable again.
- **(b) The preamble gains the words in use** — one edit, but
  severity becomes a free tag and the six-word contract dissolves.

**Recommendation**: (a) — a flip from my first pass. "Gain the
words" was the easy fix: thirty row edits look expensive until you
notice they are one mechanical pass over one cell each, and what
they restore is a taxonomy that means something. A `flake` and a
`defect` are the same severity with different prose — the prose
already lives in the issue cell, where it can be read.

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

**Recommendation**: (a) — and the class rule that falls out of it is
the real fix: **no test in the gate asserts machine liveness**. A
contract we own gets a deterministic double (the repo already has
the stub-server pattern — the extract leg, the mock API); a true
liveness probe joins the on-demand lane like the live e2e. The easy
version of this call — blanket `#[ignore]` — buys green by deleting
signal, which is why (b) is not the recommendation even though it is
one flag shorter.

### O5 — KI-386: the corpus snapshot ignore rule

**Evidence**: `.gitignore`'s rule requires `-corpus-` before an
internally-dashed date; snapshots the runs themselves produce escape
in two shapes (the id precedes the token; the date loses its dashes),
so what gets committed is decided by name shape rather than by the
owner's ruling ("dated corpus state is runtime state").

**Options**:
- **(a) Widen to the substance AND stamp the shape where snapshots
  are born** — one glob over dated corpus state however spelled, and
  `seed_task`'s task strings name the state-fact's canonical
  spelling, so a run cannot invent a third shape (the escapes were
  born because nothing told the writer what to write). Untrack the
  already-tracked escapes — the owner's agreement required, the
  ruling's substance being what the rule encodes.
- **(b) Widen alone** — the easy fix: the glob stops splitting the
  class today, but spelling-policing is what produced two escape
  shapes in the first place; a third arrives whenever a run gets
  creative.
- **(c) Give state its own home** — snapshots move to a dedicated
  directory outside the fact corpus: no spelling to police at all,
  the structurally right answer, and the escalation if (a) ever
  recurs.

**Recommendation**: (a), with (c) named as the standing escalation.
Widening alone (my first pass's whole recommendation) was the easy
fix — it treats the symptom's spelling while the writer stays free
to mint new ones. Untracking is reversible (the files stay in git
history).

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

**Recommendation**: (a), with one build-time honesty check: the fork
presupposes the task-start snapshot is an accident of load-once
rather than a design for stable mid-task recall. The build reads
that seam first — if the snapshot is deliberate (recall stability
within a run is the point), the ruling flips to (b) and the wording
becomes the fix; if it is load-once convenience, (a) is right: the
tool's own words are the contract the caller reads, making them true
beats retraining every caller on a subtler promise, and KI-232
already paid this exact seam. The write-through must also evict the
superseded fact's vector — the row's other half — or the fix is
half-done.

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

## Rulings — 2026-11-01, the owner

The owner ruled the revised slate as recommended, with the O7 flip
(their push: the bite count was already paid) and the O2/O3/O5
revisions from the right-vs-easy re-audit standing as the
recommendations. The rulings:

- **O1 (a)** — the relay's join validates `sid`/`hash` against the
  relay's own minted-session store.
- **O2 (a)** — the send path's refusal speaks the product's words at
  the cap the request actually has; no invented ceiling.
- **O3 (a)** — the rows normalize to the six documented severities;
  the checker polices severity beside status.
- **O4 (a)** — hermetic doubles for contracts; no gate test asserts
  machine liveness.
- **O5 (a)** — widen the glob to the substance, stamp the canonical
  shape at the writer (`seed_task`), untrack the escapes (agreement
  granted by this ruling).
- **O6 (a)** — write-through index with eviction, after the build
  reads the snapshot seam; if the snapshot is deliberate, the ruling
  flips to the wording fix and this plan records it.
- **O7 (a)** — build the lane-call verb.

## Acceptance criteria

- [x] **O1–O7 each carry the owner's ruling**, recorded in the
      Rulings section above (2026-11-01) and in each call's row as
      its follow-through lands.
- [ ] **Each ruling's follow-through lands** as its own increment
      with its pin, in the order the rows' own severity puts them
      (crashers and data-loss first — none here are; O1 and O2
      before the cosmetic calls). *Landed so far: **O1** — the join
      gate (relay `RELAY_JOIN_SECRET` / face `FLEET_RELAY_JOIN_SECRET`
      and `FLEET_BOT_RELAY_JOIN_SECRET`), fail-closed both sides,
      proofless-join pinned against the real relay, KI-367 closed;
      the OpenShip env + redeploy is the owner's deployment step,
      named in the row.*
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
