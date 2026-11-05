---
name: fleet-gui-skills-manager-ruling
description: "Owner-endorsed 2026-10-16 ruling on xingkongliang/skills-manager — adopt its UX concept (card library, badges, preview, decisions queue, search; extended to memories) but refuse the dependency (Tauri+React+ SQLite+Node violates the crate contract and a second store is a second truth)"
type: project
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

Asked "can we use something like this for the skills?
(github.com/xingkongliang/skills-manager)" and then "you can probably
extend the concept to memories as well", the ruling that shaped G5
(dfdaefb): the concept transfers, the dependency does not.

What transfers — and G5 is built on it: a card library with per-item
badges (loaded/skipped, kind, usage count), preview-before-acting in the
inspector, a decisions queue as a first-class card (the proposals
awaiting an approve), an activity trail (fleet already has one — the
calib stream's `propose`/`recall_hit` rows), and search. Extended to
memories, the panel gets something skills-manager has no analogue for:
the recall-hit trail.

What does not: Tauri 2 + React + SQLite + a Node dev toolchain against
"one static Rust binary, no Node runtime, no checkout dependency";
SQLite as a store when the substrate IS the filesystem (one fact per
file, one directory per skill) — a second store would be a second truth;
and marketplace/git install plus deploying to 50 harnesses, which is the
opposite of the own-home ruling — fleet consumes `~/.agents/skills` and
never writes another harness's config. A second desktop window would
fragment the one console surface the owner asked for.

**Why:** "use something like this" was a question about design shape, not a
request for a dependency; the crate contract and own-home ruling decide
the boundary.

**How to apply:** when a future increment wants richer skill/memory
management, extend the in-repo panels in the skills-manager UX shape;
never introduce a GUI toolchain, a database store, or cross-harness
writes. Related: [[gui-revamp-browser-face-decision]],
[[canonical-skills-library-agents-skills]].
