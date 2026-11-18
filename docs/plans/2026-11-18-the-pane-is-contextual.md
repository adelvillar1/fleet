---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: the-pane-is-contextual
---

# The pane is contextual

The owner's 2026-11-18 ruling, verbatim: "the pane in the drawer should be
contextual in nature, artifacts, deliverables, files, etc should be from the
context of the project that's being worked on. right now the only contextual
part if the execution of a plan and nothing else."

## Context

The pane has six tabs. Measured against the code, none of them is scoped to
the project being worked on:

- **run** — the current conversation's execution. Correct, and it is the tab
  that exists.
- **detail** — the open object, whatever the surface passed it. Contextual
  by accident: it shows the object, not the project.
- **made here** — `threadPaneRows()` reads `face.roomFiles` and the session's
  delegated children. That is one conversation's output, not the project's
  artifacts.
- **files** — `readTree('')` → `bridge.tree(asked)` reads `self.cwd`, the
  **process** directory fleet was launched from. Change conversations or
  open another project and the tree does not move. This is the clearest
  case of the owner's complaint: "files" is scoped to wherever the binary
  started, not to what is being worked on.
- **preview** — `face.file`, one file the owner opened. No project scope.
- **web** — an arbitrary URL. Correct as-is; not project content.

So the pane is a set of per-object viewers wearing a project's name. The one
thing that IS contextual today is a plan's execution, because the board's
drawer passes it the task.

Two facts the design has to respect:

1. **A project is a folder.** `sessionProject(workspace)` takes the last path
   segment; `workspace::read` groups tasks by `binding`; the desk binds by
   `project` path. There is no project object — the folder is the identity.
2. **The bridge already knows the run's folder.** `AgentOptions::workdir`,
   `FileAccess::Folder`, and `bridge.set_folder`'s `sessions-workdirs.json`
   all name an absolute path per conversation. The pane reads none of it.

## Approach

Make the pane answer about ONE project — the one the current conversation (or
the board's open plan) belongs to — and let every content tab read through
that scope.

- The pane resolves a **project root** once: the conversation's working
  folder when set, else the open plan's workspace, else the process
  directory (today's behaviour, kept as the honest fallback and named as
  such in the UI).
- `files` reads the tree **rooted at the project root**: the route gains a
  project-scoped read (`/api/tree?project=<key>`) so the containment rule
  still holds — a path that leaves the project is refused, exactly as a path
  that leaves the launch directory is refused today.
- `made here` becomes the project's **artifacts**: the desk deliverables
  bound to the project, the design boards bound to it, and the files handed
  over in any of its conversations — from the stores that already exist
  (`desk::projects`, `canvas::projects`, the `send_file` artifacts).
- The pane head names the project it is showing, so a scope can never be
  invisible — a pane showing another project's files without saying so is
  the failure this plan exists to prevent.

`preview` and `web` keep their per-object behaviour; they are viewers, not
listings.

## Acceptance criteria

- [x] C1: the pane resolves one project root and names it — the label rides
  the files pane above the tree rather than the head, which already carries
  eight tabs at a 300px floor. A conversation with no folder says
  "no folder \u00b7 showing where the app was launched" rather than passing the
  launch directory off as a project. Pinned by the CTX harness case.
- [x] C2: `files` reads the tree rooted at the project root — `bridge.tree`
  now takes the session and resolves `effective_folder`, and the route passes
  `?session=`. The containment rule is unchanged in spirit: a path that
  leaves the root is still refused by name. Pinned by
  `the_tree_follows_the_conversations_folder` and by the CTX case's
  `session=this-one` assertion.
- [x] C3: `made here` is the CURRENT SESSION's artifacts (the owner's D2:
  "d2 is for artifacts, files, deliverables on the current session").
  `threadPaneRows` already read `face.roomFiles`, the session's delegated
  children, its written and recalled facts, and its tasks — so the tab needed
  no new store, only the ruling that says the session is the boundary.
- [x] C4: the pane is scoped by the CONVERSATION, and one source of truth
  means the board does not get a second notion of the project (the owner's
  D3: "we shouldn't have two different sources, and that rule basically
  should apply everywhere"). A conversation that opens a plan's task
  therefore shows that conversation's folder — the same one the run uses.
- [x] C5: two tests prove it — `the_tree_follows_the_conversations_folder`
  sets a chat's folder elsewhere and asserts the tree lists THAT folder's
  file and not the process root's `src/`; and the CTX harness case asserts
  the read names the conversation.

## Files to be touched

- `src/gui/bridge.rs` — a project-scoped tree read; the artifacts read.
- `src/gui/server.rs` — the route's `project` parameter.
- `src/gui/assets/app.js` — the pane's scope resolution, the head, the two
  tabs that read it.
- `src/gui/assets/index.html` — the pane head's project label.
- `tests/face/harness.mjs` — the case.
- `docs/plans/2026-11-18-the-board-at-execution-level.md` — C4's join.

## Decisions the owner holds

**Settled by the owner, 2026-11-18.**

- **D1 — the conversation's folder is the project.** Most conversations have
  one; a conversation without a folder cannot do real work anyway, because
  work is bound to a folder. So the project root IS the working folder — no
  inference, no launch-directory fallback dressed as a project.
- **D2 — `made here` is the CURRENT SESSION's artifacts, files and
  deliverables.** Not the project's across every conversation. The privacy
  boundary is drawn at the session, and the tab's name says so.
- **D3 — ONE source of truth, and that rule applies everywhere.** The pane
  does not keep a second notion of the project beside the conversation's
  own. `Bridge::effective_folder` is that source, the pane reads it, and
  where two surfaces could disagree the disagreement is a defect rather than
  a policy question.

## Out of scope

- Any change to what the run's own `files` containment permits on disk.
- Project switching as a verb (this scopes the pane to the current project;
  it does not add a project picker to the pane).
- The desk's and canvas's own surfaces.

## Verification

`cargo test -p fleet --lib`, `cargo clippy --all-targets`, `node
tests/face/harness.mjs`, and a live read with two project folders seeded to
prove C5.

## Linked artifacts

- `src/gui/bridge.rs` `tree_read` — the read that is scoped to the process
  today.
- `src/desk.rs` `projects()` / `Desktop: session` — the deliverable store.
- `src/canvas.rs` `projects()` — the design store.
- `docs/plans/2026-11-18-the-board-at-execution-level.md` — the execution
  half this joins.
