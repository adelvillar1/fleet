# D3 — Routing is Reviews' first section

The redesign's own D3 says it: *"Routing is Reviews' first section (the U44
amendment's own words)."* The surface pass measured why it is not just a
sentence: **the standalone Router page duplicates a section that already
exists** — calibration's routing section renders the router's chain card
(8 rows live), and the two pages read the same read.

This is the mockup the decision needs before code. One screen, drawn in the
design system's own tokens (the redesign's `system.css`, the map study's
shell), measured in WebKit, both themes.

## The screen

[`screens/routing-in-reviews.html`](screens/routing-in-reviews.html) — the
Reviews surface with **routing first**:

- **the section rail** — routing (current) · the gate · routes · turns · eval
  runs · recall hits · tool screens · embed timings · footprint checks ·
  shapes · validations. Every count in the rail already presses.
- **the strip** — `15 calls today · 15 ok · 0 errors · 100% success · 0
  tokens · sys1 judge`, each a **door** (dotted rule, the caption says so)
  onto the section that holds its population.
- **the live decision** — the run's own routed event.
- **the declared chain** — quick / standard_code / frontend as **rows inside
  one card** (not cards in a card), the candidates as pressable chips.
- **cost × quality** and **the workload mix** — the day's evidence, side by
  side.
- **today's route rows** — the day's stream, newest first.
- **the note** — what retires and what stays, in plain words.

What moved in, whole: the live decision, the declared chain, cost × quality,
the workload mix, the route rows. What stayed in Reviews: the gate, the
scorecard, the counterfactual replay, and every op section — untouched.

## Measured

| Capture | Viewport | Facts |
|---|---|---|
| `png/routing-in-reviews-night-1440.png` | 1440×1400, night | rail 11 sections (routing current), 4 doors, 3 chain rows, 7 cards, **no horizontal overflow** |
| `png/routing-in-reviews-day-1440.png` | 1440×1400, day | same composition, day tokens |
| `png/routing-in-reviews-night-820.png` | 820×1180, night | the map rail yields to the section rail; the composition holds, **no overflow** |

## What the gate decides (not guessed)

1. **The opening section.** Does Reviews open on **routing** every time (the
   mock), or does the rail remember the last section with routing merely
   first in order? *Recommend: routing first AND the default — a fold whose
   landed page is the new section.*
2. **The address.** `#/router` resolves to `#/calibration/routing`.
   *Recommend: a silent replace, the house's own fold style — a notice for a
   moved address is the kind of transient the U13 rulings keep out.*
3. **The day instrument.** The old page's "the stream" controls (day before /
   after, judge) merge into this surface's instrument, as drawn. *Confirm.*
4. **The palette and the folded map.** Routing leaves `advanced` in the
   palette's labels and keeps its plain name as a door onto the section.
   *Confirm the wording: "Routing · a section of Reviews".*
5. **The head.** KNOWLEDGE / Reviews is the page; routing is the section.
   The eyebrow names the region, the title the lens — the surface pass's own
   rule (R7). *Confirm.*

## On approval, the implementation is small and named

- `#/router` folds to `#/calibration/routing` (the redirect); the Router
  panel, its head and its strip retire.
- The routing section takes the moved cards (live decision, chain, cost ×
  quality, workload mix, route rows) — the builders exist; the routing
  section already renders the chain card.
- Pins: the redirect, the section's five moved objects, the rail's order
  (routing first), the palette's door, the crumb/eyebrow, and the standalone
  page's absence.
