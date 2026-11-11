---
status: completed
created: 2026-11-11
updated: 2026-11-11
slug: cross-harness-memory-pull
---

# Plan: the cross-harness memory pull — armada's feeder, local first

## Context

A few days ago the owner transplanted memories zcode → fleet as a one-time
*push*. This is the inverse, made ongoing: fleet *pulls* a project's facts
from other installed harnesses into its own substrate. The owner's
portability ruling settled the shape — **import into the substrate, not
federated recall**: a fresh armada instance working an existing project has
no local foreign stores, so the memories must live in the git-tracked corpus
armada syncs, not behind a live read. The trigger is a **one-time bootstrap
plus a commit-cursor delta**: pull all of a project's facts once, then only
when a new commit lands on that project in a foreign harness *or* a foreign
instance. The commit is the cursor — git-sync semantics at project scope,
which is exactly what armada's corpus speaks.

The owner wants this built **now, before armada is scoped**: it tests the
sync from the foreign-harness side, and the source abstraction prepares the
foreign-instance (armada) feeder so it plugs in later without rework. One
corpus, two feeders — harnesses feed it locally, armada will distribute it
globally.

## Decision-model check

No new sys1 judgment shape fits. The pull is deterministic I/O and arithmetic:
discover foreign stores (read-only), map their facts to fleet's `Fact`,
dedup by name/content, advance a revision cursor. Dedup/conflict is the
substrate's existing supersede semantics, not a head. The value lane is the
substrate itself; no model call is spent. (Mapping a foreign fact's *type* to
fleet's vocabulary is a lookup, not a classification.)

## Approach

One path: a new feeder module over the existing fact writer, with a source
abstraction that is local-harness now and armada-instance later.

1. **Source abstraction.** A `Source` trait (or enum) yielding a project's
   facts plus a *revision cursor*. `LocalHarness` (zcode first, hermes next)
   is implemented now; `RemoteInstance` (armada) is a named stub with the
   same interface, so the pull is source-agnostic. The cursor is a generic
   revision: the store's git HEAD where it is git-tracked, else a content
   hash — so a non-git local store still syncs, and armada's git commits
   plug in unchanged.
2. **Discovery, read-only.** Scan the known foreign memory paths under the
   user's home (the `mcp_registry::scan` precedent: "reads them and never
   writes them"), project-scoped — zcode's
   `~/.zcode/cli/memories/projects/<slug>/memory/`. An absent store is a
   named absence, never a silent skip. The own-home ruling holds: knowledge
   only, never a foreign harness's credentials or config.
3. **Bootstrap import.** Pull all of a project's foreign facts into the
   substrate through the existing `Remember` writer: provenance-tagged
   (`source: zcode:<project>:<revision>`, `basis: Inferred` — evidence
   another harness produced, half-strength like a session fact), the foreign
   `metadata.type` mapped to fleet's fact vocabulary, deduped against what
   fleet already holds.
4. **Commit-cursor delta.** A per-project, per-source last-pulled revision
   (a sidecar). On a new revision, import only the changed facts since the
   cursor and advance it. Detection is check-on-touch (a lazy revision read
   when fleet works the project) — bounded, no daemon; the armada fetch is
   the remote trigger later.
5. **Import semantics.** Add / update / supersede / archive. A fact modified
   at the source supersedes fleet's imported copy; a fact *deleted* at the
   source archives fleet's copy (preserve provenance), never hard-deletes.
6. **The verb.** `fleet memories --pull [--project <name>] [--source <harness>]`
   over the same writer; the bare bootstrap and the delta share one path.

## Acceptance criteria

- [x] **C0 — read-only discovery.** The feeder names the installed foreign
      harness stores and the project's facts within them; an absent store is
      a named absence; nothing foreign is written. Pinned by a test over a
      scripted foreign store and an absent one.
- [x] **C1 — bootstrap import.** A project's foreign facts import into the
      substrate provenance-tagged (`source`, `basis: Inferred`), type-mapped,
      and deduped against existing facts. Pinned by a test importing a
      scripted zcode-shaped store and asserting the landed facts.
- [x] **C2 — the cursor delta.** After the bootstrap, a new source revision
      imports only the changed facts and advances the cursor; an unchanged
      revision imports nothing. Pinned by a two-revision test.
- [x] **C3 — import semantics.** An updated source fact supersedes the
      imported copy; a deleted source fact archives it (no hard delete).
      Pinned by an update/delete test.
- [x] **C4 — the source abstraction.** The pull is source-agnostic:
      `LocalHarness` works now and `RemoteInstance` is a named stub with the
      same interface; the cursor is a generic revision (git HEAD or content
      hash). Pinned by a test driving the pull through the trait with a
      scripted source.
- [x] **C5 — the verb + the gate.** `fleet memories --pull` runs the
      bootstrap and the delta; `cargo fmt --all --check`, `cargo clippy
      --all-targets -- -D warnings`, `cargo build --release`, `cargo test`
      all exit 0.

## Verification evidence (2026-11-11)

Implemented as `src/memory_pull.rs` (the `Source` trait, `LocalHarness` zcode
reader, `RemoteInstance` named stub, the generic revision cursor, the pull
through the validated `Remember` writer) with two minimal `pub(crate)` seams
in `src/memory.rs` (`sanitize_description`, `archive_fact`) and the
`fleet memories --pull [--project] [--source]` verb. Verified by the session
itself:

- **Four-leg gate, exit codes read directly:** fmt = 0, clippy = 0 (0
  warnings), build --release = 0, test = 0 (2,190 lib passed, 0 failed).
- **C0** — `discovery_names_the_store_and_the_absence`,
  `discovery_names_the_files_it_cannot_read` (foreign bytes unchanged).
- **C1** — `bootstrap_imports_provenance_tagged_and_deduped` (source
  `zcode:fleet:<rev>`, basis inferred, type-mapped, native collision named
  and skipped, not clobbered).
- **C2** — `the_cursor_delta_imports_only_what_changed` (two revisions;
  unchanged imports nothing; cursor advances).
- **C3** — `an_update_supersedes_and_a_delete_archives` (no hard delete).
- **C4** — `the_pull_rides_the_source_abstraction`, `a_git_store_reads_its_head`,
  `the_content_revision_is_stable`.
- **C5** — `memories_pull_parses_its_store_flags_and_combines_with_nothing`
  + the gate + a live pull.
- **Live pull** (independent, into the repo corpus then reverted): the first
  pull imported the zcode fleet facts with named within-batch and native
  collisions; the second pull read `unchanged since the last pull`; an absent
  project and `--source armada` were named (the stub). The real zcode store
  is **not git-tracked**, so the cursor basis is the stable content hash
  (`content:<sha256>`); git-tracked stores read `git:<HEAD>` (test-pinned) —
  the basis armada's commits will ride.

Design note: change detection is a writer-normalized content digest against
the substrate (the revision is the trigger), uniform across git and non-git
stores. The substrate is the durable record of what was imported.

## Files to be touched

- Backend: `src/memory_pull.rs` (new — the feeder, the source abstraction,
  the cursor), `src/memory.rs` (reuse the `Remember` writer / `Fact`),
  `src/cli.rs` + `src/main.rs` (the `--pull` verb), `src/mcp_registry.rs`
  (the read-only discovery precedent, reused or mirrored).
- Tests: the feeder unit tests over scripted foreign stores.
- Docs: the linked artifacts below.

## Out of scope

- **The armada remote-instance feeder** — the `RemoteInstance` source is a
  named stub; armada's sync is its own program (the direction note).
- **Continuous polling / a background watcher** — detection is
  check-on-touch; a watch is a later refinement.
- **Credentials or config import** — knowledge only; the own-home ruling.
- **A new embedding or recall channel** — imported facts ride the existing
  six-channel recall as ordinary facts.

## Verification

- C0–C4: scoped feeder tests (`cargo test -p fleet --lib memory_pull::`),
  then the whole gate.
- C5: the four-leg gate whole, exit codes read directly (KI-014); a live
  `fleet memories --pull --project fleet --source zcode` against the real
  zcode store, reading the landed facts back.

## Linked artifacts

- `docs/features/memory.md` (or the substrate's feature doc) — the pull
  feeder, the cursor, the import semantics, the source abstraction.
- `docs/architecture/2026-11-11-armada-direction.md` — add the pull as the
  corpus's cross-harness feeder (the local half of the shared-corpus layer).
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — the feeder.
- `FUNCTIONAL-SPECIFICATIONS.md` — the `fleet memories --pull` verb.

## Risks

- **Foreign formats drift.** zcode's frontmatter is the shape today; a
  harness that changes its layout must degrade by name, never import garbage.
  The mapper validates against the `Remember` writer's own grammar.
- **The cursor's basis.** A non-git store has no commits; the content-hash
  fallback must be stable (sorted, normalized) or every read looks like a
  change. The cursor is per-project per-source, never global.
- **Dedup collisions.** A foreign fact and a native fact may share a name;
  the import must not clobber an owner-stated native fact — provenance and
  the supersede rule decide, and a collision is named.

## Dependencies

- The substrate's `Remember` writer and supersede semantics (`src/memory.rs`).
- `docs/architecture/2026-11-11-armada-direction.md` — the shared-corpus
  layer this feeder plugs into (the remote half arrives with armada).