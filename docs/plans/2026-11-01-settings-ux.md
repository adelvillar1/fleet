---
status: active
created: 2026-11-01
updated: 2026-11-01
slug: settings-ux
---

# Plan: the settings surface — the feedback loop, reversibility, and comprehension

## Context

The 2026-10-29 revamp shipped whole (its S1–S10 all green): the rail,
one section at a time, one-line rows, the item in the pane, the
filter, the budgets, addressable sections and items. The macro shape
is right and stays. The owner's 2026-11-01 verdict — *"settings user
experience is atrocious"* — lands on the layer above the bones: what
happens **after a press**, what a value **means**, and how a wrong
turn is **undone**. The revamp moved 18,448 px of page into eight
lists; it did not give the lists a conversation.

Every gap below is evidence in the code as shipped, measured against
the research this plan was commissioned on.

## Research

**The local UX corpus** (the design database's rules, by severity):
*Submit feedback* (high) — confirm every submission with loading →
success/error; *silent success* is the named anti-pattern. *Error
messages* (high) — announce with `role="alert"`, and always offer the
recovery path. *Inline validation* (medium) — validate on blur, never
submit-only. *No results* (medium) — a dead end must suggest, never
blank. *Empty states* (medium) — guide, never blank. *Confirmations*
(medium) — brief success, auto-dismissing.

**Choice architecture** (Thaler & Sunstein's frame, per the
enclosed research): the **default effect** — pre-set options are
kept passively, so a default that is easy to reverse captures the
benefit without the trap; **reversibility** is what makes a default
safe (a nudge must not forbid); **feedback** is a named
choice-architecture tool — show the consequence after the change;
**translating attributes** — convert raw numbers into meaningful
ones (the "gallons per 100 miles" move); **choice overload** and
**attribute overload** — too many visible options lower motivation,
so advanced detail hides behind the pane, never the row.

**The 10-29 plan's own unshipped promises**, read against the code:
its approach promised "a provenance line **in the owner's words**"
(partially: the derivation line exists but names no next step), a
filter that "opens the chosen item's card" (it opens the section and
drops the item), and its rejected-options section already refused
nested scroll traps and dual visual languages — those refusals stand.

## The gaps (each with its evidence)

- **G1 — a save's confirmation lands off-surface.** `writeSettings`'
  success calls `notice()`, which appends to `face.stream` — the
  conversation feed (`app.js:2451`) — so on `#/settings` the
  confirmation is invisible at the seat. The research's named
  anti-pattern: silent success.
- **G2 — a lost connection is fully silent.** `writeSettings`'s
  `.catch(function () { })` (`app.js:~13040`) swallows a rejected
  fetch: no notice of any kind. No error, no recovery path.
- **G3 — a written knob cannot be undone.** A `file`- or
  `env`-sourced knob's card (`knobSections`) has no revert; tier
  and provider overrides do (`remove the override`). The one
  reversibility the document CAN offer is offered inconsistently,
  and the read does not even carry the compiled default to revert
  to (`settings::State` has value/source, no default —
  `settings.rs:495`).
- **G4 — a destructive act is unrecoverable and unlabeled.**
  `provider-remove`, a rule's delete, a tool's disable are one-press
  with no blast-radius line and no named recovery (the provider's
  override is gone; the key survives only in the vault).
- **G5 — the filter dead-ends silently.** With no match,
  `paintSettingsMatches` simply removes the card (`app.js:~14895`):
  no "no results", no suggestions; under two characters there is no
  response at all.
- **G6 — the filter finds, then drops the item — and S9's item
  route never shipped.** Choosing a match navigates to the owning
  SECTION and clears the filter (`app.js:14893-14899`) — the card
  never opens. S9's criterion also promised
  `#/settings/<section>/<item>` opening both; the code reads only
  `parts[1]` (`settingsSectionId`) and no `parts[2]` handler exists
  anywhere in `app.js` — the harness pinned the press-to-card, not
  the route, so the promise is unfulfilled rather than unused.
- **G7 — the field validates only after the round trip.** Every
  scalar input is a bare text box (`knobSections`); "abc" into a
  numeric knob survives until the server refuses — after a full
  write attempt. Research: validate on blur, error beside the field.
- **G8 — a refusal names no door.** The derivation line says "an
  env var still wins" and stops: how to unset it, which file to
  edit, what to restart is not said. The ceiling line names the
  bound but not that it is a constant, not a config value.
- **G9 — raw numbers without consequence.** Budgets render as bare
  figures; the turn cap as an integer — where the calibration
  stream already holds yesterday's spend and the observed turns,
  the card shows none of it. Research: translating attributes.
- **G10 — two vocabularies for one surface.** The GUI's marks
  (`set`/`env`/`default`) and `fleet settings`' words
  (`unset · default`) name the same facts differently; neither
  document maps them.

## Approach

**The bones stay; the conversation starts.** Seven increments, each
its own commit, each pinned in the face harness. No new token, no
new colour, no new chrome — the 10-29 plan's S6 rule carries.

### U1 — the confirmation, on the surface

Settings writes confirm **on the settings surface**: a transient
card at the board's head (or the section's head on narrow screens)
— `saved <label>`, `refused: <reason>`, or `unreachable — the face
lost the server` (the fetch rejection, which today is nothing). The
card is `role="alert"`, auto-dismissing at the house's toast
discipline, and never lands in the conversation stream. The stream's
`notice` stays for run-adjacent events; a settings write is not one.

### U2 — revert, where the document offers one

- The settings read carries each scalar's **compiled default**
  (`default` beside `value`/`source`/`ceiling`), from the one
  SETTINGS table.
- A knob whose source is `file` or `env` carries **revert to
  default** in its card: one press writes the default back through
  the same op, the row's mark flips, the confirmation names it. For
  an env-sourced knob the revert writes the file default and the
  derivation line says the env var still answers until it is unset
  (honest: the revert does not touch the environment).
- Tier and provider overrides keep their existing removal; every
  destructive list act gains a **blast-radius line** in the card
  ("removing this override disables its tier rows; the key stays in
  the vault"; "this rule retires; the corpus keeps the facts").
  One-press stays (the revamp's rule), but the recovery is named
  where the press is.

### U3 — the filter finishes the job

- A no-match state is a card, not an absence: **no settings match
  "<query>"** with three nearest suggestions from the index's own
  labels, and the hint that two characters are needed.
- A match opens the **item**: the three-part route
  `#/settings/<section>/<item>` is built (S9's unfulfilled half) —
  the route handler opens the section, finds the row, opens its
  card, and marks it — so a deep link lands on the setting itself,
  not its neighborhood. The filter's click sets that route.
- The match list takes the keyboard: arrows move, Enter opens, and
  Escape clears the filter.

### U4 — the field validates before the round trip

Numeric and bounded knobs validate on blur against the kind and the
ceiling the read already carries — the error appears beside the
field (`role="alert"`), the save button stays enabled but the press
names the bound before any request. The server remains the
authority: a refusal it makes still lands through U1's card.
Choice and bool stay selects — they cannot be mistyped.

### U5 — every refusal names the door

The card's derivation lines gain the next step, from the document
itself: an env-sourced knob — "unset `<KEY>` in the environment or
edit `<home>/fleet.env`, then reload the face"; a ceiling — "the
compiled bound is a change to the constant, not a config value"
(its own summary, surfaced); a vault-gated value — the passphrase
flow. No line states a problem without the action.

### U6 — the value, translated where the data already exists

Only numbers the calibration stream already holds: the money
section's budget cards show **yesterday's spend beside the cap**
(stream `spend` rows), the turn cap shows the **observed turns of
the last runs** (stream `turn` rows) when the stream answers, and
absence is named ("no spend recorded yet") rather than zeroed. No
new telemetry, no new writer — the cards read what `fleet calib`
already reads, through the settings read.

### U7 — one vocabulary

A mapping table in the settings feature doc — the GUI's marks, the
CLI's words, the document's sources — one passage both surfaces
cite. The CLI itself is unchanged; the words converge on the doc.

## Acceptance criteria

- [ ] **U1 — the confirmation is on-surface.** A save shows
      `saved <label>` in the board within the same paint cycle; a
      refused write shows the reason there; a dead fetch shows the
      unreachable line — and nothing lands in the conversation
      stream. Harness: stub the post three ways.
- [ ] **U2 — a written knob reverts.** The read carries each
      scalar's default; a file-sourced knob's card reverts in one
      press and the row's mark flips; a destructive list act names
      its blast radius and the survival (the key, the facts).
      Harness: the fixture's revert round-trip.
- [ ] **U3 — the filter finishes.** No match shows the card with
      suggestions; a match opens the item's card through the
      three-part route (the route handler opens the section, finds
      the row, opens and marks it — S9's half that never shipped);
      a bare deep link to `#/settings/<section>/<item>` does the
      same; arrows and Enter drive the list. Harness on each.
- [ ] **U4 — the field validates.** A numeric knob refuses "abc"
      and a value past the ceiling **at blur**, beside the field;
      the server's refusal still lands. Harness with a fixture
      ceiling.
- [ ] **U5 — every refusal names the door.** The env, ceiling and
      vault derivations each carry their next step; harness
      assertions on the three lines.
- [ ] **U6 — the translation reads the stream.** Budget and turn-cap
      cards carry the stream's numbers when present and the honest
      absence when not. Harness with a written fixture stream.
- [ ] **U7 — one vocabulary.** The mapping passage exists and both
      surfaces' doc sections cite it.
- [ ] **U8 — the gate.** fmt, clippy, build, test by exit code; the
      face harness green; a Chromium probe quoting: a save's
      confirmation visible on `#/settings`, the no-results card
      present, and the filter reaching a known knob in three
      keystrokes and one press.

## Files to be touched

`src/gui/assets/app.js` (U1–U5, the confirmation card, the filter,
the field validation, the derivation lines), `src/settings.rs` (U2's
`default` on the read, U6's stream numbers beside the caps),
`src/gui/bridge.rs` (the settings read's additions), the face
harness (`tests/face/harness.mjs`) for every pin, the Chromium probe
in `tmp/`, `docs/features/gui.md` (U7's passage).

## Out of scope

- **Re-partitioning the eight sections** — the owner's 10-29
  ruling stands; if `models` still reads as four questions in one
  coat after this plan, that is its own ruling, named here so it is
  not silently reopened.
- **The TUI's settings surface** — the vocabulary converges through
  the doc (U7); the TUI's own revamp is a later slice.
- **New telemetry** — U6 reads the stream that exists; a card never
  spawns a probe.
- **Confirm-everything modals** — one-press stays the rule (the
  10-29 plan's own); what changes is that the recovery is named and
  the undo exists.

## Verification

The face harness pins every increment's handlers and text; the gate
runs by exit code; the Chromium probe at 1440×900 and a narrow
viewport quotes the confirmation's presence on `#/settings`, the
no-results card, and the filter's keystroke path. The settings
surface's existing S-series checks (the rail, the budgets, the
addressability) keep passing unchanged — nothing here re-opens the
revamp's contract.

## Linked artifacts

- `docs/plans/2026-10-29-settings-rail.md` — the revamp whose bones
  this plan keeps (S6's no-new-chrome rule carries whole)
- `docs/plans/2026-10-31-provider-models-settings.md` — the
  providers catalog this plan's models section renders
- `docs/features/gui.md` — the surface doc the U7 passage lands in
