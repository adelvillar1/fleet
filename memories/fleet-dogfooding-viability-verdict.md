---
name: fleet-dogfooding-viability-verdict
description: "Owner's goal (2026-10-15) is to dogfood Fleet as the harness for building its own missing parts; the viability fix plan is IMPLEMENTED (2026-10-16, complete) — gate runs whole (fmt+clippy+build+test, 1013 passed), KI-007/208-213 closed; dogfooding experiment unblocked; CI stays disarmed"
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

The owner stated their goal (2026-10-15): understand how far Fleet is from a **viable harness they can use to do actual work** — specifically building Fleet's own still-missing parts — testing it by real use. They want distance-to-viability, not an exhaustive defect ledger.

Verdict delivered: the harness is viable for attended real work today (Ask mode prompts every call, so approval-gate findings don't apply). Nothing structural missing.

**The fix plan is IMPLEMENTED (2026-10-16, status complete)** — owner then said "implement the plan as designed" and all eight phases landed: commits `c7741c9` (KI-208..213 filed) → `3e05dfe`/`7643cf7` (rustfmt + clippy passes; rustup shim installed, rustfmt 1.8.0/clippy 0.1.89 — KI-007, KI-097, KI-104 closed, so `fleet improve apply` is unblocked on this machine) → `8a3275e` (bash bounded drain, KI-211) → `d7b93dd` (approval gate `unlabel` strips all `subagent → ` repetitions, KI-209/210) → `328faf5` (route record maps through walk order, KI-208) → `36facd6` (fold stamps only landed summaries; forget half follows the fold, KI-212) → `9b586dd` (torn-tail tolerance, KI-213) → `2736a91` (convergence + recap). The gate now runs **all four legs locally and green: 1012 passed / 2 ignored** — cite that count, not the old 977. (1013 → 1012 is net −1 from the 2026-10-16 own-home ruling below: two harness-leg tests removed, one negative regression test added.)

**The owner's own-home ruling (2026-10-16): Fleet's router reads no other harness's config.** Once the roster migration had put every key in `<FLEET_HOME>/.env` ([[fleet-roster-migrated-from-router-kit]]), the third credential leg — env → `<FLEET_HOME>/.env` → the DeepSeek Harness's `~/.dsh/.credentials.yaml` `refs` block — was redundant, and the owner ruled: "fleet's router should never be reading a config file from a different harness." It is gone from `route.rs::resolve_key`, `main.rs::setting`, and a fourth site the plan hadn't named (`install.rs`'s credential-chain note); the `home.rs` helpers that made the read possible were deleted outright, not merely unwired, so it cannot return through dead code. Commits `6bf970d` (removal), `b8530be` (ruling recorded in the repo's own corpus via `fleet remember`), `4038b00` (state); plan `docs/plans/2026-10-16-own-home-credentials.md` is complete. A negative regression test writes a decoy harness store, points `DSH_HOME` at it, and pins that the key still does not resolve — the one intentional exception to the plan's "grep returns nothing in src/" verification. The boards' read-only API seams are now ruled too: the router-kit operator API is gone from the board entirely, which reads Fleet's own state instead ([[fleet-routes-through-its-own-router]]); the second-brain board still reads its state files, which the owner ruled separately.

Two implementation corrections recorded in the plan's convergence section: the streamed client always assembles one choice, so a `[DONE]`-only stream is a completed run with an empty answer (the hits firing there is correct; KI-068's zero-choice premise was unreachable); and an in-process loopback server + ureq client in one process breaks (EINVAL) — use the cross-process mock-server. The phase-3 tests live as unit tests beside the private `run_bash`, not integration tests.

**CI ruling (owner, 2026-10-15): keep CI exactly as it is — disarmed, local gate only.** Owner is considering a self-hosted Gitea instance on Railway as a future gate host but undecided; counsel given: Gitea buys remote history/backup more than CI for a solo private repo whose real gate is local. Nothing in the plan depends on CI.

Backlog to build *with* the harness (small, named, none structural): live sys1 `composed` judgment (slice 21's open leg), episodic-summary shape/phrase inheritance, cross-day demotion (KI-050), skills drafter model-polish (KI-052), structured provider editor, web fetch/max_urls forwarding, sdm1 latency grading (KI-107), per-OS release artifacts + Windows verification (KI-011).

Counsel given: deliberately do NOT fix the remaining ~20 second-dive findings before dogfooding — they wait in the plan's out-of-scope; fix on encounter. The proposed experiment is now unblocked: work attended in the TUI with `fleet calib`/`judge`/`delegate` watching; anything unattended follows the KI-066 mock-pinning discipline.

The owner asked how to launch the TUI (2026-10-16, right after the fixes landed) — bare `fleet` is now the TUI from any directory, `fleet chat` is headless. **The binary is registered: `~/.local/bin/fleet` symlinks to `target/release/fleet`** (2026-10-16), so every `cargo build --release` updates it in place; a `cargo clean` dangles the symlink until the next build (swap for a copy if that annoys the owner). `~/.local/bin` was already on PATH. This is likely the settings pane's first human open, which the settings plan named as the experience's proof; if they drive it from tmux, KI-163 applies (`tmux send-keys` delivers control keys as raw bytes — test with real keypresses).

Detailed defect evidence: [[deep-dive-2026-10-15-second-pass-findings]]; gate procedure: [[fleet-gate-exit-code-trap]]; the routing roster was then migrated off the pin (keys sourced from the kit, sys1 judgment live): [[fleet-roster-migrated-from-router-kit]].

**Dogfooding verdict CONFIRMED (owner, 2026-10-16, after the harness-voice/orientation/session work): "fleet is working much better now."** The TUI session driving the ki-triage plan visibly displayed the installed behaviors unprompted: the outcome-first recap shape (what landed / verification / committed / next), claim-time verification with tool output cited (`rows: 218 open: 88`, gate legs per change — "no Rust source touched, so the gate is unaffected"), and premise-correction — it found the plan's 85-open-row count was really 88 (unescaped pipes in `docs/known-issues.md` had shifted cells so KI-128/146/207 read as closed), named that "a naive parser reports *plausible* wrong," and chose to re-derive the list before touching source. Committed `0f2d44e` (KI-084's fix + the C0 parser `scripts/ki_ledger.py`); the ledger is now parser-clean. Lesson: when fleet shows a new failure mode in dogfooding, first check whether it logs the KI row itself — if not, that's a teachable finding.
