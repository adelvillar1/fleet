---
status: draft
created: 2026-11-12
updated: 2026-11-12
slug: workspace-rebuild
---

# Plan: the workspace rebuild — the face becomes the mockups

## Context

The audit ([`../architecture/2026-11-12-ux-audit-and-rebuild.md`](../architecture/2026-11-12-ux-audit-and-rebuild.md))
compared the three approved mockups against the shipping 0.7.5 face and found
the gap is architectural, not cosmetic:

- the mockups are **one workspace over one graph**; production is **21 panels
  in 4 rail groups**, each owning its own store;
- `GET /api/tissue` returns **22 nodes and 0 edges** on a live home — the
  "connective tissue" has no connections, because the graph's only edges are
  minted by `task→thread`/`task→bot` and no tasks exist;
- `/api/tasks` and `/api/desk` are **empty in real use** while 280 runs sit in
  sessions — the work model does not match where work actually lands;
- `pivotTo` swaps panels despite its "renders IN PLACE" comment, and the S6
  spine read `/api/workspace` is fetched and consumed by nothing;
- 446 CSS rules set type below 12px; the rail names subsystems a user cannot
  know (`router`, `delegation`, `second brain`, `workspace history`, `vault`,
  `telemetry`, `huddles`).

The owner's ruling: rebuild the face. The agent core, the bridge's reads, the
ledgers, the release discipline, and the beta bar are untouched.

## The owner's rulings (2026-11-12, this session)

1. **The work item is the unit of record.** A *work item* owns everything: a
   session is its transcript, an artifact is what it produced, a lesson and a
   review are what it taught and how it was graded. The board, the desk, the
   threads and the graph are four views of work items.
2. **The 12 destinations with no place in the approved design move to
   Settings → Advanced** — one page each, reachable, never in the main rail:
   router, delegation, second brain, workspace history, vault, plugins,
   telemetry, design, bots, bundle, settings, about.
3. **Scope: the full rebuild**, reusing the mockups as the acceptance
   references and this plan as the contract.

## Design choices inherited

**U0–U42** in [`UI-DECISIONS.md`](../plans/UI-DECISIONS.md) — in particular U0
(one graph, every surface a projection), U3 (one focus), U4 (the eight signal
hues, no tenth), U5 (tokens, no literals), U9 (the shell grid), U12 (console +
collapse doors), U14/U17/U21/U23 (board, inbox, optimistic, bulk), U20/U39
(pinned composer, the generated-objects pane), U29 (armada additive), U30–U33
(desk loop, calibration word, dream, learning), U34–U38/U41 (setup as graph
growth), U40 (Intelligence), U42 (the metaphor is architecture, never a label).
Hard rules 5, 11, 12.

## Design choices this plan settles (append at P0)

- **U43 — The work item is the unit of record.** One noun in the UI (`work` /
  `work item`; never "task AND deliverable AND session"), one id space, one
  status vocabulary (`queued · working · blocked · done`), one transcript
  (the session), one artifact set. Every lens is a projection of the same
  collection. *Constrains:* the spine read, the board, the desk, the threads
  lens, the graph, the onboarding empty states.
- **U44 — Seven lenses, everything else Advanced.** The rail is Desk, Threads,
  Board, Intelligence, Learning, Reviews, Inbox, Fleet (eight with armada on;
  Desk/Threads/Board are three projections and may land as two groups).
  Everything else lives under Settings → Advanced and is reachable, named, and
  never in the main rail. *Constrains:* the rail, the palette, the address
  grammar, every deep link.
- **U45 — Two registers, plain by default.** Every user-facing string resolves
  through one vocabulary file; subsystem names (sys1, corpus, gate, armada,
  principal, calibration, lane, substrate) are the **expert** register, shown
  only when the switch is on. *Constrains:* every label, empty state, toast,
  error, and settings page.
- **U46 — Type floor and pair measurement.** 12px minimum (one tertiary
  micro-label may be 11px); small text (`<13px`) must clear **7:1** against
  its actual background. The contrast guard measures **component pairs**
  (chip text on chip wash, label on overlay, graph label on page), not only
  tokens against surfaces. *Constrains:* the token set, every component, the
  guard.
- **U47 — One address grammar, no panel registry.** `#/<lens>` and
  `#/<lens>/<id>`, parsed in one place. No `PANEL_EL`, no hidden-panel loop,
  no arrival table that rewrites the address on entry. *Constrains:* the
  router, every link, deep links from the CLI and the TUI.

## Approach — seven phases, ordered by dependency

The order is load-bearing: **P1 before P3** (or the new shell is prettier
silos — a lens with no edges to project), **P2 before P4** (or the rebuild is
attempted inside a 30k-line closure). Each phase is its own gated increment;
P3 and every P4 lens is its own release.

### P0 — the contract (docs only, no cut)

- `docs/design/VOCABULARY.md`: every noun and verb the UI may say — plain
  word, one-line meaning, expert synonym, forbidden alternatives.
- `docs/design/OBJECT-MODEL.md`: the work item and its edges (transcript,
  artifacts, facts, skills, lesson, review, bot, instance), field lists,
  status vocabulary, and the migration note for today's sessions.
- Append U43–U47 to `UI-DECISIONS.md`; update the plan index.
- Acceptance: the two docs exist, the five U-ids are in the ledger, and every
  string currently in `index.html` maps to a vocabulary entry or is marked
  for deletion.

### P1 — the spine (backend; the decisive phase)

- Extend `GET /api/workspace` into **the** read: every work item with id,
  label, status, stage, gate, coverage, owner (bot), instance, timestamps,
  and its artifacts/thread/facts/skills/lesson/review; plus `bots`,
  `instances`, `intelligence` counts. Sessions become the thread's transcript
  inside the work item (not a peer list).
- **Give the graph edges that exist**: `work→thread`, `thread→artifact`,
  `thread→fact`, `thread→skill`, `artifact→lesson`, `artifact→review`,
  `work→bot`, `work→instance`, `instance→mind`. `/api/tissue` derives from
  the same read; a live home with runs must report **edges > 0**.
- One relation source: extend `src/workspace.rs` (or its successor) so the
  graph and the spine cannot disagree.
- Keep the old endpoints alive but unconsumed (the TUI and the CLI use some).
- Acceptance: on this machine's real home, `/api/workspace` returns the 12
  scheduled sessions **as work items with their transcripts**, and
  `/api/tissue` returns non-zero edges; a test pins that a session with a
  tool call yields a thread→artifact or thread→fact edge; the four-leg gate
  is green.

### P2 — split the front end (mechanical; no user-facing change)

- Convert `app.js` (30,010 lines, one IIFE) to native ES modules, no build
  step, no framework (hard rule 12): `core/` (store, api, router, events),
  `shell/` (rail, topbar, console, connections, palette, drawer, toast),
  `lenses/`, `components/`, `settings/`.
- Delete on the way out: the tower console, `#panel-later`, `#panel-huddles`,
  the duplicate bots block (app.js:19206/19377), the legacy skills surface.
- `index.html` keeps one stage shell; the 20 static `<main>` panels go.
- Acceptance: the face behaves identically (harness green case-for-case
  before/after), no module over ~900 lines, `grep -c 'PANEL_EL'` is 0, and
  the bundle still needs no build.

### P3 — the shell (the law, once)

- `focus(kind, id)` is the only selection; the focused object's home lens is
  the only reason the centre changes; the connections column and focus card
  never remount on pivot.
- Rail: the seven/eight lenses with live counts; Settings → Advanced holds
  the 12 orphans, one page each.
- One address grammar; deep links `#/<lens>/<id>` resolve to focus; browser
  back/forward works; no arrival rewrites the address on entry.
- The console keeps its live controls (density, motion, connections, labels,
  armada) and gains **nothing new**.
- Acceptance: a harness case proves a pivot does not swap `main` elements;
  a case proves `focus` survives a lens change; a case proves every orphan
  route resolves under Settings → Advanced; the 7-lens rail renders with
  live counts from one read.

### P4 — the lenses, each its own cut

Order: **Threads → Board → Desk → Intelligence → Learning → Reviews → Inbox →
Fleet.** Threads goes first because it is where work happens and it proves
the streaming/pane contract; Desk goes after Board because the board's status
model feeds the desk's stage projection.

Per lens, one release: new lens body, its harness cases, its empty states
naming the step that fills them, the old panel deleted (not hidden), the
deep links redirected. Onboarding's graph-growth re-wire (U34–U38, U41) is
rebuilt **after** Fleet, on the finished shell, from the same spine read.

### P5 — type and colour (measured)

- Type scale as tokens: 12px floor, 13–14px body, one 11px tertiary label.
  Delete the 8–11.5px band (446 rules today).
- Every accent gets a measured `on-<accent>` ink; washes carry text at the
  size-appropriate ratio; extend the guard to component pairs and hold small
  text to 7:1.
- Graph labels: word-boundary truncation, hide below a threshold, never a raw
  id or a sentence.
- Acceptance: `grep -cE 'font-size: *(8|9|10|10\.5|11|11\.5)px'` on the
  rebuilt CSS is 0 except the one named tertiary token; the guard reports
  every pair; the visual check vs the mockups passes.

### P6 — language

- The vocabulary file becomes the copy source; a test refuses a user-facing
  literal that is not in it. Plain register by default; Settings →
  Advanced → **Expert view** reveals the subsystem names.
- Acceptance: a grep of the built face finds no forbidden term outside the
  expert register; every empty state names the step that fills it; the
  visual check reads a job end to end without a subsystem name.

### P7 — delete and reconcile

- Remove the remaining dead surfaces, the three navigation declarations, the
  six hash parsers. Record before/after counts (app.js lines, routes, panels,
  CSS rules <12px).
- Update the ledgers: `UI-DECISIONS.md`, the plan index, `TECHNICAL-DOCUMENTATION.md`'s
  interface section, `FUNCTIONAL-SPECIFICATIONS.md` (surface by surface), the
  handoff packet's traps, and `CLAUDE.md`'s pointer index.

## Acceptance criteria

- [ ] C0 (P0): the vocabulary and object-model docs exist; U43–U47 are in
      `UI-DECISIONS.md`; every current UI string is mapped or marked for
      deletion.
- [ ] C1 (P1): one spine read serves every object with real edges; on this
      home `/api/workspace` shows the scheduled sessions as work items and
      `/api/tissue` returns edges > 0; the edge contract is pinned by test.
- [ ] C2 (P2): the face is native ES modules, no build step, harness green
      case-for-case; `PANEL_EL` is gone; no dead surface carried over.
- [ ] C3 (P3): one focus, one address grammar, seven/eight lenses, twelve
      orphans under Settings → Advanced; pivot re-projects without swapping
      the stage; pinned by harness cases.
- [ ] C4 (P4): all eight lenses shipped one at a time, each with its negative
      cases and its empty states; no old panel remains reachable; onboarding
      rebuilt on the finished shell.
- [ ] C5 (P5): the type floor holds; component pairs are measured at 7:1 for
      small text; graph labels are readable at any density.
- [ ] C6 (P6): plain register ships by default; the expert switch reveals the
      subsystem names; the vocabulary lint is green.
- [ ] C7 (P7): the dead code and duplicate declarations are gone; the counts
      are recorded; the ledgers and the pointer index are reconciled.
- [ ] C8: every phase carries a local four-leg gate on its committed tree, a
      green face harness, and the visual check against its mockup section;
      the version declarations move per release; **the beta bar is untouched
      — 0.x is still beta, the owner's call.**

## Files to be touched

**Backend:** `src/gui/server.rs` (the workspace/tissue arms),
`src/gui/bridge.rs` (the spine + the edge derivation), `src/workspace.rs`
(the relation source), `tests/` for the edge contract.
**Face:** all of `src/gui/assets/` — `app.js` → `core/ shell/ lenses/
components/ settings/`, `index.html` (one stage), `app.css` (tokens, type
scale, component pairs), `tests/face/harness.mjs` (rewritten around the
stage instead of panel ids).
**Docs:** `docs/design/VOCABULARY.md`, `docs/design/OBJECT-MODEL.md`,
`docs/plans/UI-DECISIONS.md` (U43–U47), `docs/plans/PLAN-INDEX.md`,
`TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`, `CLAUDE.md`.
**Not touched:** the agent core, the tools, the routing, the lanes, the TUI
(its boards keep working against the old reads), the release scripts.

## Out of scope

- The TUI's visual system (it already carries the status colour law; a
  rebuild there is a separate plan).
- Lane 2 (the org), multi-user, the vault UI.
- Any new agent capability. The rebuild changes what the face *shows* and how
  it is navigated; it adds reads and relations, not tools.
- Promoting beta.

## Verification

Per phase: `node tests/face/harness.mjs` green (no case silently dropped —
the old panel cases are replaced by lens cases in the same commit), the
four-leg gate on the committed tree, and the visual check — the lens beside
its mockup at 1440px and at 1080px. The mockups stay the acceptance
references; a divergence is recorded in `UI-DECISIONS.md` or fixed, never
quietly accepted.

## Risks

- **The spine is the whole bet.** If P1 ships without real edges, P3's shell
  will render an empty connections column beautifully. C1 is the gate that
  must not be waved through; verify it on this machine's real home, not a
  fixture.
- **Folding sessions into work items can lose history.** The migration must
  be additive (a session becomes a work item's transcript; nothing is
  rewritten) and the old archive must stay readable through the old endpoint
  until P7.
- **A module split can become a rewrite.** P2 is behaviour-preserving by
  definition: same DOM, same ids, harness green case-for-case. Any temptation
  to "fix it while we're in there" moves to P3/P4.
- **Type and colour changes late can invalidate the lens builds.** If P5
  lands after P4, each lens is restyled twice. Mitigation: the type scale and
  the pair tokens land in **P0's** contract as the design target, and P5 is
  the enforcement pass, not the first definition.
- **The owner's stored theme** (paper, on this machine) is not the ledger's
  default (night). Every phase's visual check runs in both.

## Dependencies

- Approved: this plan (status moves to `active` on the owner's word).
- The three mockups under `docs/design/` and `UI-DECISIONS.md` (U0–U47 after P0).
- The audit: [`../architecture/2026-11-12-ux-audit-and-rebuild.md`](../architecture/2026-11-12-ux-audit-and-rebuild.md).
- The stale deployed binary on :3080 (0.6.27) is **not** the reference — the
  reference is the built tree; the deployment is refreshed by the first cut.

## Notes

The old face is preserved as-is at tag `v0.7.5` — the rollback point, and the
comparison baseline for the audit's numbers. The plan does not delete it; P7
does, once the new shell has carried a full week of real work.
