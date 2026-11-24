---
status: completed
created: 2026-11-24
plan: session-workspace-handoff
---

# The folder a chat works in is recorded at birth, not re-derived

The owner's report (2026-11-24): *"even though both chats are bounded to the
fleet project folder they show categorized and no project."*

Two conversations, each with `/Users/alejandrodelvillar/Projects/fleet`
recorded as its working folder, both render in the `no project` group of the
conversations list.

## What the evidence says

Reproduced from the live home, not inferred:

- `~/.fleet/sessions-workdirs.json` holds **both** chats keyed to
  `/Users/alejandrodelvillar/Projects/fleet` —
  `20261124T000813Z-e3181c69` and `20261124T005304Z-4fd11c7e`.
- Both session logs' `start` records say `"workspace": "/"`.
- The desktop app that served them runs with **cwd `/`** (pid 89156,
  `/Applications/fleet.app/Contents/MacOS/fleet-desktop`).
- The archive-wide census is the tell: **every** `initiator.kind == "user"`
  start record on this home carries `workspace: "/"` (4 of 4), while the 691
  pre-initiator logs carry the project folder correctly.

### The mechanism

`app.js:16960` groups the list by the row's `workspace`, and a row whose
workspace is falsy is named `no project` (`sessionProject`). The row's
`workspace` comes from `bridge.rs`'s `sessions_read`, which publishes
`entry.workspace` — the log's own `Start` record, nothing else.

That record is written by `Session::open_new` (`src/session.rs:1025`) from
`std::env::current_dir()` — **the process's** directory, at the instant the
log is created. Meanwhile the owner's chosen folder lives in the bridge's
`workdirs` map, and `new_session` (`bridge.rs:913`) assigns it to the chat
*after* `Session::create` has already written the start record:

```rust
let created = session::Session::create(&self.home, &self.model)?;   // writes cwd
...
let folder = { let mut state = lock(&self.state); state.name_launch(&id) }; // the owner's folder
```

So the two facts are known at the same moment and only one of them reaches the
log. When the process cwd *is* the folder (the CLI, `fleet gui` launched from
the project) the bug is invisible; when it is not — the desktop app launchd
starts with `cwd /` — the chat is born projectless and stays that way.

`ef2921` — the workspace history of this very defect class — is the same shape
one layer down: a fact the face holds, dropped on the floor at the log seam.

## The fix

The folder the owner chose **is** the session's workspace. Record it at birth:

- [x] C1. `Session::open_new` takes the workspace as a parameter instead of
      reading `current_dir()`, and every creator (`create`,
      `create_with_persona`, `create_with_parent`, `create_lesson`,
      `create_scheduled`, `create_probe`, `create_subagent`) passes the value
      its caller supplies.
- [x] C2. The bridge's terminal/pane creation path passes
      `state.effective_folder(...)` when it is known, falling back to the
      process cwd **only** when no folder was chosen — never silently, and the
      fallback is the same one `folder_value` already publishes to the page.
- [x] C3. `new_session` assigns the chat's folder *before* creating the log, so
      the folder the owner picked at the welcome is the one recorded; a chat
      created with no folder records none and says so.
- [x] C4. A crate test drives the real path: create a chat with a chosen
      folder while the process cwd is somewhere else, and assert the start
      record names the chosen folder — the regression test must fail against
      today's code.
- [x] C5. A crate test asserts the `no project` group is what an *absent*
      workspace produces, so the honest-empty case keeps working and the
      grouping never silently invents a project.
- [x] C6. Repair: the chats on this home that already carry `workspace: "/"`
      while `sessions-workdirs.json` names their folder get the recorded fact
      — a bounded, named backfill, not a rewrite of history.
- [x] C7. The full gate: fmt, clippy `-D warnings`, release build, `cargo
      test`; `python3 scripts/doc_truth.py`, `python3 scripts/ki_ledger.py`
      and `python3 scripts/plan_check.py` all clean.

## Rows

- **R1 — the seam.** `open_new` takes the workspace; creators thread it.
  Serves C1.
- **R2 — the callers.** Bridge and main pass the effective folder; the cwd
  fallback stays but is explicit. Serves C2, C3.
- **R3 — the tests.** C4 and C5, driven through the real creators.
- **R4 — the repair.** C6, bounded and reported.
- **R5 — the gate.** C7.

## A note on what not to do

The tempting cheap fix is to have `sessions_read` fall back to
`sessions-workdirs.json` when the log has no workspace. That would make the
symptom vanish while leaving the log — the record the owner is told is
authoritative — still wrong. The workdirs file is the face's per-chat
preference; the log is the session's fact. This plan fixes the writer and then
repairs the records, rather than papering over the reader.
