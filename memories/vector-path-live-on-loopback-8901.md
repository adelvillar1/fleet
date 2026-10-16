---
name: vector-path-live-on-loopback-8901
description: "fleet's recall runs the vector channel live: llama-server serves embeddinggemma-2 at 768 dimensions on 127.0.0.1:8901, doctor names the path, and every fact's vector caches in the embedder-keyed index at memories/.recall-index.json."
type: project
date: 2026-10-15
basis: confirmed-by
tier: durable
shape: [deps]
phrases: [FLEET_EMBEDDING_ENDPOINT overrides the 8901 default; a model swap invalidates the index whole; absence degrades to keyword with one named line]
missing: []
recall-count: 65
superseded-by:
valid-until:
---

The probe is a real embed round trip of the text 'fleet recall probe', not a health poll: a health endpoint that answers ok proves nothing about the embeddings route, and a route that cannot embed is exactly the silent degradation the probe exists to name. The index is keyed to the embedder's endpoint and dimensionality, so a cache written by another model - or by the pre-key format - is invalidated whole and rebuilt rather than serving stale-dimension vectors that cosine would score 0.0. Verified 2026-10-15: doctor reports 'recall ok - vector path live - http://127.0.0.1:8901/v1/embeddings (768-dim embeddings serve recall)' and the index holds one entry per fact in the corpus.
