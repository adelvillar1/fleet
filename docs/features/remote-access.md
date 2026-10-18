# Remote access — pairing, channels, and the relay

Fleet reaches a phone and other machines three ways: a paired device over
the machine's own address, a channel plugin carrying work from a
messaging platform, and an outbound relay for a phone on another network.
All three are off by default; the chat path is byte-for-byte what it was
when nothing is configured.

Plan: [`../plans/2026-10-18-remote-access-lanes.md`](../plans/2026-10-18-remote-access-lanes.md)
(remote access, R0–R8, complete).

## The shape

A session has **endpoints**, and an endpoint is a (transport, peer)
pair. The TUI, the `fleet gui` browser tab, a paired phone, and a
Telegram chat are all endpoints of the same session. The agent core is
untouched by any of it — a surface is a bridge, not an agent.

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

The remote face is a second listener with a default-deny allowlist: a
paired device may watch the stream, read state, list and open sessions,
drive a run or a plan, and answer a seat. Everything else answers 403 by
name, so a route the loopback face gains later stays closed here until it
is deliberately added. The access lists are a loopback read and write
only — a phone that could revoke devices is a phone that could lock its
owner out.

## The inbox

The `#/channels` surface lists every session this home holds, grouped by
the project it was born in, each row naming the channel and the peer when
it has one — a reply typed there goes back to that peer. The access card
rides the same surface: who may reach this face, with a revocation beside
each.

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
| webhook | a POST to `127.0.0.1:3081/hook` | `fleet channels pair-webhook <name> --callback <url>` mints a token |
| telegram | a private-chat message to the bot | `fleet channels allow-telegram <chat_id>` after a refusal names it |

A seat is raised once and presented everywhere the session has a
surface: the channel the delivery arrived on and the GUI inbox alike.
First answer wins, and the resolution is broadcast naming who answered,
so the surface that lost renders "answered elsewhere" rather than a
button standing for a decision already made. What a channel cannot do is
retract a notification it has already sent, and the module says so rather
than pretending otherwise.

## The relay

`fleet gui --remote --relay <url>` dials out and holds a long-poll
against a relay; a phone's HTTP requests arrive as frames on that poll
and the answers go back as frames. The relay never parses either — it is
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
