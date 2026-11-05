---
name: a-test-that-only-passed-while-the-code-refused
description: "feedback 2026-10-30 — the improve CLI tests never pinned FLEET_SKILLS_DIR, so a test fixture applied into the REAL ~/.agents/skills the moment the depth rule made skill applies succeed; isolation held only by a refusal is not isolation"
type: feedback
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Feedback 2026-10-30 (fleet, the skills-rail build). The `tests/improve.rs`
CLI helper set `FLEET_HOME` but not `FLEET_SKILLS_DIR`, so the binary
resolved the canonical library to the real `~/.agents/skills`. It never
mattered while every skill apply refused (KI-374: no cargo on PATH) — the
test took the refusal branch and passed. The moment the depth rule made
applies succeed, the green branch ran and the test's `demo` fixture was
WRITTEN INTO THE OWNER'S REAL LIBRARY.

**Why:** a test that only passes because the code refuses is vacuously
isolated; the refusal was masking the missing pin, and fixing the product
code removed the mask.

**How to apply:** when a test exercises a writer, pin every env-resolved
root the writer touches (`FLEET_SKILLS_DIR` beside `FLEET_HOME`), even —
especially — if the code currently refuses before reaching it. When a fix
opens a previously-dead path, audit which tests were passing on the dead
path. And if a fixture does reach the real store, move it out (mv to /tmp,
reversible) and check the count — see also
[[fleet-memories-tests-need-their-own-home]], the same class on the
memory side.
