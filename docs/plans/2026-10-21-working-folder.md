---
status: draft
created: 2026-10-21
updated: 2026-10-21
slug: working-folder
---

# Plan: a working folder per chat

## Context

The conversation surface now carries this chat's standing choices — permissions and a provider pin (`48cd01e`) — and can attach text files to a turn (`f51906c`). What it cannot do is tell the agent **where** to work: every run uses the process's launch directory.

The tools resolve `std::env::current_dir()` at call time. `Work::Builtin` already carries a `root` field (`src/agent.rs:2332`, `:2646`), but it is filled from the cwd and only used by the write-time vocabulary guard; the four built-ins ignore it (`src/tools.rs:1242` calls `run_bash(command)`, `run_read_file(path)` and friends with no root), and `checkpoint_turn` captures from the cwd (`src/agent.rs:2051`). A per-chat folder that the tools do not enforce would be a prompt line pretending to be a boundary.

The owner asked for this directly: "we need to be able to attach files to chat and set a working folder." The first half shipped; this plan is the second.

## Approach

Thread one root through the run: the agent's built-ins, their relative paths, and the checkpoints read the chat's folder instead of the process cwd.

The root already exists at the right seam — `Work::Builtin.root` — so the change is to fill it from `AgentOptions.workdir` and pass it to `run_builtin(tool, arguments, root)`. Bash spawns with `Command::current_dir(root)`; read/write/list resolve a relative path against root and canonicalize it; checkpoints capture from root. The default is the process cwd, so a chat with no folder behaves exactly as today.

The folder is per session, chosen in the run bar, validated by the server with the pane's own `pane_path` containment rule (canonicalized, symlinks resolved), and persisted in a fleet-owned sidecar keyed by session id. The session log does not change: it is the conversation, and a surface setting does not belong in its format.

High-level steps:

1. `tools.rs`: `run_builtin(tool, arguments, root)` and the four tools take the root; tests pin relative read/write/list, `pwd` in bash, and an escaping path.
2. `agent.rs`: `AgentOptions.workdir`, the builtin root and `checkpoint_turn` from it, one prompt sentence naming the folder, subagent inheritance; tests.
3. `bridge.rs` + `server.rs`: per-session folder, `/api/run` `folder`, `pane_path` validation, the sidecar; tests for a good folder, an escaping one, and a missing one.
4. `app.js` / `index.html` / `app.css`: the run bar's folder control, its popover, the transcript card on change; live verification.

## Use cases

- [ ] A: A chat's runs must operate in the owner's chosen folder, and the tools — not a prompt line — must enforce it.
- [ ] B: The choice must be per chat and survive a reload; a chat with no choice keeps the launch folder.
- [ ] C: A folder outside the launch boundary, a symlink that escapes it, or a folder that no longer exists must be refused or named, never silently used.
- [ ] D: Delegated work must land in the same folder: a subagent's tools inherit the parent's.

## Acceptance criteria

These are the contract. Each one is a verifiable assertion. The work is "done" when every box is checked.

- [ ] C0: With no folder set, bash `pwd`, a relative read, a relative write, a list, and a checkpoint are unchanged from today's launch-cwd behaviour.
- [ ] C1: With a folder set, bash runs with that folder as cwd (`pwd` prints it), and a relative read, write and list resolve under it.
- [ ] C2: `checkpoint_turn` captures the tree rooted at the chat's folder, not the process cwd.
- [ ] C3: A relative path that escapes the folder (`../x`), an absolute path outside it, and a symlink resolving outside it are each refused with a reason that names the class.
- [ ] C4: A chat's folder is stored per session, restored after a page reload, and a second chat starts at the launch folder.
- [ ] C5: `POST /api/run` with a folder outside the launch root returns 400 with the pane's containment reason; a path that does not exist returns 400 naming it; a valid folder is accepted.
- [ ] C6: When the stored folder no longer exists at run time, `POST /api/run` falls back to the launch folder and the run's first event names the fallback; a test creates a chat folder, deletes it, and asserts both the launch path used and the named notice.
- [ ] C7: The run bar shows the current folder (its basename, or "launch folder"), and changing it adds one line to the transcript; the change applies to the next run, never the one in flight.
- [ ] C8: A subagent spawned by a run uses the parent's folder for its built-in tools.

## Files to be touched

**Backend / API:**
- `src/tools.rs` — `run_builtin(tool, arguments, root)`; `run_bash`, `run_read_file`, `run_write_file`, `run_list_dir` resolve against the root.
- `src/agent.rs` — `AgentOptions.workdir`, the `Work::Builtin.root` fill, `checkpoint_turn`, the gui prompt sentence, subagent inheritance.
- `src/gui/bridge.rs` — per-session folder map, `/api/run` folder, `pane_path` validation, the sidecar under `<FLEET_HOME>`.
- `src/gui/server.rs` — parse `folder` from the run body and refuse a malformed value before dispatch.

**Frontend:**
- `src/gui/assets/index.html` — the folder control in the run bar.
- `src/gui/assets/app.js` — the picker, the chip, the transcript card, the run body field.
- `src/gui/assets/app.css` — the control and popover styles.

**Tests:**
- `src/tools.rs` — root-relative reads/writes/lists, bash cwd, escaping paths.
- `src/agent.rs` — builtin root, checkpoint root, subagent inheritance.
- `src/gui/server.rs` — folder validation and refusal shapes.

**Documentation (see Linked artifacts):**
- `docs/features/gui.md` — the run bar's third control.
- `TECHNICAL-DOCUMENTATION.md` — the tool root and checkpoint section.
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation surface's working-folder contract.

## Out of scope

- Re-rooting the files pane at the chat's folder (the pane stays at the launch directory; a follow-up can make it follow).
- Folders outside the launch directory, or an allowlist for them.
- The TUI and CLI gaining a per-session folder.
- Changing the session log format; the folder is a surface setting in a sidecar.
- Changing the folder for a run already in flight.
- Lane processes' own cwds; a lane's service home is its own.

## Verification

**C0: default behaviour unchanged**
```bash
cargo test --lib tools:: agent::tests::checkpoint
```
Expected: the new root tests pass with the launch cwd, and no existing tool/checkpoint test changes.

**C1/C2/C3: the root is enforced**
```bash
cargo test --lib tools:: run_builtin
```
Expected: bash `pwd` prints the temp root; relative read/write/list land under it; `../x`, an outside absolute path, and an escaping symlink are refused by name; a checkpoint captures files under the root.

**C4: per chat**
Live: open two chats, set `docs/` in the first, run nothing in the second; reload and confirm the first still shows `docs` and the second still says launch folder.

**C5/C6: refusal and fallback**
Live or curl: `POST /api/run` with `folder: "../"` → 400 naming the escape; with a deleted path → 400 naming it; with `docs` → accepted. Then delete the folder and run again: the reply or transcript names the fallback.

**C7: the surface**
Live at 1440 and 390: the run bar shows the folder; changing it adds the transcript line; a reload restores it; no overflow or console errors.

**C8: inheritance**
```bash
cargo test --lib agent::tests::subagent
```
Expected: the child's options carry the parent's workdir.

## Linked artifacts

- `docs/features/gui.md` — the composer's run bar gains the folder control and its contract.
- `TECHNICAL-DOCUMENTATION.md` — the built-in tools' root and the checkpoint capture root.
- `FUNCTIONAL-SPECIFICATIONS.md` — "the conversation" section: a chat's working folder and its boundary.

## Risks

- **The tool refactor is cross-cutting.** `run_bash` and the three file tools are called from one dispatch; the change is contained there, but a missed call site would silently keep the process cwd. The default-equals-cwd test in C0 is the guard.
- **Symlinks.** Canonicalize before the containment check; a lexical `..` check alone is not enough (the pane already made this distinction).
- **Checkpoint store vs capture root.** The store root stays under `FLEET_HOME`; only the capture root moves.
- **Sidecar lifecycle.** A sidecar entry for a deleted session is harmless but should be dropped when the session log is gone.
- **Resumed chats.** A folder that no longer exists falls back and is named, rather than failing the run.

## Notes

Decisions taken while drafting:

- **Containment inside the launch directory.** This is the same rule the files pane enforces, so a browser-driven run cannot be pointed at `~/.ssh`; an allowlist for outside folders is a later decision, not a free-text field.
- **A sidecar, not a session record.** The session format would need a version bump for a new record type; the folder is a surface setting, so `<FLEET_HOME>/sessions-workdirs.json` keeps it out of the conversation format.
- **The root already exists.** `Work::Builtin.root` is currently only the vocabulary guard's root; filling it from the run is the smallest seam that makes the folder real.
