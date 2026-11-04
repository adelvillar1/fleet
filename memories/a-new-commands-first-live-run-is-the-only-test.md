---
name: a-new-commands-first-live-run-is-the-only-test
description: "feedback 2026-10-23/24 — a shipped command can pass the whole gate and still be unusable; `fleet tools coverage` panicked on every machine and a move/copy resolved its destination as a read. Run each new command once, live, against real data before calling the increment done"
type: feedback
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

During the tool-availability close-out, four things had passed unit tests and
a green gate yet failed on first real use:

- `fleet tools coverage` **panicked on every machine** — `block_in_place` on
  the current-thread runtime every headless command builds, plus asking the
  sem1 lane for `embed` when the wire name is `sem1_embed`.
- `file_change`'s move and copy resolved the destination as a **read**, so
  the common case (a destination that does not exist yet) errored.
- `grep` answered a folder-scoped search with absolute temp paths because a
  symlinked base (macOS `/var` → `/private/var`) never prefix-stripped; and
  with `context` it counted the neighbourhood as matches.
- `--dry-run`, the plan's own documented flag, was refused as redundant.

**Why:** a unit test exercises the function and the gate exercises the build;
neither exercises the *command line on this machine against this data* — the
runtime flavor, the wire name a server actually registers, a path through a
symlink, and whether the flag the plan printed exists at all.

**How to apply:** before marking an increment done, run what it added once,
live, and read the output: the CLI verb against the real archive, the tool
through a folder reached by a symlink, the flag exactly as the plan spells
it. A command that panics is not a green gate's business — it is the first
test that counts. Prefer "ran it and here is what it printed" in the recap
over "covered by tests". Related: [[a-green-gate-covers-what-was-built-not-what-was-left-out]],
[[verify-fix-actually-ran]], [[a-fallback-path-hides-the-defect]].
