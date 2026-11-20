---
status: completed
created: 2026-11-20
updated: 2026-11-20
slug: corpus-root-resolver
---

# One resolution for the fact corpus' root

KI-074 found the cwd walk adopting a foreign corpus (a `~/.hermes` tree
read as Fleet's memories); KI-087 found the same shape in `cron`, and its
close note says the cron path "inherits the rule by construction (it is a
deliberate mirror of `drive`)". Two mirrors of one rule is the defect
waiting to happen a third time.

## Context

- The rule, as fixed: resolve `FLEET_HOME`'s `memories/` first when it
  exists **and holds facts**, the cwd walk second (the repo's own corpus
  in development).
- **Status pass, 2026-11-20 (verified in the tree):** the resolver is
  already one function with a testable seam —
  `memory::resolve_root(cwd)` reads the home itself and delegates to
  `memory::resolve_root_from(cwd, home)` (`src/memory.rs:957-1000`),
  which holds the home-first rule and the `holds_facts` scaffold guard.
  Both callers use it: `main.rs`'s `drive` (`src/main.rs:481`) and
  `cron::execute` (`src/cron.rs:1681`). KI-074 and KI-087 are closed by
  it. So the first two steps below are already true; this plan's
  remaining work is the guard and the wiring test.
- The corpus is load-bearing: 965 fact files, 964 indexed at this
  session's measurement.
- The failure has a real blast radius: a scheduled run outside any
  intended corpus silently adopts another tool's memory store and nothing
  in the run's output says which root it took.

## Status pass, 2026-11-20

Verified first, then built the missing piece: the resolver already existed
as one function with a testable seam, and both callers used it, so KI-074
and KI-087 were already closed by construction. What the plan still owed was
the guard, which now exists and was worth writing — see the acceptance
criteria for the six modules it found on its first run.

**Still owed:** AC2 — the root's *reason* (`home` / `walk` / `absent`) is
not exposed to the caller, so a run does not yet log which corpus it took.
That is a signature change at the resolver and both call sites, plus the
log line.

## Approach

1. One function — `memory::resolve_root(home, cwd)` (the name the module
   already uses) — holding the precedence exactly as today, with the
   chosen root returned alongside its *reason* (`home`, `walk`,
   `absent`) so a caller can log which rule fired.
2. Both call sites call it; neither keeps its own copy of the walk.
3. A text-scan guard in the structure-guards style: no module outside
   `memory.rs` resolves a `memories/` path itself — the same
   `include_str!` census shape the beat guards use (and the guard-list
   inversion plan makes such a guard account for new modules by default).
4. Tests: a scratch home with its own corpus wins over a cwd corpus; a
   home without `memories/` falls back to the walk; a foreign
   `~/.hermes`-shaped tree under the cwd is never adopted when the home
   carries a corpus; `cron::execute` and `drive` resolve to the same root
   for the same inputs.

## Acceptance Criteria

- [x] Exactly one implementation of the precedence rule exists, and both
      callers use it: `memory::resolve_root_from` holds the home-first walk
      with its `holds_facts` guard; `main.rs:487` (`drive`) and
      `cron.rs:1681` (`cron::execute`) both call `resolve_root`.
- [x] The root's reason (`home` / `walk` / `absent`) is available to the
      caller and logged where a run names its corpus: `RootReason` and
      `RootChoice` in `memory`, with `resolve_root`/`resolve_root_from`
      delegating to `resolve_root_choice*` so no call site changed meaning.
      `fleet chat` prints `fleet: corpus <path> (<rule>)`; `fleet run` writes
      it to stderr, because that command's stdout is the event stream.
      `the_resolver_names_the_rule_it_applied` pins all three reasons, and
      pins that the home's empty scaffold does not win (KI-074's second
      half).
- [x] A text-scan guard reds when another module builds a `memories/` path:
      `memory::tests::the_corpus_root_has_one_rule`, which also pins the
      resolver to one definition. Writing it found six modules that join the
      name themselves — one sanctioned (`corpus::corpus_root`, the sync's
      unconditional root per D29) and five test scaffolds — every one now an
      exemption with its reason written beside it.
- [x] The resolver's own tests stay green, including the two pinned
      reproductions from KI-074/KI-087 (`resolve_root_from`'s tests).
- [x] The full gate exits 0: fmt 0, clippy 0, `cargo test` 2654 passed /\n      0 failed, `cargo build --release` 3m39s.

## Files to be touched

- `src/memory.rs` — the resolver and its reason
- `src/main.rs` — `drive` calls it
- `src/cron.rs` — `execute` calls it
- `docs/architecture/structure-guards.md` — the guard's entry

## Out of scope

- Changing the precedence rule itself.
- The session directory's and the calibration root's resolutions (they
  already use the home; this plan makes the corpus agree with them).

## Verification

1. `cargo test -p fleet --lib memory` and the cron module scoped.
2. The two historical reproductions (KI-074's scratch-cwd probe and
   KI-087's cron probe) re-run as tests.
3. The full four-leg gate once, at the end.

## Linked artifacts

- KI-074, KI-087 — the two rows this plan closes the class for.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F3.2.
