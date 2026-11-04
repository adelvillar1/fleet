---
name: fleet-docs-revamp-direction
description: "owner direction 2026-10-21 — SHIPPED 7a589a9 — README + FUNCTIONAL-SPECIFICATIONS rewritten in product-manager language, TECHNICAL-DOCUMENTATION in technical-program-manager language, on the arc spine; gates green (Rust 1489, plan-gate, docs-gate, arch-gate); the plan-gate coverage lesson and the docs-gate linked-artifact format are recorded"
type: project
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

2026-10-21 the owner asked whether I have enough context to revamp
README.md, TECHNICAL-DOCUMENTATION.md and FUNCTIONAL-SPECIFICATIONS.md,
with an audience mapping: **README + FUNCTIONAL speak the language of a
product manager; TECHNICAL speaks the language of a technical program
manager.** Answered yes-on-substance; nothing written yet — the plan with
one fully written mockup section per file is the next step, and the owner
has not said go.

Constraints that shape it (found by measuring, not guesses):
1. `arch-gate` judges every future plan's claims against
   TECHNICAL-DOCUMENTATION.md ("low documented = UNDOCUMENTED"), so the
   module reference must be **retained as the gate's substrate** and
   framed for a TPM (systems, interface contracts, environments,
   runbooks, risk register, sequencing), not replaced by narrative.
2. The write-session-recap skill proposes text into both contract docs
   every session ("summary developer-onboarding contract" /
   "user-flow contract"), so the new section shapes must remain
   appendable by that step or the document loop breaks.
3. `docs-gate` checks linked-artifact coverage + staleness; CLAUDE.md's
   house rules: one home per fact, English-only, no updated-dates.

Target shapes agreed in conversation: README = what Fleet is, who for,
the jobs, shipped-vs-in-flight, principles as promises, two-minute start;
FUNCTIONAL = capabilities as user flows by surface with named gaps as a
backlog; TECHNICAL = the TPM list above. Before drafting: read the three
files in full (~700 lines — only grepped so far) and the 18 feature docs'
openings (a plan is rejected for what it OMITS). Sample PM-voice opening
already shown to the owner ("Fleet is a coding agent that runs on your
machine, under your keys, inside the folder you point it at…").

**How to apply:** when the owner says go, the deliverable is a plan
(docs/plans/, gate PASS, mockup sections for voice approval) before any
rewrite. Related: [[plan-is-a-file-not-prose]],
[[plan-brainstorming-before-development]],
[[fleet-harness-orientation-doc]].

Update 2026-10-21, later same day — the understanding work is DONE and
committed. The owner steered the effort three times mid-turn, and the
steerings are the brief: **"focus on the functionality, not on the
mechanics"**; **"we are not fixing code, we are making the documentation
more aligned to the spirit of the app. the what and the why"**; **"the
how, it's in the code."** I landed on every rail destination live
(Playwright, channel "chrome", text inventory + screenshot per surface —
11 destinations: chat, sessions, router, delegation, memory, skills,
second-brain, bots, vault, settings, calibration) and wrote the substrate
the revamp will lean on: **`docs/architecture/surfaces.md` (dabdad6,
pushed)** — the shared grammar (group kicker, title, one-sentence
purpose, an identity card whose **ask** buttons are the tell — every
surface is an entry into the conversation about the thing shown, "the
conversation is the app"; strip of counts; column; rail), the principle
each surface embodies (local-first egress printed under the composer,
log as system of record, Auto/no-pin over hardcoded, no form asks,
proposals never apply silently, graded not recorded, a value is never
rendered, plan-delegate-verify), and per surface what/what-you-do/why/
honest state. The docs' data state is mid-development: the boards' red
numbers are the 14 nightly corpus-refresh jobs (most failing, KI-277) and
the pin week — 98 commits / 61k insertions in three days — not product
failure; the brief says so in one paragraph. Feature-doc theses read for
all 18 docs/features files. Screenshots + `surfaces-inventory.json` live
in the session visualization directory.

Update 2026-10-21, still later — the lanes brief and the plan are done.

**`docs/architecture/lanes.md` (a869048, pushed)** answers the owner's
showcase ask: why sys1/sdm1/sem1/gen1/blend1 exist and beat LLMs at
their jobs. The spine: the LLM is one component, not the harness — every
kind of work goes to the component that can do it and the language model
speaks LAST. Hallucination control by construction, not instruction:
closed output spaces (a label from a declared set, a number, a vector —
none can produce a fluent invention), confidence gates in the path (the
route floor 0.72, blend1's floor that keeps the LLM silent until the
decision model clears it), a gated/bounded/validated generative leg, and
graded outcomes that decide whether the failover may learn. Smaller
models smarter: judgment, measurement, recall and seeing are carried by
components cheap and reliable beside the model, so flash-tier runs do
work they would fail alone. The router is the dispatcher that makes the
five one system. Same register rules as surfaces.md — goal, intent,
solution; no growing pains.

**The plan `docs/plans/2026-10-21-docs-speak-the-product-and-the-program.md`
is written, plan-surface run, and plan-gate PASSES (5/5 covered, 2/2 in
scope); status stays `draft` until the owner approves the voice.** One
narrative spine for all three documents, which is the owner's own arc:
"we had an idea, we experimented, found a possible solution, tested it,
and then we are where we are." It carries a before/after section map
(README: Status/Build/Commands/Configuration/Sessions/Source layout →
What Fleet is/The arc/What ships today/What is next/Start here/Where
things live; FUNCTIONAL: …→ What Fleet is for/The surfaces/The commands
as verbs/**Run behavior (kept)**/What Fleet tells you when/Known gaps;
TECHNICAL: Crate contract/Environment contract/Module reference →
System overview and boundaries/Interface contracts/Environments and
dependencies/Operational runbooks/Verification and release/Risk
register/Sequencing/**Module reference (kept verbatim)**/Known
documentation gaps). Three mockup sections — README's opening, a
conversation flow, an interface-contracts opening — are quoted in the
approval request so the owner approves the register before 700 lines are
written in it. Five criteria C0–C4; two gate constraints unchanged
(module reference is the arch-gate substrate; `## Run behavior` and
`## Module reference` are the recap step's append points).

**SHIPPED 2026-10-21 as 7a589a9** (the owner approved with
`/goal implement the plan as designed`): all three documents rewritten in
the approved voices, plan status completed, recap at
`docs/recaps/2026-10-21-docs-speak-the-product-and-the-program.md`,
Rust gate 1489, plan-gate PASS, docs-gate PASS, arch-gate PASS with
conventions gaps (the technical doc does not yet describe its own new
sections — the next plan touching it folds them in). The recap carries
the per-section source map C4 asked for. Two entries in the module
reference were corrected rather than byte-copied because today's router
rulings made their old sentences false (`route.rs`'s "DeepSeek serves
only when pinned", `subagents.rs`'s child-runs-on-the-parent's-client);
disclosed in the recap. CLAUDE.md's pointer index and the harness
orientation's docs row name the new homes.

**Gate lesson earned here (the plan-gate whack-a-mole):** the coverage
judge scores a criterion against the plan BODY, so a criterion with no
home in the body fails no matter how verifiable it is — and the scores
shift between criteria with each edit until every one has a home.
What made it PASS: a before/after **section map** table (criteria map
to its rows), enumerating the target `##` headings in the approach
(criterion and body must name headings IDENTICALLY — "System overview"
vs "System overview and boundaries" flipped C2 to MISSING), and folding
process criteria ("the gate is green") into the criteria they pin, with
the gate kept in Verification. Mockup quotes inside the plan body got a
SCOPE? flag — they moved to the approval request instead. Run plan-gate
WITHOUT `--tests` pre-build (the tests don't exist yet) and WITH
`--tests src` post-build.

**docs-gate's linked-artifact format (earned here):** every path under
`## Linked artifacts` must be **backticked and carry a promise** naming
the section that delivers it ("`CLAUDE.md` — the "Where to find things"
pointer index will name…"). Un-backticked paths read as "no linked
artifacts found"; a promise whose section doesn't exist in the artifact
reads as NOT COVERED. The judge covers an artifact when the plan body
names a section the artifact actually has.
