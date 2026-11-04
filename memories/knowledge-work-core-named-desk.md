---
name: knowledge-work-core-named-desk
description: "Owner ruled 2026-11-04 — the knowledge-work core (the brief→claims→outline→render→review→knowledge loop) is named desk, not deliverables"
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

The owner ruled on 2026-11-04 that the knowledge-work core's surface is
named **desk** — "kind of where knowledge work happens in the real
world". My earlier recommendation was `deliverables`; the owner's name
won because it names the PLACE (the loop), keeping `deliverable` free as
the ARTIFACT term (the way huddles keeps "room", design keeps "frame").

Consequences: route `#/desk`, module `src/desk.rs` (verified no
collision), feature doc `docs/features/desk.md`. Known adjacency to
watch: **desk/deck are one letter apart** and `deck.rs` / deck cards /
deck events already exist — the plan must keep the two words
unmistakable (rail icon, harness keys, prose).

The handoff brief behind this (verified against the tree the same day:
extract.rs's jobhound floors 0.85/0.35, deck.rs's refusing checks,
documents.rs Route enum, canvas.rs project.json, presenter role) is
pre-plan. Two design holes the plan must answer:
1. prose deliverables have no refusing checks — deck.rs's law
   (Pyramid/MECE/bolding) is presentation-shaped; the document render
   target needs its own check vocabulary;
2. URLs listed as brief inputs vs the no-network local extraction path —
   v1 boundary needs a ruling.

**Why:** the name is an owner-level ruling, not derivable from code or
git; the next session drafting the plan must not re-litigate it.

**How to apply:** the plan is DRAFTED and AMENDED —
`docs/plans/2026-11-04-desk.md` (status draft, 20 criteria C1–C20), the
lane work sized by the same-day deep dive
`docs/architecture/lanes-deep-dive-2026-11-04.md` (all four sibling
repos + jobhound read in primary source). The lane mapping: sources via
documents/extract.py; claims via extract.rs + claim-kind labels + a
batched verifier (per-mechanism floors, chunked at the caps); coverage
as one vendored sys1 task with the judge beat's spend cap; figures via
sdm1_classify + the canonical-number guard; drafting via blend1 ask
(gate + number check ride free); corpus via blend1 seed/refresh/search;
render review via the existing vision tool on pixels. Known seams: sys1
MCP exposes ONE tool (rich shapes are HTTP/task_spec; Fleet's vendored
tasks.py is the seam — tool_route/curate precedent), gen1 voice legs
are library-only, Fleet's vendored sys1 predates clef, perception.py is
Fleet-tree-only. Gates: arch-gate PASS zero drifts; plan-gate's last two
flags (C15/C19) are judgment-band variance — disposition at
approval. **The owner's project-knowledge ruling (2026-11-04, later
same day):** desk work binds to a workspace project and its folder;
deliverables become integral project knowledge — indexed and embedded
through the EXISTING substrates (memory recall, blend1
seed/refresh/search; no new store); skills reach it via assembled
context + recall + the shapes vocabulary; huddles compose both ways
(reference a desk project; hand a room's deliverable to the desk).
Verified precedent: canvas's binding law (canvas.rs:211-221) and K6
designs-are-project-knowledge (canvas.rs:505-509, the bounded
assembled-context section). **The universe ruling (2026-11-04, final
steer of the day):** the project is the universe; sessions, design,
huddles, desk are WHERE work happens and HOW you gain access, not
containers of truth; artifacts originate in one surface and are invoked
in others (the owner's example: a frontend design originates in
design, is invoked in a huddle, ends up in a desk presentation). Landed
as: the universe statement in the plan's Direction + design (references
are not copies — deck.rs's image engine is its own "later"; the
reference card renders beside the pane's live artifact), C20 widened to
the surfaces' seam (into the desk / out of the desk / the hand), and an
explicit out-of-scope: no cross-surface artifact registry in v1 (the
read seams — K6 digests, session recall, the desk store — are the v1;
a registry is its own plan if the owner wants one). **The shapes answer
(owner asked 2026-11-04):** half-built — the compiled-in taxonomy has 7
structure shapes (deploy, build, deps, docs, rendering, state-sync,
change) + 6 failure shapes (incl. claim-outran-evidence — the desk's
law already named), facts carry both axes, roles narrow recall by
shapes; skills carry NO shape (C15 adds it). Landed in C15: the desk's
kinds (presentation, document, analysis) join SHAPE_KEYWORDS as ONE
vocabulary (the homonym failure at memory.rs:47 forbids a second
table), and the desk project records the shape its brief routes under —
that selects the methodology skills; when NO shape routes, the library
ranks through the tool router's sem1 seam (skills were the one corpus
with no embedder — memory's vector channel and session_recall's index
are the precedents), lexical fallback named. Next: owner reads
both docs, then status → active and mockups first. See also
[[a-new-plan-must-answer-the-past-plans]].

**Implementation status (2026-11-04, the build session):** both plans
ACTIVE (owner's /goal: implement). The desk's Rust core is LANDED on
face-plugin-host: the store + document law + digest + context section
(21b5243), the claim set — batched narrowing at 0.70, kind labels, the
0.85/0.35 floors (c4b4e36), the coverage beat — vendored desk_coverage
task (3.9-safe score head: make_score's zip(strict=True) breaks the
system python — the blend1 guard class), judge-path ask, the $0.05/day
spend cap (2dbbc0b), the number leg — sdm1 figures + the canonical
number guard (5ddaa3c), the shapes vocabulary — skills read shapes:,
SHAPE_KEYWORDS gained presentation/document/analysis appended last so
the order rule stands, shape-first discovery + lexical fallback
(3e4e7eb), the blend1 seed leg + the fleet desk card (5a77c0f, 645ae78).
Mockups committed 5a26137 — AWAITING THE OWNER'S READ (C1 gates face
code). Gate green: lib 1,866/0, integration 165/0, clippy 0, release
build ok. Remaining: the stage runner, the face (gated), the huddles
seam, harness+live proof+delivery+docs, then the transplant plan whole.
