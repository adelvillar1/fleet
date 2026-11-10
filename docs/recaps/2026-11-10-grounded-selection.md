# Session Recap — 2026-11-10: the grounded-selection program

**Summary.** The owner's exploration ruling — decision heads and small
models across the estate, all patterns together, fail fast — produced a
21-criterion plan and executed it to its honest terminal state in one
session: **9 criteria ticked with evidence, 2 kill-gates recorded, 2
per-seam gates run (one pass, one kill), the kill-gate over the
selection phases holding.** Eleven gated commits, every cleanup act on
the plan's own ledger, 112 GiB free at close.

## The arc

- **C0 — the baseline** (`703e28a`): `fleet tools audit --all` measures
  the archive — 26,556 calls over 764 logs: wrong-tool 0.9%,
  malformed-args 0.1%, repair 54.2%, and the number that matters: an
  **8,028-token declaration tax on every request** (~11% of the mean
  70.2k prompt).
- **C1 — the head-to-head and the kill gate** (`2c02ab6`): 120 recorded
  selection turns, three heads over each session's own catalog. Best
  head: drex 56.7% top-1, glide 51.3% with **margin AUC 0.787**. The
  gate fired: no head approaches the model's own 99.1% — **Phases 1-3
  (C2-C9) stop**, and stay open as the owner's re-openable record.
- **Phase 4 — the generation seam** (`e0bbcc9`, `f84a896`, `b37cd83`,
  `1635f31`): blend1's `generate` tool (LFM3-1.7B probed live; the
  1024-token reasoning floor measured), fleet's `src/generate.rs`, the
  fold through the lane with the routed path as named fallback, the
  end-of-run title/summary act with owner titles winning, the face's
  `lane_summary` field.
- **C11** (`89186d3` + the advisor leg): skill-proposal bodies and the
  tools-advise card narrate through the lane; three surfaces
  dispositioned on design authority and recorded on the criterion.
- **Phase 6** (`1c2f1b2`, `8dbb0b8`): gen1's images grade through sys1's
  vision_qc (the classify tool carries images over MCP now); a
  validation-failed remember/improve emission re-authors through the
  lane behind the full validation. **C18 killed on the probe**: Liquid
  serves no hosted embeddings — three 404s, a chat-only model list.
- **Phase 5 — the per-seam gates** (`89186d3`..): **C12 passed and
  landed**: the destructive head measures 1/1 recall with 76/77 safe
  precision where fleet's list catches 0/1 and false-flags 8 — the
  head's yes annotates through the ordinary gate, the list the
  zero-spend fast path. **C13's shape half killed**: 62% against the
  corpus's own labels. C14-C16 stay open behind named fixtures.

## The pattern the data taught

Same vendor, opposite verdicts per task: the heads are **dull at
selection** (reproducing the model's contextual choice from intent
alone) and **sharp at judgment** (yes/no destructiveness, calibrated
margins). The C1 kill gate did its job for pennies; the per-seam gates
found the one seam worth building.

## Cleanup (C20's ledger, carried through every phase)

No worktrees, no build lanes, no scratch venvs (one pytest venv, used
and deleted), the sweep run twice (124 GiB free at close, release
binaries untouched), every phase's disk cost recorded in the plan.

## Open

- **C2-C9** — killed by C1's gate; the owner re-opens with new evidence
  (the heads' calibration, glide AUC 0.787, is the recorded lead for a
  verifier-shaped pivot).
- **C13** mention-kind/silent-stop halves, **C14-C16** — open behind
  small hand-labeled fixtures (the designs are on the criteria).
- KI-515 (stale blend1 corpus tests) and KI-516 (the layerfs
  coherence test's schedule flake) filed en route.
