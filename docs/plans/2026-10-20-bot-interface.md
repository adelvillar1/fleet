---
status: completed
created: 2026-10-20
updated: 2026-10-20
slug: bot-interface
---

# Plan: The bot interface — the channel service gets a surface in the editorial console

## Outcome (2026-10-20)

Carried into master by the port plan
([`2026-10-20-remote-access-port.md`](2026-10-20-remote-access-port.md),
completed) as B1–B7. This pass verified the landed surface in Chromium
against a scratch home and repaired what did not hold: the sub-900px
shell collapse (KI-269), the decorative `--callback` (KI-270), the
pending chip and the dead activity rows (KI-271, KI-272), the CLI's port
and missing help (KI-273), and the port's unformatted, lint-red landing
(KI-274). Every acceptance criterion below is confirmed on the repaired
tree, with the four-step gate green.

## Context

The channel service was built on 2026-10-18 and is complete and tested on
the `remote-access` branch (`b0cff86`, worktree `../fleet-remote`): the
plugin contract, the peer store and policy, the driver, the watcher, the
webhook and Telegram plugins. Master has none of it — no `src/channels/`,
no `src/remote/`, no `fleet channels`, no `--channels`. On the app the
owner runs, the bot experience has no interface because it is not in the
app.

Where it does exist, the interface is a CLI and a viewer:

- `fleet channels` lists peers, pairs a webhook peer, revokes one, allows
  a Telegram chat id.
- `fleet gui --channels` starts the webhook listener on 3081 and, when
  `TELEGRAM_BOT_TOKEN` is in the environment, the long-poll.
- `#/channels` is an inbox of sessions grouped by workspace plus an
  access card. Pairing is a printed CLI hint (`app.js:2800`), and the
  service has no start, stop, or status anywhere.

The remote-access plan defined R6 as the in-app half — "reply from the
desktop to a peer, pairing and device management, and the inbox's
unread/attention states" — and closed it on the third that fell out of
the architecture: typing into a peer's attached session does reply,
because the driver's watcher is subscribed to that session's stream.
Pairing and management in the app, and the attention states, were never
built. This plan is R6, done deliberately, in the GUI that exists now.

**The GUI this must align with.** The editorial console landed on
2026-10-19 (`ddb8724`, `cd8ad1a`) and the TUI was revamped to match
(`e8ebdec`, `968471c`). Its language is fixed and this plan adds no
dialect: `pageHead` (eyebrow, display-serif title, one plain sentence,
the read's stamp), `statStrip` and tiles that apply a filter, `boardCard`
with its own `board-stamp`, `filter-bar` chips with counts and
`aria-pressed`, badges with ok/warn/down tones, one object inspector with
two homes (the dock's object card above 1180px, the fixed drawer below),
the action-card bus above the composer, browser counters in
`<FLEET_HOME>/ui.json`, the reading rules (a refused read keeps the last
good numbers and names its reason; a bounded read names its bound; no
credential value in a payload), and the `data-act` audit — no visible
row, tile, badge, segment, or related row is a dead end
(`docs/features/gui.md`).

**The distance.** Master is 25 commits ahead and the branch 14; seven
files conflict (`src/cli.rs`, `src/gui/assets/{app.css,app.js}`,
`src/gui/bridge.rs`, `src/gui/mod.rs`, `src/gui/server.rs`,
`src/session_recall.rs`). `src/channels/` itself is clean. The expensive
conflict is `src/gui/bridge.rs`: the branch's bridge is a session
registry (`run(session, task)`, `answer(session, decision, by)`,
`AnsweredBy`), while master's is single-session (`run(task)`,
`answer(decision)`) and has since grown stop/steer, the action cards, the
home and related reads, and the usage counters. The registry is not
incidental churn: the driver needs to run in a peer's own session and the
seat needs to name which surface answered, so the registry is the
load-bearing port.

**The defects this plan closes** (proposed rows; filed in the same commit
as each fix; the line numbers are the branch's, since most of these files
do not exist on master):

| # | Defect | Where |
|---|---|---|
| KI-257 | A listener that cannot bind is swallowed — `let _ = served.serve(…)` — so the terminal and the panel can say "live" while nothing is listening | `src/gui/mod.rs:189`, `:212` |
| KI-258 | The access read loads webhook peers only; a paired Telegram chat never appears in "who can reach this face" | `src/gui/bridge.rs:1942` |
| KI-259 | `Observe` is unreachable: every pairing path grants `Task`, so the policy's listen-only half cannot be used | `src/main.rs:464`, `:508` |
| KI-260 | The peer→session map is memory-only while its doc claims an archive scan; the channel record stores the peer's *name*, so a scan could not disambiguate anyway. A restart starts every peer a fresh conversation | `src/channels/mod.rs:317-322`, `:350`, `:370` |
| KI-261 | `fleet channels --port` is parsed and ignored; `gui --channels` always binds `DEFAULT_PORT` | `src/cli.rs:1793`, `src/main.rs:436` |
| KI-262 | There is no TUI surface, and the CLI's bare view cannot see a running face's state | `src/tui/` |

**What the interface must answer.** Five questions, in the owner's
words: is it running; who can reach it; how do I let someone in or throw
them out; what have they done; and where is their conversation.

## Approach

Six decisions, then the increments.

**D1 — one archive, two surfaces.** The branch's `#/channels` inbox is
retired, not ported. Master already has `#/sessions`; a second archive
is the duplicate the editorial console removed. Sessions becomes the
inbox (channel and peer named on the row, a `via a channel` filter,
project on the row), and a new `#/bots` surface becomes the control
plane. The word "channel" stays in the code and the CLI; the surface is
"Bots", because that is the owner-facing concept.

**D2 — the service becomes a supervised runtime.** A `ChannelService`
owns every plugin and reports per-channel state (`stopped`, `starting`,
`live`, `failed(reason)`), start/stop at runtime, and the bind error
captured rather than dropped. It is configured by
`channels.enabled` / `channels.webhook_port` in the one settings writer
(`src/settings.rs`), default off, so `--channels` stops being the only
door and the invariant holds: with no channel configured, no socket
binds and the chat path is byte-for-byte unchanged.

**D3 — the truth is one file other surfaces can read.** The service
writes `<FLEET_HOME>/channels/status.json` atomically on every
transition and a heartbeat while live (pid, started, updated, per-channel
state). The GUI's read, `fleet channels`, and the TUI all read that one
document, so they cannot disagree. A stale heartbeat is named stale
rather than believed.

**D4 — identity is editable and pending peers exist.** The policy is the
only place the decision lives; the refusal path records the refused peer
as *pending* (`<channel>-pending.json`, capped), so a Telegram bot's
first unknown chat can be approved in the app instead of the owner
hunting a numeric id. Capability is editable (`may act` / `may only
listen`), which makes `Observe` real. Pairing is in-app: the webhook
token is minted server-side and returned once; the Telegram approval is a
button on the pending row.

**D5 — continuity is written down by id.** The channel record gains the
peer's id beside its name, and the peer→session mapping persists to
`<FLEET_HOME>/channels/sessions.json`, validated on load by checking the
log exists and carries that peer's record. A restart resumes the same
conversation.

**D6 — the TUI reads, the GUI hosts.** The driver needs a session
registry; the GUI's bridge is one, the TUI's agent thread is a single
anchored session. Hosting channels in the TUI would mean porting R2's
registry onto the TUI's agent, which is a different slice with little
payoff. The TUI gets a Bots panel that reads D3's document and opens a
peer's conversation, and says plainly which process hosts the service.
This is named as a scope boundary, not sold as parity.

**The one credential exception.** The pairing token is the only secret
that may cross a payload. It is returned by the mint route exactly once,
shown in a one-time card, never in `/api/state`, the activity log,
counters, or any subsequent read, and the store keeps only its digest —
the same rule as a device token, whose pairing code is likewise shown
once. The reading rule is amended to name this exception rather than
bent silently.

### The surface

```
system · bots                                          read 12s ago

Bots
The ways work reaches fleet from outside, and who may send it.

 2 channels   1 may act   3 via a channel   0 refused today

the service                                            [stop] [start]
  webhook    live · 127.0.0.1:3081 · 2 peers · last delivery 4m ago
  telegram   stopped · no bot token in the environment

waiting to be let in (1)                          [let in] [only listen] [ignore]
  telegram · Alejandro (4242) · asked 6m ago · 3 deliveries refused

peers                                      [all] [may act] [only listen]
  the CI box   webhook   may act   paired 10-18   seen 4m ago   [open] [revoke]
  Alejandro    telegram  may act   paired 10-20   seen 6m ago   [open] [revoke]

pair a peer
  channel [webhook ▾]   name [        ]   callback [        ]   [mint a token]

activity                                  [all] [ran] [refused] [seat]
  10:42  the CI box  ran       check the build → session 3f2a
  10:39  Alejandro   refused   not paired · now pending above
```

Each card is a `boardCard`; the strip is a `statStrip` whose tiles apply
the matching filter; the peers and activity bars are `filter-bar` chips
with counts; the state of a channel is a badge (`live` ok, `failed`
down, `stopped` plain); every row's actions are buttons with `data-act`,
and "open" resolves the peer's conversation and selects that session in
the archive. Selecting a peer opens the object inspector (dock above
1180px, drawer below) with its capability, its sessions, its last
delivery, and its related sessions; selecting a channel opens its status
and error. The rail's Bots entry carries a count when pending peers or
waiting peer seats exist, and nothing when they do not. Below 900px the
peers table becomes stacked rows; nothing overflows.

It is not a console. The tower is the one surface allowed cockpit
density; Bots is five cards and the actions that change them, no chart
and no KPI wall — the counts exist to filter the lists beneath them.

### The increments

**B1 — land the transport.** Merge `remote-access` and resolve the seven
conflicts by rule, not by picking a side wholesale:

- `src/gui/assets/*`: master's editorial shell wholesale. The branch's
  `#/channels` panel and its CSS are not taken; B5 adds only the classes
  the new surface needs.
- `src/gui/bridge.rs`: port the branch's registry semantics onto master's
  bridge — session-scoped `run`/`answer`/seat, `AnsweredBy`, the access
  read — and keep master's stop/steer, action cards, home, related, and
  usage. This is the expensive file and the reason the increment is first.
- `src/gui/mod.rs`, `src/gui/server.rs`: master's structure, then the
  channel block and routes in B2/B3 (not the branch's).
- `src/session.rs`: union the fields — master's `served_model` beside the
  branch's `workspace` and `Channel`.
- `src/cli.rs`, `src/main.rs`, `src/session_recall.rs`: union the
  commands and match arms; keep master's deck/gui flags and the branch's
  `channels`/`devices`/`pair`.

Acceptance is the existing gate plus the branch's channel tests and an
end-to-end webhook delivery through the merged bridge.

**B2 — the service and its status.** `src/channels/service.rs`: the
supervisor, the status enum, the status document (D3), start/stop,
captured bind errors, and heartbeat. The service also owns the activity
log — `<FLEET_HOME>/channels/activity.jsonl`, one line per delivery,
refusal, reply, and seat, capped at 500 lines with the oldest dropped,
carrying the peer, the kind, the first line of the task, and the session
— because "what have they done" is the question the surface exists to
answer and nothing records it today. `channels.enabled` and
`channels.webhook_port` join the settings table; `--channels` becomes an
override of the setting; `fleet channels --port` is honored or removed
(it currently lies). The server takes `Arc<ChannelService>`; the remote
listener's default-deny allowlist does not gain the control routes, so a
phone answers 403 by name — the same rule as settings.

**B3 — identity: pending, capability, pairing.** The driver's refusal
path records pending peers (both plugins, one place); `PeerStore` gains
`set_capability`; `access_read` reads every channel's store, not webhook
only; pairing and approval routes land (D4). The CLI grows
`--capability` on its pair forms so the terminal is not the poorer client.

**B4 — continuity and the archive's fields.** The channel record carries
the peer id; the mapping persists and is validated on load (D5);
`SessionSummary` gains `workspace`, `channel`, and `peer_id`, and
`summarize` reads them tolerantly (an old log has none). Then the
sessions panel can name the peer and offer the filter.

**B5 — the Bots surface.** The new `#/bots` panel, first in the system
group (access, before the configuration surfaces), in the language above:
page head, stat strip, the service card, the pending card, the peers card
with filters and actions, the pair card with the one-time token reveal,
and the activity card. It is wired to `GET /api/channels` (one bounded
document: status, peers, pending, activity's newest 50 with its bound
named) and to the writes
(`/api/channels/start|stop|pair|approve|capability|revoke`). Every new selector
joins the `data-act` audit; the counters record pair, revoke, start,
stop, and open.

**B6 — sessions becomes the inbox.** The sessions rows gain `via
<channel> · <peer>` and the project, the `via a channel` filter chip, and
the reply's destination named before the owner types (the existing
watcher behavior made visible). The archive stays a flat newest-first
list — the branch's workspace grouping is not carried over, because
grouping is a layout change the owner approved a different shape for;
the project rides the row and its filter instead.

**B7 — the TUI reader, the docs, and the record.** `PanelKind::Bots`
reading `boards::read_bots` (the same document the browser reads), with
status, peers, pending, and activity, and open-conversation; start/stop
only in the face, said plainly. Update `docs/features/gui.md` (surface
table, endpoints), `docs/features/remote-access.md` (the inbox section
names the retired `#/channels` and the new surfaces), `docs/features/tui.md`,
and file KI-257…262. Close this plan only when every criterion below is
confirmed.

## Acceptance Criteria

1. `cargo fmt --check`, `clippy --all-targets -- -D warnings`,
   `build --release`, and `test` all pass; the branch's channel, webhook,
   and Telegram tests survive the merge.
2. On a fresh home with `channels.enabled` absent or false, `fleet gui`
   binds 3080 and a connection to 3081 is refused; `--channels` starts
   it; setting `channels.enabled: true` starts it without the flag.
3. With 3081 already occupied, starting the service reports `failed` with
   the operating system's error in the terminal and in
   `GET /api/channels`; no surface says live.
4. `POST /api/channels/stop` closes the socket (connection refused), and
   `start` reopens it; both routes answer 403 on the remote face by the
   existing allowlist.
5. Pairing from the panel returns the token exactly once; a second read
   of the API never contains it; the on-disk store contains only its
   digest; the token appears in no activity line and no counter.
6. A refused Telegram chat appears in pending with id, name, and first
   seen; approving `may act` makes its next message run; approving
   `may only listen` leaves it paired, refused by name, and not running.
7. A peer's conversation survives a restart: deliver, stop the face,
   start it, deliver again — the same session id, and the mapping on disk
   names that peer's id.
8. A session that arrived on a channel renders `via webhook · the CI box`
   in the archive, the `via a channel` filter counts it, and a reply
   typed there reaches the peer (the watcher's own end-to-end test).
9. `#/bots` renders at 1440, 1180, and 900px with no overflow and no
   console errors; every visible row, tile, badge, and activity line
   carries a `data-act`, button role, or title (the Playwright audit
   extended to the new selectors).
10. The surface's numbers agree with the files: the peer count equals the
    stores' rows, the activity card equals the newest 50 lines and says
    so, and a refused read keeps the last good numbers with its reason.
11. The rail's Bots entry carries an attention count when pending peers
    or waiting peer seats exist, and none when they do not.
12. A peer's waiting seat is named on `#/bots` with its session and is
    answerable there or in the attached conversation; first answer wins,
    and the losing surface renders answered elsewhere.
13. The TUI's Bots panel shows the same status, peers, and activity from
    the same document, and says which process hosts the service.
14. The docs name the surface, its routes, and the retired panel; the KI
    ledger carries KI-257…262; this plan is `completed` only when 1–13
    are confirmed.

## Files to be touched

| Increment | Files |
|---|---|
| B1 | `src/channels/{mod,webhook,telegram}.rs`, `src/remote/{mod,pairing,listener,relay,qr}.rs`, `src/lib.rs`, `src/main.rs`, `src/cli.rs`, `src/session.rs`, `src/gui/bridge.rs`, `src/gui/server.rs`, `src/gui/mod.rs`, `src/gui/assets/*` (master's, kept), `src/tui/app.rs`, `src/session_recall.rs`, `docs/features/remote-access.md`, `relay/relay.py` |
| B2 | `src/channels/service.rs` (new), `src/settings.rs`, `src/gui/mod.rs`, `src/gui/server.rs`, `src/cli.rs`, `src/main.rs` |
| B3 | `src/channels/mod.rs`, `src/gui/server.rs`, `src/gui/bridge.rs`, `src/main.rs` |
| B4 | `src/session.rs`, `src/channels/mod.rs`, `src/gui/bridge.rs` |
| B5 | `src/gui/assets/{index.html,app.js,app.css}`, `src/gui/server.rs`, `src/gui/bridge.rs` |
| B6 | `src/gui/assets/app.js`, `src/gui/bridge.rs`, `src/session.rs` |
| B7 | `src/tui/app.rs`, `src/tui/panels.rs`, `src/boards.rs`, `docs/features/{gui,remote-access,tui}.md`, `docs/known-issues.md` |

## Out of scope

- Discord or a third plugin; the contract is proven and a new plugin is
  its own increment.
- Hosting the channel service in the TUI (D6's reason).
- A separate inbox surface, or workspace grouping in the archive (§B6).
- End-to-end encryption for the relay, and multi-user authentication.
- Per-peer model selection, attachments, and webhook request signing.
- A delivery acknowledgement protocol beyond the reply that already
  exists.

## Verification

- **Gate:** the four commands in AC1, read in full — never through a
  filter that can lose the exit status (KI-014).
- **End to end:** a real webhook delivery from `curl` through the real
  driver to a peer's session and back to a callback; a Telegram update
  fed to the real plugin against a fake Bot API, as R8 did.
- **Runtime:** a home with channels off (socket refused), on (socket
  open), and a port deliberately occupied (failed, named).
- **Browser:** the Playwright pass at the shipped widths for rendering,
  overflow, console errors, the `data-act` audit, and screenshots of
  `#/bots` in all four themes; the browser is the truth about what
  renders, the stub is not.
- **TUI:** the existing harness opens the Bots panel against a scratch
  home holding a status document, a peer store, and an activity log.
- **Continuity:** the restart test in AC7, against a scratch home.

## Linked artifacts

- The branch that holds the transport: `remote-access` (`b0cff86`,
  worktree `../fleet-remote`), with the R6 promise at
  `docs/plans/2026-10-18-remote-access-lanes.md:176-183` and its closure
  at `:244-249` — that file lives on the branch, not on master.
- The design it must match: `docs/features/gui.md`; commits `ddb8724`,
  `cd8ad1a`, `e8ebdec`, `968471c`.
- The channel contract it surfaces: `src/channels/mod.rs` (on the
  branch), `docs/features/remote-access.md:42-70`.
