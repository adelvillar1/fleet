---
name: an-icon-discipline-is-guarded-by-rendered-rows-not-named-keys
description: "The icons discipline degrades silently because its pin asks \"is every named key drawn?\" (dot satisfies that) rather than \"does every rendered row get its own mark?\" — and duplicate or nonsensical drawings are only found by rasterising each mark and reading it, not by counting."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [MEMORY_SECTION_ICONS fallback dot, every rendered rail row gets its own drawing, CALIBRATION_OPS shared verify, rasterise the path to a 17x17 grid and read it, tools was a chevron not a wrench, key regex needs the underscore for route_judged]
missing: []
premises: []
recall-count: 16
superseded-by:
valid-until:
source:
---

The 2026-11-04 icons discipline (`docs/plans/2026-11-04-icons.md`) has two halves, and only the first was ever pinned.

**The rule.** One meaning per mark: every key drawn, `icon()`'s fallback a visible `unknown`, the same concept sharing one drawing deliberately, and the marks redrawn for 17px with a sheet rendered and *read* before the code is touched.

**Why it degraded (KI-569, found 2026-11-17).** The pin `everyIconKeyTheFaceAsksForIsDrawn` asks *"is every key the face names drawn?"* — and `dot` IS drawn, so three rows silently falling through `MEMORY_SECTION_ICONS[id] || 'dot'` satisfies it. The discipline's real question is *"does every rendered row get its own meaning?"* That gap is the whole failure:
- Intelligence folded skills/tools/workflows into the memory rail (U40); the map was never extended → three rows, one hollow bullet.
- The plan specified a `route_judged` drawing ("a route with a check") that was never drawn, so the calibration map substituted `verify` — three ops wearing one check.
- `tools` was drawn as `M5 7l5 5-5 5` (a next-arrow) plus a rule; at 17px a row's mark read as the rail's own collapse control.

**The method that finds these.** Do not count drawings — **rasterise and read them.** A 17×17 grid render of each path made both bad drawings obvious in seconds (`tools` as a broken fragment, `route_judged`'s check running past the 24-unit box). The original work measured *legibility*; nobody read the *shapes*. The sheet in `docs/plans/mockups/2026-11-04-icons/` was the discipline's own instrument and it had not been re-rendered since.

**The repair.** Extend the map for every segment the rail can render; give each op its own key; draw the marks the plan specified; and pin the real question — `everyRenderedRailRowGetsItsOwnDrawing` (no two rendered rows share a drawing, none carries none) and `everyCalibrationOpNamesItsOwnDrawing` (no two ops share a mark, every op names a drawn one).

**A trap inside the fix:** the old pin's key regex had no underscore (`[a-z][a-z-]*`), so `route_judged` — both an op name and a drawing key — was skipped on *both* sides. The check written to guard that exact key reported it as "absent" while never looking at it. When a regex enumerates identifiers in this codebase, the character class must include `_`.
