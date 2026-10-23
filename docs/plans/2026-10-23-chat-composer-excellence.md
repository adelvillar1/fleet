---
status: draft
created: 2026-10-23
updated: 2026-10-23
slug: chat-composer-excellence
---

# Plan: the chat and composer, made to excel — low friction without context switching, depth on demand

## Context

The owner's ask, 2026-10-23: "what would make the chat and composer
excel? user friction needs to be low and allow for the user to work
without context switching, but also allow to go deeper to understand
what's going on. In essence what would make this harness a top tier
chat experience" — and, in the follow-up: a persistent goal display "like
in zcode."

The transcript's depth ladder already exists and is right: streamed text
becomes rendered markdown at the block boundary; a thought is one closed
line opening to the words; a command card opens to the command, its
time, its output, and the raw record; the strip's folded lines open
their records. What the ladder cannot do today is go **across** (to the
file a reply names, the artifact a run produced, the conversation a
mention points at) or **back** (to where the reader was, to what changed
while they were gone). And the composer — the surface touched ten times
an hour — still has command-shaped friction.

The seams this plan rides, verified 2026-10-23:

- **The goal** rides every run as its own prompt section (`/api/goal`,
  per-session, read per run). The run bar carries a chip today, but it
  is `hidden` until a goal exists, clips the text to 28 characters
  (`clip(face.goal, 28)` at `app.js:6074`), and a click prints a notice
  card telling the owner to use `/goal clear`.
- **The draft** is one global localStorage key (`fleet-draft`,
  `app.js:5828`) — switching conversations silently shares and
  overwrites what was being typed.
- **Rewind exists and is append-only.** `Bridge::rewind(session, turn)`
  (`src/gui/bridge.rs:2372`) restores a checkpoint's tree and appends
  the `Rewind` record — the log is never rewritten. The turn marks
  already carry a rewind button on hover.
- **Attachments** ride `POST /api/attach` (`bridge.attach`), with a
  file-picker input today and no paste path.
- **The typeahead opens on `/` only** (`typeAhead`, `app.js:6627`:
  `charAt(0) !== '/'` closes it) — commands, ranked semantically, with
  the KI-243/244/245 fixes behind it.
- **Checkpoints** are readable (`GET /api/checkpoints`, `GET /api/diff`)
  and the face already fetches them (`app.js:11462`) — but the run's
  receipt reports turns, tokens, and the closing report, never the files
  the run touched. KI-289 (a deliverable has no address) is open in the
  same family.
- **The work list's search reads row metadata only**
  (`sessionMatchesQuery`: id, title, workspace, model, kind, job, class,
  state, channel) — never message text. The archive is local JSONL.
- **No notifications.** A seat that opens while the owner is in another
  tab waits silently; the phone PWA is installed.

## What we should do

Five moves, each landing green and reviewable on its own — the composer
first, because it is the surface touched most:

1. **The run bar owns the standing state, always visible.** The goal
   chip becomes always-present (a quiet set affordance when unset), with
   a popover that sets, replaces, and clears. Reviewable in one sitting.
2. **The composer stops losing work.** Per-conversation drafts,
   edit-and-resend on the owner's own messages (riding rewind, so the
   log stays append-only), paste-to-attach.
3. **The typeahead speaks two languages.** `/` commands (as today) and
   `@` references — `@file` (the pane's own tree), `@session` (the work
   rows), `@skill`, `@memory` — inserting a reference the reply can act
   on.
4. **Returning is anchored.** A "since you were here" divider, a
   back-to-live control with the unread count while scrolled up, and
   seat/finish notifications when the tab is hidden.
5. **The run ends by handing over the work.** The receipt lists the
   files the run touched with a diff peek from the checkpoint the run
   already wrote, and paths inside replies open the pane (KI-289's
   transcript half). Then, the two bigger legs: **turn navigation**
   (collapse per turn, next/prev, expand-whole-run) and **content
   search** across the home's logs, server-side and literal.

**Four decisions to confirm before increment 1** (my recommendation
first in each):

- **The goal's home.** An always-present chip in the run bar with a
   popover (recommended — the run bar is already the home of the run's
   standing choices: mode, provider, folder) — rather than a floating
   bubble over the transcript, which costs the quiet the owner has twice
   ruled for.
- **Edit-and-resend's mechanism.** Ride rewind (recommended): the edit
   rewinds to the turn before the message and runs the edited text as a
   new run; the log gains a `Rewind` record and the original message
   stays in history — append-only, no rewrite. The alternative —
   in-place message mutation — would rewrite history and is refused by
   the repo's own rules.
- **Content search's engine.** Literal server-side match over the logs'
   owner/assistant text (recommended) — not embeddings: the 2026-10-20
   measurement scored cached session vectors 1/81 top-1 with a 0.00
   median margin, so a semantic pass would be a classifier nobody
   measured against ground truth. A literal match names the line it
   found, which is also the honest result shape.
- **Notifications' carrier.** The browser's Notification API on the
   loopback/PWA face, permission asked lazily on first enable
   (recommended) — no new server push, no provider dependency.

## Approach

**D1 — standing state is visible, not behind a press.** The goal chip's
current shape (hidden until set, clipped, click-prints-a-notice) repeats
the coin-toss pattern the owner has already ruled against. The chip is
always present: unset shows a quiet `◎ + goal` in tertiary ink; set
shows the icon and the text on one line, ellipsized, full text on hover.
The press opens a popover — the full text, and Set / Replace / Clear
riding `/api/goal`. `/goal` remains the keyboard path. The stale-goal
risk is the reason this matters: an objective that rides every run's
prompt and cannot be seen is an objective nobody remembers to drop.

**D2 — the composer's state is per-conversation.** The draft key becomes
per-session (`fleet-draft-<id>`), migrated from the single key on first
read; a conversation switch saves and restores rather than shares. The
queue is already per-conversation by construction (it rides the face's
session).

**D3 — edit-and-resend rides the rewind seam.** The owner's own messages
gain an edit affordance on hover (the same hover grammar as the turn
mark's rewind). Pressing it fills the composer with the message's text;
sending rewinds to the turn before the message and posts the edited text
as the next run. While a run is in flight the edit offer queues instead,
naming the driving conversation the way the queue already does. The
append-only log keeps both the original and the rewind record — the
audit trail survives the correction.

**D4 — paste is an attach path.** A paste carrying files (images above
all) into the composer becomes attach chips through the existing
`/api/attach` pipeline — the client holds the blob, posts it, and the
chip names it; a paste carrying text inserts text, as today. Five files
and the documented byte bounds stay the attach contract.

**D5 — one typeahead, two sigils.** `/` keeps the command surface
exactly as it is. `@` opens the same listbox over four reference
families, each riding a read the face already makes: `@file` over the
pane's tree (`/api/tree`), `@session` over the work rows
(`/api/sessions`), `@skill` over the skills read (`/api/skills`), and
`@memory` over the memory read. A file reference inserts the path and
offers the pane preview; a session reference inserts a link that opens
that conversation; skill and memory references insert their `use_skill`
/ recall forms. Enter sends the highlighted row (KI-245's ruling,
unchanged).

**D6 — returning has anchors.** Per conversation, the face records the
stamp of the last event it showed the owner while they were at the
bottom; on return, a divider marks the first event after it. While the
log is scrolled up and new events arrive, a quiet back-to-live control
appears at the transcript's foot with the unread count; pressing it (or
arrival at the bottom by hand) clears it. When the document is hidden
and a seat opens or a run ends, a Notification fires — asked lazily,
degrading silently where permission is absent.

**D7 — the receipt hands over the work.** The run's checkpoint window
already names the files it touched; the receipt gains an artifacts block
— one row per file, opening the diff the checkpoint holds. Paths inside
rendered replies become links into the pane (KI-289's transcript half;
the route-table half stays that row's). The receipt's numbers stay the
document's own.

**D8 — the turn is a navigational unit.** Turn marks exist; they gain
collapse (a turn folds to its mark line), next/prev turn jumps, and one
expand-this-run control for the audit pass. Quiet by default: nothing
auto-collapses.

**D9 — content search is literal, server-side, and bounded.** The
sessions read gains a match pass over the logs' owner/assistant text
when the query asks for it: per-log, first match wins the row, the
matched line rides the row, and the bound is named (first N logs
newest-first). Results appear in the work list's own grammar — a row
opens the conversation, the matched line names why. No second derivation
in the browser.

## The app

**The run bar** (one line, unchanged width class): mode · provider ·
folder · **goal** — the goal field always present, popover on press.
**The composer**: drafts per conversation, paste-to-attach, `@`/`/`
typeahead, edit-and-resend on the owner's bubbles. **The transcript**:
unchanged ladder, plus the return divider, back-to-live, collapsible
turns, and clickable paths; the receipt's artifacts block. **The work
list**: the same rows, plus the matched line when a content query drove
them. **The chrome**: one notification permission, asked lazily.

## Increments

Each increment closes green — the four-command gate in full, each exit
code read on the cargo invocation itself — with a harness case pinned in
`tests/face/harness.mjs` and a live verification before its commit.

1. **The goal in the run bar.** The always-present chip, the popover,
   the `/api/goal` write-through, the unset affordance. Harness case +
   live pass.
2. **Drafts and edit-and-resend.** Per-conversation drafts with the
   single-key migration; the edit affordance, the rewind-then-run flow,
   the queue's in-flight naming. Harness cases + a live two-conversation
   pass.
3. **Paste and the `@` typeahead.** The paste path over `/api/attach`;
   the `@` families over the four existing reads; the shared listbox
   grammar. Harness cases.
4. **The return pack.** The divider, back-to-live with its count, the
   lazy notification on seat/finish while hidden. Harness cases + a live
   scroll-away pass.
5. **The receipt's artifacts and clickable paths.** The checkpoint
   window's files in the receipt with diff peeks; paths in replies into
   the pane. Harness case + live pass on a run that edits files.
6. **Turn navigation.** Collapse per turn, next/prev, expand-run.
   Harness case.
7. **Content search.** The server's match pass, the row's matched line,
   the jump. A server test over fixture logs + a harness case.

## Acceptance criteria

- [ ] C1 — the goal field is present in the run bar on every visit:
  unset shows a set affordance in tertiary ink; set shows the goal on
  one line with the full text on hover; a press opens a popover whose
  Set/Replace/Clear actions ride `/api/goal` and repaint the chip; `/goal`
  still works and the two agree. Pinned by a harness case.
- [ ] C2 — each conversation's draft is its own: typing in one,
  switching to another, and returning restores the first conversation's
  text and leaves the second's untouched; a reload restores both; the
  legacy single-key draft migrates into the visiting conversation on
  first read. Pinned by a harness case.
- [ ] C3 — an owner message offers edit on hover; editing fills the
  composer, and sending rewinds to the turn before the message and posts
  the edited text as a new run, so the log gains a `Rewind` record and
  keeps the original message; while a run drives, the edit queues with
  the queue's own named reason. Pinned by a harness case and a live
  pass.
- [ ] C4 — pasting a file into the composer produces an attach chip
  through `/api/attach` and the five-file bound still refuses the sixth;
  pasting text inserts text. Pinned by a harness case over a stubbed
  clipboard event.
- [ ] C5 — `@` opens the same listbox as `/` over the four families,
  each row sourced from the read the face already makes; Enter inserts
  or sends the highlighted row per KI-245; a `@file` reference opens the
  pane preview and a `@session` reference opens that conversation.
  Pinned by a harness case over stubbed documents.
- [ ] C6 — a divider marks the first event after the owner's last
  at-the-bottom visit, per conversation; while scrolled up, arriving
  events show a back-to-live control with the unread count that clears
  on return to the bottom; with the document hidden, a seat or a
  finish fires one Notification, and none fires while visible. Pinned by
  harness cases (a Notification stub included).
- [ ] C7 — a finished run's receipt lists the files its checkpoint
  window names, each row opening the diff that checkpoint holds; paths
  inside rendered replies open the pane at that file. Pinned by a
  harness case and a live pass on a run that edits files.
- [ ] C8 — a turn collapses to its mark line and expands again; next/prev
  turn jumps turn-to-turn; one control expands the whole run; nothing
  auto-collapses. Pinned by a harness case.
- [ ] C9 — a content query drives the work list from the server's match
  pass over the logs' owner/assistant text: the matched line rides the
  row, a row opens the conversation, the pass names its bound (first N
  logs, newest-first), and the row metadata fields keep their current
  matches. Pinned by a server test over fixture logs and a harness case.
- [ ] C10 — the gate is green: `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  each exit code read on the cargo invocation itself.

## Files to be touched

- `src/gui/assets/app.js` — the goal chip and popover; drafts; the edit
  affordance and its rewind-then-run flow; the paste path; the `@`
  typeahead; the divider, back-to-live, and notification wiring; the
  receipt's artifacts block; clickable paths; turn collapse; the matched
  line in the work list
- `src/gui/assets/app.css` — the popover, the divider, back-to-live, the
  collapsed turn, the artifacts rows, the edit affordance's hover
- `src/gui/assets/index.html` — the goal popover's anchor, the
  back-to-live control, the divider's mount
- `src/gui/bridge.rs` — the content match pass beside the sessions read
  (D9); nothing else server-side: `/api/goal`, `/api/attach`,
  `/api/rewind`, `/api/checkpoints`, `/api/diff` already carry what the
  composer half needs
- `tests/face/harness.mjs` — the seven new cases, with the Notification
  and clipboard stubs
- `src/gui/server.rs` — a test for the content match pass over fixture
  logs

## Out of scope

- **A "what the model saw" inspector** (the assembled context of any
  turn, reconstructed from the log) and **single-tool re-run with edited
  arguments** — named as the audit layer's next plan, not this one.
- **Floating overlay bubbles** over the transcript, and chat-app
  furniture (read receipts, avatars, typing dots) — ruled out in design.
- **Semantic/embedding search** over sessions — measured 1/81 top-1 on
  this corpus (2026-10-20); a literal match is the honest engine.
- **History rewrite of any kind** — edit-and-resend rides the
  append-only rewind; the original message always remains.
- **TUI parity** for these controls — the TUI's revamp is its own track.
- **A per-run verbosity setting** — the depth ladder plus
  expand-this-run (C8) is this slice's answer.

## Verification

- C1–C8: harness cases over the stubbed DOM and bridge documents, each
  driving the control the way a pointer or keystroke would; the
  Notification and clipboard events stubbed in the harness.
- C3, C7: live passes on the loopback face — a real edit-and-resend
  across two conversations; a run that edits files, read back through
  the receipt's artifacts and the pane.
- C9: a server test over fixture logs (a match in the owner text, one in
  an assistant reply, a bound that stops at N) plus the harness case.
- C10: the full gate, exit codes read directly.

## Linked artifacts

- `docs/features/gui.md` — the composer, the transcript, and the work
  list sections carry the new grammar
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation section: drafts,
  edit-and-resend, mentions, the return anchors, artifacts, turn
  navigation, content search
- `docs/known-issues.md` — KI-289's transcript half closes here (paths
  clickable, artifacts addressed); the row is updated when C7 lands
- `TECHNICAL-DOCUMENTATION.md` — the sessions read's match pass in the
  face's route list

## Risks

- **Popover and typeahead weight in one line.** The run bar and composer
  are dense; the popover and the `@` listbox must reuse the existing
  listbox/popover grammar rather than inventing a second one, or the
  composer grows chrome the owner just finished removing.
- **Rewind is tree-restoring, not context-folding.** `Bridge::rewind`
  restores checkpoint files and appends the record; the conversation
  context after a rewind is the log's own replay. The edit flow must
  feel like "correct and continue" — the live pass exists to check
  exactly that feeling against the mechanism.
- **Notification permission is a browser courtesy.** Safari's PWA
  install may gate it behind the installed app; the feature degrades to
  silent (today's behavior) and the criterion only requires the hidden
  tab case.
- **The content pass walks logs on the ask.** Bounded and newest-first,
  it is a disk scan over JSONL; the bound is named on the row so "no
  match" always means "no match in the first N logs", never a lie.
- **The write guard (KI-284) and the credential scan (KI-308)** — this
  plan's assets are prose and JS; no mandated spellings are expected,
  and `sk-` remains a live constraint on any string the face ships.
