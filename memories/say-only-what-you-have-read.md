---
name: say-only-what-you-have-read
description: "The owner's hard ask — stop making things up; no claim stated as fact unless read in its primary source, inferences labelled, and a correction means go read, not reword"
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

The owner's words: *"stop making shit up"*. Three instances in one session,
all the same failure:

1. I said sys1 was "a vendored lane, so nothing needs a key" — from a lane
   spec comment, not from `services/sys1/src/sys1/config.py`.
2. When corrected, I restated the same framing more carefully instead of
   going to the source. The correction is the signal to read, not to reword.
3. I said C14 "needs a provider key" — treating a choice (a scratch home
   starts empty) as a blocker, and asserting it as fact.

The owner's follow-up made the cost explicit: *"it would have taken you
seconds to read what sys1 is, and now it's been minutes you litigating with
me, the one that created sys1 what sys1 is."* Reading the file was seconds;
the argument was minutes; and the person who built the subsystem was the one
I told what it was.

**Why:** a confident wrong claim costs the owner more than an admitted gap.
They cannot audit my inferences, so anything I state as fact becomes
something they have to verify. Two wrong claims about the same component in
one session is the pattern, not the slip. And defending the claim instead of
reading it burns the owner's time and their patience.

**How to apply:**
- Before writing "X is Y" about this codebase, have I opened the file that
  says so? If not: either read it first, or write "I have not read X; my
  understanding is Y — I'll confirm".
- Label inferences and assumptions explicitly. "A scratch home starts
  empty" is a design choice, not a fact about the machine; "C14 needs a
  key" was neither.
- When the owner corrects me, go to the primary source immediately. Do not
  restate the corrected framing with more hedging — that repeats the error.
- Prefer "I don't know yet" over a plausible sentence.
- When corrected, say the correction is right in one line and move to
  reading. Do not litigate.

Related: [[read-the-services-own-config-first]],
[[do-not-guess-what-a-subsystem-needs]]
