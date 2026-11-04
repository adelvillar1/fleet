---
name: rust-toolchain-moved-to-199-and-clippy-red
description: "project 2026-11-02 — the owner ruled "aim for the latest stable rust 1.99.1"; the machine moved 1.89.0→1.99.1 (rustup update stable), fmt passes, clippy -D warnings fails with 41 errors (39 mechanical new lints, 2 result_large_err needing a call), and the move is a shared-default change that also moves a co-tenant session's gate"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner ruled (2026-11-02, on the Tauri research): *"can we aim to have the
latest stable rust version 1.99.1?"* — and it is the precondition for current
Tauri (2.12's MSRV is 1.90, policy "stable − 3"). Done the same turn:

- `rustup check` had already reported the gap before anything was touched:
  `1.89.0 (29483883e 2025-08-04) -> 1.99.1 (58cf8f9c3 2026-10-13)`.
- `rustup update stable` moved the machine's **default** toolchain;
  `rustc 1.99.1` / `cargo 1.99.1` active. `CLAUDE.local.md`'s PATH line points
  at the `stable-aarch64-apple-darwin` directory, so it follows with no doc
  change. `rustup toolchain list` also carries a `1.85.1` toolchain.
- **The gate on 1.99.1**: `cargo fmt --all --check` **passes**;
  `cargo clippy --all-targets -- -D warnings` **fails — 41 errors**.

**The 41 are new lints in 1.99's clippy, not regressions in fleet** — ~39
mechanical, 2 needing a call:

| Lint | n | Fix |
|---|---|---|
| `unnecessary_sort_by` | 12 | `sort_by_key` |
| `manual_is_multiple_of` | 10 | `is_multiple_of()` (new std method) |
| `chunks_exact_to_as_chunks` | 3 | `as_chunks()` |
| `useless_vec` | 2 | slice |
| `filter_next` | 2 | `rfind` / `find` |
| `collapsible_match` | 2 | collapse |
| `result_large_err` | 2 | **judgment** — `src/remote/relay.rs:409`, `src/search.rs:342` |
| `needless_range_loop`, `manual_checked_ops` | 2 | iterator / `checked_*` |
| rustc: unused imports, unused `mut`, unreachable pattern | 3 | `src/explain.rs:44`, `src/anomalies.rs:99`, `src/calib.rs:672` |

~27 files. Worth doing — it is the precondition for current Tauri — but it is
its own increment under hard rule 1, not something to fold into a feature.

**Two things the turn exposed, both worth carrying:**

1. **The tree is not the one warmed up on.** The co-tenant session landed the
   entire usage plan mid-research (Increment A `0253e89`, B `7aa0544`, C
   `43b37e5`, criteria ticked `140c739`), completed settings-ux and the
   closeout (`a061ae8`), and is mid-flight on the **plugin host** (`dea8393`,
   `a1b7f5c`). The clippy run therefore measured *their* tree —
   `src/anomalies.rs:99` is their fresh code. Same lesson as
   [[a-parallel-session-co-edits-your-file]] and
   [[a-memory-index-line-is-not-the-plans-state]]: re-read the tree before
   attributing a defect to anyone.
2. **`rustup update stable` is a shared-default change.** Every session on the
   machine inherits 1.99.1, so the co-tenant's next gate run hits the same 41
   errors — a move that disrupts a live session mid-plugin-host. The owner was
   told and offered revert / schedule / coordinate; no ruling yet.

**Process note (KI-014's exact trap):** the first clippy run piped to `tail`,
so the exit code read was tail's 0 while clippy exited 101. Re-run without
the pipe to get the true code.

**RESOLVED the same day** — the owner stopped the co-tenant session and ruled
*"fix now"*; all 41 are fixed, the gate is green, and the work is committed at
`eabe031` (on `face-plugin-host`, not `master`). The full outcome, including the
wrong half-fixes the stopped session had left and the `rust-objcopy` defect, is
in [[rust-1991-toolchain-move]] — read that one for current state; this file
survives as the move's record.

Related: [[tauri-native-shell-research-2026-11-02]] (the reason for the move),
[[the-gate-runs-only-for-code-changes]] (the gate's scope and exit-code rule),
[[verification-depth-matches-the-change]].
