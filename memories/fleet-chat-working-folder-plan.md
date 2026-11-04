---
name: fleet-chat-working-folder-plan
description: "2026-10-21 — a per-chat working folder is SHIPPED (f554a59; amended: no folder = no filesystem access, full access needs a folder)"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Shipped 2026-10-21 in `f554a59`** (plan `docs/plans/2026-10-21-working-folder.md`,
closed `status: completed`), then **amended the same day in `7e2e3c9`** and the
status word corrected in `0b0f070`. Gate: fmt/clippy/release build/`cargo test`
all exit 0 (1,456 passed, 1 ignored).

**The seam, as built:** `AgentOptions.file_access: FileAccess` (`Unrestricted` |
`Folder(PathBuf)` | `None`) → `Work::Builtin.access` → `run_builtin` picks
`Tool::execute` (unrestricted) or `Tool::execute_in` (contained); a `None` run is
refused before dispatch; `checkpoint_turn` captures from the folder; the prompt
states the access; `Delegation.file_access` carries it to subagents.

**The design decision the plan did not state (amended the same day):** a chat
with no folder has NO filesystem access at all. `FileAccess::Unrestricted` is
the process cwd with paths as named — the CLI/TUI/headless, launched from a
directory; `Folder` is a chat's grant; `None` is the GUI chat default. A `None`
run refuses all four builtins by name **before** the approval gate, captures no
checkpoint, and the refusal + prompt tell the model to ask for a folder. Full
access needs a folder: the control is disabled, `mode: full` with none is a
400, and a stale stored full mode is refused. A folder is a grant, so choosing
the launch directory in the picker stores it like any other; a stale folder
falls back to none, never to the launch dir. The label is "no folder".

**GUI:** `sessions-workdirs.json` under the fleet home (absent entry = no
folder), keyed by the state key and moved from the pending `""` key when the
thread names the launch session (fan_out persists). The folder rides
`Command::Run/Plan`, resolved at acceptance, so a change can never move under a
run. `POST /api/folder` records the change line; `/api/run` also accepts
`folder` (strictly validated → 400). A stored folder that is gone falls back
with the run's first event naming it and the sidecar entry cleared.

**The face:** the run bar's third control (`FOLDER [docs ▾]`), a popover that
browses the launch dir one level at a time via `/api/tree`, and a `folder` event
card. The folder document carries `path`, `name`, and `rel` because macOS has
two spellings of the launch dir (`/var` vs `/private/var`). Live-verified in
Chromium at 1440 and 390: choose docs → reload restores → reset; no overflow, no
console errors.

**Concurrent-tree note:** commit `543ff95` (another session's empty-state fix)
swept this feature's `index.html` markup into its own commit; `f554a59` carries
the rest and says so. `docs/plans/2026-10-21-document-perception.md` and the
memory recall bumps belong to that session, not this work.

**The parallel-planning session did not write this feature and was challenged
for it (2026-10-21).** The owner asked that session "are you executing the plan
without my approval?" — it had written the plan (`4077099`) and was drafting a
second (perception), then saw `f554a59` land from this session and the plan
flip to `done`. Its answer: the implementation is a different session's commit
under the same git identity, not an unattended execution. Lesson for whoever
reads a `docs/plans/` draft while a sibling session is working: a plan appearing
`draft` is not consent, and a commit implementing it may be the sibling's — say
whose commit it is before accepting or reverting it. Also note the plan left
with `status: done`, where `docs/plans/README.md` specifies `completed`.

**Not built (out of scope, per the plan):** re-rooting the files pane, folders
outside the launch dir (needs an allowlist), the TUI/CLI gaining a folder, lane
process cwds.

Related: [[fleet-composer-needs-permissions-and-a-route-pin]],
[[plan-brainstorming-before-development]],
[[shared-file-commits-can-sweep-another-sessions-work]].
