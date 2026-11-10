# Selection — the decision-head seam

The grounded-selection plan's feature doc. One home for the seam's contract,
the measured baseline every criterion compares against, and the head-to-head
results. Status mirrors `docs/plans/2026-11-10-grounded-selection.md`.

## Baseline (C0, measured 2026-11-10)

The command:

```sh
cargo run --bin fleet -- tools audit --all
```

The numbers, over the whole archive — 764 session logs played, 0 skipped
(2 non-log entries in the sessions dir are not logs):

| Number | Value | Denominator |
|---|---|---|
| Tool calls | 26,556 | — |
| Failures (error/denied results) | 915 (3.4%) | calls |
| **Wrong-tool rate** | **0.9%** (235) | calls — refused resolutions |
| **Malformed-argument rate** | **0.1%** (28) | calls — `error[validation]` |
| **Repair-turn rate** (recovery after a failure) | **54.2%** (496) | failures |
| **Mean declaration tokens per request** | **8,028 est.** (chars/4; lane tools at the known-spec mean) | 9,321 requests declaring a tool set |
| Mean declared tools per request | 27.3 | same |
| Mean prompt tokens per request | 70,167 | 9,413 API-answered requests |
| Resolutions | exact 20,584 · refused 235 · retrieved 2 | resolutions recorded |

Reading the numbers:

- **The declaration tax is real and measured**: ~8.0k of ~70.2k prompt
  tokens per request — ~11% of every request is tool definitions, before
  the conversation. C2's injection attacks this number directly.
- **The model rarely names a wrong tool** (0.9%) and almost never
  malforms arguments (0.1%) — the emitted call is not the weak link.
  Selection quality beyond "the name parses" is what C1's head-to-head
  grades: whether a calibrated head can match the recorded choice.
- **Half of failures repair** (54.2%) — the other half died on the first
  error. C4's verify+repair leg targets that half: a head verdict before
  execution costs a forward pass instead of a flagship turn.
- **`retrieved` resolutions are 2 of 20,821** — `tool_search`'s two-stage
  selection almost never runs in practice. The deferred-tools machinery
  exists; the catalog-as-choice injection (C2) is where selection
  actually happens.

Method notes: the aggregate reads the logs alone — no model, no lane, no
network. Subagent logs are included (children's calls are real calls).
Declaration tokens are an estimate by design: chars/4 over the compiled
specs, lane tools counted at the known-spec mean since their schemas are
not compiled in. Re-run the command after each phase; the plan's criteria
compare against this table.

## The destructive head (C12, measured 2026-11-10)

The measurement (`scripts/c12-destructive-measure.py`): 78 real bash
commands from the archive — every command the substring list flags (59)
plus a stride sample of unflagged ones — hand-labeled on one rule
(destructive = destroys work that is not trivially recoverable). One
command in the set is destructive; the rest are temp-file, lock, and
probe cleanup.

| classifier | destructive recall | safe precision | false flags |
|---|---|---|---|
| **GLiDE yes/no head** | **1/1** | **76/77** | **1** |
| fleet's `DESTRUCTIVE` list | 0/1 | 69/77 | 8 |

The head catches the one destructive command (a tracked script removed
and the removal pushed) that the list cannot see — `git rm --cached`
carries no marker — and false-flags one safe command where the list
flags eight. The label distribution is extreme (1 destructive / 77 safe)
and the labels are one rater's; the honest claim is narrower than the
table looks: **the head adds detections at comparable-or-better
precision on real traffic**, which is the per-seam gate C12 needed.
Landed as the pre-pass in `execute_calls`: the list stays the fast path
(a listed command never spends), an unlisted bash command asks the head
through an inline `task_spec`, and a `yes` annotates the call so the
ordinary gate asks. A dead lane changes nothing.

## Head-to-head (C1, measured 2026-11-10)

The command:

```sh
python3 scripts/selection-headtohead.py --sample 120 --providers glide,drex,liquid
```

120 recorded selection turns (deterministic stride over the archive in name
order), each asked of every head over that session's own declared catalog
(mean 27 options). Ground truth: the recorded choice, when the call resolved
`exact`/`alias` and its result did not fail. The head sees the intent alone —
the owner's last message plus the assistant's step so far — a deliberately
harder task than the model's context-full baseline.

| head | answered | top-1 | top-3 hit | accuracy | margin AUC | errors |
|---|---|---|---|---|---|---|
| drex | 120 | 68 | 75.8% | 56.7% | 0.756 | 0 |
| glide | 119 | 61 | 76.5% | 51.3% | 0.787 | 1 |
| liquid | 119 | 34 | 55.5% | 28.6% | 0.568 | 1 |

**The kill gate fires.** The plan's rule: a head must beat baseline
selection to carry a runtime phase. Baseline selection accuracy is 99.1%
(C0: 0.9% wrong-tool with full context); the best head reproduces 56.7%
of recorded choices from intent alone, and its top-3 — the number C2's
injection actually needs — is 76.5%. Injection as specified would leave
roughly one turn in four without the tool the model wanted, regressing the
wrong-tool rate far past the baseline's 0.9%. **Phases 1, 2, and 3 (the
head-consuming selection phases) do not proceed.** This is the gate doing
its job: one day of measurement, no runtime change, and the honest answer.

What the data still says, recorded for any future pivot:

- **The heads are calibrated even where they are not sharp.** Glide's
  margin AUC is 0.787 — its top-minus-second probability separates its own
  right answers from its wrong ones. Drex carries the best top-1; glide
  the best calibration. A verifier role (C4's shape: score the emitted
  call, not pick the tool) is the role this data supports, not selection.
- **Liquid d1 is the wrong head for tool selection** (28.6% top-1, AUC
  0.568 — barely better than chance at ranking). Its strength elsewhere
  (vision_qc, needs_generation) is untouched by this result.
- **Phase 5's heads are different tasks** — destructive yes/no, shape
  classification, silent-stop — none of which this replay measured. The
  plan gates each of those seams on its own pinned test (the head must
  beat the word list), so they proceed only as fast as their own
  measurements prove them.
- Phases 4 and 6 were built independent of head sharpness and proceed.
