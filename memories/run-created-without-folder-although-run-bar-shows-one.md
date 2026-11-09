---
name: run-created-without-folder-although-run-bar-shows-one
description: "A chat run can be created with no working folder bound even though the owner reports the run bar already shows the folder, and nothing inside the run can add it afterwards."
type: project
date: 2026-11-09
basis: inferred
tier: durable
shape: [incomplete]
phrases: [this chat has no working folder, ask the owner to set a folder in the run bar; the next run works there, the chat started with the folder set to fleet, browse is disabled — no use over the measured window]
missing: [the face's run-bar folder picker and how its value reaches run creation, the run-creation path's handling of an absent folder and any inheritance from a parent session, whether a folder set after a run starts is meant to rebind the live run or only the next one]
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Observed 2026-11-04 in the browser face at http://127.0.0.1:3080. The owner stated the chat started with the folder already set to `fleet`. Independent of that statement, every filesystem tool in the run refused: `bash`, `read_file`, and `glob` each returned the same sentence — "this chat has no working folder, so `<tool>` cannot touch the filesystem. Nothing was read, written, or executed. Ask the owner to set a folder in the run bar; the next run works there." `read_document` refused with its own wording on `~/Projects/fleet/docs/harness-orientation.md` ("a path neither side chose"). `browse` was separately disabled for the measured window, so the face's run bar could not be inspected from inside the run to see what it actually displayed.

Two things the refusals do establish: (1) the binding is decided at run CREATION, not consulted per tool call — the wording says "the next run works there", so a folder set after the run began cannot rescue it; (2) the refusal was already in place on the session's very first tool attempt, before the owner's message, so this was never a case of a valid folder being rejected mid-run. What is NOT established is why: whether the picker failed to commit while still rendering a label, whether a compaction/background child session was spawned without inheriting the parent's folder, or whether the face and the run-creation call disagree about what the field means.
