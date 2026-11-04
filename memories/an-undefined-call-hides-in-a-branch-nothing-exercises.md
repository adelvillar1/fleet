---
name: an-undefined-call-hides-in-a-branch-nothing-exercises
description: "a call to a function that does not exist throws only when its branch runs — the receipt's "ask what is next" called askFace (never defined) and no test could catch it; grep the app for bare calls with no definition rather than relying on a syntax check or on the surrounding code looking right"
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

While rewiring the conversation surface (2026-10-23) I found
`src/gui/assets/app.js` calling `askFace(...)` in the run receipt's
*ask what is next* action. The file contained exactly one occurrence — the call
— and no definition, so the click threw a `ReferenceError` and the composer
stayed empty. `git show HEAD:… | grep -c askFace` also returned 1, so the defect
predated that session and had shipped unnoticed.

**Why it survived everything:** `node --check` passes it (an undefined global is
not a syntax error), no face test exercised the receipt's action, and the code
*reads* correctly — the name is adjacent to the real helper `ask()` that fills
the composer, so a reader assumes a near-synonym exists. It is the JS sibling of
"a proof nobody ran": the branch that would throw is the branch nobody clicks.
Same family as [[an-effect-with-no-visible-trace-reads-as-broken]] — the button
looked decorative because it was broken.

**How to apply:** for a large hand-written asset with no test runner, do a
one-pass **undefined-call audit** before shipping: extract every bare
`name(` call from the file and diff it against the set of function
declarations (`function X(`, `var X = function`), then subtract the real
globals (`document`, `window`, `Math`, `JSON`, `fetch`, `requestAnimationFrame`,
`Promise`…). This session's version reported seven names and one was an
undefined function; four others were legitimate callback parameters. A regex
audit has false positives — check each hit rather than trusting the list — but
it finds the class in one command, which no syntax check will. Related:
[[gui-client-verified-by-a-vm-harness]] (the live face is where an unexercised
branch shows up), [[a-verification-with-no-output-is-not-a-pass]].
