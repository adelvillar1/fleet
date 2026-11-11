# Armada — the decomposition ledger (the build map)

**The atom registry.** This is the third ledger, and the one that gives full
understanding of *how the product is built*. The three ledgers answer the
three questions of building:

| Ledger | Question | Unit |
|---|---|---|
| **ARMADA-DECOMPOSITION.md** (this) | *What are the pieces, and how do they fit/build?* | the **atom** (L1.1 … L9.5) |
| [ARMADA-DESIGN-DECISIONS.md](ARMADA-DESIGN-DECISIONS.md) | *What did we decide, and why?* | the **choice** (D0 … D57) |
| [ARMADA-CONTRACTS.md](ARMADA-CONTRACTS.md) | *How do the pieces connect?* | the **interface** (offers/expects) |

An **atom** is the smallest coherent buildable, verifiable increment — one
thing, with its own test and its own surface (hard rule 11). The product is
the composition of its atoms; the plans are how the atoms land; the choices
and contracts are what keep them composing. Every atom traces four ways:
**atom → plan** (where it's built), **atom → contract** (the interface it
implements), **atom → choices** (the D-ids it settles/uses), **atom → atoms**
(what it depends on).

How to read an atom row:
- **Atom** — the stable id (`L<layer>.<n>`).
- **Builds** — the one thing it builds.
- **Depends** — the atoms that must land first (the atom-level DAG).
- **Choices** — the D-ids it settles or uses (→ design-decisions ledger).
- **Status** — `planned → drafted → building → done`. All are `drafted` now
  (their layer plan is drafted; nothing built yet).

The contract each atom implements is its layer's section in
[ARMADA-CONTRACTS.md](ARMADA-CONTRACTS.md) (noted per layer). Layer-level
choices that span atoms are noted in the layer header.

## Wave 1 — the bedrock

### L1 Identity — plan `2026-11-11-armada-l1-identity.md` · contract §2
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L1.1 | mint the install-id (persisted, idempotent) | — | D7 | drafted |
| L1.2 | the PrincipalId type + owner bootstrap | — | D8 | drafted |
| L1.3 | per-run principal propagation (child inherits) | L1.2 | D9 | drafted |
| L1.4 | the device→principal map (pairing) | L1.2 | D8, D2 | drafted |

### L2 Journal — plan `2026-11-11-armada-l2-journal.md` · contract §3
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L2.1 | the Seq + Event type + sequencer | L1.1 | D10 | drafted |
| L2.2 | durable append (JSONL + fsync cadence) | L2.1 | D11 | drafted |
| L2.3 | durable named consumer + cursor | L2.2 | D12 | drafted |
| L2.4 | the journal behind `record()` (additive) | L2.2, L2.3 | D13 | drafted |

## Wave 2 — the foundations

### L3 Security — plan `2026-11-11-armada-l3-security.md` · contract §4
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L3.1 | the grant schema + store (folder grant = first row) | L1.2 | D14 | drafted |
| L3.2 | the uniform guard at `prepare_call` (owner bootstrap) | L3.1, L1.3 | D15 | drafted |
| L3.3 | the per-principal declaration filter | L3.1, L1.3 | D16 | drafted |
| L3.4 | guard audit → journal | L3.2, L2.4 | D17 | drafted |
| L3.5 | the assertion format + Ed25519 verification | L3.2 | D18 | drafted |
| L3.6 | the governance boundary (immutable to agent) | L3.2, L3.4 | D19 | drafted |
| L3.7 | step-up 2FA (in-turn approval lane 1; assertion org) | L3.5 | D18 | drafted |

### L6 Relay transport — plan `2026-11-11-armada-l6-relay.md` · contract §7
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L6.1 | the Rust broker (replaces relay.py) | L1.1 | D20 | drafted |
| L6.2 | push / ack / cursor | L6.1 | D21 | drafted |
| L6.3 | the control-frame discriminator | L6.2, L3.5 | D22 | drafted |

## Wave 3 — the mind + edges

### L4 Corpus — plan `2026-11-11-armada-l4-corpus.md` · contract §5
*Layer-level choices: D28 (sync graded by read-back), D29 (never-sync set).*
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L4.1 | the corpus remote + sync driver | L1.1 | D23 | drafted |
| L4.2 | the recall-count merge rule (sum/delta) | L4.1 | D24 | drafted |
| L4.3 | distributed supersede / serial-collision rule | L4.1 | D25, D26 | drafted |
| L4.4 | the RemoteInstance feeder over Source | L4.1, L4.2, L4.3 | D27 | drafted |

### L5 Communication — plan `2026-11-11-armada-l5-communication.md` · contract §6
*Layer-level choices: D35 (dispatch graded), D36 (relay is a channel).*
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L5.1 | the durable outbox | L2.2 | D30 | drafted |
| L5.2 | the identity graph (peer→principal) | L1.4, L3.1 | D31, D32 | drafted |
| L5.3 | the pub/sub bus over the journal | L2.3 | D33 | drafted |
| L5.4 | gated dispatch (threads the principal) | L5.2, L3.2 | D34 | drafted |

### L7 Workspace — plan `2026-11-11-armada-l7-workspace.md` · contract §8
*Layer-level choice: D43 (work graded against outcome).*
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L7.1 | the task object (id, status, assignee) | L1.2 | D37, D38 | drafted |
| L7.2 | the work-object edges + hand_to_desk fix | L7.1, L2.4 | D39 | drafted |
| L7.3 | surfaces as views over the graph | L7.2 | D40 | drafted |
| L7.4 | bot invocation as a workspace action (gated) | L7.2, L3.2 | D41, D42 | drafted |

## Wave 4 — the hand

### L8 Control plane — plan `2026-11-11-armada-l8-control-plane.md` · contract §9
*Layer-level choices: D49 (autonomy earned by measurement), D50 (relay-down degradation).*
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L8.1 | registration + roster + heartbeat | L1.1, L6.3 | D44 | drafted |
| L8.2 | the authority transport (sign/verify before obey) | L3.5, L3.2, L6.3 | D45 | drafted |
| L8.3 | the control vocabulary (reuses bridge verbs) | L8.1, L8.2, L4.4 | D46 | drafted |
| L8.4 | the flagship role (role + config, key custody) | L8.2 | D47 | drafted |
| L8.5 | the human second factor (issues the assertion) | L3.7, L8.4 | D48 | drafted |

## Wave 5 — the body + face

### L9 Deployment + surfaces — plan `2026-11-11-armada-l9-deployment.md` · contract §10
*Layer-level choice: D57 (deployment graded by doctor + settles).*
| Atom | Builds | Depends | Choices | Status |
|---|---|---|---|---|
| L9.1 | provision→register→join (local + VPS) | L4.4, L8.1 | D51, D52 | drafted |
| L9.2 | the `fleet armada` CLI verbs | L8.3 | D53 | drafted |
| L9.3 | the TUI armada board | L8.3 | D54 | drafted |
| L9.4 | the face armada section | L8.3 | D55 | drafted |
| L9.5 | fleet-desktop wraps the armada face | L9.4 | D56 | drafted |

## The atom-level build order

The waves are the layer order; within and across them the atom DAG forces
this sequence (an atom lands only after its Depends):

```
L1.1, L1.2 ─┬─ L1.3, L1.4
            ├─ L2.1 → L2.2 → L2.3 → L2.4
            ├─ L3.1 → L3.2 → {L3.3, L3.4, L3.5} → {L3.6, L3.7}
            ├─ L6.1 → L6.2 → L6.3
            ├─ L4.1 → {L4.2, L4.3} → L4.4
            ├─ L5.1 ; L5.2 ; L5.3 ; L5.4
            └─ L7.1 → L7.2 → {L7.3, L7.4}
                 then  L8.1, L8.2 → L8.3 → L8.4 → L8.5
                 then  L9.1 ; L9.2, L9.3, L9.4 → L9.5
```

**The critical path to armada** (the longest chain): `L1.2 → L3.1 → L3.2 →
L3.5 → L8.2 → L8.3 → L9.2`, with `L4.1 → L4.2/L4.3 → L4.4` feeding L8.3 and
`L6.1 → L6.2 → L6.3` feeding L8.1/L8.2. Identity and security are on the
critical path — which is why they are the bedrock and the bottleneck. The
journal (L2) and corpus (L4) branches run in parallel beside it.

**The first atom to build: L1.1** (mint the install-id) — no dependencies,
unblocks the journal, corpus, relay, and roster at once. L1.2 (the principal)
is its bedrock twin.

## Status summary

- **40 lane-1 atoms**, all `drafted` (their 9 layer plans are drafted; nothing
  built yet). 0 `building`, 0 `done`.
- The cross-cutting **trust-is-earned-by-measurements** obligation (design-
  decisions ledger) attaches a verifiable grading property to each layer; the
  atoms that discharge it: L3.4/L3.7 (security), L4.4 (corpus read-back),
  L5.4 (dispatch grade), L6.2 (ack/liveness), L7.4 (work grade), L8.1/L8.3
  (reliability), L9.1 (doctor+settles).

## Lane 2 — the org (deferred atoms, a generalization)

Not yet decomposed into plans; each consumes the unchanged lane-1 contracts:
- **O1** user store + device→principal mapping (many) — extends L1 (D8).
- **O2** roles as grant templates — extends L3 (D14).
- **O3** per-user 2FA factors — extends L3/L8 (D18, D48).
- **O4** team/org scopes — already data in L3 (D14).
- **O5** team-workspace surfaces (tasks/docs/chat/mail) — extends L7 + L5.

None is a rewrite — the guard, assertion, audit, and governance boundary are
unchanged. Lane 2 is drafted when lane 1 is built and the owner says go.

## Pointers

- The contracts: `../architecture/2026-11-11-armada-atomic-decomposition.md` (the prose decomposition this registry tracks).
- The choices: [ARMADA-DESIGN-DECISIONS.md](ARMADA-DESIGN-DECISIONS.md).
- The interfaces: [ARMADA-CONTRACTS.md](ARMADA-CONTRACTS.md).
- The plans + waves: [PLAN-INDEX.md](PLAN-INDEX.md).