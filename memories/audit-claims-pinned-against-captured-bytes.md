---
name: audit-claims-pinned-against-captured-bytes
description: "feedback 2026-10-23 — \"what the model saw\" is only honest if it IS the loop's own composer (one shared function, no second implementation), trimmed to the last request's edge, hold-backs named — and pinned byte-equal against the CAPTURED provider request bodies; a tool re-run's answer lands in the drawer and NEVER in the session log, because the log is the model's context"
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

Two audit-surface rulings from the chat/composer build (2026-10-23,
plan `2026-10-23-chat-composer-excellence.md` D10/D11, commits
`ee5b305` + `f813b63`), both generalizable:

**An inspector that paraphrases lies.** "What the model saw" was first
scoped out for fear a bridge-side reconstruction would drift from the
run loop's real assembly — an approximating inspector is worse than
none, because the owner audits against a lie. The fix is structural,
not careful prose: factor the loop's request assembly into ONE
composer (`composed_prompt` + `base_declarations` in agent.rs) with
two callers — the loop and `GET /api/context`. Then the drift question
is decidable by a test, and the only test that settles it is
BYTE-EQUALITY against the captured request bodies of a scripted
conversation (messages and tools arrays), not "looks right". Three
subtleties the test forced: the reconstruction trims trailing
assistant messages past the last user/tool message (the reply is the
log's next record, not part of what the model saw when asked); the
goal/skills ride from their log records (hard rule 5 is what makes the
feature possible); and hold-backs are NAMED — `improve`'s declaration
carries the lesson store's live records, a fact the log cannot hold,
so the reconstruction says it is absent rather than inventing it.

**A re-run is a playground, not a run.** Re-running one recorded call
executes through the tools' own executor and bound (lane calls over
the registry/roster one-call seam, ask-class calls standing a real
seat) — but the result lands in the drawer and NEVER in the session
log, because the log is the model's context and an unasked-for result
would silently enter every future prompt. Pinned by a test asserting
no record contains the output. The write class (`write_file`) is
refused by name with the correction path named (edit-and-resend,
which checkpoints).

**Why:** audit features earn trust exactly to the degree their claims
are checkable; "reconstructed from the log" is a claim, and captured
bytes are its only referee.

**How to apply:** when building any "show what happened inside" view,
share the producer's own code path, pin equality against captured
artifacts, trim to the boundary of what the consumer actually
received, and name what the reconstruction cannot hold. Related:
[[chat-excel-direction-2026-10-23]],
[[a-factored-composition-must-keep-the-exact-texts]] (the refactor
lesson this same build produced).
