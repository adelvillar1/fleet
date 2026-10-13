---
name: fleet-the-gate-is-local-and-its-exit-code-is-read
description: "Fleet's gate is local and permanent: cargo build --all-targets then cargo test, with the exit code read on the cargo invocation itself and never through a status-losing pipe; CI is disarmed by owner decision because the GitHub Actions budget is exhausted."
type: feedback
date: 2026-10-13
basis: confirmed-by
shape: [build]
recall-count: 0
superseded-by:
valid-until:
---

Run the gate before every push and read the exit code directly — PIPESTATUS or the tool's own exit_code — because a pipe reports the last command's status, not cargo's. rustfmt and clippy are absent on this machine (rustup profile minimal, KI-007), so the style is matched by hand and the build must stay warning-free.

Source: CLAUDE.md hard rule 2 and the environmental blockers, 2026-10-13.
