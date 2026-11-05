---
status: draft
created: 2026-11-05
updated: 2026-11-05
slug: house-methodology
---

# Plan: the house methodology, traveling — the lessons as skills, the scar tissue as a seedable corpus

## Context

The question behind this plan: how much of how fleet has been developed
is embedded in the product, how much is skills, and how much is only
memories — because every project run through fleet should be developed
as thoroughly as fleet has been. The verified answer (2026-11-05):

- **Tier 1, product, automatic**: plans-as-contract (the `plan` tool;
  only an approved plan generates a graph — `src/tools.rs:96-98`), the
  graph's three real gates fed verdicts from outside
  (`src/graph.rs:776`), the evidence spine (the desk), the logged loop,
  approvals, compaction and context health, the judged router and the
  calibration beats, the memory substrate, shape-first skill discovery.
- **Tier 2, skills, transfers**: the methodology's how-to body in
  `~/.agents/skills` — fleet's canonical library by default
  (`src/skills.rs:6-7`).
- **Tier 3, does not transfer**: the judgment lessons —
  right-fixes-not-cheap-fixes, say-only-what-you-have-read, the
  two-gates lesson (KI-410), the load-sensitive test class,
  measure-AND-read, deferred-action-is-an-error, census discipline —
  live in fleet's own corpus and this repo's conventions. The corpus
  resolution is home-first, then the ancestor walk
  (`src/memory.rs:734-781`), so a jobhound run recalls jobhound's
  corpus; fleet's scar tissue stays home.

The owner's ruling from the desk brief already names the direction —
the methodology is integral to the harness, process travels with the
operator — and this plan's steer lands the skills in the canonical
folder: `~/.agents/skills`.

**The approach is both halves**: the skill carries the *rule*, the
seeded fact carries the *scar tissue* that makes the rule credible.

## Approach

1. **The skill family, authored in the canonical library**
   (`~/.agents/skills/`, the steer): a compact house set, each
   `SKILL.md` carrying `shapes:` from the one vocabulary
   (`src/memory.rs`'s structure and failure shapes plus the desk's
   kinds) so shape-first discovery routes it, and a body that states
   the rule **with the measured failure that earned it** — the
   numbers, the KI, the incident. The candidate set, each surviving a
   dedup against the library's existing coverage:
   - `house-right-fixes` — the cheap-fix catalog: coordinate-splicing
     a hunk, a validator silent about skipped keywords, a one-key
     defaults table (the 2026-11-02 stop), and the cure each right fix
     taught. shapes: change, build.
   - `house-evidence-discipline` — say only what you have read; a
     claim names its primary source; a correction means go read, not
     reword; inferences labeled. The sys1 mischaracterization as the
     scar. shapes: docs, analysis, document.
   - `house-declaration-is-not-execution` — KI-410's two passes: pin
     both the declaration gate and the execution gate, drive it live;
     a scoped tool can still be refused at the scope gate. shapes:
     change, build.
   - `house-load-sensitive-tests` — the class (KI-385/KI-405/KI-426):
     a fixture whose budget is counted in different units than the
     mechanism; the cure is the observable naming itself. shapes:
     change, build.
   - `house-mockups-first` — draw, measure **and read as pixels**;
     the read catches what the measurement calls green and the
     measurement catches what the eye cannot (the 2026-11-04 lesson);
     a surface ships with its harness pins. shapes: rendering,
     presentation, document.
   - `house-immediacy` — a control that applies only later is an
     error in the owner's eyes; say its scope where the control
     stands. shapes: change, rendering.
   `house-` prefix, so the family lists together and never collides
   with a harness's own skills.
2. **The dedup pass**: each candidate checked against the 459-skill
   library before authoring — `verify-fix-actually-ran`,
   `regression-claim-verification`, `root-cause-first-debugging`,
   `verify-issues-before-planning` already carry adjacent ground; the
   house skills **reference** them where they overlap and carry only
   the house-specific judgment. The dedup table lands in the plan's
   own recap.
3. **The fact pack** at `~/.agents/house/facts/` — beside the
   canonical library, one markdown file per house fact in the
   writer's frontmatter shape (name, description, type, basis,
   shape), the scar tissue as the body. The pack is the seedable
   source; it is **never** `~/.fleet/memories` — see the hazard.
4. **The seed procedure** (`scripts/house-seed.py`, the transplant
   scripts' generalization): `--project <dir>` lands the pack into
   `<project>/memories/` through `fleet remember` — the writer is the
   only frontmatter authority, the collision refusal makes it
   idempotent, the reconciliation prints per fact. No import mechanism
   ships in the binary; the loop is a procedure, the transplant's own
   ruling.
5. **The shadow hazard, named where it bites**: the home corpus
   shadows every project corpus when it holds facts
   (`resolve_root_from`, KI-087's rule). The pack therefore never
   lands in `~/.fleet/memories`, and the script refuses that target
   by name.
6. **Prove on a real scratch project**: a throwaway directory, the
   seed run twice (the second lands nothing — the collision refusal),
   then one recall per family member — the lesson ranks, its scar
   tissue in the body, quoted in the recap.
7. **The pins**: the C3 Rust test in `src/skills.rs`'s tests module
   names each house skill and asserts `select_for_query` routes it —
   the only `src/` touch, which is what keeps C7's promise honest.
8. **Docs**: `docs/features/skills.md` names the house family and the
   shapes; `docs/features/memory.md` names the pack and the seed
   path; the transplant's doc points at the generalization.
9. **The gate runs whole before any push that carries code** (C9):
   `cargo fmt --all --check`, `cargo clippy --all-targets -- -D
   warnings`, `cargo build --release`, `cargo test` — the four
   commands, exit codes read directly, the C3 pins riding the suite;
   the doc-only majority of the diff skips the gate only where hard
   rule 2 says it may.

## Acceptance criteria

- [ ] **C1 — the family.** Six `house-*` skills exist in
      `~/.agents/skills/`, each with `shapes:` from the one
      vocabulary and a body carrying its rule with the measured
      failure that earned it — numbers and KI names included.
- [ ] **C2 — the dedup.** A per-candidate overlap check against the
      existing library, recorded in the recap; overlaps reference the
      existing skill instead of restating it.
- [ ] **C3 — the shapes route.** With the canonical library loaded,
      `select_for_query` on a family-matching instruction returns the
      house skill — pinned by a Rust test naming each skill.
- [ ] **C4 — the pack.** `~/.agents/house/facts/` holds one markdown
      fact per scar-tissue lesson in the writer's frontmatter shape;
      the count and the listing recorded.
- [ ] **C5 — the seed script.** `scripts/house-seed.py --project
      <dir>` lands the pack through `fleet remember`, idempotent by
      the collision refusal, reconciliation printed; it refuses
      `~/.fleet/memories` as a target by name, citing the shadow
      rule.
- [ ] **C6 — the scratch proof.** A throwaway project seeded, then
      one recall per family member: the lesson ranked with its scar
      tissue in the body, the ranked lines quoted.
- [ ] **C7 — no product changes.** The skills load through the
      canonical-library default; the seed is a script; the diff
      touches no `src/` file except tests that pin C3.
- [ ] **C8 — the docs.** The three doc notes landed; the recap
      carries the dedup table, the pack listing, and the scratch
      proof's quotes.
- [ ] **C9 — the gate runs whole before any push carrying code:**
      the Approach's step 9 executes `cargo fmt --all --check`,
      `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, and `cargo test`, each exit code read
      directly, and the run's record quotes them.

## Files to be touched

- `~/.agents/skills/house-*/SKILL.md` — six new skills.
- `~/.agents/house/facts/*.md` — the fact pack.
- `scripts/house-seed.py` — the seed procedure.
- `src/skills.rs` or `src/memory.rs` tests — the C3 pins only.
- `docs/features/skills.md`, `docs/features/memory.md` — the notes.

## Out of scope

- Any binary import mechanism — the seed stays a script (the
  transplant's ruling).
- Landing the pack into fleet's own repo corpus — fleet already
  carries these facts; the pack is for projects that don't.
- A home-corpus redesign (the shadow rule stays as it is; the plan
  works around it and names it).
- Reviewing or re-curating the existing 459-skill library.

## Verification

```sh
# the family loads and routes
cargo test --lib skills::

# the pack seeds a scratch project, idempotently
python3 -I scripts/house-seed.py --project /tmp/house-proof
python3 -I scripts/house-seed.py --project /tmp/house-proof   # second run: already in the store

# the scar tissue recalls
cd /tmp/house-proof && fleet memories --recall "a test that only passes alone"

# the gate (exit codes read directly)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

## Linked artifacts

- `docs/features/skills.md` — the house family and its shapes.
- `docs/features/memory.md` — the pack, the seed path, the shadow
  hazard.
- `docs/recaps/` — the dedup table, the pack listing, the proof.

## Risks

- **The shadow rule cuts both ways.** The home corpus wins when it
  holds facts — so much as one fact accidentally landed in
  `~/.fleet/memories` would shadow every project corpus on the
  machine. The script refuses that target by name (C5); the pack
  lives under `~/.agents/house/`, not the fleet home.
- **Skill sprawl.** Six new skills join a 459-strong library; the
  60-skill declaration cap means discovery, not volume, is what
  matters — the shapes are what put a house skill in a run's
  declaration.
- **The dedup judgment is advisory.** Overlap with existing skills is
  a judgment call; the recap records the calls so the owner can
  overrule.
