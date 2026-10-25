# The extraction leg: E0's harness, E1's client, E2's door

**2026-10-25** · plan `2026-10-19-sys1-extraction-leg.md` (draft →
**active**) · E0 blocked on credentials (KI-312, owner action), E1 and
E2 landed · full gate green (**1674 passed, 0 failed** +
embed_tail's suite), fmt/clippy/release clean.

## What the leg is

The System One tier judges; this gives it the extract stage. One
extraction-class pass over unstructured state answers every requested
field — typed values with spans, the model's own doubt (`p_null`,
`confidence`), and what it would have guessed — in one request, where
the four verbs (extract, compute, estimate, forecast) are field types,
not endpoints. `p_null` is a first-class answer: a named absence, never
a hallucinated value.

## The wire, reconciled before the client

E0's live probe is blocked on credentials that exist nowhere on this
machine (`CLOUDFLARE_ACCOUNT_ID`, `WORKERS_AI_API_TOKEN` — KI-312,
owner action). So the wire was reconciled from the announcement and
the Workers AI REST docs: POST `/accounts/{account}/ai/run/
@cf/cloudflare/clairic`, Bearer auth, `{"state", "fields"}`, answers
in `result.answers`. The harness is committed runnable —
`scripts/extract-probe.sh` calls all four verbs over a representative
invoice, captures the raw JSON under `tmp/`, prints per-verb latency —
so E0-C1/C2/C3 complete in one command when the key lands, and the
plan's E0 section records the blocker and the two env vars that lift
it.

## The client and its door

`src/extract.rs` (E1): the four verbs as field types over a typed
model; credentials through the fleet home's dotenv with the process
vault; `sys1_extracts()` as its own once-per-process gate — orthogonal
to the decision keys, per the plan's rule that a machine with only one
kind of key keeps that kind working; one retry absorbing the stale
pooled-connection failure (KI-243's mechanism); a 401 read as an
ANSWER out of ureq's Status error — the vendor's message named, the
token and account id masked from every rendered reason. Base URL and
model are config; the stub-server tests pin endpoint-agnosticism, the
named absence, the one-retry, the masking, and — landed with the leg —
E3-C3's type-level refusal: estimates and forecasts cannot become gate
inputs, because `gate_legal` refuses them by kind.

`tools.rs` + the agent arm (E2): the `extract` tool takes `{state,
fields[]}` — nothing deck-specific, any text the run holds — declared
in the spec (a field type the spec does not declare is invisible to
the model), executed off the async worker, and the named absence is a
result the model reads, never a failed run.

## Why E3–E6 wait here

Each inheritor calls the live leg. Building four more features against
an unprobed wire compounds the speculative debt the plan's own
E0-first discipline exists to prevent — the deck-suggest lesson in the
plan's own words: the schema the docs show and the schema the wire
speaks are reconciled, not assumed. E0's probe is one command once the
key lands; the inheritors then build against the measured wire.

## The reconstruction catch, from the what-the-model-saw test

The first gate run red ten integration tests: `reconstruct_document`
reads `base_declarations`, the live request reads `declarations`, and
extract had joined only the live path — so the what-the-model-saw
test's byte-equality (the tools array is the request's own) failed the
moment the leg was declared. The leg is now declared in both, in the
same slot after the deck, and the count pins and the lane-tail slice
widened with it. The model-visible ⟺ logged rule has teeth precisely
because that test exists.

## The state

- Plan: **active** — E0 blocked (KI-312), E1/E2 landed, E3–E6 next.
- Ledger: 312 rows — 294 closed, 3 open (KI-249, KI-252, KI-312), 15 noted.
- KI-249's window is also accumulating: the embed_tail reader reads it
  whenever the bounds-setting pass runs.

## The commits

```
03fe85d feat(E0): the harness committed, the blocker named
c1fa009 feat(calib): embed_tail (KI-249's landed half)
ae5675a feat(E1+E2): the leg and the door
e891c7c docs(ledger+plan): KI-312, the plan active
```
