---
name: a-green-that-needed-the-developers-shell-is-a-lie
description: "feedback 2026-10-26 — code that works because the developer's process env happened to carry a fact is not working code; the clean-environment run is the test, and every value a spawn needs must be composed from the machine's own record"
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

Every defect the LayerFS seamless plan's live close-out found had the
same shape: it worked in my shell. The Sandbox's `deploy.volume` was a
fact the environment carried, never one the driver composed; the face's
reads composed nothing, so a GUI launched with a clean environment
answered every read with a named absence — honestly, but emptily; the
register script rode the home's worker endpoint for its Init, which
installs the Store and cannot answer through a worker that serves an
installed one; and the history script resolved against the fleet home
instead of the LayerFS home the install writes it into. Nine defects,
all green in the suite, all dead in a clean environment.

**Why:** a development shell accumulates the facts the feature needs —
one export at a time, each reasonable at the time — and the code learns
to rely on them without ever naming them. The suite runs inside that
same shell, so it inherits the same invisible precondition. This is not
the PATH trap of [[fleet-gate-exit-code-trap]] (a command not resolving);
this is a command resolving perfectly against a value nobody composed.
The tell is a feature that has never once run where it will actually
run.

**How to apply:** run the new path once under `env -i` with only the
facts a real launch would have, and treat any named absence as a defect
in the composition, not in the reader. The rule that follows: anything
one code path needs, every code path that touches it must compose from
the machine's own record — if the arm's spawns compose the environment,
the face's reads and the install's provisioning compose it too, from
the same source, through the same function. And when a read answers with
a named absence on a machine you know is provisioned, believe it: that
is the composition telling you it never happened.
