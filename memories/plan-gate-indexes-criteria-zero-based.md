---
name: plan-gate-indexes-criteria-zero-based
description: "feedback 2026-10-24 — dev-decisions plan-gate indexes criteria 0-based (C0..Cn) while plan authors label theirs 1-based; section headings asserting \"C2 is this increment's deliverable\" cross-wire the small coverage judge and slide failing heads DOWN, and a close-out criterion needs its own body section to pass"
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

2026-10-24, gating the deck-vector plan: three coverage heads kept
failing (p sliding 0.37 → 0.22) while their steps sat verbatim in the
body. The mechanism was not prose quality — my section headings said
"**C2** is this increment's deliverable / C3 / C4" while the judge's own
heads were C0..C4 over the same lines, so each section asserted a
NEIGHBORING criterion's identity to a small provider (jev) that trusts
the labels. Relabeling every heading to the judge's 0-based indexing
flipped three failing heads to covered in one draw.

**Why:** the coverage judge matches criterion text against section
bodies, and the criterion labels in the state are the tool's own C0-based
head ids — a section claiming the wrong letter reads as covering a
different criterion than the one being scored. Also learned the same
night: a close-out criterion ("full gate green, the ledger agrees") with
no body section of its own fails coverage even when the preamble mentions
it — the head asks "does the plan body contain steps", and a preamble
paragraph was not enough.

**How to apply:** in docs/plans files, either label criteria C0-based to
match the gate, or put NO letter labels in section headings and let
position align; and give every close-out/gate criterion a short
`### Close-out` body section with its literal steps (the four gate
commands, the row/plan agreement). When coverage slides DOWN as text is
added, stop adding and check the labels — that signature means
cross-wiring, not thin prose. Related: [[plan-is-a-file-not-prose]],
[[fleet-docs-revamp-direction]].

**2026-10-29 addendum (relay-on-OpenShip plan):** two more forms. (1) A
`## Use cases` checkbox block JOINS the head count — 6 criteria + 3 use
cases read as 9 heads C0..C8, so `**C0**`-style labels inside the
criteria text and `**C0 —**` verification headings misalign with the
tool's numbering; dropping the letter labels entirely (checkbox order is
identity) is the robust fix. (2) Coverage flags can be a calibration
FLOOR, not a gap: two heads sat at p≈0.3 across three revision passes
that made the approach restate the criteria nearly verbatim — at that
point record a disposition with the justification at approval instead of
editing again. Related: [[remote-relay-openship-plan-2026-10-29]].

**2026-11-01 addendum (design-parity plan):** plan-surface AND plan-gate
both **hard-cap at 12 criteria** — a 14-checkbox plan parsed as "12
criteria, C0..C11", silently dropping the last two (verified: the surface
JSON's criterion texts end at the 12th checkbox; rewording the 13th's
route text changed nothing). The dropped criteria fall outside coverage
scoring entirely — keep their own Verification steps in the contract and
say so when presenting gate verdicts. Also: the standalone
`## Decision-model check` section (the plan skill's shape) got flagged
as SCOPE? by plan-gate — folding the no-sys1-fit line into the Approach
paragraph cleared it, matching the skill's literal "put the fit in the
Approach". Related: [[design-parity-plan-2026-11-01]].
