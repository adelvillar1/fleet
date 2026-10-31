# The shakeout's pass two — the manifest walked deep, two crashers fixed

**2026-10-31** · the beta shakeout's second session: KI-380's zombie tab
fixed at all three disposition legs (the reads re-arm behind a visible
banner, the seat's cue moved to the tab title, the heartbeat's silence
measured, and the `always`/`remember` misread resolved as a pin) · the
whole manifest walked — groups A through H and HH — on scratch homes,
a keyless scratch face (every failure a named absence, **zero model
spend**), and the live face with restore-only rows on my own probe
session · **52 rows held, 2 finds, both fixed with pins in the same
session** (KI-380's disposition and KI-381: `fleet onboard` was
unreachable from the CLI) · one named skip (the device-side relay
rows). The gate: **1,845 passed / 0 failed**, the face harness **482
checks / 52 cases**, the ledger green at **381 rows, 34 open**.

## The session, in order

- **KI-380 closed** (`441979d`) — the three legs the row named:
  (1) every read fails into a bounded 5s self-retry that raises the new
  `#conn-down` banner — "the connection was lost · reload" — instead of
  the silent swallow, and the SSE error path declares and re-loads state
  on its own clock; (2) a seat from *any* path — live event, state read,
  replay — sets the tab title to "— needs you · …", the one surface a
  dead page cannot restyle, and the heartbeat now measures silence (the
  10s beat missing 15s raises the banner); (3) the `rule: null` misread
  resolved as a pin, not a wire change: `always` is the run-scoped latch
  and writes nothing, `remember` writes the standing rule — pinned by
  `always_answers_the_run_and_remember_writes_the_rule` and the face
  harness's new KI-380 case. **The live face serves the fix only after
  the owner restarts it — deliberately left to their hand, since the
  face is this run's own surface.**
- **HH5 found KI-381** (`fc0f802`) — `fleet onboard` answered
  "internal: an async command reached the sync dispatch" on every
  invocation: its arm was missing from the `needs_runtime` list in
  `main`, so the guided setup was unreachable from a terminal, a fresh
  machine's first run included. One arm fixed it; the walk and its
  resume are pinned end to end
  (`the_onboard_command_walks_and_resumes_end_to_end` drives the real
  binary twice on a fresh home).
- **The manifest walked** (`06b8b32`) — pass two's table in the plan
  carries every verdict with its evidence: A (run loop, keyless face),
  B (sessions rail, live face, my probe session), C (memory, scratch
  corpus plus the honest empty), D (skills, scratch library), E
  (calibration, the real 512-cap day), F (workspace history, the
  substrate's named absences), G (the seams, 12 concurrent writers and
  a read-only home), HH (the CLI, headless). Two isolation lessons rode
  in the probes themselves: the KI-087 shadow rule walks a scratch home
  up to the repo's corpus (the honest empty lives in a foreign cwd),
  and B3's run-in-flight guard caught this probe's own wrong-shaped
  fork aimed at the owner's live session — the guard works.
- **The evaluation sweep** — everything the walk found that already had
  a pin said so in the table; nothing was re-pinned for ceremony.

## Plans worked on

### `docs/plans/2026-10-30-beta-shakeout.md` — the beta shakeout

Status after this session: **active** — B1–B4 ticked; B5 (the beta
ships) waits on pass three: the re-walk after the owner restarts the
face and uses it for real.

| Acceptance criterion | Status | Notes |
|---|---|---|
| B1 the manifest walked once, shallow, whole | ✅ met | pass one (8 probes) + pass two (the whole matrix deep) |
| B2 every find is a row | ✅ met | KI-378, KI-380, KI-381 — filed, fixed, closed; checker green |
| B3 crashers and data-loss fix first | ✅ met | the zombie tab first, then the onboard crasher |
| B4 the deep pass re-runs the fixed rows | ✅ met | A3 live, the harness's KI-380 case, the onboard e2e pin |
| B5 the beta ships | partial | matrix whole, no open crasher; pass three closes it |

## Commits

| Hash | Message |
|------|---------|
| `441979d` | fix(face): the zombie tab cannot hide a seat or lose the reads — KI-380 closed |
| `fc0f802` | fix(cli): fleet onboard was unreachable — the missing runtime arm (KI-381) |
| `06b8b32` | docs(shakeout): pass two — the manifest walked deep, 52 held, 2 finds fixed |

## What was added

- **The connection banner** (`#conn-down`) — one element a dead page
  cannot repaint away, in the warn register, theme-aware; raised by
  every read's failure path and the stream's error, cleared by the
  first proof of life.
- **The tab-title seat cue** — "— needs you · state · fleet" derived
  from the one face state, repainted by every painter, so no
  navigation can hold a stale cue.
- **The heartbeat silence watch** — the 10s beat missing 15s raises
  the banner; any event or heartbeat clears it.
- **The pass-two matrix** — 55 rows with verdicts and evidence in the
  plan, the runbook's contract kept.

## What was fixed

- **KI-380** — the zombie tab (the owner's twice-reported find): reads
  re-arm, the stream's death is loud, the seat cannot be hidden, and
  the always/remember semantics are pinned so a null rule can never
  read as a lost write again.
- **KI-381** — `fleet onboard` was unreachable from the CLI (the
  missing runtime arm); found by HH5, fixed with the arm, pinned end
  to end.

## Open questions / next steps

- **The owner's restart of the live face** — KI-380's fix and the
  banner land on `http://127.0.0.1:3080` only after a launchctl
  kickstart; this run did not restart its own surface.
- **Pass three** — the re-walk (the fixed rows plus whatever the
  owner's own use shakes loose) closes B5 and the beta gate.
- **The task field's send bound** — still the owner's triage (KI-378's
  row): a one-shot spend, not a standing tax; the wire cap bounds it
  at 64 KB meanwhile.
- **The three genuinely-pending skill proposals** still wait the
  owner's approve; KI-367's disposition remains theirs.
- **The probe sessions** — mine and its fork are archived on the face
  (restorable); the scratch homes live under `tmp/shakeout/` for the
  daily sweep to age out.

## Notes

- The gate held all session: fmt/clippy/build clean, the suite closed
  at 1,845 / 0 / 5, the face harness at 482 / 52, the ledger checker
  green (381 rows, 34 open).
- The walk spent **zero model tokens**: the spend-class rows ran on the
  keyless scratch face, where the route's named absence is the honest
  answer (which is also G5's probe, so the row did double duty).
- The evaluation habit caught what the harness could not: HH5's crasher
  was invisible to every unit test because the runtime-list match and
  the dispatch match are two sites — the e2e pin now drives the real
  binary, and the comment at the list names why the arm must never be
  lost again.
