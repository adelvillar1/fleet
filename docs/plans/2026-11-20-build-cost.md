---
status: completed
created: 2026-11-20
updated: 2026-11-20
slug: build-cost
---

# Build cost: make the gate cheap enough to run

## Goal

Reduce the time, memory, and disk a full gate run costs on the owner's
laptop, without weakening what the gate proves.

## Context

Measured on the owner's machine (16 GiB RAM, 10 cores, APFS), 2026-11-20:

- **A cold release build took 20m21s** in a fresh worktree (its own
  `target/`), and a warm one 3m36s–5m30s. Two such builds ran today.
- **`target/` reached 25 GiB**: `debug/deps` 13.9 GiB, `debug/incremental`
  7.2 GiB (9.8 GiB at its peak), `release` 2.6 GiB.
- **The machine ran out of RAM during a gate run**: 0.07 GiB free, 5.5 GiB
  wired, 2.7 GiB compressor, **16.1 M cumulative pageouts** — and a test
  that asserts a run completes failed because Fleet's admission control
  (`footprint::admit`) correctly refused to start it at 0.38 GiB of
  headroom. That is a real user-visible symptom: the harness refuses work
  on this machine while a build is in flight.

Five causes, in order of what they cost:

1. **Fat LTO on two binaries.** `[profile.release] lto = true` with
   `strip = true`. Each `cargo build --release` LTO-links both `fleet` and
   `fleet-desktop`; it is the single longest phase and the largest peak
   memory consumer.
2. **The Tauri shell is in the default members.** `default-members =
   [".", "src-tauri"]` means **every** `cargo test` and
   `cargo clippy --all-targets` also builds `fleet-desktop` (tauri, objc2,
   window-vibrancy, minisign-verify). The native-shell plan's C0 asked for
   exactly this, so the gate covers both crates — but it also means a
   lib-only test run pays for a desktop shell.
3. **No `[profile.dev]` at all**, so dev builds carry full debuginfo
   (`debug = 2`) and incremental compilation. That is the 13.9 + 7.2 GiB.
   For a crate this size incremental's hit rate is poor: it costs disk and
   a fingerprint check on every run.
4. **One 267K-line library.** Any edit to any module recompiles the whole
   crate and relinks both binaries; there is no unit boundary to save you.
   (`2026-11-20-bridge-module-split.md` and `appjs` address the two worst
   files, not this.)
5. **No shared cache.** `sccache` is not installed, so a fresh worktree —
   which is how parallel work is supposed to happen here — pays a full
   cold build every time. Today's 20-minute build was that, twice over.

## Approach

Ordered by (effect ÷ controversy):

- **Step 1 — dev profile, no behaviour change.** Add
  `[profile.dev] debug = 1` (line tables, not full type info) and
  `incremental = false`. Expected: `target/debug` from 22 GiB to roughly
  a third; rebuilds on this crate no slower in practice. Pure disk win.
- **Step 2 — a shared compiler cache.** Install `sccache`, point
  `~/.cargo/config.toml` at it (`build.rustc-wrapper`). A fresh worktree
  then reuses dependencies instead of recompiling 300 crates. This is
  user-level config, not a repo change, so it needs no ruling — but it
  must be measured before it is claimed.
- **Step 3 — cap build parallelism when the machine is busy.**
  `CARGO_BUILD_JOBS=3` (or `-j 3`) keeps peak RSS down; the machine has
  16 GiB and ten rustc workers will happily take more than it has. Cheap
  to test: time a build at `-j 10` against `-j 3`.
- **Step 4 — a gate ruling on LTO.** `lto = "thin"` costs a few percent of
  runtime and saves minutes per build and a large share of the peak memory.
  Alternatively a custom `[profile.gate] inherits = "release"` with
  `lto = "thin"` that the gate builds, leaving the shipped artifact on fat
  LTO. **This is the owner's call** — the shipped binary's speed is a
  product decision, and `strip = true` + fat LTO was chosen deliberately.
- **Step 5 — a fast loop that does not pretend to be the gate.** Document
  (and wire into `scripts/`) a `cargo test -p fleet --lib` /
  `cargo check` loop for the inner cycle, keeping the full workspace gate
  where it is. This does not change what the gate proves; it changes what
  a session runs forty times a day.

## Acceptance Criteria

- [x] `[profile.dev]` is set: `debug = 1`, `incremental = false`.
      `target/debug` **22.2 GiB → 3.9 GiB**; a full test-target rebuild from an
      empty debug dir 95s.
- [x] A cold build is measured both ways. Fat LTO, no cache, fresh worktree:
      **20m21s**, twice, on 2026-11-20. Thin LTO + `sccache` + six jobs, with
      `target/release` deleted: **2m14s**, 454 of 1,000 compile requests served
      from cache. Warm relink: 3m45s → 3m07s.
- [x] Peak memory is measured: the relink phase peaks at **2.21 GiB RSS**.
      Before this change the machine reached 0.07 GiB free with 16.1M pageouts
      and Fleet refused a run mid-gate; six workers is the cap that keeps the
      rest of the work alive beside a build.
- [x] The gate still covers both crates — `default-members` is untouched, and
      the four legs run whole on the tree with `[build] jobs = 6`.
- [x] `CLAUDE.md` names both loops and says what a release build costs
      (3m45s warm, 20m21s cold before this change).

## Out of scope

- Splitting the lib (its own plans).
- Changing `strip = true`.
- Caching beyond the compiler (Docker layers, remote caches).

## Files to be touched

- `Cargo.toml` — the profiles.
- `~/.cargo/config.toml` — sccache and job caps (user-level).
- `CLAUDE.md` — the two loops.
- `scripts/` — the fast-loop entry if one is warranted.
