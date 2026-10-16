---
name: deliverable-cadence-report-per-unit
description: "The owner's cadence: report at each completed deliverable, halt immediately when a finding changes the plan's premises, and commit only tested code or a verifiable document"
type: feedback
date: 2026-10-16
basis: confirmed-by
tier: durable
shape: [change, docs]
phrases: [per deliverable, commit only after proper testing, verifiable document, stop when a finding changes the plan's premises, rows: 218 open: 85 PROBLEMS (8), turns end on raw tool output with no verdict]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's cadence ruling (2026-10-17): a turn ends when a deliverable is complete, and the completion is reported in that same turn — never on raw tool output with no verdict attached.

A deliverable is one unit of: committed code (which requires a green gate — `cargo fmt --all --check`, `cargo build --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test`, exit codes read on the cargo invocation itself, never through a pipe), a verifiable document (markdown, a plan, a ledger row — verified by the check that can prove it, e.g. `scripts/ki_ledger.py` for the ledger), or a plan file.

The rule exists because two turns ended without a verdict: a prior session cut off with no handoff, and a turn that ended on a wall of parser output whose meaning (`rows: 218 open: 85 PROBLEMS (8)`) was never stated.

Companion rule from the same ruling: a turn also halts immediately — mid-deliverable — when a finding changes the plan's premises. HEAD `c31c2d3` not compiling was such a moment and was only surfaced because the owner asked "what's next".

So there are exactly two turn-ending modes: a deliverable completed (report it), or a premise changed (halt and report). Everything else flows without stopping.
