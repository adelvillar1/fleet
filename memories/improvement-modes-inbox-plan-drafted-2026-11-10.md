---
name: improvement-modes-inbox-plan-drafted-2026-11-10
description: "project 2026-11-10 — the improvement-modes + inbox plan is DRAFTED at 13 criteria (docs/plans/2026-11-10-improvement-modes-inbox.md, uncommitted, awaiting owner review): modes + inbox + evaluator wall, with the Strategist dive's filing-path trio folded in as C10–C12 (content-query lessons with cross-target inspiration, refusals forward, analysis note); re-gated 13/13 after the drex,jev re-pin"
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

2026-11-10, the owner steered the inbox work into a bigger contract:
*"improvement modes like the composer has ask, autonomous, and full …
autonomous (non-destructive), full. depending on the mode decisions go
to the inbox."* The plan is **DRAFTED, uncommitted, status draft** at
`docs/plans/2026-11-10-improvement-modes-inbox.md` (the slug offered in
[[direction-decisions-in-the-inbox-2026-11-10]] was superseded by this
broader name); presented for review, **not activated, not committed** —
the owner's "looks good" flips it to active.

**The owner's ruling, asked and answered this session:** in `full`
mode destructive changes (retire skill, forget fact, config-remove)
**apply** — the mode selection IS the standing approval, undo the
backstop — but **confirmed-by memory facts still wait** under every
mode (provenance is the substrate's permission model, full's
always-ask class). Ask = today exactly (everything waits);
autonomous = `!destructive()` applies itself.

**The shape (one mode, one seam, two inbox classes):**

- `improve.mode` joins the one `SETTINGS` table (`Kind::Choice`, ask
  default) + a face select on the decisions-queue card
  (`skillsDecisions` `app.js:8596`, composer-runbar idiom).
- One attempt seam in `improve.rs`: filing and mode-writes both drive
  it; a **capability pre-check** (source: the eval probe's `target/`
  rule `src/eval.rs:1007`, cargo reachable) leaves a proposal
  **pending, never burned**, with the reason named — only a real gate
  failure refuses and stamps `last_refusal`. Reconcile on mode write
  (CLI synchronous, bridge `spawn_blocking`), bounded sequential.
- Two new `owner_inbox` classes: **`improve`** rows = what still needs
  the owner *under the current mode* (signature
  `name|target|status|last_refusal`, KI-497), skill targets approve in
  place via `POST /api/skill`, others open `#/skills/needs-you`; and
  **`demand`** = the deep dive's direction row — ONE row, top
  Flat/Worsening class ≥ DEMAND_MIN_ROWS, signature
  `family|class|trend|rate-decile`, mode-independent (direction stays
  the owner's), TTL-cached 14-day collect in `spawn_blocking`. Its
  route `#/calibration/demand` is a new computed section, which lands
  the self-improvement plan's unbuilt UX promise.
- **The evaluator wall (C7)** = the dive's areas 1–2 as refusals in
  `stage_patch`: no patch to `src/eval.rs`/mock bins, no
  path≠declared-target, for **every mode including `--approve`**;
  `fleet eval --promote` stays the corpus's one door. Under
  unattended apply this is the structural reviewer that replaces the
  owner's read — the reason the wall rides THIS plan, not a later one.

**Gate status — FIXED and re-gated same session (the full arc in
[[jev-chain-failure-diagnosed-2026-11-10]]):** the `chain: jev`
failures were the by_task pins + TypeSafe HTTP 402; the owner chose B
and `plan_gate`/`plan_surface_map`/`plan_deps` were re-pinned to
`drex,jev`. Two structural defects the reruns then exposed (both fixed
in the plan, mechanism recorded in [[plan-gate-drafting-mechanics]]):
the 5 use-case checkbox boxes were being counted as C0–C4 (de-boxed to
`- **A:**`) and the monolithic Approach + wrapped criteria starved
coverage (Approach split into criterion-anchored `###` subsections,
every criterion one physical line). After that: **plan-gate coverage
10/10** (dispositions: template-sections `waived`, the Reconcile misread
`overridden`), plan-surface suggested order = checkbox order,
arch-gate PASS.

**The fold-in (owner: "can we fold your recommendations into the
previous plan?"):** three of the four Strategist areas landed in the
plan as **C10–C12** (C0–C9 byte-identical, identity preserved) —
**C10** content-query lessons (BM25 read-time over `<home>/lessons/`:
one relevance hit + one deliberate-drift cross-target inspiration hit
in the filing confirmation; nothing written under `memories/`), **C11**
the target's recent refusals with `last_refusal` reasons listed at
filing (absorbs ERNIE area 3 too — same gap argued by two dives), **C12**
"no analysis on record" when no `analyze` row within
`ANALYZE_WINDOW_DAYS` (observation, never a refusal). **Strategist
area 2 (card numbers) deliberately OUT** — it waits on ERNIE area 5's
settlement readback (Out of scope says so); the plan's C10 only gives
those cards the retrieval they'd be ranked by. Re-gate after the fold:
**coverage 13/13**, same four template-section flags waived again,
surface's want-to-move-C9-late overruled on the identity rule — but
`disposition plan-surface` reported **no matching gate event** (surface
dispositions don't record; the overrule lives in the plan's own
order-is-identity header and the conversation). Use case F added.

**ERNNIE area 5 (settlement readback) asked-about, explained, and
still OUT — NOT a technical constraint** (owner: "why are we leaving
ernnie area 5 out? is it a technical constraint?"): every instrument
already exists (`grade_judgments` `src/calib.rs:1085`, demand trends,
`propose` rows) — the exclusion is three judgment calls: (1) it lives
on the outcome side of the lifecycle (N-days-after grading) while this
plan edits the filing/apply path, and the plan's own out-of-scope bans
exactly that resident pass; (2) two open design questions the dive
itself left unsettled — the **target→demand-class mapping** (`src/route.rs`
plausibly maps to `route/`, `src/tools.rs` maps to nothing; a structured
proposal field is the honest fix but is the known whole-write-path
shape) and the **trigger** (CLI vs beat vs reconcile-piggyback); (3)
contract hygiene — a gated plan must build without mid-build decisions.
**Recommended sequencing:** its own settlement plan next, which closes
ERNNIE area 5 *and* Strategist area 2 (card numbers) in one contract —
this plan's C10 only gives those cards the retrieval they'd be ranked
by. Folding it in here would need the owner to settle the mapping
question first (declared field = recommended; derive-from-text =
fragile and judgment-shaped).

**Why:** the mode is how the owner selects their own appetite for
self-modification, and the inbox must carry exactly what each mode
leaves undecided — no more (buried decisions) and no less (silent
auto-applies); the confirmation's three reads make filing time start
from the WHOLE loop (applied + failed + un-analyzed), not its applied
half.
**How to apply:** next session starts at the plan file; when the
owner approves, flip status to active and build **C0→C12** (settings
entry, the seam + wall, reconcile, the two inbox classes, the
calibration section, the confirmation's three reads, harness).
Related: [[ernie-evolve-deep-dive-2026-11-10]],
[[strategist-deep-dive-2026-11-10]],
[[direction-decisions-in-the-inbox-2026-11-10]],
[[owner-inbox-plan-drafted]].
