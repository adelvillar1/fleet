# Handoff — the U-reskin implementation

**For the next agent implementing [`2026-11-12-ux-reskin.md`](2026-11-12-ux-reskin.md).**
Everything here was learned by driving the real thing; the mockups and the
ledger are the contract, this packet is the terrain.

## The owner's words (verbatim — the rulings)

- *"we are looking for connective tissue and to eliminate the disjointed
  surfaces. so wow me"* — and: *"this is both for armada and fleet"*.
- *"I like the armada design so we can expand this direction into fleet"* —
  one design system, both products.
- *"the design is approved"* — with amendments: *"on threads the composer
  needs to be pinned down at the end of the chat area"*; *"we still need the
  collapsible side pane that allows to explore, and view objects generated on
  the threads"*.
- *"maybe memories is renamed to intelligence, and we organize memories,
  skills, tools, workflows, etc in there"*.
- *"at a minimum we have a system one model, we could provide links to the
  preferred ones, fastino, liquidai, and drex pages. this is the step after
  setting a provider and a model."* — and the correction: *"liquidai is d1
  not jev"* (jev is a separate head; never conflate).
- *"record to the corresponding ledgers"* — every decision lands in its
  ledger; an unrecorded decision will be re-litigated.

## Read first

1. [`UI-DECISIONS.md`](UI-DECISIONS.md) — U0–U41, the contract. Cite the ids
   you implement in every commit.
2. The three mockups under `docs/design/` — **open them; they work.** They are
   the acceptance references, not wireframes.
3. [`2026-11-12-ux-reskin.md`](2026-11-12-ux-reskin.md) — the phased plan.
4. `CLAUDE.md` — the house rules (explicit paths, gate before push, no
   secrets).

## What exists (verified on this tree)

**Face API (GET unless noted)** — `src/gui/server.rs`: `/api/projects`,
`/api/workspace-history`, `/api/armada` + `POST /api/armada/dispatch` (L9),
`/api/calibration`, `/api/desk`, `/api/learning`, `/api/memory`,
`/api/skills`, `/api/huddles`, `/api/inbox`, `/api/onboard`, `/api/router`,
`/api/sessions`, `/api/state`, `/api/settings`, `/api/version`, `/api/demand`
… **Face conventions** — `app.js` is vanilla JS, one IIFE, no framework:
`ROUTES` (hash table), `GROUPS` (the rail + palette source), `el` bindings +
`PANEL_EL`, the `arrive` map (paint-before-read), `readX(force)` with a 5s
freshness cache on `face.*`, `paintX` from state, the `post(path, body)`
helper. **Harness** — `tests/face/harness.mjs`: zero-dep node runner;
`boot({hash, documents})`; a fetch stub with **explicit per-route branches**;
`check(name, ok)`; the `cases` array at the tail.

**Backends the reskin renders** — skills (`src/skills.rs`, `/api/skills`),
the desk loop (`src/desk.rs`, STAGES + `end_stands`/`end_desk`), calibration
(`bridge.calibration(day)`, the append-only rows), the workspace graph
(`src/workspace.rs`), armada (`src/armada/*`, `/api/armada`). Gaps to build
(READ endpoints over existing stores, no new capability): **tools** (the
registry/catalog read), **workflows** (the store read), and an intelligence
aggregate if the face wants one fetch.

## What NOT to rebuild

- **Do not port the mockup JS wholesale.** The mockups prove behavior and
  visuals; production follows the app.js conventions above.
- **No framework, no build step, no tenth hue, no hardcoded color literals**
  (U4/U5 — tokens only).
- **No wholesale rewrite of app.js** — incremental per section, harness green
  between sections.
- **Do not touch the release discipline** — cut per phase (0.7.x), three
  version declarations + lock + changelog head section move together, the feed
  read-back runs 90s (KI-526's window), explicit git paths only.
- **Do not promote beta** — the owner's bar is a whole project run reliably;
  0.7.x is still beta (the owner's call, never a plan's).

## The traps (each bit this session)

1. **The harness fetch stub needs an explicit branch per route** — the default
   returns `{}`; a missing branch renders an empty section *silently*. Add the
   branch + the `documents` key + a case in the same change.
2. **A new face section has five touch points**: ROUTES, GROUPS, `el` binding,
   `PANEL_EL`, the `arrive` arm — miss one and the rail or palette lies.
   Adding a GROUPS entry **moves the main-rail icon-count pin** — update the
   count and its comment in the same commit.
3. **Registry-style lookups must learn new kinds** — the mockup test caught
   `byId` missing skill/tool/workflow (the focus card went blank and the graph
   silently fell back to the home node). Search for kind maps when adding a
   kind.
4. **Persist per-thread message arrays before mutation** — `threadMsgs[id] ||
   defaultThread(id)` loses sent messages on the next re-render (the orphan
   array). Assign first.
5. **`prompt()`/`alert()` do not work in sandboxed preview contexts** — use
   inline composers with generated names.
6. **Streaming re-renders must be cheap and idempotent** — auto-follow the
   transcript only while the reader is already near the bottom (±90px); a
   pinned composer never scrolls away.
7. **Timing tests size budgets in polls/counts, not seconds** (KI-426's
   class) — and the integration tests that read the process-global
   `FLEET_HOME` hold `AMBIENT_LOCK` across the whole run, not acquire-and-drop.
8. **`renderFocus`-style functions bail on unknown ids** — a stale focus must
   fall back by name, never blank the pane (the mockup's `if(!o)` guard).
9. **The face honesty rules are load-bearing**: paint-before-read, refused
   reads name themselves, notes ride the `face` state object (KI-447) so a
   refresh does not erase them.
10. **The visual check is a gate, not a vibe** — open the mockup beside the
    section; the palette law (U4), the status law (U15), and the tissue
    presence (U0) are asserted by eye *and* by the harness.

## Where to start

Phase **Cut 1** (tokens + shell, 0.7.1): `app.css` tokens → theme toggle →
rail reorganization. Everything else themes through the tokens, so nothing
ships before them. The loop: implement the section → `node
tests/face/harness.mjs` → the four-leg gate → the visual check vs the mockup →
commit citing the `U` ids → cut when the phase's criteria are ticked.

## Definition of done

Every criterion in the plan ticked with evidence; the three mockups still the
references; the ledgers updated (UI-DECISIONS for any new decision, the plan
closed `completed`, this packet's traps extended with what you learned); the
owner's beta bar unchanged.
