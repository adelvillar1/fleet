# The control plane — the hand (armada L8)

The layer the whole stack below exists to serve. A flagship — a fleet install
in a flagship **role** (role + config, one binary, never a fork) — commands
instances over the relay. Authority rides *in the frame*, signed by the
flagship's Ed25519 key and verified at the instance's guard before it obeys;
the relay stays a dumb broker (L6). An instance earns dispatch autonomy by
measured reliability, never by registration. `src/armada/` composes every
lower contract and re-implements none.

## Registration + roster + heartbeat (D44)

`roster.rs`: an instance registers on join with its install-id (L1), sid,
version, and capacity (the contract's reading). The flagship roster lives at
`<home>/roster.json`. **Liveness is earned by heartbeats, not assumed** — a
sid with no heartbeat within the window is reported not-live. A heartbeat
cannot register an unknown sid (that is the join's gated act). An install with
no roster is an ordinary single-instance fleet, unchanged.

## The authority transport (D45)

`authority.rs`: every control frame carries the principal and, for
`destructive`/`governance` verbs, a fresh assertion (L3) **signed by the
flagship's Ed25519 key**. The instance verifies the signature *before* it
obeys — a forged, unsigned, stale, or principal-mismatched assertion is
refused by name. The relay cannot fabricate (it holds no key), an instance
cannot escalate (it verifies against the flagship's public key), and an agent
cannot borrow (the private key lives only at the flagship).

**Key custody (D47):** the flagship mints the Ed25519 key once, persists the
PKCS#8 `0600` at `<home>/armada/flagship-key.pkcs8` and the raw public key at
`flagship-key.pub`. An instance holds only the public key (distributed
out-of-band) — it can verify, never sign. Rotation/revocation is an open fork
flagged for the owner.

## The control vocabulary (D46)

`control.rs`: each verb reuses a bridge verb and runs through the instance's
own guard (D15) and admission, after authority is verified:

| Verb | Reuses | Class |
|---|---|---|
| `dispatch` | `bridge.run` (gated dispatch, L5) | Dispatch |
| `state` | `bridge.state` | Read |
| `corpus-sync` | L4 sync | Write |
| `version-roll` / `retire` | deployment (L9) | Destructive |
| `health` | the health probe | Read |
| `register` / `heartbeat` | the roster | Governance |

An ungranted verb is refused by name and settles a `guard` journal event; a
privileged verb also clears step-up (D18) — the human second factor issues the
fresh per-action assertion the agent cannot forge (D48). The flagship cannot
overwhelm a small VPS: the instance admits for itself.

## Autonomy earned by measurement (D49)

A fresh instance is **un-calibrated until it settles runs**. A scheduled
reliability beat grades dispatch outcomes over the journal (never a
synchronous sys1 ask — the socket guards); a dispatch is graded against its
outcome before the pattern changes.

## Relay-down is a named degradation (D50)

Instances run autonomously, the corpus still syncs over git (L4 is independent
of the relay), and live control resumes when the relay returns — never a
silent stop.

## Surface

`fleet armada {roster|status|sync}` is the first surface (terminal-first): the
roster with earned liveness, an instance's measured reliability, and the corpus
sync. The richer dispatch verb, TUI board, face section, and Tauri shell are L9.

## Verification

`cargo test -p fleet --lib armada` pins registration/liveness, signature
verify + forge/unsigned/stale refusal, key custody (flagship only, 0600), verb
classes, admission gating + audit, the second-factor step-up, corpus-sync
reaching L4, the reliability grade (un-calibrated until settled), relay-down
named, and the no-roster install unchanged. The full gate confirms a
single-instance fleet behaves as today.