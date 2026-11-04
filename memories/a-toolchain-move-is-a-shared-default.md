---
name: a-toolchain-move-is-a-shared-default
description: "feedback 2026-11-02 — `rustup update stable` moves the MACHINE's default toolchain, so every co-tenant session inherits it and its gate turns red on the new clippy; a toolchain move is a shared-state change that needs the other session's landing coordinated, not a private upgrade"
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

2026-11-02: to satisfy Tauri 2.12's MSRV (1.90) I ran `rustup update stable`
on the owner's instruction, moving `1.89.0 -> 1.99.1`. It worked — and it
turned fleet's clippy leg red with **41 errors** (39 mechanical new lints, 2
`result_large_err` needing a call) across ~27 files. Because the move is to
the **default** toolchain, the co-tenant session sharing the worktree inherits
1.99.1 and will hit the same 41 on its next gate run — while it is mid-flight
on the plugin host.

**Why:** a toolchain is not a per-session tool on this machine —
`CLAUDE.local.md`'s PATH line points at the `stable` toolchain *directory*, so
every session resolves the same compiler. Moving it is closer to editing a
shared config file than to installing a private tool: the blast radius is
every session's gate, and the new clippy's lints land as someone else's
defects. The owner was told and offered revert / schedule / coordinate, but
the move had already happened.

**How to apply:** before `rustup update` (or any default-toolchain change),
check whether another session is live in the tree — `git worktree list`, a
dirty `git status`, or a recent commit cadence are the tells. If one is, either
move it as its own planned increment (fix the new lints first, so the next
gate run is green) or revert the default until the other session lands. When
the move is the owner's own instruction, say plainly that it is shared state
and name the co-tenant's exposure in the same turn — the owner's ruling
authorizes the change, not the collateral. Same shape as
[[shared-file-commits-can-sweep-another-sessions-work]] and
[[a-parallel-session-co-edits-your-file]]: shared state needs the other
author's outcome read first.

Related: [[rust-toolchain-moved-to-199-and-clippy-red]] (the move and its 41),
[[rust-1991-toolchain-move]] (the completed move and the trunk landing),
[[a-parallel-session-can-reintroduce-your-fixed-lints]] (the co-tenant's
merges reintroducing the fixed lints — the shared-default lesson's second
half),
[[the-gate-runs-only-for-code-changes]] (the gate's exit-code rule).
