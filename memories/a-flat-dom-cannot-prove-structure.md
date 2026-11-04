---
name: a-flat-dom-cannot-prove-structure
description: "feedback 2026-11-01, extended 2026-11-02 — fleet's face harness builds a FLAT id-map (one El per id=, no tree, no bubbling), so parentage, geometry and event propagation are untestable; and its shim must honour the DOM seam the face actually uses (dataset mirrored into data-*, bare [name] selectors) or a correct face reads as broken"
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

The design-parity build's C1 check first asserted the floating
composer's parentage (`float.parentNode === page.IDS['design-viewport']`)
and a bubbled background deselect (`world.fire('pointerdown')` reaching
the viewport listener). Both failed forever: the harness's boot builds
`PAGE_IDS.forEach(id => IDS[id] = new El('div'))` — every element is a
detached single, `parentNode` is null, `children` is empty unless JS
appends into it, and `fire()` walks `parentNode` which goes nowhere.
The checks that DID work drove the listener with an explicit target and
asserted the pieces + collapsed state, leaving geometry to a real
Playwright render.

**2026-11-02 — the second half of the lesson.** The live-turn rewind pin
(`.row-rewind[data-live]`) failed while the row sat in the DOM: the shim's
`.dataset` was a plain object, so `el.dataset.live = 'true'` never set the
attribute a `[data-live]` selector matches — and the selector engine only
understood `[name="value"]`, never a bare `[name]`. **A shim that does not
mirror the seam the face uses makes a correct face read as broken**, and
the failure message points at the product. Fixed by making `dataset` a
Proxy that writes through to `attrs['data-<key>']` (delete mirrored too,
and the object must never be reassigned — a `this.attrs = {}` later in the
constructor orphans the mirror) and by teaching `matchesSimple` the bare
`[name]` form.

**Why:** the harness is a vm sandbox over the real app.js with a
hand-rolled DOM shim — it proves behavior (wires, writes, state
transitions, POST bodies), never layout or document structure. Asking it
for structure produces a check that cannot pass, and the failure reads
like a product defect when it is a test-shape mismatch — two rounds went
into debugging app code that was fine.

**How to apply:** before writing a harness check, ask what the shim can
observe: ids, textContent, `dataset` (now attribute-backed), classList,
value, hidden, attributes, listeners fired directly, `find` predicates.
Anything needing parentage, bounding boxes, styles, bubbling or real
events belongs to the render leg (Playwright at the owner's viewport,
then READ the PNG). When a proxy/alias is added to the shim, keep the
underlying object's identity stable for the shim's lifetime.

**2026-11-02 — three more seams, all in the plugins case** (see
[[a-harness-find-returns-an-array]]): `El.find` returns an **array** (index
`[0]`); the selector engine does **no attribute selectors** —
`page.rows('[data-role="x"]')` finds nothing, so write
`board.find((el) => el.dataset && el.dataset.role === 'x')`; and
`/api/state` is served from `documents.state` in `boot(options)`, **not**
`options.reply` — a `reply:` fixture silently starves the face. `setAttribute`
exists on the Els, but the shim's `dataset` is the seam the face actually
uses, so a face that sets `el.dataset.sandbox` is observable and one that
calls `setAttribute` is not.

Related:
[[gui-client-verified-by-a-vm-harness]],
[[the-screenshot-is-the-visual-review]],
[[a-card-builder-returns-sections-not-a-node]],
[[a-harness-find-returns-an-array]].
