---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: consumer-wiring-guard
---

# A capability ships with its consumer, or says why it has none

Thirteen ledger rows share one shape: the feature exists and nothing in
the product can reach it. The vault's largest consumer went unwired while
the feature passed its own tests; `deck::to_pptx` writes a real
PowerPoint file no command, tool, or route calls; `Capability::Observe`
was unreachable because every pairing path hardcoded `Task`. Unwired
capability is worse than absent capability — it reads as done.

## Context

- The three product surfaces where a missing consumer is a defect: CLI
  verbs (`src/cli.rs`), the face's routes (`src/gui/bridge.rs` /
  `src/gui/server.rs`), and the agent's declared tools (`src/tools.rs`).
- A general "every `pub fn` has a caller" is too strict — the crate has
  internals. The tractable rule is per surface: every verb, route, and
  tool has a reachable path from a documented entry point, or carries a
  named annotation saying it does not.
- The plan template already exists (`docs/plans/README.md`); adding one
  required line is the cheapest half of the fix.

## Approach

1. Add a **Consumer** line to the plan template in `docs/plans/README.md`:
   `Consumer: <entry point> — proven by <test or surface>`. A plan whose
   feature has no consumer states that and why.
2. Add three census tests (same `include_str!` shape as the structure
   guards):
   - every `Command` variant has a dispatch arm and appears in `fleet
     help` (the ledger already caught two misses here: `channels` absent
     from help, `cleanup` unlisted);
   - every bridge route registered in the router table has a handler and
     a caller in `app.js` (a light grep for the path string);
   - every tool declared in `declarations()` has an executor arm.
   Each reds when a new surface entry has no counterpart.
3. Sweep the thirteen ledger rows' subjects and write down which are
   still unwired (the vault's router leg and `deck::to_pptx` are the two
   the ledger names explicitly). Wiring them is out of scope; naming them
   is this plan's deliverable.
4. Red-proof each census test by adding a dummy verb/route/tool and
   confirming the failure names it.

## Acceptance Criteria

- [ ] The plan template carries the Consumer line, and
      `docs/plans/README.md` requires it for `active` plans touching a
      product surface.
- [ ] Three census tests exist and red on a dummy verb / route / tool;
      each failure names the missing counterpart.
- [ ] The sweep names every still-unwired capability from the thirteen
      rows, with a ledger row for each (or a close note if already
      wired).
- [ ] No existing verb, route, or tool reds falsely — the census is run
      against the whole surface and passes.
- [ ] The full gate exits 0.

## Files to be touched

- `docs/plans/README.md` — the template line
- `src/cli.rs` and its tests — the verb census
- `src/gui/server.rs` and its tests — the route census
- `src/tools.rs` and its tests — the tool census
- `docs/known-issues.md` — rows for anything still unwired
- `docs/architecture/structure-guards.md` — the new guard class

## Out of scope

- Wiring `deck::to_pptx` or the vault's router leg (each is its own
  decision; this plan only makes them visible).
- A dead-code lint over the whole crate.

## Verification

1. Each census test alone.
2. The red-proofs, reverted.
3. The full four-leg gate once, at the end.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — improvement
  #3 in the ranked list.
- KI-250 (vault), KI-259 (`Capability::Observe`), and the `deck::to_pptx`
  row — the class's clearest instances.
