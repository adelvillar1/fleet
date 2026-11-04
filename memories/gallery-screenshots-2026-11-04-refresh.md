---
name: gallery-screenshots-2026-11-04-refresh
description: "the 2026-11-04 doc refresh — a reproducible scratch-home fixture for the screenshots, the four traps in shooting a live SPA, and the two face defects the shoot found"
type: project
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

**2026-11-04, branch `face-plugin-host`, pushed `8a009f7`.** The owner asked
for the documentation and screenshots refreshed, the new **Telemetry**
bundle featured, and the **design surface's design-canvas lineage** named.
Delivered: README (the lineage in the design bullet, the plugin host + the
telemetry bundle in "What ships today", a 16-tile captioned grid), 16
re-shot surfaces + 4 new shots (a huddle room, the rooms column, the
plugins host, the telemetry mount, the folded rail), `docs/features/
{design,gui,huddles}.md`, `docs/architecture/surfaces.md` (Plugins + the
bundle mount), both specs.

**The fixture is a script, not a copy.** `/tmp/gallery-seed.py` writes a
self-contained home: sessions as `Record` jsonl (a conversation, a room
with a mention/deliverable/recommend, an ended+archived room with its
breakout child, a capped run), memories with frontmatter, six canonical
skills copied (the only thing taken from the machine), `roles/*.md`,
`canvas/` boards, `plugins/telemetry` + `plugins.json`, calibration rows as
`{"op":"route"|"turn"|"tool_screen",…}` under `<home>/logs/<Y>/<M>/<D>/`,
and `graph.json` in the boards' `GraphFile` shape (**status is
pending|running|verified|failed**, nodes need `tools/skills/workflows/
loops` and `peak`). Nothing of the owner's own home is read except the
public skills; the one surface that does show machine data (second brain)
shows exactly what the already-published shot showed.

**Four traps shooting a live SPA over CDP** (`/tmp/gallery-shoot.mjs`):
- A `?bust=` before the hash strands the router — the face reads the hash
  only; navigate `about:blank` → `/` → `/#/route`.
- `localStorage` written after boot does nothing for the rail's fold (the
  rail is built once): set it, `Page.reload`, then address.
- Readiness must be polled **on the panel the shot is about**
  (`#panel-<route>`): `main.stage:not([hidden])` can be the previous
  surface, which is stable and long enough to pass any generic predicate.
  Design loads in ~7 s, telemetry paints at ~6 s, the router at ~2 s.
- Transcript prose is one line per paragraph: hard-wrapped fixture text
  renders as broken paragraphs.

**Two face defects the shoot found** (both real, both shipped before this):
- **KI-422** — a document's refusal is its entries, never the key:
  `refused: []` is truthy, so `if (doc.refused)` read every *good* read as
  a refusal. The router board sat on its loading card forever ("The board's
  read has not landed yet" against a 200 with 15 keys) and the folder
  browser refused to list a directory it had just read. Fixed with one
  `refusals(doc)` predicate across four readers + a first-read refusal that
  names itself; the harness had **never stubbed `/api/router`**, which is
  why the board's read went untested — the stub and the case now exist
  (`KI-422 an empty refused list is not a refusal`). The same surface
  printed the served rate as **10000%** (the bridge's `success_rate` is
  already a percentage).
- **KI-424** — the plugins board is the 12-column `.board` grid and nothing
  spanned it, so the surface rendered as a dozen 100px slivers.

**The composed live leg names its machine fact.** `tests/composed_live.rs`
started failing with no code change: the sys1 verdict said `composed`, but
`BLEND1_BEARER_TOKEN` is unkeyed on this machine (sealed/absent), so blend1
never enters the walk and the order began at stepfun. The leg now skips
loudly naming the unkeyed credential, and still asserts blend1-first when a
key resolves — see [[a-load-sensitive-test-is-a-defect-in-the-code]].

Open at the close: the branch is pushed but **not merged to master**
(origin/master is `ef591be`; the branch carries `e1b4dc9` from the
co-tenant plus `8a009f7`).

**KI-425 — the row was still a machine dump.** The plan's C1 said "no
prose paths" and its probe passed, but the *bridge* sent
`detail: "at /Users/…/services/sys1"` for lanes and the raw program string
for servers, and the face rendered it verbatim (`overflow-wrap: anywhere`)
— rows wrapped to three lines and read as noise. The harness fixture
carried already-clean strings, so the surface was pinned against a document
the producer never sent: [[a-fixture-can-match-the-face-and-miss-the-bridge]]
again. Fixed at the producer (a state word + one human phrase; the path
and the command line live on the card the row opens), one-line clamp in
the CSS, pinned by a Rust test over synthetic lane documents and a harness
no-path check.

**Follow-up push `b28d04b`** (the KI-425 row fix, after the owner's report):
`src/gui/bridge.rs` (`built_ins_value` + the `lane_row_state`/`lane_phrase`
helpers), `src/gui/assets/app.css` (one-line clamp on `.plugins-row-state`,
`min-height: 44px` on `.plugins-row`), the harness fixture rewritten to the
bridge's real document shape (`{state, detail}`), `docs/features/plugins.md`
§ "A row's words", and `docs/images/surface-plugins.jpg` re-shot from the
scratch home.

**Useful facts from this round:**
- The deployed console lives at **127.0.0.1:3080** (the LaunchAgent's port),
  not 3091 — the scratch gallery server runs at 3099. Capture the owner's
  live surface with `BASE=http://127.0.0.1:3080` and the same CDP script.
- The desktop app is a **stale artifact** after a face change: the old
  `fleet.app` only updates on `cargo tauri build` (output lands in
  `target/release/bundle/dmg/`, and the packaged `fleet-desktop` binary can
  be grepped with `strings` from the mounted dmg to prove it carries the
  fix). If the owner says a surface "still looks bad", ask whether he is in
  the app or the browser before hunting a code defect.
- `cargo test <name>` with `--lib` ran **0 tests** for a `#[test]` in the
  bin's test module — run `cargo test <name>` with **no target flag** to see
  it (it printed `running 1 test` for `src/lib.rs`).
- Verify a CSS layout fix twice before filing it: `.plugins-row` boxes were
  already 1138×44 before the `min-height`, so the "slivers" defect was real
  in the build that shipped the docs, and the 44px rule was belt-and-braces,
  not the fix.
- A `#[test]` that reds only while Chrome plus a second fleet server are
  running (`the_probe_retries_after_a_pause`) is the KI-405 load-sensitive
  class — re-run on a quiet machine and say so, never silently.

Open at the close: the branch is pushed (`b28d04b`, 3 commits ahead) but
**not merged to master** (origin/master is `ef591be`; the branch carries
`e1b4dc9` from the co-tenant, `8a009f7` and `b28d04b` mine).
