---
name: fleet-folder-is-a-default-not-a-ritual
description: "Owner ruling 2026-10-27: a session's working folder is a fleet-wide default, not a per-session ritual — sessions without their own choice work in the owner's declared folder; KI-337 closed at three layers."
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 71
superseded-by:
valid-until:
---

Owner ruling 2026-10-27, after KI-337 ("the working directory is already set on fleet"): **a folder is a default, not a per-session ritual.** Sessions without their own choice work in the owner's declared folder; the deny floor only holds where the owner left no folder at all.

**Why:** the workdirs registry was strictly per-session, so every fleet-created session (dogfood runs, new conversations) ran bare — `bash`/`read_file`/`write_file`/`list_dir` refused by name — and the folder the owner set stranded on the pending key `""`, which `load_workdirs`' prune dropped from memory on every restart (the file kept it, the process never had it).

**How to apply:** the resolution is `GuiState::effective_folder` (own choice → pending key → launch session's entry) — read by BOTH `run_folder` and the state documents, so the face shows what the run will use. Any new surface that resolves a session's folder must use `effective_folder`, not `workdir_of`. Pinned by `a_chat_without_its_own_folder_works_in_the_fleet_default`, `the_pending_folder_survives_the_load_prune`, and the re-pointed per-session test. Related: [[sessions-rail-cap-keeps-oldest-200]].
