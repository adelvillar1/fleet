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

## Head-to-head (C1)

Pending — the offline replay grades GLiDE, d1, and Drex against the
recorded choices. Results land here with the kill-gate decision.
