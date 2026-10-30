# Fleet is beta — and the shakeout begins

**2026-10-30 (fourth session)** · the owner's declaration — *"I think
we can call fleet in beta stage. What I wanted in the app is there and
looks and works fine. Time to find bugs"* — landed as `0.3.0-beta` in
the crate, the status line, and the corpus · the shakeout's plan
became a manifest fleet can execute · pass one walked eight probes on
the live face: one find (KI-378, fixed at the writer the same session),
seven held · and the temp garbage went to the Trash. The gate: **1,843
passed / 0 failed**, the face harness **474 / 51**, the ledger green at
**378 rows, 33 open** (KI-378 filed and closed in the same session).

## The session, in order

- **The beta flip** (`87e4071`) — the crate version, CLAUDE.md's
  status line, and the fact in the memories corpus
  (`memories/fleet-is-beta.md`, `confirmed-by` — the owner's own
  words). The work mode changes with it: fixes land before additions.
- **The shakeout's pass one** — eight adversarial probes on the live
  face: a 100 KB paste into the composer, a 10k goal through the wire,
  a 2 KB hostile rename (set and restored), regex and control
  characters in the search, four rapid section flips mid-read, a pane
  card open across a panel flip, the day stepper into the far past,
  and a 20k search. **One find, seven held.**
- **KI-378 — the standing goal was unbounded.** A 10,000-character
  paste through the goal chip was accepted and stored verbatim — and
  the standing objective rides EVERY following run's system prompt, a
  silent per-run tax invisible on every surface (they all clip it to
  42 chars). Fixed at the writer: `set_goal` refuses past
  `GOAL_MAX_CHARS` (2,000), the refusal naming the bound and why;
  pinned by `the_goal_refuses_past_its_ceiling`. The composer's task
  field is the same class at one-shot scale — filed in the row as the
  owner's triage.
- **The manifest** (`7fa195e`) — the shakeout plan deepened into a
  hunt specification fleet can run: the hostile corpus defined once
  (ten input classes), ~50 probes across eight groups (the run loop,
  the sessions rail, memory, skills, calibration, workspace history,
  the seams, the CLI headless), each with its exact action and what
  held means, plus the runbook for a fleet run — the task template,
  the isolation rules, the model-spend budget, the ledger rules, and
  the three passes.
- **The cleanup** — 60 stale tmp/ entries (27 → 16 MB) and the
  `tmp/stray-20261030/` copies moved to the Trash (reversible; the
  keeps — `tmp/relay-live-probe.py`, `tmp/bot-dogfood.md`, this
  week's probes and shots — intact). The top-level strays
  (`home.txt`, `rr.json`, `.tmp-corpus-refresh/`,
  `services/browser/generated/`) were already gone before the pass.

## Plans worked on

### `docs/plans/2026-10-30-beta-shakeout.md` — the beta shakeout

Status after this session: **active** — pass one is walked (the matrix
recorded in the plan), passes two and three await what the owner's own
use and the fleet-run manifest shake loose.

| Acceptance criterion | Status | Notes |
|---|---|---|
| B1 the manifest walked once, shallow, whole | ⏸️ partial | pass one: eight probes on the live face; the manifest's ~50 rows await the fleet-run pass |
| B2 every find is a row | ✅ met | KI-378 filed with its repro and evidence before the fix |
| B3 crashers and data-loss first | ✅ met | no crasher found; the standing-cost find fixed same session |
| B4 the deep pass re-runs the fixed rows | ⏸️ partial | KI-378's repro is pinned; the live re-run rides the next pass |
| B5 the beta ships | ⏸️ partial | no open crasher; the matrix walks on |

## Commits

| Hash | Message |
|------|---------|
| `87e4071` | chore(beta): fleet is beta — 0.3.0-beta, and the shakeout begins |
| `7fa195e` | docs(shakeout): the manifest — the plan fleet can execute |

## What was added

- **The beta marker** — the version, the status line, the corpus fact.
- **The hunt manifest** — the hostile corpus (ten input classes),
  ~50 probes with their exact actions and held-meanings, the runbook
  (the task template, the isolation and budget rules, the ledger
  rules), and the three-pass structure.

## What was fixed

- **KI-378** — the standing goal's unbounded write, bounded at
  `set_goal` with the refusal naming the ceiling; the pin runs the
  real bridge over a scratch home.

## What was changed

- **tmp/ pruned** — 60 entries older than a week and the stray-stash
  dir moved to the Trash (12 MB), the named keeps and this week's
  evidence intact; the tree is clean (tmp/ is gitignored scratch).

## Files changed

**Backend / API**
- `src/gui/bridge.rs` — `GOAL_MAX_CHARS` and the bound at `set_goal`; the pin

**Configuration / Infra**
- `Cargo.toml` — `0.3.0-beta`
- `CLAUDE.md` — the status line
- `memories/fleet-is-beta.md` — the fact

**Documentation**
- `docs/plans/2026-10-30-beta-shakeout.md` — the manifest, the runbook, pass one's matrix
- `docs/known-issues.md` — KI-378 (open → the fix landed in the same session; the row's triage notes the task-field follow-up)

## Open questions / next steps

- **The manifest's ~50 rows await the fleet-run pass** — the runbook's
  task template is the launcher; pass two goes deep on whatever shakes
  loose.
- **The task field's send bound** — the owner's triage (KI-378's row
  names it): a one-shot spend, not a standing tax.
- **The TUI and the phone rows** stay named skips until a terminal
  driver or a device is at hand.
- **The Trash** holds the cleanup's 60 entries
  (`fleet-tmp-cleanup-20261030`) until it is emptied.

## Notes

- The probe's first two 409s were its own wrong shape — the title
  route reads the session from the query, not the body; the corrected
  probe set and restored a 2 KB hostile title cleanly. A hunt's finds
  include the hunter's mistakes.
- The gate closed at 1,843 / 0 / 5; the face harness at 474 / 51; the
  ledger checker green (378 rows, 33 open).
- The face is live at `http://127.0.0.1:3080` serving `0.3.0-beta`.
