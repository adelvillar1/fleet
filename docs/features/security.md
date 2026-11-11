# The security substrate — the authorization layer (armada L3)

The owner's example: a layer added to fleet that supports the armada vision
and exposes well-defined contracts armada consumes. It is principal-agnostic
from the first cut (D8): lane 1 has one principal (the owner, holding a
wildcard grant); the org (lane 2) multiplies principals and leaves this code
unchanged. Authority is code-enforced (R2) — a prompt is not a security
boundary.

## The model (R1–R5, D14–D19)

- **A grant** is `(principal, dimension, scope, action-class)` (D14). The
  dimensions (`project, storage, resource, corpus, tool, skill, workflow,
  loop, governance`) and classes (`read, write, dispatch, destructive,
  governance`) are closed sets; a scope is data (`instance`, `project:<id>`,
  `team:<id>`, `org`), not code. A `None` field is a wildcard.
- **The uniform guard** checks the grant default-deny (D15) at `prepare_call`,
  the one choke point every call crosses. The owner's wildcard is the lane-1
  bootstrap — nothing the owner does today is refused. A non-owner without a
  matching grant is refused by name.
- **Step-up** (D18): a `destructive`/`governance` action needs fresh human
  proof. Lane 1's in-turn approval (hard rule 7) satisfies it; the signed
  `Assertion` (Ed25519, `ring`) is the org-scale form — verified here, issued
  by the flagship at L8. The agent holds no signing key and cannot forge it.
  Per-action freshness (a 120s window + nonce), not a blanket window.
- **Governance** (D19): the grant store, guard config, roles, and org SOUL are
  immutable to the agent; grant-making is the root privilege. The agent is
  never declared a governance tool.

## The contract (for the layers above)

```text
Grant { principal, dimension?, scope?, class? }   // None = wildcard
Grant::owner_wildcard() / Grant::folder(project_id)
GrantStore::load(home) / save(home)               // <home>/grants.json, 0600
guard::check_grant(principal, dimension, scope, class, store) -> Decision
guard::check_step_up(class, in_turn_approved, assertion?, public_key?, now) -> Decision
Decision = Granted | Denied { name } | StepUpRequired
admits_tool(principal, tool, store) -> bool       // the declaration filter
Assertion::sign(key_pair, ...) / verify(public_key, now) -> bool
is_governance_target(path) -> bool
```

The control plane (L8) verifies the frame's assertion at this guard before
obeying; the workspace (L7) gates bot invocation through it; the org (lane 2)
assigns roles as grant templates. See `docs/plans/ARMADA-CONTRACTS.md` §4 and
`ARMADA-DESIGN-DECISIONS.md` D14–D19.

## Integration

- **`prepare_call`** checks the grant (owner wildcard → granted) and audits
  every decision to the journal as a `guard` event (D17) carrying principal,
  dimension, scope, class. A non-owner denial is a named, model-visible
  result. The step-up tier stays with the approval gate (lane 1's in-turn
  approval is the fresh proof).
- **`declaration_set`** filters tools per principal (D16): the owner sees
  every tool (unchanged); a non-owner sees only granted tools — an ungranted
  tool is not even declared.

## Surfaces

- **`fleet grants`** prints the owner wildcard bootstrap and the explicit
  grants per dimension.
- **The settings board** carries a grants card beside the identity card.

## Verification

`cargo test -p fleet --lib security::` pins: the owner wildcard grants every
class; default-deny refuses an ungranted principal by name; the folder grant
is a row `(owner, project, project:<id>, write)` and grants its principal;
per-principal tool admission (a principal sees only granted tools); step-up
accepts in-turn approval or a valid assertion and otherwise requires it; the
assertion verifies and rejects forged/stale/tampered; the governance layer is
named; a malformed store is an error not a silent reset. The full gate
(2211 lib + 172 integration + 16 install) and the face harness confirm
single-owner behavior is unchanged.