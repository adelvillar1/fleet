# Slice 12 — self-maintenance under gates (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 12).
**Feature:** `docs/features/self-maintenance.md`. **Ledger:** KI-097..KI-103.

## What landed

`src/improve.rs` (new, 2,441 lines with 25 unit tests) — the self-maintenance
loop under gates — plus wiring in `tools.rs`, `agent.rs`, `cli.rs`,
`main.rs`, `calib.rs`, `route.rs`, `skills.rs`, `cron.rs`, `subagents.rs`,
`tui/mod.rs`, `lib.rs`; `tests/improve.rs` (new, 7 CLI e2e); two tool-path
tests in `tests/integration.rs`; the feature doc; the plan's record.

- **The proposal.** A file per proposal under `<FLEET_HOME>/proposals/
  <name>.md`: frontmatter (name, target — skill|memory|source|config —
  target-name, basis, created, status) and a body carrying the
  justification, the acceptance criteria, the expected effect, and the
  change as typed ops (`write-skill`, `retire-skill`, `supersede`,
  `forget`, `config`, `config-remove`, `patch`). Slice 7's drafter
  drafts in `skills/proposals/` are the skill-proposal feedstock,
  adapted at read time, never migrated (KI-098 names the two stores).
- **The gate.** In the repo root, in order: `cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
  `cargo test`. Exit codes read directly from each step's own status —
  never a pipe — and every step runs. **A step that cannot run is a
  named refusal, never a silent pass**: on this machine, with rustfmt
  absent, every proposal fails closed at `fmt` (KI-097). That is the
  correct consequence — a gate that can't run isn't green — and it
  means the loop currently cannot apply anything here, which the
  ledger says plainly.
- **Apply, revert, reversibility.** Source: write, gate, and on red
  `git checkout HEAD -- <paths>` with the failure named; on green an
  explicit-path commit whose message names the proposal (and carries a
  `fleet improve:` marker, so a machine-made commit is self-describing);
  undo is `git revert`. Memory: through slice 8's `Memory::supersede` —
  the provenance rule wired, not reimplemented — with the calib stream
  deliberately withheld so no `consolidate` row survives a gate revert.
  Home-side targets (skills, config): the pre-image is stashed in
  `proposals/preimages/` and restored byte-for-byte; a retirement moves
  the skill directory aside rather than deleting it.
- **Destructive changes wait.** A proposal that retires a skill, forgets
  a fact, or removes a config entry waits for explicit approval in the
  current turn: status `waiting`, nothing applied, the gate never run,
  the message naming exactly what will be destroyed. `--approve` is a
  per-process flag — no persisted approval, no stale vector.
- **The surface.** `fleet improve` (list: target, basis, status, last
  gate result — house cards), `apply`, `undo`; and an `improve` TOOL
  declared beside the others that only FILES proposals — the model
  cannot apply anything, and the tool's own description says so. Config
  proposals validate against the roster's `ProviderEntry`
  (`deny_unknown_fields`). Subagent children and cron runs do not carry
  the tool.
- **The ledger.** A `propose` calib op: one row per verdict
  (applied|refused|waiting) with the reason and the gate's per-step
  evidence; a proposal that never reached a verdict records nothing —
  the filing itself lives in the session log (hard rule 5).

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **412 unit + 7 (tests/
  improve.rs) + 90 integration passed, 1 ignored** in the final state
  (baseline 385 + 88; nothing deleted). `git status --porcelain
  memories/` empty after the gate. Cargo.toml/Cargo.lock unchanged.
- The five ACs, each with its test: a justified skill update applies
  through a green stand-in gate and lands the row; a failing gate is
  refused with the failing test named in both the reason and the row,
  the tree at its pre-image; a destructive proposal waits with the
  destruction named and the gate unrun; an applied change reverts
  cleanly on both paths (git revert, byte-for-byte pre-image restore);
  a source proposal runs all four steps before it survives.
- Real-binary evidence on this machine is the fail-closed path: the
  gate's own `error: no such command: 'fmt'` in the refusal and the
  row. A real green gate is impossible here (KI-007) and is disclosed
  as stand-in-covered rather than claimed.

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all ten checks;
  the five ACs mapped to tests; the deviations (route.rs for
  `ProviderEntry`, skills.rs for parser reuse) in scope and disclosed.
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — one Critical,
  one Important, both fixed in `e7b1732`:
  1. **the Critical**: the `patch` op accepted any repo-relative path,
     including `memories/<fact>.md`, so a model-filed proposal could
     rewrite a confirmed-by fact with no approval — the provenance rule
     that is this module's permission model, bypassed in one line of
     JSON. A patch whose first component is `memories/` is now refused
     naming `supersede` as the only door, with a regression test
     asserting the refusal, the untouched file, and that the gate never
     ran (KI-102);
  2. **the Important**: the apply overwrote a target before the gate ran
     and a red gate's revert restored HEAD, so pre-existing uncommitted
     work in the target was silently destroyed. A dirty target is now
     refused naming the work, on both the patch and memory paths
     (KI-103).
- **Both reviews dispatched in ONE parallel call** — the pattern's
  fourth use.

## Not done (slice 13+)

- Diagnosis (13), the boards (14), install (15).
- KI-097's real fix (a toolchain with rustfmt/clippy, so the gate can
  actually run green here), KI-098's store unification, KI-099's
  repo-root resolution, KI-100's pre-image namespace, KI-101's git
  identity.
- The minors logged or disclosed: the plan prose's "a diff" rides inside
  the patch change JSON but is not surfaced; a merge draft's card shows
  the survivor as target; a draft's `created` is forced to today.
