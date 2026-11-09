# Session Recap — 2026-11-09: the inbox that can be worked

**Summary.** The owner asked for a redesign of the inbox — more controls, a
dismiss, an archive, context-aware actions — and reported that the setup button
did nothing. The mockup was drawn, measured and read; the plan
`docs/plans/2026-11-09-inbox-controls.md` was built to. Six releases came out of
it (0.6.8 through 0.6.13), because each live proof found something the tests
could not: the archive staying in the queue, group headers rendering type
names, a dismissal with no way back, and a press that changed nothing for
seconds.

## The two faults in the report

- **The setup button.** Every onboarding row's only action set the hash to
  `#/chat` — the conversation the owner was already reading. The ladder's real
  surfaces live elsewhere, and the inbox had no store, so there was nothing to
  dismiss or archive either (KI-496, KI-497).

## What was built

- **A store**: `<FLEET_HOME>/inbox.json` keeps `dismissed` and `archived` marks
  keyed by the row (`class:id`) with the fact's own **signature** — a dismissed
  row comes back when its fact changes; an archived row keeps its day, outlives
  its fact, and is restorable.
- **The read** carries each row's key, signature, label and class, plus the
  `archived` and `dismissed` lists; **one POST** (`/api/inbox`) carries the
  four ops and answers with the fresh document.
- **Context-aware controls**: the primary button is the class's own act — a
  seat's once/always/deny, an onboarding step's own surface through one table
  (`vault` → `#/vault`, `github` → `#/settings`), an ask's room, a cron job's
  settings, a doctor finding. A `…` menu on every row (open/dismiss/archive);
  a seat's menu only opens its conversation, because a seat holds a run.
- **The pane**: Waiting/Archived segments, class filters with counts, a
  `dismissed · N` toggle with **bring back** on each row, the keyboard
  (j/k, enter, d), and a refusal painted where the press stands.

## The live findings, one release each

| Release | What the live check caught |
|---|---|
| 0.6.8 | the redesign itself |
| 0.6.9 | `archive` answered while the row *stayed* in the queue — the filter only knew about dismissals (pinned by a bridge test driving `inbox_op`) |
| 0.6.10 | every group header rendered `[object HTMLSpanElement]`: `node(name, class, text)` was handed a list of elements — the inbox had shipped that way, and the old surface never asserted the heads' words |
| 0.6.11 | burned: the bump's output was piped through `head`, SIGPIPE stopped it after two sites, `tauri.conf.json` stayed at 0.6.10 and the assets cut refused; the empty release briefly held `releases/latest` and the feed 404'd until it was marked a prerelease (KI-500) |
| 0.6.12 | the bring-back (the `undismiss` op existed since 0.6.8 with no control reaching it), with the version sites repaired |
| 0.6.13 | a decision's reply carries a full read (the doctor probe among it), so a press changed nothing on screen for seconds — the row's fate now paints in the same tick and the reply reconciles |

## The pins

Six IBC harness cases (the setup surfaces, the dismissal and its optimistic
paint, the refusal in the pane, archive/restore, the seat's menu, the keyboard,
the bring-back) plus three bridge tests (the store's round-trip and signature
rule, the archive's cap, an archived row leaving the queue). The harness is
1061 checks across 132 cases; the lib is 2071 tests. Two pins were red-proven by
reverting the code (the group heads, the optimistic paint).

## The live proofs

- The read carries identities: `onboard:vault` with its signature and class.
- A bad op: `POST {"op":"nope"}` → **400** with the reason.
- Archive → the queue empties and the archive holds it with `live: true`;
  restore → back.
- In the browser, on the face: the vault row's «set it up» → **`#/vault`**;
  the `setup` chip narrows to the onboarding rows; a row's menu shows
  open/dismiss/archive; dismiss → `Waiting · 2` with a `dismissed · 1` toggle;
  the toggle shows the row with **bring back**, and the server read answers
  `total: 3, dismissed: []` after the press; archive → `Archived · 1`.

## The burned release (for the record)

`v0.6.11` exists as an empty, prerelease-marked release; its tag points at a
tree whose version sites disagree. 0.6.12 carries the same content correctly.
Deleting the release and the tag is the owner's call (it is a destructive,
outward-facing act), and the feed is unaffected — `releases/latest` names
0.6.13.

## Files changed

- `src/gui/bridge.rs` — the store, the signatures, the read's filter and
  archive, `inbox_op`, the dismissed list.
- `src/gui/server.rs` — `POST /api/inbox`.
- `src/gui/assets/app.js` — the pane's rewrite: segments, filters, menus,
  primaries, keyboard, optimistic paint, the bring-back.
- `src/gui/assets/app.css` — the controls' styles; the settings paint's guard.
- `tests/face/harness.mjs` — the seven IBC cases, the inbox route.
- `docs/features/inbox.md` (new), the plan, `docs/known-issues.md`
  (KI-496..500), `CHANGELOG.md` (0.6.8..0.6.13), the mockup.

## Open questions / next steps

- The owner's call on deleting the burned v0.6.11 release and tag.
- A later refinement, not this plan's: a per-step deep link for the settings
  rows (`#/settings/<item>` already exists) so `github` lands on its card
  rather than the top of settings.
- The `bump-version.sh` script could refuse to finish when a site it expects
  does not carry the old literal — the guard that would have caught KI-500's
  half-bump instead of the assets cut catching it.
