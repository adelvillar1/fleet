---
name: readme-greenfield-rewrite-2026-11-11
description: "project 2026-11-11 — owner rejected the README, ordered a GREENFIELD rewrite; THREE iterations: identity-first (~210 lines) COMMITTED 9d9b6de → owner reframed to the WHY + WHAT'S IN IT FOR ME (verifiable-facts epistemology front and center, \"your keys\" demoted to commonplace) → benefit-led rewrite (~250 lines) APPROVED by the owner (\"let's go with that and we can evolve from there\"), COMMITTED 6f2d4af + pushed to origin/master, tree clean; durable framing rule in [[readme-leads-with-why-and-benefit-not-your…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner's ask (2026-11-11, after the armada ideation arc): "you have
learned a lot about fleet and the direction we are heading. would you mind
taking a fresh stab at the project readme? I don't like what we have right
now, so treat it as a greenfield creation. I don't think the page reflects
what fleet is, and what it's trying to accomplish."

**Why:** the old README was an internal feature encyclopedia — 344 lines of
machinery (compaction triggers, recall anti-vectors, ~20 screenshots), a
stale version string (0.5.0-beta.1 while the repo shipped 0.6.29), and the
vision completely absent. It recited the harness instead of arguing what
fleet IS and where it is GOING. The durable lesson: the README is the
product's front door (product-manager register per CLAUDE.md) — lead with
identity and direction, curate the machinery, link out for depth.

**What landed (README.md rewritten whole, UNCOMMITTED, awaiting the owner's
read):** ~210 lines. Opens "A coding agent that belongs to you" — your
machine, your keys, your memory; then THE BET: most agents rent you a model
and wrap a prompt around it, fleet takes the opposite position — **the model
is one component, not the harness** (the five-lane table serves the bet, not
dominates). A "What makes it yours" section: self-contained / a memory you
can read and edit / autonomy graded on evidence / your work is the view.
"What it does today" = nine tight bullets linking to docs/features/. **"Where
it is heading" = the armada story** (one agent that's yours → many installs
one shared mind → the team workspace), linking
[[armada-multi-install-c2-idea-2026-11-11]]'s direction note. Honest beta
(0.6.x, the owner's whole-project bar — see [[beta-until-a-whole-project-runs]]).
Screenshots curated to four (chat, design canvas, huddle, TUI) + the
architecture drawing; install/start/foundations trimmed. Deep detail lives
in the linked docs, not the page.

**How to apply:** if the owner reacts, iterate tone/emphasis/featured
surfaces — the rewrite is a draft for their read, not a settled artifact.
Keep any future README edits identity-and-direction-first; never let it
regrow into the feature dump the owner rejected. The README and the armada
direction note were both uncommitted at session end (the memory-pull build
was still running in the background); commit them as docs increments when
the owner says go.

THE OWNER'S CONTEXT OFFER (2026-11-11, after the rewrite): "you have all the
recaps of everything we have done if you need more historical context." The
recap arc was read to ground the page: docs/recaps/ holds ~100 recaps, oldest
2026-10-12-fleet-dashboards.md (the FORK's fleet-profile dashboard plugins +
the shared board seam, pre-native-repo) → the 2026-10-13 slice recaps (the
native Rust port: async core, TUI, lanes, routing, calibration, skills,
memory, compaction) → the surfaces/methodology era → the recent resource-
contract/multi-session/burndown work → armada. The rewrite deliberately kept
the journey OUT of the page (a front door, not the encyclopedia the owner
rejected); the history lives in the recaps + the direction note, linked from
the README. I offered a short "the arc" paragraph if the owner wants the
journey told explicitly — unanswered; do not add it unprompted (it risks
drifting back toward the feature dump).

THE OWNER'S REFRAME — TWO RULINGS (2026-11-11, after the identity-first
rewrite was committed at 9d9b6de with the armada direction note): (1) "can we
add some information on how memory, skills, tools, workflows, and loops are
designed and the philosophy ... the different product surfaces and wanting to
make fleet a single productive front end ... grounded and bounded in a sound
and proven methodology, that verifiable facts is what drives everything ...
front and center. The fact that the harness is yours and that you use your
keys is pretty commonplace these days." (2) "it's the why and the what's in it
for me." So the identity-first lead ("belongs to you / your keys") was the
WRONG emphasis — that angle is table stakes; the differentiator is the
EPISTEMOLOGY (verifiable facts drive everything) framed as the WHY + the user
BENEFIT. The durable rule is saved as
[[readme-leads-with-why-and-benefit-not-your-keys]].

THE BENEFIT-LED REWRITE (README.md rewritten whole again, ~250 lines,
UNCOMMITTED, awaiting the owner's read): opens "A coding agent whose work you
can verify" — most agents ask you to trust a black box that guesses/forgets/
changes itself silently; fleet's answer = verifiable facts drive everything,
the model speaks LAST on computed/recalled/checked facts; "your keys / one
binary" explicitly demoted to "that is table stakes." Sections: "Why it is
built this way" (a model is a convincing guesser; fleet won't let it guess at
judgment/arithmetic/recall/risk/decisions); "What is in it for you" = five
benefit sections carrying the design philosophy — Work you can trust (the
five-lane table), A memory that compounds your knowledge (basis stated/
inferred/validated, dream pass validates inferences, corrections protected,
failures as anti-vectors, shape-first recall), Capability you govern (skills
gated proposals / tools bounded+seated / workflows composed+named / loops
self-maintaining on evidence — measured before kept, reversible, leave a
lesson), One productive front end (the surfaces + the work-object-graph
connective tissue), A machine it respects (the resource contract); "How it
stays trustworthy" = the scientific method (plan=hypothesis, build=experiment,
gate=measurement, calibration=grading vs outcomes, recap=conclusion; "more
capable without getting less reliable"); "Where it is heading" = armada as the
same bet scaled (subordinate). Grounded in the feature docs read this turn
(memory.md provenance-as-permission + validated rung + dream pass;
self-maintenance.md every change a proposal measured before kept + reversible
+ leaves a lesson; calibration-loop.md decisions graded vs outcome evidence;
resource-contract.md measure/admit/settle/curtail, a breach is arithmetic).
STATE: the identity-first version + armada note were COMMITTED 9d9b6de and
pushed; the benefit-led reframe is UNCOMMITTED — commit when the owner is happy
(they are iterating tone/emphasis). The memory-pull feature was committed
separately at 2907d19 (see [[memory-pull-from-other-harnesses-idea-2026-11-11]]).

THE OWNER'S APPROVAL (2026-11-11, closing the iteration): "let's go with that
and we can evolve from there." The benefit-led reframe was COMMITTED 6f2d4af
("docs: the README, reframed around the why and the payoff") and PUSHED to
origin/master (9d9b6de..6f2d4af); tree clean. The README is now a SETTLED
foundation to EVOLVE from — not a draft awaiting a read. Future edits keep the
benefit-led shape ([[readme-leads-with-why-and-benefit-not-your-keys]]) and
iterate from this committed base; do not reopen the identity-first vs
benefit-led question (the owner chose benefit-led).
