---
status: completed
created: 2026-11-11
updated: 2026-11-11
slug: armada-l3-security
---

# Plan: armada L3 — the security layer, the owner's example

## Context

The load-bearing foundation the owner named: *the security layer added to
fleet must support the armada vision and provide well-defined contracts
armada can use.* Today fleet has rich action-class machinery (the approval
gate, the destructive class, the folder grant) but **no principal and no
uniform guard** — every check is mode + tool + arguments, never "who is
acting," and the checks are scattered. This plan builds the principal-agnostic
authorization substrate: a grant store, one uniform guard, per-principal
declaration filtering, durable audit, the signed assertion, and the
governance boundary — atoms L3.1–L3.7. Built with one principal (`owner`)
now, so the org (lane 2) is configuration, not a security retrofit.

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared), **D7–D9** (L1: the principal the guard
reads, riding `AgentOptions.principal`), **D10–D13** (L2: the journal the
audit settles into). Honors **D6** hard: the owner holds every grant by
default, and lane-1 destructive approval still works through the existing
in-turn approval — nothing the owner does today is refused.

## Decision-model check

The guard is a deterministic grant lookup (default-deny), not a judgment.
The destructive *class* is fleet's existing named substring list
(`approvals.rs`), deliberately not a heuristic. **No sys1 job fits the
guard** — authority is code-enforced (R2: a prompt is not a security
boundary). The advisory `judge_destructive` sys1 explanation stays advisory;
it never grants.

## Approach

One new module, `src/security.rs`: the `Grant` tuple
`(principal, dimension, scope, action-class)` with the closed `Dimension` /
`ActionClass` / `Scope` enums, a grant store at `<home>/grants.json`, and the
uniform `guard.check(...) -> Granted | Denied{name} | StepUpRequired`. The
guard hooks `prepare_call` (`src/agent.rs:5184`), the single choke point
every call crosses; the folder grant becomes the first row
`(owner, project, project:<id>, write)`. **Lane-1 bootstrap:** the owner
principal holds a wildcard grant, so the guard grants everything the owner
does today; a `destructive`/`governance` action's step-up is satisfied by the
existing in-turn approval (hard rule 7) — the signed assertion is the
org-scale form, verified when present, issued by the flagship at L8.
Declaration filtering at `declaration_set` (`src/agent.rs:2443`) hides an
ungranted tool. Every guard decision settles a `guard` event into the journal
(D10). The governance layer (grant store, guard config, roles, org SOUL) is
outside the agent's writable surface; grant-making is the root privilege.

High-level steps:

1. `src/security.rs`: the enums, `Grant`, the store, `guard.check`,
   default-deny + the owner wildcard bootstrap.
2. Hook `prepare_call`; the folder grant migrates to the first grant row.
3. Per-principal declaration filter at `declaration_set`.
4. Guard audit → journal (`kind="guard"`, principal/dimension/scope/class).
5. The `Assertion` format + Ed25519 verification (`ring`, already in the
   tree); the step-up gate accepts in-turn approval (lane 1) or a valid
   assertion (org). Signing-key custody + human issuance deferred to L8.
6. The governance boundary: `governance` class, immutable to the agent,
   grant-making the root privilege, refused by name and audited.

## Acceptance criteria

- [x] C0: the grant store holds `(principal, dimension, scope, action-class)` tuples; the folder grant is represented as `(owner, project, project:<id>, write)`; an ungranted action is refused by name (default-deny).
- [x] C1: the uniform guard checks every call at `prepare_call`; with the owner wildcard, no existing owner action is refused — the four-leg gate passes and behavior is unchanged.
- [x] C2: a principal lacking a tool's grant does not see that tool declared (a test pins the per-principal declaration filter).
- [x] C3: every guard decision (grant/deny) settles a `guard` journal event carrying principal, dimension, scope, and class (the durable per-principal audit).
- [x] C4: a `destructive`/`governance` action requires fresh human proof — lane-1 in-turn approval satisfies it (unchanged), a valid Ed25519 assertion satisfies it, and neither yields `StepUpRequired` (a test pins all three).
- [x] C5: an agent attempt to modify the governance layer (grant store, guard config) is refused by name and audited; the agent is never declared a governance tool.
- [x] C6: the assertion verifies against the flagship public key and rejects a forged or stale one (a test signs with a test key and pins freshness/nonce).
- [x] C7: the security surface — guard refusals are named in the transcript and the settings trust section renders the principal's grants.

## Files to be touched

**Backend:**
- `src/security.rs` — new: enums, Grant, store, guard, assertion, governance.
- `src/lib.rs` — `mod security`.
- `src/agent.rs` — `prepare_call` guard hook; `declaration_set` filter.
- `src/gui/bridge.rs` — the folder grant migrates to a grant row; the trust section reads grants.
- `src/settings.rs` — the grant store writer (one writer, CLI + GUI).
- `src/cli.rs`, `src/main.rs` — `fleet grants` verb.
- `Cargo.toml` — promote `ring` to a direct dependency (already in the lock).

**Tests:**
- `src/security.rs` (module tests) — default-deny, owner bootstrap, filter, audit, assertion verify/forge/stale, governance refusal.

**Documentation:** see Linked artifacts.

## Out of scope

- Signing-key custody + rotation and the human second-factor issuance (L8).
- Multiple principals / roles-as-templates / per-user factors (lane 2).
- The control-frame authority transport (L8 consumes this guard).
- Workspace invocation gating (L7 consumes this guard).
- Changing the destructive substring list or the approval modes (unchanged).

## Verification

**C0/C1/C2:** `cargo test -p fleet --lib security` — default-deny, owner
wildcard, declaration filter; the four-leg gate proves behavior unchanged.

**C3:** a test runs a call and asserts the `guard` journal event with the
principal/dimension/scope/class.

**C4/C6:** tests sign an assertion with a test Ed25519 key — valid passes,
forged/stale rejected; lane-1 in-turn approval still satisfies step-up.

**C5:** a test attempts a governance write as the agent and asserts the
named refusal + audit.

**C7:** `fleet grants` renders; a refusal is named in the transcript.

## Linked artifacts

- `docs/features/security.md` — new: the grant/guard/assertion/governance contract.
- `TECHNICAL-DOCUMENTATION.md` — the authorization substrate contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D14–D19 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L3 atoms ticked when shipped.

## Risks

- **The guard is on every call's hot path.** Mitigation: the grant lookup is a cheap in-memory match (the store is small); the owner wildcard short-circuits lane 1; measure the overhead (D4).
- **Non-breaking step-up** — requiring a cryptographic assertion immediately would refuse the owner's current destructive approvals. Mitigation: lane 1 accepts the in-turn approval as the fresh proof; the assertion is additive (the org-scale form).
- **Check order is load-bearing** (folder before the gate, scope after — `agent.rs:2690`). Mitigation: the guard preserves the documented order; the screening row stays the gate's verdict.
- **`ring` promotion** — already transitive; promoting it adds no foreign crate (self-contained, hard rule 12).

## Dependencies

- `2026-11-11-armada-l1-identity.md` (the principal), `2026-11-11-armada-l2-journal.md` (the audit home). Inherits ledger D0–D13.

## Design choices this layer settles (append to the ledger)

- **D14 — `Grant = (principal, dimension, scope, action-class)`; the closed
  enums; the store at `<home>/grants.json`; default-deny; the folder grant
  is the first row.** *Constrains:* L7 invocation, L8 control verbs, lane 2
  roles — all grant against this schema.
- **D15 — the uniform `guard.check` at `prepare_call`; lane-1 owner wildcard
  bootstrap (owner holds all).** *Constrains:* every action; L8 verifies the
  frame's principal at this guard.
- **D16 — per-principal declaration filter at `declaration_set`; an
  ungranted tool is not declared.** *Constrains:* L7 bot tool scope, L8
  dispatch, lane 2 (an unprivileged user's agent lacks governance tools).
- **D17 — every guard decision settles a `guard` journal event (principal,
  dimension, scope, class).** *Constrains:* L8 audit federation, lane 2
  per-user audit.
- **D18 — `Assertion {principal, class, scope, target, issued_at, nonce,
  signature}`, Ed25519 via `ring`; flagship private key, instance public
  key; step-up accepts in-turn approval (lane 1) or a valid assertion (org);
  custody + issuance at L8.** *Constrains:* L8 authority transport, lane 2
  per-user factors.
- **D19 — the governance layer (grant store, guard config, roles, org SOUL)
  is immutable to the agent; grant-making is the root privilege, a
  `governance` action, refused by name and audited.** *Constrains:* L8
  (flagship owns governance), lane 2 (org admin).

## Notes

This is the contract armada stands on. The guard, assertion format, audit,
and governance boundary are identical in lane 2 — only the number of
principals and the role templates change. The non-breaking bootstrap (owner
wildcard + in-turn approval as step-up) is what lets it land without
refusing anything the owner does today. Approve this and the ledger carries
D14–D19 into the upper layers.