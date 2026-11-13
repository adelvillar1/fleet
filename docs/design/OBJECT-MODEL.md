# The object model — one work item, everything hanging off it

**U43's source document.** The face has exactly one unit of record: the
**work item**. Every lens is a projection of the same collection, and every
edge below is derived from a store that already exists. Nothing here is a new
store; nothing here is copied.

## The work item

| Field | Type | Where it comes from today |
|---|---|---|
| `id` | string | `workspace::Task.id` when a task owns it; otherwise the session id |
| `title` | string | task title; else the session's first user message, trimmed to one line |
| `status` | `queued · working · blocked · done` | task status (`working`/`blocked`/`done`); a session with no end record is `working`; a session whose `end.stop_reason` is `completed` is `done`; `failed`/`refused` is `blocked`, named |
| `owner` | assistant name | `workspace::Task.assignee` (a principal), resolved to the role's display name |
| `machine` | instance sid | armada roster, when the work was dispatched to one |
| `created` | timestamp | session `start.created`; task id embeds nothing, so the graph log's first record time |
| `updated` | timestamp | session `end`; else the session file's mtime |
| `turns` | number | session `end.turns` |
| `transcript` | session id | the session the work was done in (`session::summary.id`) |
| `artifacts` | desk slugs | `desk::DeskProject.session == transcript` |
| `facts` | fact names | memory frontmatter `source:` naming the transcript |
| `skills` | skill names | the session log's own `skills` record |
| `lesson` | lesson id | `workspace` graph edge `learns` |
| `review` | review id | the calibration row whose task hash joins the transcript's first record |
| `stage`, `gate`, `coverage` | artifact fields | the artifact's own document (`desk::Stage`, end gate) |

`status` is the only opinionated field. It is derived, never written back:
`done` is earned, and it stays earned only while the artifact passes its gate
(the existing D43 rule, unchanged).

## Edges — derived, never copied

An edge exists only when a store already says so. The spine derives them on
read; nothing is written.

| Edge | Source of truth | Today |
|---|---|---|
| `work → transcript` | task `works_in` edge, or the session itself | graph log; sessions |
| `work → artifact` | task `produces` edge; `DeskProject.session` | graph log; `desk.rs:203` |
| `work → assistant` | task `assigned_bot` edge | graph log |
| `work → lesson` | task `learns` edge | graph log |
| `transcript → artifact` | `DeskProject.session` | `desk.rs:203` |
| `transcript → skill` | session log's `skills` record | `{"type":"skills","skills":[…]}` |
| `transcript → tool` | session log's `tool_resolution` records | `{"requested": …, "resolved": …}` |
| `transcript → fact` | memory frontmatter `source: <session id>` | `memories/*.md` |
| `artifact → lesson` | the lesson row's binding to the artifact | `workspace::view` |
| `machine → this machine` | the armada roster | `armada::roster::load` |
| `review → work` | the calibration row's task hash | `calib.rs` (join by hash) |

Edges that **cannot** be derived today are named, not faked:

- `review → work` holds only when the calibration row carries a task hash and
  the work item's first record carries the same hash. Where it does not, the
  review renders unlinked rather than guessed (U2: bounded and named).
- `work → fact` is rare because most facts record `source: zcode:fleet` (the
  cross-harness pull) or nothing. The edge appears where the fact names its
  transcript; the absence is honest.

## The eight projections

| Lens | Projection | Answers |
|---|---|---|
| Threads | work items with a transcript, newest first | what is being worked on |
| Board | work items by `status`, in three columns | what is moving, stuck, or done |
| Desk | work items with artifacts, by `stage` | what landed and how good it is |
| Intelligence | facts, skills, tools, workflows (the capability set) | what it knows and can do |
| Learning | lessons and practices | what it learned |
| Reviews | review rows with confidence and the owner's word | was it right |
| Inbox | messages with identity resolved | who is talking to it |
| Fleet | machines with earned liveness | what is in the fleet |

The first three are the same collection under three groupings — that is the
whole point of the rebuild, and the test of it: adding a work item must make
it appear in Threads, on the Board, and (once it produces an artifact) on the
Desk without any other write.

## Today's sessions become work items

- **Additive only.** A session is not rewritten. It becomes the work item's
  transcript; the session log stays exactly as it is, and the old
  `/api/sessions` keeps serving the archive until P7.
- **A task that owns a session wins.** When `workspace` records a task whose
  `works_in` edge names a session, the work item is the task and the session
  is its transcript. Otherwise the session is its own work item.
- **Scheduled runs are work items too.** They are titled by their job
  (`corpus-refresh-sys1`), not by the bracket sentence the scheduler writes
  into the first message. The plain title is derived, never copied back.
- **No history loss.** An archived or unreadable session still yields a work
  item with its id as the title and a named status; the spine never drops a
  row silently (U2).

## What the model deliberately does not have

- No `deliverable` distinct from `artifact`, no `session` distinct from
  `transcript` in the plain register (see `VOCABULARY.md`).
- No tenth top-level concept. If a surface needs a new noun, it is either a
  field on the work item, an edge, or it does not ship.
- No second store. The spine is a read; writing still goes through the store
  that owns the fact (desk, memory, calibration, roster).
