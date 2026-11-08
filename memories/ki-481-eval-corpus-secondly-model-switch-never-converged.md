---
name: ki-481-eval-corpus-secondly-model-switch-never-converged
description: "KI-481: the face's built-in eval-corpus cadence re-ran the 21-scenario corpus every second instead of daily because an empty persisted model read back as None and (Some(_), None) => true treated no-baseline as a model switch that never converged"
type: project
date: 2026-11-08
basis: confirmed-by
tier: durable
shape: [state-sync, resolved]
phrases: [eval corpus ran, eval-minute.txt, run_eval_if_due, read_eval_model, String::new()]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

While tracing a session the owner could not find in the history (the good-citizen / resource-responsibility work of 2026-11-08), a live defect was found running on the owner's machine: the face's scheduler was firing the once-daily eval corpus on every one-second tick. Mechanism: the face spawns the scheduler with String::new() as the served model (src/gui/bridge.rs:11959, blame 553b669 2026-11-07); run_eval_if_due's (Some(_), None) => true guard treats "no persisted model" as a model switch; the empty model writes a lone newline to ~/.fleet/cron/eval-model.txt which read_eval_model trims back to None — so "changed" stayed true forever. Measured: 9 full corpus runs a minute (log line count 5940→5943 over 20s), 7,075 run-lines in the gui log since Nov 7, 3,143 eval_corpus calibration rows for the day, 21,260 eval rows / 40 MB in the day's events.jsonl, ~24.6k fleet-eval-* scratch dirs (104 MB) in the system temp dir, created ~39/second at peak. Fixed 2026-11-08: the guard and the persist both filter the empty model (an empty model is not a baseline — it forces nothing and persists nothing); pinned by cron::tests::an_empty_model_is_no_baseline_and_the_corpus_fires_once (verified to fail on the reverted guard: left 5, right 0). The pre-fix binary still runs in the face until the app quits and reopens. Phrases that helped: `eval corpus ran`, `eval-minute.txt`, `fleet calib`. This row rides a session that was itself open: the good-citizen session (KI-477/478/479) has no recap — the trail is the commits 7a1fc00, ca633b6, eeff109, 95e0c0d, 068307c, 384852c.
