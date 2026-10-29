---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: the-right-pane
---

# Plan: the right pane — one surface, one entry, the owner's width, and a browser in it

## Context

The owner's report (2026-10-29, with the topbar screenshot): two icons
in the topbar open different versions of the right side pane —
confusing. The pane should carry preview and browser capability like
ZCode and the DeepSeek harness have, a drag component for its width,
and the whole thing needs deep research on what a right pane should be
in a harness.

What the code says the confusion is:

- **Two surfaces, one grid slot.** The dock (`aside#dock` — the run's
  process card and the selected-object card) and the pane
  (`aside#pane` — files, preview, web) both live in grid column 3, and
  `body[data-pane="open"] .dock { display: none }` makes them mutually
  exclusive. The topbar carries a toggle for each
  (`aside-toggle`, `pane-open`), so the owner picks a version of "the
  right thing" and the other icon silently swaps what lives there.
- **The pane is closer to done than the icons admit.** Three tabs —
  files (tree + file viewer), preview (the deck renders ahead of any
  file), web (an address bar whose read reduces a page to prose, no
  script). What is missing is the live render, the owner's hand on the
  width, and the run's process — which sits hidden behind the other
  icon.

## Research: what a right pane is in a harness

The research (2026-10-29, across Claude's artifacts and Claude Code's
panel, Cursor's agent panel and its in-app browser, Copilot Workspace's
Specify→Plan→Implement pipeline and Copilot agent mode's working set,
Cline/Roo's timeline with per-step checkpoints, Canvas, Devin's
worklog + tabbed workspace with a real browser and IDE, and Windsurf's
Cascade steps) holds a stable core: **process visibility, files/diffs
with accept-reject, checkpoints, a live preview, a resizable pane, and
steering**. Anthropic's own harness essay names the process a
first-class UI object; the artifact-beside-the-chat lineage analysis
names the two failure modes fleet's design already avoids (no forced
opens, no siloing). Fleet's run tab, preview, and steering match the
canon; the owner's width (P1) is table stakes everywhere.

**Fleet's two real gaps against the peers** — both capabilities exist
in fleet but are not pane-resident: a **Changes** section (fleet's diff
review measures the tree against checkpoints with per-hunk reject — it
lives behind /review, where Cursor and Copilot put it in the pane) and
a **plan card** (fleet's plan mode renders an editable plan document —
Devin and Claude Code keep the plan visible in the workspace). Both are
named follow-ups, not this slice's silent scope.

The design the plan follows:

A harness's right pane is **the work surface beside the words**. Across
the harnesses worth stealing from, the same core sections recur — the
agent's process/plan (what is it doing now), the artifacts (files, diffs,
rendered deliverables), a live render surface (the artifact as the user
will see it — HTML in a frame, a running app), and the selected object's
detail (what did that card mean). The differentiators are trust surfaces
(the raw events behind a number) and navigation depth (an address bar,
history). Resize is table stakes where the pane exists at all; the
harnesses that omit it are the ones whose pane is a fixed drawer.

For fleet, a team-of-one local harness, the pane should be exactly one
surface with:

- **the run** — the live loop (route, think, act, verify), the process
  card with its stop and steer controls; "where are we" before "what
  exists";
- **files** — the working directory, one file's code beside the tree;
- **preview** — the deliverable as the owner will see it, deck first;
- **web** — an address bar that renders the site live when the site
  allows framing, and says so honestly when it does not, with the prose
  reader as the fallback and the owner's browser as the second door;
- **the object** — the selected card's detail, rendered in the pane
  above the tabs when something is selected, exactly as the dock's
  object card behaved.

One entry, one surface, the owner's own width, and no second icon.

## Approach

### P0 — one surface, one entry

The pane gains the **run** tab: the dock's process card moves into the
pane (the DOM relocates; the ids and the painters are untouched, so the
live loop paints where it always did — just behind the pane's tab strip
now). The dock's object card moves into the pane too: an object press
opens the pane with the object card above the tab body, its close
returning; below 1180px the fixed drawer keeps that job, as today. The
dock `aside`, its grid slot, `data-aside`/`data-dock` state, and the
auto-fold logic that existed to make the dock room retire. The topbar
keeps one icon: `pane-open` becomes **the pane** (run, files, preview,
web), `aside-toggle` is removed, ⌘\ stays the keyboard verb, Escape
keeps its pane-when-empty walk.

### P1 — the owner's width

A grip on the pane's left edge: pointer-drag sets `--pane-width`
between a 300px floor and `min(720px, the viewport's own floor for the
stage)`, persists in `localStorage["fleet-pane-width"]`, applies at
boot, resets on double-click. The grip is a `role="separator"` with
arrow-key resize and `aria-valuenow`, so the keyboard owns it too.

### P2 — the browser in the web tab

The address bar gains two doors. The bridge answers
`GET /api/web/frame?url=` with a server-side framing check — fetch the
headers, read `x-frame-options` and CSP `frame-ancestors`, answer
framable or the named reason, HTML content-type only. Framable: the
pane renders the site live in a sandboxed iframe (no top-level
navigation, so the page cannot escape the pane), with a back control
over the tab's own address history. Not framable: the face names the
site's own header as the reason and offers the two doors beside it —
the prose reader (the existing `/api/web` read, one press) and open in
the owner's browser. Local deliverables stay in preview; the browser is
for the web.

## Acceptance criteria — met 2026-10-29

- **[x] P0-C1** One icon opens the pane; the run tab paints the live
  process (the same ids, relocated); an object press opens the pane
  with the object card above the body; below 1180px the drawer keeps
  that job. The dock aside, `aside-toggle`, and the
  `data-aside`/`data-dock` machinery are gone (including the rail's
  auto-fold, which existed to make the dock room).
- **[x] P0-C2** The C7 case asserts the unified pane; the harness is
  green at **344 checks across 37 cases**.
- **[x] P1-C1** The grip drags within its bounds (300px floor, the
  stage's own cap), persists, resets on double-click, and steps by
  arrow keys — pinned by harness checks, and measured in Chromium: the
  variable drives the grid at 440/720/300, the grip takes the pointer
  (`tmp/pane-width-probe.mjs`).
- **[x] P2-C1** A framable address renders live in the sandboxed frame
  (no top-level navigation); a refusal names the site's own header and
  offers the reader door and the owner's browser; the back control
  walks the tab's history. The check: `web.rs framable` (headers only,
  body never read), the bridge route, and the harness's frame/refusal
  checks.
- **[x] Close-out** The gate ran whole with exit codes read directly
  (fmt 0, clippy 0, build 0, test 0 — lib 7, doc 16, integration 149
  passed, 3 ignored). gui.md's pane section describes the one surface,
  the functional spec's rows carry `--probe` beside them. The two
  research-named gaps (P3 the Changes section, P4 the plan card) stay
  named follow-ups.

## Files to be touched

`src/gui/assets/index.html` (the pane's tabs, the grip, the topbar),
`src/gui/assets/app.js` (the tab logic, the drag, the browser, the
dock's retirement), `src/gui/assets/app.css` (the grid, the grip, the
tooltip-adjacent polish), `src/gui/server.rs` + `src/gui/bridge.rs`
(the framing check route), `tests/face/harness.mjs` (the rewritten C7,
the new checks), `docs/features/gui.md`,
`FUNCTIONAL-SPECIFICATIONS.md`, this plan.

## Out of scope

- A full driven browser (click-through automation) — the pane renders
  and reads; driving a site is the agent's own web lane's job.
- **P3 (named): the Changes section** — the review surface's hunks and
  accepts rendered as a pane section, the peers' biggest gap closed.
- **P4 (named): the plan card** — plan mode's document visible in the
  pane while a run works under it.
- The inspector drawer's redesign — it keeps its job below 1180px and
  as the raw-event surface.
- Touch-gesture resizing — the pointer and the keyboard are the
  owners.

## Verification

The harness carries every check; the Chromium layout probe
(`tmp/pane-*.mjs`, the queue probe's pattern) measures the drag bounds
in real layout; the gate runs whole with exit codes read directly.
