---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: gate-fmt-cleanup
---

# The gate's fmt leg: clear the nine-file debt

Fixes KI-571. The four-leg gate is the only gate (CI is disarmed), and
its first leg is red at HEAD on files no session owns — so no session can
run the gate whole without formatting another session's code.

## Context

Measured 2026-11-20 twice with identical results: `cargo fmt --all
--check` exits 1 with **36 hunks across 9 files**:

| File | Hunks |
|---|---|
| `src/armada/join.rs` | 14 |
| `src/skills.rs` | 5 |
| `src/security.rs` | 4 |
| `src/main.rs` | 3 |
| `src/corpus.rs` | 3 |
| `src/armada/mcp.rs` | 3 |
| `src/armada/control.rs` | 2 |
| `src/identity.rs` | 1 |
| `src/gui/bridge.rs` | 1 |

Nineteen of the 36 are in `src/armada/` — the newest feature wave landed
unformatted. One file, `src/skills.rs`, is also modified by the session
in flight at this plan's writing, so it is the one file this commit must
not take.

## Approach

1. Run `rustfmt --edition 2021` over the eight files that carry no
   in-flight work, explicit paths only — `src/armada/join.rs`,
   `src/security.rs`, `src/main.rs`, `src/corpus.rs`,
   `src/armada/mcp.rs`, `src/armada/control.rs`, `src/identity.rs`,
   `src/gui/bridge.rs`.
2. One commit, message `style(fmt): the armada wave's formatting debt`,
   stage the eight explicit paths, and prove the diff is whitespace-only:
   `git diff -w <sha>^ <sha> -- src` must print nothing.
3. `src/skills.rs` is taken in the same shape once the in-flight work
   lands — by that session in its own commit if it formats as it goes, or
   by the next session as a one-file follow-up. The plan is not complete
   while `cargo fmt --all --check` is non-zero.
4. If a hunk looks semantic (rustfmt only reflows, but a stray `#[allow]`
   or a moved attribute can hide one), stop and treat it as its own
   ledger row rather than folding it into this commit.

## Acceptance Criteria

- [ ] `cargo fmt --all --check` exits 0 with a clean tree; exit code read
      directly, never through a pipe (hard rule 2).
- [ ] The eight-file commit's semantic diff is empty: `git diff -w`
      between the parent and the commit prints nothing for `src`.
- [ ] The commit touches only `src/` formatting — no test, doc, or
      dependency change rides it.
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo test` are at
      least as green as the tree was before the commit (formatting cannot
      change either, but the gate is run once, whole, to prove it).
- [ ] KI-571 is closed in `docs/known-issues.md` with the commit named.

## Files to be touched

- `src/armada/join.rs`, `src/armada/mcp.rs`, `src/armada/control.rs`
- `src/security.rs`, `src/main.rs`, `src/corpus.rs`, `src/identity.rs`
- `src/gui/bridge.rs`
- `src/skills.rs` — the follow-up file, after its in-flight work lands
- `docs/known-issues.md` — KI-571's close text

## Out of scope

- Any `rustfmt.toml` change (no config exists; the default is the rule).
- Reformatting files that are already clean.
- Folding any semantic fix into the commit — separate increments.

## Verification

1. `cargo fmt --all --check` before (36 hunks / 9 files) and after (0), exit
   codes read directly.
2. `git diff -w` on the commit — empty.
3. The full four-leg gate once, at the end, in the eight-file tree.
4. `python3 scripts/ki_ledger.py docs/known-issues.md` green with KI-571
   closed.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F1, the
  measurement this plan acts on.
- KI-571 — the open ledger row with the same next action.
