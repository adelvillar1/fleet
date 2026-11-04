---
name: verify-crypto-against-published-vectors
description: "hand-rolled crypto passes its own self-checks and still fails — GCM's tag is S XOR AES(J0), not AES(S XOR J0), and the two agree whenever GHASH is zero, so an empty-input test cannot tell them apart"
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

Writing AES-256-GCM and PBKDF2 by hand (no new crate, per fleet's crate
contract) produced a bug that survived every check I had written, and the
way out generalizes past crypto.

**The bug.** I implemented the GCM authentication tag as
`AES_K(S ⊕ J0)`. The spec's `T = MSB_t(GCTR_K(inc32(J0), S))` reduces, for a
single 16-byte GHASH block, to `S ⊕ AES_K(J0)` — the mask is the *encrypted*
counter block, not the counter block encrypting the GHASH. **The two formulas
agree whenever GHASH is zero**, which is exactly the empty-plaintext,
empty-AAD case — so my "empty input round-trips" test passed, my field-axiom
tests passed, and the implementation was wrong for every real secret.

**Why it took so long to find, and the three traps:**
1. **A remembered test vector is not a test vector.** I asserted GCM against
   NIST "test case 2" values (`0388dace…`) from memory. Those are the
   **AES-128** values — a 16-byte key. My code is AES-256. I spent several
   rounds convinced the ciphertext was right and the tag wrong, when the
   vector was simply for a different cipher.
2. **Self-consistency is not correctness.** I "verified" my GF(2^128)
   multiplication against a value I also recalled from memory, and it
   matched — because both came from the same wrong recollection. I then
   verified it against a second independent implementation I wrote myself,
   which agreed with the first. Two implementations of the same
   misunderstanding agree perfectly. The axioms I tested (powers of x,
   distributivity) only probe a 1-dimensional subspace and pass regardless.
3. **An oracle beats a vector.** What actually settled it was deriving the
   answer from a *reference implementation* (Python's `cryptography`) rather
   than from anything in my head: encrypt the degenerate case where GHASH is
   provably zero, observe which formula the tag matches, and only then trust
   the structure. An empty-input check is the one case that cannot
   discriminate two formulas that agree on empty input.

**How to apply:** when writing or reviewing crypto (or any arithmetic with a
degenerate case), test the *non-degenerate* case first, and get the expected
values from a tool you did not write rather than from memory. If two of your
own implementations agree, that is one data point, not two. And when a
published vector "fails", check which cipher and which key length it is for
before suspecting your code. In fleet this lives as
`vault::tests::gcm_matches_a_reference_implementation`, which carries the
lesson in its doc comment so the next reader does not re-derive it. Related:
[[fleet-vault-credentials]].
