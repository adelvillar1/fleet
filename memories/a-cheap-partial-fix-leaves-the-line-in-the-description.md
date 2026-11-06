---
name: a-cheap-partial-fix-leaves-the-line-in-the-description
description: "feedback 2026-10-23 — a plan criterion that could be written two ways (\"attach the image or fix the sentence\") gets the cheap half done and is then immune to re-audit, because the grep the earlier audit used no longer finds the lie; choose the fix that makes the claim TRUE, and when auditing, re-derive each criterion from the code rather than re-running the old check"
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

Building the tool surface (2026-10-23), the plan's C0 said `computer_use`
must either "attach the screenshot as an artifact **or** its description drops
the claim that the image rides to the model". I took the second branch — one
sentence changed in `tools.rs` — and wrote "C0 landed" in the progress
section. The tool still does not deliver an image to the model; the model
reads "read it with `vision` when you need to look at it". That is arguably
fine, but the criterion read as satisfied, and the completion audit's grep for
the *old* sentence now passes because the lie was deleted rather than the
capability added.

**Why:** an "or" in a criterion converts a capability gap into a wording
chore, and a checklist item that can be closed by deletion is closed by
deletion. The audit is worse off, not better: the first pass flagged the lie
because the text disagreed with the code; the second pass finds neither the
lie nor the feature and calls it done.

**How to apply:** when a criterion offers two branches, take the one that
makes the claim true, or write the honest one as an explicitly open item in
the same breath (as the plan already does elsewhere: "C12 wiring open, the
policy is tested but nothing calls it"). When auditing a plan, do not re-run
the previous audit's grep — re-derive each criterion from the current code,
and ask "does the capability exist, or did the sentence move?" Related:
[[a-green-gate-covers-what-was-built-not-what-was-left-out]],
[[the-check-and-the-work-must-be-in-the-same-place]].
