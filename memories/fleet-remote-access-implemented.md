---
name: fleet-remote-access-implemented
description: "2026-10-18/20 — the remote-access plan (docs/plans/2026-10-18-remote-access-lanes.md) is COMPLETE: 14 commits on the unmerged branch `remote-access`, R0-R8 + the QR + the KI-022 merge, 1257 tests green; the phone pairing was enabled and verified live (66c9817 closed the two gaps the live pass found). The owner then ruled "port it to what we have in master now, plan first" — the port plan is committed on master (docs/plans/2026-10-20-remote-access-port.md, 4afc653, P0-P6) and NOT yet implemented; the branch …"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The plan drafted earlier the same day (recorded as
`fleet-remote-access-plan-drafted`, now replaced by this file) was
**implemented in full on 2026-10-18** and closed: the plan file is
`status: completed`, the feature doc is `docs/features/remote-access.md`,
and the recap is `docs/recaps/2026-10-18-the-remote-face-lands.md`.

**Fourteen commits on the branch `remote-access`, not merged to master**
(the other session held master for the KI ledger all day; the branch was
rebased onto master before each increment and again at the end). R0 the
workspace field on the log's `Start` record · R1 pairing (`fleet pair`,
`fleet devices`, sha256-only device tokens) + the default-deny remote
listener · R1b the hand-rolled QR encoder · R2 the bridge as a session
registry · R3 the phone layout + the project-grouped inbox · R4 the
channel service + webhook plugin · R5 seat arbitration · R6 the inbox's
channel/peer naming + the access card · R7 the outbound relay (with
`relay/relay.py` as the reference switchboard) · R8 the Telegram plugin ·
plus `a82113a`, the merge of master's KI-022 (lazy session creation) into
the registry.

**The phone pairing was then enabled and verified live** (the owner asked
"what do we need to do to enable the phone pairing"), and the live pass
found two real gaps in the seam *after* pairing, both fixed in `66c9817`:
the page appended `?token=` unconditionally, so the pairing page's
redirect to `/` asked for `token=null` and 401'd even though the cookie
was set — the gate now tries every place a token can ride, cookie first,
accepting the first that matches; and the remote allowlist covered the
API routes but not `/`, `/app.css` and `/app.js`, so a paired phone got
an inert shell (a face that looks fine and does nothing). Verified:
`fleet gui --remote`, open `/pair`, tap Pair, then the page, the
stylesheet, the script, the state and the archive all answer by cookie
alone, and a forbidden route still refuses by name. **Not yet verified on
real iOS hardware** — everything is curl against the real listener, and
the QR has not been scanned by a phone camera. Tailscale is the owner's
chosen transport and is now installed and paired on both devices (see
[[tailscale-remote-access]]); the iOS-safe path is `tailscale serve`
terminating TLS, because iOS ATS blocks cleartext HTTP to the CGNAT
range.

**Deferred, named in the plan and the feature doc, not silently
assumed:** end-to-end encryption on the relay (the relay sees frame
plaintext; the pairing secret gates the session), and streaming over a
relay (`/events` answers with the session's tail and the page polls —
a phone polls well, a held-open connection through a relay does not).

**The ordering insight held:** R1 shipped the phone experience for the
current session with zero core refactor, and the registry (R2) was
indeed a keyed map rather than a rewrite. **The judgment calls held
too:** webhook was the reference channel plugin and Telegram the proof;
the typed pairing code shipped before the QR; zero new crates across
every increment (the relay is HTTP long-poll over the `ureq` already in
the tree).

**Why:** the plan's own file is the source of truth for what landed;
this memory records that it is done, where the branch is, and what was
left out on purpose, so a future session does not re-plan the work or
assume the deferred pieces shipped.

**How to apply:** the code exists on the branch — do not re-derive it.
The port to master is planned, not started. Related:
[[fleet-phone-pairing-two-reference-architectures]] (the references the
design is the union of), [[fleet-lane-service-target-architecture]] (the
design this implemented), [[gui-revamp-browser-face-decision]] (the face
the inbox extends — and whose per-launch token master has since
removed), [[tailscale-remote-access]] (the off-network transport),
[[a-regression-test-must-fail-when-the-bug-returns]] and
[[a-relay-loops-sender-must-outlive-its-spawn]] (the two debugging
lessons this work paid for).

---

## 2026-10-20: the owner ruled — port to master; the port plan is committed

The owner asked "why is this on a different project folder? this should
be an integral part of the app", then answered the trust-model blocker
himself: **"port it to what we have in master now, plan first."** That
single instruction resolved both open questions from the earlier
session. (a) The trust model is the device token: the loopback face
keeps master's address-alone shape from `919416a` untouched, and the
remote face gates on pairing — because the ruling's own condition,
"authentication returns only when fleet becomes a multi-user app", is
exactly a paired device. (b) This session owns the port; the other
session's untracked `docs/plans/2026-10-20-bot-interface.md` (B1 = land
the transport) is deferred to where the two overlap, not done around.

**The port plan is committed on master**: `4afc653`,
`docs/plans/2026-10-20-remote-access-port.md`, status draft, increments
P0–P6. **Not yet implemented — P0 has not started.** Its load-bearing
decisions, which a future session must not re-derive or quietly reverse:

- **`src/gui/assets/*` resolves to master's editorial shell wholesale.**
  The branch's `#/channels` inbox is RETIRED, not ported — master's
  `#/sessions` archive becomes the inbox (project and channel ride the
  row, a `via a channel` filter joins the existing five) and a new
  `#/bots` surface is the control plane (service state, pending peers,
  peers, pair form, activity).
- **The bridge ports the branch's registry semantics onto master's**:
  session-scoped `run`/`plan`/`answer`, the per-session seat,
  `AnsweredBy`, the access read — while keeping everything master has
  grown (stop/steer, action cards, home and related reads, usage
  counters). This is the expensive file.
- **`src/session.rs` unions the fields**: master's `served_model` beside
  the branch's `workspace` and `Channel`.
- **Six defects are fixed in the port, not carried** (the branch's tests
  passed around all of them): KI-257 the swallowed listener bind error,
  KI-258 the webhook-only access read, KI-259 `Observe` unreachable,
  KI-260 the memory-only peer→session map (a restart started every peer
  a fresh conversation), KI-261 the parsed-and-ignored `--port`, KI-262
  no TUI surface.
- **The pairing token is the ONE credential allowed to cross a payload** —
  minted server-side, returned exactly once, shown in a one-time card,
  never in `/api/state`, the activity log, or the counters. The reading
  rule is amended to name that exception rather than bent silently.
- **A named narrowing of the owner's original ask**: they asked for
  sessions *grouped by project*, but master's archive is a flat list by
  the owner's own later editorial-console ruling, so the project rides
  the row and its filter instead of a grouping. The plan says so
  explicitly so it reads as a decision the owner can overrule, not a
  drift.
