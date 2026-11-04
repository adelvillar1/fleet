---
name: an-edge-whose-source-is-retired-cannot-be-walked
description: "feedback 2026-10-18 — before designing retrieval expansion over a declared edge, check whether the substrate still considers the edge's source live; `superseded-by` looked like a perfect expansion edge and was structurally dead, because the frontmatter retires the fact that carries it"
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

Building the recall graph, I designed expansion over `superseded-by` —
"when a superseded fact matches, the fact that replaced it is what the
turn actually needs." It could never fire. `Fact::forgotten` reads a
non-empty `superseded-by` as **retired**, and the pool filter drops
forgotten facts, so the fact carrying the stamp is out of recall by
design and there is no forward source to expand from. The substrate
declared an edge and its own semantics neutralised it.

**Why:** the edge looked load-bearing in the schema and was dead in the
retriever, and nothing in either place said so. The plan asserted "the
substrate already declares three edge types and the retriever walks none
of them" — true, but one of the three was unwalkable, and only writing the
test exposed it. Related to [[name-the-component-that-has-the-capability]]:
before saying a system can do X, check whether the component that would is
actually in the path.

**How to apply:** for every edge you plan to expand over, ask what the
substrate does to the fact that *carries* it. If the carrier is retired,
hidden, or filtered, the edge is not a retrieval edge. The fix that worked
here was to invert the direction: the retired fact **vouches** for its
replacement without ever being a hit — a retired fact whose own vocabulary
matches points at its successor, and a corpus whose only match is a
folded-away fact has an EMPTY primary pool, which is exactly when the
replacement must still surface. That inversion also required removing an
early return gating the hop on a non-empty pool. See
[[fleet-recall-measured-state]] for the measurement context.
