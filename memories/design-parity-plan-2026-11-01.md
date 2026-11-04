---
name: design-parity-plan-2026-11-01
description: "project 2026-11-01 — the design-parity plan (five gaps + the fan-out amendment) LANDED WHOLE on design-parity (0303ac8..8bb7925) and MERGED to master at ecbf280 (four folds credited, merged-tree gate 1,723 lib / 0 failed): K-block knowledge seams first, then P1–P5, all 16 criteria ticked; the relay-secret leftover RESOLVED same evening by the rotation + KI-393 fix (737dadc)"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-11-01, `/goal execute in the order that makes sense for both plans,
no leftovers or lazy deferrals` — both plans built whole on
**`design-parity`** and pushed (`0303ac8..8bb7925`, 0 ahead). Order:
the project-knowledge seams first (they reshape the parity files), then
the parity phases hinge-first, then close-out.

**Commits:** K1–K6 `223e4a5` (binding field, digest at the save funnel,
seed glob + KI-277 reconcile, lifecycle facts, the assembly's design
section), K2 grouping `67a4764`, P1 `4f2ee6f`, P2 `466e36c`, P3
`c61a0e5`, P4 `fb757a1`, P5 `def9867`, docs `e66de1b`, plans-completed
`8bb7925`. Close-out evidence: **gate four legs exit 0 — 1,908 tests / 0
failed across all targets** (lib 1,714; harness **608 checks / 56
cases**), ledger checker green (392 rows, zero problems), C1's 1440×900
render read (floating stack, focus chip `board · Board 1`), C13's live
curl (`/site/sample-board/` 200, `connect-src 'none'` in the CSP
header, page links rewritten). The parallel session committed its KI-369
`fleet mcp call` onto this branch too (`6f434e5`).

**Shipped surface:** the composer floats bottom-center over the canvas
(log collapsed, seat, focus chip `board · <name>` / `page · <name>`;
background press clears focus; mode follows focus); boards/pages CRUD
(head inline rename, confirm-delete taking store+digest+fact, in-place
page rename, the `frame-set` op + bar picker); viewport presets on the
frame bar; the frame line with description + `gen_meta`; the review
card's critic pin, score deltas, fix-drift over coherence findings; the
save chip (`saving…`/`saved`/`save failed: <reason>`); every export
downloadable via the one-segment route; **the local host**
`GET /site/<slug>/<page>` (live document, rewritten links, network-dead
CSP) + the site ZIP; the model chips as a **multi-select fan-out** (one
pressed = one pinned run; K pressed = K runs, K pins, one progress line
each).

**Two harness lessons (worth their own form):**
- The harness's ids are a **flat map** — `new El('div')` per id, no
  parent/child tree, no bubbling. Parentage/geometry are the render
  leg's; drive listeners directly (`el.fire(type, {target})`).
- The model chips' first cut **orphaned the clicks**: the paint
  reassigned `face.designModelPicks = picks.filter(...)` AFTER building
  handlers that closed over the old array, so every toggle mutated a
  dead array (the harness caught it as a one-model "fan-out"). Mutate
  the live array in place; filter/reassign only BEFORE handlers exist.
  A test seam (`TEST_BOARD_FACT_ROOT`, `TEST_DESIGN_CONTEXT`) keeps
  lifecycle-fact and design-section writes on fixture roots — any other
  resolution under test walks into the REPO's real corpus (the KI-087
  walk in reverse); the pin's first pre-seam run leaked
  `memories/design-board-harbor.md` and it was removed.

**Leftovers (named, none of the plans'):** ~~the relayed faces are DOWN~~
— RESOLVED same evening: both relay secrets rotated (OpenShip env
set+apply, proofs in `.env` + plist), the launchd face rejoined both
relays, and KI-393 fixed at `737dadc` the real cause (the join-proof
check read process env only while `.env` sat unread) — see
[[relay-join-secret-rotation-2026-11-01]]. Still open: the
corpus-refresh untracked cluster stays uncommitted; fleet's vendored
sys1 still carries `MAX_CRITERIA` (KI-392's next action, rides the next
vendor sync). Related:
[[designs-are-project-knowledge-plan-2026-11-01]],
[[a-kernel-is-not-the-product]], [[the-screenshot-is-the-visual-review]].

**Gallery + two shoot-found fixes (2026-11-02, `735f462` + `572852b`,
pushed to master; rebased over the parallel session's `5a65a6a`).** The
owner asked for "all relevant documentation including the README with
screenshots" — six shots at 1440×900 from a **scratch home**
(`FLEET_HOME=/tmp/fleet-gallery-home`, its own face on :3099, real canvas
never touched; `config.json` copied, placeholder OLLAMA/ZAI keys for
availability only, fixture `Version`s need an `html` field): README's
design bullet rewritten for the parity surface + a three-row gallery
table, `docs/features/design.md` gains "In pictures"
(`../images/…` — relative paths from `docs/features/`). Reading the
frames found two real defects the harness is blind to: every act on a
selected page's title bar was **dead** (the bar's `pointerdown` called
`preventDefault` + `setPointerCapture` before the control's click
landed; fix = bail before `preventDefault` when the target is
`button, select, a, input`; filed **KI-394**) and the mode select
clipped `refine — Landing` mid-word (widest share + ellipsis). The
shoot script per-state fresh pages (`goto → drive → screenshot`), because
one page's accumulated screenshots made a later frame look pane-less —
that was my misread, not a defect. Gate on the merged tree:
fmt/clippy/build 0, **1,714 lib + 194 integration / 0 failed**, harness
**608/56**, ledger 394 rows / 7 open. **Trap:** `cargo test --release`
reds `the_binary_check_accepts_the_gate_roots_own_target_dir` because the
test binary lives under `target/release/deps` and that IS under
`target/release` — a pre-existing dev-profile-only pin, not a defect of
the change; gate with plain `cargo test`. A KI-393 collision: an
overnight `corpus-refresh` run filed a second "board saves write no
canvas digests" row on a stale base; the digest writer had already
landed in the K-block, so I **dropped** the row rather than renumber it
(rules say the ledger is re-read, the stale row is obsolete). Also:
`cargo` is not on PATH here — prepend
`$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin`.

**Merged to master (`ecbf280`, pushed `314a7f4..ecbf280`)** — master
had moved far (the corpus-contract W1–W5, O6/O7, the settings-UX plan
drafts), so the merge was built on a sibling branch from master's tip
(their worktree `fleet-master-wt` held master with UNCOMMITTED
settings-ux edits — never touched) and folded deliberately, credited:
cron.rs's seed task carries their O7 `fleet mcp call` rewrite AND the
digest glob; mcp_registry keeps master's newer `spec_for` seam;
known-issues and open-calls keep master's completed O6/O7 records (the
branch forked before those close-outs). The MERGED tree gated green —
fmt/clippy/build/test 0, **1,723 lib / 0 failed** (their W-blocks + the
K/P blocks together), ledger checker 392 rows / 7 open. The branch is
fully contained in master and deletable; the parallel session's next
pull brings the merge and their uncommitted app.js/settings.rs edits
fold over the parity rewrites.
