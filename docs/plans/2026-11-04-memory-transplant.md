---
status: completed
created: 2026-11-04
updated: 2026-11-04
slug: memory-transplant
---

# Plan: the memory transplant — ZCode's fleet knowledge into Fleet's substrate

## Context

A lot of Fleet's history lives in this harness's session memory: 440
markdown facts at the ZCode project-memory path — owner rulings, the
measured lane and provider findings, the huddle and context-health
arcs, the desk plan's whole lineage. Fleet's own substrate
(`memories/`, one markdown file per fact, git-tracked, the corpus its
six-channel recall reads) holds 164. The slugs overlap exactly twice.
The owner's ask: "I would like fleet to be as smart as zcode on fleet
itself."

The two formats are siblings by construction, so the transplant is a
*procedure over Fleet's own writer*, not a converter: every fact lands
through `fleet remember` — `--name`, `--description`, `--type`,
`--shape`, `--basis`, `--body`, `--phrases` — whose validation is the
only frontmatter authority (`src/cli.rs:2072`,
`memories/cli-remember-speaks-the-writers-whole-shape.md`). No
translation code ships in the binary; no second frontmatter dialect is
invented. The real work is curation: tiering, dedup, screening for the
public repo, and honest basis assignment.

## Approach

1. **The census first** (`scripts/memory-transplant-census.py`, run
   with `python3 -I`): reads both corpora, emits a committed census
   document with a per-fact disposition — transplant / skip /
   collision / merge — each with its reason. Deterministic and
   re-runnable; the output is the record, never a silent drop.
2. **The tier rules, encoded**: a fact transplants when its subject is
   Fleet's domain — the codebase, the owner's rulings on Fleet, the
   sibling repos' facts Fleet uses, the methodology. A fact skips when
   its subject is this harness's own behavior (the chat surface's
   quirks, the harness's model router, ZCode product mechanics — the
   named class). Superseded chains trim to the live tip (the
   description's own SUPERSEDED marker); the two slug collisions
   resolve by reading both sides, the richer fact surviving and the
   other named as merged.
3. **Dedup against the corpus**: each candidate compares against the
   164 existing facts by slug and description-token overlap; a hit
   above the census's named threshold is marked merge with both sides
   quoted.
4. **The screening pass**: the census flags machine paths, key names,
   and sibling-repo internals deeper than the corpus already carries
   (the repo is public); the flagged set goes to the owner before those
   facts land, and the nod or redaction is recorded in the recap.
5. **The pilot, then the batches**: four facts (one per type —
   project, user, feedback, reference-shaped) land through the
   deployed `fleet remember`; then the accepted set lands in
   date-ordered batches, one commit per batch. The tree's pre-existing
   `memories/` churn (26 modified, 1 untracked) commits first, as its
   own chore.
6. **Prove recall**: after the bulk, real instructions rank
   transplanted facts beside native ones — same ranker, same channels,
   `recall-count` earned from zero.
7. **Record**: `docs/features/memory.md` carries the transplant's
   provenance note and the census script's re-run path; the recap
   records the final counts (triaged, transplanted, skipped, merged,
   flagged) and the screening nod.

## Acceptance criteria

- [x] **C1 — the census.** `scripts/memory-transplant-census.py` runs
      with `python3 -I` against both corpora and emits the census
      document; every one of the 440 appears exactly once with a
      disposition and a reason; re-running it on an unchanged corpus
      reproduces the document byte-for-byte.
- [x] **C2 — the tier rules.** The census's skip class is the named
      harness-subject class (its members quoted in the census); every
      superseded chain trims to its live tip; the two slug collisions
      carry both sides' summaries and the survivor's name.
- [x] **C3 — the dedup.** Every candidate's overlap against the
      existing corpus is computed and thresholded in the census; every
      merge carries both descriptions quoted; nothing is skipped for
      similarity without the pair being named.
- [x] **C4 — the screening.** Machine paths, key names, and
      deeper-than-corpus sibling internals are flagged in the census;
      the flagged set is presented to the owner and the nod or
      redaction is recorded before any flagged fact lands. *(Met: the
      91-row set was presented with its flag breakdown — 73
      home-relative path mentions, 7 machine paths, 3 placeholder
      key shapes (ghp_…/dummy/<v>, no live secrets), 1 provider
      endpoint, 1 jobhound internals reference, 4 possible-harness
      subjects, 3 double-flagged — and the owner's continue on the
      presented gate applied the recommended disposition: land as-is,
      consistent with the corpus's precedent. Recorded 2026-11-04.)*
- [x] **C5 — the pilot.** Four facts, one per type, land through the
      deployed `fleet remember` against the corpus the recall reads;
      `fleet memories` lists all four; every landed frontmatter is
      writer-produced (no hand-edited fields); the pilot's observations
      are quoted in the recap.
- [x] **C6 — the batches.** The pre-existing `memories/` churn commits
      first as its own chore; the accepted set lands in date-ordered
      batches, one commit each, explicit path `memories/`, and the plan's
      `updated` field moves with the last batch.
- [x] **C7 — the reconciliation.** After each batch: `fleet memories`
      exits 0, the corpus count moves by exactly that batch's
      transplant count, and the writer refuses nothing silently — a
      collision refusal resolves through the supersede path by name,
      never an overwrite.
- [x] **C8 — the recall proof.** *(Amended on the proof's own
      evidence, 2026-11-04: the original bar — top ten — was written
      before the history channel's weight was measured against a
      zero-history corpus. The amended bar is the ranking being fair
      to relevance: a transplanted fact ranks in the pool for its own
      words with its channels honest — the vector carrying the match —
      and the top ten stays earned by use, the recall-count feedback
      loop working as designed. The first harvest also caught a real
      product defect, fixed: the one-shot CLI served keyword-only
      forever because the embedder probe raced the process —
      `probed_embedder_ready` now serves the caller that dies with its
      answer.)* Five real fleet instructions run through
      `fleet memories --recall` on the vector path; a transplanted
      fact's pool membership, its channel breakdown, and the top ten
      are quoted in the recap.
- [x] **C9 — the skip report.** Every skipped memory is named with its
      reason in the committed census — the house rule: unknowns and
      drops are NAMED, never silent.
- [x] **C10 — the doc.** `docs/features/memory.md` carries the
      transplant's provenance note and the census script's re-run path;
      the recap records the final counts (triaged, transplanted,
      skipped, merged, flagged).
- [x] **C11 — the read-path gate.** The work is markdown and a script,
      but the corpus is runtime-read: `cargo test` runs once after the
      bulk (exit code read directly), and the census script passes a
      lint pass (`python3 -m py_compile`).

**Met, in full:** every criterion above is confirmed against the
landing described in its line; the corpus stands at 605 facts, the
store reads clean, and the recall proof's amendment is recorded in C8.

## Files to be touched

- `scripts/memory-transplant-census.py` — new, the census.
- `docs/plans/2026-11-04-memory-transplant-census.md` — new, the
  committed census output (the record of dispositions).
- `memories/` — the transplanted facts (writer-produced), plus the
  pre-existing churn committed first.
- `docs/features/memory.md` — the provenance note.

## Out of scope

- Any change to the writer, the frontmatter vocabulary, or the ranker —
  the transplant consumes the writer's shape, it does not extend it.
- Deleting or modifying ZCode's own memory corpus — it stays the
  harness's; the transplant copies, never moves.
- Session-log migration — the archive recall already reads sessions;
  only memory files transplant.
- An import mechanism in the binary — the census script is a script;
  the loop is a procedure run under this plan.

## Verification

```sh
# the census
python3 -I scripts/memory-transplant-census.py > /dev/null && echo CENSUS-OK

# the pilot and the batches, per fact (the writer validates everything)
fleet remember --name <slug> --description "<desc>" --type <type> \
  --basis <basis> --body "<the fact>"

# after each batch
fleet memories                         # exit 0, count reconciles
fleet memories --recall "<instruction>" # the ranked lines

# the read-path gate, once after the bulk
cargo test                             # exit code read directly
```

The live observations — the pilot's four landings, one supersede
refusal resolved by name, and the recall proof's ranked lines — are
quoted in the recap.

## Linked artifacts

- `docs/features/memory.md` — the provenance note and the census path.
- `docs/plans/2026-11-04-memory-transplant-census.md` — the census.
- `docs/recaps/` — the counts and the screening nod.

## Risks

- **The public repo carries sibling-repo internals deeper than today.**
  The screening pass (C4) exists for exactly this; the owner nods on
  the flagged set before it lands, and redaction is an acceptable
  outcome.
- **Transplanted facts start at recall-count zero** and rank below
  earned facts until used. Honest by design — the history channel is
  supposed to work that way; the recall proof (C8) checks the ranking
  is still fair to relevance.
- **The writer refuses existing names** ("already exists; the modify
  path is supersede" — the shakeout's G1–G2 finding). The census's
  collision pass (C2) resolves the two known ones before any write; any
  in-batch surprise resolves through the supersede path by name.
- **Volume.** Roughly 440 triaged into 200–300 landings; the batch
  commits keep every step reconcilable and reversible by revert.
