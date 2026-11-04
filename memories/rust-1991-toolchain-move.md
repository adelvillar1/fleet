---
name: rust-1991-toolchain-move
description: "The machine's stable Rust moved to 1.99.1 on 2026-11-02 — the gate runs on it, and a rustup update silently breaks the release strip until a symlink is reapplied"
type: project
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

On 2026-11-02 the machine's default `stable` toolchain moved 1.89.0 → 1.99.1
(`rustup update stable`; 1.99.1 is the latest stable, 2026-10-13). Fleet's gate
now runs on it: fmt, clippy `-D warnings`, `cargo build --release`, and
1,996 tests / 0 failed / 6 ignored — all clean at `eabe031`.

Two things a future session needs:

- **41 lints were new in 1.99** (`unnecessary_sort_by`, `manual_is_multiple_of`,
  `chunks_exact_to_as_chunks`, `filter_next`, `collapsible_match`,
  `result_large_err`, …). If a future toolchain move goes red, expect the same
  shape: mostly mechanical fixes, plus `result_large_err` needing a judgment
  call (Fleet allowed the two relay closures — their `ureq::Error` is matched
  and formatted in-function, never propagated).
- **The 1.99.1 toolchain ships a broken `rust-objcopy`.** It is dynamically
  linked against `@rpath/libLLVM.dylib`, but rustup installs that dylib at
  `<toolchain>/lib/` while the binary's rpath (`@loader_path/../lib`) resolves
  to `<toolchain>/lib/rustlib/aarch64-apple-darwin/lib/`. So the release
  profile's `strip = true` aborts with SIGABRT and the binary ships unstripped —
  as a *warning*, so the gate stays green either way. Repaired with a symlink at
  `<toolchain>/lib/rustlib/aarch64-apple-darwin/lib/libLLVM.dylib` →
  `../../../libLLVM.dylib`. **A `rustup update` that replaces the toolchain
  directory removes it** — reapply it, or the strip fails silently. KI-403.

Note the 1.85.1 toolchain also on the machine is unaffected (its `rust-objcopy`
is statically linked), so the defect is specific to the 1.99 line's packaging.
Related: [[the-gate-runs-only-for-code-changes]].

## The move is DONE, and where it landed (2026-11-02)

The owner ruled *"fix now, I have stopped the other session and we will pick it
up once the 1.99.1 fixes are done"* — so the co-tenant was stopped and the move
completed in one pass:

- **All 41 fixed**; the gate is green end to end (fmt, clippy `-D warnings`,
  `cargo build --release`, **1,996 tests / 0 failed / 6 ignored**).
- **Committed as `eabe031`** — *on `face-plugin-host`, not `calib-usage-anomalies`*,
  because the stopped session had switched the checkout before the commit. The
  commit therefore also **carries the plugin host's parked work** (the `eval`
  action and `src/plugins/eval.rs`, the face host's remaining reads/routes in
  `bridge.rs` and `app.js`, the plan and its docs, the demo fixture, the
  consolidation memories). `calib-usage-anomalies` stayed at `140c739`, and
  **`eabe031` is not on `master`** — it reaches the trunk when
  `face-plugin-host` merges. The owner was told and offered a split; no ruling.
- **The stopped session's half-fixes were wrong**, not just unfinished:
  `array_chunks` (not a real method — clippy's own suggestion was `as_chunks`),
  a `sort_by_key` closure destructuring a 2-tuple against a 4-tuple, and a
  let-chain that is edition-2024-only while the crate is edition 2021. **A
  co-tenant's parked work is not necessarily correct work** — verify it before
  building on it.
- **`cargo clean -p fleet` removed 110.4 GiB** (190,748 files) while confirming
  the strip. Regenerable build cache, but a large deletion to name.
- **The strip repair is confirmed working**: the rebuilt binary's symbol table
  carries 178 symbols and a direct `rust-objcopy --strip-debug` on a copy
  changes nothing. Cargo's cached replays still print the old SIGABRT warning
  for targets that were not rebuilt — that warning is stale, not a live failure.
- **KI-403 filed** (402 rows / 11 open; the checker is clean).

**2026-11-02 later — the branch reached the trunk.** `eabe031` was not on
master; the follow-up session finished the plugin host and landed the whole
thing at **`c8dde93`** (origin/master), with the gate re-verified on the
committed state: fmt 0, clippy `-D warnings` 0, `cargo build --release` 0,
**1,996 tests / 0 failed / 6 ignored**, face harness **678 checks / 0 across
62 cases**. The re-verification mattered: the co-tenant's merged commits had
**reintroduced six of the lints already fixed** (see
[[a-parallel-session-can-reintroduce-your-fixed-lints]]) — a clean gate is a
property of the tree at a moment, not of anyone's branch.

**The owner's correction to my "nothing left to merge" worry (2026-11-02).**
I said the two commits on `face-plugin-host` were "the only thing left that
isn't on master" and offered a fast-forward. The owner: *"nothing left to
merge, the tree is supposed to be clean."* **They were right** — `master` and
`origin/master` already carried `eabe031` and everything below it, so there
was no merge to do; the branch was merely a second ref at the same tip plus
two later commits. The lesson: before proposing a merge, read
`git log --oneline master..HEAD` and `HEAD..master` **and** check whether the
commits are already ancestors of the trunk (`git merge-base --is-ancestor`)
— "ahead N" is not the same as "unmerged", and a branch pointing at the
trunk's tip is not a pending merge. The tree was then cleaned with two
commits of its own: `15cfb6a` (the daemon's recall-count bumps) and `f9c6ad6`
(the session recap), leaving `face-plugin-host` 2 ahead / 0 behind.

**2026-11-02, the session's tail — the tree is clean and the branch is ahead
only by docs.** The plugin host was declared done by the owner
(*"the plugin work is done"*; the plan completed at `c8dde93`), so the
remaining commits were the Tauri plan's rewrite (`d90123c`, `104bc06`) and
two more rounds of the daemon's recall-count bumps (`9851283`, `78be9bc`).
**`memories/` will keep dirtying the tree between sessions** — the daemon
bumps recall counts every pass, and the daily-maintenance LaunchAgent normally
commits it at 09:20; a session that finds it dirty should commit it as
`chore(memories)` rather than leave it. Final state: `face-plugin-host` at
`78be9bc`, working tree clean, `master`/`origin/master` at `c8dde93`.
