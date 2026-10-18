---
status: draft
created: 2026-10-18
updated: 2026-10-18
slug: remote-access-lanes
---

# Plan: Remote access — pairing, channels, and the bot experience

## Context

The owner asked what it would take to reach Fleet sessions from a
phone, then named the two pieces the answer needs: the QR-to-Safari
remote-control flow he already uses in ZCode (a relay rendezvous plus a
mobile web UI that lists sessions grouped by project), and the
pairing/trust/transport model of OpenMausBot
(github.com/milind-soni/OpenMausBot). Three sessions of design work
converged on one shape, recorded in the
`fleet-phone-pairing-two-reference-architectures` memory:

- **A session has endpoints.** An endpoint is a (transport, peer
  identity) pair. The TUI, the `fleet gui` browser tab, a paired phone,
  and a Telegram chat are all endpoints of the same session.
- **Both experiences, always.** The deep chat experience (one owner,
  one session, owner-initiated) and the bot experience (an inbox across
  lanes and devices, externally-initiated intake) are two attachment
  patterns over one session table. The core has no modes.
- **The agent core is untouched.** A third surface is a new bridge, not
  a new agent — the seam the GUI revamp already proved (G1).

The owner's closing constraint, which this plan is built around: the
normal chat experience must remain exactly what it is today. The remote
and lane machinery is opt-in and loads no new code path when unused.

### What the code already holds (verified, not assumed)

- `src/gui/bridge.rs` exposes `sessions()`, `replay(id)`, `resume(id)`,
  `run(task)`, `answer(decision)` — the five operations the ZCode phone
  UI performs. `/events` is a `tokio::sync::broadcast`, so more SSE
  clients cost nothing.
- The agent thread's only per-session state is `current_session` plus
  the prepared route; every task opens a fresh `Session` and `Agent`
  from the log path (`bridge.rs:2939-3010`). Multiplexing is therefore
  a keyed map, not a rewrite.
- Checkpoints are already per-session (`checkpoints/<session>/<turn>`).
- `approvals::resolve` is the precedent for policy in one place; the
  remembered-rules writer is the one writer.
- The GUI rail is one `GROUPS` declaration in `app.js` that builds both
  the rail and the palette; a new surface is one entry plus a renderer.

### The gaps this plan closes

1. `SessionSummary` and the log's `Start` record carry no workspace, so
   sessions cannot be grouped by project — the view the owner asked for.
2. One token per launch, never persisted: no per-device identity, no
   hashed-at-rest store, no revocation.
3. `server::loopback_address` refuses any non-loopback `--host` by
   name; there is no remote listener and no route policy for one.
4. One anchored session per bridge: a phone tapping session B would
   hijack the desktop's chat, and two surfaces would race for the seat.
5. `src/lanes.rs` is a tool-lane registry (Python MCP children); a
   channel is a transport plugin with peer identity, reconnect, and
   inbound events — a sibling registry, not an entry in that one.

## Approach

Vocabulary, fixed once: a **channel** is a transport plugin (webhook,
Telegram, …); a **peer** is the identity on a channel's far end; a
**device** is a paired remote surface (the phone); an **endpoint** is a
(transport, peer-or-device) pair attached to a session.

Increments, each independently shippable and verified. R0–R2 need no
new crates. Only R7's relay has a dependency question, and it has a
zero-crate answer.

### R0 — the session log carries its workspace

`Start` gains `workspace: Option<String>` (the launch cwd, following
the `kind`/`parent` pattern: `#[serde(default,
skip_serializing_if = "Option::is_none")]`); `SESSION_FORMAT_VERSION`
stays 1 — a same-version addition, tolerant readers, an absent field
defaults. `SessionSummary` gains the field; `session::list` reports it.
`fleet sessions` and the GUI sessions panel show the project column.
Nothing else changes. This is the prerequisite for every grouped view
and is useful on its own.

### R1 — pairing and the remote listener

`src/remote/pairing.rs`: mint a device token from `/dev/urandom` (the
same entropy source the launch token uses), store **only its SHA-256**
in `~/.fleet/devices.json` (a stolen store is not a stolen fleet),
compare in constant time, single-use pairing codes with an expiry and an
attempt cap (OpenMausBot's numbers: 12 characters, five minutes, five
attempts — both the code and the token it mints burn together).
`fleet pair` prints the code and the URL; `fleet devices` lists and
revokes.

`src/remote/listener.rs`: a second listener beside the loopback one,
reusing `server.rs`'s HTTP primitives (widen their visibility; do not
copy them). Gated by device tokens, not the launch token. **Default
deny**: an allowlist of routes a remote device may call — `state`,
`events`, `sessions`, `session` (replay), `run`, `approve`, `resume`,
`plan`. Everything else 403s by name: a paired phone may never reach
settings writes, fact edits, skill approvals, or the file/tree pane
until an explicit later decision adds them. `fleet gui --remote` starts
it, auto-detecting the address the way OpenMausBot's `listener.ts`
ranks candidates (physical interfaces first, tailnet address when
Tailscale is present, virtual interfaces never first), with
`--remote-host` as the override. `loopback_address` and the default
`fleet gui` path are untouched.

**R1 delivers the phone experience for the current session with no core
refactor**: the paired phone gets today's chat surface — the same
routes, the same stream — over a paired token.

QR generation is deliberately not in R1. The pairing flow must work by
typing the code (OpenMausBot's own fallback: "open /pair on the address
you use and type the code"). A vendored ANSI half-block QR encoder is
R1b, a separate increment that can slip without blocking anything.

### R2 — the session registry (the core refactor)

`bridge.rs` becomes a registry of session workers. The agent thread's
`current_session: String` + `prepared: Option<Prepared>` become a map
keyed by session id; `Command::Run`/`Plan` carry the session they
belong to; each session worker owns its prepared route, its command
receiver, and its seat. The launch session is the map's first entry and
the default for every route, so **the desktop experience is
byte-for-byte what it is today**. `GuiState.seat` becomes per-session,
and `state`/`events` gain an optional `?session=` (absent means the
launch session — today's contract). The TUI keeps its own single-session
loop untouched; it is one entry in the same table by behavior.

Acceptance rides on the existing suite: the whole gate stays green with
no test rewritten for behavior, only for the new parameter's default.

### R3 — the phone web app and the inbox

A phone layout (dynamic viewport units, a composer that survives the
virtual keyboard, touch-sized approval targets) and the **bot inbox**:
sessions grouped by project from R0's field, each row naming its lane
and peer when it has one, tap to attach the existing conversation
surface to that session. In the GUI's rail this is one entry in the
`work` group beside `chat` and `sessions`, built from the same `GROUPS`
declaration. Because R2 made attachment per-surface, a phone attaching
to session B no longer moves the desktop's chat.

### R4 — the channel service

`src/channels/`: a sibling registry to `src/lanes.rs` — these are
transport plugins, not tool lanes, and conflating them would drag
pairing secrets and reconnect loops into a model built for stateless
stdio children. The plugin trait: connect/reconnect, inbound event
mapped to (peer, session, task), outbound send, and a seat presentation
(the channel's own approve/deny affordance). Peer identity and the
authorization policy — which peers may start tasks, which may answer a
seat, which workspaces they may see — live in one resolver, the
`approvals::resolve` discipline applied to peers.

A lane-originated task is an ordinary run in its own session log,
driven by the registry, with the lane's approver raising the seat on
the channel and the same seat visible in the GUI inbox — raised once,
offered everywhere, answered once. The reference plugin is **webhook**
(zero external accounts; tests drive it with curl; it exercises the
whole path). Telegram and Discord follow as R8: outbound long-poll over
the `ureq` already in the tree, no new crates.

### R5 — seat arbitration

First answer wins — the rule stays. What is added: the seat names its
session, the answer broadcasts an `answered` event carrying who
answered and from where, and every attached surface renders the
resolution instead of leaving a stale button. This is where the
multi-surface race closes, and it is small.

### R6 — the GUI as a lane client

The `channels` surface grows the lane half: reply from the desktop to a
peer (the GUI speaks as the bot on that lane), pairing and device
management, and the inbox's unread/attention states. The GUI is not
special here — it is one more client of the channel service, differing
from the phone only in trust (loopback owner versus paired device).

### R7 — the relay (off-network access)

Only if the owner wants the phone to reach Fleet with nothing installed
on the network. Fleet dials **outbound** to a hosted relay; the relay
rendezvouses the phone's connection with Fleet's and shuttles frames.
Zero-crate first: an HTTP long-poll held on a dedicated thread (ureq is
blocking; the bridge already spawns threads), the relay serving the
phone over SSE. `tokio-tungstenite` only if measurement says the
long-poll is not enough. The threat model, stated plainly: in R7 the
relay sees frame plaintext; the pairing secret gates it; end-to-end
encryption (OpenMausBot's HPKE-pinned key) is a later hardening
increment, not silently assumed.

### R8 — a second channel plugin

Telegram (or Discord), proving the plugin contract generalizes past the
reference implementation. Until this lands, the channel service is a
hypothesis with one data point.

## Acceptance Criteria

- **The invariant.** With no channels configured and no paired
  devices, every existing behavior is byte-for-byte unchanged, and none
  of R1–R8's code is on the path. Verified by the full suite plus a
  manual pass of `fleet gui`, `fleet chat`, and `fleet run`.
- R0: a session created in directory X lists and replays with
  workspace X; a pre-R0 log reads with the field absent; `fleet
  sessions` shows the project.
- R1: `fleet pair` prints a single-use code; redeeming it from a
  browser on the LAN yields a working session; a second redemption
  fails; `fleet devices` lists the device and revoke kills it; every
  non-allowlisted route answers 403 with the reason named; the
  loopback face still refuses a non-loopback `--host` by name.
- R2: the full gate green with the desktop contract unchanged; two
  surfaces attached to two different sessions see independent
  transcripts and independent seats; the launch session remains the
  default for a request with no `?session=`.
- R3: a phone-width viewport renders the inbox grouped by project;
  tapping a session attaches it without moving the desktop's chat; an
  approval raised on one surface shows as answered on the other.
- R4: a webhook delivery from an unpaired peer is refused by name; a
  paired peer's delivery starts a run in its own session log; its
  approval seat is answerable from the channel and from the GUI inbox;
  an answer on one closes the seat on both.
- R5: two surfaces race to answer one seat; the winner's decision
  lands, the loser sees "answered elsewhere" with who answered.
- R6: a reply sent from the GUI reaches the peer on the channel; a
  device revoked in the GUI stops reaching the listener.
- R7: with the relay up, a phone on cellular reaches the session; with
  the relay down, the desktop face is unaffected and says why.
- R8: the second plugin pairs, receives, and answers with no change to
  the channel service's core.

## Files to be touched

| Area | Files |
|---|---|
| Session log | `src/session.rs` (`Start`, `SessionSummary`, `list`) |
| Pairing | `src/remote/pairing.rs` (new), `src/cli.rs`, `src/main.rs` |
| Remote listener | `src/remote/listener.rs` (new), `src/gui/server.rs` (share primitives), `src/gui/mod.rs` |
| Session registry | `src/gui/bridge.rs` (the one large refactor) |
| Phone UI | `src/gui/assets/{index.html,app.css,app.js}` |
| Channels | `src/channels/{mod.rs,webhook.rs}` (new), `src/channels/telegram.rs` (R8) |
| Docs | `docs/features/remote-access.md` when it lands; this file's `status` |

No new crates before R7; R7 has a zero-crate path.

## Out of scope

- Native iOS/Android apps — the phone experience is the web app
  installed to the home screen, as in both references.
- Messaging platforms as the remote UI — channels are for inbound
  tasks and approvals, not for replacing the session view.
- End-to-end encryption on the relay (named as a hardening increment
  above), push notifications, voice, and desktop-to-desktop companion
  mode.
- The QR encoder as a blocker (R1b, separate).

## Verification

The repo's gate, run whole — never through a status-losing filter
(KI-014) — plus per-increment: R1 driven with `curl` against the
listener from a second machine; R3 verified in a real phone browser at
a real phone width, not a narrowed desktop window; R4 driven end to end
with a webhook delivery; R7 verified on cellular with the laptop on
another network. Every new module carries its own tests in the repo's
existing style (a live listener on an OS-picked port, a fresh temp
home), and the pairing store's tests cover the constant-time compare,
the single-use burn, and the expiry.

## Linked artifacts

- `docs/plans/2026-10-16-gui-revamp.md` — the face this extends; the
  `GROUPS` rail pattern and the G1 seam.
- `docs/plans/2026-10-13-fleet-slice-4-lane-tools.md` — the tool-lane
  model the channel registry sits beside.
- Memory: `fleet-phone-pairing-two-reference-architectures`,
  `zcode-remote-session-bot-gateway-architecture`,
  `gui-revamp-browser-face-decision`.
- External references: ZCode's `webRemoteControl` (app.asar) and
  OpenMausBot's `shared/pairing-link.ts`,
  `companion/src/{devices,listener,routes,endpoints}.ts`,
  `docs/self-hosting.md`, `docs/desktop-companion.md`.
