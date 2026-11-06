---
name: triage-at-the-owners-register-deployment-facts-first
description: "Owner feedback 2026-10-30 — stop consumer-grade triage (reloads, caches, browser pedagogy); answer \"is it deployed\" with process-start vs binary-mtime and the page's build stamp, measure the real viewport, and make the product name its build"
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

The owner's correction, verbatim (2026-10-30): *"the irony of this is
that I've been working in enterprise IT for 30 years, I built this
harness and yet you are triaging like you are talking to a vibe coder"*
— after an evening where I explained PWA window lifetimes and caches,
dug through Chrome's CacheStorage LevelDB, and re-advised reloads, while
my own `build && kickstart` chains were being cancelled before the
restart, so the process he was hitting was genuinely stale.

**Why:** he built this system; a peer-level account is root cause, the
instrument, and the verified fact — not browser fundamentals he wrote a
worker around. Two of my three failure modes were *his* to see through
and mine to have instrumented correctly:
- I rendered the face once at 1440×900 and called it green; the defect
  lived at his window height (rowTop 747 below a 700px fold). Measure
  the matrix, every state-dependent screen — [[the-screenshot-is-the-visual-review]]
  says read the PNG; it also means read it at the height the owner uses
  ([[mockup-review-matrix-is-a-tool]] names the same rule).
- Asked "is this the new code", I went cache-spelunking instead of
  comparing `ps` start time to the binary's mtime and grepping what the
  live server *serves*. That comparison answers it in one command.

**How to apply:**
1. "Is it deployed?" = process start vs binary mtime, then
   `curl <face>/app.js | grep` the change, then the page's own build
   stamp. Three facts, ten seconds. Never a cache story.
2. Every face now prints `build <hash>` (the counters strip; the asset
   hash from `?v=`). Check it in any window before reasoning about
   stale JS. A build identity on the page beats any forensic.
3. Any deploy chain that includes a build goes in the background —
   interrupted foreground builds leave the process old while the
   narration moves on (this happened repeatedly tonight).
4. Reproduce in the owner's client, not the test client: the defects
   that mattered (KI-370/371/372) surfaced only through a real browser
   at the real origin — the loopback playwright shots and the disk
   forensics both said "fine" while the paired phone was broken.
5. Report the root cause in engineering terms with the evidence: the
   line that decided it, the measured number, the command. No
   reassurance, no pedagogy, no "try reloading."

Related: [[owner-enterprise-it-fingerprint]],
[[both-faces-required-console-and-bot]], [[verification-depth-matches-the-change]].
