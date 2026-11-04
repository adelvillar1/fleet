---
name: a-fixture-can-pin-the-bytes-and-encode-the-wrong-shape
description: "feedback 2026-10-26 — a stub fixture captured byte-for-byte from the real system still launders a wrong interpretation of it to green; the DERIVATION over the captured data must be re-derived from the live answer too, and a bound must select the rows you meant, not the tail you got"
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

The LayerFS face's stub fixture was the captured bytes of a real Store —
18,777,528 logical, 11,229,684 stored, dedupe 1.25553 — and the test over
it was green. The live run showed the interpretation was wrong in two
places. The merge state was derived from main's commits, but the real
Store's main carries no merge commit at all (its head stays null while
the layers chain), so the evidence is the layers' `sourceBranch`; the
DAG drew main's spine from commits when main's line IS its layers. And
the child-branch join read the last 512 rows of the day's whole
calibration stream, which on a busy mixed-op day contains no delegation
row at all — the face lost its join while the router was busy, and the
code's own doc comment said so.

**Why:** pinning captured bytes pins the INPUT. It says nothing about the
shape you read out of it, and a fixture captured once freezes the shape
as it was understood that day. The suite then defends the
misunderstanding as strongly as it defends the bytes — this is
[[audit-claims-pinned-against-captured-bytes]]'s rule applied to the
wrong half: the capture was honest and the reading was a guess. The
second failure is the same error at a different level: a bound that
selects "the last N rows" instead of "the last N rows OF WHAT I MEANT"
is correct arithmetic answering the wrong question, and it degrades
exactly when the system is busiest.

**How to apply:** after capturing a fixture, re-derive every computation
over it from one live answer before calling the test done — the live run
is the only thing that tells you the fixture's shape was your
assumption. When a bound reads a shared stream, bound the OP (the
op-bounded reader), not the stream, and say in a comment which one the
caller meant. And when a live read disagrees with a green stub, the stub
is what changes — including its fixture.
