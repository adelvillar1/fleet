---
name: a-one-item-fixture-let-the-switch-check-pass
description: "feedback 2026-10-31 — the design rail's "pressing a row opens that board" check passed comfortably against a ONE-board fixture (the only assertion it could make was "some read happened"); a switch needs two items, a stub that answers per key, and a check on the key the press sent"
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

**2026-10-31 (the design canvas's boards rail pin).** The case asserted
opening a board "is a read by slug" — against a fixture holding **one**
board, so the honest thing it could check was `reads.length >= 1`. That
passes whether the rail switches anything or not, and it passed after the
rail had been fully rewritten, which made it worthless as a guard on the
rewrite.

**Fixed by making the fixture able to fail:** two boards
(`sample-board`, `another-board`), the design stub answering **per the
requested `project=` slug** (which is also what the bridge does), and
three checks — two rows with only the open one marked, a press that the
`calls` ledger shows as `project=another-board`, and `localStorage` now
holding `another-board`. Harness went 545 → **546 checks / 55 cases**.

Two mistakes made while repairing it, both worth remembering:

- **Order matters when a gesture changes which document is loaded.** The
  switch block originally sat before the frame/selection assertions and
  reset the board out from under them (the selection check read
  `canvas-frame` where two frames had been). A test that switches the
  loaded document runs **last**.
- **The stub read `opts.url`.** Its parameter is `url` and it logs
  `calls.push({url, ...})`; `opts.url` is undefined, so the per-slug
  answer silently fell back to the base document and the reminder check
  failed with the old slug. When a stubbed response "ignores" what you
  think you sent, read the stub's own signature and its `calls` ledger —
  that ledger is also the cheapest way to assert the key a press sent.

**How to apply:** a check on a *switch* is only a check if the fixture
holds at least two things to switch between and the response can differ
by the key the press sent. If the only available assertion is "a request
happened", the fixture is too thin — widen it before trusting the green.
See [[a-fixture-that-never-rules-is-invisible]] and
[[a-fixture-corpus-flatters-a-retriever]].
