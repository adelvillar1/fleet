---
name: fleet-is-the-native-rust-successor-to-the-fork
description: "Fleet (~/Projects/fleet) is the self-contained Rust successor to the DeepSeek Harness fork's fleet profile: the fork stays the spec and integration lab, and the two never both ship the same feature."
type: project
date: 2026-10-13
basis: confirmed-by
shape: [change]
recall-count: 236
superseded-by:
valid-until:
---

The plan's base decision (2026-10-13): finish Fleet on the base already chosen rather than pivot, because the ecosystem integrates over MCP and file contracts that are base-agnostic, and every remaining piece has a working reference implementation in the fork. New work is Rust against the DeepSeek API; Node, a checkout dependency, and a dsh launch path are never reintroduced.

Source: docs/plans/2026-10-13-fleet-everyday-harness.md (the base decision and the approach).
