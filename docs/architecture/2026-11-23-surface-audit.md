# The surface audit — every page measured against the bar

The method (the surface pass plan, R2): per surface, every object it renders
is classified **act / drill / explain / dead / noise**; dead objects are named
(pressable-looking with no press, or a press with no effect); noise is
anything that only shows a number or a label. Evidence is a probe face
(`fleet gui --no-open --port 3995`, the same home, read-only) and the
documents it reads — never the owner's live window captured into a document.

The bar: an object must **act**, **drill down**, or **explain**. A card with
a number and nothing else is noise.

## 2026-11-23 — connections (the moved surface)

The graph moved from a permanent third column to a knowledge surface
(`8c131cac`). Inventory as measured on the live home:

| Object | Count | Job now | Class |
|---|---|---|---|
| focus node (centre) | 1 | opens the focused object's lens | act |
| ring nodes | 9 | drill the focus in place (graph re-centres, card names it) | drill |
| far nodes / "N more" | 1 | expands / collapses the rest | drill |
| legend chips | 5 | isolate a kind; press again to clear | act |
| focus card | 1 | kind, title, status, delegated processes | explain |
| card action ("open threads") | 1 | opens the kind's lens | act |
| where-line | 1 | "what this connects to · N more below" — states the picture | explain |
| hover card | 1 | names a node and its kind | explain |

**Dead object found and fixed:** the card's **"open skills" press did
nothing** — `pivotTo` read `FOCUS_LENSES` only, and skills carries a lens in
the kind table but no follow hook. Fixed in `8c131cac` (the kind's own lens
is the fallback), pinned by a skill-focus boot that must land on
`#/memory` with the intelligence panel shown.

**Noise removed:** the legend was five captions with counts; each is now a
press that isolates its kind, so the number is the reason to press rather
than the whole story. The old column's collapse chevron, reveal strip and
console switch were controls over a column that no longer exists — retired,
not left lying.

## 2026-11-23 — conversations (R3's measurement)

The owner's rule: only human-initiated or human-involved conversations.

Measured on the live home (2026-11-23):

- `/api/sessions` in the window: **99 rows**, of which **40 are
  `class: background` / `kind: scheduled`**; the whole home counts
  `background: 948`, `verification: 90`, `empty: 45`.
- The conversations list draws **12** rows ("active 12 · archived 47 ·
  all 59"), and **none** is a scheduled run; the head says "12
  conversations", which matches `counts.yours` and the drawn rows.
- The archive filter draws **47** owner rows, still **zero** scheduled.
- The connections graph's thread nodes: **16, zero scheduled**.

**Verdict:** the rule already holds on the conversations list, its archive
filter and the graph — `railRow()` (class `yours` or absent, and not a shell)
is the one predicate, applied before every state filter and the search
counts. So the system sessions the owner calls noise are **not** coming from
these surfaces. The audit's next targets, named rather than guessed: the
Home's changed-since block (events across sessions), the inbox's rows, the
PWA/bot face, and any surface that prints a count of the unfiltered
document (`background`, `verification`, `empty`).

## Pending surfaces

Home/inbox · board · intelligence (+ skills cards, the owner's example) ·
learning · desk · calibration · design · armada · second brain · workspace
history · vault · plugins · bots · settings · the drawer's four tabs. One
entry per surface, each with its inventory and its verdict, as measured.
