---
name: a-fixture-can-match-the-face-and-miss-the-bridge
description: "feedback 2026-11-03 — the plugins surface passed 679/0 while the live face showed nothing, because the bridge returned a nested {built_ins, plugins, proposals} wrapper where the face read three flat keys; the harness's flat fixture matched the CONSUMER's reading but not the PRODUCER's actual shape, so a producer/consumer mismatch is invisible to a case whose fixture is hand-shaped"
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

The owner reported *"the plugin page under settings is not displaying any of
the built in plugins we have"* — and the harness was green at 679/0 across 62
cases. Two layers:

1. **The real gap**: the plugins surface rendered only installed bundles; the
   built-ins (the second-brain tool plugin, the vendored lanes, the MCP
   registry's servers) never rode the state read at all, and the registry was
   genuinely empty — so the page read as bare. The fix composes
   `built_ins_value` from the **same documents the lane cards and the MCP
   card read** (`lanes_document`/`servers_document`, plus the boards' own
   `brain_home` resolution for the tool plugin), so the surface cannot
   disagree with them about state.
2. **The seam mismatch that hid the first fix**: `plugins_value` returned a
   nested `{built_ins, plugins, proposals}` wrapper object, while app.js read
   the three as FLAT state keys (`state.plugins`, `state.proposals`,
   `state.built_ins`). The face read the wrapper as one truthy object →
   `plugins.length` undefined → the empty state fired. The harness case
   passed because its `documents.state` fixture was hand-shaped FLAT — it
   matched the consumer's reading, not the producer's actual output.

**Why it matters.** A fixture is a third party between two real sides. When
it is hand-shaped, it can agree with the reader while disagreeing with the
writer, and the case stays green against a live face that shows nothing. The
earlier "3 plugins" reading was the same trap in reverse: `len()` over the
nested wrapper counted its three KEYS as three bundles.

**How to apply.** When a live surface disagrees with a green harness case,
diff the PRODUCER's actual document (`curl /api/state | python3 -c …` the
keys) against the fixture before touching the face — the mismatch is in the
shape, not the render. Fix the producer to the flat shape the consumer
reads (the state document carries `plugins`, `proposals`, `built_ins` as
three sibling keys), keep the fixture flat, and add the live-read check to
the delivery pass.

**Same trap, third time (2026-11-04, KI-425).** After the sliver fix the
owner still said *"the plugin interface still looks really bad"*. The
plugins-ux plan's C1 said "no prose paths" and its probe passed — against a
harness fixture carrying already-clean strings — while the real bridge sent
`detail: "at /Users/…/services/sys1"` for every lane and the raw program
string for every server, so rows wrapped to three lines. The FIXTURE carried
the CONSUMER's ideal text, not the PRODUCER's output. Rule strengthened: a
hand-shaped fixture for a **text** field drifts exactly as it does for a
shape — read the live document's actual strings (not just its keys) before
believing a display case, and pin the producer's wording with a unit test of
its own.

Related: [[a-harness-find-returns-an-array]] (the stub
seam family), [[a-post-helper-parses-the-body-not-a-response]] (the same
consumer/producer split, one layer earlier), [[creator-mode-plugin-host-exploration-2026-11-02]].
