# T1 — tokens by project

The owner's ask (2026-11-23): *"token accounting per project, using the information
we already use from the cache plugin, just a different view — tokens by model per
project, a donut type visualization with total tokens and on hover over all the
information reported by the model provider."* Then, in the same thread: *"the donut
is per project, with the composition of the different models that worked across all
sessions related to the project"*, and *"a project card with the donut and either
hover over numbers for each model, or a legend below the donut."*

This pack is the gate that drawing needs before code, in the cache instrument's own
vocabulary (its sibling under `plugins/cache`): a near-black field, a mono numeral
voice, one accent per model.

## The screen

[`screens/tokens-by-project.html`](screens/tokens-by-project.html) — a grid of
project cards:

- **the card** — the project's name and path, the donut, the caption naming how many
  calls reported, and the **legend below the donut** with every model's tokens and
  share. The numbers are readable without hovering; the legend is also the touch and
  keyboard path.
- **the donut** — one per project; its slices are the models that worked across
  **every session scoped to that project** (a session is never a slice; two sessions
  on one model are one slice). The centre is the project's reported total.
- **the panel** — hover or focus a slice or a legend row: calls and how many
  reported, prompt, completion, billed, the provider's own total, cache read (with its
  share of the prompt), cache write, reasoning. A field the provider did not send
  reads **not reported**, never 0.
- **the buckets** — the calls no session can account for, each with its sentence:
  `unattributed` (rows written before routes named their session), `unbound` (the
  session has no workspace), `unknown session` (the log is gone).

The fixture is labelled as a fixture on the page: its numbers show the shape and the
honesty rules, not the live archive.

## Measured (WebKit)

| Capture | Viewport | Facts |
|---|---|---|
| `png/tokens-1440.png` | 1440×1000 | 3 cards in one row; slices 4 / 3 / 2; legend rows 4 / 3 / 3; every row ≥44px; card heads on one line; **doc 1440, no overflow** |
| `png/tokens-1440-panel.png` | 1440×1160 | the detail panel open on one model — every provider field, and `reasoning: not reported` shown as words, not zero |
| `png/tokens-820.png` | 820×1333 | cards reflow to 2 per row (2 + 1); **doc 820, no overflow**; rows still 44px |
| `png/tokens-390.png` | 390×1907 | one column; the same card, the same legend; **doc 390, no overflow** |

Measured with a WebKit evaluator over the real page (`innerWidth`, document
`scrollWidth`, distinct card tops = layout rows, slice and legend-row counts,
computed row heights).

## What the gate has already answered

1. **The donut is per project** — composed of the models that worked across all
   sessions related to the project.
2. **A project card** — the donut on it, with the per-model numbers on the legend
   below it (and hover adds the detail).
3. **The slice metric** — billed tokens (prompt + completion), the reading the cache
   instrument already uses; cache read is a subset of the prompt, never an addition.
4. **Absence is words** — a field the provider did not report says "not reported".

## What the gate still decides

1. **Card density and order at 1440 / 820 / 390** — this draws three across, two,
   one, ordered by billed tokens descending.
2. **The buckets' housing** — a footnote strip (drawn) or three cards of their own.

## On approval, the implementation is small and named

- **The data seam**: `calib::RouteRow` gains `session` (defaulted — empty on rows
  written before it and on seedless runs); `Prepared` keeps the conversation seed it
  already receives and `record_route` stamps it. The project resolves at read time
  from that session's own Start-record `workspace` — the fact the rail groups by,
  never the current working directory.
- **The document**: `cache_report` gains `projects` from the same walk it already
  does (the cache plugin's `models`/`totals` stay untouched), with the three buckets
  as their own entries and the provider's differing total carried as a flag.
- **The bundle**: `plugins/tokens/` — untrusted, `queries: ["cache"]`, group
  `system`, label `tokens`, the same static-HTML + `postMessage` pattern as its
  sibling. No new query name and no sandbox widening.
- Pins: the grouping and every bucket in unit fixtures; the row's session stamp;
  `fleet plugins eval tokens`; and the counts this screen draws (slices sum to the
  project's reported total; a session never becomes a slice).
