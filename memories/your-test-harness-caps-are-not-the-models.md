---
name: your-test-harness-caps-are-not-the-models
description: "when benchmarking a zero-cost local model, the caps you set on your own harness read as the model's ceiling — and one bad run that matches a documented failure mode for a config you used is a confound, not a conclusion"
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

Benchmarking `prism-ml/Ternary-Bonsai-2-2B` on the owner's PrismML fork
endpoint, the owner pushed back: *"bonsai at this size has about 8K context
window, why are you artificially capping a model that is running at zero
cost?"*

They were right, and the caps were mine. `-c 8192` was a flag on the
llama-server **I** launched (and killed once the fork turned out to be the
answer); `max_tokens: 300/900` in my test requests were diagnostic caps — the
900-token run was what exposed the thinking-mode budget exhaustion. Neither
was a limit on the owner's server. Once removed, that server accepted an
**11,230-token prompt** (55KB) and answered correctly, in 20.5s.

**Why:** a cap you set to bound your own experiment is indistinguishable, in
your writeup, from a ceiling the subject has. The failure is not the cap — it
is reporting the capped number as the model's. And the cost asymmetry matters:
a hosted model's context costs money per token, so capping it is prudent; a
local model at zero cost has no such reason, so an arbitrary cap is pure loss
of information.

The same exchange produced a second correction. My first bad run used thinking
**on** at temperature **0** — exactly the card's own documented failure mode
("avoid greedy decoding with thinking on; it can get stuck reasoning"). The
owner quoted it back. One data point that happens to match a documented
failure mode for a config you used is a **confound**, not a conclusion. Run the
matrix: thinking × temperature. Doing so kept the conclusion (thinking off is
still required for short constrained tasks — even at the card's recommended
temp 1.0 it deliberates past its budget) but put it on measured footing instead
of one compromised run.

**How to apply:** before reporting a local model's limits, ask which caps were
yours and remove them. When a bad result matches a documented failure mode for
a setting you used, disentangle the variables before concluding — especially
when the subject's own documentation warns about that setting. And when the
owner states a figure ("about 8K context"), treat it as their observation, not
as the measurement: here the card declared 262,144, the owner observed ~8K
effective, and the server accepted 11K+. Three numbers for three different
things, and the one that drives design is the measured **latency curve**
(0.2–0.3s short, 20.5s at 11K tokens), not any of the context figures.
Related: [[ternary-bonsai-needs-the-fork]],
[[verify-crypto-against-published-vectors]],
[[owner-stated-facts-are-authoritative]].
