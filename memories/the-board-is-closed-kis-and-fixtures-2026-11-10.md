---
name: the-board-is-closed-kis-and-fixtures-2026-11-10
description: "project 2026-11-10 (late) — the open-items pass CLOSED five KIs (515/516/476/494/459) and the contract-doc debt; the fixture campaign ran (extract → self-label → score: all five seams beat their string rule, but the labels are MINE not the owner's); the per-item score variant was abandoned UNRUN once the owner asked why we keep spending cycles on cost"
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

After the triage close and the 0.6.26 deploy, the owner said "let's try to
close those open items." Five KIs closed in one pass, each with evidence
(ledger `ki_ledger.py` green, open count 12 → 8 pre-existing rows):

- **KI-515** (blend1 corpus tests unpacking 4 of 5 values): updated the
  three sites plus one stale `_source_files` 2-of-3; the whole blend1
  suite runs **26 passed**. (It had been fixed the prior evening; the
  LEDGER ROW still said open — closing a fix means closing its row.)
- **KI-516** (the layerfs coherence test failing alone): two hypotheses
  were checked and RULED OUT first — a sibling test's stub (every layerfs
  test already has its own pid-keyed argv path) and the thread-local
  project override (`test_project()` has exactly ONE reader, the
  delegation in subagents.rs). The real cause: **the parent's folder
  registration walks the working tree through the same stub BEFORE the
  child runs, and the assertions used a whole-log `find`, so `read`
  matched a pre-child occurrence and `unmount < fs read` failed on
  traffic the test never meant to assert.** Fix: hold AMBIENT_LOCK across
  the assertions AND scope the sequence search from the child's first
  `fleet-child-` line. Proof: 3 consecutive runs alone green, then the
  suite 171 passed, then the gate 2,357.
- **KI-476** (Send button's white label under AA on both dark themes):
  measured at 3.06 and 2.93 against a 4.5 floor; darkened each dark
  gradient to its **minimal compliant stops** (console rgb(76,113,201)/
  rgb(52,88,182) = 4.65/6.53; graphite rgb(73,115,199)/rgb(48,87,181) =
  4.60/6.65; paper 4.81 and contrast 9.92 already cleared). The pin is a
  Rust test that PARSES app.css itself and walks every `rgb(…)` span —
  splitting on commas cuts the triples apart. **Red-proved**: restoring
  the old token names "white on (96, 143, 255) is 3.06 against a 4.5
  floor".
- **KI-494** (two enabled MCP servers with no working command):
  `computer-use` pointed at a deleted `~/Downloads/Codex Computer Use.app`
  path, `fli` died on ImportError. Both **disabled through the registry
  writer** (`fleet mcp disable <name>`); fleet's own desktop driver
  answers for computer use.
- **KI-459** (the two-stage recall benchmark timing the embed, not the
  rank): added the seams `partition_with_vector` / `recall_with_vector`
  and hoisted each instruction's embedding OUT of the timed region.
  **Second cause found:** `probed_embedder()` answers KEYWORD-ONLY on its
  first call (the probe runs in a background thread) — the measurement
  must use `probed_embedder_ready(timeout)` or the vectors are empty and
  the cost half is undecidable a second way. Result, median-of-7 over 10
  instructions on the live 700-fact corpus, three runs identical:
  **single 249–252 ms vs two-stage 232 ms (~7.5% cheaper) but mean
  recall@10 agreement 0.870 against the 0.98 gate** → `memory.vectors.
  two_stage` stays OFF — the speed is real and does not buy a different
  answer. Landed as a substrate fact.

**Contract-doc debt paid:** TECHNICAL-DOCUMENTATION §Interface contracts
gained the four seam contracts (sys1_classify incl. the 2–3-head
practical envelope, blend1's `generate` incl. the 1024 floor, the
experiment gate, the repair leg and why it is off); the
FUNCTIONAL-SPECIFICATIONS gained the five owner-visible behaviours
(fold-via-lane line, auto title+summary, the EXPERIMENT report, the
destructive-head flag line, the vision-qc verdict). CLAUDE.md's stale
state (0.5.0-beta.1 / 1,013 tests) corrected to 0.6.26 / 2,357.

**The fixture campaign — extraction, self-labeling, scoring:**
`scripts/extract-fixtures.py` → `~/.fleet/fixtures/` (home, never the
public repo) and `scripts/score-fixtures.py` grades a head against the
filled last column beside the string rule. Parser lessons: skill
`description:` values may be **YAML block scalars** (`>` on the
description line, real text on the following indented lines) — a naive
grep captures the `>`; C16's three surfaces need a RUNNING counter (each
section restarted at 1) and a cross-vocabulary **positive/negative
axis** (worth/stale, high/low, ready/not) or a literal-label compare
scores 0%. Re-extraction **overwrites the labeled files** — relabel
after any re-extract.

**With no owner labels available I labeled the fixtures myself** —
explicitly flagging which calls are mechanical (reports vs next-step
prose, literal duplicate skills) and which are mine alone. Result
(`score-fixtures.py glide`): mention 83% vs rule 17%, silent 80% vs 65%,
skills 90% vs 70%, recall 68% vs 13%, queues 100% vs 75% — **all five
seams "BUILD"**. **CAVEAT that must travel with these numbers: the
labels are mine, not the owner's; a head measured against the author's
own judgments is a system-agreement measure, not ground truth. C13–C16
stay FURTHER EVALUATION until the owner's labels land.**

**The per-item score variant was abandoned UNRUN.** The owner asked "why
are we spending cycles on cost? did I ever mention that as a concern?"
— they had not. Cost was never on their list (reliability, grounded
decisions, experimentation, cleanup, closing the board were). Response:
deleted the unrun `scripts/c-score-peritem-measure.py` (it had a "6x the
noul shape" cost line bolted on), and stripped the Cost-delta section
from docs/features/generation.md. **Lesson: do not introduce a metric
the owner never asked for and dress it as rigor; measure what they
named.**

**Face/deploy state re-verified in this pass:** the 0.6.26 release is
complete and is the auto-update prereq (`gh release view` not
draft/prerelease, 6 assets; `curl -sL` on latest.json → 0.6.26 /
darwin-aarch64; a no-`-L` curl returns EMPTY through the redirect); the
face was still pid 86100 on 0.6.24 and `fleet reload` re-execed it to
pid 21082 on 0.6.26; `/api/state` then read `folder: None` (KI-492's
silent-refusal shape) and was granted with
`POST /api/folder {"folder":"/Users/alejandrodelvillar/Projects/fleet"}`
— **the face's port is 3080**.

Related: [[board-state-closeout-2026-11-10]],
[[grounded-selection-expanded-tier123-2026-11-10]],
[[seat-decision-buttons-duplicate-their-labels]],
[[experimentation-over-refusal]].
