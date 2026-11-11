# Identity — the install-id and the principal (armada L1)

Fleet's identity bedrock: every install has a stable id, and every run acts
for a principal. This is armada's first layer (L1) — the substrate the
guard (L3), the corpus cursor (L4), the identity graph (L5), the roster
(L8), and deployment (L9) all key on. It carries identity; it grants nothing
and refuses nothing — authority is the guard's job (L3).

## The two identities

- **The install-id** (`src/identity.rs`, D7): 32 lowercase hex characters,
  minted once from the OS entropy source, persisted at `<home>/install-id`.
  Idempotent — a home that holds an id keeps it byte for byte; a fresh home
  is a fresh install. `fleet install` mints it after the layout creates the
  home; `identity::install()` read-or-mints on first use, under the file's
  flock with a re-check after the lock so two racing writers converge on one
  id.
- **The principal** (D8): the unit of authority an action acts for, distinct
  from the device it arrives on. Lane 1 has exactly one, `owner`.
  `identity::principal_for()` resolves it; every contract is
  principal-agnostic so the org (lane 2) multiplies principals without
  changing the code.

A **device** maps to a principal, never the reverse (D2). The registry
(`<home>/identity.json`) holds the principals and the device→principal
edges, written atomically and privately (0600) under the flock — the device
store's own discipline. An unlinked device resolves to `None`, never a
silent fallback to the owner; the communication layer (L5) turns that into a
named refusal.

## How the principal rides the run

`AgentOptions.principal: Option<PrincipalId>` (D9), `None` being the owner —
the run this crate has always had. It is distinct from `AgentOptions.identity`
(the SOUL voice): authority is not voice. A delegation child inherits its
parent's principal through the `Delegation` spec, the way it inherits the
voice and the file access — the authority follows the run, never the actor.
Lane 1 leaves every surface `None`, so behavior is unchanged.

## Surfaces

- **`fleet identity`** prints the install-id, the principal, and the paired
  devices linked to a principal.
- **`fleet install`** mints the id and names it (`fleet: install: identity:
  <id>`).
- **The settings board** carries an identity card beside the SOUL slot
  (`settings-install-identity`): the install-id, the principal, and the
  paired-device count, read from the settings document's `identity` object.
- **Pairing** links a newly paired device to the owner (`link_device`); a
  link failure is named, not silent, and does not undo the pairing.

## The contract (for the layers above)

```text
identity::install() -> InstallId                       // read-or-mint, idempotent
identity::principal_for(home) -> PrincipalId           // lane 1: owner
identity::device_principal(home, device) -> Option<PrincipalId>
identity::link_device(home, device, principal) -> Result<()>   // governance
identity::Registry::load / save                        // the store
```

See `docs/plans/ARMADA-CONTRACTS.md` §2 for the offers/expects, and
`docs/plans/ARMADA-DESIGN-DECISIONS.md` D7–D9 for the choices.

## Verification

`cargo test -p fleet --lib identity` pins: mint stability + a fresh home
mints a different id, persistence (the id lives on disk), lane-1 resolution
to the owner, device-link resolution + reload persistence, upsert (no
duplicate edges), and a malformed registry is an error not a silent reset.
The install integration tests pin the mint rides install; the face harness
pins the settings card.