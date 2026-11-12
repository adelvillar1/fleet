# The workspace — the work-object graph (armada L7)

Fleet already had the *objects* of team work but no *graph*: the thread is a
session, the deliverable a desk project, the lesson a learning session's
memory facts, the bot a role — four stores, not one. And there was no
first-class task: `todo` is a renderer whose only persistence is a
full-replacement record in one log, with no id, assignee, or `blocked`. L7
builds the object-keyed graph over those stores, composing the bedrock
(L1 identity, L2 journal, L3 security) rather than re-implementing it.

## The graph

`src/workspace.rs` owns the graph. A `WorkObject {id, kind}` carries
`kind ∈ {task, thread, deliverable, bot, lesson}`. Only **task** is
store-owned; thread/deliverable/lesson/bot are typed references derived on
read from the session/desk/roles stores — no parallel store (KI-300). Tasks
and `Edge {from, to, kind}` persist as append-only JSONL at
`<home>/workspace/graph.jsonl`, folded to current state on read — the
journal's shape (D10) in a home exempt from journal retention (D11 curtails
day-files by age; durable work objects need their own home). This is not
`graph.rs` (cwd-bound dev-plan machinery).

- `graph.link` refuses a dangling endpoint, a mis-typed endpoint pair, or an
  unknown kind **by name** — never a silent broken link. Endpoints resolve
  live against the canonical stores.
- `graph.view(filter)` is the object-keyed read: each row carries its
  workspace binding derived from its store, so surfaces group by object, not
  by folder.

## The task

`Task {id, status: working|blocked|done, assignee: PrincipalId}` (D38): a
stable cross-session id minted once, the assignee from L1's principal
(`owner` in lane 1). The `todo` renderer and the per-session `Tasks` record
are untouched — the workspace task is the additive durable unit armada
assigns.

## The hand, fixed

`hand_to_desk` now binds the room's **own workspace** (the cwd the session
was born in, read off its log) instead of the server's cwd, attaches the
room's thread (`desk.session` was `None`), and records the thread→deliverable
edge (D39).

## Surfaces as views

`projects()` is a projection of `graph.view`: the same deliverable/lesson/
practice rows the folder-keyed union read (references derive live from the
same stores, so the rows are unchanged), plus task objects grouped by their
workspace binding. The face is unchanged; the source is object-keyed (D40).

## Bot invocation, gated and graded

`task.assign` (D41/D42) admits through `guard.check` under the assignee
principal before any run, opens a session under the role overlay (a huddle
**stays a session** — KI-300), links the task to its thread and bot, and runs
carrying the assignee principal (the deputy, D9). A refusal is named and
settles a `guard` journal event. The run's outcome settles the task (D43):
the deliverable lands linked and the status reads `done` **only** when the
deliverable passes the desk end gate, otherwise `blocked` with the named
refusal — a bot's work is graded against its outcome, never trusted by
assertion. Every mutation settles a `work` journal event.

## Surface

`fleet workspace` prints the tasks, their edges, and the object-keyed view;
the face's projects rail reads the same view.

## Verification

`cargo test -p fleet --lib workspace::` pins the closed kinds + named link
refusal, task id stability + assignee, reference resolution with no parallel
store, the gated assignment + audit, and the graded settlement (done/blocked).
Bridge tests pin the hand's thread/workspace binding + edge, the non-breaking
projects projection + task grouping, and the deputy principal on the assigned
run. The full gate and the face harness confirm single-owner behavior is
unchanged.