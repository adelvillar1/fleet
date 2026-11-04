---
name: fleet-work-and-the-conversation-revamp
description: "owner request 2026-10-23, SHIPPED the same day (b0d4530 … bb0cd25) — the sessions surface mixed owner and background work, opening a session was a coin toss, there was no exit/switcher/new, and replies were plain text; the archive is now a "work" list by attention with the app's own runs behind one hidden line, one open verb, a conversation head, a real POST /api/session/new, and anything asserted about the face now lives in a committed harness run by cargo test"
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

Owner, 2026-10-23, after dogfooding: *"Sessions is logging work that I initiate and
work the app does in the background, I cannot easily find the sessions that need
attention or that are in the middle of some work, also opening those sessions is
still a coin toss to get the composer style interaction, when you go to
conversations there is no way to get out of a conversation, and select a
different one, there is no way to create a new conversation from that view. Also
the composer experience needs a bit of TLC, model replies are just plain text
with no formatting, there is wasted space, and it's not a seamless experience."*

**The structural causes (read from the code, cited in the plan):** the archive
carried `ended: bool` and nothing else, so `!ended` painted as `live` forever —
a crashed run and a `max-turns` stop both looked live, and the four endings the
agent actually writes never reached the face. Owner vs background was inference
by absence (`kind` is `scheduled`/`subagent`, `channel` marks a peer). The coin
toss was two handlers under one visible label: the row body always selected a
read-only replay, while the row's `open` action resumed the chat only when
`row.id !== current && !face.running` — and `current` is the *launch* session,
not the loaded one. `#panel-chat` had no head, so no exit and no switcher, and a
session was created only on the first send of a *process*, so a second
conversation was impossible without a restart (`/clear` reloaded the launch
session). Replies were `textContent` with `pre-wrap` while the file pane already
had a markdown renderer.

**Shipped 2026-10-23** (plan `docs/plans/2026-10-23-work-and-conversation.md`,
all ten criteria ticked, `status: completed`; commits `b0d4530` feature →
`bb0cd25` docs; pushed; gate 1506 passed; harness 83 checks / 11 cases):

- `SessionSummary` gained `stop_reason`, `turns`, `last_write` (log mtime), and
  the bridge derives one word per row: `needs-approval`, `paused` (from the
  `.control` file), `working` (this bridge drives it), `active` (unfinished and
  written within a 10-minute `LIVE_WINDOW_SECS`), `completed`/`capped`/`stopped`,
  `interrupted`, `new` (a log with no owner message). Class: `yours`,
  `background`, `delegated`, `channel`; `job` parsed from the cron task frame.
  **The log's own `end` is authoritative over a stale `driving` flag.**
- **`/api/sessions` carries `counts`** (`needs`, `working`, `background`,
  `earlier`, `yours`, `total`) derived once in the bridge — the page never
  re-derives what the server classified (KI-300), and the head names the owner's
  own conversations from `yours`.
- The list is *needs you* → *working* → **one background line** → *earlier* by
  project. The owner ruled the app's own runs out of the view: the line reads
  `background · N runs · hidden`, closed by default, opening to the family lines
  and then the runs (KI-303; on the real archive 146 background runs against 90
  own conversations).
- **One verb:** any row resumes into the conversation in place with a composer.
  The read-only replay retired (and its 400-char clip); a retained
  `showSessionObject` serves the child-branch drawer only.
- Conversation head: back (sheet widths), title → switcher, a **pencil that
  renames** (two gestures, two controls — the plan was amended to say so, not the
  code), the state in words, and New → `POST /api/session/new` (writes
  `Session::create`'s log, so state = `new` before the first message). **`/clear`
  now calls `newConversation()`** — it had been a cleared view over the launch
  session.
- Per-session running: `/api/state` carries `driving`; `record()` stamps every
  event with its session; the client drops another session's events and queues a
  send there naming the working conversation.
- Markdown at the block boundary (`closeAssistant`), which also fixes a caret
  class that never came off; the receipt's report renders through it too.
- Dock no longer auto-opens at ≥1600; the work column (320px, overlay sheet
  ≤1180px) takes the space; prose stays ≤80ch.

**How it was verified, in three layers.** (1) Unit/bridge tests — the state
vocabulary, a run posted into a New session landing in that session, `driving`
pinned and cleared on finish. (2) **The committed face harness** — `tests/face/`
(see [[gui-client-verified-by-a-vm-harness]]); a plan criterion that says "a face
test" is only satisfied by a case in it, never by a live pass. (3) A live two-
conversation pass against a **local stand-in provider** (`node` SSE mock writing
phrases with delays, a scratch `FLEET_HOME` whose `config.json`/`routing.json`
point one provider at it) — the only way to exercise a real run without a key:
it produced a New session's log `start, skills, memory, user, assistant, end`,
the queue line naming the working conversation, and mid-stream
`{streaming:1, rendered:0}` → after `{streaming:0, rendered:1, code:1, li:2}`.

**The deliverable lesson of this session: the verifier reads the criteria, not
the intention.** Four rounds were refused for things that were built but not
*asserted where the criterion said* — a run posted into a New session, the queue
naming its holder, the stream boundary, the earlier-group exclusion, a second
row picked in the switcher, `/clear` posting anything, a 1600px layout, and even
the plan's own `- [ ]` boxes. Writing a criterion ending in "a face test
asserts…" commits you to adding that case in the same delivery
([[a-green-gate-covers-what-was-built-not-what-was-left-out]]).

**Defects found on the way**, all in the ledger with their pins: the receipt's
*ask what is next* called `askFace`, never defined (KI-298); `/favicon.ico` 404'd
on every load (KI-299); the head re-derived counts (KI-300); **`driving` was never
cleared when a run finished**, so `/api/state` reported a finished run as driving
and the next send anywhere queued behind it — a New conversation's first task
silently never started (KI-301); the composer jumped to the top of the panel
(KI-302, [[a-declared-grid-row-re-orders-the-footer]]); background runs opened the
list (KI-303). My own first commit repeated the branch's opening mistake (source
paths only, six fixtures left behind, the committed tree not compiling). Then
KI-305 (the column stranded beside a folded rail) and KI-306 (no fold) — both
straight from the owner's eyes on the shipped surface, which is the pattern of
this whole revamp: the verifier checks the criteria, the owner finds the
geometry.

**Kept honest:** there is no heartbeat record, so active-vs-interrupted is a
10-minute write-time window plus the driver — the plan and `LIVE_WINDOW_SECS` say
so rather than pretending certainty. The technical doc names `plan-surface` and
`plan-gate`, but **neither exists as tooling in this environment**; coverage was
checked by hand and said so (contrast [[plan-is-a-file-not-prose]]). The bundled
visualize `render.py` failed on a `lucide-1.17.0.js` hash mismatch, so the mockup
was validated structurally instead of by preview. 32 start-only logs exist under
the real home (scheduled/boot runs that died before their first message) — worth
a cron-start look, not swallowed by this change.

Related: [[fleet-sessions-archive-groups-by-project]] (superseded as the primary
grouping), [[fleet-conversation-control-inventory]] (the sibling that made the
run controllable; this one makes it findable),
[[a-mislabeled-affordance-reads-as-missing]],
[[gui-composer-width-is-not-the-windows]],
[[the-dock-an-object-opens-needs-a-way-back]].
