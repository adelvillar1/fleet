# The remote face lands, and a phone reaches fleet through a switchboard

Status: complete
Date: 2026-10-18

Branch: `remote-access`, one session, one plan: the remote-access plan,
R0 through R8 plus the QR. Thirteen commits, and the session's whole arc
is one question the owner asked at the start — what would it take to
reach fleet's sessions from a phone — answered three different ways
before the code started.

Three sessions of design came first. The owner asked the question; the
first answer named three architectures (direct LAN, zero-trust mesh,
hosted relay); the owner then pointed at ZCode's own remote control,
which turned out to be an outbound WebSocket relay plus a QR that opens
a Safari page listing sessions grouped by project; and then at
OpenMausBot, which turned out to supply the layer ZCode's relay assumes
— pairing, peer identity, and a trust model. The plan that landed is the
union of the two references, and the whole of it is in
`docs/plans/2026-10-18-remote-access-lanes.md`.

## The shape

A session has endpoints, and an endpoint is a (transport, peer) pair. The
TUI, the `fleet gui` browser tab, a paired phone, and a Telegram chat are
all endpoints of the same session. The agent core is untouched by any of
it — a surface is a bridge, not an agent, which is the seam the GUI
revamp already proved.

The bridge became a registry of attached sessions, each with its own
transcript, running flag, and seat. A command may name a session or name
none, and none means the face's own launch session — which the rebase
onto master then taught to create lazily, under KI-022, so the registry
and the lazy log had to be merged rather than merely combined.

## The increments

- **R0** — the session log's `Start` record gained `workspace`, the
  working directory the session was born in. Everything grouped by
  project depends on it, and a log written before the field reads as
  "no project recorded" rather than as corruption.
- **R1** — `fleet pair` mints a one-time code and `fleet gui --remote`
  prints it as a QR; the remote face is a second listener with a
  default-deny allowlist, gated by device tokens stored as their
  SHA-256. A paired phone may watch, read, run, and answer a seat, and
  nothing else — every other route answers 403 by name.
- **R1b** — the QR encoder, hand-rolled: byte mode, level M, versions 1
  through 10, with mode segmentation so a 43-byte pairing URL fits
  version 3 instead of pushing to 4. Reed-Solomon, the BCH codewords,
  the zigzag, and the eight masks, all of it, because the crate
  contract's dependency set is the one it is.
- **R2** — the registry. Events route to the session the thread is
  driving, which is exact because the command loop is serial.
- **R3** — the phone layout and the inbox: sessions grouped by project,
  a row attaching the conversation surface, the dynamic viewport unit
  and the keyboard-inset variables a phone's browser actually needs.
- **R4** — the channel service: a trait, a registry, a peer policy
  resolved in one place, and the webhook reference plugin. A paired
  peer's task runs as an ordinary run in that peer's own session log.
- **R5** — seat arbitration: first answer wins, and the resolution
  names who answered, so the surface that lost renders "answered
  elsewhere" rather than a button standing for a decision already made.
- **R6** — the inbox names the channel and the peer, and the access
  card lists who may reach this face with a revocation beside each.
- **R7** — the relay: fleet dials out, holds one poll, and a phone
  reaches the page, the stylesheet, and the live session state through
  it. The page is fleet's own, served through the relay rather than
  copied into it.
- **R8** — the telegram channel, which is the proof the contract
  generalizes: a platform that dials instead of listens, numbers its
  peers, and draws no buttons runs through the same driver, the same
  policy, and the same seat.

## The bugs the verification caught

Each of these would have shipped as a thing that looked fine and was
not:

- The QR's alignment table was indexed by version instead of
  version-minus-one, so version 3 silently used version 4's centres. The
  module matrix still looked like a QR; a scanner would have found
  nothing.
- The finder separators were placed on two sides, leaving the top-right
  finder's left edge and the bottom-left finder's top edge open for the
  data pass to fill.
- The terminal rendering used reverse video, which inverts the code on a
  dark terminal and leaves it correct on a light one — a QR is the one
  thing a terminal prints that a camera has to read.
- The channel driver remembered a peer's reply handle *after* building
  the session mapping, so every peer's conversation carried no way back
  and a seat raised three turns later would have had nowhere to go.
- The webhook plugin reached past its sink straight into the driver,
  which would have made a plugin untestable without a live one.
- Three relay bugs: the poll reply was a wrapper object the client
  parsed as a bare array; the client's timeout was shorter than the
  relay's hold, so every poll was abandoned by the client before the
  relay answered it; and the shutdown sender was scoped to the block
  that spawned the loop, so it dropped at once and `changed()` answered
  immediately — a poll task leaked per iteration until the blocking pool
  saturated at 512 threads. That last one has a regression test that
  asserts one poll in flight at a time, and the test was rewritten once
  because its first form passed vacuously.

The QR's verification is the session's best example of the difference
between a test that passes and a test that guards: the module matrices
are compared against a real implementation for three payloads, and the
terminal rendering is round-tripped back into modules, which is what
caught the reverse video — a rendering that reads correctly to a human
and backwards to a camera.

## The gate

`cargo build` and `cargo test` at every increment, exit codes read
directly and never through a filter. 1257 tests in the lib and the
binaries, zero failures, zero warnings. Six integration tests fail in
this environment and fail identically on master — the sys1 and blend1
lanes are not running on this machine — and the branch adds no failures
of its own. The shared target directory with the main tree was its own
lesson: two builds' `fleet` binaries at one path, and the other
session's gate overwriting this one's, which is why the worktree now
carries its own.

## Commits

| Hash | Message |
|------|---------|
| `49f3a16` | docs(remote): the plan closes — nine increments, each answered by what landed |
| `ca5e4d6` | feat(channels): the telegram channel — the second plugin, and the proof the contract generalizes |
| `b16f5c8` | feat(remote): the outbound relay — Fleet dials out, a phone reaches it through the switchboard |
| `a82113a` | fix(remote): the session registry meets KI-022 — the launch log is created on the first task |
| `4b54576` | feat(gui): the inbox names the channel and the peer, and who can reach the face |
| `db1a823` | feat(gui): seat arbitration — first answer wins, and the resolution names who |
| `dfb73ff` | feat(channels): the channel service — a transport plugin carrying work from outside |
| `0824b34` | feat(gui): the inbox — sessions grouped by project, and a face a phone can use |
| `f676d0d` | feat(gui): the bridge becomes a session registry — a phone and a desktop in two conversations |
| `ea35e1a` | feat(remote): the pairing QR — a hand-rolled encoder, verified module for module |
| `59fbc8d` | feat(remote): the paired-device face — pairing, the listener, and default-deny |
| `952f56f` | feat(session): the log records the workspace it was born in |

## Notes

- The other session worked the known-issues list in the same tree all
  day, and the collision was managed the way the repo has managed one
  before: a worktree of its own on a branch of its own, rebased onto
  master before each increment. The one shared thing was the target
  directory, and the one real cost was the binary it clobbered.
- The relay sees frame plaintext, and the plan says so in both the plan
  and the feature doc rather than leaving it to be discovered. A
  compromised relay reads what crosses it; the pairing secret gates the
  session; end-to-end encryption is a named later increment, not a
  silently assumed one.
- Streaming over a relay is a poll. `/events` answers with the session's
  tail and the page reads the marker and polls, which is what a phone
  does well and a held-open connection through a relay does badly. The
  plan names that as deferred rather than shipping a stream that only
  works sometimes.
- `tmp/` holds the session's scratch — the gate script, the reference QR
  fixtures, the vm harness — and is untracked but not gitignored, which
  is the same posture the ledger's recap left it in.
