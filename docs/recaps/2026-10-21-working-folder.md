# A working folder per chat

Status: complete
Date: 2026-10-21

The owner asked for two things in one sentence: attach files to a chat,
and set a working folder. The first shipped in `f51906c`; this session
built the second against the plan drafted at `4077099`, and it landed in
`f554a59`.

## What changed

A chat now carries a folder, and the agent's own tools — not a prompt
line — work in it.

- **The tools.** `Tool::execute` is still the unrestricted run: the
  process's working directory, paths as the model named them. A new
  `Tool::execute_in(arguments, root)` is the chat folder: `bash` spawns
  with it as its cwd, and a relative read, write, or list resolves under
  it. Three escapes are refused by class — a `../x` walk, an absolute
  path outside, and a symlink that resolves outside.
- **The run.** `AgentOptions.workdir: Option<PathBuf>` feeds
  `Work::Builtin.root` and the turn checkpoint's capture root (the store
  stays under the home), and adds one prompt section naming the folder.
  A subagent inherits it through `Delegation.workdir`, the way it
  inherits the model and the voice.
- **The chat.** The bridge stores a folder per session in
  `<FLEET_HOME>/sessions-workdirs.json`, keyed the way sessions are and
  moved from the pending launch key when the thread names the log. The
  folder is resolved when a run is accepted and rides its command, so a
  change applies to the next run and never the one in flight.
- **The refusals.** `POST /api/run` validates a named folder before
  dispatch: an escape or a missing path is a 400 naming the class. A
  *stored* folder that has gone missing is different — it is cleared,
  the run falls back to the launch folder, and the run's first
  transcript event says which folder and why.
- **The face.** The run bar's third control shows the folder's basename
  or "launch folder"; its popover browses the launch directory one
  level at a time through the pane's own `/api/tree`, and
  `POST /api/folder` records the change as one line in the transcript.

## The decision the plan did not make

C0 requires a chat with no folder to behave exactly as today, and
"exactly as today" includes absolute paths outside the launch
directory — the crate's own tests and the everyday CLI use them. So the
containment boundary exists only when a folder is set. `execute` stays
the unrestricted run; `execute_in` is the boundary. `Work::Builtin.root`
became an `Option` to say which one a call is.

## Verification

- Gate: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo build --release`, `cargo test` — all exit 0. The
  suite is 1,453 passed, 1 ignored.
- New tests: the tools' rooted run and the three refusals; the prompt
  and the checkpoint capture root; the per-session sidecar across a
  reload; the outside/missing/symlink refusals at the bridge; the
  deleted-folder fallback as the run's first event; the run route's 400s
  and validation; and an integration test where a child's `pwd` lands in
  the parent's folder.
- Live in Chromium against a scratch home, at 1440 and 390: the bar
  shows "launch folder", the picker lists the launch tree and walks into
  `docs/`, choosing it records a card and moves the button to `docs`, a
  reload restores it, and the reset returns to "launch folder". No
  horizontal overflow at either width and no console errors. The live
  run route answered `../` and a missing path with 400s naming each.

## Amendment: no folder, no access (owner ruling)

The same day, the owner corrected the first cut's reading of C0: a
no-folder chat must not reach any folder. The split is now explicit and
enforced in the agent, not the prompt:

- `AgentOptions.file_access` is one of `Unrestricted` (the CLI, the TUI,
  and headless runs, launched from a directory), `Folder` (a chat's
  grant), or `None`. A `None` run refuses `bash`, `read_file`,
  `write_file`, and `list_dir` by name **before** the approval gate —
  nothing is asked of the owner for a call that cannot run — and the
  refusal tells the model to ask for a folder. The prompt says the same
  before the first call. The turn captures no checkpoint of a tree the
  chat was not given.
- Full access is a grant only a folder can bound: the control is
  disabled on a no-folder chat, a mode left on full falls back to ask,
  and `POST /api/run` with `mode: full` and no folder is a 400 naming
  the rule. The stored-mode door (a remote caller) refuses it too.
- The consequences the ruling implies for the labels and the fallback:
  a chat starts at **"no folder"**, not "launch folder"; choosing the
  launch directory in the picker is a real grant rather than a reset;
  and a stored folder that has gone missing falls back to no folder
  and says the filesystem tools are unavailable until one is set —
  never to the launch directory.

New tests: all four refusals with an approver that panics if asked (the
gate is never reached), the no-checkpoint assertion, an integration run
whose `write_file` lands nothing on disk, the full-access 400 at the
route, and the stored-full refusal. Live: the Full control renders
disabled and reads "full access needs a working folder" until a folder
is chosen, then enables.

## What is left

The plan's own out-of-scope list stands: the files pane still roots at
the launch directory, folders outside it need a settings allowlist
before they can be offered, the TUI and CLI have no per-session folder,
and a lane's own process cwd stays its service home.

## A concurrent commit

While this work was in the tree, another session's commit `543ff95`
(the empty state's recipient fix) swept this feature's `index.html`
markup, which was uncommitted at the time, into its own commit.
`f554a59` carries the rest and its message says so. The other
session's in-flight plan edit
(`docs/plans/2026-10-21-document-perception.md`) and the memory
recall-count bumps were left uncommitted — they are that session's
work.
