---
name: cleanup-pass-trash-not-rm-and-locks-are-runtime-state
description: "feedback 2026-10-30 — the destructive gate blocks `rm -rf` even on an owner-authorized cleanup, so move garbage to `~/.Trash/<named-dir>` (reversible) instead; and `.lock` sidecars beside docs are flock runtime state — never delete them, the fix is gitignore"
type: feedback
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Two lessons from fleet's beta-shakeout cleanup pass (2026-10-30, the
owner's "let's recap and cleanup the garbage temp files"):

**The destructive gate blocks `rm -rf` even when the owner explicitly
asked for the cleanup** — and it is right to: the compound was
irreversible. The shape that satisfies both the owner and the gate is
**move to a named Trash dir** (`~/.Trash/fleet-tmp-cleanup-20261030/`)
— reversible, the owner empties it when satisfied. A `find -mtime +7`
prune set plus named keeps (files the recaps name as kept) moves the
same way.

**`.lock` sidecars beside docs (`docs/known-issues.md.lock` etc.) are
flock runtime state, never garbage** — fleet's `filelock.rs` holds the
flock through read-modify-write, and unlinking a flocked file another
process holds is the race the `.gitignore` comment itself names. The
fix for them showing as untracked noise is **gitignore the pattern**,
not deletion. (The per-fact locks under `memories/` are already
ignored for exactly this reason.)

**Why:** a cleanup pass that deletes can destroy a probe's evidence,
another session's keeps, or a lock a live process holds; the Trash and
the gitignore cost nothing and reverse everything.

**How to apply:** any owner-authorized cleanup — survey and classify
first (grep the docs for references to each name), move to the Trash
rather than delete, keep what recaps or plans name, and treat
`.lock`/`.tmp` runtime sidecars as state to ignore, not state to
remove.
