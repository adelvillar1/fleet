---
name: a-live-probe-in-a-dev-provider-is-not-a-provider
description: "feedback 2026-11-01 — the connect→discover→pin flow passed against a stub, went blank on the real binary because the provider entry carried no key_env, and was never proven against a genuinely NEW provider with a genuinely NEW key; the stub answered because it was born already keyed, the live probe reused a dev provider whose key was already there (the source slug `a-live-probe-inside-a-pre-existing-development-provider-is-not-a-provider` exceeded the name grammar's 64 characters; this legal name carries it…"
type: feedback
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

The 2026-11-01 settings build. I wrote the connect flow (endpoint,
key, name → discover → model list → pin), pinned it with six harness
checks against a stub that boots already-keyed, then drove it with
Playwright against the rebuilt binary — and the page came back with
**zero model rows**, two accounts, then nothing. Only the live read
found it: a provider entry written without `key_env` is refused by the
roster load, and the refusal empties EVERY list that reads the roster
(models 0, providers 0, roster 0). The stub could not see it for the
same reason the unit tests could not: the entry it was given was never
incomplete, and the harness asserts the POST's shape, not the reply's
effect.

**Why:** a probe proves the path only for the exact starting state it
ran in. My live probe reused the scratch home's pre-existing dev
provider, whose key was already in the dotenv, so "it worked" meant
"nothing new was exercised". The owner's requirement was the opposite:
*"make sure that keys are imported… they are working so there is no
point for me to re-add them again"* — the whole ask is about a home
that starts WITHOUT the key.

**How to apply:** after building a create-a-thing flow, run the proof
on a **fresh home with a provider the home has never seen and a key it
has never held** (`rm -rf` a scratch home, start the face on it, then
drive the flow) — the stub and the reused account both lie. When the
result is an empty list after a successful write, suspect the reader's
validator before the writer: a roster/document validator that refuses
one entry takes every list that reads it down with it. Pin the
validator's own admission (here: the entry names `key_env`) in the
write's test, and make the write stamp what the reader needs rather
than trusting the form to send it. Related:
[[settings-ux-plan-2026-11-01]],
[[a-green-pin-can-hide-a-broken-first-run]],
[[a-post-helper-parses-the-body-not-a-response]].
