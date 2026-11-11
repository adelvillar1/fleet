---
name: two-stage-recall-stays-off
description: "The two-stage recall prefilter is ~7.5% cheaper and changes results 13% of the time — it stays off"
type: project
date: 2026-11-11
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Measured 2026-11-10 with the embed hoisted out of the timed region (KI-459's fix: partition_with_vector/recall_with_vector; probed_embedder_ready, because probed_embedder answers keyword-only on its first call). Median-of-7 over 10 instructions on the live 700-fact corpus, three runs identical: single-pass 249-252 ms vs two-stage 232 ms per pass — ~7.5% cheaper — but mean recall@10 agreement 0.870 against the 0.98 gate. Verdict: memory.vectors.two_stage stays OFF. The speed is real and does not buy a different answer.
