---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: beta-shakeout
---

# Plan: the beta shakeout — find the bugs the features' own gates could not

## Context

Fleet is beta by the owner's declaration (2026-10-30): what they wanted
is there, it looks and works fine, and *"time to find bugs"*. The
suite's 1,842 tests and the face's 474 checks prove what their authors
thought to assert; a shakeout hunts where that empathy ends — the edges
between features, the inputs nobody typed twice, the states a surface
can be in when another acts. The find is the deliverable: every bug
lands in `docs/known-issues.md` as a row with its evidence, the ledger
discipline the repo has run since slice one. Fixes follow the ledger's
own triage — crashers and data-loss first.

## Approach

**A matrix, walked and re-walked — not a checklist of features.** Four
axes; a find is the intersection the features never tested:

1. **The surfaces** — the eight rail pages, the landing, the TUI, the
   CLI's verbs, the phone at both relay origins.
2. **The flows** — run, chat, plan/delegate, resume, rename, goal,
   pair, approve, supersede, the cycle, the day stepper.
3. **The adversarial classes** — empty and enormous inputs (a 100 KB
   paste, a 10k-char goal), hostile characters (emoji, newlines, quotes,
   HTML/script fragments, path separators), state collisions (a drawer
   open while a run lands, two writes in one second, a restart
   mid-flow), boundary counts (row budgets at exactly 30/31, the 512-row
   stream cap on a busy day), and the seams (LAN vs relay vs loopback,
   the phone's poll cadence, the CLI and the face writing one home).
4. **The passes** — pass one walks the matrix fast and shallow, filing
   everything; pass two goes deep on whatever pass one shook loose;
   pass three re-runs the fixed rows.

The hunt rides the live face at `:3080`, the CLI binary, and a scratch
home where a find could damage the real one. Every find: one KI row,
its repro named, its evidence quoted. No fix ships without its pin.

## Acceptance criteria

- [ ] **B1 — the matrix is walked once, shallow, whole.** Every
      surface × every adversarial class has at least one probe
      attempted and its outcome recorded (a find, or "held" with the
      probe named) in the plan's matrix table.
- [ ] **B2 — every find is a row.** Each bug carries a KI row with its
      repro and evidence before any fix is written; the ledger checker
      stays green.
- [ ] **B3 — crashers and data-loss fix first.** A find that loses
      work, corrupts a file, or wedges the face is fixed (with its pin)
      before cosmetic finds are touched.
- [ ] **B4 — the deep pass re-runs the fixed rows.** A fix's repro is
      re-run live and the harness carries its pin; the matrix's row
      moves to "held".
- [ ] **B5 — the beta ships.** The pass ends with the matrix whole, no
      open crasher, and the shakeout's summary in the recap — the beta
      gate the owner's declaration set.

## Files to be touched

**Everything is in scope for a find; fixes land where the bug lives.**
The plan touches only: `docs/known-issues.md` (the finds), this file
(the matrix), and the recap.

## Out of scope

- **New features** — the bug budget comes first; an idea found while
  hunting is a note, not a branch.
- **Refactors** — a fix reshapes the minimum the pin needs.

## Verification

The matrix table in this file, walked; the ledger checker green; the
gate per fix.

## Pass one — the matrix, walked 2026-10-30

| Probe | Surface × class | Outcome |
|---|---|---|
| S1 a 100 KB paste (emoji/script/quotes) into the composer | conversation × enormous inputs | held — the draft carried 131k chars; the task's send bound filed beside KI-378 as the same class |
| S2 a 10k-char goal through `POST /api/goal` | conversation × enormous inputs | **FIND — KI-378**: accepted and stored; the standing objective rides every run's prompt. Fixed: the writer refuses past 2,000 chars, named; pinned |
| S3 a 2 KB emoji/newline/HTML rename | sessions × hostile characters | held — 200 and restored 200; served verbatim (the earlier 409s were the probe's own wrong shape) |
| S4 regex + control characters into the memory search | memory × hostile characters | held |
| S5 four rapid section flips mid-read | all rails × state collisions | held — no page errors |
| S6 a pane card open while the panel flips | all rails × state collisions | held — the card closed with the panel |
| S7 the stepper four days back | calibration × boundaries | held — the honest empty state ("2026-10-26", no rows named) |
| S8 a 20k-char search | memory × enormous inputs | held — the body intact |

The find's fix: `Bridge::GOAL_MAX_CHARS` (2,000) at `set_goal`, the
refusal naming the bound and why; pinned by
`the_goal_refuses_past_its_ceiling`. The task's send bound is the
owner's triage (a one-shot spend, not a standing tax). Pass two goes
deep on whatever the owner's own use shakes loose.

## Notes

- The live face runs under the LaunchAgent — a find that wedges it is
  restarted with `launchctl kickstart`, and the find is filed with the
  restart named.
- The phone's relay origins are the owner's device; the matrix's relay
  rows wait for the owner's hand or a scratch device.
