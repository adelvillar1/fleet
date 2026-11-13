# Armada — the plan index

> **Status 2026-11-12: lane 1 COMPLETE.** All nine layers (L1–L9) delivered,
> gated, cut, and feed-verified — v0.6.31…v0.6.38 then the **v0.7.0** milestone
> (L9 completes the plan). All 40 atoms done; all nine plans `completed`.
> Lane 2 (the org) is the next generalization, not a rewrite.

The dependency-ordered plan set that fits fleet for armada, built bottom-up.
The vision is `../architecture/2026-11-11-armada-direction.md`; the
contract-first atomic decomposition is
`../architecture/2026-11-11-armada-atomic-decomposition.md`. Four ledgers
keep the execution honest — together they are the full understanding of how
the product is built:
- [`ARMADA-DECOMPOSITION.md`](ARMADA-DECOMPOSITION.md) — the **build map**: the
  40 atoms (L1.1…L9.5), each tracing to its plan, contract, choices, and
  dependencies, with the atom-level build order + critical path + status.
- [`ARMADA-DESIGN-DECISIONS.md`](ARMADA-DESIGN-DECISIONS.md) — the **choices**
  (D0–D57) that keep the plans from diverging.
- [`ARMADA-CONTRACTS.md`](ARMADA-CONTRACTS.md) — the **interfaces**, what each
  module OFFERS / EXPECTS / is CONSUMED BY, so the seams compose without
  guesswork — including the as-built record (§10a) of what v0.6.38–v0.7.0
  actually shipped.
- [`UI-DECISIONS.md`](UI-DECISIONS.md) — the **design system** (U0–U41): the
  signal-palette law, the shell/tissue layout contract, and the interaction
  law that keeps every surface one workspace — fleet-core, armada, and
  onboarding alike. The reference implementations are the live, interactive
  mockups under [`../design/`](../design/): `fleet-onboarding-mockup.html`
  (setup as the graph growing — fresh home to a whole workspace, fleet +
  armada, with the system-one step), `fleet-workspace-mockup.html` (desk,
  threads, learning, intelligence, calibration, inbox — with the Armada
  switch and the thread pane), and `armada-workspace-mockup.html` (board,
  fleet, dispatch). Open them before building any surface.

Read the decomposition to see *what to build and in what order*; the choices
before drafting; the contracts before building a module; the U-ledger and the
mockups before touching a surface.

## The U-reskin — the production faces (drafted 2026-11-12, awaiting activation)

- [`2026-11-12-ux-reskin.md`](2026-11-12-ux-reskin.md) — the plan: implement
  the approved design system across the production faces in six cuts
  (tokens+shell → Intelligence → Threads → Desk+Calibration → Onboarding →
  TUI+desktop), each citing the `U` ids it implements, the harness green
  between sections.
- [`2026-11-12-ux-reskin-handoff.md`](2026-11-12-ux-reskin-handoff.md) — the
  handoff packet: the owner's rulings verbatim, the face conventions, the
  endpoint inventory, the ten traps, and where to start. **The next agent
  starts there.**

## The workspace rebuild — the face becomes the mockups (drafted 2026-11-12, awaiting activation)

- [`2026-11-12-workspace-rebuild.md`](2026-11-12-workspace-rebuild.md) — the
  plan that supersedes the reskin's shell assumptions, after the audit in
  [`../architecture/2026-11-12-ux-audit-and-rebuild.md`](../architecture/2026-11-12-ux-audit-and-rebuild.md):
  the mockups are one workspace over one graph; production is 21 panels whose
  graph reports **22 nodes and 0 edges**, and whose work surfaces are empty
  because real work lands in sessions that are not objects. Seven phases,
  dependency-ordered (contract → spine → module split → shell → lenses →
  type/colour → language → delete). The owner's rulings: **the work item is
  the unit of record**; the 12 destinations with no place in the approved
  design move to **Settings → Advanced**; the scope is the **full rebuild**.

## How these plans are drafted (the awareness rule)

Bottom-up, one wave at a time. **Every plan reads the ledger's settled
choices below it and appends its own.** A plan carries two extra sections:
*Design choices inherited* (the `D` ids it conforms to) and *Design choices
this layer settles* (the new `D` ids it adds to the ledger). Within a wave
the layers are independent (they share only the bedrock choices already in
the ledger), so they may be drafted in parallel; across waves the order is
forced — a wave reads the choices the previous waves settled.

## Sequencing rationale (owner directive, 2026-11-11)

Armada is a new bottom-up build of layers fitted into fleet, each exposing
well-defined contracts armada consumes, infused with fleet's principles
(responsible citizen, trust earned by measurements). The owner's rule:
**L1 informs L2's choices, and so on — no plan may contradict the layers
below.** The bottleneck is **Identity (L1)**: nearly every layer needs the
principal or the install-id, so it is built first. The journal (L2) is the
second bedrock — the durable home security's audit and communication's bus
ride. Security (L3) is the load-bearing foundation the owner named, built
principal-agnostic from atom one so the org is configuration, not a retrofit.

## The waves and the dependency table

| Wave | Layer | Plan | Depends on | Unblocks |
|---|---|---|---|---|
| 0 | done | memory pull + git sync; capacity | — | L4 |
| 1 | L1 Identity | `2026-11-11-armada-l1-identity.md` | — | L3,L4,L5,L6,L7,L8 |
| 1 | L2 Journal | `2026-11-11-armada-l2-journal.md` | L1 (install id on events) | L3,L5,L7 |
| 2 | L3 Security | `2026-11-11-armada-l3-security.md` | L1, L2 | L5,L7,L8 |
| 2 | L6 Relay transport | `2026-11-11-armada-l6-relay.md` | L1, L3 (control frame) | L8 |
| 3 | L4 Corpus | `2026-11-11-armada-l4-corpus.md` | L1 | L8,L9 |
| 3 | L5 Communication | `2026-11-11-armada-l5-communication.md` | L1,L2,L3 | L8 |
| 3 | L7 Workspace | `2026-11-11-armada-l7-workspace.md` | L1,L2,L3 | L8 |
| 4 | L8 Control plane | `2026-11-11-armada-l8-control-plane.md` | L1,L3,L4,L5,L6 | L9 |
| 5 | L9 Deployment + surfaces | `2026-11-11-armada-l9-deployment.md` | L4,L8 | — |
| 2 | lane 2 org | *(deferred — a generalization of L1/L3/L5/L7)* | all lane 1 | — |

```
Wave 1   L1 ─┐  L2 ─┐          bedrock (independent)
Wave 2   L3 (L1,L2)   L6 (L1)  foundations
Wave 3   L4 (L1)   L5 (L1,L2,L3)   L7 (L1,L2,L3)   the mind + edges (independent)
Wave 4   L8 (L1,L3,L4,L5,L6)     the hand
Wave 5   L9 (L4,L8)              the body + face
```

## Status

All nine lane-1 layer plans are **drafted** (L1–L9), bottom-up, each reading
the ledger and appending its choices. The ledger holds **D0–D57** plus the
cross-cutting *trust-is-earned-by-measurements* obligation (a per-layer
verifiable row). Wave 3 (L4/L5/L7) was drafted in parallel — safe because the
layers share only the settled bedrock and each used its own D-id range.

All plans stay `draft` until the owner approves each. No plan is `active`
without the owner's "looks good." Lane 2 (the org) is deferred — a
generalization of L1/L3/L5/L7, not a rewrite.

**Drafted:** L1 identity, L2 journal, L3 security, L6 relay (waves 1–2);
L4 corpus, L5 communication, L7 workspace (wave 3); L8 control plane (wave 4);
L9 deployment + surfaces (wave 5).