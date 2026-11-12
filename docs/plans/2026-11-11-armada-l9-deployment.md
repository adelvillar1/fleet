---
status: completed
created: 2026-11-11
updated: 2026-11-12
slug: armada-l9-deployment
---

# Plan: armada L9 — deployment + the surface progression, the body and the face

## Context

The last lane-1 layer: armada's body (deployment of the self-contained
payload) and its face (the surface progression fleet already walked —
CLI/TUI → face/PWA → Tauri). A fresh install inherits the corpus by cloning
the remote with a fresh install-id and registers with the flagship; the
control plane (L8) gains its surfaces, one implementation four surfaces. The
deployment is a responsible citizen too: a fresh install is trusted only
after it passes doctor and settles runs — never by arrival. Atoms L9.1–L9.5.

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared), **D7** (install-id), **D23–D29** (corpus:
sync D23, the never-sync set + fresh-install clone D29), **D44–D50** (control
plane: roster D44, the vocabulary D46, the flagship role D47, autonomy earned
D49). Discharges the **cross-cutting obligation's L9 row**: a fresh install
is trusted only after doctor + settles. Honors **D6**: the payload is the
self-contained binary (hard rule 12); the surfaces reuse fleet's stack (§9).

## Decision-model check

Deterministic provisioning + surface rendering. The reliability grade is
L8's scheduled beat (D49), not a provisioning-time ask. **No sys1 head fits.**

## Approach

`src/armada/deploy.rs` provisions a target: a **local spawn** lays out a new
home directly; a **VPS** is reached over SSH (or a cloud API) and dropped the
same self-contained binary (the release raw-binary channel). The target runs
`fleet install`, mints a fresh install-id (D7), **clones the corpus remote**
(inheriting the mind, D29 — identity is not in the corpus), and registers
with the flagship (D44). The fresh install is un-calibrated until it settles
(D49). The surfaces follow §9's progression over the one `src/armada`
library: the **TUI armada board** (instances, health, capacity, dispatch),
the **face armada section** (roster board, dispatch composer, corpus view —
the centre-conversation + context-pane shape, the chat composer borrowed
never rebuilt), and **fleet-desktop** wrapping the same face (no second
frontend, no second transport). The CLI gains `fleet armada deploy` beside
L8's verbs. Each stage ships its surface at the "genuinely visual" bar (hard
rule 11).

High-level steps:

1. `src/armada/deploy.rs`: provision (local spawn + VPS drop), the payload
   from the release channel; credentials out-of-band (env/vault).
2. `fleet install` on the target: fresh install-id + corpus clone + register.
3. The TUI armada board (ratatui pane).
4. The face armada section (roster, dispatch composer, corpus view).
5. fleet-desktop wraps the armada face.
6. `fleet armada deploy` + the surface harness pins.

## Acceptance criteria

- [x] C0: provision drops the self-contained binary on a target (local spawn and VPS over SSH) and runs install; the fresh install mints a new install-id and clones the corpus remote (inherits the facts, not the identity).
- [x] C1: the fresh install registers with the flagship roster and is reported un-calibrated until it settles runs (a test pins it).
- [x] C2: the TUI armada board renders instances, health, capacity, and dispatch (genuinely visual, not a plain-text table).
- [x] C3: the face armada section renders the roster board, dispatch composer, and corpus view in the centre-conversation + context-pane shape; the face harness pins it.
- [x] C4: fleet-desktop wraps the armada face over the same server/bridge (no second frontend, no second transport).
- [x] C5: `fleet armada {roster|dispatch|status|sync|deploy}` drives the control plane headlessly.
- [x] C6: provision is a governance action (step-up, D18/D19); a fresh install is trusted only after it passes doctor and settles runs (graded, not by arrival).
- [x] C7: the four-leg gate passes, the face harness is green, and a single-instance fleet is unchanged.

## Files to be touched

**Backend:**
- `src/armada/deploy.rs` — new: provision (local + VPS).
- `src/install.rs` — fresh-install corpus clone + register.
- `src/cli.rs`, `src/main.rs` — `fleet armada deploy` + verbs.

**Frontend / surfaces:**
- `src/tui/` — the armada board.
- `src/gui/assets/app.js`, `src/gui/server.rs` — the face armada section.
- `src-tauri/` — fleet-desktop wraps the armada face.

**Tests:**
- `src/armada/deploy.rs` — provision, inheritance, registration, the grade.
- `tests/face/harness.mjs` — the armada section captures.

**Documentation:** see Linked artifacts.

## Out of scope

- The org / multi-user team surfaces (lane 2).
- SSH/QUIC alternative transport, cloud provisioning beyond SSH (later options).
- The control-plane logic (L8) — L9 renders and deploys it.
- Signing-key rotation/revocation UX (the custody fork, flagged for the owner).

## Verification

**C0/C1:** `cargo test -p fleet --lib armada` — provision, corpus inheritance,
registration, un-calibrated-until-settles.

**C2/C3/C4:** the TUI render test; the face harness (`tests/face/harness.mjs`)
pins the armada section; fleet-desktop builds over the same server.

**C5:** `fleet armada …` drives the control plane headlessly.

**C6:** a test pins provision as governance (step-up) and the doctor+settles
grade.

**C7:** the four-leg gate + the face harness; a single-instance install is
unchanged.

## Linked artifacts

- `docs/features/armada.md` — extend: deployment + the surfaces.
- `FUNCTIONAL-SPECIFICATIONS.md` — the armada surface (roster, dispatch, corpus view).
- `TECHNICAL-DOCUMENTATION.md` — the deployment contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D51–D57 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L9 atoms ticked when shipped.

## Risks

- **VPS credentials** — SSH/cloud access must resolve out-of-band (the own-home ruling), never stored in the corpus. Mitigation: env/vault only; the corpus carries no credentials (D29).
- **The surface progression is UX-heavy** (hard rule 11). Mitigation: each stage ships its surface at the visual bar; the face harness pins it; the chat composer is borrowed, never rebuilt.
- **Fresh-install clone conflicts** — two installs syncing at once. Mitigation: L4's merge rules (D24–D26) handle it; the clone is a bootstrap pull (content dedup makes it safe).

## Dependencies

- `2026-11-11-armada-l4-corpus.md` (the corpus inheritance), `2026-11-11-armada-l8-control-plane.md` (the roster + vocabulary it deploys and renders). Inherits ledger D0–D50.

## Design choices this layer settles (append to the ledger)

- **D51 — provision→register→join: a local spawn lays out a new home; a VPS
  is reached over SSH/cloud and dropped the self-contained binary (the
  release raw-binary channel); the target runs install, mints a fresh
  install-id (D7), clones the corpus remote (D29), registers (D44);
  credentials out-of-band.** *Constrains:* lane 2 (org provisioning).
- **D52 — fresh-install corpus inheritance: clone the remote with a fresh
  install-id (identity is not in the corpus, D29); the install starts
  un-calibrated until it settles (D49).** *Constrains:* the shared mind's
  distribution.
- **D53 — the surface progression (§9): CLI/TUI → face/PWA → Tauri over the
  one `src/armada` library; one implementation, four surfaces; UX co-equal
  (hard rule 11), genuinely visual.** *Constrains:* lane 2 team surfaces.
- **D54 — the TUI armada board: instances, health, capacity, dispatch (a
  ratatui pane).** *Constrains:* the terminal-first surface.
- **D55 — the face armada section: roster board, dispatch composer, corpus
  view; the centre-conversation + context-pane shape; the chat composer
  borrowed never rebuilt.** *Constrains:* the PWA surface, lane 2.
- **D56 — fleet-desktop wraps the armada face over the same server/bridge (no
  second frontend, no second transport).** *Constrains:* the native shell.
- **D57 — deployment graded by measurement: provision is a governance action
  (step-up); a fresh install is trusted only after doctor + settles (the
  cross-cutting obligation).** *Constrains:* the trust boundary at the edge.

## Notes

The body and the face. Deployment is fleet deploying fleet — fully
self-contained, no foreign control-plane service. The surfaces are fleet's
proven progression reused, not a parallel app. With L9, lane 1 is complete:
every contract armada needs works for one owner, and lane 2 (the org) is a
generalization — more principals, roles as grant templates, per-user factors,
team/org scopes — not a rewrite. Approve this and the ledger carries D51–D57.