---
name: defer-is-a-counting-claim-check-the-bite-count
description: "feedback 2026-11-01 — I recommended deferring KI-369's spawn route on \"it bites twice first\" while the ledger rows I had already read listed three bites (KI-221/224) plus that morning's KI-387; the owner's \"why defer?\" exposed it — a defer recommendation is a factual claim that the evidence accumulated so far does not yet pay for the work, so count the ledger, don't count recent-green days"
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

While planning the open calls (2026-11-01), I recommended deferring
KI-369 — the durable spawn route for lane jobs — because "the hand-driven
path has run nightly for five days without a new defect; the increment
earns itself when it bites twice." The owner pushed back with one line:
"almost agree with everything, but why defer ki-369?" Re-examining the
rows I had **already quoted in the same plans**, the bite count was paid
at least three times over: KI-221/KI-224 (the job could not execute as
written, "fixed" by pointing the task string at a hand-built driver),
every 2026-10-28 catch-up hand-driving the own same stdio JSON-RPC
session, and KI-387 itself — filed that very morning — where the first
run through the spawn route cost a Content-Length dead-read of 90s, a
relaunched child killed as a leftover, a bytes/str TypeError, and a
fourth attempt before running clean. My own W3 criterion (write the
transport contract where drivers read it) was a documentation patch over
what should be a call into `src/mcp.rs`.

**Why:** a "defer" recommendation is not a judgment call — it is a
factual claim that the cost already paid is smaller than the cost of the
work. That claim is checkable against rows I had read and summarized an
hour earlier, and "five green days" was anecdote against the ledger's
own record. Recent quiet is not absence of bites; a recurring nightly
job that each run re-derives its contract is paying the tax every night
whether or a new row gets filed.

**How to apply:** before recommending defer on any ledger row, grep the
cluster for prior bites of the SAME shape (the row's own provenance is
usually "found by a catch-up," which means the family recurs), and count
them out loud in the recommendation. If the count is ≥2, the bar is
already met. Also: when the owner challenges ONE item among many
("almost agree with everything, but…"), treat that item as the one where
my reasoning was weakest, re-derive from the evidence in hand, and
revise with a named commit — the revision cost one line in the plan file.

**The follow-on challenge, same session:** "is what you are suggesting
the right fix or the easy fix?" — a challenge to the WHOLE slate rather
than one item. Re-audit every recommendation by naming the fix my rec
AVOIDS and why I avoided it: if the answer is "it costs more edits"
rather than "it's wrong," the rec was the easy fix. That audit flipped
O3 (documenting the drift severities instead of normalizing to the
taxonomy — one edit that dissolved the contract), hardened O2 (an
invented product ceiling copying the goal's pattern the composer's own
131 KB draft contradicts) and O5 (widening the gitignore while leaving
the writer free to mint new spellings), and sharpened O1/O4/O6.
Related: [[corpus-contract-and-open-calls-plans-2026-11-01]],
[[a-ledger-row-can-name-the-wrong-cause]].
