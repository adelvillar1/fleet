# Session recap — 2026-11-01: design parity and project knowledge, landed whole

**Summary.** The two design plans built end to end and merged to master: boards
became project knowledge (binding, digest, corpus, facts, context, grouping —
K1–K6) and the canvas reached source parity on the owner's five issues plus
the fan-out amendment (P1–P5, C1–C16) — the centered composer with
first-class focus, boards/pages CRUD, page settings, save/downloads/the local
host, and the user-selected multi-model fan-out. The session also removed the
plan gates' 12-criterion cap (KI-392) and amended the gate rule to code
changes only.

## The work, in order

1. **Warmup and the triage context.** Both design plans drafted after a
   deep dive comparing the source design-canvas (`~/Projects/design-canvas`)
   with the native port: the kernel was whole (the whole-shebang T0–T6), the
   missing layer was exactly the owner's five issues. The knowledge plan
   followed the owner's steering message — design files belong to projects
   and must be part of knowledge, memories, context, embeddings.

2. **The gates' 12-cap removed (KI-392).** The owner's ruling mid-plan:
   *"a plan needs to have however many criterion it needs to pass."* All six
   ceilings came out — `_PLAN_GATE_MAX_CRITERIA`, `_PLAN_GATE_MAX_SECTIONS`,
   `_EVIDENCE_GATE_MAX_CRITERIA`, `_ARCH_GATE_MAX_CLAIMS` (both
   `dev_decisions.py` and its `devdec/gates.py` twin, one inode each), and
   sys1's `plansurface::parse_criteria` (`out[:MAX_CRITERIA]`). plan-gate,
   plan-surface and arch-gate now judge every criterion and claim; proven on
   the same plan that exposed the cap (14/14, then 16/16 after the fan-out
   amendment). Committed and pushed in their own repos (`57d3ad0` on
   dev-decisions, `1508b09` on sys1); KI-392 filed through `fleet ki append`
   — the locked verb's first real use. The fleet-vendored sys1 still carries
   the cap and rides the next vendor sync (the row's next action).

3. **The gate rule amended (hard rule 2).** The owner's ruling: the four-leg
   gate runs before every push **that carries code changes**; a docs-only
   push skips it — there is no compile surface. The exit-code rule (KI-014)
   stands for every run that runs. Landed `314a7f4`.

4. **The branch, then the build.** `design-parity` forked master so the
   ledger closing could keep holding master. The build walked the knowledge
   seams first (they reshape the files the parity phases touch), then the
   parity phases hinge-first, then the close-out:
   - **K1–K6** (`223e4a5`, `67a4764`): the board's `project` binding field,
     the deterministic digest at `canvas::save` — the one write funnel — the
     corpus-refresh glob with the KI-277 reconcile carrying it to existing
     jobs, the lifecycle facts through the substrate's own supersession
     chain (create remembers, rename/bind supersede forward, delete retires),
     the assembly's bounded design section with the `Record::Memory.design`
     field that keeps the prompt reconstructable (rule 5), and the rail's
     project grouping (a header row in the shared `sectionRail`).
   - **P1–P5** (`4f2ee6f`, `466e36c`, `c61a0e5`, `fb757a1`, `def9867`): the
     floating bottom-center composer with the focus chip and the wire
     contract (`frame_id` iff a page selected); boards/pages CRUD with the
     delete taking store, digest and fact in one write; viewport presets on
     the frame bar, the frame line's whole account, the review critic whose
     pick rides the run's pin, score deltas, fix drift over the coherence
     report, the set picker; the save chip's three states, every export
     downloadable through the one-segment route, and the **local host** at
     `/site/<slug>/` under a network-dead CSP with the site ZIP the same
     bytes; and the model fan-out — toggle chips over the designated
     catalog, one pressed is one pinned run, several pressed fans the brief
     out (K runs, K distinct pins, one progress line each).
   - **C14** (`e66de1b`, `8bb7925`): the contract docs carried the new
     surface and both plans flipped completed with landed-evidence sections.

5. **The merge.** Master had moved 19 commits (the corpus-refresh contract
   W1–W5, O6/O7, a settings UX plan) and was held by the parallel session's
   worktree with uncommitted settings-UX edits — so the merge was built on a
   sibling branch from master's tip and gated there: fmt/clippy/build/test
   all 0, **1,723 lib tests / 0 failed** across every target. Four folds,
   credited both ways: `cron.rs`'s seed task carries the O7 spawn-route
   rewrite *and* the digest glob; `mcp_registry` keeps master's newer
   `spec_for` seam; `known-issues` and `open-calls` keep master's completed
   O6/O7 records (the branch forked before those close-outs). Pushed
   `314a7f4..ecbf280`.

## Commits (this session, fleet)

| Hash | Message |
|---|---|
| `20415d3` | docs(plans): the model fan-out joins the scope, user-selected (P5, C15-C16) |
| `314a7f4` | docs(claude): hard rule 2 scopes the gate to code changes |
| `0303ac8` | chore: the design-parity branch opens, the plan goes active |
| `223e4a5` | feat(design): boards are project knowledge (K1-K6) |
| `67a4764` | feat(design): the boards rail groups by project (K2) |
| `4f2ee6f` | feat(design): the composer sits in the center frame, focus first-class (P1, C1-C3) |
| `466e36c` | feat(design): boards and pages CRUD complete (P2, C4-C6) |
| `c61a0e5` | feat(design): page settings parity (P3, C7-C10) |
| `fb757a1` | feat(design): save status, export downloads, and the local host (P4, C11-C13) |
| `def9867` | feat(design): the model fan-out — user-selected (P5, C15-C16) |
| `e66de1b` | docs(design): the parity surface reaches the contract docs (C14) |
| `8bb7925` | docs(plans): both design plans completed (C14) |
| `ecbf280` | Merge branch 'design-parity' — both design plans land whole on master |

Sibling repos: dev-decisions `57d3ad0` (the caps removed), sys1 `1508b09`
(`parse_criteria` returns every checkbox).

## What the build proved — and the two lessons it taught

Every criterion (C1–C16, K1–K7) is pinned: the digest's byte-determinism,
the fact chain's single live account, the reconcile boundary over a pre-glob
job, the context section both directions, the walk-out refusal on downloads,
the network-dead CSP, the fan-out's K pins — and the **live host** proven by
curl (`/site/sample-board/` 200, the index listing its pages with rewritten
links, `connect-src 'none'` in the header).

- **The harness's DOM is a flat id-map.** No tree, no `parentNode`, no
  bubbling: parentage and geometry belong to the render leg (the 1440×900
  Chromium render read for C1), and the harness drives listeners directly
  with an explicit target. The first C1/C2 cuts failed on assertions the
  shim structurally cannot make.
- **A repaint that reassigns under the handlers orphans the clicks.** The
  model chips' first cut rebuilt the picks array at the end of every paint,
  leaving each handler closing over an orphan — the harness caught it as a
  one-model "fan-out". The chips now mutate one live array in place, and
  the availability filter runs before the handlers exist.

## Honest notes

- **The relayed faces are down.** The face was restarted mid-session to
  serve the new build; `--relay` needs `FLEET_RELAY_JOIN_SECRET`, which
  lived only in the launching shell (not in `~/.fleet/.env`). The loopback
  face on `:3080` serves the merged build; restoring `fleet.opsh.io` /
  `fleetbot.opsh.io` needs the owner's relaunch with the secret (or the
  secret added to `.env`).
- **The K-block briefly landed on master.** A commit went to master before
  the branch discipline caught it; it was moved to the branch via a
  temporary worktree and removed from master cleanly (soft reset + checkout
  of the touched paths), with the parallel session's in-flight files
  untouched.
- **One test fact leaked into `memories/`** by the binding pin's first run
  (before its isolation seam existed — any other resolution would adopt the
  repo's corpus, the KI-087 walk in reverse). Removed; the pin now points
  at a fixture root, and the agent's design section has the same seam.
- The corpus-refresh cluster (memories churn, `scripts/refresh-*`,
  `.corpus-refresh/`, `.runs/`) remains uncommitted — the earlier session's
  work, untouched by this one.
- The parallel session's worktree still holds master at `fa71ffd` with
  uncommitted `app.js`/`settings.rs`/`harness` edits; their next pull brings
  the merge, and the app.js edits will fold over the parity rewrites.

## Doc updates applied this session

- `docs/features/design.md` — the floating composer and its focus,
  boards/pages CRUD, the boards-are-project-knowledge section, the
  multi-select fan-out, the new ops and routes (site, download), the save
  chip, the honest proven list.
- `docs/features/cron.md` — the corpus-refresh jobs' digest glob.
- `TECHNICAL-DOCUMENTATION.md` — the route list (board-project,
  board-delete, frame-set, download, site, the export kinds).
- `FUNCTIONAL-SPECIFICATIONS.md` — the design surface's new behavior.
- `docs/architecture/surfaces.md` — what is new on the design destination.
- `CLAUDE.md` — hard rule 2's scope; the branch topology.

## CLAUDE.local.md changes

None.

## Open questions / next steps

- Restore the relayed faces (the secret, above).
- The parallel session folds its settings-UX work over the merged master.
- The fleet-vendored sys1 sync (KI-392's next action: `MAX_CRITERIA` still
  in `services/sys1/src/sys1/plansurface.py`).
- `design-parity` is fully contained in master — deletable whenever.

## Notes

Cache sweep: debug 29.8 GiB → 21.3 GiB (dev profile cleaned whole, over the
backstop); free space 31.1 → 39.6 GiB; release targets untouched.
