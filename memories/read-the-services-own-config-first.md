---
name: read-the-services-own-config-first
description: "I twice mischaracterized sys1 from a lane comment in lanes.rs; read the vendored service's own config before describing what it is"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 40
superseded-by:
valid-until:
---

I described sys1 two ways and was wrong both times, because I worked from
the lane spec comment in `src/lanes.rs` instead of the service's own config.
First I said it was "a vendored lane, so nothing needs a key" — then, when
corrected, I called the models hosted but framed it as a caveat rather than
the whole point. The owner's final correction: *"there is nothing local on
sys1, they are all system 1 type models on hosted apis, sys1 is an
abstraction layer to substitute multiple providers."*

The truth, from `services/sys1/src/sys1/config.py`: sys1 is an **abstraction
layer that substitutes multiple hosted System One providers** — Fastino
Decide and GLiDE, TypeSafe Jev, Nace Drex, ScaleDown classify-1, Typellm,
Supersonic Julia, Liquid d1 — every one an HTTP API with its own key env
(`FASTINO_API_KEY`, `TYPESAFE_API_KEY`, `DREX_API_KEY`, …). **Nothing about
the models is local.** The only local thing is the Python MCP lane process
that routes across them; even the `local` provider option runs the *hosted*
Decide model, just in a local venv.

**Why:** a lane's *launcher* is local; its *models* need not be. Saying
"vendored lane" says where the process runs, not where the judgment comes
from — and I stated the implication as fact twice, then argued about it.

**How to apply:** before describing any vendored service (`sys1`, `sdm1`,
`sem1`, `gen1`, `blend1`), read that service's own `config.py`/`VENDOR.json`
and quote what it actually connects to. A lane spec in Fleet names how to
launch it; the service names what it is. And when corrected, find the
primary source rather than restating the same framing more carefully —
that repeats the error. Related: [[say-only-what-you-have-read]],
[[do-not-guess-what-a-subsystem-needs]]
