---
name: the-pane-card-is-live-the-board-pair-was-dead
description: "feedback 2026-10-31 — my provider-form edits landed in dead code: providersCard/providerEditor had no callers (the pane's providerSections was live); plus the harness's fake DOM matches attribute selectors against element PROPERTIES (aria-label/data-act queries return nothing — use El.find property predicates, and page.el(id) reaches the pane)"
type: feedback
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**2026-10-31 (provider-models landing, two stacked traps):**

1. **The dead-definition trap, again.** app.js had both a BOARD card (`providersCard`, `card.id='settings-providers'`) and a PANE card (`providerSections`) for the same object. I grepped, found `providersCard`'s id, and wrote the key-field/catalog/reload form into it — with **zero callers**. The live surface is the row-press → pane card (`providerSections(read, provider, override)` at the `card:` closure in `settingsSections`). Caught only because the harness's `#settings-providers` query came back null. Fix: deleted the dead pair in the same edit (the existing lesson [[a-rewrite-that-leaves-the-old-definition-hoists-it-last]] names the shape — here it was a dead twin, not a stale duplicate).

2. **The fake DOM's selector rule.** The harness's `matchesSimple` resolves attribute parts (`[aria-label="..."]`, `[data-act="..."]`) against the element's **PROPERTY of that name** — so `setAttribute('aria-label', v)` (stored in `.attrs`) and `dataset.act` (stored in `.dataset`) both FAIL to match, silently (0 rows, no throw). What works: `el.name`/`el.type`/`el.value`/`el.className` property predicates via `El.find(fn)` (note: `El.find` returns an **array**, so `[0]` — one check threw `reading 'includes'` on `keyField.placeholder` because `keyField` was `[]`). The pane's contents live in the id registry, not the mounted tree, so `page.text(id)` reads them but `documentElement.querySelector` misses them: I added `page.el(id) → IDS[id]` for element-level card queries.

**How to apply:** before writing into a UI function, confirm it has a CALLER (grep the call, not just the definition); in harness checks, prefer `pane.find(one => one.name === 'button' && one.textContent.includes(...))` over any attribute selector, and fetch card elements with `page.el('dock-object-body')`.

**2026-10-31, the same trap in two more forms (the design-canvas build):** (a) my `el` registry edit for the new stage was silently LOST because the python script that added it aborted on a later assertion — the `panelDesign` entry never landed, `PANEL_EL` stayed undefined, and **55 harness cases went red with the identical error** (`PANEL_EL[key].hidden` on undefined), which reads like one systemic breakage and is actually one missing line in an `el` registry; (b) the new stage's route also vanished from `ROUTES` in the same class of edit — the surface existed in HTML, in `PANEL_EL`, in `GROUPS`, and still fell back to chat because `ROUTES` is what makes a hash *known*. Both were caught by the harness's first check, not by reading. Lesson: a scripted multi-edit into `app.js` reports success by exit code, not by content — after any such script, grep the three registries (`ROUTES`, `GROUPS`, `PANEL_EL`) plus `el` for each new key, or accept a red harness as the verifier.
