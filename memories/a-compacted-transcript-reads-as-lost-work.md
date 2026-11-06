---
name: a-compacted-transcript-reads-as-lost-work
description: "feedback 2026-10-20 — the owner said part of the conversation \"disappeared from this session\" after compaction folded older turns; the work was committed and live. Answer with the commit and the live address, and read the second signal underneath (the design round they were enjoying was skipped)"
type: feedback
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

**The message, verbatim:** "I thought we were working on the memories & skills
using atlas but that part of the conversation disappeared from this session.
what happened? I really liked what you were doing." Owner, 2026-10-20.

**What it was.** Context compaction, not lost work. The memory/skills rebuild
was committed and pushed (`e7e36c2`), the release binary rebuilt, and the face
restarted under `com.fleet.gui`; the visible transcript simply no longer held
those turns. The right answer was to say so and point at the commit, the live
address (`#/memory`, `#/skills`), and the numbers on the page — not to redo
anything.

**The second signal, which matters more.** "I really liked what you were
doing" was about the *design round*, not the outcome: that rebuild skipped the
populated Ledger/Atlas-style mockup the second-brain work had, and applied the
Atlas grammar straight to the live panels. See the grammar rule in
[[plan-brainstorming-before-development]]. The offered next step was the
carousel for memory and skills before touching those panels further.

**How to apply:** when the owner describes missing conversation, first prove
the work is still there (commit hash, pushed state, the running face) before
explaining anything; then ask what part of it they valued — the disappearance
and the disappointment can be two separate facts. Related:
[[rebuild-is-not-commit-push-or-ship]],
[[fleet-second-brain-visual-redesign-awaiting-pick]].
