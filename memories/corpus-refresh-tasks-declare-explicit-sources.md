---
name: corpus-refresh-tasks-declare-explicit-sources
description: "Every seeded corpus-refresh task now declares the refresh arguments itself — {project, sources: [<repo>/docs/**/*.md, <repo>/*.md]} — reconciled into the live store 2026-10-28 (KI-339, commit ec79da3), so per-store source recovery is obsolete; a project-only call under the lane-home cwd still returns no-source"
type: project
date: 2026-10-28
basis: inferred
tier: durable
shape: [state-sync]
phrases: [action: no-source, missing_sources, sources derived from the store manifest keys, docs/**/*.md plus the three root contract docs, BLEND1_SEM1_PYTHONPATH, seed_task]
missing: []
recall-count: 2
superseded-by:
valid-until:
---

The invocation-level fix has landed (2026-10-28, fleet commit ec79da3): seed_task (src/cron.rs) now declares the refresh arguments in every seeded corpus-refresh job's task string — {"project": "<project>", "sources": ["<repo home>/<project>/docs/**/*.md", "<repo home>/<project>/*.md"]} — and `fleet cron seed` reconciled all fourteen live jobs (KI-277's reconcile path; owner schedules, enabled flags and counters untouched). A literal reading of the task string now works. What remains true: a project-only tools/call under the lane-home cwd still returns action "no-source" (the child's default source set is cwd-relative to services/blend1/, which has no docs/), and the deeper fix — a refresh report naming the cwd its default set resolved from, or a refusal on an empty default derivation — is open in KI-339. Verified on gen1 2026-10-28: the explicit-globs call returned 4 files / 30 items / action unchanged with the store sha256-identical before/after. The alternative route (child cwd = the target repo, project-only args) also works and is the blend1-refresh-spawn-driver-recipe-v2 spec; the two routes are equivalent in outcome.
