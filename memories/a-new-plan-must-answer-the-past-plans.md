---
name: a-new-plan-must-answer-the-past-plans
description: "feedback 2026-11-02 — the owner asked whether a drafted plan is \"in line with what we have done in the past\"; a new plan that presents its shape as obvious must name the past plans it reopens and argue the difference, not just the fit"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's question, 2026-11-02, about the freshly drafted Tauri native-shell
plan: **"is the plan in line with what we have done in the past?"** The plan
presented the thin-shell shape as the obvious continuation; the audit found it
**in line in shape, against three past positions, and silent about all three**:

- **"One static binary" is the founding contract** — `CLAUDE.md:7` ("Slice 1
  shipped: one static binary…"), `docs/features/install.md:233` ("The release
  path is `cargo build --release` — one static binary"), and the install doc's
  per-target cross-build path. A second binary *and* a second
  build/bundling toolchain is a real departure.
- **The PWA plan refused a wrapper shell in exactly these words** —
  `docs/plans/2026-10-20-pwa-shell.md:66-68`: *"The Android shell stays out: a
  Kotlin/Gradle tree and a second toolchain for a crate whose contract is one
  static binary, while the PWA covers the home-screen need on both iOS and
  Android."* Tauri is that move for desktop.
- **The PWA was chosen to supply the "app" part** — the same plan's Context:
  *"What it lacks is the 'app' part: Add to Home Screen with a real icon, a
  launch that survives a cached-stale reload, and a readable page when the face
  is down."* And `docs/architecture/surfaces.md` has no desktop destination
  among its thirteen.

**Why:** fleet's plans are its institutional memory, and a plan is read against
them — not against the current session's reasoning. A shape that is genuinely
the house pattern (the thin shell **is** how design-canvas and the plugin host
both made a web thing feel native: served by the bridge, mounted as rail
surfaces) should be argued as the pattern; a shape that reopens a settled
choice must say so and name the difference that makes it arguable. Here the
difference is real but unstated: the PWA refusal was about **mobile**, where
the PWA already covers the need and Kotlin/Gradle is a foreign toolchain — on
desktop the need is uncovered and Tauri is Rust.

**How to apply:**
- Before drafting, grep `docs/plans/` for the plan's own subject words
  (`native`, `shell`, `wrapper`, `pwa`, `installable`, `toolchain`,
  `binary`) — the repo has ~100 plans and the relevant precedent is usually
  three of them, not one.
- Give the plan a **"Precedents this plan answers"** section that names each
  past position and argues the reopen. The owner's test is continuity, not
  novelty.
- Check the **founding contract** (`CLAUDE.md`, the install doc) as well as
  the topical plans: "one static binary" is stated in both places and is the
  sentence a second binary has to answer.
- Watch for **terminology collisions** with what the docs already call a
  thing: `docs/features/gui.md:1` names this "the browser face" and `:398`
  "two shells, two ports" already means the bot and main faces — a third
  meaning of "shell" will collide. Name the new thing in the docs' own words.
- The same audit is worth running on any plan that claims to be "in line" or
  "obvious": the claim is the thing to verify, not the shape.

**RESOLVED the same day — the owner corrected the framing and the plan was
rewritten whole.** The owner's answer: *"the refusal was to allow us for rapid
iteration on a working app, and pwa was the way to go, the next step in the
evolution of fleet is a native installable 'shell'"* — so the PWA refusal was
**not** a settled anti-shell ruling, it was a rapid-iteration choice, and the
native shell is the next step rather than a reversal. The plan was rewritten
(not patched) with a **"Precedents this plan answers"** table, the founding
self-containment intent stated as its own constraint, a 16-row component
inventory, and criteria C0–C10 whose weight is on the install (`d90123c`,
`104bc06`). The audit's recommendation is now **applied**; this memory survives
as the rule, not as an open debt.

**The correction's wider lesson:** when the owner says a past ruling was for
*velocity* rather than for *principle*, the plan must say so — the PWA plan's
own words ("for a crate whose contract is one static binary, while the PWA
covers the home-screen need") read as a permanent refusal until the owner
explains the intent behind them. **Ask what a past ruling was FOR before
arguing that it forbids the new thing.**

Related: [[plan-brainstorming-before-development]] (the plan-first rule this
rides), [[a-memory-index-line-is-not-the-plans-state]] (read the plan files,
not the summaries), [[tauri-native-shell-research-2026-11-02]] (the plan this
audit produced), [[contract-docs-are-the-third-gate]].
