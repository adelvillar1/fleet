# Huddles UX — the room

Four screens and one state, drawn before any code (the mockups-first
ruling). Plan: [`docs/plans/2026-11-04-huddles-ux.md`](../../2026-11-04-huddles-ux.md).

The palette is `app.css`'s, hand-copied: the reference scale, the alias
tier and the component tokens keep their names and values, so the
implementation is a translation and not a reinvention. Every screen links
one `mockup.css` and carries no build step; the theme switcher in the
corner sets `data-theme` and writes `?theme=` so a dark screen is the same
file.

| screen | what it decides |
|---|---|
| `rooms.html` | The split: the rooms rail (active / parked / ended, phases nested), the room, its head and shelf, the transcript with a mention line and a capability gap, the composer with its `to` chip. This is the surface's default state. |
| `room.html` | The rooms rail folded to its 48px strip (the sessions strip's own geometry), so the room reads at its full measure. The `‹` returns to the list. |
| `deliverable.html` | A shelf artifact pressed: the pane opens on the file's preview, the shelf card carries the selection, the rooms stay folded. The deliverable is what a room exists to produce, so it gets a place of its own. |
| `empty.html` | No rooms yet: the rail says what a room is, the centre is the invitation — the goal, the bots with their voices, one press. |
| `seat.html` | A destructive act takes the seat — the app's **inline** seat above the composer, with sys1's verdict as data. Not a modal: the console has never used one. |
| `rooms-console.png` | The same screen in the `console` theme — proof the room rules read aliases only. |

## What it is, and is not

Slack's **model** — a room list on the left, a conversation with real
authors and times, deliverables posted in-line, a composer that says who
it is addressing — in Fleet's **paper**: New York serif room names, the
warm page, the mono for times, sizes and counts, the 820px measure, the
same 236px rail and 440px pane.

It is not Slack's skin: no purple, no avatar mosaic, no emoji reactions,
no glass, no skeleton loaders, no new token.

## How it was verified

Rendered in Chromium at **1440×968** and measured over the DevTools
protocol in all four themes (paper, console, graphite, contrast):

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --user-data-dir=/tmp/cs --screenshot=rooms.png --window-size=1440,968 \
  "file://$PWD/rooms.html"
```

The measured pass (the script lives outside the repo; it reads the live
DOM, not the CSS):

| fact | target | measured |
|---|---|---|
| horizontal overflow | 0 | 0 on all five screens, all themes |
| elements past the right edge | 0 | 0 |
| clipped text (non-ellipsis) | 0 | 0 |
| room rows | ≥44px | 72.8px |
| rail rows | the app's 34px | 34px |
| primary action (`send`, `open the room`) | ≥30px | 30–38px |
| secondary head action | the app's 26px | 26px |
| mark ink on its own disc | ≥4.5:1 | 5.7 – 7.0:1 |

The first pass failed one cell: the `clerk` mark (lane-0) measured 4.3:1
in the `contrast` theme. The ink mix moved from 82% tint to 68% tint, and
every mark now clears 5.7:1 at worst.
