---
status: draft
created: 2026-11-09
updated: 2026-11-09
slug: inbox-controls
---

# Plan: the inbox that can be worked

## Context

The owner's report (2026-11-09): *"a full redesign of the inbox, it needs more and
better controls — there is no dismiss option, and the setup button doesn't work
— context aware controls, plus dismissal and archive."*

The inbox today is a derived view, not a store: `bridge.owner_inbox` reads five
machineries that already exist (pending seats, the onboarding ladder's open
steps, the rooms' unanswered asks, the cron jobs whose last run failed, the
doctor's failing checks) and `paintInbox` draws them in five groups. A row can
only leave by its fact resolving, so there is nothing to dismiss and nowhere to
archive — and one action is wrong: every onboarding row's `set it up` button
navigates to `#/chat`, the conversation the owner is already reading. The
ladder's real surfaces are elsewhere (the vault has `#/vault`, the github step
its own account card, the rest the settings page), so the button does nothing
the owner can see.

The redesign mockup (measured and read, paper + graphite): 
`docs/plans/mockups/2026-11-09-inbox-redesign/index.html`.

## Approach

One path: **give the inbox the two decisions the owner asked for, and let every
row carry the verbs its own facts allow.**

**A store for the owner's decisions.** `<FLEET_HOME>/inbox.json` — two maps
keyed by a stable row key (`class:target`, e.g. `onboard:vault`,
`ask:<session>`, `cron:<job>:<last_run>`, `doctor:<check>`): `dismissed` and
`archived`, each holding a signature and a stamp. The **signature** is the
underlying fact's own fingerprint (the seat's call id, the ask's `at`, the
job's `last_run`, the step's done flag, the check's message): a dismissed row
comes back the moment its fact changes, so dismissal hides a condition, never
the machinery's report of a new one. The file is written through the same
atomic writer the other sidecars use, capped (200 archived rows, 90 days) so it
cannot grow without bound.

**The read carries the decisions.** `owner_inbox` gains `key` and `signature`
per row, filters dismissed rows out of the groups, and returns `archived` rows
re-derived from the same five reads (a row whose fact has since resolved still
shows in the archive with its day, so restoring it is honest). One GET stays
one GET.

**The verbs, from the row's own facts.** Every non-seat row gets a quiet `…`
menu with **open**, **dismiss** and **archive**; the primary button stays the
class's own act:
- seat — once / always / deny, and the menu's only verb is *open the
  conversation*. A seat is a live gate holding a run: hiding it would strand
  the run, so it cannot be dismissed or archived (deny is its dismissal).
- onboarding — the step's own surface (`vault` → `#/vault`, `github` → the
  account card, the rest → `#/settings`), mapped in one table; *not now* is a
  dismiss.
- ask — answer in the room; dismiss/archive from the menu.
- cron — open the job in settings; dismiss/archive from the menu.
- doctor — open doctor; dismiss/archive from the menu.

**The pane.** A Waiting / Archived segment, class filters with counts, per-group
counts, the row's context menu, keyboard (j/k move, enter runs the row's
primary verb, d opens the menu), and an empty state per segment. The rail badge
counts waiting rows only.

High-level steps, each closing the criteria it names:

1. **C0** — record the setup button's dead navigation as a ledger row, and read
   the current pane live (the five groups) before touching it.
2. **C1** — the step → surface table, and the onboarding row's button wired to
   it.
3. **C2, C3, C4, C6** — the sidecar, the signatures, the read's filter and
   archive, the `POST /api/inbox` ops (dismiss, archive, restore, undismiss),
   and the per-class verbs.
4. **C5, C7** — the pane: segments, filters, counts, the menu, keyboard, the
   badge.
5. **C8** — the harness cases, one per verb plus the setup fix and the
   signature's return.
6. **C9** — the feature doc, the ledger closure, the recap, both checkers, the
   gate, the cut.

## Acceptance criteria

These are the contract; each is a verifiable assertion. The work is done when
every box is checked.

- [ ] C0 — The current pane is read live and the setup button's dead navigation is a ledger row (KI-496) with its evidence before any code changes.
- [ ] C1 — Every onboarding row's primary button navigates to its step's own surface, and the step → surface mapping lives in one table (vault → `#/vault`, github → its account card, the rest → `#/settings`); a harness case presses the vault row and asserts the hash.
- [ ] C2 — Dismissing a row removes it from Waiting and from the rail badge, and the row returns when its signature changes — pinned by a harness case that dismisses an ask and then replays a new ask for the same room.
- [ ] C3 — Archiving a row moves it to the Archived segment with the day it was archived, and Restore puts it back; both survive a restart because they ride `<FLEET_HOME>/inbox.json`.
- [ ] C4 — Every non-seat row carries a context menu with open, dismiss and archive; a seat carries its three answers and only "open the conversation" in the menu; pinned per class.
- [ ] C5 — The segment and filter counts are honest: Waiting excludes dismissed and archived rows, Archived counts only archived ones, and the filters narrow within the segment.
- [ ] C6 — The read stays one GET (`/api/owner-inbox` returns waiting plus archived, each row with its key and signature) and every decision is one `POST /api/inbox` with an op and a key; a malformed op or key is a 400.
- [ ] C7 — The keyboard works on the pane: j and k move the selection, enter runs the selected row's primary verb, d opens its context menu; pinned by a harness case.
- [ ] C8 — The harness carries a case per verb and per the setup fix (dismiss, archive, restore, the signature's return, the seat's menu, the keyboard), and the suite is green.
- [ ] C9 — `docs/features/inbox.md` describes the store, the verbs and the signature rule; the ledger row is closed; `ki_ledger.py` and `plan_check.py` exit 0 at status: completed; the four-leg gate is green; the cut is published.

## Files to be touched

**Backend / API:**
- `src/gui/bridge.rs` — the inbox store (`inbox.json`, signatures, the cap), the read's filter and archive, `POST /api/inbox` handling.
- `src/gui/server.rs` — the new route.
- `src/gui/mod.rs` — nothing expected; listed only if the route table forces it.

**Frontend:**
- `src/gui/assets/app.js` — `paintInbox` (segments, filters, counts, the menu, keyboard), the step → surface table, the rail badge.
- `src/gui/assets/app.css` — the menu, the segments, the selected row.

**Tests:**
- `tests/face/harness.mjs` — the cases named in C1–C8.

**Documentation:**
- `docs/known-issues.md` — KI-496 (the setup button), closed with its evidence.
- `docs/features/inbox.md` — new: the five classes, the store, the verbs, the signature rule, the archive.
- `CHANGELOG.md` — the release section.

## Out of scope

- Snooze / remind-me-later: the signature rule makes dismissal reversible; a snooze adds a clock and is its own increment.
- Search inside the inbox, and bulk "archive everything".
- New item classes (the five are the machinery's own).
- The rail's ordering and the badge's placement.

## Verification

**C0** — `curl /api/owner-inbox` before the change; the KI row carries the live rows and the `#/chat` action.
**C1** — harness: press the vault row's primary button, assert `location.hash` is `#/vault`; press the github row's, assert its account card opens.
**C2** — harness: dismiss an ask (POST `/api/inbox`), assert the row leaves and the badge drops; push a state whose ask carries a new `at`, assert the row returns.
**C3** — harness: archive a row, assert the Archived segment holds it; restore, assert it returns; then restart the bridge in `src/gui/bridge.rs`'s own test and assert the sidecar round-trips.
**C4** — harness: for each class, assert the menu's verbs; the seat's menu has one.
**C5** — harness: counts after a dismiss, an archive and a filter press.
**C6** — `curl -X POST /api/inbox -d '{"op":"nope","key":"x"}'` → 400; a real op → 200 and the document.
**C7** — harness: press j/k/enter/d on the pane.
**C8** — `node tests/face/harness.mjs` green, with the new cases named.
**C9** — `python3 scripts/ki_ledger.py`, `python3 scripts/plan_check.py`, the four gate legs, and the cut's own feed read-back.

## Linked artifacts

- `docs/features/inbox.md` — new: the classes, the store, the signature rule, the verbs, the archive.
- `FUNCTIONAL-SPECIFICATIONS.md` — the inbox surface's controls and states.
- `CHANGELOG.md` — the release section, written at cut time.
- `docs/plans/mockups/2026-11-09-inbox-redesign/index.html` — the measured and read mockup this plan builds.

## Risks

- **Dismissal could hide a live condition.** The signature rule is the guard: a fact that changes returns the row. The seat exception (no dismiss) keeps a live gate visible.
- **The sidecar grows.** Capped at 200 archived rows / 90 days, pruned on write.
- **The mockup's token reading is the real app's.** The primary button is a solid accent fill, not the gradient that failed AA in KI-476; the built surface must keep that (the harness's contrast checks and the mockup's measurement both read ≥ 5.2:1).
- **`#/settings` is a wide surface for a single step.** Acceptable for this increment; a per-step deep link is a later refinement.

## Dependencies

- `docs/plans/mockups/2026-11-09-inbox-redesign/index.html` — the mockup, measured (no overflow, no clipping, ink 5.29–16.85:1) and read in paper and graphite.
- The owner's report of 2026-11-09 (this plan's context).
