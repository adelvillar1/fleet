# UX audit — the mockups vs the shipping face, and what a rebuild means

**Date:** 2026-11-12 · **Version audited:** 0.7.5 (face at `src/gui/assets/`)
**References:** `docs/design/fleet-workspace-mockup.html`,
`docs/design/armada-workspace-mockup.html`,
`docs/design/fleet-onboarding-mockup.html`, `docs/plans/UI-DECISIONS.md` (U0–U42)

This is the review the owner asked for: read the three approved mockups, read
what ships, and say whether we need to merge surfaces, rearrange things, or
collapse workspaces. Everything was on the table.

**Verdict:** the mockups are a workspace. Production is a dashboard of
panels. The gap is structural, not cosmetic — the v0.7.x series shipped the
visual language and several good panes, but never the shell's law (one graph,
one focus, every surface a projection of it). A further reskin cannot close
it; the face needs a rebuild. The backend, the agent core, and the ledgers are
not the problem and are not in scope.

---

## 1. The measurements that decide it

Every one of these was read from the live 0.7.5 build, not inferred.

| Measure | Mockups | Production (0.7.5) |
|---|---|---|
| Top-level destinations | **7 lenses** (Desk, Threads, Learning, Intelligence, Calibration, Inbox, Fleet) | **20 rail entries in 4 groups + 21 routes** |
| Navigation model | one focus `{kind,id}`; every lens is a projection of one graph | **three selection models** (`face.focus` route, `face.focusObject` tissue, panel-local drawers) |
| Graph edges in real use | the whole UI is edges | `GET /api/tissue` → **22 nodes, 0 edges** |
| Work objects on screen | tasks, threads, deliverables, lessons, facts, skills, tools, workflows, calib, instances — all one graph | board `[]`, desk `[]`, workspace `[]` — real work lives in *sessions*, which are **not graph objects** |
| Front-end size | 4 mockups × ~1.1k lines, one file each, ~450 lines of JS | `app.js` **30,010 lines**, one IIFE; `app.css` 11,668 |
| Type scale | 12–14px body, 11px labels | **446 rules below 12px**; 8px–11.5px is the house norm (3× 8px, 14× 8.5px, 30× 9px, 75× 9.5px, 91× 10px…) |
| Colour | measured pairs in the mockup | contrast guard validates **7 text tokens × 3 surfaces** at token level — never a component pairing, never the overlay surface, never a chip's text on its own wash |

Three of these deserve to be spelled out, because each one alone explains the
"disjointed" feeling.

### 1.1 The connective tissue has no edges

`bridge.rs` builds `/api/tissue` edges from the workspace graph's own edge
list (`bridge.rs:2055`). The workspace graph's edges are `task → thread` and
`task → bot`, minted only by dispatch/link. With no tasks, there are **no
edges**. Sessions — where every real run lands — are appended as nodes
(`bridge.rs:2055` region) with no relation at all.

So the column the owner named "connective tissue" renders a ring of isolated
dots. The labels are the raw strings, so they arrive as
`[scheduled run — catch-up for job \`corpus-refresh-deepseek-harness\`: the
2026-11-13T00:55Z window was missed while fleet was down]` — clipped to a
16-character node label, overlapping its neighbours. That single artifact is
the readability complaint, the jargon complaint, and the architecture
complaint at once.

### 1.2 The work surfaces are empty because work doesn't land there

Live reads on the running 0.7.5 face:

```
GET /api/tasks     → {"tasks":[]}
GET /api/desk      → {"projects":[],"unreadable":[]}
GET /api/workspace → {"tasks":[],"deliverables":[],"instances":[],
                      "threads":[ "…[scheduled run — job corpus-refresh-sys1…]" × 12 ]}
```

The board renders its honest empty state ("The tasks read has not landed
yet."). The desk is empty. Meanwhile the home card counts **817 facts, 460
skills, 280 runs** — the work exists, it just lives in sessions, and sessions
are not objects in the model the workspace is built on.

That is the deepest finding: **the mockups model work as tasks that become
deliverables; production models work as sessions that become files.** A
rebuild has to pick one and make every surface a view of it. It should be the
mockup's model, with sessions folded in as the thread's transcript rather than
as the unit of record.

### 1.3 "Render in place" is not what the code does

`pivotTo` (app.js:29219) is documented as "renders IN PLACE" and the S6
acceptance criterion was ticked on that claim. The function sets focus, calls
`history.replaceState`, then calls `showRoute()` — which runs the panel-hide
loop at app.js:27027 (`PANEL_EL[key].hidden = …`). The only difference from a
plain rail click is that no `hashchange` fires. The lens still swaps the whole
panel; nothing re-projects in place. `pivotTo('thread')` returns early to
`openWork`, which sets `face.panel = 'sessions'` and **never hides the panel
the owner came from** — a known visible bug.

And `GET /api/workspace` — the S6 "spine" — is fetched by `readSpine`
(app.js:29165) and **read by no painter**. `face.spine` is written and never
consumed.

---

## 2. What the mockups actually specify

Worth restating plainly, because the intent is good and complete:

**The shell.** A rail of seven lenses with live counts, a centre that is one
lens at a time, a right-hand **Connections** column holding an ego-graph and a
**focus card**, a bottom console of live controls, and overlays (drawer,
command palette, toasts).

**The law.** One `state.focus = {kind,id}`. Everything else is derived:
clicking a node or a row focuses it; the focus card, the graph centre, and
the lens all follow; the lens changes only when the object's home lens differs.
Selection (bulk) is separate from focus.

**The object model.** `deliverable → thread, facts[], skills[], lesson, calib`;
`task → thread, bot, instance, deliverable, lesson`;
`workflow → skills, deliverable`; `skill → deliverables, workflows`;
`instance → deliverables`. Every edge in the graph is a real pointer in the
data, and the focus card's "Connections" list is the same function's output.

**The jobs it makes first-class.**

1. *New brief* → inline composer → deliverable at stage 0 + thread. No modal.
2. *Advance / settle* → the five-stage loop with the end gate, refusals shown
   on the card, coverage measured.
3. *Dispatch* → task/instance chosen, thread opens, run streams in the same
   workspace, and the objects it creates appear in the **"Made here"** pane
   with a fresh pulse as they land.
4. *Find work* → board columns (working/blocked/done, drag, bulk) and desk
   stage chips, both filtered by the same search.
5. *Capabilities* → Intelligence as four segments (memories, skills, tools,
   workflows) with counts, provenance, and the apply-through-the-gate action.
6. *Trust* → calibration rows with confidence meters and the owner's word
   riding the same append-only row.
7. *Grow the fleet* → setup as graph growth, then provision an instance that
   joins un-calibrated until its first settle.

**The armada mockup's addition.** The board becomes the default lens; tasks
carry assignee bot + instance; the fleet lens is instance cards with
liveness earned; the same shell, one switch away.

**The onboarding mockup's addition.** Setup is not a wizard — each step adds a
node to the same graph, the progress meter counts nodes, locked steps name
their predecessor ("unlocks when the judgment model is connected"), a skipped
step stays *named*, and completion is "the workspace is whole".

---

## 3. Where production diverges, surface by surface

| Mockup lens | What production actually has | Status |
|---|---|---|
| **Desk** | `#/desk` — shelf + project columns, its own store (`/api/desk`), its own composer placement | surface exists, **empty in real use**, no shared focus |
| **Threads** | `#/chat` + `#/sessions` + `#/huddles` — three addresses, one panel, one work column with four tenants | **split three ways**; the mockup's single "thread per work object" does not exist |
| **Board** | `#/board` — own store (`/api/tasks`), columns + drag | exists, **empty in real use**, disconnected from sessions |
| **Learning** | `#/learning` — lessons + practice | closest to the mockup; still a private store |
| **Intelligence** | `#/memory` (+ legacy `#/skills` redirect) | shipped with four segments; the **best-executed** part of the reskin |
| **Calibration** | `#/calibration` | exists; separate store |
| **Inbox** | `#/inbox` (owner queue) **and** `#/bots` (channels), two different inboxes | **split two ways**, neither is the mockup's omnichannel stream |
| **Fleet** | `#/armada` | exists; instance roster only |
| *(no counterpart)* | `#/router`, `#/delegation`, `#/second-brain`, `#/workspace-history`, `#/vault`, `#/plugins`, `#/telemetry`, `#/settings`, `#/about`, `#/bots`, `#/design`, `#/bundle` | **12 destinations with no place in the approved design** |

Plus, in the code: `#panel-huddles` is dead markup; `#panel-later` is
unreachable; the tower console is entirely dead code; the bots block is
**duplicated verbatim** (app.js:19206 and 19377, 171 lines each); `skills`
survives as a legacy duplicate of an Intelligence segment; `BUILT` is a third
declaration of the route set that `ROUTES` already holds.

### The five structural seams

1. **Two selection models with different lifetimes** — `face.focus` (route)
   vs `face.focusObject` (tissue). `openRoom` writes only the first;
   `openDeskProject`/`openLesson` only the second; the comments claim one.
2. **Every lens owns a store and reads it on entry** — 21 separate reads, 4s
   caches of different lengths, a 5s poller that only refreshes the active
   panel. No invalidation, no shared identity.
3. **Panels are rebuilt wholesale on every paint** — `textContent = ''`
   then rebuild; scroll position, selection, and transient DOM state die on
   every repaint.
4. **The same object renders differently in five places** — a task appears on
   the board, in the goal pane, in the thread pane, in the desk as a
   "deliverable", and as a tissue node; none share a shape.
5. **Routing is parsed in six places** with six grammars (`showRoute`,
   settings `parts[2]`, `memorySectionId`, `calibrationDay`, `designSlug`,
   `workspaceSectionId`), and three arrivals rewrite the address on entry.

### The two quality findings

**Readability.** 446 CSS rules set type below 12px; the modal size is 10px.
Micro-caps + mono + 9.5px is the default voice for labels, chips, legends,
graph labels, and table headers. The contrast guard passes because it measures
seven *tokens* against three *surfaces* — it never measures a 9.5px muted
label on `surface-overlay`, a chip's text on its own 12% wash, or a graph
label over the ambient field. Passing 4.5:1 at 9.5px is still hard to read;
WCAG's floor is a floor, not a design target.

**Jargon.** The rail names the implementation: `router`, `delegation`,
`second brain`, `workspace history`, `calibration`, `vault`, `plugins`,
`telemetry`, `bots`, `huddles`, `sessions`, `armada`. The copy repeats it:
"lanes", "sys1", "gate", "seat", "breakout", "distil", "brief", "shelf",
"basis", "anti-vector", "the corpus", "the decisions queue". U42 retired the
*metaphors*; it did not touch the *subsystem names*. A user does not know what
a sys1 is, and should never have to.

---

## 4. What a rebuild looks like

Not "another reskin". The sequence below is ordered by dependency, and the
first two steps are the ones that make the rest cheap.

### P0 — contract v2 (docs only)

- A single **user-facing vocabulary** file: every noun the UI may say, in
  plain language, one line of definition, its expert-view synonym. This
  becomes a lint: a label not in the file cannot ship.
- The **object model** the shell will serve, written once: work item, thread
  (transcript), artifact (deliverable), lesson, fact, skill, tool, workflow,
  review (calibration), message, instance, bot. Field lists + edges.
- The **seven lenses** and what each answers: *What needs me? What is being
  worked on? What landed? What does it know? What can it do? Is it right?
  Who/what is talking to it?*
- U43+ appended to `UI-DECISIONS.md` for every decision this forces —
  especially the vocabulary law and the type scale.

### P1 — the spine (backend, small, decisive)

- Extend `/api/workspace` into the **one read**: every object, with ids,
  labels, status, and edges. Sessions become real objects (a thread's
  transcript), not a separate list. `/api/tissue` derives from it.
- Give the graph **relations that exist**: thread→artifact, thread→facts,
  thread→skills, artifact→lesson, artifact→review, work-item→bot/instance.
  Today `0 edges` is not a rendering bug, it is a missing model.
- Retire the duplicated reads: `tasks/desk/sessions/huddles/learning…` stay
  as endpoints but the shell stops using them one by one.

### P2 — split `app.js` (mechanical, before the rebuild proper)

`app.js` is 30k lines in one IIFE. The rebuild will not fit in that file.
Split into native ES modules, no build step (hard rule 12 stays):

```
src/gui/assets/
  core/      store.js (one state, one subscribe), api.js (the spine read),
             router.js (one address grammar), events.js
  shell/     rail.js, topbar.js, console.js, connections.js, palette.js,
             drawer.js, toast.js
  lenses/    threads.js, board.js, desk.js, learning.js, intelligence.js,
             review.js, inbox.js, fleet.js
  components/ card.js, chip.js, meter.js, stage-track.js, empty.js
  settings/  the advanced surfaces, one page each
```

The split is a refactor with the harness green at every step; it does not
change behaviour. `index.html` loses the 20 static `<main>` panels and keeps
one stage. Expected result: ~4–6k lines of readable modules instead of 30k
lines of one closure, and the dead code (tower, panel-later, panel-huddles,
duplicate bots) simply not carried over.

### P3 — the shell (the law, once)

- `focus(kind, id)` — one selection. Lens = a projection keyed off the focused
  object's home lens. Pivot **swaps the lens body only**; the connections
  column and the focus card never remount.
- Rail: seven lenses with live counts. Everything else moves behind
  **Settings → Advanced** (routing, delegation, versions, secrets, plugins,
  telemetry) — reachable, never in the main rail.
- Address grammar: one parser. `#/<lens>` and `#/<lens>/<id>`; no panel
  registry, no arrival table that hides panels.

### P4 — the lenses, in this order

Threads first (it is where work happens and it proves the streaming/pane
contract), then Board, Desk, Intelligence, Learning, Review (calibration),
Inbox, Fleet. Each lens replaces its old panels in the same release; the old
panel is deleted, not hidden.

Deferred to the end of P4: the onboarding graph-growth re-wire (U34–U38,
U41) — it reuses the same node/edge API and should not be built against a
shell that is about to change.

### P5 — type + colour (measurable)

- **Minimum 12px**, body 13–14px; kill the 8–11px band except for one
  genuinely tertiary micro-label at 11px. Type scale as tokens.
- Colour: keep the eight signal hues but **pair them properly** — every
  accent gets a measured `on-<accent>` ink; washes carry text at the contrast
  the text size requires. Extend the guard from *token × surface* to
  **component pairs** (chip text on chip wash, label on overlay, graph label
  on page) and raise the floor to 7:1 for anything under 13px.
- One graph label rule: wrap, truncate at word boundaries, hide labels below
  a zoom/space threshold, and never render a raw id or a 100-character
  sentence.

### P6 — language

The vocabulary file becomes the copy source. Two registers: **plain** (the
default) and **expert** (a switch that reveals sys1, corpus, gate, armada,
principal, calibration, lane — for the owner and for debugging). Model names
stay; subsystem names do not.

### P7 — delete

Remove the dead panels, the legacy skills surface, the duplicate bots block,
the tower, the three navigation declarations, and the six hash parsers. The
harness gains the new cases and drops the old panel ones.

---

## 5. What this costs and what it protects

- **Protects:** the agent core, the bridge's reads (they are the spine's raw
  material), the ledgers, the release discipline, hard rules 5/11/12, and the
  owner's beta bar (unchanged — this is not a beta exit).
- **Replaces:** the shell, the navigation, the front-end module structure, the
  type and colour system, and every user-facing string.
- **Deletes:** 12 destinations with no place in the approved design, four
  dead surfaces, three duplicate declarations, 30k lines of one closure.
- **Risk:** P1 must land before P3, or the new shell will be a prettier set
  of silos. P2 must land before P4, or the rebuild will be attempted inside a
  file too large to reason about. The order is not negotiable; the scope of
  each phase is.

## 6. Open decisions for the owner

1. **Is the task the unit of record?** Recommended: yes — a work item is the
   unit, a session is its transcript, an artifact is what it produced. This is
   the one decision every lens depends on.
2. **What happens to design / deck / second-brain / workspace-history?** They
   are real capabilities with no place in the approved design. Recommended:
   Settings → Advanced, one page each, no rail entry.
3. **Expert view: a switch or a mode?** Recommended: a switch in Settings that
   changes density and vocabulary, not a separate UI.
