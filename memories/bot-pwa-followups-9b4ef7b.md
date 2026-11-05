---
name: bot-pwa-followups-9b4ef7b
description: "2026-10-28 follow-up commit 9b4ef7b ("let's address the issues") — per-turn persona stamps in Assistant records, the owner's goal verb admitted remotely with a bot affordance, KI-329 closed (absolute-path seed + cron seed reconcile), KI-340 closed (signal over clock); gate 1,770 tests, ledger 17 open"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28, owner: **"let's address the issues"** — the issues the
bot-PWA build ([[bot-pwa-implemented-2976098]]) had surfaced, landed as
**`9b4ef7b`** the same day.

- **Per-turn authorship** — the agent resolves the session's lead persona
  ONCE per run from the log's own records (start set folded with joins −
  leaves, the summary's derivation) and stamps every `Record::Assistant`
  it writes via the new tolerant `persona: Option<String>` field. The
  replay card carries it; the bot's byline reads the record, not the
  header. A pre-P2 log carries no field and the summary's fold still
  names its author.
- **The goal change, answered with the explicit verb** — `/api/goal`
  joined the remote allowlist and the bot's goal line is tappable; the
  sheet restates the ruling (the owner's change is recorded; a persona
  joining or leaving never touches it; a changed shape deserves a new
  conversation). The next run stamps a `Goal` record, so the change is
  reconstructable (hard rule 5). This answers the plan's "new
  conversation vs goal-change event" question: both exist — the owner's
  verb is the recorded change, and joins still cannot move the goal.
- **KI-329 closed** — the seeded lifecycle-sweep names the binary where
  the machine builds it: `$HOME/Projects/fleet/target/release/fleet tools
  sweep --apply` (bare `fleet` is not on PATH, KI-007; `$HOME` expands at
  run time so the declaration stays home-shaped). Key mechanics: the seed
  manifest is the DECLARATION and `seed_missing` reconciles existing
  jobs' task strings + tools to it (schedule/enabled untouched, KI-277's
  path), so a manifest fix plus `fleet cron seed` updates the LIVE store
  at `~/.fleet/cron/jobs.json` — verified reconciled the same day.
- **KI-340 closed** — the flake test waits on the thread's own end event
  instead of a wall clock (see
  [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]], form
  13).
- **No code needed** for the other two named questions: pre-P2 sessions
  read *unassigned* (no persona invented; the inbox files them under
  fleet itself) and shared conversations file under the lead — the
  implemented behavior IS the answer, recorded in the plan's risks.

Gate green by exit code (fmt/clippy/build/1,770 tests); ledger 340 rows
/ 17 open, checker clean. The remaining open rows predate the bot work:
KI-325 (embed saturation = the embed-bounds plan's window, still
accumulating), KI-332 (remember-during-task recall — needs a design
ruling), KI-339's deeper half (a refresh report naming the cwd — a
blend1-side change). Related: [[one-goal-many-personas-destructive-is-the-only-gate]],
[[assistant-record-field-cascade]].
