# Session recap — 2026-11-03: the native shell, the composer that applies, and the providers that connect

**Summary.** Three plans implemented whole — the Tauri native shell, the
composer actions applying on submit, and the provider-connect honesty —
plus the toolchain's move to Rust 1.99.1, hard rule 12, and the serving
binary given one home. The session's through-line is the owner's
self-containment ruling made structural: one shell carrying every
component, one serving binary, and controls that act when they are
submitted.

## The arc, in order

1. **The 1.99.1 toolchain move.** The owner ruled for the latest stable;
   `rustup update stable` moved 1.89.0 → 1.99.1, and the gate surfaced 41
   new lints (39 mechanical, two `result_large_err` allowed with their
   reason). The toolchain's own `rust-objcopy` proved broken — dynamically
   linked against `@rpath/libLLVM.dylib`, which rustup installs where the
   rpath does not resolve — so the release strip aborted with SIGABRT and
   the binary shipped unstripped. Repaired with a symlink; KI-403 records
   that a `rustup update` removes it.
2. **The Tauri native shell, researched, planned, and implemented.** The
   research settled the two migration shapes; the plan chose the thin
   shell — keep the server the remote face needs, open a window on the
   loopback origin — and carried the owner's component inventory: every
   Fleet component accounted for in the install, no loose ends. The shell
   sets `FLEET_SERVICES_ROOT` (the checkout walk needs a `Cargo.toml`
   sibling and cannot resolve inside a bundle), sets the lane wiring
   itself, builds the venvs on the first run, and never opens a browser.
   Proven in the artifact: the packaged app resolved the bundled services
   and served the face on an ephemeral port; the service worker
   registered; a turn ran.
3. **Hard rule 12.** The owner's ruling made self-containment a standing
   contract: anything added that becomes a dependency must ship, be
   built, or be fetched by the app — never the owner's shell profile, a
   checkout, or a foreign runtime.
4. **The composer actions apply when submitted.** The audit found one
   mechanism with six symptoms: the agent loop reads the control file
   only before each model call, so while an approval seat waited, stop,
   steer, pause, fold and pin hung indefinitely — and a reload stranded
   the run entirely, because `openWork`'s resume was refused for a run in
   flight and skipped the state read. F1 makes the approval wait poll the
   control file; F2 loads the view on the refusal; F5 keys the queue's
   rebuild so send, edit and drop land mid-run.
5. **The providers connect.** The owner's OpenCode connect failed with a
   404: the pasted value was the full endpoint, and the listing lives at a
   different prefix from the chat. A pasted endpoint now normalizes to the
   base, a provider entry can name its own `models_url`, the connect card
   carries a models field for providers that publish no listing, and the
   discovery's reply is honest about what landed (KI-407's
   add-without-catalog ruling, and the catalog the picker reads).
6. **The serving binary has one home.** The LaunchAgent hardcoded a second
   worktree's own `target/` binary, which went stale — the browser and
   the PWA served UI that lagged the pushed master (KI-408). The daemon
   now runs `~/.fleet/bin/fleet`, the reload runbook is two commands and
   a restart, and the served face is verified byte-identical to the
   source.
7. **The turns-left display retires.** The owner's ruling: turn count is
   meaningless to end users — compaction count or context degradation is
   the only thing a user would care for. The composer's field is gone;
   the head's context gauge is the one degradation signal; the turn count
   stays internal. Whether the turn *budget* itself should stay
   turn-driven is recorded as a deferred discussion.

## Plans worked on

- `2026-11-02-tauri-native-shell.md` — **completed**; C0–C10 ticked with
  their evidence, including the owner walk.
- `2026-11-03-composer-actions-apply-on-submit.md` — **completed**; C0–C6
  ticked (C0/C1 over the real bridge, C2/C3/C5 over the harness).
- `2026-11-03-provider-connect-honesty.md` — **completed**; C0–C4 ticked
  (C0's stored-entry half, C1 over the write and the catalog, C3 over the
  card's answer).
- `2026-10-31-quota-failover-and-compaction-count.md` — the freshness
  plan carries the 2026-11-03 amendment (decision 3) and the deferred
  budget-shape discussion.

## Work by the numbers

- **Gate, final:** fmt, clippy `-D warnings`, release build — all exit 0;
  **2,012 tests / 0 failed**; face harness **699 checks across 67 cases**;
  `plan_check.py` and `ki_ledger.py` clean.
- **Plans:** three completed and ticked; one decision amended with a
  deferred discussion recorded.
- **Ledger:** 407 rows / 14 open; KI-403, KI-405, KI-406, KI-408 filed;
  KI-407 closed; the checker clean.
- **The serving face:** reloaded onto `~/.fleet/bin/fleet` and verified
  byte-identical to the current source — the browser and the PWA serve
  one build again.

## Commits (this session, fleet)

| Hash | Message |
|---|---|
| `eabe031` | chore(toolchain): Rust 1.99.1, and the plugin host's parked work |
| `32b215c` | docs(plans): the native shell — Tauri over the loopback origin |
| `d90123c` | docs(plans): the native shell carries every Fleet component |
| `104bc06` | docs(plans): the services-root walk needs a checkout |
| `9b1e9b2` | feat(desktop): the native shell — the face in its own window |
| `6b26a1e` | feat(desktop): the package builds, and the inventory is pinned |
| `4cd8702` | docs(desktop): the owner walk is done |
| `7ba1d02` | docs(rules): self-containment is a standing contract (hard rule 12) |
| `2e0c780` | docs(plans): every composer action applies when it is submitted |
| `9a39cf2` | docs(plans): the queue's buttons, and the replay path that strands them |
| `8ce0c0d` | log KI-407 and plan the provider-add decoupling |
| `191eeab` | make adding a provider independent of reading its catalog |
| `980e350` | fix(providers): a pasted endpoint normalizes, and a listing can live elsewhere |
| `521724c` | fix(providers): the connect card carries the models its owner names |
| `29b5187` | docs(install): the serving binary has one home |
| `b2a4f9f` | fix(face): every composer action applies when it is submitted |
| `4caed48` | fix(face): the composer's turns-left retires |
| `a6b7b82` | docs(plans): the turn budget's own shape is a deferred discussion |

The co-tenant session's work interleaved on the same branch — the plugin
host's P0–P2, the plugins-ux rethink, KI-404, and the usage plan's
completion — and is recorded in its own recap
(`2026-11-02-session-parity-and-the-plugin-host.md`, its plugins-ux
addendum).

## What the build proved — and the lessons it taught

- **The window is the easy half.** The native shell's real work was the
  install: the services root (the checkout walk needs a `Cargo.toml`
  sibling), the lane wiring a dock-launched app's missing shell profile
  would drop, the venvs, gen1's named weights, and the daemon lifetime.
  The component inventory was written because of it, and C5–C8 exist
  because of what the inventory exposed.
- **A control that cannot act now must say when it will.** The stop said
  "at its next step" and there was no next step to reach — the seat the
  owner was never going to answer held the run hostage. The fix is at the
  wait, not the wording.
- **A serving binary needs one home.** Two worktrees meant two
  `target/release` binaries and a daemon pointed at the one nobody
  rebuilt; the discrepancy was invisible on the machine that browsed the
  same stale server and surfaced only on the phone.

## Doc updates applied

- `docs/features/install.md` — the native shell's component inventory,
  and the serving-binary runbook (one home, the two-command reload).
- `CLAUDE.md` — hard rule 12; the two-crates topology; `fleet desktop`
  in the command list.
- `docs/plans/2026-10-31-quota-failover-and-compaction-count.md` —
  decision 3 (the composer's turns-left retires) and the deferred
  budget-shape discussion.

## Doc updates deferred (proposed text, awaiting the owner's word)

- `docs/features/gui.md` § the runbar — the turns-left field is gone; the
  proposed text, for the paragraph that currently ends *"a turn count
  survives only as a remaining budget, never a size claim"*:

  > A turn count no longer faces the user at all — the runbar carries no
  > turns-left field, and the head's context gauge (used against the
  > compaction budget) is the one degradation signal on the surface. The
  > turn count stays internal: the loop's own state, never a user-facing
  > number.

- `FUNCTIONAL-SPECIFICATIONS.md` § the same rule — the matching sentence
  *"A turn count remains only where it is a remaining budget, never a
  size claim"* becomes:

  > A turn count does not face the user. The composer carries no
  > turns-left field; the context gauge — context used against the
  > compaction budget — is the run's one user-visible degradation signal.

- `docs/features/gui.md` § the connect card — the models field and the
  honest answer, proposed:

  > The connect card derives the listing's address from the endpoint and
  > lets the owner edit it — OpenCode's chat is
  > `…/inference/openai/v1/chat/completions` while its listing is
  > `…/inference/v1/models`. A provider that publishes no listing is
  > still connectable: the card carries a models field, the owner's list
  > rides the entry, and a failed discovery never overwrites it.

## Open questions / next steps

- **The turn budget's shape** — deferred by the owner. A turn is not a
  unit a user would measure (50 turns of 2,000 tokens ≠ 20 turns, or
  50,000 tokens); whether the run's cap stays turn-driven or moves to a
  token/context budget that degrades the way the gauge measures is
  recorded in the 2026-10-31 plan's decision 3 for the future discussion.
- **KI-406** — the native window has no automated witness; the HTTP API
  is the shell's test surface until a remote-debugging seam is chosen.
- **KI-403** — the `rust-objcopy` symlink dies on the next `rustup
  update`; the strip fails silently when it does.
- **KI-405** — the probe-retry pin is load-sensitive; open.
- **KI-404** — the bound-board digest source; open, owner decision.

## Notes

- The gate ran on every push that carried code, by exit code; the last
  full gate on this session's tree: fmt 0 · clippy 0 · release build 0 ·
  2,012 tests / 0 failed · harness 699 checks / 67 cases.
- The cache sweep at close: debug 15.2 GiB → 15.1 GiB; 168 files older
  than 7d pruned; the dev profile cleaned whole; release targets
  untouched.
- The daemon now runs `~/.fleet/bin/fleet`; the `fleet-master-wt`
  worktree is fast-forwarded to the pushed master and rebuilt, but it no
  longer serves anything.
- The owner's OpenCode provider is connected and live: base normalized,
  38 models from `…/inference/v1/models`, the key in the store.
