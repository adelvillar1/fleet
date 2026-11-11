---
status: draft
created: 2026-11-11
updated: 2026-11-11
slug: armada-l7-workspace
---

# Plan: armada L7 — the workspace (the work-object graph)

## Context

Armada is fitted into fleet bottom-up as a stack of responsible, measured
contracts (`../architecture/2026-11-11-armada-atomic-decomposition.md`, §3.7).
The workspace is the work-object graph: the team workspace, the org's
tasks/docs/chat, and contextual bot invocation (assign a task to a persona →
a run under the role overlay, work lands as a linked deliverable). Today
fleet has the *objects* but no *graph*: the thread is a session
(`src/session.rs:202`), the deliverable a desk project (`src/desk.rs:184`),
the lesson a memory fact (`src/lesson.rs:362`), the bot a role
(`src/roles.rs:30`) — four stores, not one (survey §2). There is **no
first-class task**: `todo` is a renderer (`src/tools.rs:3384`) whose only
persistence is a full-replacement `Tasks` record in one log
(`src/session.rs:194,417`), with no id, assignee, or `blocked`. The nearest
graph view, `projects()`, unions deliverables + lessons **by folder**
(`src/gui/bridge.rs:1586`), not by object; and `hand_to_desk` loses the
thread — `desk.session` stays `None` (`src/gui/bridge.rs:782`,
`src/desk.rs:202`). This plan builds the object-keyed graph over those
stores — atoms L7.1–L7.4 — composing identity, journal, and security rather
than re-implementing them. `graph.rs` is cwd-bound dev-plan orchestration
(`src/graph.rs:1`) and is **not** reused for the runtime work graph.

## Design choices inherited (the ledger)

Conforms to `ARMADA-DESIGN-DECISIONS.md`: **D0–D6** (shared — the
responsible-citizen shape D3, trust-by-measurement D4, the stack D5,
non-breaking/JSONL/self-contained D6); **D7–D9** (L1 — the assignee is a
`PrincipalId`, the bot is the deputy under the user's principal riding
`AgentOptions.principal`); **D10–D13** (L2 — work-object events ride the
journal, `record()` hooks it additively); **D14–D19** (L3 — invocation is
gated by `guard.check`, the bot's tools are the principal's declaration
filter, a destructive bot task hits step-up, every decision is audited, the
governance layer is immutable to the agent). Honors **D6** hard: a huddle
**stays a session** (the KI-300 one-derivation rule — no parallel huddle
store), the four object stores are referenced not duplicated, and nothing
the owner does today is refused.

## Decision-model check

No sys1 head fits: graph link/view/assign and status settlement are
deterministic plumbing, invocation gating is the L3 guard's deterministic
lookup (D15), and grading reuses the desk's existing deterministic end gate
(`src/desk.rs:1550`) — no new per-item classify/gate/score/extract judgment.

## Approach

One new module, `src/workspace.rs`, owns the object-keyed graph. A
`WorkObject {id, kind}` carries `kind ∈ {task, thread, deliverable, bot,
lesson}`; only **task** is a store-owned object — thread/deliverable/lesson/
bot are typed references derived on read from the existing session/desk/
memory/roles stores (no parallel store, KI-300). Tasks and `Edge {from, to,
kind}` persist as append-only JSONL at `<home>/workspace/graph.jsonl`, folded
to current state on read — the journal's shape (D10) in a home **exempt from
journal retention** (D11 curtails day-files by age; durable work objects
need their own home). `graph.link` refuses a dangling endpoint or unknown
kind by name; `graph.view(filter)` is the object-keyed read. `hand_to_desk`
is fixed to attach the room's thread (`desk::attach_session`,
`src/desk.rs:344`) and bind the room's workspace (from
`session::summarize(...).workspace`, `src/session.rs:1230`) instead of the
server cwd. `projects()` becomes a projection of `graph.view` — object-keyed
source, folder-keyed output preserved so the face is unchanged.
`task.assign(task, principal) -> Run` admits through `guard.check` under the
assignee principal, then reuses `bridge.run_as` (`src/gui/bridge.rs:4653`)
under the role overlay (a huddle stays a session); the run's outcome settles
the task status, links the deliverable, and journals a `work` event — the
task reads `done` only when the deliverable passes the desk end gate,
otherwise `blocked`, named.

High-level steps:

1. `src/workspace.rs`: `WorkObject`/`Edge`/`Task` + the closed kind
   vocabularies; the JSONL store + fold; `graph.link` (named refusal on a
   dangling/unknown edge) and `graph.view(filter)`; the `home.rs` path
   helper. Reference kinds resolve to `desk::project_rows`,
   `session::list_kind`, `roles::load`, and the lesson facts — never copied.
2. **L7.1 — the task object.** `Task {id, status: working|blocked|done,
   assignee: PrincipalId}`: a stable id minted once, assignee from
   `identity::principal_for` (D8). The `todo` renderer and the session
   `Tasks` record are untouched — the workspace task is the additive,
   cross-session unit.
3. **L7.2 — the edges.** Wire task↔thread↔deliverable↔bot↔lesson; fix
   `hand_to_desk` (`src/gui/bridge.rs:782`) to `desk::attach_session` carrying
   the room id and to bind the room's workspace, recording the
   thread→deliverable edge. An @mention reuses `classify_mention`
   (`src/session.rs:151`) to route to the same invocation seam.
4. **L7.3 — surfaces as views.** `projects()` (`src/gui/bridge.rs:1586`)
   reads `graph.view`, grouping objects by the workspace binding derived from
   each referenced object's store; the folder-keyed output shape is preserved
   (references derive live from the same stores, so existing rows stand).
5. **L7.4 — bot invocation.** `task.assign` → `guard.check` (D15) under the
   assignee principal → `bridge.run_as` under the role overlay (multi-bot via
   `huddle`, `src/gui/bridge.rs:748`, which stays a session) → settle the
   outcome (status + linked deliverable + `work` journal event) → grade via
   the desk end gate. The run carries the assignee principal (the deputy, D9).
6. The surface (hard rule 11): a `fleet workspace` verb renders the object
   graph; the face projects rail reads the object-keyed view.

## Acceptance criteria

- [ ] C0: `src/workspace.rs` exposes `WorkObject {id, kind}`, `Edge {from, to, kind}`, and `Task {id, status, assignee}` with `kind` the closed set {task, thread, deliverable, bot, lesson}; `graph.link` with a dangling endpoint or an unknown kind is refused by name (a test pins both).
- [ ] C1: a workspace `Task` has a stable id that survives restart, `status ∈ {working, blocked, done}`, and `assignee` equal to the L1 principal (`owner` in lane 1); the `todo` renderer output and the session `Tasks` record are unchanged (a test pins the renderer).
- [ ] C2: `graph.link` records task↔thread↔deliverable↔bot↔lesson edges and `graph.view(filter)` returns the linked objects, with thread/deliverable/lesson/bot resolving to the existing session/desk/memory/roles stores and no parallel object store written (a test pins the resolution).
- [ ] C3: `hand_to_desk` sets `desk.session` to the room's id (not `None`) and binds the room's workspace (not the server cwd), and records a thread→deliverable edge (a test pins all three).
- [ ] C4: for an existing home, `projects()` returns the same deliverable/lesson rows as before (non-breaking) now sourced from the object-keyed `graph.view`, and a task object appears grouped by its workspace binding (a fixture-home test compares old vs new output).
- [ ] C5: `task.assign(task, principal)` admits through `guard.check` under the assignee principal; an ungranted principal is refused by name and the refusal settles a `guard` journal event (a test pins deny + audit).
- [ ] C6: `task.assign` under a granted principal reuses `bridge.run_as` under the role overlay with the run carrying the assignee principal (a huddle stays a session — no parallel huddle store), and the work lands as a deliverable linked to the task's thread (a test pins the deputy principal and the link).
- [ ] C7: a bot run's outcome settles the task status and a `work` journal event (kind `work`, principal + object ids); the task reads `done` only when the linked deliverable passes the desk end gate, otherwise `blocked` with the named refusal (a test pins both paths).
- [ ] C8: single-owner behavior is unchanged — the four-leg gate passes and the face projects rail, desk, and huddle flows work as before.
- [ ] C9: the workspace surface renders — `fleet workspace` prints the object graph (tasks + edges) and the face projects rail reads the object-keyed view.

## Files to be touched

**Backend:**
- `src/workspace.rs` — new: WorkObject/Edge/Task, the JSONL store + fold, `graph.link`/`graph.view`, `task.assign`.
- `src/lib.rs` — `mod workspace`.
- `src/home.rs` — the `workspace/graph.jsonl` path helper.
- `src/gui/bridge.rs` — `hand_to_desk` fix (attach_session + room workspace); `projects()` reads `graph.view`; `task.assign` wiring reusing `run_as`/`huddle`.
- `src/cli.rs`, `src/main.rs` — the `fleet workspace` verb.
- Consumed unchanged: `src/desk.rs` (`attach_session`, end gate), `src/session.rs` (`summarize`, `list_kind`, `classify_mention`), `src/roles.rs` (`load`), `src/identity.rs` (`principal_for`), `src/security.rs` (`guard.check`), `src/journal.rs` (`append`).

**Tests:**
- `src/workspace.rs` (module tests) — link refusal, task identity, reference resolution, assign gating, settlement/grading.
- `src/gui/bridge.rs` (tests) — `hand_to_desk` thread/workspace, `projects()` non-breaking.

**Documentation:** see Linked artifacts.

## Out of scope

- Multiple principals / the user store / team-org scopes (lane 2).
- The control-plane `dispatch` verb (L8 consumes `task.assign`).
- Corpus remote-sync and doc provenance write-back beyond the desk's existing digest (L4).
- A parallel huddle store (forbidden — KI-300) and reusing `graph.rs` dev-plan nodes (forbidden — cwd-bound).
- The journal itself (L2 provides) and the guard itself (L3 provides).
- The @mention / co-author face polish (L9 surfaces); L7 provides the invocation seam they route through.

## Verification

**C0/C1/C2:** `cargo test -p fleet --lib workspace` — closed kinds, named
link refusal, task id stability + assignee, reference resolution with no
parallel store.

**C3:** a bridge test hands a room to the desk and asserts `desk.session ==
room id`, the binding equals the room's workspace, and a thread→deliverable
edge exists.

**C4:** a fixture-home test snapshots `projects()` before/after and asserts
the deliverable/lesson rows are identical, with a task object grouped by its
binding.

**C5/C6:** tests drive `task.assign` with an ungranted principal (named deny
+ `guard` event) and a granted one (run under the role overlay, deputy
principal, linked deliverable, huddle is a session).

**C7:** a test settles a passing deliverable (task `done` + `work` event) and
a refused one (task `blocked`, named refusal).

**C8:** the four-leg gate (`cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`); the
face harness still streams.

**C9:** `fleet workspace` renders the graph; the face projects rail reads the
object-keyed view.

## Linked artifacts

- `docs/features/workspace.md` — new: the work-object graph contract (WorkObject, Edge, Task, graph.link/view, task.assign).
- `TECHNICAL-DOCUMENTATION.md` — the workspace contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D37–D43 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L7 atoms ticked when shipped.

## Risks

- **`projects()` non-breaking** — the object-keyed view must reproduce the folder-keyed rows exactly. Mitigation: reference kinds derive live from the same stores (`desk::project_rows`, `session::list_kind`); a fixture-home pin compares old vs new output.
- **`hand_to_desk` binding change** — binding the room's workspace instead of the server cwd changes the digest key. Mitigation: the room's workspace is the correct binding (the survey names the bug); a test pins it; an unbound hand is still named.
- **KI-300 one-derivation** — a huddle must stay a session. Mitigation: `task.assign` reuses `run_as`/`huddle`, which create sessions; the graph stores only a thread reference, never a parallel huddle.
- **Guard on the invocation path** — composes L3's cheap in-memory lookup; the owner wildcard short-circuits lane 1 (D15); overhead is measured (D4), not assumed.
- **Fold-on-read growth** — the append-only graph log is folded on read; a materialized snapshot is a measured optimization (D4) if the settles show need, not premature.

## Dependencies

- `2026-11-11-armada-l1-identity.md` (the assignee principal), `2026-11-11-armada-l2-journal.md` (the work-event home), `2026-11-11-armada-l3-security.md` (the invocation guard). Inherits ledger D0–D19.

## Design choices this layer settles (append to the ledger)

- **D37 — the workspace is `src/workspace.rs`, an object-keyed graph:
  `WorkObject {id, kind}` (kind ∈ task|thread|deliverable|bot|lesson); only
  `task` is store-owned — thread/deliverable/lesson/bot are typed references
  derived on read from the session/desk/memory/roles stores (no parallel
  store, KI-300); tasks + edges persist as append-only JSONL at
  `<home>/workspace/graph.jsonl` folded on read, a home exempt from journal
  retention; NOT `graph.rs` (cwd-bound dev-plan machinery).** Rationale:
  object-keyed not folder-keyed; self-contained JSONL (D6); reuse not
  rebuild; durable work objects cannot live in the age-curtailed journal
  (D11). *Constrains:* L7.2 edges, L7.3 views, L7.4 invocation, L8 (reads the
  work graph), lane 2 team workspace.
- **D38 — the Task object is `Task {id, status: working|blocked|done,
  assignee: PrincipalId}`: a stable cross-session id minted once, assignee =
  the L1 principal (D8); the `todo` renderer and the per-session `Tasks`
  record are unchanged — the workspace task is the additive durable unit.**
  Rationale: `todo` has no id/assignee/blocked (`src/tools.rs:3384`,
  `src/session.rs:194`); the workspace task is what armada assigns.
  *Constrains:* L7.4 assignment, L8 dispatch (a dispatched task is this
  object), lane 2.
- **D39 — `Edge {from, to, kind}` carries a closed kind vocabulary
  (task↔thread↔deliverable↔bot↔lesson); `graph.link` refuses a dangling
  endpoint or unknown kind by name; `hand_to_desk` is fixed to
  `desk::attach_session` carrying the room's thread and to bind the room's
  workspace (not the server cwd), recording the thread→deliverable edge.**
  Rationale: the missing reverse edges; `desk.session` stays `None` today
  (`src/gui/bridge.rs:782`, `src/desk.rs:202`). *Constrains:* L7.3 traversal,
  L7.4 work lands as a linked deliverable.
- **D40 — surfaces are views: `projects()` becomes a projection of the
  object-keyed `graph.view(filter)`, objects grouped by the workspace binding
  derived from each referenced object's store; the folder-keyed output shape
  is preserved so the face is unchanged.** Rationale: the folder-keyed union
  is the nearest graph view but folder-keyed; object-keyed is the contract
  (§2.3). *Constrains:* L9 surfaces, lane 2 team workspace.
- **D41 — bot invocation is a workspace action: `task.assign(task, principal)
  -> Run` (Run = the session the deputy works in) reuses `bridge.run_as`
  under the role overlay and `huddle` for multi-bot; a huddle STAYS a session
  (KI-300 — no parallel huddle store); the bot is the deputy under the user's
  principal (D9), the run carrying the assignee principal.** Rationale:
  reuse roles + huddle + bridge.run, never rebuild. *Constrains:* L8 dispatch
  (reuses this seam), lane 2.
- **D42 — invocation is gated: `task.assign` admits through `guard.check`
  (D15) under the assignee principal before the run; the bot's declared tools
  are the principal's grants narrowed by the role overlay (D16); a
  destructive bot task hits step-up (D18, lane-1 in-turn approval); a refusal
  is named and settles a `guard` journal event (D17) — the workspace does not
  re-implement the guard.** Rationale: contracts stack (D5); invocation is a
  guarded action. *Constrains:* L8, lane 2.
- **D43 — the responsible-citizen settlement + grading: every work-object
  mutation settles a `work` journal event (D10; principal + object ids);
  `task.assign` settles the run's outcome to the task — the deliverable lands
  linked and the status settles `done` only when the deliverable passes the
  desk end gate, otherwise `blocked` with the named refusal; a bot's work is
  graded against its outcome, never trusted by assertion (D4).** Rationale:
  graded not just recorded; named refusal/degradation, never silent (D3).
  *Constrains:* L8 cross-install tracing, lane 2.

## Notes

The graph connects the four stores without duplicating them: the task is the
one new object, the edges are the new tissue, and the surfaces become views.
It composes the bedrock — the assignee is L1's principal, work events ride
L2's journal, invocation admits through L3's guard — so the org (lane 2) is
more principals and team scopes over the same graph, not a rebuild. The
non-breaking shape (references derived live, folder-keyed output preserved,
huddle stays a session) is what lets it land without changing what the owner
does today. Approve this and the ledger carries D37–D43 into L8.