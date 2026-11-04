---
name: a-native-install-must-carry-every-component
description: "feedback 2026-11-02 — the owner's requirement for the native shell is that every Fleet component is accounted for in the install with no loose ends; the window is the easy half, and the loose ends are all places the PWA era leaned on something outside the app (the services root, the lane wiring, the venvs, the daemon lifetime)"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's requirement for the Tauri native shell (2026-11-02): *"we need to
make sure that all of fleet components are accounted for in the install, no
loose ends."* Self-containment was the founding intent *"from the very
beginning"* — no Node runtime, no staged harness payload, no checkout
dependency — so the native install must **close** the loose ends the PWA era
left, not add new ones.

**Why:** the PWA ran in a browser the owner already had, from a checkout, with
a shell profile that carried the lane exports. A dock-launched app has none of
those. Every dependency the PWA borrowed from its surroundings becomes a
loose end in the bundle — and none of them fail loudly: the lanes are simply
dead, or a scheduled job simply never fires.

**How to apply — the inventory before the window.** A native/embedded install
plan needs a **component table** (component · where it lives today · how the
install carries it), and the rule that **a component with no home fails the
criterion rather than being noted**. Fleet's six real loose ends:

1. **The services root.** The 12-ancestor walk requires a **`Cargo.toml`
   sibling** (`src/lanes.rs:985`) and its fallback is the compile-time manifest
   dir (`:991`) — neither exists inside a bundle, so the app must set
   `FLEET_SERVICES_ROOT` to the bundled `Contents/Resources/services`.
2. **The lane wiring.** `fleet install` *prints* `export SEM1_ST_VENV=…`,
   `SDM1_MODELS_VENV`, `GEN1_KOKORO_PYTHON` and a sys1 `config.toml` note —
   for the owner's shell profile, which a dock-launched app does not source.
   The app must set the three variables in the child environment and write
   sys1's key itself. **Anything a CLI prints as an `export` is a loose end in
   a GUI-launched app.**
3. **The venvs** (1.6 GB, `uv`, receipt discipline) — the first-run install
   builds them and degrades by name.
4. **gen1's ONNX weights** — machine-owner downloads; the app names them, never
   fetches.
5. **Credentials** — owner-supplied; the app never writes a key.
6. **The daemon lifetime.** The consolidator and cron scheduler are spawned by
   the GUI bridge (`src/gui/bridge.rs:10062,10068`) — they run while the app
   runs and stop when it quits. If scheduled jobs must outlive the app, that is
   a separate LaunchAgent increment (the repo already runs
   `local.fleet-maintenance` that way), not a silent gap.

**The general form:** when a plan wraps or packages an existing app, enumerate
what the app *borrowed* from its current environment (a browser, a shell
profile, a checkout, a terminal, a running server) and give each a home in the
new one. The wrapper itself is the smallest part of the work.

Related: [[tauri-native-shell-research-2026-11-02]] (the plan this requirement
shaped), [[a-new-plan-must-answer-the-past-plans]] (the audit that preceded it),
[[fleet-is-the-native-rust-successor-to-the-fork]] (the founding contract),
[[fleet-gui-is-a-launchagent]] (the app's own launch posture),
[[fleet-folder-is-a-default-not-a-ritual]] (the same "no ritual" instinct).
