---
name: fleet-remote-port-onto-master
description: "The remote face and channel service are on master — port plan completed (P0-P6, relay defects), and the follow-up audit closed KI-269..274 (phone-width collapse, decorative callback, dead activity rows, fmt/clippy red)"
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

**2026-10-20: the port is complete.** The `remote-access` branch was
rebased onto master in the `fleet-remote` worktree
(`/Users/alejandrodelvillar/Projects/fleet-remote`), conflicts resolved
by the rules in `docs/plans/2026-10-20-remote-access-port.md` (status:
completed), then P0–P6 landed as nine commits on top: `ff84472` (the
port), `0039b3d` (KI-258..261), `e853001` (the supervisor, KI-257),
`c6896ce` (pending/capability/pairing door), `1cb4dd7` (`#/bots`),
`6c178ae` (the archive is the inbox + TUI reader, KI-262), `226eb4f`
(the relay fixes), `ef0dc96` (an observing peer hears why).

**Landed.** Master was fast-forwarded onto `remote-access` (22 commits)
and pushed: `origin/master` and `origin/remote-access` are
`ef0dc96`+`40e483b`, 0/0 against local. The release binary the
`~/.local/bin/fleet` symlink points at was rebuilt, so `fleet` carries
the work.

A test-double framing flake surfaced here and is worth the note: the
fake Telegram server read only to the head terminator and took the rest
of the buffer as the body, so under build load (head and body split
across reads) a send went unrecorded and `sent: []` failed one run in
three, never on its own. It reads to the declared `content-length` now
(`40e483b`). The lesson is the same one the relay defects taught: a test
that passes on an idle machine has not been tested under load.

## What the port changed from the branch

- Assets are master's editorial shell wholesale. The branch's
  `#/channels` panel, inbox CSS, and phone media query are **retired**;
  `#/sessions` is the inbox (project + `via <channel> · <peer>` on the
  row, a `via a channel` chip) and `#/bots` is the control plane.
- The bridge is master's with the registry's per-session semantics
  underneath: `state(session)`, `AnsweredBy`, the per-session seat. The
  stop/steer control now targets the *driving* session, not the anchored
  one.
- `Channel::serve` gained `ready` (oneshot: `Ok(port)` after the thing
  that could fail succeeded) and `shutdown` (watch). That is how KI-257
  closed — a bind failure is a fact, not a dropped `let _ = …await`.
- `Record::Channel` gained `peer_id` and `SessionSummary.channel` is
  `(channel, id, name)`; the validation KI-260 needs cannot work on a
  name alone, because two peers may share one.

## The relay defects re-verification found (all fixed in `226eb4f`)

Three survived the branch's own live pass, each because the body came
through and nobody read the status — see
[[a-verification-with-no-output-is-not-a-pass]] for the general shape:

1. `status_of` parsed word **two** of a `NNN Reason` line, so every
   relayed answer reached the phone as a 500 with a correct body under
   it.
2. `relay/relay.py`'s `do_POST` forwarded only `/join` and `/frame`, so
   a phone could read the face through the relay and could not POST —
   the entire write half was unreachable.
3. `answer_frame` rebuilt the head from the frame's headers, and the
   relay strips `content-length`, so every POST body declared zero bytes
   and arrived as `EOF while parsing a value`.

Also closed: the relay path skipped `allowed()`, so a relayed phone
reached `/api/settings` by falling off the router as a 404 rather than
by the named 403. The boundary now does not depend on the transport.

## Verification habit that paid

Every increment was checked live before its commit: the loopback face
binds `127.0.0.1` alone with no flags; `channels.enabled` alone brings
the service up; a held port reports `failed` with the errno on the
terminal and in `status.json`; 51s after a kill the document reads
stale; a restart resumes the same session log; a pairing token is on the
pair reply and nowhere after a reload; the relay carries page, CSS, JS,
state, and a phone-started run. The GUI surfaces were judged in real
Chromium (Playwright), not a stub.

Gate: 1272 lib tests, three consecutive full runs green. The six
`tests/integration.rs` failures are the same six on clean master — the
sys1 `FakeMcp` path, untouched by this work.

## What the port's own verification missed (2026-10-20 audit)

The "verified live" list above was true only at desktop widths, and the
gate it cites never ran the formatter or linter. A follow-up audit closed
six defects as KI-269..274 — the phone-width collapse, a `--callback`
that printed and stored nothing, a pending chip that emptied the list,
activity rows that went nowhere, a CLI reporting the wrong port and
missing from `fleet help`, and the fmt/clippy red landing. Detail in
[[fleet-channels-bot-surface-gap]]; the general lesson in
[[a-media-query-adds-no-specificity]]. Repaired and gate-green at 1276
lib tests; the repair was left **uncommitted** at session end (the owner
had not been asked to commit).

Related: [[fleet-remote-access-plan-drafted]],
[[fleet-channels-bot-surface-gap]], [[fleet-gui-editorial-desk-redesign]].
