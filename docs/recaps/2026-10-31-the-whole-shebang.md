# The whole shebang: routing from the page, and the plan completed

**2026-10-31** · the day's fifth session. The owner's ask — *"I want the
whole shebang of design-canvas"* — was already a plan
(`docs/plans/2026-10-31-design-canvas-complete.md`, T0–T6); this session
finished it, after one correction that changed the shape of the routing.

The correction first: T0–T1 had assumed a `design` **workload** in the
router. The owner overruled it — *"design-canvas uses three models or lets
the user choose; the router should not use the models designated for
design work"*, then *"routing comes from the design page, by the fact that
it is a design task coming from that page"*. The workload was removed
whole, and the page took the decision: a **picker of the three designated
models**, a pair pin, and a **steer card** that walks the owner to the
design page whenever a frame lands anywhere else.

Then T2–T6 landed, each whole — code, pins, docs:

| Phase | What |
|---|---|
| T2 | the 62-preset library with its avoid lists; assets (store, inlining, sanitizer, panel); brands |
| T3 | the reference site: a hardened fetch, a style-fact scan, a cached board reference, a redesign verb |
| T4 | page sets: slugs, link rewriting, a coherence report, one packet with captures |
| T5 | the review loop: eight closed scores, Rust-driven dual captures, a bounded history |
| T6 | the token diff, the React component (ported codegen), the board-level selection packet |

## Commits

| Hash | What |
|---|---|
| `b46383d` | the page's models and the steer to it |
| `cb48dc4` | T2 — assets, brands, and the whole style library |
| `7432498` | T3 — the reference site, scanned into DNA |
| `4c995fa` | T4 — page sets, coherent and handed over whole |
| `6b9a939` | T5 — the review loop, closed |
| `cebb35a` | T6 — versions, the diff, the component, the selection |

## What the correction changed

- **The router has no design workload.** No preference entry, no sys1
  label, no leader exception, no surface hint — all reverted. A design run
  is pinned by the page (`pin: "provider/model"`, a pair pin that skips the
  judgment), and the designated models are the page's data
  (`canvas::DESIGN_MODELS`), resolved against the roster so an unavailable
  one is refused *before* a run starts.
- **Design work started elsewhere is routed to the page.** A `frame` or
  `review_frame` landing off the design stage raises one card naming the
  board; its press opens `#/design/<slug>`. A fan-out updates the one card.
- **The live proof**: a run started as the composer starts one
  (`pin: deepseek/deepseek-flash`) landed `frame f-48b9fac8` on
  `sample-board` with no approval seat, and its `end` record reads
  `served_model: "deepseek-flash"`.
- **The frame tool's result now names its preview route**
  (`/api/design/frame/<slug>/<id>`), because the same live run spent eight
  of twelve turns browsing for a canvas route that does not exist as one.

## The traps this session hit

- **A raw string containing `"#`** (`href="#top"`, `fill="#b4552d"`)
  closes `r#"…"#` — the fixture must be `r##"…"##`. Bit twice.
- **An intersection cannot report a gap.** `coherence`'s missing-token
  list was computed over the shared set, so it could only ever be empty;
  the pin caught it and the report is over the union now.
- **A write op answers `{type, op, wrote}`** — the test read `put["brand"]`
  and got null while the write had succeeded.
- **The relay suite's load flake (KI-385)** fired three times across the
  gate, a different test each time, every one green alone.
- **A composer row that cannot wrap pushes the pane out of the viewport** —
  668px past a 319px panel, caught by the 1440×900 render, not by any test.

## What is proven, and what is not

Proven live: the pinned design run above; the TSX export of a real frame
**transpiled under TypeScript with zero diagnostics**; the asset route
serving sniffed bytes; a 1440×900 render of the stage with every new
control, no page errors, no horizontal overflow.

Not proven: **no model run has walked a page set, a reference redesign or
the new review capture end to end** — every piece is pinned, the
composition is Rust's, and the captures are the browser lane's, but the
three runs are unspent. The Tailwind class mapping in the source's
`syntax.ts` was **not** ported (W14 asks for TSX, not Tailwind); a set's
PNGs are captured at the frame's size and a phone's, never at a custom
size.

## Files changed

- **Backend**: `canvas.rs` (assets, brands, the reference fetch and scan,
  page sets and coherence, the codegen, the diff, the packets),
  `gui/bridge.rs` + `gui/server.rs` (the routes: reference, review, asset,
  version, diff, the exports), `tools.rs` (the frame tool's `set` and
  `assets`).
- **Frontend**: `app.js` (the model and brand pickers, the reference
  field and page picks, the assets card, the set badge and packet press,
  the review card's eight scores and history, the versions diff and
  preview, the pick), `index.html`, `app.css` (the composer's wrap).
- **Data**: `design-presets.json` — the source's 62 directions, spec and
  avoid split.
- **Docs**: `docs/features/design.md` (six new sections), the plan (W1–W14
  all ticked, status complete).

## Open

- **KI-385 is closed** (2026-11-01): the "load-sensitive relay suite" was
  four product defects — the test server answering without draining the
  request body (a kernel reset under the client's header read), its poll
  holding a runtime worker with a thread sleep, `ureq`'s pooled default
  agent handing back a closed connection, and then three more the same
  sweep found: a process-global vault unlock leaking into neighbouring
  tests, a blocking embedder probe under a `OnceLock` that stalled async
  workers and cached a failure forever, and read budgets asserting
  latency where library-wide reads were being made. Filed as KI-388/389/
  390, all closed. Proven at KI-385's own bar: 8/8 module runs, 6 full
  suites sequentially, and 5 full suites **under four CPU burners**.
- **The three unspent runs**: a page set, a reference redesign, the review
  capture. The next session that has model time should spend them.
- **The Tailwind mapping** and a custom capture size are the two pieces of
  the source's export surface deliberately left out.
