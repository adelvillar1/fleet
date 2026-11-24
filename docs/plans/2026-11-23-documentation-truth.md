---
status: completed
created: 2026-11-23
plan: documentation-truth
---

# Documentation truth — correct the record, then make the counts unable to lie

The owner's ruling (2026-11-23): *"I feel that the documentation discipline has
been a bit lacking lately… correct and close the gap, we shouldn't have these
gaps in projects."*

A full audit of the documentation surface (92 docs, 71 local links, four
front-door documents, the module reference, 46 feature docs, the design pair,
and both Archify diagrams) measured against the code as it stands today. The
records were wrong in four distinct ways, and — the point of this plan — only
one of those ways is a mechanism Fleet already owns.

## The measured state (the yardstick this plan corrects against)

Read on 2026-11-23 at `5b239078`, version `0.9.4`:

| Fact | Value |
|---|---|
| `.rs` files under `src/` | 154 |
| lines under `src/` | 288,334 |
| top-level modules | 103 |
| test attributes | 2,455 |
| built-in tools (`ALL_TOOLS`) | 11 |
| all declarable tool names | 37 (11 + 26 specials) |
| lanes in `lanes.rs` | 6 (`sys1`, `sdm1`, `sem1`, `gen1`, `browser`, `blend1`) |
| lane dirs under `services/` | 7 (the six + `documents`) |
| rail rows rendered | 15 |
| addresses in `ROUTES` | 21 |
| eval scenarios | 22 |
| commits total / before `v0.4.0` | 1,931 / 1,040 |

## What the audit found

### A. The diagrams are 345 commits stale

Both `docs/architecture/fleet-architecture.html` and
`docs/architecture/fleet-turn.html` are pinned to `meta.repository.revision`
`d4ad7790` (2026-11-16), 345 commits behind HEAD. Every one of their 36
`file:line` `SRC` badges still resolves in-bounds, so nothing fails — but none
of them lands on the code it claims. Verified by reading the same line at the
pinned revision: `src/agent.rs:3062` was `pub async fn run(&mut self, task:
&str)` when drawn, and is a desk-context call today. The diagram's own
strongest sentence is that its badges point at the line each claim was read
from, which is exactly the sentence that has stopped being true.

The *shape* still holds: the 16 components, two boundaries and edge labels
match today's system (read from `candidate.json` and the PNG).

### B. The module reference covers half the crate

`TECHNICAL-DOCUMENTATION.md` calls itself "the system of record" and carries
**54 module rows against 103 top-level modules** — 54 modules have no row at
all, including real subsystems: `tool_router.rs`, `mcp_registry.rs`,
`fileops.rs`, `search.rs`, `web.rs`, `security.rs`, `identity.rs`,
`update.rs`, `recall.rs`, `verify.rs`.

### C. Statement drift across the front door and the durable docs

| Where | Says | Actually |
|---|---|---|
| `README.md:106` | nine lenses (threads, archive, board, desk, intelligence, learning, reviews, inbox, health) | 15 rail rows; `threads`/`archive` are not rows |
| `README.md:219` | beta `(0.8.x)` | `0.9.4` |
| `FUNCTIONAL-SPECIFICATIONS.md:28` | "Fourteen destinations in four groups" | its own body lists 15; the rail renders 15 |
| `docs/architecture/surfaces.md:4-8` | narrates eleven → … → fifteen; groups work/intelligence/knowledge/system | 18 sections; the groups in `RAIL_MAP` are software/design/knowledge/system |
| `docs/architecture/lanes.md:3,21` | five specialized services | 6 — `browser` is absent |
| `services/README.md` lane table | 5 rows | 6 lanes vendored; `browser` absent |
| `CLAUDE.md:90,130,138` | "eight built-ins", "five vendored lanes", "eleven destinations" | 11, 6, 15 |
| `docs/features/tools.md:126` | "the eight built-ins" | 11 — while line 20 of the same file correctly says eleven |
| `docs/features/tools.md:78` | "36 tools and 10,000 definition tokens" | `MAX_DEFINITION_TOKENS = 10_800` |
| `docs/harness-orientation.md:15`, `docs/architecture/overview.md:4` | "45 modules … ~70k lines" | 103 / 288,334 |
| `CHANGELOG.md:6` | "everything before it is `git log` (492 commits)" | `v0.4.0` has 1,040 ancestors |

### D. What was already right (kept, not reworked)

Cited so the fix does not overshoot: all 71 local links across 92 docs resolve
with zero broken; all nine README image references resolve; all 22 commands in
CLAUDE.md's command block exist in `cli.rs`; the eleven-tool table in
`docs/features/tools.md` matches `ALL_TOOLS` tool for tool;
`structure-guards.md`'s 10,800 and 36 match the constants; `CHANGELOG.md` is
current through v0.9.4 and agrees with the tag; CLAUDE.md is 245 lines, inside
its own ≤300 rule.

## The gap — why the mechanism did not catch this

`src/drift.rs` already exists for exactly this failure, and its docstring *is*
the 10,050-versus-10,800 story. It registers 7 metrics and 3 budgets with
file-path sites (never line numbers). It did not catch any of A–C, for four
structural reasons:

1. **Counted facts have no producer.** A drift row needs a `live` closure that
   calls the real producer. `ALL_TOOLS.len()`, `LANES.len()`, the rail row
   count and the module count have nothing to call, so no row can exist. These
   are precisely the numbers that rotted.
2. **Pinned-revision artifacts are outside the registry's model.** The
   diagrams' staleness is a function of commits-since-pin, which no metric
   expresses.
3. **An unregistered site is invisible.** `tools.md`'s `10,000` sits in a file
   no row lists, so even the existing 10,800 row cannot see it.
4. **A self-defeating doc shape.** CLAUDE.md's "Read the numbers, do not trust
   them here" warns the reader and then re-publishes `45 modules / ~70k /
   2,357 / 0.6.26` as its own baseline; `overview.md`'s stale-snapshot banner
   says "now 45 modules". A doc that both disclaims and asserts its numbers
   teaches the reader to skip both.

The owner's framing — *"we shouldn't have these gaps in projects"* — makes this
a mechanism row, not a copy-editing row: correcting the prose without closing
the gap means the same rot returns within a release or two.

## Acceptance criteria

**Correction (A–C).**

- [x] C1. Every row of the C table is corrected at its named site, and each
      corrected claim is re-read from the producer that owns it (the constant,
      the `RAIL_MAP` declaration, `git rev-list`, the `services/` listing) —
      never from another doc.
- [x] C2. `TECHNICAL-DOCUMENTATION.md`'s module reference covers all 103
      top-level modules, aggregated from each module's own doc comment.
- [x] C3. `docs/architecture/lanes.md` and `services/README.md` both carry the
      sixth lane (`browser`) and state the `documents` service's status
      truthfully (a local extractor, not an MCP lane).
- [x] C4. `CLAUDE.md` and `docs/architecture/overview.md` stop publishing a
      stale module/line/test count as their own baseline; each points at the
      command that measures it instead.
- [x] C5. Both Archify diagrams are regenerated against HEAD at a named
      revision, with every `SRC` badge re-verified against that revision, and
      the PNGs re-shot. A `fleet drift` row names the pin and reports
      commits-since-pin, so the next staleness is a finding.

**The gap (D).**

- [x] C6. `src/drift.rs` gains counted-fact producers that read the real
      collection (`ALL_TOOLS.len()`, `LANES.len()`, the rail's rows and
      `ROUTES`) and registers every doc site that states them.
- [x] C7. A crate test asserts the registered doc sites still state the
      registered values — so a doc number that moves away from its producer
      fails the suite, not a later audit.
- [x] C8. `docs/features/tools.md`'s `10,000` is either corrected to the
      constant or added to `max_definition_tokens`'s `sites`; the registry can
      see the file either way.
- [x] C9. A repeat of this audit is a command: one checker that reports every
      statement-drift row above as pass/fail, so C1–C3 cannot silently return.

**The gate.**

- [x] C10. `cargo fmt --all --check`, `cargo clippy --all-targets -- -D
      warnings`, `cargo build --release`, `cargo test` all exit 0 (exit codes
      read directly, never through a pipe); `python3 scripts/ki_ledger.py`
      reports zero malformed rows; `python3 scripts/plan_check.py` clean.

## Rows

Each row names the criterion it serves. No deferrals.

- **R1 — the dimensionless names.** Extend `src/drift.rs` with producers over
  the real collections and a `counted` row type whose `live` calls them.
  Serve C6.
- **R2 — the doc assertions.** A test that parses each registered doc site for
  its stated number and compares to the producer; plus `tools.md`'s `10,000`
  resolved one of the two honest ways. Serve C7, C8.
- **R3 — the audit as a command.** `scripts/doc_truth.py`: the C table's rows
  as checkable assertions, exit 0/1 with the first failing claim's own words.
  Serve C9.
- **R4 — the front door.** README, FUNCTIONAL-SPECIFICATIONS, CLAUDE.md,
  harness-orientation, overview. Serve C1, C4.
- **R5 — lanes and services.** `lanes.md`, `services/README.md`, and the
  `browser` lane's absence from both. Serve C1, C3.
- **R6 — the module reference.** All 103 modules aggregated from their own doc
  comments. Serve C2.
- **R7 — the diagrams.** Regenerate both at HEAD, re-verify every badge, re-shoot
  the PNGs, add the pin row. Serve C5.
- **R8 — the gate and the ledger.** The full four-leg gate, the ledger check, and
  the CHANGELOG's commit-count correction. Serve C10, C1.

## Ordering

R1 and R2 before R4–R6 (the mechanism must exist before the prose is corrected
against it, or the correction is the next thing to rot). R3 after R4–R6 (it
asserts the corrected text). R7 is independent of the rest and can run any time.
R8 closes.

## Supersession

No draft is superseded by this plan. `docs/plans/2026-11-20-dependency-dedup.md`
remains a separate open draft; this plan does not cover it.
