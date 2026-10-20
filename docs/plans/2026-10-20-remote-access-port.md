---
status: draft
created: 2026-10-20
updated: 2026-10-20
slug: remote-access-port
---

# Plan: Port the remote face onto the editorial console

## Context

The remote-access work landed complete and green on the `remote-access`
branch (`66c9817`, worktree `../fleet-remote`): the workspace field, the
pairing store, the remote listener with its default-deny allowlist, the
QR encoder, the session registry, the channel service with webhook and
Telegram plugins, seat arbitration, and the relay. Master has none of
it, and master has since moved 26 commits: the editorial console
(`ddb8724`, `497b60b`, `cd8ad1a`), the TUI revamp (`e8ebdec`,
`968471c`), stop and steer, the action cards, the usage counters, and —
decisively — `919416a`, which removed the per-launch token from the
loopback face on the owner's ruling that the machine is the boundary and
authentication returns only when fleet becomes multi-user.

The port is therefore not a merge. It is a re-derivation of the branch's
semantics on top of master's structure, with one substantive decision
the owner's ruling forces.

## The one decision: what gates the remote face

`919416a` makes the loopback face open to any browser on the machine.
The remote face cannot inherit that: a phone is not this machine, and
the port it reaches is routable.

The owner's ruling on what that gate *is*: a paired device is not a
second user — it is the same user with access to the existing sessions.
Pairing is at most a handshake token shared between devices. So the
device token is not authentication in the multi-user sense the ruling
deferred; it is a **capability the owner hands to a device he owns**, a
secret shared between two of his own machines, and its whole job is to
keep an unpaired network peer off a routable port. Consequences that
follow and are load-bearing:

- The loopback face keeps master's shape exactly: no token, Host and
  Origin checks, the address alone opens it. Untouched.
- The remote face gates on the pairing handshake, as the branch built
  it — but the store's shape is renamed to say what it is: a handshake
  secret per device, digested at rest, revocable. Not a user account,
  not a role, not an identity. There is one user; there are N devices.
- Everything a paired device may reach is everything the owner may
  reach from the loopback face over the same sessions. The remote
  allowlist is a *route* boundary (a phone cannot write settings), not
  an *identity* boundary. There is no per-device permission model,
  because there is no second user to have permissions.
- The channel peers are the one place a genuinely foreign identity
  arrives, and there the policy stays what it was: `may act` /
  `may only listen` per peer, pending peers, revocable. That is a
  channel's authorization, not the remote face's.

This is the only place the two faces disagree, and it is a disagreement
about which machine is asking rather than about who is asking.

## The second decision: one archive, not two

The branch's `#/channels` inbox is retired rather than ported. Master's
`#/sessions` is already the archive — page head, stat strip, filter bar,
the split list — and a second archive surface is the duplicate the
editorial console removed. Sessions becomes the inbox: the project and
the channel ride the row, and a `via a channel` filter chip joins the
existing five. A new `#/bots` surface, first in the system group, is the
control plane: the service's state, who can reach it, how to let someone
in or throw them out, what they have done, and where their conversation
is. The owner asked for sessions grouped by project; master's archive
is a flat list by the owner's own later ruling, so the project rides the
row and its filter rather than a grouping the console does not have.
That is a real narrowing of the original ask and it is named here rather
than papered over.

## The increments

### P0 — the transport, by rule

Merge `remote-access` and resolve each conflict by rule, never by
picking a side wholesale:

- `src/gui/assets/{app.css,app.js,index.html}` — **master's editorial
  shell wholesale.** The branch's `#/channels` panel, its inbox CSS, and
  its phone media query are not taken; P3/P4 add only the classes the
  new surfaces need.
- `src/gui/bridge.rs` — **port the branch's registry semantics onto
  master's bridge**: session-scoped `run`/`plan`/`answer`, the per-session
  seat, `AnsweredBy`, the access read. Keep master's stop/steer, the
  action cards, the home and related reads, and the usage counters. This
  is the expensive file and the reason this increment is first.
- `src/gui/{mod.rs,server.rs}` — **master's structure**, then the
  channel block and the routes in P2/P3. Not the branch's.
- `src/session.rs` — **union the fields**: master's `served_model` beside
  the branch's `workspace` and `Channel`.
- `src/cli.rs`, `src/main.rs`, `src/session_recall.rs` — **union the
  commands and match arms**: master's deck and gui flags beside the
  branch's `channels`, `devices`, and `pair`.
- `src/remote/`, `src/channels/`, `relay/relay.py` — **the branch's,
  unchanged**, except where P2's supervisor replaces the launch-time
  spawn.

Acceptance is the full gate — master's 1241 plus the branch's channel,
remote, and QR tests — with the loopback face's own tests still green
unmodified, which is what proves the ruling survived the port.

### P1 — the defects the port must not carry

The branch's own tests passed while these were true; each is a real
defect and each is fixed here rather than ported.

| # | Defect | Fix |
|---|---|---|
| KI-257 | The listener's bind error is swallowed (`let _ = served.serve(…)`), so the terminal can say "live" while nothing listens | P2's supervisor captures it |
| KI-258 | The access read loads webhook peers only; a paired Telegram chat never appears | `access_read` walks every channel's store |
| KI-259 | `Observe` is unreachable — every pairing path grants `Task` | capability is a parameter of every pair form |
| KI-260 | The peer→session map is memory-only, so a restart starts every peer a fresh conversation | the mapping persists and is validated on load |
| KI-261 | `fleet channels --port` is parsed and ignored | the setting is the truth; the flag overrides it or is removed |
| KI-262 | No TUI surface | P5's reader |

**Landed.** KI-258, KI-259, KI-260 and KI-261 are fixed; KI-257 is
P2's supervisor and KI-262 is P5's reader, as the table says.

- KI-258 — `access_read` walks `channels::CHANNELS`, so a paired
  Telegram chat appears beside a webhook peer, and each row names the
  channel it arrived on. Pinned by
  `the_access_read_names_every_channels_peers`.
- KI-259 — `--capability task|observe` is a parameter of both pair
  forms, applied after the whole line is read so it may sit on either
  side of the pairing. `Observe` is now reachable, and a peer paired to
  listen is refused a task by name and still receives outcomes. Pinned
  by `channels_pairs_a_peer_with_the_capability_it_asked_for` and the
  live webhook delivery below.
- KI-260 — the peer→session mapping persists to
  `<FLEET_HOME>/channels/sessions.json` (0600, atomic) and every entry
  is validated on load: the log must exist and carry that peer's own
  channel record. This required P6's record change early — `Channel`
  now carries `peer_id` beside the name, because a name is not unique
  and a scan by name would hand the wrong conversation over. Pinned by
  `a_peers_conversation_survives_a_restart_and_a_stale_one_is_dropped`
  (restart, deleted log, and another peer's log).
- KI-261 — `fleet channels --port` is removed rather than kept as a
  flag that lies; the port belongs to the process that binds it, and
  P2's `channels.webhook_port` setting is the truth the service reads.
  The bare `fleet channels` also walks every channel now, not the
  webhook alone.

Verified live: an `observe` peer's delivery is refused ("that peer may
only listen; it cannot hand fleet a task") while an acting peer's task
is accepted into its own session; the access read names the webhook
peers and the Telegram chat; and after a restart the same peer's next
delivery lands in the same session log.

### P2 — the channel service, supervised

`src/channels/service.rs`: a `ChannelService` owns every plugin and
reports per-channel state (`stopped`, `starting`, `live`,
`failed(reason)`), starts and stops at runtime, and captures the bind
error instead of dropping it. It is configured by `channels.enabled` and
`channels.webhook_port` in the one settings writer, default off — so
`--channels` becomes an override of a setting rather than the only door,
and the invariant holds: with no channel configured, no socket binds.

The truth is one file other surfaces read: `<FLEET_HOME>/channels/status.json`,
written atomically on every transition with a heartbeat while live (pid,
started, updated, per-channel state). The GUI's read, `fleet channels`,
and the TUI all read that document, so they cannot disagree; a stale
heartbeat is named stale rather than believed. The service also owns the
activity log — `<FLEET_HOME>/channels/activity.jsonl`, one line per
delivery, refusal, reply, and seat, capped, carrying the peer, the kind,
the task's first line, and the session — because "what have they done"
is the question the surface exists to answer and nothing records it
today.

### P3 — identity: pending, capability, pairing

The refusal path records a refused peer as *pending*
(`<channel>-pending.json`, capped), so a Telegram bot's first unknown
chat can be approved in the app instead of the owner hunting a numeric
id. Capability becomes editable (`may act` / `may only listen`), which
makes `Observe` real. Pairing is in-app: the webhook token is minted
server-side and returned exactly once, shown in a one-time card, never
in `/api/state`, the activity log, the counters, or any subsequent read.
The pairing token is the one credential that may cross a payload, and
the reading rule is amended to name that exception rather than bent
silently. The CLI grows `--capability` on its pair forms so the terminal
is not the poorer client.

### P4 — the Bots surface

`#/bots`, first in the system group, in the console's own language:
`pageHead`, a `statStrip` whose tiles filter, `boardCard`s for the
service, the pending peers, the peers, the pair form with its one-time
reveal, and the activity. Every row's actions are buttons with
`data-act`; selecting a peer opens the object inspector with its
capability, its sessions, its last delivery, and its related sessions;
selecting a channel opens its status and error. The rail's Bots entry
carries a count when pending peers or waiting peer seats exist, and
nothing when they do not. It is not a console: the tower is the one
surface allowed cockpit density, and Bots is five cards and the actions
that change them.

Wired to `GET /api/channels` — one bounded document: status, peers,
pending, and the newest 50 activity lines with the bound named — and to
the writes `/api/channels/{start,stop,pair,approve,capability,revoke}`.
The remote listener's default-deny allowlist does **not** gain the
control routes: a phone answers 403 by name, the same rule settings
already has. A phone may read its own session and answer its own seat,
and nothing else.

### P5 — sessions becomes the inbox, and the TUI reads

Sessions rows gain `via <channel> · <peer>` and the project, a
`via a channel` filter chip, and the reply's destination named before
the owner types — the watcher behavior the branch built, made visible.
The archive stays flat newest-first.

`PanelKind::Bots` in the TUI reads the same status document: status,
peers, pending, activity, and open-conversation. Start and stop live
only in the face that hosts the service, and the TUI says plainly which
process that is. Hosting channels in the TUI would mean porting the
registry onto the TUI's single-session agent, which is a different slice
with little payoff; this is a scope boundary, not sold as parity.

### P6 — continuity, the relay, and the docs

The channel record gains the peer's id beside its name, and the
peer→session mapping persists to `<FLEET_HOME>/channels/sessions.json`,
validated on load by checking the log exists and carries that peer's
record. The relay ports unchanged — its client, its handler, and
`relay/relay.py` all sit on master's structure — and its phone path is
re-verified through the merged bridge. `docs/features/remote-access.md`
names the retired `#/channels` and the new surfaces;
`docs/features/gui.md` gains the surface table's rows and endpoints;
`docs/features/tui.md` gains the reader; the settings table gains the
two keys.

## Acceptance Criteria

- The loopback face is byte-for-byte master's: no token in the URL, the
  address alone opens it, Host and Origin checks refuse a foreign
  request, and master's own gui tests pass unmodified.
- With no channel configured and no paired device, no socket binds
  beyond loopback and every existing behavior is unchanged.
- `fleet gui --remote` binds the machine's address, prints a pairing
  code, and a phone redeems it once; a second redemption fails; the
  paired phone reaches only the allowlisted routes and 403s by name
  everywhere else.
- A webhook delivery from a paired peer runs in that peer's own session,
  its seat is answerable from the channel and from the GUI, and a
  restart resumes the same conversation.
- A refused peer appears as pending, is approvable in the app, and its
  capability is editable; `Observe` refuses a task and still receives
  replies.
- The Bots surface reports the service's real state; a bind failure is
  named there and in the terminal rather than swallowed.
- `#/sessions` names the channel and peer on the row and filters by
  them; the TUI's Bots panel reads the same document the browser reads.
- The relay carries the page, the assets, and the live state to a phone
  on another network.

## Files to be touched

| Area | Files |
|---|---|
| The merge | all seven conflicted files, by the rules above |
| The service | `src/channels/service.rs` (new), `src/channels/mod.rs`, `src/channels/{webhook,telegram}.rs` |
| Settings | `src/settings.rs` (two keys) |
| The bridge | `src/gui/bridge.rs` (registry semantics onto master's) |
| The surfaces | `src/gui/assets/*`, `src/tui/*` |
| Docs | `docs/features/{remote-access,gui,tui}.md` |

No new crates.

## Out of scope

- End-to-end encryption on the relay (the threat model stays as written).
- Streaming over the relay (the poll stands).
- Native phone apps, push notifications, voice.
- A second relay implementation.

## Linked artifacts

- `docs/plans/2026-10-18-remote-access-lanes.md` — the plan this ports.
- `docs/plans/2026-10-20-bot-interface.md` — the sibling plan for the
  surface half; where the two overlap, this plan defers to its D1–D6.
- `docs/plans/2026-10-16-gui-revamp.md` — the console this lands in.
