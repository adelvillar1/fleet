# The delivery seam — an architecture decision, deferred deliberately

Status: open — no ruling yet
Date: 2026-10-15

## What it is

Every other output fleet produces stays on the machine: session logs
under `<FLEET_HOME>/sessions/`, the calibration stream under
`~/.local/share/dev-decisions/logs/`, the memory corpus in the repo,
the boards read from loopback. A **delivery seam** would be the first
path by which fleet sends data *outward* — a finished cron job's result
pushed to a messaging platform, a long run's report arriving somewhere
the owner can read it while AFK.

`src/cron.rs:24` already names it and deliberately does not build it:
"the delivery seam, which this crate does not build."

## Why it is deferred rather than skipped

It is not an implementation detail. It is the first reversal of the
architecture's grain. The ecosystem doc records second-brain as "the
localhost control tower (never exposed beyond the machine)," and the
whole design arc is local-first: the lanes are local services, the
corpus is the repo, the boards are loopback reads. A seam that pushes
out is a seam that has to answer, before any code is written:

1. **What leaves the machine?** A cron job's task text, its result, the
   session id, the route it walked? Each is a different disclosure, and
   the answer is not "all of it" by default.
2. **Under whose key, to whose platform?** The credential rule is
   env-var NAME only, never a value — which means a delivery credential
   is a new class of secret in `<FLEET_HOME>/.env`, resolved at runtime
   like every other.
3. **What is the trust model for what comes back?** A delivery channel
   that can *receive* is a channel an attacker can write to. Fetched
   web text already rides framed as data-only, never as instructions
   (blend1's web leg, the prompt-injection defense). An inbound message
   needs at least that much, and inbound is strictly harder.
4. **Is there a localhost-only middle path?** A seam that writes to a
   file, a notification center, or the second-brain board gives an AFK
   owner most of the value with none of the egress. That option should
   be on the table before any platform is chosen.

## The ruling to make

Owned by the owner, in its own session, at the top — not discovered
halfway through an implementation. The plans that touch this
(`web-tool-for-every-agent`, `run-control-surfaces`) both name it as
out of scope and point here.

## What is NOT in question

The local surfaces that make an AFK owner's life easier without any
egress: `fleet sessions --recall` over the archive, the route-decision
cache's hit rate on `fleet calib`, the recall-hit feedback loop. Those
ship now; they just do not leave the machine.
