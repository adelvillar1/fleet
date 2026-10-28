# Remote access — pairing, channels, and the relay

Fleet reaches a phone and other machines three ways: a paired device over
the machine's own address, a channel plugin carrying work from a
messaging platform, and an outbound relay for a phone on another network.
All three are off by default; the chat path is byte-for-byte what it was
when nothing is configured.

Plans: [`../plans/2026-10-18-remote-access-lanes.md`](../plans/2026-10-18-remote-access-lanes.md)
(the branches, R0–R8, complete) and
[`../plans/2026-10-20-remote-access-port.md`](../plans/2026-10-20-remote-access-port.md)
(onto the editorial console, P0–P6, complete). Where the two differ, the
port is the truth: this document describes what is on `master`.

## The shape

A session has **endpoints**, and an endpoint is a (transport, peer)
pair. The TUI, the `fleet gui` browser tab, a paired phone, and a
Telegram chat are all endpoints of the same session. The agent core is
untouched by any of it — a surface is a bridge, not an agent.

**A second window beside this one** (plan
[`../plans/2026-10-20-bot-pwa.md`](../plans/2026-10-20-bot-pwa.md),
feature doc [`bot-app.md`](bot-app.md)): the remote paired-device face
keeps serving the console — the work experience, the origin the owner's
phone PWA was installed from — and the **bot face** binds its own port
(`--bot-port`, default 3081) to serve `fleet bot`, the assistant-first
shell. Two ports, two origins, one bridge: a device token pairs once and
works on either face, and each face refuses the other's assets by name.

The bridge is a registry of attached sessions, each with its own
transcript, running flag, and seat. A command may name a session, or name
none — which is the face's own launch session, created on the first task
(KI-022) and named back on the event stream. Events route to the session
the thread is driving, which is exact because the command loop is
serial.

## Pairing a device

`fleet pair` mints a one-time code and prints it with the address a phone
opens; `fleet gui --remote` prints it at launch, with the URL as a QR.
Redeeming the code burns the window and writes a device record whose only
trace of the token is its SHA-256 — a stolen `devices.json` is a roster
of names and digests. `fleet devices` lists them; `--revoke` forgets one.

The face has its own door: the topbar's QR icon mints a window and shows
the URL, the code, and a scannable QR. The URL is `remote.public_url`
when set, else a detected tailnet HTTPS name that is verified to reach
the remote face, else this machine's address with the plain-HTTP caveat.
Over Tailscale that is the intended path — point `tailscale serve` at the
remote listener, then scan; iOS refuses cleartext to a tailnet address,
so the HTTPS name matters. The card mints one live window: opening it
again invalidates the previous code. A paired phone cannot call the mint
route; it is loopback only.

The remote face is a second listener with a default-deny allowlist: a
paired device may watch the stream, read state, list and open sessions,
drive a run or a plan, and answer a seat. Everything else answers 403 by
name, so a route the loopback face gains later stays closed here until it
is deliberately added. The access lists are a loopback read and write
only — a phone that could revoke devices is a phone that could lock its
owner out.

The cookie is resolved before the root is decided: a paired phone that
opens `/` gets the face, and an unpaired one gets the pairing page. The
reverse ordering — pairing page first, cookie second — made the redirect
a successful pairing performs land back on the pairing form, so a paired
device could reach every allowlisted API route but never the shell.

The face is installable on a paired device: the manifest, the worker, and
the icons are allowlisted and carry no data. Installation needs a secure
context, so an HTTPS remote face (a TLS proxy or Tailscale serve) can be
added to the home screen while the same face on plain-HTTP LAN cannot;
the relayed path under `/s/<sid>/` is per-session and is not installable.
The worker caches only the two versioned script assets and never HTML,
an API response, or the event stream.

## The archive is the inbox

There is no second archive. `#/channels` existed on the branches and is
**retired**: the editorial console already had `#/sessions`, and a second
list of the same logs is the duplicate the console removed. The archive's
rows carry the origin instead — the project a session was born in under
its name, and `via <channel> · <peer>` when a delivery started it — with
a `via a channel` filter chip joining the five, and the replay card's
channel record saying the same. A reply from a peer's conversation goes
back to that peer.

**Who can reach this face** lives on `#/bots`, the control plane: the
service's real state, the peers waiting at the door, the pair form with
its one-time token reveal, every paired peer with its editable
capability, and the newest activity. A channel is live only when its
plugin says so — see *The service* below.

## Channels

A channel is a transport plugin carrying work from outside — a sibling
registry to the tool lanes, not an entry in it, because a channel is a
long-lived connection with a peer identity while a lane is a stateless
stdio child. The driver is where the halves meet, and a peer's capability
is decided in exactly one place, the discipline `approvals::resolve`
holds for a run's mode. A paired peer's task runs as an ordinary run in
that peer's own session log: the same agent, the same log format, the
same seat.

| Channel | How it arrives | How it pairs |
|---|---|---|
| webhook | a POST to `127.0.0.1:3081/hook` (or `channels.webhook_port`) | `fleet channels pair-webhook <name> [--callback <url>]` mints a token, shown once |
| telegram | a private-chat message to the bot | `fleet channels allow-telegram <chat_id>`, or the Bots surface once a refusal recorded it |

The callback is where an outcome or a seat goes. A webhook delivery may
name its own in the body — the more specific instruction wins — and the
pairing's stored callback is the default a delivery that names none
falls back to. Its absence is not a hidden failure: the pairing says
"none stored", and a delivery with neither gets no reply because there is
nowhere to send one. Telegram needs no callback; its reply handle is the
chat id.

Capability is a parameter of the pairing — `--capability task|observe`,
or a choice in the Bots form — rather than a later edit only. `observe`
is a peer that is told things and cannot hand Fleet a task; `fleet
channels capability <channel> <peer> task|observe` changes it afterwards,
and the policy follows. A peer that tried and is not paired is recorded
as **pending** (`<channel>-pending.json`, capped at twenty) so it can be
let in from the app instead of the owner hunting a numeric id out of a
log. The webhook's unpaired token has no identity to approve, so that
attempt is an activity line rather than a peer.

A seat is raised once and presented everywhere the session has a
surface: the channel the delivery arrived on and the GUI inbox alike.
First answer wins, and the resolution is broadcast naming who answered,
so the surface that lost renders "answered elsewhere" rather than a
button standing for a decision already made. What a channel cannot do is
retract a notification it has already sent, and the module says so rather
than pretending otherwise.

## The service

`channels.enabled` in the settings brings the channel service up with the
face; `fleet gui --channels` overrides the door on a home that has not
set it. Off — and off is the default — binds nothing and writes nothing.

The service is a supervisor, and readiness is its whole point: a plugin
answers a `ready` signal exactly once, `Ok(port)` after the thing that
could fail has succeeded (the bind for a listener, the first answered
poll for a long-poller) or `Err(reason)` when it could not. A channel is
`live` only after that report, `starting` while it waits, and `failed`
with the reason otherwise — on the terminal and in the document, so a
face cannot say "live" over a socket nothing holds.

Two files are the truth every surface reads:

- `<FLEET_HOME>/channels/status.json` — written atomically on every
  transition, refreshed on a ten-second heartbeat, with `updated_unix`
  beside the readable stamp. Three missed beats read **stale**, and a
  stale document's phases are labelled "last report" rather than
  believed, because a dead process's last words look exactly like a live
  one's report.
- `<FLEET_HOME>/channels/activity.jsonl` — one line per delivery,
  refusal, reply, seat, state change, and unpaired-token attempt (never
  the token), carrying the peer, the kind, the task's first line, and the
  session. Capped at 500 lines.

`fleet channels`, the browser's `#/bots`, and the TUI's Bots panel read
those two files, so they cannot disagree. The peer→session mapping
persists to `channels/sessions.json` and is validated on load — the log
must exist and carry that peer's own channel record — so a restart
resumes the same conversation and a stale mapping is dropped rather than
handing a peer somebody else's log.

## The relay

`fleet gui --remote --relay <url>` dials out and holds a long-poll
against a relay; a phone's HTTP requests — any verb — arrive as frames
on that poll and the answers go back as frames, status line included.
The relayed phone is held to the same allowlist the socket path applies
and refused by name, because the boundary must not depend on the
transport. The relay never parses either — it is
a map of pending request ids and two queues, which is why the reference
implementation in `relay/relay.py` is a hundred lines of anything. The
page is Fleet's own, served through the relay rather than copied into it:
there is no second copy of the face to keep in sync.

The stream is the one route a frame cannot carry — a frame is a whole
reply and a stream is a sequence — so `/events` over the relay answers
with the session's tail and a marker the page reads as "poll instead",
which is what a phone does well and a held-open connection through a
relay does badly.

**The threat model, stated plainly:** the relay sees frame plaintext — a
task's text, a seat's summary, a transcript. The pairing secret gates the
session, so an unpaired relay cannot stitch a phone to a device, but a
compromised relay reads what crosses it. End-to-end encryption is a later
hardening increment, and this module does not pretend to be it.
