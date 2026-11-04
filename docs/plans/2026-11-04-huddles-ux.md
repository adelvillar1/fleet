---
status: active
created: 2026-11-04
updated: 2026-11-04
slug: huddles-ux
---

# Plan: huddles — the room

## Context

The huddle model works: a room is one session with members, a mention
routes the turn to a bot, a capability gap convenes or hires, phases break
out, the room keeps what it recalled and what it produced. Its **surface**
was shipped as scaffolding and the owner's verdict is blunt: *"What the
previous model did can only be called horrible… Wow me."*

The audit of what shipped:

- **A dead column.** `#huddles-rail` is a 188px `<nav>` bound in
  `app.js:666` and never filled — the split layout reserves a gutter for
  nothing.
- **A broken grid.** `#panel-huddles` is `stage-board` (a block scroller)
  carrying `.board` (a 12-column grid), but `.hsection` has no
  `grid-column` rule, so every section and the new-room button auto-place
  into one ~93px track while `.hrow-name` demands 210px.
- **Unstyled primitives.** `.press`, `.dot`, and `.chip.work|wait|idle|ended`
  have **no CSS rules anywhere**, so "new huddle" renders as a UA button,
  the status dot is a zero-width span, and every state reads identically.
- **No room.** A row press navigates to `#/sessions/<id>` — opening a
  huddle drops the owner into Sessions. There is no rooms rail beside the
  conversation, no last-line preview, no presence, no place for the
  deliverables the room exists to produce.
- **A head made of a strip.** Goal, marks, a `·`-joined string of voices,
  phases, knowledge and five one-line actions are crammed into one block
  above a second head (`#conv-head`), so a room has two headers and no
  hierarchy.

**Outcome.** `#/huddles` becomes the split surface the app already knows
how to build — a rooms rail, the room, the right pane — carrying Slack's
interaction model in Fleet's paper-and-ink skin.

## Direction lock

**The room — Slack's model, Fleet's paper.** A room reads as an editorial
page: a serif room name, a quiet goal line, a member row with each bot's
voice, then a transcript with gutter marks, real time stamps and
deliverables posted as artifact cards. Density is comfortable
(Slack-like), not an ops console. The rail is the channel list: name, who
spoke last and what they said, when, and what needs you.

**What it is NOT.** Not Slack's visual skin — no purple, no avatar
mosaic, no emoji reactions, no glass. No new palette and no new tokens:
the page/surface/ink/accent aliases, `--card-radius`, `--shadow-1`, New
York serif, system sans and SF Mono are the whole vocabulary. No invented
unread counts: the row says what the log proves (`awaiting_you`, the live
run, `archived`).

**The three affordances** (recommended; the picker returned no answer, so
none is labelled user-approved): needs-you and mention badges, the
last-message preview, and presence (who is working). Threads, reactions
and typing dots stay out of scope — they are model work, not surface.

## Precedents this plan answers

- **The settings-ux and plugins-ux verdicts** ("the settings experience
  feels like a wall", "the plugin UI is atrocious"): the page opens on the
  answer, the machinery sits behind doors that name themselves, and the
  owner's question leads. Here the answer is *which room needs me*, so the
  rail is the surface's first sentence.
- **The mockups-first ruling** (plugins-ux): the screens are drawn and
  read before any code. Phase 0 is four static screens.
- **The sessions split is the one split idiom** (`docs/features/gui.md`):
  the work column is absolute over the rail's edge, the stage gives up
  `--work-width` by margin, the 48px strip and the ≤1180 sheet are the
  same component. The rooms rail reuses it rather than inventing a second.
- **`docs/architecture/gui-design-research-2026-10-17.md`**: 13–14px base,
  tabular numerals on every measured readout, mono only for ids/numbers/
  code, translucent dark hairlines, 120/180ms motion, press `scale(.98)`.
- **The huddles plan's out of scope**: no threads, reactions, presence
  beyond the run state, or typing dots. This plan does not reopen it.

## Research

**What exists and is reused** (read, not assumed):

| seam | where |
|---|---|
| the shell grid, rail, pane | `app.css:341-359`, `6205-6493` |
| the work column's mechanics (absolute, `--work-width`, margin, folded strip, sheet) | `app.css:7283-7320`, `7556-7641`, `7685-7697` |
| the row anatomy `.work-row` + `.work-group` | `app.css:7452-7536`, `app.js:10113-10251` |
| the conversation column, `.log`/`.stream`, 820px measure | `app.css:2246-2313` |
| the composer, `#commands` `@` popover | `index.html:432-632`, `app.js:22453-22584` |
| the artifact/event card `.card-row` | `app.css:2605-2704` |
| the pane and its preview | `app.js:21476`, `app.css:6205-6493` |
| the room reads (`/api/huddles`, `/api/state`) | `bridge.rs:7304-7465`, `812-901` |
| the room's events (mention, recommend, byline) | `app.js:2020-2077`, `2648-2664` |
| the themes (paper, console, graphite, contrast) | `app.css:173-266`, `themes.rs` |

**What is new.** `huddles_read` rows carry no last line, so the rail's
preview needs `preview` and `from` added where the fold already walks each
log. `#/huddles/<id>` needs `showRoute` to keep the huddles panel and the
rooms column — today a room is only reachable as `#/sessions/<id>`.

## The design

**1. The rooms rail (320px).** `#room-col`, the work column's twin: a
serif `huddles` head with the active count, `+ new`, a search; sections
`ACTIVE · PARKED · ENDED` (ended folded); rows on `.work-row` — a 3px
state bar, the member stack (`you` first), the room name, the last line
(`designer: the hero copy is tightened…`) with the time, a `needs you`
badge when a bot asked `@you`, and `2 files · 3 facts` as quiet meta.
Phase rows nest one level under their room. The 48px folded strip reads
`rooms` with the needs-you count.

**2. The room head.** One header, not two: the room name in the display
serif with the state chip, and `break out · distil · archive` at the
right; the goal as a quiet line; the member row with each bot's mark, name
and voice, and a presence ring on the one speaking; then the
**deliverable shelf** — `DELIVERABLES · 2` over a horizontal strip of
artifact cards (file mark, name, size, author), each press opening the
pane's preview.

**3. The transcript.** An assistant turn is a room message: the member's
mark in the gutter, `designer · 14:02` above the prose at the 820px
measure. Owner turns stay the accent bubble. A mention is a system line
with hairlines (`designer → scholar · review requested`). A deliverable is
a real `.card-row` artifact card that opens the preview. The capability
gap is an inline proposal — who needs what, why, and the press.

**4. The composer.** The app's composer, unchanged in mechanics, with a
`to` chip at its head when the draft addresses a member (`to designer`,
else `the room`) and a send button that names it; the hint states the rule
where it applies: *a mention routes the turn — `@you` asks, never runs.*

**5. The states.** No rooms: the room area carries the invitation (goal
field, bot picks with voices, `open the room`). No room selected: the rail
stays, the centre names the recent rooms and offers `new room`. A refused
read: the reason in the room area, the rail's last good list kept. No
members: the room says what a huddle is.

## Acceptance criteria

- [x] **C1 — the mockups.** Five screens at *Met:* five screens, a60713b; measured 0 overflow, ink 5.7-7.0:1, rows 72.8px.
      `docs/plans/mockups/2026-11-04-huddles-ux/` (`rooms.html`,
      `room.html`, `deliverable.html`, `empty.html`, `seat.html`) in the
      app's own tokens, rendered at 1440×968: no right-edge overflow, no
      clipped text, room rows ≥44px, the app's rail row at 34px and its
      action sizes, every mark's ink ≥4.5:1 on its disc in all four themes.
      Shown to the owner before the face is touched. *(Amended on
      implementation: five screens, not four — the pressed shelf and the
      open pane earned their own state; and the seat is the app's
      **inline** card above the composer, not a modal, which is what the
      console has always used.)*
- [x] **C2 — the room's last line.** `/api/huddles` rows carry `preview` *Met:* rows carry `preview`/`from` from the same fold; `a_rooms_row_carries_its_last_line`; live: the rail showed "scholar Sent - handed landing-copy.md".
      (the last owner/assistant text, bounded) and `from` (the persona, or
      `you`), read from the log the fold already walks; pinned by a Rust
      test.
- [x] **C3 — the room address.** `#/huddles/<id>` opens the huddles *Met:* `#/huddles/<id>` survives a reload; the row press writes it; a session with no members names itself ("that room is not here").
      surface with that room in place and survives a reload; the row press
      writes it (never `#/sessions/<id>`); a stale id names itself rather
      than loading a blank room.
- [x] **C4 — the rooms rail.** `#room-col` renders the three sections in *Met:* the rooms column - harness (sections, rows, preview, needs you, ended folded, a press opens it) and live.
      order, each row with its member stack, name, last line, time and
      needs-you badge; the current room carries the accent wash; phases
      nest under their room; the folded strip and the ≤1180 sheet behave
      exactly as Sessions' do.
- [x] **C5 — the room head.** One header: name, state chip, goal, members *Met:* one head; `theRoomHeadNamesItsMembers` pins the name, state, goal, members, voices, and the conversation head standing down.
      with voices and the presence ring, and the actions; `#conv-head` is
      hidden inside a room and the ordinary conversation is unchanged
      outside one.
- [x] **C6 — the shelf.** The room's deliverables render as artifact *Met:* `theShelfOpensThePanePreview` + live: the artifact press opened the pane on `landing-copy.md`; the shelf is read off the log so a resumed room draws it.
      cards in the head; a press opens the pane's preview on that file;
      a room with none says so quietly.
- [x] **C7 — the room message.** An assistant turn carries the member's *Met:* bylines (live "SCHOLAR"), the mention as a line between hairlines (harness pins the row and the review variant), the owner bubble unchanged.
      mark, name and time; the operator's turn keeps the bubble; a mention
      is a system line naming both members and the kind.
- [x] **C8 — the composer's address.** A draft containing `@name` shows *Met:* `theComposerNamesWhoHearsIt` - `to the room`, `to designer` + `send to designer`, `to you`.
      `to <name>` (or `to the room`) and the send button names it; `@you`
      reads as the ask that starts no run.
- [x] **C9 — the states.** The four states of the design section (no *Met:* `theHuddleSurfaceNamesItsStates` - the invitation, the refused read; a room and the invitation are exclusive.
      rooms, no room selected, a refused read, no members) each render
      their own words, and none pretends a room exists.
- [x] **C10 — the dead scaffolding is gone.** `#huddles-rail`, the *Met, amended:* the list page and the `hh-*`/`hrow`/`hsection` rules are gone; `.press` got ONE shared base rule instead of deletion, because ten call sites use it across the huddles doors and the plugins surface (which was rendering bare UA buttons).
      `page-head` list, `.hsection`, `.hrow`, `.huddle-head` and `.hh-*`
      are removed with their rules; `.press` is not styled as a one-off
      but deleted (its consumers ride the app's buttons).
- [x] **C11 — the harness.** New cases pin the rail's sections/preview/ *Met:* harness 825 / 92.
      badge/presence, the head and shelf, the shelf→pane press, the
      mention line, the `to` chip, the three empty states and the room
      address; the existing 83 cases stay green.
- [x] **C12 — the gate.** fmt, clippy `-D warnings`, release build and *Met:* fmt 0, clippy 0, release build 0, `cargo test` 2,033/0.
      `cargo test` exit 0; the harness count is read from its own line.
- [x] **C13 — the live proof.** On a scratch home (the machine's *Met:* driven on a scratch home - create, the rail preview, open, @ a bot (byline + mention line), a deliverable (send_file, seat answered, the shelf, the pane preview), a breakout (the child nested), archive (ended, the child kept).
      credentials in the environment, nothing written to disk, the home
      removed after): create a room, read its preview in the rail, open
      it, `@` a bot and watch the mention line and byline, post a
      deliverable and open it from the shelf, break out a phase, archive.
- [x] **C14 — the delivery.** `~/.fleet/bin/fleet` is the fresh binary,
      the served `/app.js` carries the room's markers, the dmg is rebuilt,
      and the branch is pushed. *Met:* the fresh binary installed and the
      service restarted (pid 47679, 127.0.0.1:3080); the served `/app.js`
      carries `room-head`, `composer-to` and `deliverables`, and the served
      `/` carries the room head block, the composer's `to` chip and the
      shelf; `/api/state` on the live home carries `deliverables`.

## Out of scope

- Threads, reactions, typing dots, and any new `Record` — a second plan.
- The bot face, the huddle model, the mention kinds, the hire, the
  knowledge/distil, the seat and the guardrail: untouched.
- A new endpoint beyond the two row fields.
- Retheming: no token is added or changed.

## Verification

```sh
# the mockups
open docs/plans/mockups/2026-11-04-huddles-ux/*.html

# the face
node tests/face/harness.mjs

# the gate (exit codes read directly)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

The live proof drives the release binary on a scratch home and reads the
`/api/huddles` row, the room's head, the shelf's press and the pane's
preview; the delivery step copies the binary to `~/.fleet/bin/fleet`,
restarts the agent and greps the served `/app.js` for the room's markers.

## Linked artifacts

- `docs/features/huddles.md` — the surface's new parts.
- `docs/features/gui.md` — the `#/huddles` row and the rooms column.
- `docs/architecture/design-system.md` — the handoff packet: tokens,
  type, component inventory, the room's composition, anti-patterns.
- `docs/plans/mockups/2026-11-04-huddles-ux/` — the four screens.

## Risks

- **The work column is shared machinery.** `paintWork()` and the huddle
  rail share `.work-col`, `--work-width`, `setWork` and the sheet rules; a
  mode switch must leave Sessions byte-identical. Pinned by the existing
  session harness cases.
- **A stale room id.** `#/huddles/<id>` for a session that is not a room
  must name the refusal, not render an empty transcript.
- **The preview line is per-row log work.** `huddles_read` already folds
  every room's log for state and counts; the preview rides that fold and
  must not add a second read pass.
