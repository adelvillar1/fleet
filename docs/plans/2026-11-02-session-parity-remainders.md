---
status: completed
created: 2026-11-02
updated: 2026-11-02
slug: session-parity-remainders
---

# Plan: the session-parity remainders — the Responses protocol, the trace id, the face's headers row

## Context

The provider-session-headers and session-header-parity plans landed
Fleet's session-id machinery: a per-provider `headers` field, an opaque
resume-stable `{session}` id, injection-safe validation, the request-id
capture. Three items were named out of scope and stood open. This plan
closes them — or, where closing is the wrong move, closes the question:

- **The Responses protocol** — the Luna-class models speak
  OpenAI's `/responses` wire, not `/chat/completions` (zai #840's
  related gap: with the session header present, `/chat/completions`
  answers `ModelProtocolUnsupported` while `/responses` serves). Fleet
  speaks chat-completions only and hardcodes the path
  (`src/api.rs:610,656`), so those models are unreachable no matter
  what headers ride.
- **`{trace}`** — the observability gateways that group by session
  (Helicone) also trace by request chain (Portkey's
  `x-portkey-trace-id`, LiteLLM's `x-litellm-trace-id`); Fleet's
  headers carry no per-run id distinct from the conversation's session
  id.
- **The face's headers row** — the roster field is config-and-CLI only;
  the GUI's account card is where the owner will look for it, and that
  card is settings-ux U9's rebuild.

The fourth named item — **auto-generation for gateways that
fingerprint** (Vercel's session-affinity fallback) — is a non-item by
examination: the fallback fires when no session header is sent, and
Fleet's config alone decides what is sent; there is nothing for the
client to generate. It is recorded here as closed so it is never
silently reopened.

## Research

**The Responses wire, documented.** `POST {base}/responses` carries
`model`, `input` (a list of typed items — role/text items, and
`function_call_output` items referencing a prior call's `call_id`),
`instructions` (the system side), `tools` (flat
`{type: "function", name, description, parameters}` entries), and
`stream`. Streaming is typed SSE — `response.output_text.delta`,
`response.function_call_arguments.delta`, `response.output_item.added`,
`response.completed` (which carries the full output and the usage) —
not chat-completions' flat delta chunks. Tool calls arrive as
`function_call` output items with `call_id`; the result returns as a
`function_call_output` input item. The streaming reference and the
function-calling guide document the shapes; a LiteLLM issue (#20711)
records a real-world streamed-fragment bug in the argument deltas —
the class of wire surprise the probe below exists to catch.

**The statefulness is the trap.** The Responses API's headline feature
is server-side conversation state (`previous_response_id`, the
Conversations API, 30-day retention). Fleet cannot use it: hard rule 5
holds that anything the model sees must be reconstructable from the
session log, and a server-held conversation would make the prompt
reconstructable only by trusting the provider's retention. The adapter
therefore uses the protocol as a **wire dialect only**: `store: false`,
no `previous_response_id`, the full history as `input` every turn —
the same stateless discipline the chat-completions path has always
had. (The fork's #5495 reached the same shape for DSH.)

**The mixed gateway is the real case.** OpenCode's Go gateway serves
kimi/deepseek/glm models over `/chat/completions` and the Luna models
over `/responses` — one roster entry, two protocols, chosen per model.
An entry-level protocol field alone would not carry it; the field needs
a per-model override beside it, the way `model_settings` and
`disabled_models` already ride per model.

## Approach

**Three workstreams and a probe, hinge-first.** The wire probe comes
first because the sys1-extraction-leg's E0 lesson applies verbatim: the
schema the docs show and the schema the wire speaks are reconciled, not
assumed. Then the protocol fields (they reshape the files the adapter
touches), then the adapter, then the trace placeholder, then the face's
row — which lands with settings-ux U9's card or after it, never before.

High-level steps:

1. **W0 — the wire probe.** A scratch harness (`scripts/`) that calls
   a real `/responses` endpoint over a representative tool-call turn —
   one request, one streamed answer — and captures both documents under
   `tmp/` (never committed). It is committed and runnable in one
   command the moment a `/responses` provider and key are at hand; its
   reconciliation notes land in this plan as W2's deltas-close. Until
   then the adapter builds against the documented shapes with the stub
   pins below — the E0 precedent, byte for byte.
2. **W1 — the roster learns the protocol.** `Candidate` gains
   `protocol: Option<Protocol>` (an entry-level default, `chat` when
   absent) and `model_protocols: BTreeMap<String, Protocol>` (the
   per-model override, serde-default like `disabled_models`); the edit
   overlay and the merge gain the arms; the settings writer validates
   the value. `one_attempt` reads the position's model's protocol and
   routes the request through the matching mapper — a mixed gateway's
   walk serves kimi over `/chat/completions` and a Luna model over
   `/responses` in one run.
3. **W2 — the adapter.** A `responses` module in `api.rs`: the request
   mapper (`ChatRequest` → the `/responses` body — system frames to
   `instructions`, messages to typed input items, tool results to
   `function_call_output`, tools to the flat function shape,
   `store: false`, no `previous_response_id`), the response mapper
   (output items → `ChatResponse`'s choices/content/tool calls, usage
   from `response.completed`), and the streaming mapper (the typed SSE
   events → the existing `StreamEvent` vocabulary, including the
   argument-delta fragments). Both request paths route through the
   mapper when the protocol says so; the chat-completions path is
   byte-identical when they do not.
4. **W3 — `{trace}`.** A third placeholder beside `{session}` and
   `{env:NAME}`: a fresh UUID-shaped id per preparation — distinct from
   `{session}` even when both are derived, because a trace is not a
   session. Preparation-scoped, documented as such; the per-run
   refinement stays named out of scope (the threading it needs is the
   churn two plans have already measured).
5. **W4 — the face's headers row.** The account card (settings-ux U9's
   one card) gains the headers editor: the names and the **config's own
   values** (placeholders visible — what the owner typed), never a
   resolved value, the constant-value warning rendered inline beside
   the field, add/edit/remove in the card. Sequenced after U9; if U9
   has not landed, this increment waits rather than bolting a row onto
   the surface that plan is reworking.
6. **W5 — the docs and the gate.** routing.md's protocol passage and
   the trace placeholder; gui.md's settings section gains the row;
   TECHNICAL-DOCUMENTATION.md and FUNCTIONAL-SPECIFICATIONS.md carry
   the field rows; the four-leg gate by exit code.

**Decision-model check:** the protocol mapping is deterministic
translation, the placeholder is a generator, the row is a form — no
per-item classification, scoring, or gate fits sys1; nothing delegates.

## Use cases

- [ ] A: a mixed gateway (the OpenCode Go shape) serves a
      chat-completions model and a Luna-class model in one walk — each
      request spoken in the protocol its model declares, the tool loop
      whole on both.
- [ ] B: the owner points a provider at an observability gateway and
      composes `x-portkey-trace-id: "{trace}"` in the roster's headers —
      the `{trace}` placeholder resolves in any header value to a fresh
      UUID-shaped id per preparation, distinct from the same value's
      `{session}` id, so the gateway sees one trace per preparation and
      one session per conversation.

## Acceptance criteria

- [x] **C0 — the probe exists and its reconciliation lands.** The
      harness is committed, runs in one command against a configured
      `/responses` provider, captures the request and the streamed
      answer under `tmp/`, and this plan records the documented shapes'
      deltas when it runs — before W2's mapper is called done on a live
      provider. Until a provider is at hand, the harness's absence of a
      run is a named state here, not a silent one. *`scripts/responses-probe.sh` committed, syntax-checked, runnable in one command against a configured provider — and its absence of a run is the named state the criterion itself defines: no `/responses` provider or key sits on this machine today, so the live reconciliation rides the probe's first run. The stub pins carry the adapter until then.*
- [x] **C1 — the protocol fields parse, merge, and route.** An entry's
      `protocol` and a model's `model_protocols` override parse and
      merge through the writer (an unknown protocol value refused by
      name); a mixed gateway's walk serves a chat model over
      `/chat/completions` and a responses-marked model over
      `/responses` in one run. Pinned at the config layer and the walk
      layer. *`a_protocol_rides_the_roster_and_the_model_wins` (entry default, model override, compiled-roster chat, the unknown-value refusal) — route.rs — and `a_responses_marked_model_rides_the_responses_path` (the walk serving a responses-marked model over `/responses`, never the chat path, in one run) — api.rs.*
- [x] **C2 — the adapter speaks the wire, statelessly.** Against a stub
      `/responses` route: the request carries `store: false`, no
      `previous_response_id`, `instructions` from the system frame, the
      typed input items, and the flat tool shape; the answer maps to
      the assembled `ChatResponse` with usage; the streamed events map
      to `StreamEvent` (text deltas, argument fragments, the tool
      calls, the close); a full tool-call round trip runs the agent
      loop. The chat-completions path is byte-identical when the
      protocol is `chat`. Pinned on every leg. *`the_responses_request_maps_the_history_statelessly` (`store: false`, no `previous_response_id`, the typed items, the flat tool shape), `the_responses_answer_maps_back_to_the_chat_shape`, `the_responses_stream_maps_the_typed_events` (deltas live, the fragment consumed, the terminal event's authoritative call, usage with its finish reason) — and the chat-completions pins byte-identical throughout (the full suite). Pinned on every leg.*
- [x] **C3 — `{trace}` is fresh, distinct, and shaped.** A preparation's
      `{trace}` value is UUID-shaped, differs from its `{session}`
      value even when both are derived from the same seed, and differs
      across preparations. Pinned. *`a_trace_is_fresh_distinct_and_shaped` — fresh per preparation, distinct from the same seed's session id, UUID-shaped, substituted beside it.*
- [x] **C4 — the face's headers row completes the chain on one card.**
      The account card carries the headers editor (names, the config's
      own values, add/edit/remove), never renders a resolved value, and
      shows the constant-value warning inline. Face-harness pinned.
      Lands with settings-ux U9's card or after it.
      *LANDED 2026-11-02 — U9's card arrived (`d13ca29`) and the row moved
      in the same day: the read carries the CONFIG's headers
      (`RosterEntry.headers` taken from the raw candidate beside the
      resolved one — `resolve` substitutes `{env:NAME}` in place, so the
      zip feeds the surfaces the config's own values), the card edits
      them with add/remove and renders the trap warning live beside a
      constant session value, and the save writes the entry's
      `headers` object through the provider write it already makes.
      Eight face pins (the pre-filled placeholder, the trap's live
      show-and-silence, add/remove, the save's write, the no-resolved-
      value rule the server's own credential sweep enforces on every
      payload). Harness 633/0 across 56 cases.*
- [x] **C5 — the docs and the gate.** routing.md (the protocol passage,
      the trace placeholder), gui.md (the row), TECHNICAL-DOCUMENTATION.md
      and FUNCTIONAL-SPECIFICATIONS.md (the field rows); fmt, clippy,
      build, test green by exit code. *routing.md's wire-dialect passage and the three-placeholder list; TECHNICAL-DOCUMENTATION.md's Keys bullet; FUNCTIONAL-SPECIFICATIONS.md's headers and dialect rows. fmt 0, clippy 0 (`-D warnings`), `cargo build --release` 0, `cargo test` 0 — 1,941 passed / 0 failed, the face harness green inside the run. The gui.md row rides W4's landing (below).*

## Files to be touched

**Router / client:**
- `src/api.rs` — the `responses` adapter module (request, response, and
  streaming mappers), the `Protocol` type, the path selection in both
  request paths, the stub `/responses` route and its pins
- `src/route.rs` — the `Candidate`/`ProviderEntry` protocol fields, the
  merge arms, `one_attempt`'s protocol read, the pins

**Settings:**
- `src/settings.rs` — the protocol arms in the writer's validation

**Face (W4, sequenced with settings-ux U9):**
- `src/gui/assets/app.js` — the account card's headers editor
- `tests/face/harness.mjs` — the row's pins

**Scripts:**
- `scripts/responses-probe.sh` — the W0 harness (never runs in CI or
  the gate; on demand only)

**Surfaces:**
- `src/main.rs` — nothing expected; the providers card unchanged
  (protocol shown, if anywhere, in the routing.md doc and the face's
  card)

**Documentation (see Linked artifacts below):**
- `docs/features/routing.md`, `docs/features/gui.md`,
  `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`

## Out of scope

- **Server-side conversation state.** `previous_response_id`, the
  Conversations API, `store: true` — the adapter speaks the dialect,
  never the state mechanism; hard rule 5 is why, and it is pinned, not
  promised.
- **The per-run trace refinement.** `{trace}` is preparation-scoped; a
  per-run id would thread a value through every walk signature — the
  churn two plans measured — for no gateway Fleet targets today.
- **Non-OpenAI Responses dialects.** The adapter maps the documented
  OpenAI shape; a provider that extends it (reasoning-item variants,
  built-in tools other than functions) is a delta against the probe's
  capture, named here when the probe runs.
- **The providers card's protocol column.** The roster card stays
  names-only; a protocol shown nowhere on the CLI is acceptable — the
  doc and the face's card carry it.

## Verification

**C0 — the probe:**
```sh
scripts/responses-probe.sh [dotenv-file]   # on demand, when a provider is at hand
```
Expected: the two captures under `tmp/responses-probe/`, and this
plan's W2 delta notes updated in the same turn.

**C1 — the protocol fields and the mixed walk:**
```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo test --lib route:: settings::
```
Expected: the parse/merge/validation pins green; the mixed-gateway walk
pin green (one run, two paths).

**C2 — the adapter:**
```sh
cargo test --lib api::
```
Expected: the request, response, streaming, tool-call-round-trip, and
statelessness pins green against the stub `/responses` route; the
chat-completions pins unchanged.

**C3 — the trace placeholder:**
```sh
cargo test --lib route::
```
Expected: the freshness/distinctness/shape pin green.

**C4 — the face's row:**
```sh
node tests/face/harness.mjs
```
Expected: the headers row's pins green — names and config values
editable, no resolved value rendered, the warning inline.

**C5 — the gate, by exit code:**
```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: every command exits 0, read directly (KI-014).

## Linked artifacts

- `docs/features/routing.md` — the protocol fields, the adapter's
  statelessness rule, the `{trace}` placeholder, the Luna-class reach
- `docs/features/gui.md` — the account card's headers row (U9's card)
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — the
  roster's protocol fields and the adapter's dialect-only rule
- `FUNCTIONAL-SPECIFICATIONS.md` section "The commands as verbs" — the
  provider edit op's protocol arms

## Risks

- **The documented shape is not the wire.** The whole reason W0 exists;
  the adapter's stub pins prove the mapper against the documented
  shapes, and the probe's reconciliation is the gate before any live
  claim. A delta re-opens W2, never ships silently.
- **Argument-delta fragmentation** (the LiteLLM #20711 class) — the
  streaming mapper assembles fragments the same way the chat path
  assembles `ToolCallDraft`s; the pin streams a fragmented call.
- **W4 collides with settings-ux's open work** in `app.js` — the
  increment is sequenced after U9's card and re-based on whatever shape
  that card lands with; it is never built beside it.
- **A responses-marked model on a gateway that does not speak the
  protocol** fails the attempt by name (the path 404s into the existing
  error machinery) and the walk advances — the named failure, not a
  hang.

## Dependencies

- `docs/plans/2026-11-02-provider-session-headers.md` and
  `docs/plans/2026-11-02-session-header-parity.md` — the field, the
  placeholder machinery, and the two-gate validation this plan extends.
- `docs/plans/2026-11-01-settings-ux.md` — U9's account card is where
  W4 lands; the increment waits for it.
- A `/responses` provider and key — W0's live half and the final
  reconciliation; absent, the named state in C0 stands.

## Notes

- The three out-of-scope items this plan closes come from
  `2026-11-02-session-header-parity.md`'s Out of scope section; the
  fourth (fingerprint auto-generation) is closed there by examination
  and recorded here so the set is whole.
- The statelessness rule is the plan's spine: the fork's #5495 reached
  the same shape for DSH, and hard rule 5 requires it — a server-held
  conversation would break the prompt-reconstructable law that has held
  since slice one.
- Alternatives considered: an entry-level protocol only (rejected —
  the mixed gateway is the real case, and OpenCode's is exactly that);
  extending `ModelSettings` with the protocol (rejected — that bag's
  vocabulary is windows and ceilings, and its precedence machinery is
  provider-vs-hand; a protocol is the owner's routing fact, not a
  model's declared number); per-request protocol threading through the
  walk signatures (rejected — the position's model already resolves the
  protocol from the candidate the walk is standing on).
