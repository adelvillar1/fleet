---
name: a-reducer-dropped-case-goes-silent
description: "feedback 2026-10-23 — the face's event reducer had no `case 'session'`, so face.session never learned the run's session and every check keyed on it drifted; a switch with a default that silently ignores unknown events hides a missing case, and the symptom surfaces far from the cause"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Fleet's browser face reduces one event stream with a big `switch (event.type)` and a
`default: break`. When a run creates its session log, the thread sends a `session`
event — and the reducer had **no case for it**. Nothing failed: `face.session` simply
kept whatever the last `/api/state` load had anchored (empty on a fresh face, the
previous session after a resume), and every check written against it compared against
a stale value.

That surfaced as an approval card that stayed open after the answer, about one run in
two (KI-283's regression, then KI-295). The reducer's silence is what made it hard:
the event arrived, was well-formed, and was dropped by the default branch, so the
symptom appeared in a handler written for a different event, three layers away.

**Why:** an event switch with a permissive `default` is a place where a producer can
add an event and every consumer keeps working — with a stale value. The compiler
cannot help across the wire, and neither can a live probe unless it inspects the
reducer's own state rather than the DOM (my first probes read the element's `hidden`
and concluded the wrong thing because the state was fine and the *condition* on it
was wrong).

**How to apply:** when adding an event on the bridge side, grep the reducer for its
`case` in the same change — a producer without a consumer is a silent no-op. When a
handler's condition misbehaves, print the state it reads (not the DOM it writes)
before theorizing. Prefer keying a check on the object's own carried field
(`seat.session`) over an ambient one (`face.session`), so a missing case degrades
that check alone. Related: [[an-effect-with-no-visible-trace-reads-as-broken]],
[[a-predicate-naming-shapes-that-never-occur]] (both are checks that believe a
vocabulary the producer does not speak).
