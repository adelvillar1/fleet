---
name: the-pane-icon-opens-the-contextual-view
description: "The pane icon (and cmd+backslash) now opens the contextual view — the inspected object, else the surface's own context — instead of dropping the owner on whichever tab they last used."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, resolved]
phrases: [the icon to show the pane should open the drawer and show the contextual pane, openContextualPane, contextualPaneTab, openPane restores face.paneTab]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling: "the icon to show the pane should open the drawer and show the contextual pane."

THE GAP. `el.paneOpen`'s handler was `if (el.pane.hidden) openPane(); else closePane();` — and `openPane()` restores `face.paneTab`, the tab the owner LAST usused. So the icon landed them wherever they were before, never on what they are looking at. The icon is the owner asking "show me what I am looking at"; a remembered tab is the wrong answer to that question.

THE FIX, two small functions in app.js:
- `contextualPaneTab()` resolves the most specific contextual view, in order: the inspected object (`face.objectTitle` set AND `tab-detail` visible → `'detail'`), then the surface's own context (`paneContextAvailable()` → `'context'`, which is the desk deliverable / lesson properties), else `''`.
- `openContextualPane()` calls `openPane()` then `showPaneTab(contextual, false)` when one resolves — `remember=false`, so opening the pane this way never overwrites the owner's saved tab. With nothing contextual open it leaves `openPane`'s remembered answer standing, and the run tab is the honest fallback.

Both the icon AND the cmd+\\ shortcut now call `openContextualPane()`, because they are one verb; `openPane()` keeps its remembered-tab behaviour for a reload (the reload-reattach preference is the owner's, and it must not be confounded with this). The button's aria-label and title became "the detail pane: what you are looking at, its files, preview and web".

A TEST LESSON worth keeping: the first version of the harness case pressed the icon straight after opening a task, and it PASSED with the feature reverted — because `closePane()` leaves `face.paneTab === 'detail'`, so the remembered branch happened to agree with the contextual one. The case only discriminates after the owner switches to another tab first: open the task → click `files` (so `face.paneTab === 'files'`) → close → press the icon. Then the remembered answer is `files` and the contextual answer is `detail`, and the reverted code fails with "detail=false files=true". Any test of a preference-override must first move the preference away from the wanted value.

Gate: 2354 Rust tests, face harness 1441 checks across 201 cases (1 new), fmt and clippy clean. Live-verified on a scratch home: the served bundle carries `openContextualPane`/`contextualPaneTab` and the new label.

STILL OPEN, and separate: the pane's tabs are not actually scoped to a project yet — the contract for that is `docs/plans/2026-11-18-the-pane-is-contextual.md` (status: draft, D1-D3 unresolved). This ruling makes the icon reach whatever context exists; it does not create the project scope.
