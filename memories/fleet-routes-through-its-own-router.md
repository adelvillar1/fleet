---
name: fleet-routes-through-its-own-router
description: "Fleet's router is the in-process route::select it owns — not a process, not a port, not a service; the agnostic-router-kit is the precursor Fleet superseded, not a dependency, and its roster entry is gone entirely (owner ruling 2026-10-16)"
type: project
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

Owner ruling (2026-10-16, after five corrections in one session): **Fleet is a
self-contained app with a router Fleet itself owns.** That router is the
in-process `route::select` in `src/route.rs` — sys1 asks for a workload
verdict, the workload preference table picks the provider, and failover walks
the declared chain. It is not a process, not a port, not a service, and
nothing in Fleet needs a router to be up: not to select a route, not to show
the router board.

- **The `agnostic-router-kit` is the precursor to Fleet, not a dependency.**
  Its entry is gone from the compiled-in roster entirely (7 candidates now,
  `HEAD_ID` = `zai-coding-plan`). The owner's words: "the precursor is not the
  solution." The kit deployment on the machine stays running — the owner ruled
  it stays, and it is not Fleet's business to touch. Its two local converters
  (`codebuddy` :8787, `opencodex` :10100) stay out of the roster, with
  `fleet settings provider.add:` as the seam.
- **Port 8300 belongs to the zcode harness's own router**, not to the kit's
  operator API. Fleet never routes through it and never reads its config. The
  error message that started the session ("fleet cannot find the router")
  named the wrong thing — Fleet was never supposed to find a router.
- **The router board dials nothing.** Its three reads are named in its own doc
  section: `boards::read_router` filters the calibration day's own route rows
  through the existing `calib::read_day` reader. The tier chains are derived
  from `route::default_preferences()` at draw time, so a fixture can never
  disagree with the table about what a workload's own chain is — the old
  hand-written fixture did exactly that (it claimed `standard_code`'s primary
  was `step-5-preview` when the table says `step-3.7-flash`).
- **`BoardData` boxes all three variants** — a snapshot is a message carried
  over the update channel, not a value the caller holds, so without the boxes
  the enum would be the brain snapshot's 616 bytes whichever board is actually
  carried. Shrinking `RouterSnapshot` to 360 had widened the gap past clippy's
  `large_enum_variant` threshold, so the boxes were already owed.
- **blend1 points at its own lane port (8420)**, not the kit's 8300.
- **DeepSeek is the payg reserve and serves only by an owner pin** in
  `<FLEET_HOME>/routing.json` — declared in no workload's chain, so the payg
  guard reports exactly that (KI-214 records the inverted guard it replaced).
  Route rows carry no token counts, so the board's token line was removed
  rather than estimated.

- **The router has a settings surface now (adfd353, 2026-10-20)**, and it is
  honest about what it does not have: fleet's router has **no
  mixture-of-agents, capability chains, or picker profiles** — those belong to
  the zcode kit, and the owner's screenshot of the kit's page was explicitly
  *not* a demand to fake them. The page exposes the spend guard
  (`gates.allow_api_models`), the tier chains, the judge thresholds, the
  default workload, and the pin, all through `settings.rs`. Its `resolved now`
  column is the first candidate admissible under the gate, enabled, and keyed
  (`route::admissible` is `pub(crate)` for exactly this), and an **empty pin
  now reads as no pin** in the loader itself, not only in the TUI's status bar.

Docs carrying the ruling:
`docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` (C1–C8, with
the superseded design recorded rather than deleted),
`docs/architecture/ecosystem.md` (supersession banner on the routing rows),
the 2026-10-15 deep dive's banner, and the recap
`docs/recaps/2026-10-16-fleet-routes-through-its-own-router.md`.

Related: [[fleet-roster-migrated-from-router-kit]] (the migration that put
every key in Fleet's own home and closed KI-047, and enabled this ruling),
[[fleet-dogfooding-viability-verdict]], [[owner-stated-facts-are-authoritative]].
