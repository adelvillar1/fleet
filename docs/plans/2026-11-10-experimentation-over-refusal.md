---
status: active
created: 2026-11-10
updated: 2026-11-10
slug: experimentation-over-refusal
---

# Plan: experimentation over refusal — the sandbox-experiment gate

## Context

The owner's ruling (2026-11-10, rule 8 of the decision-head playbook):
refusal answers a prediction question; uncertainty plus reversibility
runs the outcome question. Fleet's destructive gate predicts from a
substring list + the C12 head (measured: 8 false flags per 77 safe
commands, 0/1 on the real one) and refuses/asks on the prediction. The
build: a destructive-flagged bash command in a git repo runs in an
isolated clone first, the outcome (status + diff) rides the tool
result, and the real repo stays untouched until the ordinary approval
gate — the owner approves the outcome, not the guess.

## Approach

`src/experiments.rs` (new): the clone-run-report machinery — size
guard, `git clone --local` to a scratch dir, the command runs with cwd
= the clone (bounded timeout, bounded output), the outcome captured as
`git status --porcelain` + `git diff` (bounded), the clone removed. The
intercept in `prepare_call` (bash + destructive + git repo + setting
on) returns the experiment report as the settled result — the model
sees the outcome and re-issues or adapts; the real run always crosses
the ordinary approval gate, whose semantics are untouched. Non-git cwd
or clone failure → the ordinary path, named.

## Acceptance criteria

- [ ] C0: A destructive-flagged bash call in a git-repo cwd produces an
  experiment report (status + diff from the isolated clone) as its tool
  result, and the real repo's status is byte-identical before and after.
- [ ] C1: A destructive-flagged bash call outside a git repo, or with
  the setting off, takes the ordinary approval path unchanged — pinned
  by a test.
- [ ] C2: A clone failure or an over-size repo skips the experiment
  with a named reason and takes the ordinary path.
- [ ] C3: The experiment command runs under a bounded timeout with
  bounded captured output, and the clone is removed after the report is
  captured.

## Files

- `src/experiments.rs` (new) — the machinery + unit tests
- `src/agent.rs` — the intercept before the approval gate
- `src/settings.rs` — the `experiments.destructive` key (default on)
- `tests/integration.rs` — the end-to-end pins

## Out of scope

- Host-path commands outside git repos (no sandbox) — the ordinary gate
- Non-bash tools; the layerfs child path (its own sandbox already)
- Unattended auto-apply — the real run always crosses the gate

## Verification

C0/C1/C2/C3 by test; the gate legs on every increment.
