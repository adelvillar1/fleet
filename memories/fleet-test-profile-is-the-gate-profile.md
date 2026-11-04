---
name: fleet-test-profile-is-the-gate-profile
description: "feedback 2026-10-20 — run fleet's suite with plain `cargo test`; a `--release` run reports two failures that are artifacts of the profile (a test asserting the gate binary lives outside target/release, and a relay socket error under load)"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

After the rail-mark change I ran `cargo test --release gui::` (because the release binary had just been built) and got 3 failures that all passed in isolation; the full `cargo test --release` then reported 1304 passed, 2 failed on tests unrelated to any asset:

- `eval::tests::the_binary_check_accepts_the_gate_roots_own_target_dir` — `assertion failed: !under_target(&exe, &target.join("release"))`. The test asserts the eval/gate binary does **not** live under `target/release`, which is necessarily false when the test itself is the release-profile test binary.
- `remote::relay::tests::the_serve_loop_answers_until_it_is_told_to_stop` — the fake relay refused the answer with a network error (`Invalid argument (os error 22)`) and zero frames arrived; load/timing shaped.

The project's gate is **debug**: plain `cargo test` gave 1306 lib + 6 + 2 + 7 + 13 + 103 ≈ **1437 passed, 1 ignored**, in ~30s. The GUI-module flakes earlier under `--release` (`the_address_alone_opens_the_face...`, `the_rank_route_orders_candidates...`, `a_leftover_control_does_not_survive...`) also passed alone, and all passed in the debug run.

**Why:** a profile-mismatched run manufactures red just as reliably as a real defect, and the honest report of "2 failed" invites a hunt that has no bug at the end. The `--release` run is useful for one thing only: it is the same compilation the serving binary takes.

**How to apply:** gate with plain `cargo test` (with the rustup PATH — [[fleet-gate-exit-code-trap]]); if a release run is done for confidence, exclude or explain these two by name instead of treating them as regressions. For a flake, re-run **that test exactly** first (`cargo test <module>::tests::<name> -- --exact`), then the full suite once the machine is quiet — a live GUI warming ~400 embeddings beside the suite starves the rank route ([[fleet-gui-editorial-desk-redesign]]). Related: [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]].
