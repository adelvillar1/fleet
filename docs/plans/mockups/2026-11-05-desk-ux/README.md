# The desk, as a first-class surface — mockup

`desk.html` is one navigable page covering the whole matrix: **4 states × 3 widths ×
4 themes = 48 cells**, each addressable by URL:

```
desk.html?state=<shelf|deliverable|composer|empty>&w=<1440|1024|390>&theme=<paper|console|graphite|contrast>
```

The rack at the top switches state, width and theme and writes them back to the URL,
so a review comment can name one cell exactly.

## Why this exists

The desk shipped on 2026-11-04 as a data model with a face that renders. This mockup is
the design pass for making it a surface the owner would choose to use. Every screen here
answers one measured failure of the shipped face (see `docs/plans/2026-11-05-desk-ux.md`
for the audit and the criteria that close them).

## The tokens are copied, not invented

Every colour, size, radius and shadow in `desk.html` is hand-copied from
`src/gui/assets/app.css` — the four real themes, their real values, the real type scale
and the real radii. The translation to Rust-side CSS is meant to be a translation. If a
value here is not in `app.css`, it is a bug in this mockup, not a new token.

One deliberate exception, named so it cannot hide: the mockup's rack (the dark strip at
the top) is review furniture and is not part of the product screen.

## What it demonstrates

**The three zones the shipped plan promised and the build dropped.** A project rail, the
shelf of deliverables, and a standing column answering *what the desk owes you*. The
shipped face collapsed all three into one full-width stack.

**The blocking state above the fold.** On the deliverable the refusals are the FIRST thing
under the stage rail, not the last thing under the draft. Measured: gate top is 304px at
1440 against a 700px viewport; the shipped face put it at 1001px.

**The way out always reachable.** The end band and the composer's submit row pin to the
bottom of the viewport. The shipped end button measured `top: 1252` on a 700px screen,
552px below the fold, permanently disabled, with its only diagnostic inside a tooltip.

**Colour that carries the state.** Shelf rows take a left edge — red refused, accent
running, none otherwise — and every pill is toned: `ok` green, `warn` amber, `down` red.
The shipped face had no `.chip-ok` / `.chip-warn` / `.chip-down` rules at all, so every
state chip rendered the same grey and the whole message was lost.

**Fields the API computes and the face threw away.** Coverage reads as a 5-segment meter
with its reason in words; the escalated flag is a pill; the near-miss overlap (0.41) is
quoted beside the refusal it failed to satisfy; `figures`, `created`, `claim.kind` and
`sources[].title/fetched_at` all have a home.

**Refusals that offer the way out.** Each refusal row carries its own action — *reword
it*, *add the figure* — instead of enumerating the problem and stranding the owner.

**A composer that names what it wants.** Per-source kind chips (file / fact / url), a
real source list with remove, a shape picker, and a submit row that says which fields are
still missing rather than silently mutating nothing.

## Verification

Measured in Chromium at every one of the 48 cells — never by reading the PNGs, because a
PNG cannot be measured and this session cannot see one. The probe
(`tmp/desk-probe/ink.cjs`) asserts, per cell:

| assertion | instrument | result |
|---|---|---|
| no sideways overflow, nothing past the device edge | `getBoundingClientRect` vs the device box | 48/48 clean |
| no clipped text without an ellipsis | `scrollWidth > clientWidth` | 48/48 clean |
| no interactive target under 24px | every `button/a/input/textarea/select` box | 48/48 clean |
| ink contrast meets WCAG AA | colour composited up the painted chain, then §1.4.3 | min **4.59:1** |
| the blocking state is above the fold | gate top vs `innerHeight` | 12/12 above |
| the primary action is reachable | action bottom vs `innerHeight` | 48/48 in fold |
| every cell addressable | URL → rendered state | 48/48 |

Three real defects were found by this instrument and fixed before the matrix went green,
each one a defect the eye would have passed:

1. `.srcs` used an implicit grid column, which sizes to max-content — a long source path
   pushed every row 173px past the device edge at 390px. Fixed with
   `grid-template-columns: minmax(0,1fr)`.
2. A done bead's `✓` is white on `--ok`, which is **2.36:1** in console and graphite. The
   fill is bright in the dark themes, so the glyph inverts to dark ink there.
3. `position: sticky` never engaged, because `overflow: hidden` on the device made it the
   sticky scrollport and it never scrolls. Fixed with `overflow: clip`.

A fourth was caught only because the fold assertion was added after the first green run:
the deliverable's primary action measured `bottom: 1419` against a 700px viewport — the
exact shipped defect, reproduced in the mockup. The end band now pins.

## The screenshots

`<state>-<width>-<theme>.png` for all 48 cells. They are evidence of the render, not the
instrument; the measurements above are the instrument.
