---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: claim-state-invariants
---

# `CLAUDE.md`'s state section: invariants, not counts

`CLAUDE.md`'s "Today's state" section says 45 modules, ~70 K lines, 2,357
tests, and 0.6.26. Measured at this plan's writing: 95 modules, 267,068
lines under `src/`, ~2,450 tests, 0.8.21. The file's own protocol ("state
bullets go stale; cross-check them against the latest recap, the ledger,
and `git log`") is why this is a documentation issue rather than a
defect — but a reader who does not cross-check is misled, and the gap has
grown in five weeks.

## Context

- Exact counts rot at the release cadence: ~18 releases since the section
  was written (0.6.26 → 0.8.21).
- The section's job is orientation — "what is in the crate, roughly, and
  where to look next" — not bookkeeping. The recap and the ledger are the
  precision homes.
- `CLAUDE.md`'s housekeeping protocol forbids "Updated YYYY-MM-DD"
  markers, so the fix cannot be a refreshed timestamp.

## Approach

1. Replace the numeric claims with invariants that a release cannot
   invalidate: "~100 modules", "~2.5 K tests", and a pointer to the
   newest recap by directory (`docs/recaps/`, newest file = the state).
2. Keep the parts that are facts rather than counts: the crate's shape
   (agent core, lanes, routing, memory, delegation, calibration, TUI,
   face, shell), CI's state (disarmed by budget), and the DeepSeek
   reserve's role.
3. Add one sentence that the precise counts live in `git log` and the
   newest recap, so the next reader is told where accuracy lives.
4. Delete the "cargo test → 2,357 passed" style line from the state
   section; keep the gate's shape (the four legs) in the rules section
   where it belongs.

## Acceptance Criteria

- [ ] The "Today's state" section contains no number that a release
      invalidates (module count, line count, test count, version).
- [ ] It points at `docs/recaps/` (newest) as the precision home, in one
      sentence.
- [ ] `CLAUDE.md` remains ≤ 300 lines (its own housekeeping rule).
- [ ] No other section loses content: the source map, hard rules, and
      topology sections are untouched.
- [ ] A reader following the pointer lands on a file that exists.

## Files to be touched

- `CLAUDE.md` — the "Today's state" section only

## Out of scope

- The source map's accuracy (its entries are verified per-session as
  modules are touched).
- `README.md` and the product documents (they carry no counts).
- Rewriting the protocol or house rules.

## Verification

1. Read the section against `docs/recaps/`' newest file and `git log -1`.
2. `wc -l CLAUDE.md` ≤ 300.
3. A grep for `\b0\.[0-9]+\.[0-9]+\b` and `[0-9],[0-9]{3}` in the file
   finds no state claims outside code samples.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F6.
