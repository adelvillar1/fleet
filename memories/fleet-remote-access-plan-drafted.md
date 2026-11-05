---
name: fleet-remote-access-plan-drafted
description: "2026-10-18 — the remote-access plan (docs/plans/2026-10-18-remote-access-lanes.md) is FULLY IMPLEMENTED and committed on the remote-access branch in the fleet-remote worktree, UNMERGED to master; increments R0-R8+R1b all landed 2026-10-18, the branch rebased three times onto newer master tips"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

After the design converged ([[fleet-lane-service-target-architecture]]), the
owner said "let's create a plan to implement it" and the plan was written to
**`docs/plans/2026-10-18-remote-access-lanes.md`** on 2026-10-18, following the
repo's plan conventions (frontmatter, Context/Approach/Acceptance Criteria/
Files/Out of scope/Verification/Linked artifacts). **The whole plan then landed
the same day**: every increment R0–R8 plus R1b is committed on the
`remote-access` branch, which lives in the **`/Users/alejandrodelvillar/Projects/fleet-remote`
worktree** — the plan-closes doc `f4f0e7e` and the recap `b0cff86` ("the remote
face lands, and a phone reaches fleet through a switchboard") are the branch
tip. **The branch is NOT merged to master**: master carries none of the
pairing/listener/inbox/channel/relay work, so an owner looking for the phone
face on master will not find it — it exists only on that branch. The branch was
rebased three times onto progressively newer master tips (0cfdd3e, 4ef5cae,
2340733); no commits were dropped.

**Increments R0–R8 (+R1b):** R0 workspace field on the session log (the
prerequisite for project grouping) · R1 pairing (`fleet pair`/`fleet devices`,
sha256-only device tokens in `~/.fleet/devices.json`, constant-time compare)
+ the remote listener with a default-deny route allowlist · R2 the session
registry in the bridge · R3 the phone layout + the inbox · R4 the channel
service with the webhook reference plugin · R5 seat arbitration ("answered
elsewhere") · R6 the GUI as a lane client · R7 the off-network relay · R8 a
second channel plugin (Telegram) proving the contract.

**The ordering insight that shaped the plan:** R1 delivers the phone experience
for the *current* session with **zero core refactor** — a paired phone gets
today's routes over a device token. The deep refactor (R2) is only required
once a phone must tap a *different* session without hijacking the desktop's
chat (today the bridge is one anchored session per launch). So the valuable
thing ships first and the risky thing is sequenced behind it.

**The refactor is smaller than it looks** (verified by reading the code while
planning): the bridge's agent thread holds only `current_session` plus the
prepared route as per-session state — every task already opens a fresh
`Session` and `Agent` from the log path (`bridge.rs:2939-3010`), and
checkpoints are already keyed per session (`checkpoints/<session>/<turn>`).
Multiplexing is therefore a keyed map with per-session seats and a `?session=`
parameter defaulting to the launch session, not a rewrite. The whole existing
suite must stay green with the desktop contract unchanged.

**Three judgment calls in the plan worth remembering:**
1. **Webhook, not Telegram, is the reference channel plugin** — it needs no
   external account and the tests drive it with curl; Telegram lands in R8 as
   the proof the plugin contract generalizes.
2. **The QR is not a blocker** — pairing works by typing a code (OpenMausBot's
   own fallback), and the ANSI half-block QR encoder is a separate increment
   (R1b) that can slip.
3. **The relay sees frame plaintext in R7** — the pairing secret gates it and
   end-to-end encryption (OMB's HPKE-pinned key) is a named later hardening
   increment, not silently assumed. Zero new crates before R7, and R7 has a
   zero-crate HTTP long-poll path (ureq is blocking; the bridge already spawns
   threads).

**The invariant is the first acceptance criterion:** with no channels
configured and no paired devices, every existing behavior is byte-for-byte
unchanged and none of R1–R8's code is on the path.

**Why:** the plan is the contract the next session implements against; the
increment order and the judgment calls are the parts a future session would
otherwise re-derive or get wrong.

**How to apply:** the code all exists — do not re-plan or re-derive it; to
work on the remote face, use the `fleet-remote` worktree (branch
`remote-access`), never master. The open questions are merge order and whether
master should adopt it. Never put peer policy in a surface — it belongs beside
`approvals::resolve`, which is also why the channel registry is a *sibling* to
`src/lanes.rs` (tool lanes) rather than an entry in it. Related:
[[fleet-lane-service-target-architecture]] (the design this plan implements),
[[fleet-phone-pairing-two-reference-architectures]] (the references),
[[gui-revamp-browser-face-decision]] (the `GROUPS` rail the inbox extends).
