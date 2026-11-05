---
name: unlocked-ki-items-shipped
description: "2026-10-24 evening — the mechanical KI backlog worked in one session (plan 9/9, gate 1671 green) — deck pptx+fragment doors, partial-embed pin, smoke guard in the orientation, the intent skill's schema boundary, blend1's named-env refusal upstream+vendored, embed_timing calib rows; ledger at 294 closed / 2 open / 15 noted"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The unlocked-items session (plan `docs/plans/2026-10-24-unlocked-ki-items.md`,
9/9 gate PASS; recap `docs/recaps/2026-10-24-unlocked-ki-items.md`; commits
6968032→fa06552) worked every mechanical row the owner-input audit left:

- **Deck doors (KI-239/241)** — `run_deck_with_event` now takes the session
  directory (the agent passes the session log's parent): the rendered
  fragment lands beside the session as `deck-<stamp>.html` (house
  `compact_utc_stamp`), `format: "pptx"` writes a real `.pptx` via the
  tested `to_pptx`, and both paths ride the summary the model reads; a
  failed write names itself and the deck still ships. The GUI's Work::Deck
  card is unchanged beside it. Grader note: a one-node deck is REFUSED —
  use the known-good two-node split_two_column+metric_row fixture in tests.
- **Partial embed pinned (KI-268)** — the rank route's scoring extracted to
  `bridge::rank_with` (route hands it the probed embedder; tests hand a
  double): a candidate whose embed fails rides at exactly 0.0, scored leads,
  descending holds.
- **Smoke guard (KI-066)** — a standing sentence in
  `docs/harness-orientation.md`'s environment section: assert the mock's
  port file and fail closed before any unattended `--yes` run; pinning the
  candidate alone already fell through once.
- **Intent skill boundary (KI-240)** — `visual-deliverable-brief` (the
  owner's ~/.agents/skills copy) now says research is for FACTS, never for
  the tool — the `deck` declaration is authoritative, reading `deck.rs` is
  waste; the 25-turn spiral started exactly there.
- **blend1 named-env refusal (KI-310)** — the adapter's refresh leg catches
  the absent sem1 core and names `BLEND1_SEM1_PYTHONPATH` +
  services/sem1/src; description declares the dependency. Upstream main
  (1a53246 + 45c6933 — the first push ran red on a venv with sem1 installed;
  the test now probes the child interpreter and skips naming why), fleet's
  vendored copy byte-identical (fa06552). The test needs sources that EXIST
  — a no-source refresh returns before the sem1 import and answers success.
- **embed_timing rows (KI-249, half done by design)** — `OP_EMBED_TIMING`
  on the calibration stream (kind corpus-load/probe, ms, fact count;
  redacted), wired at the run path and the scheduled load, pinned at the day
  file (ReadRow has no embed fields — the reader grows when a consumer
  exists). The bounds move only after a day of measured tail; the row stays
  open saying exactly that.

**Still open by design:** KI-249's second half and KI-252's gated blueprint
moves (Matryoshka pre-filter, asset library — each its own plan). Ledger:
294 closed / 2 open / 15 noted over 311 rows. Also closed on evidence:
KI-190 (`e1796da` fixed it nine days earlier — see
[[ledger-rows-lag-their-fixes]]).

**The evening close-out and the two contracts (commits through c5fd03f).**
The owner answered every remaining ask: KI-136 verified in a Linux docker
container (the quick start's literal sequence in `rust:1` — the only FAIL
the unset DeepSeek key, an account matter), KI-192 ruled out (no
disposition guard; the append-only design stands), and "commit pending
changes, merge, push, build" → blend1's adapter + corpus-carries branches
merged to main and pushed (`c76b2eb..4db80e1`, suite green on the merge),
fleet's pending corpus/doc changes committed and master pushed
(`6be8096..a74eb6a`). Then both remaining open rows were planned as
drafted, gate-PASSed contracts (c5fd03f): KI-249's second half
(`2026-10-24-embed-bounds-from-measured-tail.md` — window-gated on ≥20
embed_timing rows over ≥24h, then `calib::embed_tail` derives
median/p95/max and each bound is set by the rule max×3, floored at 10×
the warm median, capped at the model's 600 s) and KI-252's moves 2–3
(`2026-10-24-deck-vector-moves-2-3.md` — the Matryoshka 128-dim
pre-filter with full-width top-K re-rank behind a default-off
`memory.vectors.two_stage` setting, decided by a recall@10 measurement
gate; the caption-keyed asset library with the grader certifying aspect
and whitespace before a match is placed). Both rows point at their plans.
Related: [[fleet-ki-triage-pass-one]],
[[external-repo-ki-closures-2026-10-24]], [[fleet-deck-visual-pipeline]].
