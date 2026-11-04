---
name: memory-transplant-zcode-to-fleet-proposed
description: "Owner asked 2026-11-04 to transplant ZCode's fleet memories into fleet's own memories/ corpus — the design is proposed (fleet's own remember writer, four curation passes), plan not yet drafted"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked 2026-11-04: can ZCode's memories about the fleet project
be transplanted into fleet's own `memories/` corpus, "so fleet is as
smart as zcode on fleet itself". The verified shape of the answer:

- **The corpora are near-disjoint**: fleet's `memories/` holds 164
  facts, ZCode's project memory holds 440, and only 2 slugs overlap
  (`fleet-is-the-native-rust-successor-to-the-fork`,
  `harness-compositional-generalization-paper-vs-fleet`) — the
  transplant is almost pure gain.
- **The path is fleet's own writer, not a converter**: loop over
  curated facts invoking `fleet remember` (the CLI speaks the writer's
  whole shape — name/description/basis/tier/shape/phrases/missing,
  `src/cli.rs` parse_remember), so frontmatter is conformant by
  construction, shape is classified by fleet's own keyword rule, and
  the recall indexes are maintained by the code that owns them. No
  translation code ships in the repo.
- **Four curation passes**: (1) tiering — owner rulings, project
  state, feedback on fleet work, references transplant; ZCode-harness-
  specific feedback (chat-UI quirks, the router's model picks) skips;
  superseded chains trim to the live tip; (2) dedup against the 164
  existing facts (ZCode's project memories and fleet's consolidation
  folds recorded the same landed work from two sides); (3) public-repo
  screening — the repo is public and ZCode memories carry sibling-repo
  internals (provider economics, measured numbers); name the sensitive
  set for the owner's nod; (4) basis assignment (confirmed-by vs
  inferred) per fact.

**Why:** the owner's goal is explicit and the analysis is done — but
the work is plan-sized (~440 triaged into maybe 200–300 landed) and
per the house rules waits for its migration plan.

**How to apply:** when the owner says go, draft the migration plan
(tier rules as criteria), prove the path on a pilot batch of one fact
per type via `fleet remember`, verify `fleet memories --recall` ranks
it, then run the bulk in batches. Before landing anything, commit the
26 modified + 1 untracked `memories/` files already sitting in the
tree (the house chore(memories) pattern). See also
[[knowledge-work-core-named-desk]] (the same session's desk work).
