---
name: verify-what-renders-not-what-commits
description: "2026-10-19/20 — when the owner distrusts shipped work, commit history and a serving binary do not refute them; the rail was committed/served/not what was approved, and the port's \"verified live in Chromium\" ran only at desktop widths. Render the real artifact at the widths the feature is FOR, and check the gate's actual steps"
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

The session's arc: the owner reported the GUI "nowhere to be found"; I verified
commits, the installed binary, the served bytes, the vm harnesses — all true —
and reported the work intact, twice. The owner was right anyway: the rail they
approved existed for 43 minutes before a same-day commit (`f4e31d5`) stripped
its tree, leaving the function's comment and the stylesheet to keep promising
the missing piece. "Committed and serving" and "matches what was approved" are
different checks, and only the second answers the complaint. What finally
settled it: rendering the approved mockup and the shipped UI side by side, then
diffing `buildRail` across the strip commit.

**Why:** a green gate covers what was built
([[a-green-gate-covers-what-was-built-not-what-was-left-out]]) and says nothing
about whether what was built is what was approved; a silently degraded feature
leaves no red test, because the leftovers — the comment, the dead CSS — keep
the old story alive while every check passes.

A second occurrence, 2026-10-20: the Bots surface was committed and its
commit message said "verified live in Chromium" — true only at desktop
widths, where the phone layout under 900px was a 236px strip
([[a-media-query-adds-no-specificity]]). Rendering at the widths the
feature exists for is part of the same check as rendering at all.

A third failure mode, same audit: the owner's "there was some ui work there
already but I don't trust it" was right on substance too. Lint and format had
never been run on the tree — `fmt --check` red on 128 hunks / 17 files,
`clippy -D warnings` red on 18 ([[a-verification-with-no-output-is-not-a-pass]]).
So the check has two halves: render what the surface does, and run the gate's
actual steps rather than trusting a claim that the gate is green.

A fourth, same day: the PWA work's new remote assertion found that a paired
phone's `GET /` returned the pairing page — an ordering defect no reading of
the handler had surfaced, because the auth match looked correct until a test
asserted what a *paired* client sees at the root. Writing the assertion for
the state the feature is FOR is the cheapest way to find ordering bugs.

A fifth, 2026-10-20: the owner sent a screenshot of the rail and said "the icon
should have been changed on the rail". The app icon had been committed, rebuilt,
served and installed on his phone — and the rail still drew its old three-line
placeholder glyph, because the mark was a separate inline SVG that nobody had
repainted. "The icon" in the owner's sentence meant every place the console
draws a brand mark, and the installed icons were only the ones he could see from
outside the app. Same lesson one level down: a brand asset change lands on the
rail, the boot-fallback card and the worker's offline page, not only in the
manifest inputs.

**How to apply:** when the owner distrusts shipped work, render the real
artifact ([[gui-client-verified-by-a-vm-harness]] — Playwright at the widths
the feature is for, not just desktop) and drive it against a seeded scratch
home before citing history. Then run every gate step yourself — fmt, clippy,
build, test — and read each result line. Read the code around the feature for
leftovers that describe more than the code does — a comment promising a level
the function no longer renders, styles for elements nothing creates, a printed
flag that stores nothing — because they mark exactly where the gap is.
