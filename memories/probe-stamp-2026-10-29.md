---
name: probe-stamp-2026-10-29
description: "project 2026-10-29 — verification traffic is born marked (`fleet run --probe` → kind: probe) and the legacy backlog was marked in the sessions-probes.json sidecar via `fleet sessions --probe <id>` (74 logs, no log re-stamped); landed as 5b55768, KI-366, plan docs/plans/2026-10-29-probe-stamp.md completed"
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

The navigation plan's named follow-up (the ~59 kind-less verification
runs rendering as the owner's conversations) solved 2026-10-29 as
`5b55768`, KI-366 filed-and-closed. One word at every layer: **probe**.

- **Birth stamp**: `fleet run --probe` creates the session through
  `Session::create_probe` (the create_scheduled seam) so the start
  record carries `kind: probe`. `--probe` with `--session` refuses by
  name — the kind is the birth circumstance; a resumed log keeps the
  kind it was born with.
- **Classification**: `work_class` maps the kind to the `probe` class;
  the rail's owner-only predicate and the owner-scoped counts excluded
  it with ZERO face changes — anything not class-yours was already off
  the screen. `fleet sessions` still lists probes (the log for analysis
  stays whole).
- **Legacy sidecar**: `sessions-probes.json` (id → stamp), the
  titles/archived pattern (entries whose log is gone are dropped,
  atomic write). `fleet sessions --probe <id>` (repeatable) marks —
  the archive's own verb; it validates every id first and refuses the
  whole batch by name if one is not a log. NO log was re-stamped in
  place — append-only holds.
- **The marking ran against the live archive: 74 logs** — the
  unambiguous matches only (first-user-text matching eval-corpus
  scenarios or the named verification shapes: "Reply with exactly",
  "Say the single word", "Take one screenshot with computer_use",
  "Run this bash command exactly", etc.) plus the four `bot dogfood`
  scripted-append checks. **The borderline remainder is named, not
  marked**: "session warmup" (the owner's own ritual) and three genuine
  owner questions stay visible. Curating by exact-text match, not
  vibes, is what made the mass-mark safe.

**Why:** the owner's ruling puts system-initiated sessions in the log,
never on the user screen; kind-less probes were the last class the
classifier could not see. **How to apply:** verification runs drive
with `--probe` from now on; a new verification-shaped surface stamps at
birth rather than growing another sidecar. See
[[conversation-sessions-home-plan-2026-10-29]] and
[[the-owners-work-is-the-view]].
