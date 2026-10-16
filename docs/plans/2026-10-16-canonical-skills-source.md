---
status: completed
created: 2026-10-16
updated: 2026-10-16
slug: canonical-skills-source
---

# Plan: the canonical skills library is the load path

## Context

The owner's ruling (2026-10-16): the canonical source of skills is
`~/.agents/skills` — the cross-harness library, so skills are portable
no matter which harness loads them. Fleet's loader reads
`<FLEET_HOME>/skills/` only; today a skill reaches fleet through a
manual install (the `harness-orientation` symlink is the stopgap). Two
facts size the change:

- The library holds 444 `SKILL.md` files; 227 of them carry frontmatter
  beyond fleet's flat `key: value` dialect — a key whose value is empty
  followed by nested indented lines (`metadata:` under
  `project-warmup`) or column-0 `- item` list entries (`platforms:`
  under `rss-feeds`). Fleet's hand parser rejects every non-`key: value`
  fence line, so over half the library would skip by name.
- Fleet writes skills too: the drafter files proposals under
  `<FLEET_HOME>/skills/proposals/`, and the improve gate applies them
  into the live tree. Applied proposals must land in the canonical
  library — a fleet-private second home is the drift this ruling
  exists to end.

## Approach

Resolve the canonical directory at one seam and teach the parser the
library's dialect:

1. `skills_dir` becomes the canonical resolution:
   `$FLEET_SKILLS_DIR` when set and non-empty, else
   `<user home>/.agents/skills`. Both inputs are parameters (the
   `fleet_home` pattern — callers decide their own environment
   precedence); the CLI and TUI boundaries read the env once, and tests
   pass overrides directly, never mutating process env.
2. `proposals_dir` decouples: `<FLEET_HOME>/skills/proposals/`, fleet's
   own draft staging. The `<FLEET_HOME>/skills` directory keeps
   existing only to hold proposals.
3. The frontmatter parser accepts a continuation block: a `key:` whose
   value is empty followed by indented or `- ` lines loads, the block
   riding the key as one opaque value. A continuation line with no
   preceding key is still rejected by name; an empty value with no
   continuation block is still rejected by name.
4. The improve gate's apply and retire paths target the canonical
   library; pre-images stay stashed under `<FLEET_HOME>` so undo stays
   fleet-local and reversible.
5. Curation's retirement rule is a no-op against the canonical source
   with one logged line naming why: the usage scan reads fleet's
   session logs only, and "unused by fleet" is not unused across
   harnesses. The merge rule (description similarity) is unchanged.
6. The `~/.fleet/skills/harness-orientation` symlink is removed — the
   library is the load path, the indirection is gone.

## Use cases

- [x] A: A skill authored in the canonical library loads into every
      fleet run's declared list without an install step.
- [x] B: A library skill carrying nested or list frontmatter loads
      rather than skipping by name.
- [x] C: A drafted proposal applies into the canonical library and is
      loadable on the next run; its undo restores the pre-image.
- [x] D: Fleet redirected by `$FLEET_SKILLS_DIR` reads that directory
      whole (the seam tests and sandboxed homes use).

## UX routes

No new surfaces. `fleet skills` cards now name canonical paths; the
soft-cap line (50) will fire against a 444-skill library — a warning by
design, and the known cost of portability, recorded here rather than
hidden.

## Acceptance criteria

- [x] A1: skills load from the canonical library — `$FLEET_SKILLS_DIR`
      when set and non-empty, else `<user home>/.agents/skills` — both
      resolution inputs taken as parameters.
- [x] A2: the parser accepts the library's dialect (empty value +
      indented or `- ` continuation block loads as one opaque value);
      a leading continuation and an empty value with no block are
      still rejected, each with its reason named.
- [x] A3: `proposals_dir` is `<FLEET_HOME>/skills/proposals`, decoupled
      from the canonical directory; the drafter, curation, and
      `--show` read and write there.
- [x] A4: the gate's apply and retire paths target the canonical
      library, pre-images still under `<FLEET_HOME>`.
- [x] A5: curation's retirement rule files nothing against the
      canonical source, one logged line names why, and a KI row records
      the broken premise and its follow-up.
- [x] A6: the gate runs whole — `cargo fmt --all --check`,
      `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` — exit codes read directly,
      no new warnings.
- [x] A7: against the real library, `fleet skills` cards the loadable
      skills and the log names the skipped ones; the counts land in
      this plan's convergence section.
- [x] A8: the records say the new seam — `docs/features/skills.md`,
      `docs/harness-orientation.md`, `CLAUDE.local.md`,
      `src/install.rs`'s layout comment, the corpus fact amended — and
      the symlink is gone.

## Files to be touched

- `src/skills.rs` — the resolution seam, the parser dialect,
  `proposals_dir`, tests redirected through the parameter
- `src/improve.rs` — apply/retire target the canonical dir, tests
- `src/main.rs` — env read at the boundary, `load` call sites
- `src/tui/mod.rs` — same, for the TUI's load
- `src/install.rs` — the layout comment (`skills` now holds proposals)
- `src/curation` retire path (in `src/skills.rs`) — the no-op + line
- `docs/known-issues.md` — the retirement-premise KI row
- `docs/features/skills.md`, `docs/harness-orientation.md`,
  `CLAUDE.local.md` — the seam in words
- `memories/canonical-skills-library-is-agents-skills.md` — amended
- `~/.fleet/skills/harness-orientation` — symlink removed (not a repo
  file)

## Out of scope

- Per-skill provenance or adopt markers (which harness installed what).
- Subsetting the declared list under the soft cap; curated disclosure
  is future work.
- Any writer into the library other than the gate's apply path.
- The library's own version control (it is not fleet's repo).

## Verification

**A1/A3:** unit tests — resolution prefers the override, falls to
`<user home>/.agents/skills`, and `proposals_dir` stays under the fleet
home when the override points elsewhere.

**A2:** parser tests — the `project-warmup` shape (nested `metadata:`)
and the `rss-feeds` shape (`platforms:` list) load; a fence opening
with a continuation line and a bare `key:` with no block are rejected
with named reasons.

**A4:** improve tests — apply writes under the canonical dir and the
undo restores from the fleet-local pre-image.

**A6:**
```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```

**A7:** `./target/release/fleet skills` against the real library;
skipped names counted from the log.

## Linked artifacts

- `docs/features/skills.md` — the layout section rewritten
- `docs/known-issues.md` — the retirement-premise row
- `docs/harness-orientation.md` — the skills seam row
- `CLAUDE.local.md` — the `~/.agents/skills` machine-path line

## Risks

- The 444-skill declared list rides every run's prompt; the soft-cap
  line fires. Accepted for this slice (portability was the order);
  curated disclosure is the named follow-up.
- Parser tolerance must not swallow real malformations: the two named
  rejections stay, and tests pin them.

## Dependencies

- `docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` — the
  one-home doctrine this plan extends to skills.

## Notes

The stopgap this plan retires: `harness-orientation` was authored in
the library and symlinked into `<FLEET_HOME>/skills/` (commit `bf454a7`
records that state). The symlink comes out in the same increment the
loader starts reading the library directly.

## Convergence (graded 2026-10-16)

Commit `0fb609c`. The gate ran whole at the increment: `cargo fmt
--all --check` exit 0, `cargo clippy --all-targets -- -D warnings`
clean, `cargo build --release` clean, `cargo test` exit 0 — 1019
passed / 1 ignored at the increment's own run, 1022 / 1 with the
routing increment's tests beside it.

- **A1 met** — `resolve_skills_dir(env_value, user_home)` is the seam:
  `$FLEET_SKILLS_DIR` when set and non-empty, else
  `<user home>/.agents/skills`; both inputs are parameters, the CLI and
  TUI boundaries read the env once, and no test mutates process env.
  Pinned by `the_canonical_directory_resolves_override_then_user_home`.
- **A2 met** — the parser accepts the continuation dialect; pinned by
  `a_nested_metadata_block_loads_as_one_opaque_value` (the
  `project-warmup` shape), `a_list_block_loads_as_one_opaque_value`
  (the `rss-feeds` shape), `a_leading_continuation_line_is_rejected_by_name`,
  and `an_empty_value_with_no_block_is_rejected_by_name`. The old
  block-scalar rejection case became a positive continuation assertion.
- **A3 met** — `proposals_dir(home)` is `home/skills/proposals`,
  decoupled; pinned by `proposals_stay_under_the_fleet_home`.
- **A4 met** — `Apply::skills_dir` and `undo`'s new parameter target
  the canonical library; pre-images stay under the fleet home; the
  apply/retire/undo tests exercise a scratch `canonical-skills` dir.
- **A5 met** — the retire branch is the no-op + one logged line naming
  KI-215; merge drafting unchanged; `retire_lesson` deleted; the row is
  in the ledger.
- **A6 met** — the gate, exit codes read directly, no new warnings.
- **A7 met** — against the real library: **441 of 444 `SKILL.md` files
  load and card**; 5 skips, all correct — `.git` and `tools` are
  non-skill directories, and `creative-ideation` (names itself
  `ideation`), `deli-autoresearch` (names itself `Deli_AutoResearch`),
  and `dev-decisions.bak-20260926-124047` (a backup directory) fail the
  one-skill-one-name rule. Every nested/list-frontmatter file loads.
- **A8 met** — `docs/features/skills.md` rewritten (layout, dialect,
  soft cap, curation), `docs/harness-orientation.md`'s seam row and
  learning paragraph updated, `CLAUDE.local.md`'s machine-path line
  updated, `src/install.rs`'s layout comment corrected, the corpus fact
  amended, and the symlink removed (verified: `~/.fleet/skills` empty).

One test-hygiene lesson landed with the work: a "bare" scratch FLEET_HOME
resolves to the machine's real library through the user home, so the
bare-case e2e redirects `FLEET_SKILLS_DIR` explicitly — a test never
reads the owner's library.
