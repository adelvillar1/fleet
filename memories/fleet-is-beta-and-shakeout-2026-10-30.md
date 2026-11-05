---
name: fleet-is-beta-and-shakeout-2026-10-30
description: "Fleet is BETA as of 2026-10-30 (0.3.0-beta, 87e4071 pushed) — the owner declared it, the fact lives in the corpus, and the shakeout plan's pass one found KI-378 (the unbounded standing goal) and fixed it at the writer"
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

The owner's declaration, 2026-10-30: "I think we can call fleet in beta
stage. What I wanted in the app is there and looks and works fine. Time
to find bugs." Landed as `0.3.0-beta` (Cargo.toml, CLAUDE.md's status
line, and the fact `memories/fleet-is-beta.md` in the git-tracked corpus
with `basis: confirmed-by`), commit 87e4071, pushed.

**The work mode is now the shakeout** — fixes land before additions;
new features wait for a bug budget. The plan
(`docs/plans/2026-10-30-beta-shakeout.md`) is a HUNT MATRIX, not a
feature contract: surfaces × flows × adversarial classes (enormous
inputs, hostile characters, state collisions, boundaries, seams) ×
passes, with every find filed as a KI row before any fix.

**Pass one (2026-10-30) walked eight probes on the live face: one find,
seven held.** The find — **KI-378**: the standing goal was unbounded; a
10k-char paste through the goal chip was accepted and stored verbatim,
and the standing objective rides EVERY following run's system prompt (a
silent per-run cost, invisible because every surface clips the display
to ~42 chars). Fixed at the writer: `Bridge::GOAL_MAX_CHARS` (2,000) in
`set_goal`, the refusal naming the bound and why; pinned by
`the_goal_refuses_past_its_ceiling` (needs `#[tokio::test]` —
`Bridge::detached` requires a reactor). **The composer's task field is
the same class at one-shot scale** (a 131 KB paste held in the draft
and would send) — filed in KI-378's row as the OWNER'S triage.

Held probes (recorded in the plan's matrix): the 131 KB composer draft,
a 2 KB emoji/newline/HTML rename (served verbatim, restored cleanly —
an earlier 409 pair was the probe's own wrong shape: the route reads
`?session=` from the QUERY, not the body), regex+control chars in the
search, four rapid section flips mid-read, a pane card open across a
panel flip, the stepper four days back (the honest empty state), a
20k-char search.

Closes: suite 1,843 / 0 / 5, harness 474 / 51, ledger green (KI-378
open with its fix landed, awaiting the owner's deep-pass re-run).

**Deepened 2026-10-30 (7fa195e, pushed): the plan is now a
fleet-EXECUTABLE manifest** — the owner said "I'll run a deeper version
of this through fleet itself". The matrix became a hunt specification:
the hostile corpus defined once (H1–H10: the 100 KB giant, control
chars, regex/backtracking, path-y escapes, Zalgo/zero-width unicode,
JSON-where-a-string-belongs, hostile numbers, impossible days, the
empties), ~50 probes in eight groups (A the conversation's run loop, B
the sessions rail, C memory, D skills, E calibration, F workspace
history, G the seams — CLI+face on one home, two writers, restart
mid-flow, read-only home, UTC midnight, credentials absent — H the CLI
headless), each with its exact action and what "held" means. The
runbook for a fleet run: the task template ("walk the manifest pass N,
rows…, record verdicts, file finds"), isolation rules (restore
immediately or a scratch FLEET_HOME; a probe that can't restore skips
and says so), the model-spend budget (giant sends against the scratch
home's failover tail), and the ledger rule — the pass OWNS the ledger
during its walk (two writers filing at once is KI-285's shape).
Verdicts: held / find / skip; the TUI and phone rows are named skips
(a headless run cannot drive a terminal; the relay rows need the
owner's device or the live probe's role). B1's criterion now reads
"the manifest walked whole, no row unattempted without a named skip".
The next input is the owner's own use: whatever breaks becomes a row.

**The fourth-session recap and the cleanup (41e04e8, pushed)**: recap
at `docs/recaps/2026-10-30-fleet-is-beta.md` — the day's fourth (the
curator session kept its own). The tmp/ garbage went to the Trash:
60 entries older than a week plus `tmp/stray-20261030/` (which held
copies of the earlier top-level strays) moved to
`~/.Trash/fleet-tmp-cleanup-20261030` — reversible, tmp/ 27 MB/137 →
16 MB/77, the keeps intact (`tmp/relay-live-probe.py`,
`tmp/bot-dogfood.md`, this week's probes and the plans' evidence
shots). The top-level strays (home.txt, rr.json,
.tmp-corpus-refresh/) were already gone before the pass. The `.lock`
sidecars were deliberately NOT deleted — flock runtime state, and
unlinking a flocked file another process holds is the race the
gitignore's own comment names. Closes: suite 1,843 / 0 / 5, harness
474 / 51, ledger 378 rows / 33 open; tree clean, everything pushed
through 41e04e8.

**The shakeout's first real-use finds (2361023, pushed)** — the owner
hit the app for real and it broke twice: KI-379 (the ~30-min warmup
"hang") corrected into **KI-380, the zombie tab** — a page whose
server restarts never recovers (the warmup fetches fail into silent
catches, the SSE dies, nothing re-arms), so an approval seat
(`tool_search`, ask mode) re-stood on the resumed session where no
living page could show it, and the run hung for hours. Unstuck through
the API: `POST /api/approve?session=<id>` with decisions
always/yes/deny (`approve` is refused). Open: the reads re-arm, the
cue escapes the tab, the standing-rule write verified. See
[[ki-380-zombie-tab-2026-10-30]]. The fifth recap would be the beta
flip's own (41e04e8 covers through the manifest).
