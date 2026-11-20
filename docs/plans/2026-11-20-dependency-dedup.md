---
status: draft
created: 2026-11-20
updated: 2026-11-20
slug: dependency-dedup
---

# Dependency dedup: pin what is pinnable, name the rest

`cargo tree -d` reports 48 duplicate-version groups at `master` 60ee125 —
`base64` ×3, `getrandom` ×3, `hashbrown` ×3, `bitflags` ×2,
`core-foundation` ×2, `fastrand` ×2, and onward. Most are transitive
semver ranges outside this crate's control; a few are direct deps pinned
below what the tree already resolves.

## Context

- The direct `Cargo.toml` pins `base64 = "0.22"` while the tree carries
  0.21.7, 0.22.1 and 0.23.1 — at least one of those comes from a chain
  (`aes-gcm`, `ureq`, `tauri`'s tree) that a pin here cannot move.
- This crate is crypto-adjacent in two places (the vault's
  AES-256-GCM, the relay's X25519/AEAD), so "swap a crate to unify a
  version" is explicitly out of bounds — the cost is real, the benefit is
  build time and audit surface only.
- CI is disarmed, so a dependency change's real gate is the local four
  legs plus the release build.

## Approach

1. Produce the table first: for each duplicate group, the version, the
   chain that requires it (`cargo tree -i <crate>@<version>`), and
   whether it is direct or transitive. Put the table in the Status pass.
2. Unify only where the range allows: bump a direct dep to the version
   the tree already resolves, or relax a pin to `^` where the crate's API
   use is stable across the range. Each such change names the crate and
   the versions before/after.
3. Do **not** change a crate to unify a version. For each remaining group,
   write one line naming the chain that holds it — so a future session
   does not re-investigate.
4. Re-run `cargo tree -d` and compare the group count and the group list;
   both numbers go in the plan and, if any group is worth watching, the
   ledger carries a note rather than a defect.

## Acceptance Criteria

- [ ] The duplicate table (group, versions, chains, direct/transitive) is
      recorded in the Status pass.
- [ ] At least one duplicate group is unified without introducing a new
      duplicate — or, if none is safely unifiable, the plan closes
      cancelled with that finding and the table stands as the record.
- [ ] The `cargo tree -d` group count before and after is recorded.
- [ ] No crypto crate (`aes-gcm`, `ring`, `x25519-dalek`, `hkdf`, `sha2`)
      changes version or is swapped.
- [ ] Release build and the full test suite pass; the binary-size delta
      is recorded.
- [ ] The full four-leg gate exits 0.

## Files to be touched

- `Cargo.toml` — direct-dep pins only
- `Cargo.lock` — the resolved tree

## Out of scope

- The `src-tauri` fixture's tree beyond what workspace unification
  already shares.
- Replacing any crate for a smaller tree.
- Vendoring.

## Verification

1. `cargo tree -d` before/after, both recorded.
2. `cargo build --release` and the binary size at both points.
3. The full four-leg gate once, at the end.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F7.
