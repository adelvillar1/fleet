# The huddles mockups

Four static screens for the huddle feature, drawn in the face's own design
language (`mockup.css` shares the plugins-ux palette and the bot-pwa persona
mark). They are the review surface before any code: read them as a room you
would actually work in, and mark what is wrong.

| screen | what it decides |
|---|---|
| [`rail.html`](rail.html) | the rail: three states, member marks, deliverable counts, one new-huddle press |
| [`huddle.html`](huddle.html) | the room: head with goal and voices, the transcript with mentions and an in-line deliverable, the recommendation card, the composer, the pane |
| [`seat.html`](seat.html) | the seat: sys1's destructive verdict rendered as data, the controls, the mode's limit |
| [`breakout.html`](breakout.html) | the tree: child huddles, what each inherits, how they roll up |

## The decisions the screens make

**A huddle is a room, not another chat.** The rail lists rooms in three
states — active (a run or a recent write), inactive (parked), ended
(closed by you, its deliverables intact). The words are the sessions
surface's own vocabulary; nothing invents a fourth state machine.

**Every member sees every message, so the transcript is the shared object.**
Bylines carry the persona, `@name` renders inline, and a review request
between two bots reads like any other message — no owner relay. `@you` is
the exception the room makes: it asks, and starts no run.

**Deliverables are posts, not attachments in a drawer.** `send_file`
already produces an artifact card; the room gives it a position in the
transcript and a press into the right pane's preview. The rail's count is
the number of those cards.

**The capability gap is a suggestion, not a self-convene.** A bot that
sees the brief needs a role the room lacks posts a dashed card naming the
role and the reason. You press *convene*; the bot does not add itself.
This is where auto-convening would go later, and the plan keeps it out.

**The seat carries sys1's judgment as data.** The destructive class is
judged per action, not per member or per huddle, so the same member can
read a file under a mode and be held for an overwrite. The verdict names
its lane, its latency and its confidence beside the reason, and the
contrast row makes the judgment legible rather than asserted.

**Identity stays out of the transcript.** The mark appears in the rail
rows, the member stack and the persona card; no message carries a
portrait. An owner-supplied image replaces the derived initial in those
places and nowhere else — the bot-pwa ruling's reasons (roles are
user-authored, a face misrepresents the contract) hold, while the owner
may still name a bot `bob` and give it a face.

**The SOUL is named in the chrome.** The mode bar carries
`guardrail · your SOUL`, so the room says which rails every member speaks
within. The voice lines in the member stack are the role descriptions,
one line each.

## Verification

Rendered in Chromium and measured in the page itself: each screen reports
no element past the viewport's right edge, no clipped text, and no row or
primary action under 44 px. The persona marks' ink was measured against
its own opaque disc — every one clears AA with margin:

| persona | ink on disc |
|---|---|
| designer | 9.21 |
| scholar | 7.81 |
| presenter | 9.03 |
| clerk | 8.90 |
| advocate | 7.20 |
| auditor | 7.31 |

The three state words read without colour alone: each row carries its dot
*and* its word (`active`, `idle`, `ended`), and the seat's verdict carries
a stamp and a reason.

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --screenshot=rail.png --window-size=1440,968 "file://$PWD/rail.html"
```

Each capture's window is sized to the page's measured content height, so
`rail.png`, `huddle.png`, `seat.png` and `breakout.png` carry the whole
screen rather than the fold.

## Mapping to the plan

C1 is this directory. The screens fix the shape C2–C12 build: the rail's
three sections (C7), the room's parts (C8), the mention kinds and the
recommendation (C3, C5), the in-line deliverable and the pane (C6), the
child huddle's inheritance (C9), the seat's verdict (C10), the mark's
placement (C11) and the guardrail's place (C12).
