---
name: a-repeated-marker-slice-clobbers-whatever-follows
description: "feedback 2026-10-28 — index-slicing a file with python (`t.index(marker)` … `t.index(end, a)`) silently destroys a LATER region when the marker repeats; the file shrinks or a later scenario vanishes, and the damage only shows in a rendered state — rewrite the file whole when the marker is not unique"
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

2026-10-28, editing the bot-PWA mockups by python string-slicing:
`t[:a] + NEW + t[b:]` with `a = t.index('<div class="scroll">')`. In
`install.html` the marker appeared in two scenarios; the second
replacement matched the FIRST remaining occurrence and deleted the
installed-state region outright — the screen rendered blank, and a
verify pass caught it only because the `.phone` node went missing. The
same class of failure had already hit `conversation.html` earlier that
day (a matched anchor swallowed three scenarios; the file ballooned to
28k lines of duplicated content).

**Two rules that held:**

1. **Never index-slice on a marker that occurs more than once in the
   file.** Either scope the search (`t.index(end, a)` is not enough —
   anchor from a unique per-region string), assert the marker's count
   first, or rewrite the file whole. Rewriting whole cost one Write and
   zero surprises; every patch attempt on these files cost a round of
   debugging.
2. **A structural verify catches it, a glance does not.** The
   headless verifier checked `document.querySelector('.phone')` and the
   state switcher's buttons per file — that is what named the missing
   region. A DOM-presence check per scenario is the cheap pin for
   hand-built HTML.

Also from the same session, two CSS facts worth keeping: a fixed flex
basis (`flex: 0 0 412px`) only wins if the item may shrink below its
content (`min-width: 0` — a 900px persona rail otherwise stretched a
900px frame 239px wide), and a `.when`-style hide rule must carry the
same specificity as the base rule it hides (`.dock .primary-act
.stop-act` to beat `.dock .primary-act`), or both buttons render.

**Why:** string surgery on generated-feeling HTML fails silently and
specifically — it deletes the wrong region, not the edit. **How to
apply:** when a file has repeating structural markers, write the file
whole instead of patching it, and keep a per-state DOM-presence check in
the verifier so a swallowed region reddens. Related:
[[mockup-review-matrix-is-a-tool]],
[[a-factored-composition-must-keep-the-exact-texts]].
