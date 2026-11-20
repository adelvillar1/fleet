# Deep dive: Fleet's measured state and its recurring defect classes — 2026-11-20

Read at `master` = `60ee125` (0.8.21, 2026-11-19), working tree dirty
with an in-flight session's changes (eleven modified sources —
`agent.rs`, `approvals.rs`, `armada/deploy.rs`, `delegate.rs`, `route.rs`,
`settings.rs`, `skills.rs`, `subagents.rs`, `tools.rs`, `tui/ui/mod.rs`,
and two docs — plus two new plans and
`deep-dive-2026-11-20-oh-my-hermes.md`). This dive is read-only
against that tree: every number below was measured this session, no
heavy `cargo` run was taken (the in-flight session held the build), and
nothing here replaces or contradicts `deep-dive-2026-11-20-oh-my-hermes.md`
— that one compares Fleet to a sibling tool; this one measures Fleet
against itself.

Method: `git log`/`status`, `wc`/`grep` over `src`, `cargo fmt --check`,
`cargo tree -d`, `scripts/ki_ledger.py`, `scripts/plan_check.py`, the
plan and recap trees, and the memory index.

## State at a glance (measured)

| Surface | Value |
|---|---|
| Crate | 0.8.21, `master` @ `60ee125`, 2026-11-19 |
| Activity | 939 commits since 2026-11-01; ~18 releases since 0.6.26 (2026-10-16) |
| Source | 95 `.rs` files, 99 module declarations, 267,068 lines under `src/` |
| Tests | 2,125 `#[test]` + 316 `#[tokio::test]` in `src/`, 46 in `tests/*.rs`; 15 `#[ignore]` |
| Test harness | `tests/integration.rs` 14,047 lines; `tests/face/harness.mjs` 16,637 lines (~1,303 checks, per KI-561) |
| Ledger | 575 rows, **5 open**, `ki_ledger.py` green |
| Plans | 167 completed, 4 cancelled, 1 active, 2 drafts; `plan_check.py` strips 2 stale 10-29 plans |
| Memory corpus | 965 fact files, 964 indexed, 1 unindexed, 0 fileless |
| Hooks | `core.hooksPath=.githooks` (plan_check pre-commit live) |
| CI | `workflow_dispatch:` only — honestly disarmed, not silently broken |
| Disk | 82 GiB free of 460 (81 % used); `target/` 4.1 GiB |

The ledger is the strongest thing measured here: 575 rows across six
weeks, five open, each open row carrying its next action, and the
checker green. The open five: KI-560 (a provider card's re-enable
deletes the model), KI-562 (the reload-all card repeats KI-561's
defects), KI-566 (the settings rail throws on a partial read), KI-571
(fmt red at HEAD), KI-575 (`run` never starts the checkpoint reclaim).

## Findings, ranked

### F1 — The gate's fmt leg is red at HEAD, and the debt is the newest module (confirmed, KI-571)

Measured live this session, twice with identical results:
`cargo fmt --all --check` exits 1 with **36 hunks across 9 files** —
`src/armada/join.rs` (14), `src/skills.rs` (5), `src/security.rs` (4),
`src/main.rs` (3), `src/corpus.rs` (3), `src/armada/mcp.rs` (3),
`src/armada/control.rs` (2), `src/identity.rs` (1),
`src/gui/bridge.rs` (1). Nineteen of the 36 hunks are in `src/armada/`.
One of the nine, `src/skills.rs`, is also modified by the in-flight
session, so the formatting commit takes the other eight now and
`skills.rs` once that work lands.

Why it matters beyond tidiness: hard rule 2 is four legs, and the fmt
leg now fails on files the session does not own, so no session can run
the gate whole without formatting another session's code (KI-571's own
next action — one formatting-only commit — is the correct fix and is
cheap).

### F2 — `run` grows its checkpoint store without bound (confirmed, KI-575)

`spawn_reclaim` is started by `chat_command` and the face, not by
`run_command`; `run` is the mode the harness itself drives. Between
KI-575 and F1, the two open rows found *today* are both "the newest
path is missing a piece its older siblings have" — which is F3's shape.

### F3 — Four recurring defect families have no general guard (confirmed pattern, proposal)

The ledger records 4 explicit `RECURRENCE of KI-NNN` rows and 24 rows
naming a family. The same four shapes keep returning:

1. **Fixed-width cards clipped mid-token** — KI-078 and its three
   recurrences (compaction card, cron fire card, judge card). A width
   guard exists (`CARD_WIDTH` in `src/calib.rs`), but the ledger's own
   post-mortem says it "never fired because its fixture carried a long
   provider with a four-character shape": the guard tests fixtures, not
   the card builders' shape.
2. **A corpus root resolved by two rules** — KI-074 then KI-087, whose
   fix note says the cron path "inherits the rule by construction (it
   is a deliberate mirror of `drive`)". Two mirrors of one rule.
3. **Controls that look live and do nothing** — KI-442/556/563 and the
   settings-card sweep (27 cards, 40 presses, five silent ones). Each
   fixed on its own surface.
4. **Suite flakes under parallel load** — 23 ledger rows name
   flake/race/loaded-suite; KI-574 (two armada tests racing on the
   process environment) closed today. The class is environment isolation,
   fixed per test.

The repo already answers classes structurally — `structure-guards.md`
inventories 156 `include_str!` guard sites. But those guards (a)
enumerate modules by hand (`src/judge/tests.rs:1557` is one entry in a
literal list), so a new module is scanned by default only if someone
remembers to add it, and (b) none of the four families above has a guard.

Proposal (needs the owner's ruling, not a plan yet):

- **One card builder for every fixed-width surface** that takes the
  pane width and wraps by display cells, plus a property-style test
  (every card × widths 20..120) instead of fixture widths.
- **One `corpus_root(home, cwd)` resolution** with both callers, and a
  text-scan guard that no other module names `memories/` directly.
- **One press-feedback primitive** (every control either acts, says why
  it cannot, or is disabled), swept across the 27 settings cards again
  as a test, not a one-off press audit.
- **Test-environment isolation as the default** (env writes behind one
  lock; ports from one allocator), so KI-574's fix is a primitive rather
  than a ninth per-test workaround.
- **Invert the guard lists**: derive each guard's scanned set from
  `lib.rs`'s module declarations, so coverage grows with the crate
  instead of with the guard's upkeep.

### F4 — Asset delivery has no compression, and the phone pays for it (enhancement)

Measured: `app.js` 31,986 lines / 1,392,784 bytes; `app.css` 11,892 /
367,400; the shell `index.html` 1,544 / 82,028 plus inline script
blocks. There is **no `Content-Encoding` anywhere in `src/`** (0 grep
hits) — every byte ships identity-encoded. The cache grammar itself is
correct: assets are hash-versioned (`?v={ASSET_HASH}`) and `served()`
answers `immutable` for the embedded path, so this is a first-load and
cache-miss cost, not a per-navigation one.

Where it costs: the relay's own doc comment says the browser-forced
fetches — "the shell page, its assets, the manifest, the worker" — cross
the relay as plaintext frames. A phone's first load, and every load
after a face restart rotates the hash, carries ~1.8 MB uncompressed
where gzip on this content class is typically 4–5×. A precompressed
variant served with `Content-Encoding: gzip` is a small, contained
change in `served()`/`versioned_asset()`.

### F5 — Change cost is concentrated in three files (enhancement, no defect)

Churn since 2026-10-20: `src/gui/assets/app.js` 276 commits,
`src/gui/bridge.rs` 229, `src/gui/assets/app.css` 145, `src/agent.rs`
123, `src/main.rs` 114, `src/gui/server.rs` 104. Sizes: `app.js` 32 K
lines, `bridge.rs` 21,942 (production lines end at 12,306), `agent.rs`
19,090 (production to 8,060). The three hottest files are also the three
largest, and `bridge.rs` carries 68 public functions.

This is a shape question, not a bug: splitting `bridge.rs` by route
family and `app.js` by surface would make review and the guard lists
cheaper — but a mechanical split of a 12 K-line module is a plan of its
own, and the face's self-containment rule (no build step) argues for
keeping `app.js` a single served module. Worth a ruling, not a rush.

### F6 — `CLAUDE.md`'s "Today's state" is five weeks stale (docs, low)

It says 45 modules, ~70 K lines, 2,357 tests, 0.6.26. Measured: 95
modules, 267 K lines, ~2,450 tests, 0.8.21. The file's own protocol
("state bullets go stale; cross-check them") is why this is low — but
the fixed numbers age every session. Proposal: state invariants
(module count is "~100", tests "~2.5 K"), or drop the counts and point
at the newest recap, which is already dated by its filename.

### F7 — Small hygiene items (low)

- **48 duplicate crate versions** (`cargo tree -d`): `base64` ×3,
  `getrandom` ×3, `hashbrown` ×3, `bitflags`, `core-foundation`,
  `fastrand`… most are transitive; the direct `base64 = "0.22"` could
  ride whatever `aes-gcm`/`ureq` already resolve to. Build time and
  audit surface, not correctness.
- **Two plans stale at "?"** since 2026-10-29 (`turn-feedback`,
  `workspace-history-truth`) — close, resume, or cancel.

## What is healthy (measured, worth stating)

- The ledger's discipline is the repository's strongest asset: five open
  rows out of 575, each with a next action, and a checker that refuses
  malformed rows.
- Plan hygiene: 167 completed, one active, two drafts; the pre-commit
  hook that guards unticked criteria is live.
- The memory substrate's index is current (964 of 965 indexed) — KI-332's
  class is not currently present.
- Release automation landed its own fix (KI-556: `--app-version` is
  passed in `scripts/release.sh:426`).
- CI is disarmed *by name*, not by rot; the local gate is the documented
  authority.
- Disk headroom is real (82 GiB free; the sweep keeps `target/` at 4.1
  GiB against the 159 GiB incident).

## Not verified this session

- The full four-leg gate: `cargo test`/`build` were not run (the
  in-flight session held the build; the tree is dirty), and `clippy`'s
  state is what the ledger says (KI-572 fixed today), not what I
  measured. Only the fmt leg was measured live (red, F1).
- Any live face behaviour: no browser run; the face claims here are
  source-level (`served()`, `versioned_asset()`, `include_str!`).

## Assumptions to confirm before any plan

Per the deep-dive discipline: the four families in F3 are treated as
*assumed gaps*, not defects — each might be a deliberate per-surface
choice. Confirm or rule:

1. F3.1 card clipping: is "fix the card, move on" the policy, or should
   one builder own every fixed-width surface?
2. F3.2 corpus root: is the `drive`/`cron` mirror deliberate enough to
   keep, or is one resolver wanted?
3. F3.3 silent presses: is a shared press-feedback rule wanted, or is
   each surface its own?
4. F3.4 test isolation: one env/port primitive, or keep per-test fixes?
5. F4 compression: acceptable for the relay path today, or wanted now?
6. F5 module split: keep the three big files, or split `bridge.rs` by
   route family in a plan of its own?
7. F6: keep exact counts in `CLAUDE.md`, or move to invariants?
