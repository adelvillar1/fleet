---
status: active
created: 2026-10-19
updated: 2026-10-26
slug: sys1-extraction-leg
---

# The sys1 extraction leg

## Context

sys1 is fleet's System One tier, and today it is decision-class only.
`SYS1_DECISION_KEYS` names `FASTINO_API_KEY` and `TYPESAFE_API_KEY`
(`src/route.rs:1392`), `JUDGE_LANE` is `"sys1"`, and `sys1_classify`
answers the routing judgment with its availability cached once per
process (KI-044). The tier judges; it cannot read.

Cloudflare's Clairic (announced October 2026) is the other half of the
System One family: an extraction class. One pass, no output tokens,
sub-second. It extracts typed values from unstructured input with
spans and confidence; computes single derived values; estimates
distributions (a median with an 80% interval); forecasts a step series.
Where a decision model would guess, it returns `p_null` — a named
absence instead of a hallucinated value.

The owner's direction, 2026-10-19: build this first **into sys1** as
infrastructure, then let **fleet inherit** the functionality — and it
is not deck-centric. The deck is one consumer among four.

The composition the class exists for is a cascade: extract, then
decide, then write, each stage feeding the next cleaner input. sys1
already owns the decide stage. This plan gives it the extract stage and
wires the four inheritors.

## Approach

**A leg beside the judge, not a new tier and not a lane process.**

- **The client is a Rust module, not an MCP lane.** The `LANES` table
  spawns MCP server processes; Clairic is a single-verb HTTP endpoint.
  The leg follows the `HttpEmbedder` pattern instead: one `ureq` HTTPS
  call, `serde` on the response, zero new crates. Its health joins the
  same surfaces the embedder's probe reaches (`fleet doctor`, the TUI
  modal, the GUI doctor card).
- **Credentials ride the vault.** `CLOUDFLARE_ACCOUNT_ID` and
  `WORKERS_AI_API_TOKEN` resolve through `home::read_dotenv_with_vault`
  like every roster key since KI-250, and are never printed at any
  verbosity.
- **Extraction availability is its own gate, never a member of
  `SYS1_DECISION_KEYS`.** A machine with only decision keys must keep
  judging; a machine with only the extraction key must keep
  extracting. The two availabilities are orthogonal, so
  `sys1_extracts()` follows `sys1_judges()`'s once-per-process pattern
  (KI-044) with its own keys and its own named absence.
- **Confidence and `p_null` are gate inputs.** A value below the
  confidence floor routes to a named refusal or a card trigger; a
  `p_null` is a first-class answer that says so. Neither ever passes
  silently — that is the repo's law ("facts gate; reports corroborate")
  applied to a value that carries its own doubt.
- **Rust does the arithmetic.** The model extracts and computes single
  values; aggregation and multi-step math stay in Rust, per the
  announcement's own caveat. The deterministic checks do not move.
- **Forecasts are illustrative.** They render as deck uncertainty
  bands and nothing else; no forecast is ever a gate input or a
  calibration number.
- **The client is endpoint-agnostic from day one.** Base URL and model
  id are config, defaulting to the Workers AI hosted endpoint, so the
  open-weights release self-hosts beside the local embed server with no
  code change — the own-home ruling, designed for now, deployed later.
- **Build order: probe, leg, inherit.** E0 measures the wire before a
  typed model is written (the deck-suggest lesson: the schema the docs
  show and the schema the wire speaks are reconciled, not assumed).
  E1 is the sys1 leg. E2–E6 are the inheritors, each independently
  landable.

## Acceptance criteria

### E0 — the wire probe (re-pointed 2026-10-25: the probe exercises the sys1 shape; KI-312 dissolved — the key already resolves)

- **C1** The live endpoint is called from a scratch harness over a
  real unstructured document, and the actual request/response JSON is
  captured under `tmp/` (never committed, no tokens in it).
- **C2** The documented contract (the four verbs, spans, confidence,
  `p_null`, intervals) is reconciled against the wire, and every
  difference is recorded in this plan's Approach before E1 compiles
  against the real shape.
- **C3** Latency per verb is measured on a representative input and
  recorded here — E1's timeouts are set from the measurement, not a
  guess (KI-249's discipline applied to the new leg at birth).

### E1 — the sys1 leg (landed 2026-10-25; RE-POINTED the same day by owner ruling — see the re-point section below; the criteria below are the Clairic text, kept for the record, superseded by the re-point's shapes)

- **C1** `src/extract.rs` exposes the four verbs — extract, compute,
  estimate, forecast — over a typed response model (value, span,
  confidence, `p_null`, interval), with typed errors naming the
  endpoint, the verb, and the reason.
- **C2** Credentials resolve through the vault overlay; a sealed token
  answers; the masking discipline holds at every verbosity.
- **C3** `sys1_extracts()` is its own once-per-process availability
  gate over its own keys; with no key, every verb returns the named
  absence and no caller retries per call.
- **C4** `fleet doctor` probes the leg the way it probes the embedder,
  and the probe renders on the same surfaces.
- **C5** Base URL and model id come from config with the hosted
  default; a test drives the client against a stub server to pin that
  pointing it elsewhere is config, not code.
- **C6** One retry absorbs the stale pooled-connection failure
  (KI-243's mechanism), and the per-verb timeouts come from E0-C3 with
  the measured number named in each constant's comment.

### E2 — fleet inherits: the `extract` tool (landed 2026-10-25; re-pointed with E1 — the tool renders spans, confidence, quotes, and the named absence)

- **C1** A general agent tool takes `{state, fields[]}` and returns
  typed values with spans, confidence, and `p_null` — declared in its
  spec, because a mode the spec does not declare is invisible to the
  model (the deck-suggest lesson).
- **C2** Nothing deck-specific exists in the spec or the run: the tool
  works on any text the run already holds — a fetched page, a session
  log, a file, pasted prose.
- **C3** An unavailable leg names its absence in the tool result
  instead of failing the run.
- **C4** Pinned by a unit test against E1-C5's stub server and one
  live verification.

### E3 — the calibration loop inherits (and pays KI-249)

- **C1** Outcome evidence extraction: session logs yield typed facts
  (durations, produced files, error classes) with confidence, so the
  judge reads typed lines instead of parsed prose — the cascade's
  extract→decide step, in fleet's own loop.
- **C2** The same extractor over a day of real session logs produces
  the per-call duration distribution KI-249 prescribes; the measured
  tail is recorded in the ledger row and the bounds are set from it,
  each constant's comment naming what it measured.
- **C3** Calibration accepts only extract and compute output; estimate
  and forecast are refused at the type level, so an illustrative
  number cannot become a gate input.

### E4 — the deck inherits (one consumer, not the center)

- **C1** Metric and chart components accept typed values with
  provenance — span, confidence, source — rendered as a citation, not
  a bare number.
- **C2** A metric component may carry an estimated value rendered with
  its 80% interval as a visible uncertainty band.
- **C3** A component built from a below-floor-confidence value trips
  E5's verify card rather than rendering silently.

### E5 — the card bus inherits

- **C1** Data-shape triggers land as card sources: a number sequence
  in a bullet block suggests a chart card, dates suggest a timeline
  card, a below-floor extracted value raises a verify-this-number card.
- **C2** They push onto the existing bus with the pre-baked fix
  computed before the card shows — no new card UI, no new place above
  the composer.

### E6 — the web lane inherits

- **C1** An extract-before-read mode answers data-shaped asks from the
  lane's fetched page as typed values instead of raw prose.
- **C2** The raw-page path is byte-for-byte the default; extraction is
  opt-in per call — the core has no modes, only named options.

## Files to be touched

**The leg** — `src/extract.rs` (new: client, typed model,
`sys1_extracts()`), `src/home.rs` (no change expected — the vault
overlay already covers new keys)

**The tool and its arm** — `src/tools.rs` (spec + run), `src/agent.rs`
(the `Work::Extract` arm)

**The inheritors** — `src/calib.rs` (evidence extraction, duration
distribution), `src/deck.rs` (typed values, provenance, uncertainty
bands), `src/gui/assets/app.js` (the data-shape triggers), the web lane
run in `src/agent.rs` (extract mode)

**Surfaces** — `src/doctor.rs` / `src/main.rs` (the leg's probe),
`docs/known-issues.md` (KI-249 closes through E3-C2)

## Out of scope

- **Multi-step arithmetic in the model.** The announcement's own
  caveat; Rust aggregates.
- **Forecasts as gate or calibration input.** Illustrative renderings
  only, enforced by E3-C3's type-level refusal.
- **Touching the routing judgment.** `sys1_classify`, the lane map,
  and `SYS1_DECISION_KEYS` are unchanged; this adds a sibling gate, it
  does not re-key the tier.
- **A Python or MCP lane process.** A Rust HTTP client module, per the
  `HttpEmbedder` precedent.
- **Self-hosting the open weights.** The client is endpoint-agnostic
  from E1-C5; the local deployment is a later plan against the actual
  release.
- **Authentication and multi-user.** Deferred by ruling (KI-247); the
  vault protects credentials, the loopback boundary protects the face.

## Verification

The gate runs whole — `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`.
Per increment:

- E0 is measured, not tested; its numbers land in this file.
- E1 and E2 are pinned by unit tests against the stub server
  (endpoint-agnosticism, the named absence, one-retry, masking), plus
  one live call recorded in the increment's report.
- E3 is verified by replaying real session logs against a scratch
  `FLEET_HOME` (the memories-test lesson: pin the home, replay the
  preserved fixture), and by the duration distribution landing in the
  KI-249 row.
- E4 is verified by deck fixtures carrying typed values end to end —
  provenance rendered, a band drawn, a below-floor value tripping the
  card.
- E5 is verified by the vm/Playwright harness pattern: triggers push
  onto the bus with their fixes pre-baked.
- E6 is verified live against a fetched page, the raw path unchanged.

## E0 status — blocked on credentials, harness committed (2026-10-25; SUPERSEDED the same day by the re-point below)

**Measured 2026-10-26** (`scripts/extract-probe.sh ~/.fleet/.env`): locate
200 in **1.26 s** (fastino/gliner2.5-multi-v1 entities), verify 200 in
**0.57 s** (fastino/glide on /v1/systemone), all four fields answered —
`invoice_id` @0.98, `total_due` @0.94, `due_date` @0.99, `net_terms` @0.94
(verified). The captured shapes (`tmp/extract-probe/`, never committed)
reconcile against the client exactly: entities keyed `entities.{field}`
with `{text, confidence, start, end}`, verify keyed `answers.{name}` with
`type/noul/confidence`, `p_true` reads `noul` first — no delta the stub
tests left open. E0-C1/C2/C3 **met**, recorded in the same turn the
measurement ran.

The wire contract is documented and reconciled from the announcement
and the Workers AI REST docs: POST
`https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/run/@cf/cloudflare/clairic`,
Bearer auth, body `{"state": text, "fields": {name: {type, ...}}}`,
answers carrying `value` / `p_null` / `confidence` / `guess` /
`span`/`spans` (estimates add median/p10/p90/p_below/p_above;
forecasts add per-step arrays). The four verbs are FIELD TYPES on one
pass, not four endpoints — E1's client sends one request carrying
every requested field.

The live probe is blocked on credentials that exist nowhere on this
machine: `CLOUDFLARE_ACCOUNT_ID` and `WORKERS_AI_API_TOKEN` (Workers
AI Read/Edit token) are absent from the environment, `~/.fleet/.env`,
and sys1's env. The harness is committed and runnable in one command
the moment the key lands: `scripts/extract-probe.sh [dotenv-file]` —
it calls all four verbs over a representative invoice, captures the
raw JSON under `tmp/extract-probe/`, and prints per-verb latency
(E0-C1/C2/C3 in one command; the token is never printed or captured).
Until then E1 builds against the documented shape with the
stub-server tests this plan's C5 designs, and every timeout constant
names the announcement (sub-second typical, 0.46 s measured) as its
source pending E0's measurement.

## Re-pointed 2026-10-25 — the leg rides sys1's own models (owner ruling)

The owner challenged the Cloudflare premise ("isn't that what GLiNER
2.5 already does and it's already wired to sys1?"), the deep dive ran
against the vendored tree and `sys1 doctor`, and the ruling followed:
**"if we need to rewrite things to use sys1 appropriately we do that,
and don't invent a new approach because what's wrongly built should not
stay."** What the deep dive established (all evidence in the session's
memory, `sys1-pool-is-all-hosted-no-extraction-wire`):

- The pool's five available providers are all hosted — decide, glide,
  drex, jev, liquid. The "local venv" GLiNER entries are dead code on
  this machine (`venv not found at /private/tmp/gliner-decide`), and
  the router's own default chain is hosted-only by design
  (`glide,drex,jev`, config.py).
- The wired GLiNER is the hosted Decide model — classification-class.
  No sys1 wire extracts spans. But the extraction-class model is
  hosted by the tier's own vendor under the same key: Fastino GLiNER
  2.5 Multi (`/v1/chat/completions` with an entities schema, spans +
  confidence, $0.03/1M input).
- The Clairic leg was therefore a second vendor and a second key for a
  capability the first vendor serves — and its unique classes
  (estimates, forecasts) are the ones this plan's own law refuses as
  gate inputs. KI-312's credential ask was answering the wrong
  question: `FASTINO_API_KEY` already resolves, so the leg is live
  with no new credentials.

The rebuild follows the pattern jobhound measured for exactly this
problem (`requirement_pass.py`): **locate** (one entity call; the
requested fields are the entity names) → **segment** (pure Rust, the
jobhound segmentation) → **verify** (one decision call per carrying
segment over the sys1 chain glide → drex → jev, the router's own
measured order, filtered by the keys present) → **reconcile
asymmetrically** (assert at 0.85, veto at 0.35, contested between —
one adjudication to the chain's next verifier, capped at 3, and a
still-contested span ships marked with its quote). Every answer
carries the sentence it was read from. Estimates and forecasts do not
exist in the rebuilt leg at all: the field-kind enum has no variant
for them, which is E3-C3's refusal in its strongest form — an
illustrative number cannot even be requested. The absence semantics
are structural: no `p_null` field, a field with no surviving span is
absent BY NAME with the reason (no candidate above the entity
threshold, or none that cleared the verify floors).

Superseded in one stroke: the Clairic client, the
`CLOUDFLARE_ACCOUNT_ID`/`WORKERS_AI_API_TOKEN` pair, the four-verb
field types, and E0's blocked state. `scripts/extract-probe.sh`
exercises the new shape in one command; E0's C1–C3 remain "measure
live and record here" — now unblocked, the machine already holds the
key.

The wire contract is documented and reconciled from the announcement
and the Workers AI REST docs: POST
`https://api.cloudflare.com/client/v4/accounts/{ACCOUNT_ID}/ai/run/@cf/cloudflare/clairic`,
Bearer auth, body `{"state": text, "fields": {name: {type, ...}}}`,
answers carrying `value` / `p_null` / `confidence` / `guess` /
`span`/`spans` (estimates add median/p10/p90/p_below/p_above;
forecasts add per-step arrays). The four verbs are FIELD TYPES on one
pass, not four endpoints — E1's client sends one request carrying
every requested field.

The live probe is blocked on credentials that exist nowhere on this
machine: `CLOUDFLARE_ACCOUNT_ID` and `WORKERS_AI_API_TOKEN` (Workers
AI Read/Edit token) are absent from the environment, `~/.fleet/.env`,
and sys1's env. The harness is committed and runnable in one command
the moment the key lands: `scripts/extract-probe.sh [dotenv-file]` —
it calls all four verbs over a representative invoice, captures the
raw JSON under `tmp/extract-probe/`, and prints per-verb latency
(E0-C1/C2/C3 in one command; the token is never printed or captured).
Until then E1 builds against the documented shape with the
stub-server tests this plan's C5 designs, and every timeout constant
names the announcement (sub-second typical, 0.46 s measured) as its
source pending E0's measurement.

## Audited 2026-10-25 — against commit history

Partially overtaken by a sibling route. What landed since this plan:
the **Perception leg** (`5782497`, `35a3335`, 2026-10-20) — sys1
exposes `sys1_perceive_document` using the DREX key from its own
provider env, and fleet hands the lane a path and caches by content
hash. That proves this plan's C2 discipline (credentials through sys1,
masking held) with a real extraction-class capability — but it is not
E1: no `src/extract.rs`, no four verbs (extract, compute, estimate,
forecast), no `p_null` typed model, no `sys1_extracts()` gate. E0's
wire probe never ran (no Clairic trace in any tree). E2's fleet-side
`extract` exists as `crate::documents::extract` — local extraction, a
different lineage from the sys1 leg. Verdict: E1 remains the gap, E0
is stale-until-probed, and the next session must read Perception
first so the four-verb leg builds on the credential-holder pattern
that already landed instead of beside it.

## Linked artifacts

- The Clairic announcement —
  <https://blog.cloudflare.com/clairic-extraction-model/>
- `docs/known-issues.md` — KI-243 (stale keep-alive), KI-249 (timeout
  bounds from the measured tail; E3-C2 pays it), KI-250 (the vault
  overlay), KI-252 (the deck's vector half; E4 is adjacent, not
  dependent), KI-254 (the card bus's next sources; E5 is three of
  them)
- `docs/plans/2026-10-14-blend1-integration.md` — the triad whose
  extract leg this plan supplies
- `src/memory.rs` `HttpEmbedder` / `probed_embedder()` — the pattern
  the leg follows
