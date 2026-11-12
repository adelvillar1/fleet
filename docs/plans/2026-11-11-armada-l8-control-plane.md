---
status: completed
created: 2026-11-11
updated: 2026-11-12
slug: armada-l8-control-plane
---

# Plan: armada L8 — the control plane, the hand

## Context

This is armada's command plane — the layer the whole stack below exists to
serve. The flagship (a fleet install in a flagship role) commands instances
over the relay: a roster of installs, control verbs that reuse the bridge,
and authority that rides *in the frame*, signed, verified at the instance's
guard before it obeys. It composes every lower contract — identity (L1), the
guard + assertion (L3), corpus sync (L4), gated dispatch (L5), the sealed
push transport (L6) — and re-implements none of them. It is also the layer
where **trust is earned by measurement** most sharply: an instance earns
dispatch autonomy by measured reliability, never by registration. Atoms
L8.1–L8.5.

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared), **D7–D9** (identity), **D14–D19** (security:
the guard D15, the assertion D18, governance D19), **D23–D29** (corpus sync),
**D30–D36** (communication: gated dispatch D34, the relay channel D36),
**D20–D22** (relay: the Rust broker D20, push D21, the control frame D22).
Discharges the **cross-cutting obligation's L8 row**: a fresh instance is
un-calibrated until it settles runs; roster liveness earned by heartbeats; a
dispatch graded before the pattern changes.

## Decision-model check

The control plane is deterministic routing + the guard lookup + the
instance's admission. The *judgment* of an instance's reliability is a
grading loop over measured settles (D4), not a per-call sys1 ask. **No sys1
head fits the dispatch path** (it must stay off the synchronous path — the
socket guards); the reliability grade is a scheduled beat over the journal,
the way `judge`/`delegate` grade their rows.

## Approach

A new `src/armada/` module is the control-plane library the CLI, TUI, face,
and Tauri shell all call (one implementation, four surfaces — the bridge
seam). The flagship keeps a **roster** (`<home>/roster.json`): each instance
registers on join with its install-id (D7), sid, version, and capacity (the
contract's reading); a **heartbeat** tracks liveness — earned, not assumed.
Every control frame carries the principal (D8) and, for
`destructive`/`governance` verbs, a fresh assertion (D18) **signed by the
flagship's Ed25519 key**; the instance verifies the signature *before* it
obeys, then runs the verb through its own guard (D15) and admission. The
control vocabulary reuses the bridge verbs — `dispatch`→`bridge.run`,
`state`→`bridge.state`, `corpus-sync`→L4, `version-roll`/`retire`
(destructive), `health` (read), `register`/`heartbeat` (governance). The
flagship is a fleet install in a flagship **role** (role + config, one
binary); the signing key lives at the flagship, never on an instance the
agent can reach. The human second factor (TOTP or push) issues the assertion
the agent cannot forge. A basic `fleet armada` CLI is the first surface
(terminal-first, §9.2 stage 1); the richer surfaces are L9.

High-level steps:

1. `src/armada/roster.rs`: registration + roster + heartbeat (liveness earned).
2. `src/armada/authority.rs`: the flagship signs the frame's principal +
   assertion; the instance verifies before obeying, then the guard (D15).
3. `src/armada/control.rs`: the vocabulary, each verb through the guard +
   the instance's admission; dispatch reuses `bridge.run`.
4. The flagship role (role + config); signing-key custody at the flagship.
5. The human second factor issues the assertion (L3.7 verifies); a basic
   `fleet armada {roster|dispatch|status|sync}` CLI.
6. The reliability grade: a scheduled beat over the journal grades dispatch
   outcomes; a fresh instance is un-calibrated until it settles.

## Acceptance criteria

- [x] C0: an instance registers on join (install-id, sid, version, capacity); the flagship roster holds it; a heartbeat updates liveness, and a sid with no recent heartbeat is reported not-live (liveness earned, a test pins it).
- [x] C1: a control frame carries the principal + (for destructive/governance) a fresh assertion signed by the flagship; the instance verifies the signature before obeying; a forged or unsigned privileged frame is refused by name (a test pins it).
- [x] C2: every control verb runs through the instance's guard (D15) and admission; an ungranted verb is refused by name; `dispatch` reuses `bridge.run` and respects the instance's capacity.
- [x] C3: `corpus-sync` triggers the L4 sync (D23); `version-roll`/`retire` are destructive (step-up, D18); `health` is read; each verb's class is correct.
- [x] C4: the flagship is a fleet install in a flagship role (role + config, one binary); the signing key is at the flagship and absent from an instance the agent can reach (a test pins the custody).
- [x] C5: the human second factor issues a fresh per-action assertion the agent cannot forge; a destructive control verb without it is refused `StepUpRequired`.
- [x] C6: a fresh instance is un-calibrated until it settles runs; dispatch autonomy is earned by measured reliability and a dispatch is graded against its outcome before the pattern changes (the reliability beat over the journal).
- [x] C7: relay-down is a named degradation — instances run autonomously, the corpus still syncs over git, and live control resumes when the relay returns (a test pins it).
- [x] C8: the four-leg gate passes and a single-instance fleet is unchanged (the flagship role is additive; an install with no roster behaves as today).

## Files to be touched

**Backend:**
- `src/armada/` — new: `roster.rs`, `authority.rs`, `control.rs`, `mod.rs`.
- `src/lib.rs` — `mod armada`.
- `src/security.rs` — assertion issuance (flagship) + verification (instance).
- `src/remote/relay.rs` — control-frame handling (L6's `kind:"control"`).
- `src/gui/bridge.rs` — dispatch reuses `bridge.run`.
- `src/cli.rs`, `src/main.rs` — the basic `fleet armada` verbs.
- `src/settings.rs` — the flagship role + roster config.

**Tests:**
- `src/armada/` (module tests) — registration/liveness, signature verify/forge, verb gating, custody, the reliability grade, relay-down.

**Documentation:** see Linked artifacts.

## Out of scope

- The richer surfaces — TUI board, face section, Tauri (L9).
- Deployment / provision→register→join / fresh-install corpus inheritance (L9).
- Multiple principals / the org user store (lane 2).
- SSH/QUIC alternative transport (a later option).
- The second-factor *choice* (TOTP vs push is an open fork; the contract is factor-agnostic).

## Verification

**C0/C1/C2:** `cargo test -p fleet --lib armada` — registration/liveness,
signature verify + forge refusal, verb gating + admission.

**C3:** a test exercises each verb's class (corpus-sync→L4, destructive
step-up, health read).

**C4/C5:** tests pin signing-key custody (flagship only) and the
second-factor assertion (agent cannot forge).

**C6:** a test runs the reliability beat over journaled dispatch outcomes;
a fresh instance is un-calibrated until it settles.

**C7:** a relay-down test asserts autonomous runs + git corpus sync +
resumption.

**C8:** the four-leg gate; a no-roster install behaves as today.

## Linked artifacts

- `docs/features/armada.md` — new: the control-plane contract (roster, authority transport, vocabulary, flagship role).
- `TECHNICAL-DOCUMENTATION.md` — the control-plane contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D44–D50 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L8 atoms ticked when shipped.

## Risks

- **Signing-key custody is load-bearing** — a leaked flagship key forges authority across the roster. Mitigation: the key lives only at the flagship, never on an instance the agent can reach; rotation/revocation is an open fork (§13) flagged for the owner.
- **The authority transport is the security boundary across the relay** — verify-before-obey must be airtight. Mitigation: the instance verifies the flagship signature, then re-checks the principal's grants at its own guard (D15); the relay holds no key (D20 dumb broker).
- **Reliability grading on the hot path** — the grade is a scheduled beat over the journal, never a synchronous sys1 ask (the socket guards). Mitigation: the dispatch path stays deterministic; the beat grades asynchronously.
- **Flagship-is-fleet** — the role must not fork the binary. Mitigation: role + config only; an install with no roster is unchanged (C8).

## Dependencies

- All of L1, L3, L4, L5, L6 (the contracts it composes). Inherits ledger D0–D43.

## Design choices this layer settles (append to the ledger)

- **D44 — registration + roster + heartbeat: an instance registers on join
  (install-id D7, sid, version, capacity); the flagship roster at
  `<home>/roster.json`; liveness earned by heartbeats, not assumed.**
  *Constrains:* L9 deployment, lane 2 (the roster grows to many).
- **D45 — the authority transport: every control frame carries the principal
  (D8) + (destructive/governance) a fresh assertion (D18) signed by the
  flagship Ed25519 key; the instance verifies before obeying, then its own
  guard (D15); the relay cannot fabricate, an instance cannot escalate, an
  agent cannot borrow.** *Constrains:* lane 2 (frames carry many principals).
- **D46 — the control vocabulary reuses the bridge verbs: dispatch→bridge.run,
  state→bridge.state, corpus-sync→L4 (D23), version-roll/retire (destructive),
  health (read), register/heartbeat (governance); each through the guard +
  the instance's admission.** *Constrains:* L9 surfaces, lane 2.
- **D47 — the flagship is a fleet install in a flagship role (role + config,
  one binary); the signing key lives at the flagship, never on an instance
  the agent can reach.** *Constrains:* L9, the custody/rotation fork.
- **D48 — the human second factor (TOTP or push, factor-agnostic) issues the
  fresh per-action assertion the agent cannot forge; L3.7 verifies.**
  *Constrains:* lane 2 per-user factors; the second-factor fork.
- **D49 — instance autonomy earned by measurement: a fresh instance is
  un-calibrated until it settles runs; a scheduled reliability beat grades
  dispatch outcomes over the journal before the pattern changes.**
  *Constrains:* lane 2 per-user quota; the cross-cutting obligation.
- **D50 — relay-down is a named degradation: instances run autonomously, the
  corpus syncs over git, live control resumes on return.** *Constrains:* the
  failsafe ruling at armada scale (§8.6).

## Notes

The hand. It stands on every contract below and re-implements none. The
load-bearing properties: authority rides signed in the frame and is verified
before obeying (the relay stays dumb), and an instance earns autonomy by
measured reliability — the flagship cannot overwhelm a small VPS because the
VPS admits for itself, and cannot trust a fresh install until it settles.
Approve this and the ledger carries D44–D50 into L9.