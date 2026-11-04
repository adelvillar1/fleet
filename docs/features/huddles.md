# The huddles

The console's rooms: a huddle is **one session with members** — the owner
and one or more bots — shaped like a Slack channel the owner opens the way
they open a chat. Members see every message; a bot addresses another
member with `@` and that member answers with its own tools, skills and
voice; deliverables post in-line and open in the right pane; a phase can
break out into a child huddle. Plan:
[`docs/plans/2026-11-03-huddles.md`](../plans/2026-11-03-huddles.md)
(status: completed); the four screens it was built against live in
`docs/plans/mockups/2026-11-03-huddles/`.

## The room and its states

A huddle lives at `#/huddles`, in the work group beside the conversation
and the sessions. The rail lists every room in three sections —
**active**, **inactive**, **ended** — each row carrying its member marks,
its state word, and its outcome count. A room ends only by the owner's
act: archiving it moves the row to *ended* with its deliverables intact.

A huddle is a session, so it inherits the work list's own grammar. Creating
one writes a `Start` record whose persona set is the join order, the first
named leading, and stores the goal; a session created without personas is
not a huddle and never appears on the rail.

## The head

The open room's head carries the name, the goal, the **member stack** with
each bot's voice line, the state chip, and the composer's own
ask / autonomous / full-access modes. Below it, the **phases**: the child
huddles broken out of this one, each with its own state, each opening in
place.

## Addresses

Posting `@name` writes a `Mention` record — the member and the kind:
**directive**, **ask**, or **review**. The next turn is authored by that
member: the `Assistant` record's `persona` byline names it and that
member's role overlay rides the prompt, so the answer comes back in that
bot's own voice. A mention of the owner (`@you`) raises a needs-you on the
row and starts no run. A mention becomes a row of its own in the transcript,
where the routing fact sits; a review request is styled apart from a
directive.

A name the room cannot receive is not a refusal — it is a **gap**, an
opportunity to hire for it (below).

## A bot's own scope

The speaking member's turn carries its role's tools: a role that names
tools admits exactly those, and the declaration snapshot follows the switch,
so the log stays the prompt's truth. Each member's assigned skills lead
the run's declaration, so the prompt's sixty-skill cap never pushes a
member's own skill out of sight; the library itself is unchanged and every
speaker may load any declared skill. A role that names nothing leaves the
room's own standing; a role that does not resolve leaves the turn unnamed
rather than failing it.

## Capability gaps and hires

When the room's capabilities do not cover the brief, the speaking bot can
name what it lacks — `@role <why>`, a tool, a skill, or a need it
describes rather than names — and the gap lands as a `Recommend` record
(name, kind, reason, author) and one suggestion card. The kind decides the
press: a **role** the home keeps convenes at once; a **tool**, a **skill**,
or an unnamed capability offers to **hire for it**.

Hiring searches the home's roles first — bring a persona up, do not
re-author one — and the room can author the consultant itself:
`author_persona` is declared on every run that has a home, so a gap in a
domain the home has never kept (financial analysis, linear algebra,
non-linear optimization) is filled by writing a role that carries it. The
hire joins the room and speaks next, and it stays in the home's roles for
every later huddle. Engagements are chosen: **standing**, or **temporal** —
a temporal hire leaves when the room is archived, its role file staying
behind for the next engagement.

Nothing joins on the model's say-so: the recommendation rides the same `@`
idiom as an address, and the owner presses the convene.

## Deliverables

A `send_file` inside a huddle renders an artifact card in the transcript at
the post's position, and the card's press opens the file in the right
pane's preview tab — the renderer every transcript already uses, so a
deliverable is session-agnostic. `read_document` and `send_file` are
declared on **every** run, so a scoped bot can write a file and hand it
over; both are exempt from the scope gate that would otherwise refuse them.

## Breakouts

A room can spawn a child huddle carrying the parent's goal and a phase
brief. The child names its parent on its start record (a room, not a
delegated child — no `kind`), inherits the parent's membership unless one
is named, and carries its own phase brief or the parent's standing goal.
The parent's head lists its children with their states.

## The room's knowledge

A huddle is a knowledge space, not only a deliverable producer. The room
shows what it **recalled** — the facts that rode its runs — and what it
**produced** — the facts its members committed — both derived from the
log's own records, beside the outcome count so a room that produced no
files but taught the home three things reads as three, not zero. Skill
proposals filed from a room are home-scoped rather than room-scoped, so
they are named as a follow-up rather than counted here.

A **distil what we learned** action starts a run whose brief turns the
room's lessons into facts and skill proposals through the existing writers;
the owner reviews the proposals, as now.

## The seat and the SOUL

Only destructive acts take the seat, and sys1 judges what is destructive:
`approvals::judge_destructive` asks the vendored judgment lane, and the
verdict — destructive, reason, confidence, judged_from — rides the seat
event and its state document as data beside the approval controls. When
the lane cannot answer, the gate falls back to the standing destructive
class and says so on the seat rather than implying a judgment. The scope is
the owner's: the actions the standing class already flags, one lane call
per held action — nothing changes about which calls ask.

The owner's SOUL is the guardrail under every bot's voice. The base prompt
names the four virtues — truthful, honest, respectful, solution-oriented —
and where a persona speaks, a standing constraint follows its overlay:
**the guardrails outrank the voice**. Ordering is the point; later text
dominates earlier, so the closing line comes after the voice and appears
nowhere a voice could override it.

## Identity

A bot's name is its role file, and its face is optional. The convention is
the filename — `<roles_dir>/<name>.png|jpg|jpeg|webp|svg` — so no editor
field is added and an absent image costs nothing; the derived serif
initial stands in its place. The image renders in the rail row, the member
stack, the new-room picker and the settings persona card, and **never
beside a message**. Renaming a bot is renaming its role file; it changes
nothing about what fleet is.

## The model and the seams

| seam | where |
|---|---|
| a room is a session | `Record::Start.personas`, the goal sidecar (`session.rs`) |
| an address | `Record::Mention` with `MENTION_DIRECTIVE` / `MENTION_ASK` / `MENTION_REVIEW` (`session.rs`) |
| a gap | `Record::Recommend { name, kind, reason, from }` (`session.rs`) |
| a temporal hire | `Join.temporal`, released by `release_temporal` on archive (`session.rs`, `bridge.rs`) |
| the speaker switch | the role overlay follows the addressed member; `Assistant.persona` bylines |
| the hiring tool | `AUTHOR_PERSONA_TOOL` / `author_persona_spec` (`tools.rs`), routed in `agent.rs` |
| the destructiveness verdict | `approvals::judge_destructive` over the vendored lane (`approvals.rs`) |
| the guardrail | `GUARDRAIL_CLOSING` after the role overlay (`agent.rs`) |
| the room's reads | `bridge::huddle`, `huddle_breakout`, `join`, `session_archive`, `session_knowledge` |
| the routes | `GET`/`POST /api/huddles`, `POST /api/huddles/<parent>`, `GET /api/avatar` |
| the events | `UiEvent::Recommended`, `UiEvent::PersonaAuthored` |

## The bot face's relationship

The huddle is the **console's** room — the work face's idiom, the one
Sessions already uses. The bot face (`fleet bot`, the assistant-first
phone PWA) stays what it was: a second window over the same bridge, one
process, one log, one seat. Nothing about the huddle moves to the bot
face, and nothing about the bot face moves into the room; the two are two
clients of one conversation. See
[`bot-app.md`](bot-app.md) for the rulings both faces answer to.

## Verification

The gate is the four legs (`cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`), and
the face harness pins the room's cases — the rail's three states, the
room head, the `@` picker, bylines, the deliverable card, the persona
door, the seat with its verdict, the gap and the hire, the breakout, and
ending a room. Scope is model-side, so it is pinned in Rust rather than by
the harness.

The live proof drove a whole huddle on a scratch home — create, convene, a
mention routing a turn, a recommendation, a deliverable, a hire, a
breakout, end — and found two execution-gate defects the declarations had
hidden: a `send_file` declared but refused at execution, and an
`author_persona` declared but unknown to the tool router. Both are fixed
(KI-410), and the lesson stands: **a declaration is not an execution**.
