---
name: embeddinggemma-cosines-confirm-ki-058
description: "embeddinggemma-2's raw cosines do not separate relevance on their own: unrelated pairs measure 0.58-0.66 and related pairs 0.77-0.93, so the vector channel's score is only readable against the direction channels and never on its own."
type: project
date: 2026-10-15
basis: inferred
tier: durable
shape: [change]
phrases: [unrelated pairs sit at 0.63; related pairs reach 0.85; the 0.15 vector weight is confirmed; proximity must not outrank direction]
missing: []
recall-count: 331
superseded-by:
valid-until:
---

Measured 2026-10-15 against the live loopback embedder (llama-server, embeddinggemma-2, 768-dim) with eight probe pairs: four unrelated (router-quota vs bass-migration, recall-ranking vs bridge-tension, write-guard vs sourdough, depth-cap vs terrain-rendering) scoring 0.5836-0.6648, and four related pairs scoring 0.7673-0.9327. The unrelated floor of ~0.63 reproduces KI-058's 0.6049 on the previous stack within 0.03, which is why the vector channel was reweighted from 0.55 down to 0.15 at slice 19 and the trajectory channel made the largest term at 0.30. The re-measurement confirms that ruling on the current model: a bigger vector weight would let a maximum-proximity fact with no direction evidence outrank the direction-strong one, which is the owner's ruling inverted. The channel does carry signal - the related/unrelated gap is ~0.22 - so it is not noise; it is a relative reading, not an absolute one.
