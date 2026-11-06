---
name: a-factored-composition-must-keep-the-exact-texts
description: "feedback 2026-10-23 — factoring a prompt builder into a shared function \"for reuse\" paraphrased two of its arms and dropped a cap, and only the existing tests caught it; when extracting shared code, move the strings verbatim from the original, and let the suite's red rows name what the refactor silently changed"
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

While building the "what the model saw" inspector (2026-10-23), I
factored fleet's `system_prompt` into a shared `composed_prompt` so the
live loop and the reconstruction could not drift. The refactor itself
was the right move — the byte-equality pin depends on it — but my
rewrite **paraphrased the FileAccess folder/none arm texts and silently
dropped the 60-skill cap** (mapping ALL skills into the prompt instead
of `declared_skills`' capped list).

Neither broke compilation. Both broke behavior — and only the EXISTING
tests caught them: `a_chat_with_no_folder_refuses_the_filesystem_tools`
asserted the original arm's exact words, and
`the_declared_list_is_the_newest_under_the_cap` failed on the held-back
skill appearing. `git show <prev-commit>:file` to recover the original
strings verbatim was the fix for the texts; re-routing through the
capped list was the fix for the cap.

**Why:** when you extract shared code "mechanically", the temptation is
to retype it clean. For anything whose OUTPUT is asserted — prompt
compositions, card renderers, wire shapes — the strings ARE the
contract. A paraphrase is a semantic change wearing a refactor's clothes.

**How to apply:** (1) move strings with copy-from-the-original, never
from memory; when in doubt `git show` the pre-refactor text and diff.
(2) A factored function that takes a slightly different type (Skills →
SkillRefs here) is exactly where a behavioral step like a cap gets lost
— enumerate what the old type's conversion used to do. (3) When the
suite reddens after a "pure refactor", believe the tests over your
intent: the two red rows named the two real changes precisely.

Related: [[chat-excel-direction-2026-10-23]] (the build this happened
in), [[the-check-and-the-work-must-be-in-the-same-place]].
