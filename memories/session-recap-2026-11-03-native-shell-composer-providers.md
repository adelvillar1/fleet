---
name: session-recap-2026-11-03-native-shell-composer-providers
description: "project 2026-11-03 — the recap at docs/recaps/2026-11-03-the-native-shell-the-composer-and-the-providers.md: THREE plans completed whole (the Tauri native shell, the composer actions applying on submit, provider-connect honesty), the 1.99.1 toolchain move, hard rule 12, one serving binary, and the turns-left display retiring; the recap was UNTRACKED at the next session's start"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The session's close-of-record: **three plans implemented whole and landed**,
journaled at `docs/recaps/2026-11-03-the-native-shell-the-composer-and-the-providers.md`
(202 lines). The through-line is the owner's self-containment ruling made
structural: one shell carrying every component, one serving binary, and
controls that act when they are submitted.

1. **`docs/plans/2026-11-02-tauri-native-shell.md`** — completed; C0–C10
   ticked with their evidence, including the owner walk. The thin shell
   keeps the server the remote face needs, sets `FLEET_SERVICES_ROOT`, the
   lane wiring, builds the venvs on first run, never opens a browser.
2. **`docs/plans/2026-11-03-composer-actions-apply-on-submit.md`** —
   completed; C0–C6 ticked (C0/C1 over the real bridge, C2/C3/C5 over the
   harness). One mechanism, six symptoms: the agent loop read the control
   file only before each model call, so while an approval seat waited, stop,
   steer, pause, fold and pin hung — and a reload stranded the run.
3. **`docs/plans/2026-11-03-provider-connect-honesty.md`** — completed;
   C0–C4 ticked. A pasted endpoint normalizes to its base, a provider can
   name its own `models_url`, the connect card carries a models field, and
   the discovery's reply is honest.

**Also in the arc:** the Rust 1.99.1 move (41 new lints; the toolchain's
`rust-objcopy` is broken — KI-403), **hard rule 12** (self-containment is a
standing contract), **the serving binary given one home** (`~/.fleet/bin/fleet`,
KI-408 — two worktrees meant two stale binaries), and **the turns-left
display retiring** (the owner's ruling: turn count is meaningless to end
users; the context gauge is the one degradation signal; the turn *budget*
shape is a deferred discussion).

**Closing numbers:** gate fmt 0 · clippy 0 · release build 0 · **2,012 tests
/ 0 failed** · face harness **699 checks / 67 cases**; ledger **407 rows /
14 open**; KI-403, KI-405, KI-406, KI-408 filed, KI-407 closed. The serving
face reloaded onto `~/.fleet/bin/fleet` and verified byte-identical to the
source. The owner's OpenCode provider is connected and live (38 models).

**⚠️ State at the next session's start (2026-11-03, later):** the recap file
was **untracked** on disk (written 14:10, never committed) and the 22
modified `memories/*.md` files were uncommitted substrate churn — the session
ended without its closing commit. Branch `face-plugin-host` was even with its
upstream and 4 commits ahead of `master`. The only active plan is
`docs/plans/2026-10-30-beta-shakeout.md` at 4/5 criteria (B5 — pass three
after the owner's own use — closes the beta gate); `plan_check.py` reports
every completed plan's criteria ticked.

Related: [[tauri-native-shell-implemented-2026-11-02]],
[[composer-actions-plan-drafted-2026-11-03]],
[[provider-connect-honesty-plan-drafted-2026-11-03]],
[[self-containment-is-a-standing-contract]],
[[turn-budget-shape-is-a-deferred-discussion]],
[[a-memory-index-line-is-not-the-plans-state]].
