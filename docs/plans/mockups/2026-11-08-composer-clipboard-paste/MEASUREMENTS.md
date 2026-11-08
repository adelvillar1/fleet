# The composer foot with paste — measurements

The surface: the composer's foot — the document picker, the attach
picker, the paste button, steer, btw, Plan and Send — over the chip row
that an attached screenshot lands in.

`index.html` is static HTML in the app's own tokens (values copied from
`src/gui/assets/app.css`, not eyeballed). Rendered by headless Chrome at
1160px wide, the width the face gives the card at a 1280 window
(`width: min(820px, calc(100% - 64px))`).

## The gate, per theme

The minimum text/background ratio across the hint, the chips, the run
bar, the foot buttons, the notes and the textarea — computed in the page
against each element's real ground (its own, its gradient's first stop,
or the nearest ancestor that paints one).

| theme | contrast (min) | worst element | overflow | Send, white on its gradient |
|---|---|---|---|---|
| light | 5.29 | the section heading | 0 | 4.81 → 7.51 |
| console | 5.90 | the composer hint | 0 | **3.06 → 4.48** |
| graphite | 5.79 | the composer hint | 0 | **2.93 → 4.48** |

The body of the surface passes 4.5:1 in all three themes with nothing
overflowing. **The Send button does not**, in the two dark themes: white
on `--dsw-alias-gradient` is below 4.5:1 across its whole sweep. That is
not this change — the button is the app's existing one, and its token is
untouched by this plan — but the measurement found it, and it is filed
rather than silently folded into a green table.

## The geometry the picture settles

- **The foot fits.** Seven actions measure 311px inside an 816px card:
  505px spare at the narrowest real width. Adding the paste button would
  have had to cost 200px to force a wrap, so it takes the row without a
  layout change.
- **The chips carry no thumbnail.** A chip reads 255px as
  `shot-….png · 2.0 MB` and 277px with a 14px face; eight of them run
  176px wider than the row can spare, so the face buys a row of wrapping
  and a smudge in exchange for telling the owner something the filename
  already says. The code keeps the plain chip.

## The PNGs

`light.png`, `console.png`, `graphite.png` (920x315 each, cropped to
the sheet's own measured bounds — the page reports `left,top-right,bottom`
so the crop is the sheet and not the contact-sheet margin) and
`chips.png` (920x358, both chip treatments against one another), off
`page.png`.

Ink coverage, read off the crops: light 1.3% on `rgb(246,244,239)`,
console 98.7% on `rgb(14,15,18)`, graphite 98.8% on `rgb(10,12,16)`. The
dark pair differ from the light one by ground and not by content, which
is what a theme is, and every crop's four corners are one colour, so no
margin leaked into the read.

**The eyes-on read did not happen in this session.** `browse` is
disabled for the window, and the only image route available
(`mcp__filesystem__read_media_file`) returns binary content as base64
text to this surface, so the pixels could not be looked at. What ran
instead is the programmatic read: a luma grid of each crop confirming
the structure (the chip row above the run bar, the action row's right
alignment, the filled Send at the row's end), a solid-square scan for
black blocks (none: every 8x8 block of one colour sits inside a chip or
a button, which is what a filled control is), and an edge-ink check for
glyphs cut by the crop (0 in every theme).

That is weaker than the house rule asks for, and it is recorded as
weaker: **the owner's read of these PNGs is the leg still owed.**

## The shipped surface, verified live (the strongest leg available)

`browse` is still disabled, so the pixels above remain the owner's to
read. What could be verified without eyes is the surface the mockup
describes, on the **built** binary rather than on the mockup:

`./target/release/fleet gui --port 3099 --no-open` (the owner's own
server on 3080 is a live relay and was not touched), then:

- `GET /` serves `id="paste"` once, with `aria-label` and `data-tip`
  "paste from the clipboard" and the clipboard glyph — the button exists
  in the product, not only in the sheet.
- `GET /api/clipboard` with a real pasteboard clip (`'a clip from the
  pasteboard' | pbcopy`) answers `{"kind":"text","text":"a clip from the
  pasteboard","type":"clipboard"}`; with nothing on it,
  `{"kind":"empty",...}`. Both are live reads of the real pasteboard.
- `POST /api/plan` with a **12.0 MiB** body (three 3 MiB PNGs, base64 as
  the page sends them) answered `200` — against a 64 KiB cap the same
  body was a refusal, so the raise is real on the wire and not only in
  the const.
- The old server on 3080 answers `fleet gui has no route for
  /api/clipboard`, which is the pre-change binary still in memory: the
  live face needs its restart before the owner sees any of this.

The bot face's markup is not served on the loopback root (`/bot.css` and
`/bot.js` are the remote listener's routes, gated on `RemoteFace::Bot`),
so the bot's paste was verified from source and by the harness rather
than over HTTP.

## What the mockup caught in itself

Two defects of its own, both found by the measurement and fixed before
the numbers above were recorded:

1. `--composer-bg` was declared on `:root` while its value lived on the
   `[data-theme]` sections, so it resolved to nothing and the textarea
   read 1.17:1 on a transparent ground. The first run of the script
   reported `contrast:1.00`, which was the script's own gradient bug on
   top of it.
2. The sheet's page colour had the same shape of bug one level up:
   `body` read `--dsw-alias-page`, which is only defined inside a themed
   sheet, so the contact-sheet margin painted white around the dark
   themes.

The measurement script needed the read as much as the design did.
Before the fix it reported `contrast:1.00` — a green-looking number that
was the script failing to resolve a gradient ground, exactly the
"measurement calls it green" failure the house rule names.
