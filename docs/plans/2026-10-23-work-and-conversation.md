---
status: completed
created: 2026-10-23
updated: 2026-10-23
slug: work-and-conversation
---

# Work and the conversation: attention, switching, and a transcript that reads

## Context

The owner's report, from using the face for a day's work: sessions
mixes work the owner started with work the app ran in the background;
there is no way to find a session that needs attention or is mid-run;
opening one is a coin toss between a read-only replay and the composer;
from inside a conversation there is no way out and no way to choose
another; there is no way to start a new one; and the composer is plain
text with wasted space around it.

A read of the code confirms each one, and names why:

- **The archive does not carry an outcome.** A row has `ended: bool`
  and nothing else about how a run finished (`SessionSummary`,
  `src/session.rs:521`; the JSON at `src/gui/bridge.rs:3718`). An
  unfinished log is painted `live` whatever happened to it
  (`src/gui/assets/app.js:6435`), so a crashed run looks live forever,
  and a `max-turns` stop looks no different from a completion. The
  ending the agent actually writes — `completed`,
  `completed without a report`, `max-turns`, `stopped by the owner`
  (`src/agent.rs:1742`, `:1802`, `:2164`) — never reaches the face.
- **Owner and background are inferred by absence.** `kind` is
  `scheduled` for a cron run and `subagent` for a child
  (`src/session.rs:29`, `:36`); `channel` marks a peer's conversation; an
  owner-initiated run is neither. The scheduled job's own name exists
  only inside the framed first message (`src/cron.rs:1158`), so the
  archive cannot say *which* job ran.
- **The coin toss is real and structural.** A row's body click always
  selects the replay (`app.js:6814`). The row's `open` action calls
  `resumeSession` only when `row.id !== current && !face.running`, and
  otherwise selects the replay again (`app.js:6794-6808`) — the same
  visible label, two different handlers. `current` is the *launch*
  session, not the loaded one (`src/gui/bridge.rs:237`), and
  `face.running` is one global boolean (`app.js:521`), so a run in a
  different session changes what this row's button does.
- **There is no switcher, no exit, no new.** `#panel-chat` carries no
  head of its own (`index.html:253-495`); leaving means the rail, the
  palette, `/go sessions`, or the state pill. A session is created only
  on the first send of a process (`src/gui/bridge.rs:5619`), so a second
  conversation is impossible without restarting the face; `/clear`
  reloads the launch session instead (`app.js:151`).
- **The transcript is text.** Assistant text is `textContent` with
  `white-space: pre-wrap` (`app.js:804`, `app.css:1504`), while the file
  preview already has a markdown renderer (`app.js:11414`) the
  transcript never calls. The replay clips every message at 400
  characters (`app.js:6948`).
- **Space.** A 236px rail and a 320px dock frame an 820px stream; the
  dock opens by default at ≥1600px and holds three cards even when idle
  (`app.js:5608`, `index.html:595-653`); and the same run state is
  rendered twice — a dock card and the process strip (`app.js:2202`,
  `index.html:600`), each the other's substitute depending on the dock's
  state.

## Approach

**One noun gets a state.** A *work item* is a session: the owner's
conversations, plus the runs the app made for itself. The bridge derives
its state once and sends it, instead of the client guessing:

`needs-you` (a live run is holding an approval seat, or a run is paused
by the owner) · `working` (this face is driving it) · `active`
(unfinished, log written within the live window, not driven here — a
background run on another process) · `completed` · `capped` · `stopped`
· `interrupted` (unfinished and stale) · `scheduled ok` / `scheduled
failed` for the job family. Alongside it, a class: `yours`,
`background` (scheduled), `delegated`, `channel`.

**Work lists by what needs the owner, not by project first.** Groups in
fixed order: *Needs you* · *Working* · *Background* (collapsed to one
line per family, with counts and last outcome) · *Earlier* (ended work,
grouped by project, each row carrying its ending word). A row says its
state in words, with the age, the project, the turns, and — for
background — the job's name. The counts are the server's, not a second
derivation in the client.

**One verb.** `open` loads the work item *in place*, list beside it,
with a composer — for every state. The read-only replay retires as a
separate mode: it already renders the same cards the resumed
conversation does, so the distinction was only the coin toss and the
400-character clip. Reading and continuing are the same act, and the
composer says so when the session cannot accept a message yet.

**The conversation has a head.** Back to work (narrow widths), the
title (press to rename), its state, New, and a switcher that is the same
list as a sheet, so switching never loses the place. At wide widths the
work column stays visible and the head is the identity, not navigation.

**New is real.** `POST /api/session/new` creates a session and anchors
it; the first send runs into it. `/clear` stops being a lie and calls
the same route. The home keeps its place as the front door for new work.

**Running belongs to a session.** `/api/state` carries `driving`; the
client keys running and the seat by session. Opening another
conversation while a run works is allowed; sending there queues with the
reason named ("a run is working in <title>; this waits").

**The transcript reads.** Completed assistant blocks render through the
markdown renderer the file pane already has (headings, lists, code,
bold, links); a streaming block is text until its boundary, then
re-rendered once; the caret class is cleared when a block closes; the
receipt's closing report renders too. Owner text stays plain.

**Space goes to the work.** The dock becomes opt-in at every width
(opened from the run strip) instead of opening itself at 1600px; while
it is closed the strip is the run's one line and no column is reserved
for it. At wide widths the work column takes the space the dock held,
and the prose measure stays readable (≤80ch) while the pane may widen.

The mockup — the voice to approve before a line is built:

`fleet-work-and-conversation.html` beside this plan's session, showing
the shell with the work column (needs you, working, background
collapsed, earlier), an opened conversation carrying a markdown reply,
a tool line, a folded run strip and a live approval card, the quiet
composer, and the 390px form where the head carries back, new, and the
switcher sheet.

## Acceptance criteria

- [ ] **C0 — a row states what the log knows.** `GET /api/sessions` rows carry `stop_reason`, `turns`, `last_write` (the log's mtime), `job` (the scheduled run's job name, read with the cron module's own frame parse), `class` (`yours|background|delegated|channel`), and `state` from the vocabulary above. A unit test maps each recorded ending to its word; an unfinished log whose mtime is inside the live window and that this bridge does not drive reads `active`; a stale unfinished log reads `interrupted`; a `.control` file holding `pause` reads `needs-you`.
- [ ] **C1 — attention is first and the background is quiet.** The list renders *Needs you* → *Working* → *Background* → *Earlier* in that order, with the server's counts; *Background* is collapsed to one line per family (jobs, children, channels) with its last outcome, and expanding shows the runs. A face test over a fixture archive asserts the group order, the collapsed default, and that no scheduled run appears among *Earlier*.
- [ ] **C2 — opening is one verb.** A row's click and Enter call one function for every state; the state-dependent handler at `app.js:6794` is gone. A face test opens an ended, a working, and an interrupted row and asserts each produced the conversation pane with a composer and its history — no read-only mode, and no 400-character clip.
- [ ] **C3 — you can leave and switch.** The conversation head renders back (below 1180px), the title (which renames on a press and opens the switcher beside it), and New; the switcher rows carry the same states. A face test opens one item, picks another in the switcher, and asserts the head's title and the hash both moved, with the list still present.
- [ ] **C4 — new is a session.** `POST /api/session/new` creates a distinct log (`Session::create`, the launch model, `kind: None`), anchors it as the open item, and the next run writes into it; `/clear` calls it. An integration test calls it twice and asserts two ids, two files, one `start` record each, and that the following run's records land in the second.
- [ ] **C5 — running belongs to a session.** `/api/state` carries `driving`; the client keys `running` and the seat by session id. Opening another conversation while `driving` is set is allowed; sending there queues with the reason naming the working item. A bridge test pins `driving`; a face test opens a second item with `driving` set and asserts the composer exists and the queue chip names the working conversation.
- [ ] **C6 — the transcript is markdown.** An assistant reply carrying a heading, a list, inline code, bold, and a fenced block renders those nodes in the transcript (not only in the file pane); a streaming reply is text until its boundary and re-rendered once after; the `streaming` class is removed when the block closes. Pinned by a face test over a fixture reply, and checked live.
- [ ] **C7 — the space is the work's.** The dock is closed by default at every width and opens from the run strip; while it is closed the strip is the run's one line and no column is reserved for the dock; at ≥1600px the work column occupies that space with nothing empty beside it; the prose measure stays ≤80ch while the pane may widen. A face/CSS test asserts the grid at 1600 with the dock closed and the prose max-width.
- [ ] **C8 — the addresses hold.** `#/sessions/<id>` opens that item in place with the list beside it and survives a reload; `#/chat` is the new-work front door; `#/sessions` with no id opens the first *Needs you* item, else the newest, and the URL names the choice. A face test walks all three.
- [ ] **C9 — documented and green.** `FUNCTIONAL-SPECIFICATIONS.md`'s conversation and sessions sections, `docs/features/gui.md`, and `docs/architecture/surfaces.md` speak the state vocabulary, the group order, the one verb, New, and the markdown; the gate is green.

## Files to be touched

`src/gui/bridge.rs` (the state/class derivation, `driving`, `POST
/api/session/new`, the row payload), `src/gui/server.rs` (the route),
`src/session.rs` (`SessionSummary` gains the ending, turns, and the
log's write time), `src/cron.rs` (the job-name read, reused rather than
duplicated), `src/gui/assets/app.js` (the work list, one open verb, the
switcher, New, per-session running, the markdown transcript), `app.css`
(the three columns, the dock's default, prose measure),
`index.html` (the conversation head, the work column), and the three
documents named in C9.

## Out of scope

More than one run at a time per process — the bridge keeps one driving
run, and the queue is how a second message waits. A background job's
approval surfaced in Work (a cron job's own auto-approve policy
decides). A server-side queue. Model-authored titles. A heartbeat
record: the live window is inferred from the log's write time, and the
plan says so rather than pretending to a certainty the log does not
carry. The TUI's forms of these surfaces.

## Verification

The unit and bridge tests named per criterion; the face's own tests
(`tests/face/harness.mjs`, run from `cargo test` by `tests/face.rs` over
a stubbed DOM and the bridge's documents); the gate (`fmt`, `clippy`,
`build --release`, `test`); and a live pass on a running face against a
local stand-in provider walking C4, C5, C6 and C7 end to end — New
writes a session whose log then holds `start, user, assistant, end` for
the run posted into it, a send in a conversation opened while another
drives queues with its name and drains when that run ends, a streamed
markdown reply renders at its boundary, and the grid at 1600px with the
dock closed. The earlier live pass over a seeded archive covered the
states, the groups and the folded families.

## Linked artifacts

- The mockup: `fleet-work-and-conversation.html` in this session's
  visualization directory.
- `docs/architecture/surfaces.md` — the Conversation and Sessions
  sections this plan rewrites.
- `docs/plans/2026-10-23-conversation-control.md` — the sibling that
  made the run controllable; this one makes it findable.
- KI-289 — the deliverable-address precedent for `#/sessions/<id>`.
- `src/gui/bridge.rs`'s `cards_from_records` — the one translator the
  replay and the resumed conversation already share, and the reason the
  separate read-only mode can retire.
