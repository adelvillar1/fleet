---
name: parallel-renames-leave-dead-values
description: "when a parallel session renames or extends a vocabulary you also write, grep your own code for the old values — an attribute/CSS-driven system fails silently on a dead value rather than erroring"
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

On 2026-10-17 a parallel session landed the theme manager (`a882524`): four
themes instead of two. They took my light theme, renamed it `paper`, added
`graphite` and `contrast`, and repurposed the topbar toggle button to open a
picker rather than flip — the right call, since with four themes a blind cycle
is a guess. Their integration was clean: `themeNow()` validates against the
theme list and falls back to `console`, the head script defaults to `console`,
and the picker persists.

But the composer's `/theme` command was mine and still carried the old
two-state vocabulary: `setTheme(themeNow() === 'light' ? 'dark' : 'light')`.
Neither `light` nor `dark` is a theme name any more, so it wrote
`data-theme="dark"` — a value matching no CSS block. The app fell back to
`:root`, so the command appeared to do nothing and silently reset whichever
theme the owner had chosen. Fixed in `e05f8c3` by opening the picker.

**Why:** an attribute-driven system fails silently. A dead value is not a
compile error, not a test failure, not a log line — it is a no-op that looks
like a working feature. The failure only surfaces if someone clicks the thing,
and the gate stays green throughout.

**How to apply:** when a co-edited vocabulary is renamed or extended by another
session, grep your own code for the old values before assuming your half still
works — `grep -n "'light'\|'dark'" src/gui/assets/app.js` found this one in a
single command. The same hazard applies to any string-keyed dispatch: route
names, event types, tool names, theme names, setting keys. Related:
[[shared-file-commits-can-sweep-another-sessions-work]],
[[gui-revamp-browser-face-decision]].
