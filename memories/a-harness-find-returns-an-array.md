---
name: a-harness-find-returns-an-array
description: "feedback 2026-11-02/03 — the face harness's El.find returns an ARRAY (index [0]); its selector engine does no attribute selectors; /api/state is served from documents.state, not options.reply; a settings-POST reply is staged at opts.settingsReply (NOT inside documents); test functions must `const`-declare page (module strict mode); and the stub El has no innerText — six stub seams that make a correct face read as broken"
type: feedback
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

The plugins-surface harness case failed three ways that were all test-shape,
not product: `board.find(predicate)` returned an array and the pin read
`.dataset` off the array (`ctor=Array`); `page.rows('[data-role="proposal"]')`
found nothing because the stub's selector engine speaks classes and
`[name="value"]`, never a bare `[name]`; and a `boot({ reply: { plugins: … } })`
fixture left the face's plugin list empty because the harness serves
`/api/state` from `documents.state` (falling back to `emptyState`), never from
`reply`.

**Why:** the harness is a vm sandbox over the real app.js with a hand-rolled
DOM shim — `El.find` mirrors `querySelectorAll`'s plural, and the fixture's
reply plumbing is per-route (`documents.state`, `documents.sessions`, …). A
check written against the browser's API reads as a product defect.

**How to apply:**
- `find` → index `[0]`; `rows`/`all`/`querySelectorAll` are plural too.
- Attribute selectors are untestable — write `board.find((el) => el.dataset &&
  el.dataset.role === 'x')` and assert on the subtree.
- The face's state comes from `documents.state` in `boot(options)`; a panel's
  own read (`/api/sessions`, `/api/router`, …) comes from `documents.<name>`.
  A fixture that puts the data in `reply` feeds nothing.
- `setAttribute` DOES exist on the Els, but the shim's `dataset` is the seam
  the face actually uses (a Proxy mirroring `data-*`), so a face that sets
  `el.dataset.sandbox` is observable and one that sets `setAttribute` is not.
- When a paint runs before its data lands, the repaint after the read is the
  one to pin — and the rail's dynamic rows need an explicit rebuild when the
  list changes.

**Extended 2026-11-03 (the composer-actions/provider-connect cases) — four
more seams, all test-shape:**
- A staged **settings-POST reply is staged at `opts.settingsReply`** (a
  function of the POST body or a fixed shape), never inside `documents` —
  nesting it there feeds nothing, and the card's answer reads the default
  empty reply while looking like a product failure.
- Harness test functions must **`const`-declare `page`** (`const page =
  boot({...})`) — the module is strict mode, so a bare `page = boot(...)`
  throws ReferenceError and the whole case reads as thrown.
- The stub El has **no `innerText`** — assert on `textContent`.
- The El has **`find` but no `filter`** — narrow with a predicate inside
  `find`, or `page.rows(sel)` and index.

**Extended 2026-11-03 (the plugins-ux case rewrite) — four more seams:**
- A mid-check repaint DETACHES the elements a case already found: after a
  press that triggers `paintPlugins()` (or any paint), re-find through the
  board — the pre-repaint `door` reference found nothing and the check
  read `refusals=0` on a detached subtree.
- A press that navigates to a SETTINGS page needs a settings document in
  `documents.settings` (the minimal shape: scalars/lists/rules/env/cron/
  soul/router/catalog/lanes/servers/roles/vault/tools) — the empty default
  reply makes `paintSettings` throw `read.scalars.length` as an UNHANDLED
  REJECTION, which fails nothing but pollutes the run's tail.
- The page helper exposes no `location` — assert navigation on the PANEL
  state (`page.IDS['panel-settings'].hidden`), not the hash.
- The rail's count badge (`data-count="plugins"`) is filled by
  `paintRailCounts()`, which is not part of the repaint — the case must
  not assume the badge moved until the state landing re-runs it (and the
  span is found by `dataset.count`, one more find-returns-array site).

Related: [[a-flat-dom-cannot-prove-structure]] (the shim's shape),
[[gui-client-verified-by-a-vm-harness]] (the harness's own record),
[[creator-mode-plugin-host-exploration-2026-11-02]] (the case that hit all three).
