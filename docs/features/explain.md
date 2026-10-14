# The explanation beat: `fleet explain` (slice 17)

> Every verdict-emitting gate in Fleet names its failure without its
> cause. A red `fleet improve apply` says *which* of fmt/clippy/build/
> test failed; a regressing eval scenario names the check that did not
> hold; neither says what the failure **means** or what to change. The
> operator reconstructs the cause by hand, every time. This is the
> second beat: one ask to the lane, one card, one file of recoverable
> prose keyed to the judged artifact's sha.

## The surface

```
fleet explain --proposal <name> [--dry-run]
fleet explain --eval <scenario> [--dry-run]
```

The command reads the verdict's row from the calibration stream (the
same reader `fleet calib` uses), composes the ask from the verdict's
evidence plus the judged artifact, calls the blend1 lane once through
the MCP client, and renders a card. In the TUI, **Ctrl+X** opens the
same card for the latest explained verdict. An unknown selector is a
named refusal; an answer or a dry run exits 0, a named refusal exits
nonzero.

The verdicts covered today: the improve gate's verdict per proposal —
the failing or refused step with the per-step evidence, including the
refusal that fires most on a machine without rustfmt (KI-097) — and
the eval corpus's failed scenario, with the scenario's checks and the
first that did not hold.

## The discipline — the port, and the point

The pattern is dev-decisions' synthesis lane
(`~/Projects/dev-decisions/scripts/devdec/synthesis.py`), ported. The
sibling's rules, in the sibling's spirit:

- **blend1 proposes prose, the gate disposes.** The explanation is
  **eval-only**: it never gates, blocks, or applies anything. The
  verdict stands exactly as recorded, the row carries `eval_only: true`,
  and no verdict reads the `explain` op — there is no code path by
  which an explanation changes a gate's outcome. The socket-guard test
  is the proof: it reads the doctor, memory, agent, tools, session,
  calib, route, compaction, cron, improve, and boards sources and
  asserts none of them names this module.
- **batch-only.** No synchronous hook path references the beat — the
  doctor's boot checks, the recall path, and the tool loop's
  synchronous paths are the guarded set. The spend is real (~$0.0018
  per synthesized ask measured in the sibling, ~$0 on the direct path),
  so it is batched and never hooked.
- **`--dry-run` spends nothing.** The ask is not made; the card shows
  exactly what would be asked.
- **artifact integrity.** The explanation runs against the recorded
  artifact, and a sha mismatch with the row's `input_sha256` is a named
  refusal — explaining a drifted artifact is worse than not explaining.
- **recoverable text.** The prose lands at
  `<home>/explanations/<input_sha256>.md`. The row carries
  `explanation_sha256`, `explanation_chars`, the ask's route/validation
  telemetry, the cost, and the linked `input_sha256`. The session log
  and the calib row stay redacted: no prompt or answer text ever enters
  either — the model-visible record stays reconstructable without the
  lane's prose riding in it.
- **budget.** `EXPLAIN_BUDGET_USD_DAY = 0.50` (roughly 275 synthesized
  asks at the sibling's measured rate), with `FLEET_EXPLAIN_BUDGET_USD_DAY`
  able only to **lower** it. Exceeding the day's budget is a named
  refusal carrying the day's spend.

## What is different from the sibling

Fleet's beat is **native**: it runs through this crate's own vendored
blend1 lane over the MCP client, with no CLI call into dev-decisions
and no checkout dependency (C5's self-contained rule). The row's shape
mirrors the sibling's `explain` op — `input_sha256`, `explained_op`,
the route and validation telemetry, `explanation_sha256`,
`explanation_chars`, `cost_usd`, `latency_ms` — plus Fleet's
`eval_only` marker and the selector. The verdict rows gained the
artifact identity the beat keys on: the propose row carries the staged
content's sha, the eval-corpus row carries the corpus's own fixed
definition (every scenario's name and family in corpus order, under the
crate version) — so a corpus that changed since the row was written is
a drift the beat refuses rather than mis-explains.

## The artifact-drift guard on the apply (PS-C)

The same integrity rule rides the apply path: a proposal carries its
targets' content shas at draft time, and `fleet improve apply` refuses
a proposal whose target drifted since the draft — a named refusal
beside the existing dirty-tree guard (KI-103). Two regression tests
pin it: a drifted target is refused by name, and a still-matching
target applies through.

## The problem shape (PS-A) and its combinations

**"Why did the gate failed?"** — the naive path is the operator reading
the failing step's output and reconstructing the cause by hand, every
time. The explanation is the composed cause, and the discipline keeps
it from ever becoming a second verdict. Combined with **PS6** (the
claim that must be checked): a checked claim gains its cause, so the
verification's failure explains itself. Combined with **PS3** (the docs
that drifted): the drift's cause rides beside the drift the corpus
names.

## What is not built

- **PS-B, the calibration loop closing** — dev-decisions' judgment lane
  judging fleet's route rows and the judgments riding back into the
  router's selection. The next slice's subject.
- The doctor's beat (its artifact — the check report — is not yet
  hashed), the boards' explanation feed, and the semantic index over
  the stream (PS-D).
- A live TUI capture of the Ctrl+X modal: `tmux send-keys` delivers
  control combinations to a crossterm TUI as raw bytes on this machine
  (KI-163, reproducing on the pre-existing Ctrl+O), so the TUI surface
  is verified by its events tests and the in-process headless/TUI card
  equality rather than by a capture.
