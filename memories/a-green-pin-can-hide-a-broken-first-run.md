---
name: a-green-pin-can-hide-a-broken-first-run
description: "feedback 2026-10-31 — every design-canvas pin passed while the bundled sample board was refused on first use (the fixtures speak the source's camelCase, my serde types are snake_case); only the LIVE read caught it, and the fix was serde aliases plus a test that parses the bundled file itself"
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

**2026-10-31 (design-canvas landing):** `cargo test` was green — document round-trip, caps, injection, scan, tool handlers, the harness case — and the canvas would still have opened **empty on the first run of every new home**. The bundled sample board is copied from the source project's fixtures, which spell `tweakSpec`/`tweakValues`/`cssVar`/`type`, while my serde types are snake_case and (then) had no aliases: `GET /api/design` answered `"the bundled sample board is not a document: missing field kind"`. Every fixture in my tests was hand-written in MY spelling, so nothing tested the one file the product actually ships.

**Why the pins missed it:** each test asserted a claim I controlled — the code path with a fixture I wrote. The shipped asset was the one input no test read, and the failure lives on the *first* paint a new home ever draws, which no unit test exercises.

**How to apply:**
- When a feature ships a **bundled asset** (a sample, a seed, a default config), pin the asset itself: parse it, adopt it, assert on the result — `the_bundled_sample_board_parses` now does exactly that. A test that never reads the shipped bytes does not cover the shipped bytes.
- **Accept both dialects at the boundary.** A document ported from another app arrives in that app's spelling; `#[serde(alias = "cssVar")]`/`alias = "type"` beside the snake_case names lets one reader take both, which is cheaper and safer than rewriting a fixture (and keeps hand-written and generated frames interoperable).
- **The live read is the acceptance gate for a first run**, not the suite: probe the endpoint the surface calls (`curl /api/design`) on a fresh home and read the refusal text. A named error is how you find what the tests couldn't.

The same session's render found the companion defect: the board opened at a default transform with its second frame half off-viewport, so a board seen for the first time now fits its frames on first paint ([[the-screenshot-is-the-visual-review]]). Related: [[a-fixture-that-never-rules-is-invisible]] (a fixture that never exercises the real path is silent), [[a-new-commands-first-live-run-is-the-only-test]].

**Third form, same day, one layer up: a green pin can hide a dead FOLLOW-UP.** Building the board's conversation, the harness asserted the POST bodies and stayed green while `post()`'s `{status, body}` contract made every reply handler throw — the composed task never reached `/api/run`, so the run was never started ([[a-post-helper-parses-the-body-not-a-response]]). Same cure, wider: **pin the next step, not just the request** — a check on the `/api/run` body (or the notice, or the re-read) is what reddens when a reply path dies. The general rule both forms share: a test proves only the half it looks at, so for every feature ask "which half does my pin not see?" — the shipped bytes, or the reply that drives the next call.
