---
name: chrome-screenshot-fires-at-load
description: "feedback 2026-11-03 — Chrome headless --screenshot captures at the load event, before an SPA's async state read repaints, so the pre-data paint reads as a bare page (identical byte counts across attempts); --dump-dom with --timeout quotes the loaded render, and --virtual-time-budget can hang entirely on a page with a live SSE stream"
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

Capturing fleet's `#/plugins` surface headlessly (2026-11-03, diagnosing the
owner's "the plugin UI is atrocious" verdict) hit three capture traps in a
row:

1. **`--virtual-time-budget=8000` hung past the 120 s shell timeout** — the
   page holds a live `/events` SSE stream, so the load never completes and
   the budget never settles. Kill the Chrome process; the budget is useless
   on a streaming page.
2. **`--screenshot` (plain and `--headless=new`, with or without
   `--timeout=6000`) fired at the load event, before the async `/api/state`
   read repainted** — every capture came back byte-identical (89,593 bytes,
   the pre-data paint: the page head, the empty-state line, the install
   form, NO built-ins). A screenshot that repeats its own byte count across
   attempts and parameter changes is telling you it is capturing the same
   moment, not the surface.
3. **`--dump-dom --timeout=12000` quotes the loaded render** — the built-in
   rows were present in the DOM (three `second-brain` hits) while the
   screenshots showed none. The DOM dump waits into the async paint; the
   screenshot does not.

**Why:** the face paints in two phases — the shell at the load event, the
data after the state read's promise chain — and the screenshot's trigger is
the load, not the data.

**How to apply:** for a visual of a data-dependent SPA page, use
`--dump-dom --timeout=<window>` to verify the render landed first (grep for
a data-carrying marker), then screenshot through a harness that awaits the
data (the face harness's `flush()` loop pattern, or a Playwright wait on a
data-carrying selector) — never trust a bare `--screenshot` of a page whose
content arrives by fetch. Recorded in
`docs/plans/2026-11-03-plugins-ux.md`'s Research and Risks (the capture
trap is a finding, so the next reader does not mistake the pre-data paint
for the surface). See [[the-screenshot-is-the-visual-review]] and
[[a-visual-coherence-check-is-reading-the-png]].
