# The docs speak the product and the program

Status: completed
Date: 2026-10-21

The owner's ruling: the README and the functional specification speak
the language of a product manager, the technical documentation the
language of a technical program manager — the what and the why, never
the how, never the growing pains. Built against
`docs/plans/2026-10-21-docs-speak-the-product-and-the-program.md`,
whose voice was approved in mockup sections before a line was rewritten.

## What changed

- **README.md** — from a developer README to a product document on the
  project's own arc: what Fleet is and for whom, the idea (a
  self-contained agent under the owner's own keys and home), the
  experiment (absorbing the ecosystem: lanes, calibration, an owned
  router, a plan-first methodology), the solution (the LLM speaks last;
  the lanes decide, measure, index, render; hallucination controlled by
  construction; autonomy graded on evidence), the testing (the local
  gate, the eval corpus, the four dev-decisions gates), and here — then
  what ships today, what is next, the shortest start, and where things
  live. Mechanics moved to pointers; no build command appears before the
  story.
- **FUNCTIONAL-SPECIFICATIONS.md** — from a command reference to the
  product in the user's terms: what Fleet is for; the eleven surfaces as
  capabilities with what you do and why it exists; the CLI as user verbs;
  Run behavior (kept — the recap step's append point) updated for the
  2026-10-21 rulings (the deepseek-flash tail, the leader-on-hard split,
  `--model` as a pin, the whole-hierarchy folder picker); the error
  vocabulary as what Fleet tells you when; the gaps as a backlog.
- **TECHNICAL-DOCUMENTATION.md** — from a crate tour to the system of
  record a technical program manager runs the project from: system
  overview and boundaries, interface contracts (CLI, the face's HTTP
  surface, the MCP lane contract, the home's files), environments and
  dependencies, operational runbooks (the gate, rebuild-and-restart, the
  cache sweep, pairing, the vault, cron), verification and release, a
  risk register drawn from the open ledger, and sequencing of the
  in-flight work. The module reference is kept beneath, its eight groups
  unchanged — with two entries corrected where today's rulings made
  their old sentences false (`route.rs`'s "DeepSeek serves only when
  pinned"; `subagents.rs`'s "a child runs on the parent's own routed
  client") and the gui entry extended for the folder picker's `abs`
  listing.
- **CLAUDE.md** and **docs/harness-orientation.md** — pointer-index and
  docs-row updated to the new homes.

## The criteria, walked

- **C0** — the README opens with what Fleet is and for whom, states the
  arc, names what ships and what is next; no build, command, or
  crate-layout content precedes it (commands and platform facts live
  under "Where things live" and `docs/features/install.md`).
- **C1** — the functional spec covers all eleven surfaces as
  capabilities (each citing the surfaces note), the commands as verbs,
  and ends with the gaps as a backlog.
- **C2** — the technical document's `##` headings are exactly the section
  map's after-column; the module reference's diff is limited to the three
  corrected entries; `arch-gate` on this plan verdicts PASS.
- **C3** — `## Run behavior` and `## Module reference` both survive with
  its eight `###` groups; `docs-gate` verdicts PASS (A0–A3 covered).
- **C4** — a grep for each architecture note's path prints all three
  filenames; sections that state product facts name their sources
  inline (the surfaces note, the lanes note, feature docs, ledger rows).

Gates: the Rust gate green (1489 passed, 1 ignored); `plan-gate` PASS
(5/5); `docs-gate` PASS; `arch-gate` PASS with conventions gaps — the
three rewritten documents are not yet described *inside* themselves,
which is the gap a future plan closes.

Sources per section: the README's arc from `docs/architecture/overview.md`
and the plans of record; the solution from
`docs/architecture/lanes.md`; the surfaces from
`docs/architecture/surfaces.md` (read live); the functional spec's
behavior from the feature docs and the surfaces' own copy; the technical
document's contracts from the module doc comments and the runbooks this
session ran (the gate, the kickstart, the sweep, pairing, the vault).

Cache sweep at recap: free space on the volume: 95.0 GiB → 95.0 GiB (release targets untouched)

## What is left

The conventions gaps `arch-gate` named: the technical document does not
yet describe its own new sections, so the next plan touching it should
fold them in. The dated rows in `docs/architecture/ecosystem.md` remain a
known documentation gap, recorded in the technical document.
