---
status: completed
created: 2026-11-11
updated: 2026-11-11
slug: armada-l4-corpus
---

# Plan: armada L4 — the shared corpus (git sync + the merge rules)

## Context

Armada is fitted into fleet bottom-up as a stack of responsible, measured
contracts (`../architecture/2026-11-11-armada-atomic-decomposition.md`).
L4 is the shared mind: the fact substrate synced across installs so a fact
recalled on machine A bumps its rank for machine B, a fresh install inherits
the corpus, and the control plane's `corpus-sync` verb has something to call.
This plan is atoms L4.1–L4.4 of the decomposition (§3.4).

The substrate is already exactly right and already git-tracked: one markdown
file per fact under `memories/`, fixed frontmatter, `Basis` provenance, and
supersede-with-chain-fold (`src/memory.rs:14-17,5165-5458`). **`memory.rs`
never invokes git** — a daily shell script commits the churn and nothing
pushes, and there is no merge code (`scripts/daily-maintenance.sh:86-98`).
The cross-harness pull is the one built feeder: the `Source` trait
(`src/memory_pull.rs:72-81`), the source-agnostic `pull()`
(`src/memory_pull.rs:731`), the generic revision cursor (`git:<HEAD>` or a
content hash, `src/memory_pull.rs:485-519`), and the `RemoteInstance` stub
that answers `NotYetBuilt` (`src/memory_pull.rs:449-476`).

What is missing is the remote half and, above all, **the merge rules**. Two
facts make naive pull/push wrong:

- The recall-count is a read-modify-write counter under a *local-only* flock
  (`bump_recall_count`, `src/memory.rs:3234-3244`; `rewrite_field` →
  `lock_fact`, `src/memory.rs:3367-3376`). Daily counter churn dominates the
  corpus commit, so two installs conflict on the same `recall-count:` line
  constantly — last-writer-wins would silently drop one install's recalls.
- KI-285's two-writer lost update goes distributed: `replacement_name`'s
  free-serial scan (`src/memory_pull.rs:707-722`) and `archive_target`'s
  `create_new` serials (`src/memory.rs:5986-6004`) are local-disk decisions
  two installs make identically → add/add and `-rN` collisions; supersede's
  atomicity assumes single-writer disk (`src/memory.rs:5165-5458`).

The recall index is a local cache, gitignored, and **never syncs**
(`.recall-index.json`, `.gitignore:8`; `INDEX_FILE`, `src/memory.rs:156-159`).

## Design choices inherited (the ledger)

Conforms to `ARMADA-DESIGN-DECISIONS.md`: **D0** (InstallId stable, minted at
install — the cursor/provenance key), **D1** (principal-agnostic; the sync
carries the principal), **D3** (responsible-citizen shape), **D4** (trust
earned by measurements — the merge is graded, the basis ladder is the winner
rule), **D5** (the stack — L4 composes L1 identity + L2 journal, re-implements
neither), **D6** (without breaking fleet — sync is opt-in, owner-holds-all),
**D7** (InstallId = 32 hex, the serial disambiguator and the event stamp),
**D8/D9** (the principal rides the run and the sync's journal events),
**D10/D13** (the journal is the durable home the sync settles through,
additively). The `corpus` guard dimension (**D14**) is where L8 will gate the
`corpus-sync` verb; L4 exposes the call and does not re-state the guard.

## Decision-model check

No sys1 head fits: the merge is deterministic arithmetic (sum/delta) and
deterministic tie-breaks (basis ladder → date → install-id), not a per-item
classify/gate/score/extract judgment. The basis ladder L4 *reads* is itself
earned by the dream pass's existing `validate` judgment — L4 consumes that
product, it does not add a head.

## Approach

One new module, `src/corpus.rs`, is the L4 contract: `corpus.sync(remote)` =
**fetch → `corpus.merge(rules)` → push**, with git as the transport and fleet
owning the semantic merge. The remote and the install-scoped cursor live at
`<home>/corpus-sync.json`; with no remote configured the sync refuses by name
and fleet stays exactly as it is today (the daily script keeps committing
locally). `corpus.merge` lets git do the mechanical three-way merge and takes
over only the conflicted fact files, reading their base/ours/theirs stages
(`git show :1/:2/:3:<path>`) and applying named per-field rules: the
recall-count line merges by **sum/delta** (never last-writer-wins), and a
distributed supersede collision resolves to **one live frontier** by the basis
ladder. The serial-collision half is prevented at the source: auto-generated
replacement and archive serials carry the install-id, so two installs never
mint the same name. Every sync settles a `corpus_sync` journal event through
L2 and is graded by read-back before it reports done — the substrate's own
KI-356 discipline lifted to the merge.

The `RemoteInstance` feeder then plugs into the *unchanged* import path: it
implements `Source` by reading the remote corpus at a git commit, so `pull()`
imports another install's facts provenance-tagged (`armada:<project>:<commit>`)
through the same `Remember` writer the local feeder uses. One corpus, two
feeders — harnesses feed it locally, armada distributes it globally.

High-level steps:

1. `src/corpus.rs`: the sync driver (`sync`), the merge (`merge`) and the pure
   field rules (`merge_recall_count`, `resolve_supersede`); the
   `<home>/corpus-sync.json` state (remote, corpus ref, last-synced revision,
   the last-ten settle history); the responsible-citizen shape — measure
   (HEADs, merge-base, dirty/conflict counts), admit/refuse, work, settle,
   grade, learn, curtail. Composes `memory::load_facts`, `memory::rewrite_field`,
   `identity::install` (D7), `journal::append` (D10).
2. Install-scope the serials (the distributed KI-285 fix): thread the
   install-id's first 8 hex into `replacement_name`
   (`src/memory_pull.rs:707`) and `archive_target` (`src/memory.rs:5986`), so
   a replacement reads `<base>-r<serial>-<install8>` and an archive copy
   `<name>.<install8>[.<serial>].md`. The local free-serial scan + flock still
   order one install's writes; the install suffix disambiguates across installs.
3. `corpus.merge` rules over the conflicted stages: recall-count sum/delta
   (D24); supersede/add-add resolution to one frontier with the loser folded
   and a named retirement note (D26); any other field conflict is a named
   refusal that curtails the sync (`git merge --abort`, corpus stays at local
   HEAD — never a half-merge pushed).
4. `RemoteInstance` implements `Source` (`src/memory_pull.rs:449-476`): read
   the remote fact tree at its HEAD commit, yield
   `Snapshot{revision:"git:<commit>", facts, skips}`; the `NotYetBuilt` stub
   retires. `pull()` is untouched.
5. The surface (hard rule 11): `fleet memories --sync [--remote <name>]` runs
   the driver and prints the report (revisions, merged/summed/resolved/refused
   counts, named degradations); `fleet memories --pull --source armada` now
   reaches the live remote feeder.

## Acceptance criteria

- [x] C0: `corpus.sync` with no configured remote refuses by name ("no corpus remote configured") and writes nothing; an unconfigured install's local behavior is byte-identical to today.
- [x] C1: `corpus.sync(remote)` over two clones of one corpus fetches, merges, and pushes; the install-scoped cursor at `<home>/corpus-sync.json` advances to the remote revision, and a second sync with no changes is a named no-op.
- [x] C2: a conflicted `recall-count` line merges by sum/delta — base 5, ours 8, theirs 7 → merged 10 — never last-writer-wins (a pure three-way rule test pins it).
- [x] C3: two installs superseding or archiving the same fact mint distinct install-scoped serials, so no add/add filename collision occurs (a two-install test pins it).
- [x] C4: two replacements claiming one predecessor resolve to a single live frontier — higher basis wins, tie → later date, tie → greater install-id — the loser folded into the winner with a named retirement note, read-back verified, nothing hard-deleted.
- [x] C5: `RemoteInstance` implements `Source`, reads the remote corpus at a git commit, and `pull()` imports through the unchanged writer with provenance `armada:<project>:<commit>`; the `NotYetBuilt` stub is gone.
- [x] C6: a sync moves only the tracked fact tree (`memories/*.md` + `.archive/`); the recall index, `*.lock`, `*.tmp`, and per-install state never enter the synced tree.
- [x] C7: every sync settles a `corpus_sync` journal event (install, principal, remote, before/after revision, merged/summed/resolved/refused counts) and the merged tree is graded by read-back (loads as facts, no live fact lost, chains intact) before the sync reports done.
- [x] C8: single-owner behavior is preserved — the four-leg gate passes and no existing run is refused (sync is additive and opt-in).

## Files to be touched

**Backend:**
- `src/corpus.rs` — new: the sync driver, the merge + field rules, the sync state, the report.
- `src/lib.rs` — `mod corpus`.
- `src/memory_pull.rs` — implement `RemoteInstance.read()` over a git commit; install-scope `replacement_name`.
- `src/memory.rs` — install-scope `archive_target`/`archive_fact` (thread the install-id).
- `src/home.rs` — the `corpus-sync.json` path helper.
- `src/cli.rs`, `src/main.rs` — the `fleet memories --sync` verb; `--source armada` reaches the live feeder.

**Tests:**
- `src/corpus.rs` (module tests) — the sum/delta rule, the supersede winner rule, the two-clone sync, the never-sync set, the settle + read-back.
- `src/memory_pull.rs`, `src/memory.rs` — the install-scoped serial pins (existing import/archive pins updated in the same commit).

**Documentation:** see Linked artifacts.

## Out of scope

- The `corpus-sync` control verb's guard/assertion (L3/L8) — L4 exposes the call.
- The authority dimension on supersede (can user A overwrite user B's fact) — lane 2; lane 1's winner rule is authority-free (basis ladder).
- Skills / `mcp.json` sync (the direction note's other sync-ready artifacts) — a later atom; the retire list stays per-install (KI-215).
- The relay transport (L6) — the corpus rides git, not the control tunnel; relay-down never blocks a git sync.
- The org's multi-tenant corpus visibility — lane 2 generalizes the same contract.

## Verification

**C0/C1:** `cargo test -p fleet --lib corpus` — the unconfigured refusal; a
scripted two-clone sync (init a bare remote, clone twice, write on each, sync,
assert both converge and the cursor advances; a second sync is a no-op).

**C2:** `cargo test -p fleet --lib corpus::merge_recall_count` — the pure
three-way rule: (base 5, ours 8, theirs 7) → 10; one-sided changes pass through.

**C3/C4:** `cargo test -p fleet --lib corpus` — two installs supersede/archive
one fact: distinct serials, one live frontier, the loser folded with a named
note, read-back verified; an unresolvable content conflict curtails by name.

**C5:** `cargo test -p fleet --lib memory_pull` — `RemoteInstance` reads a
scripted remote at a commit and `pull()` imports provenance-tagged; the stub
absence is gone.

**C6:** `cargo test -p fleet --lib corpus` — after a sync, the index/lock/tmp
and per-install state are absent from the pushed tree.

**C7:** `cargo test -p fleet --lib corpus` — the `corpus_sync` event lands in
the journal with the counts; the read-back grade gates the "done" report.

**C8:** the four-leg gate (`cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`).

## Linked artifacts

- `docs/features/corpus-sync.md` — new: the sync + merge contract (the rules, the never-sync set, the report).
- `TECHNICAL-DOCUMENTATION.md` — the corpus-sync contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D23–D29 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L4 atoms ticked when shipped.

## Risks

- **Serial-format fan-out.** Install-scoping changes the auto-generated replacement/archive names, so the existing import pins (`updated-r2`, `.archive/<name>.2.md`) change shape. Mitigation: the supersede/recall/protection *semantics* are unchanged (only the serial gains an install suffix); update every affected pin in the same commit; `scripts/edit-preflight.sh` names the sites.
- **`archive_target`/`replacement_name` signature change.** Threading the install-id touches their callers (the cycle, the import path). Mitigation: resolve `identity::install()` once at each caller; a named, bounded fan-out.
- **The merge is the hard part.** A wrong sum/delta or supersede rule loses recalls or strands a live fact. Mitigation: the rules are pure functions over (base, ours, theirs) tested without git; the read-back grade (C7) gates every sync; an unresolvable conflict curtails by name rather than guessing.
- **git topology.** The dev repo tracks `memories/` beside source; the corpus remote must be dedicated so source never rides it. Mitigation: sync a corpus-only ref; the unconfigured dev install keeps the daily local commit (C0). Topology for a dedicated install lands with L9.
- **L1/L2 not yet built.** `identity::install()` and `journal::append()` are drafts. Mitigation: L4 is Wave 3, after the bedrock; it composes those contracts and builds nothing in their place.

## Dependencies

- **L1 identity** (D7 install-id, D8/D9 principal) — the serial disambiguator and the event stamp.
- **L2 journal** (D10/D13) — the durable home the sync settles through.
- Inherits ledger D0–D13. The `RemoteInstance` feeder reuses the built pull (Wave 0).

## Design choices this layer settles (append to the ledger)

- **D23 — the corpus syncs over git via `corpus.sync(remote)` = fetch →
  `corpus.merge(rules)` → push; the remote + install-scoped cursor live at
  `<home>/corpus-sync.json`; git is transport only, the semantic merge is
  fleet's; with no remote configured the sync refuses by name and fleet stays
  local.** Rationale: the substrate is already git-tracked
  (`src/memory.rs:14-17`) and the daily script commits without push
  (`scripts/daily-maintenance.sh:86-98`) — the remote is the missing half;
  opt-in preserves single-owner behavior (D6). *Constrains:* L8 corpus-sync
  verb, L9 fresh-install inheritance, the shared mind.
- **D24 — concurrent recall-count bumps merge by sum/delta over the merge base
  (`merged = ours + theirs − base`), never last-writer-wins; the count stays in
  the fact frontmatter.** Rationale: recall-count is a grow-only rank signal
  (`src/memory.rs:304,3234-3244`), so the deltas add — a fact recalled on A
  bumps its rank for B (direction §3); keeping it in frontmatter avoids a
  substrate fan-out. *Constrains:* the merge driver, any future counter field.
- **D25 — auto-generated replacement serials (`replacement_name`) and archive
  serials (`archive_target`) carry the install-id's first 8 hex, so two installs
  never mint the same name (the distributed KI-285 fix).** Rationale: the
  free-serial scan (`src/memory_pull.rs:707-722`) and archive `create_new`
  (`src/memory.rs:5986-6004`) are local-disk decisions two installs make
  identically → add/add; the writer's identity (D7) disambiguates where a
  shared maximum cannot. *Constrains:* supersede/import naming, the merge's
  add/add rule, L9 inheritance.
- **D26 — when two installs supersede the same fact, the merge keeps one live
  frontier: the higher-basis replacement wins (confirmed-by > validated >
  inferred), tie → later date, tie → greater install-id; the loser folds into
  the winner with a named retirement note, read-back verified, never
  hard-deleted; an unresolvable content conflict is a named refusal that
  curtails the sync.** Rationale: supersede assumes single-writer disk
  (`src/memory.rs:5165-5458`); the basis ladder (D4) is the authority-free
  winner rule for lane 1, and the fold reuses the substrate's own chain
  semantics. *Constrains:* lane 2 adds the authority dimension (can A overwrite
  B); the merge driver.
- **D27 — `RemoteInstance` implements `Source` by reading the remote corpus at
  a git commit (`Snapshot.revision = git:<commit>`), feeding the unchanged
  `pull()` import path; the `NotYetBuilt` stub retires.** Rationale: `pull()` is
  already source-agnostic (`src/memory_pull.rs:731`, test C4) and the cursor is
  a generic revision (`src/memory_pull.rs:485-519`) — the remote feeder plugs in
  without rework, provenance `armada:<project>:<commit>`. *Constrains:* L8
  corpus-sync, L9 register_join, the shared mind's attribution.
- **D28 — every sync settles a `corpus_sync` journal event (install, principal,
  remote, before/after revision, merged/summed/resolved/refused counts) through
  L2, and the merged tree is graded by read-back (loads as facts, no live fact
  lost, chains intact) before the sync reports done; the sync estimate learns
  from the last ten settles.** Rationale: responsible citizen (D3) + trust
  earned (D4) — the merge is graded, not just recorded, lifting the substrate's
  read-back discipline (KI-356, `src/memory.rs:5353-5384`) to the sync; composes
  L1/L2, re-implements neither. *Constrains:* L8 tracing, the control plane's
  corpus-sync outcome.
- **D29 — the recall index (`.recall-index.json`), lock siblings (`*.lock`),
  temps (`*.tmp`), and per-install state (cursors, install-id, identity, sync
  estimates) never sync — only the tracked fact tree (`memories/*.md` +
  `.archive/`) moves; a fresh install inherits the corpus by cloning the remote
  with a fresh install-id (identity is not in the corpus).** Rationale: the
  index is a local cache (gitignored, `.gitignore:8`; `src/memory.rs:156-159`)
  and the count's flock is local-only (`src/filelock.rs`) — syncing them would
  corrupt or leak; the portability ruling (cross-harness pull). *Constrains:* L9
  provision→register→join, the org's multi-tenant corpus.

## Notes

The merge rules are the load-bearing half, not the git plumbing. Git already
moves bytes; what fleet adds is the *semantic* merge that keeps a distributed
counter honest (sum/delta) and a distributed supersede single-frontier (basis
ladder + fold), and the discipline that names every refusal and grades every
merge before it is trusted. The serial fix prevents the common KI-285 collision
at the source; the merge rule resolves the residual. Sync is opt-in, so nothing
the owner does today changes until a remote is configured — and then the corpus
becomes the shared mind armada stands on. Approve this and the ledger carries
D23–D29 into L5/L8/L9.