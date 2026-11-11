---
status: completed
created: 2026-11-11
updated: 2026-11-11
slug: armada-l1-identity
---

# Plan: armada L1 — identity (install-id + principal), the bedrock

## Context

Armada is fitted into fleet bottom-up as a stack of responsible, measured
contracts (`../architecture/2026-11-11-armada-atomic-decomposition.md`).
Identity is the bedrock and the bottleneck: nearly every layer needs the
principal (the guard, the workspace assignee, the identity graph, the
control frame) or the install-id (the corpus cursor, the roster, deployment).
Today fleet has **neither** — `grep principal src/` is empty, and the only
ids are paired-device ids and ephemeral relay sids. This plan mints both,
with one principal (`owner`) now and the abstraction ready for many later.
It is atom L1.1–L1.4 of the decomposition.

## Design choices inherited (the ledger)

Conforms to `ARMADA-DESIGN-DECISIONS.md`: **D0** (InstallId stable, minted at
install), **D1** (PrincipalId the unit of authority, one `owner` now,
principal-agnostic), **D2** (device→principal, never reverse), **D3**
(responsible-citizen shape), **D4** (trust earned by measurements), **D5**
(the stack), **D6** (without breaking fleet — owner holds all, additive).

## Decision-model check

Deterministic identity plumbing — mint, persist, resolve, propagate. No
per-item classify/gate/score/extract decision; **no sys1 job fits**. The
grading loop (D4) enters at L3 (the guard grades authority); L1 only
provides the identity the guard reads.

## Approach

One new module, `src/identity.rs`, is the single home for install identity
and principal resolution. The install-id is minted once at `fleet install`
(16 random bytes via the existing `rand` crate → 32 hex, the same shape as
the relay sid), persisted atomically at `<home>/install-id`, and never
re-minted while it exists. The principal registry and the device→principal
map live in one `<home>/identity.json`; lane 1 holds exactly one principal,
`owner`, and every paired device maps to it. The principal rides the run as
a new `AgentOptions.principal` field (default `owner`), and a delegated
child inherits its parent's principal — never its own. This is authority,
deliberately distinct from `AgentOptions.identity` (the SOUL voice). The
guard (L3) will read this field; L1 only carries it.

High-level steps:

1. `src/identity.rs`: `InstallId`/`PrincipalId`/`DeviceId` types;
   `install()` reads-or-mints the persisted id; `principal_for()` resolves
   the run's principal (lane 1: `owner`); `device_principal()` reads the map;
   the `identity.json` store under the existing per-file flock.
2. `fleet install` mints the install-id (idempotent — a second run keeps it);
   `home.rs` gains the path helper.
3. `AgentOptions.principal` (default `owner`); thread it at the surfaces;
   `subagents.rs` child inherits the parent's principal.
4. Pairing writes the device→principal mapping (lane 1: → `owner`).
5. `fleet identity` CLI verb + the settings identity slot read the id and
   principal (the surface, hard rule 11).

## Acceptance criteria

- [x] C0: `fleet install` on a fresh home mints an install-id persisted at `<home>/install-id`; a second install run leaves it byte-identical; a different fresh home mints a different id.
- [x] C1: `identity::install()` returns the persisted id and `fleet identity` prints it.
- [x] C2: `identity::principal_for()` resolves `owner` for every lane-1 run, and `identity.json` holds exactly one principal.
- [x] C3: every run carries `AgentOptions.principal` (default `owner`), and a delegated child's principal equals its parent's (a test pins the inheritance).
- [x] C4: a paired device resolves to `owner` via `identity::device_principal()`, and the mapping persists in `identity.json` across restart.
- [x] C5: single-owner behavior is unchanged — the four-leg gate passes and no existing run is refused (the principal is additive; owner holds all).
- [x] C6: `fleet identity` and the settings identity slot render the install-id and the principal.

## Files to be touched

**Backend:**
- `src/identity.rs` — new: the types, the store, resolution.
- `src/lib.rs` — `mod identity`.
- `src/install.rs` — mint the install-id (idempotent).
- `src/home.rs` — the `install-id` / `identity.json` path helpers.
- `src/agent.rs` — `AgentOptions.principal` + propagation.
- `src/subagents.rs` — child inherits the parent's principal.
- `src/remote/pairing.rs` — write the device→principal map on pair.
- `src/cli.rs`, `src/main.rs` — the `fleet identity` verb.
- `src/gui/bridge.rs`, `src/settings.rs` — the identity slot reads.

**Tests:**
- `src/identity.rs` (module tests) — mint stability, resolution, inheritance, the device map.

**Documentation:** see Linked artifacts.

## Out of scope

- The grant store, the uniform guard, the assertion (L3).
- Multiple principals / the user store (lane 2).
- 2FA / the second factor (L3 format, L8 issuance).
- The event journal (L2) — the install-id only *appears* on events once L2 lands.

## Verification

**C0/C1:**
```bash
FLEET_HOME=$(mktemp -d) cargo run --bin fleet -- install
FLEET_HOME=$(mktemp -d) cargo run --bin fleet -- identity   # prints the id
```
Expected: a 32-hex id; re-install unchanged; a second home differs.

**C2/C3/C4:** `cargo test -p fleet --lib identity` — resolution, child
inheritance, device-map persistence.

**C5:** the four-leg gate (`cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`).

**C6:** `fleet identity` renders; the settings identity slot shows the id
and principal.

## Linked artifacts

- `docs/features/identity.md` — new: the install-id + principal contract.
- `TECHNICAL-DOCUMENTATION.md` — the identity contract (install-id, principal, device map).
- `ARMADA-DESIGN-DECISIONS.md` — append D7–D9 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L1 atoms ticked when shipped.

## Risks

- **`AgentOptions` field fan-out** — adding `principal` breaks every hand-built initializer. Mitigation: default `owner`, update every constructor in the same commit; `scripts/edit-preflight.sh` names the sites.
- **principal vs identity confusion** — `AgentOptions.identity` is the SOUL voice; `principal` is authority. Keep them distinct in code and docs.
- **install-id stability** — keyed to the home, so a changed `FLEET_HOME` is a new install (correct: a new home is a new instance).

## Dependencies

- None (bedrock). Inherits ledger D0–D6.

## Design choices this layer settles (append to the ledger)

- **D7 — InstallId = 32 hex (16 random bytes, `rand`), minted once at
  install, persisted `<home>/install-id`, read by `identity::install()`.**
  *Constrains:* L4 cursor/provenance, L8 roster key, L9 deployment.
- **D8 — PrincipalId = a lowercase slug; lane 1 = `owner`; the registry +
  device map live in `<home>/identity.json`; `src/identity.rs` is the single
  home (`principal_for`, `device_principal`).** *Constrains:* L3 guard, L5
  identity graph, L7 assignee, L8 frame, lane 2.
- **D9 — the principal rides `AgentOptions.principal` (default `owner`); a
  child inherits its parent's; distinct from the SOUL `identity`; the guard
  reads it.** *Constrains:* L3 guard input, L7 bot deputy, L8 frame principal.

## Notes

The first armada plan. It is deliberately small and additive — it changes
no authority semantics (that is L3), it only gives every action an identity
to carry. The owner-holds-all bootstrap (D6) means nothing the owner does
today is refused. Approve this and the ledger carries D7–D9 into L2.