---
name: name-failed-legs-never-drop-them
description: "in a fan-out pipeline a failed leg must be NAMED, not dropped to an empty string — `except Exception: return ""` lets silence flow downstream as content and produces confidently wrong output; the fix is per-leg status, not error-handling middleware"
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

The owner's TinyFish pipeline sketch (2026-10-17) handled per-source failure
with:

```python
except Exception:
    return ""   # "Gracefully handle individual public site timeouts"
```

then filtered with `[text for text in results if text]` and concatenated the
survivors into `combined_text` for clustering and synthesis.

**Why that is the opposite of graceful.** The empty string is
indistinguishable from a source that legitimately had nothing to say, so it
flows through clustering, ranking and synthesis exactly like a real source. The
result is a deck comparing the product against a competitor's pricing, sourced
from nothing, with no signal that any fetch failed. The failure is not a crash
— it is a *confidently wrong* artifact, which is worse, because it looks
finished.

This is the same shape as a failure mode already recorded in this project:
config that fails validation and silently degrades to empty reads as "no facts"
rather than "the read broke" (see `zcode-provider-config-degrades-on-validation-failure`).
An empty value is a claim, and an unlabelled one is a false claim.

**The fix is naming, not middleware.** Each leg returns `{source, status,
text}` rather than `text`. A failed leg is *named* in the output the owner
sees — "3 of 5 sources unavailable, the comparison covers two" — and the model
never sees a gap as a fact. The owner asked whether to build error-handling
middleware or a cache TTL next; the answer was neither, because **a cache TTL
on a pipeline that swallows failures just caches the emptiness**. Honesty
about which legs answered comes before any optimization on top.

**How to apply:** in any fan-out/fan-in or multi-source gather, make the
per-leg status part of the data shape from the first commit, not a later
hardening pass. Filter for *display*, never for *content* — dropping a failed
leg from the list the user sees while keeping its absence out of the model's
context is exactly backwards. Related: [[blend1-web-leg-and-corpus]] (the
pipeline this came from), [[verify-crypto-against-published-vectors]] (the
other lesson from this session about a check that passes when it should fail).
