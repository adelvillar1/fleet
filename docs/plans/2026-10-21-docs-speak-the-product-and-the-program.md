---
status: active
created: 2026-10-21
updated: 2026-10-21
slug: docs-speak-the-product-and-the-program
---

# The docs speak the product and the program

## Context

The three public documents were written in a developer's register for a
developer's moment, and the product has outgrown them:

- **README.md** (155 lines) opens with status badges and `cargo` commands
  — the how — before saying what Fleet is. It reads as a repo README for
  a repo that is now an app someone lands on.
- **FUNCTIONAL-SPECIFICATIONS.md** (150 lines) is a command reference
  with an error vocabulary: it documents the CLI, not the product.
- **TECHNICAL-DOCUMENTATION.md** (399 lines) is accurate and complete,
  but organized as a crate tour rather than as the system-of-record a
  technical program manager would run the project from.

Owner rulings (2026-10-21): the README and the functional specification
speak the language of a **product manager** — the what and the why, the
goal, the intent, and the solution, never the how ("the how, it's in the
code"), never the growing pains ("we are past that"). The technical
documentation speaks the language of a **technical program manager**.
The understanding work is done and committed:
[docs/architecture/surfaces.md](../architecture/surfaces.md) — the what
and why of all eleven rail destinations, read live — and
[docs/architecture/lanes.md](../architecture/lanes.md) — why five
specialized services stand between the owner and the language model.
This plan rewrites the three documents against those two substrates.

Two constraints shape the rewrite:

1. **`arch-gate` judges every future plan against
   TECHNICAL-DOCUMENTATION.md** — the module reference is its substrate.
   The TPM framing goes around it, never through it.
2. **The recap step appends to both contract documents every session**
   ("summary developer-onboarding contract", "user-flow contract"). The
   two append points — FUNCTIONAL-SPECIFICATIONS.md's `## Run behavior`
   and TECHNICAL-DOCUMENTATION.md's `## Module reference` — must survive
   the rewrite in appendable shape, or the
   plan → build → recap → document loop breaks.

## Approach

One narrative spine for all three documents, which is the project's own
arc: **we had an idea, we experimented, found a possible solution,
tested it, and here is where we are.**

- **README (PM):** open with what Fleet is and who it is for, then the
  arc — the belief (a self-contained agent under the owner's own keys
  and home), the experiment (absorb the ecosystem: judgment, tabular,
  semantic, media lanes; calibration; a router the app owns), the
  solution (the LLM speaks last; lanes decide, measure, index, render;
  memory reads; proposals never apply silently; autonomy is earned on
  evidence), the testing (a local gate of ~1,500 tests, plan-gate,
  evidence-gate, the eval corpus, calibration grading), and where we are
  (alpha; what ships today; what is next). Then the shortest possible
  start, and pointers — the architecture notes, the plans, the recaps.
- **FUNCTIONAL-SPECIFICATIONS (PM):** the product as its user meets it,
  in sections: `## What Fleet is for` (the product in one page) ·
  `## The surfaces` (the eleven destinations as capabilities and flows,
  each with what you do and why it exists) · `## The commands as verbs`
  (what the user asks for, not a flag reference) · `## Run behavior`
  (kept: how a run proceeds, approvals, the turn cap, compaction,
  refusals — the recap step's append point) · `## What Fleet tells you
  when` (the error vocabulary, in the user's terms) · `## Known gaps` (the
  backlog, named).
Each rewritten document links its substrate — the surfaces note, the
lanes note, or both — and each section that states a product fact names
its source inline, so a reader can trace every claim rather than trust
it.

- **TECHNICAL-DOCUMENTATION (TPM):** the system of record for running
  the project, in sections: `## System overview and boundaries` ·
  `## Interface contracts` (CLI, the face's HTTP surface, MCP lanes, the
  home's files) · `## Environments and dependencies` (keys, lanes, the
  embedding server, pairing, the LaunchAgent) · `## Operational runbooks`
  (the gate, rebuild-and-restart, the cache sweep, pairing, the vault) ·
  `## Verification and release` (plan-gate, evidence, arch-gate,
  docs-gate, eval) · `## Risk register` (the open ledger, themed) ·
  `## Sequencing` (the in-flight plans) · `## Module reference` (kept
  verbatim beneath: the arch-gate's substrate and the recap step's
  append point).

## The section map

What each document's `##` headings are today and what they become. Two
headings are kept because the recap step appends to them:
FUNCTIONAL's `## Run behavior` and TECHNICAL's `## Module reference`
(the latter verbatim, as the arch-gate's substrate).

| Document | Today | After |
|---|---|---|
| README.md | Status · Build and test · Quick start · Commands · Configuration · Sessions · Source layout · Platform support | What Fleet is · The arc · What ships today · What is next · Start here · Where things live |
| FUNCTIONAL-SPECIFICATIONS.md | Command reference · The browser face · Run behavior · TUI modes · Background jobs · Eval corpus families · Error vocabulary · Known functional gaps | What Fleet is for · The surfaces · The commands as verbs · **Run behavior (kept)** · What Fleet tells you when · Known gaps |
| TECHNICAL-DOCUMENTATION.md | Crate contract · Environment contract · Module reference · Known documentation gaps | System overview and boundaries · Interface contracts · Environments and dependencies · Operational runbooks · Verification and release · Risk register · Sequencing · **Module reference (kept verbatim)** · Known documentation gaps |

## Acceptance criteria

- [ ] **C0 — the README is a product document.** It opens with what Fleet is and for whom, states the arc (idea, experiment, solution, tested, here), names what ships today and what is next, and contains no build, command, or crate-layout content before that story is told. Commands and build facts move to a later section or are linked from CLAUDE.md.
- [ ] **C1 — the functional specification is written in the user's terms.** It covers all eleven surfaces as capabilities (each with what you do and why it exists, citing the surfaces note), the CLI as user verbs, and the error vocabulary as what Fleet tells you when; it ends with the named gaps as a backlog.
- [ ] **C2 — the technical documentation is written for a technical program manager.** Its `##` headings after the rewrite are exactly the section map's "after" column for that file — System overview and boundaries · Interface contracts · Environments and dependencies · Operational runbooks · Verification and release · Risk register · Sequencing · Module reference · Known documentation gaps. The Module reference section's diff against `HEAD` is empty (`git diff HEAD -- TECHNICAL-DOCUMENTATION.md` touches no line under it), and `dev-decisions arch-gate` on a plan written against the new document still reports its modules documented.
- [ ] **C3 — the loop survives the rewrite.** The section map's two kept rows hold after the rewrite: FUNCTIONAL-SPECIFICATIONS.md carries `## Run behavior` and TECHNICAL-DOCUMENTATION.md carries `## Module reference` with its eight `###` groups unchanged — the two append points the recap step writes into — and `dev-decisions docs-gate` exits 0.
- [ ] **C4 — nothing is invented.** `grep -rl` for `docs/architecture/surfaces.md` and for `docs/architecture/lanes.md` prints exactly three filenames each — README.md, FUNCTIONAL-SPECIFICATIONS.md, TECHNICAL-DOCUMENTATION.md — and each rewritten section that states a product fact carries its source in a parenthetical (a feature doc, a ledger row, or a plan).

## Files to be touched

`README.md`, `FUNCTIONAL-SPECIFICATIONS.md`, `TECHNICAL-DOCUMENTATION.md`
(the three rewrites), `CLAUDE.md` (pointer index and the doc homes),
`docs/harness-orientation.md` (the cycle's document step),
`docs/plans/README.md` if the status vocabulary moves.

## Out of scope

The feature docs (they are the how, and the recap step maintains them).
The architecture notes just written (they are the substrate, linked
from). Any code. The known-issues ledger. The plans and recaps.

## Verification

CLAUDE.md's pointer index and `docs/harness-orientation.md` name the
three documents' new homes; no document carries a date-stamp marker;
all three remain English-only; the recap walks C0–C4 against the
committed files with the source of each section named.

`docs-gate` passes after the rewrite; `arch-gate` on this plan still
finds the modules it names; the owner approves the three mockup sections
above before any line of the three documents is rewritten; and the
recap of this work walks C0–C4 against the committed files.

## Linked artifacts

`docs/architecture/surfaces.md`, `docs/architecture/lanes.md` (the
substrate). `CLAUDE.md`'s housekeeping protocol (one home per fact).
The owner's rulings of 2026-10-21 (registers; what and why; goal, intent,
solution; no growing pains).
