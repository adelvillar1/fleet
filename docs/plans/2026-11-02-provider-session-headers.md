---
status: completed
created: 2026-11-02
updated: 2026-11-02
slug: provider-session-headers
---

# Plan: per-provider session headers on the chat client

## Context

zai-org/feedback #840: ZCode's requests to the OpenCode Go gateway are
rejected with HTTP 400 `MissingSessionID` on session-gated models because
the client never sends `x-opencode-session`, and there is no way to make
it send one. The 2026-11-02 check against Fleet's own chat path found the
same exposure, latent: both request paths set exactly one header
(`Authorization`, plus `Content-Type` when streaming — `src/api.rs:614`,
`src/api.rs:759`), and the roster entry carries no field that could ask
for another (`src/route.rs:215`). Nothing in today's roster points at a
session-gated gateway, so nothing bites — but a provider that wants one
more header (a session id, a second token, a routing hint) is
unreachable, and the fix belongs in the roster's own vocabulary, not in a
hardcoded special case.

Fleet already does the half of #840 that made zcode's bug hard to
diagnose: the upstream body survives (`api_error` extracts
`error.message` or keeps a 300-char excerpt, `src/api.rs:916`) and lands
typed in the log (`Record::TurnFailed`, `src/agent.rs:2609`). This plan
closes the other half — the header Fleet cannot send — and leaves
Fleet's 400-does-not-failover semantics alone (see Out of scope).

## Approach

**The roster learns `headers`, the client learns to send them.** One
field end to end, reusing the seams `base_url` and `key_env` already
walk: the roster entry, the edit overlay, `resolve()`, `Router::build`,
and the two request sites. No new crate, no new token, no signature
churn through the walk — the values are resolved once at router-build
time, where a failure can be named the same way a missing lane spec is.

High-level steps:

1. **The field.** `Candidate` gains `headers: BTreeMap<String, String>`
   (serde default — every existing roster and `routing.json` parses
   byte-identical; BTreeMap for deterministic wire order). The edit
   overlay `ProviderEntry` gains the optional twin, and the provider
   merge in the settings writer (`src/route.rs:1141` shape) gains the
   headers arm beside `base_url`/`key_env`.
2. **Validation, twice.** The settings writer refuses a reserved name
   (`authorization`, `content-type`, `host`, `content-length` —
   case-insensitive; they would clobber the client's own), an empty
   name, and a malformed placeholder, each refusal naming the header.
   `Router::build` re-runs the same check for a hand-edited file, so a
   bad name fails the walk by name there too — one shared helper, two
   call sites.
3. **Placeholders, resolved at build.** A value may carry `{session}` —
   a stable id generated once per client instance (short random hex
   from pid + time; no new crate) so the gateway sees one stable id per
   router preparation — or `{env:NAME}`, resolved through the dotenv
   chain `resolve_key` already reads (`src/route.rs:1258`). Literal
   text passes through. An `{env:NAME}` that resolves to nothing is
   left standing at roster resolution and named at the walk's first
   attempt on that provider — the lane's `Result` shape, so one
   misconfigured provider cannot fail the walk for the providers that
   would have served. Resolved pairs land on the client via
   `Client::with_headers(base_url, key, headers)`.
4. **The request paths.** `chat()` and `stream_response()` set the
   stored pairs after `Authorization` (ureq `.set`, in map order).
   Because reserved names were refused at both gates, nothing can
   clobber the client's own headers.
5. **Surfaces.** `fleet providers` prints header NAMES only (the count
   and the names; never a value — a value may carry an env-sourced
   secret). `fleet settings`' provider edit op writes the field as a
   JSON object. The GUI face gains nothing this plan: the account card
   is settings-ux U9's, and the headers row rides that rebuild.
6. **The docs.** `docs/features/routing.md` gains the field's passage
   (syntax, the two placeholders, the reserved names, the masking
   rule); TECHNICAL-DOCUMENTATION.md's roster contract and
   FUNCTIONAL-SPECIFICATIONS.md's providers passage gain the field's
   rows — the third gate (contract docs) walks with the feature.
7. **The gate.** The four-leg gate by exit code — fmt, clippy, build,
   test — before the push that carries this code; the face harness
   green unchanged.

**Decision-model check:** nothing here fits sys1 — validation is a fixed
list check and placeholder resolution is deterministic string work; there
is no per-item classification, scoring, or gate to delegate.

## Use cases

- [ ] A: Fleet pointed at a session-gated OpenAI-compatible gateway (the
      #840 shape) sends that gateway's required header and the request
      routes instead of 400ing.
- [ ] B: a gateway wanting a second secret header gets it from the env
      chain via `{env:NAME}` without the secret ever entering
      `routing.json` or any surface.

## Acceptance criteria

- [x] **C0 — the field parses, overlays, and stays optional.** A roster
      entry carrying `headers` parses whole; a roster without the field
      parses byte-identical to today's; the provider edit op merges a
      headers object over an existing entry without disturbing its other
      fields. Pinned by config round-trip tests. *`a_roster_without_headers_is_the_empty_map` (byte-identical parse), `headers_merge_over_an_entry_without_disturbing_its_fields`, `a_fully_specified_new_entry_carries_its_headers` (route.rs).*
- [x] **C1 — a bad header is refused by name, at both gates.** The
      settings writer refuses a reserved name (case-insensitive), an
      empty name, and a malformed placeholder, each naming the header;
      the same refusal fires at `Router::build` for a hand-edited file.
      Pinned at both call sites. *`a_bad_header_is_refused_naming_it` (the shared check's five refusals), `a_config_proposals_headers_ride_the_entry_gate`, and the settings writer's arms in `a_provider_entry_validates_through_the_rosters_own_schema` (settings.rs) — plus `a_bad_header_in_a_hand_edited_roster_fails_the_walk_by_name` for the build's second gate.*
- [x] **C2 — both request paths carry the headers.** Against the stub
      server, `chat()` and the streamed path each present every
      configured header beside `Authorization` (which is still set from
      `key_env`), and the streamed path keeps its `Content-Type`.
      Pinned. *`a_configured_header_rides_both_request_paths` against the stub (both legs, order after `Authorization`, `Content-Type` intact), `a_plain_client_sends_no_extra_headers` for the unchanged default (api.rs).*
- [x] **C3 — placeholders resolve honestly.** `{session}` in a value is
      stable across two calls on one client and distinct between two
      clients; `{env:NAME}` resolves through the dotenv chain; a
      missing env name fails the walk at that provider, naming the
      header and the var — the lane's shape, so one misconfigured
      provider cannot fail the walk for the providers that would have
      served. Pinned. *`the_session_placeholder_resolves_stable_and_distinct` and `an_env_placeholder_resolves_from_the_dotenv_chain` (route.rs).*
- [x] **C4 — no surface shows a value.** `fleet providers` prints the
      header count and names only; a pin asserts a resolved env-sourced
      value never appears in the output. Pinned. *`the_providers_card_names_headers_and_never_a_value` — names and count present, a resolved secret and an unresolved placeholder both absent (route.rs).*
- [x] **C5 — the docs carry it.** `docs/features/routing.md` documents
      the field (syntax, both placeholders, the masking rule);
      TECHNICAL-DOCUMENTATION.md's roster contract and
      FUNCTIONAL-SPECIFICATIONS.md's providers passage name it. *`docs/features/routing.md`'s **Per-provider headers** passage (Config) and the providers-card bullet (Surfaces); TECHNICAL-DOCUMENTATION.md's **Keys** bullet; FUNCTIONAL-SPECIFICATIONS.md's verbs table row.*
- [x] **C6 — the gate.** fmt, clippy, build, test green by exit code;
      the face harness green unchanged (no face change in this plan). *fmt 0, clippy 0 (`-D warnings`), `cargo build --release` 0, `cargo test` 0 — 1,736 lib + 194 across the other targets, and the face harness green unchanged inside the run (tests/face.rs, 16/0). Every exit code read directly.*

## Files to be touched

**Router / client:**
- `src/route.rs` — the `Candidate`/`ProviderEntry` fields, the merge
  arm, `resolve()`, `Router::build`'s validation + placeholder
  resolution, `Client::with_headers`
- `src/api.rs` — the `headers` field on `Client`, both request sites,
  the stub-server pins

**Settings:**
- `src/settings.rs` — the provider edit op's headers arm + writer
  validation
- `src/cli.rs` — nothing expected; only if the edit op's grammar needs a
  help line

**Surfaces:**
- `src/main.rs` (the `fleet providers` card) — names-only display

**Tests:**
- `src/api.rs` `#[cfg(test)]` — C2/C3 stub-server pins
- `src/route.rs` `#[cfg(test)]` — C0/C1/C3 build-time pins
- `src/settings.rs` `#[cfg(test)]` — C0/C1 writer pins

**Documentation (see Linked artifacts below):**
- `docs/features/routing.md` — the field's passage
- `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — the
  contract rows

## Out of scope

- **The `/responses` protocol.** #840's related gap — the Luna models
  speak only the Responses protocol; Fleet speaks chat-completions only
  (`src/api.rs:610,656`). That is an adapter plan of its own if the
  owner ever wants those models; a header fix does not carry it.
- **Fleet's 400-failover semantics.** `failure_class` maps 400 to
  `client-error`, which does not advance the walk
  (`src/route.rs:2296,2318`) — with this plan, a correctly-configured
  gateway stops producing that 400, and flipping the semantics would
  burn the whole chain on every genuine bad-request bug. Changing it
  needs its own calibration-backed ruling.
- **The GUI face.** No face change: the account card's headers row is
  settings-ux U9's rebuild, not a row bolted onto the surface that plan
  is reworking.
- **Per-conversation id granularity in the GUI.** `{session}` is stable
  per router preparation; where the face prepares one route for many
  conversations they share an id. The gateway's requirement is a stable
  id, which this satisfies; finer granularity would thread session ids
  through every walk signature for no wire-visible gain.

## Verification

**C0 — the field parses, overlays, and stays optional:**
```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo test --lib route:: settings::
```
Expected: the roster round-trip pins green — an entry with `headers`
parses whole, one without parses byte-identical to today's, the edit op
merges without disturbing sibling fields.

**C1 — a bad header is refused by name, at both gates:**
```sh
cargo test --lib settings:: route::
```
Expected: the writer-refusal pins green (reserved name case-insensitive,
empty name, malformed placeholder — each naming the header) and the
`Router::build` refusal pin green for a hand-edited file.

**C2 — both request paths carry the headers:**
```sh
cargo test --lib api::
```
Expected: the stub-server pins read every configured header off both the
`chat()` request and the streamed request, `Authorization` still set from
`key_env`, `Content-Type` still set on the stream.

**C3 — placeholders resolve honestly:**
```sh
cargo test --lib api:: route::
```
Expected: `{session}` stable across two calls on one client and distinct
across two clients; `{env:NAME}` resolved from the dotenv chain; the
missing-env build failure naming the header and the var.

**C4 — no surface shows a value:**
```sh
cargo test --lib main::   # the providers card's own module
```
Expected: the names-only pin green — a resolved env-sourced value never
appears in the card's output.

**C5 — the docs carry it:**
Read `docs/features/routing.md`'s new passage, then:
```sh
grep -c "headers" docs/features/routing.md TECHNICAL-DOCUMENTATION.md FUNCTIONAL-SPECIFICATIONS.md
```
Expected: the passage present in all three (syntax, both placeholders,
the masking rule; the contract rows), non-zero counts in each.

**C6 — the gate, by exit code:**
```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: every command exits 0, read directly (KI-014).

## Linked artifacts

- `docs/features/routing.md` — the `headers` field: syntax, the two
  placeholders, the reserved names, the names-only masking rule
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — the
  roster schema's new field and the build-time refusal
- `FUNCTIONAL-SPECIFICATIONS.md` section "The commands as verbs" — the
  provider edit op's headers arm and the `fleet providers` card's
  names-only display

## Risks

- **A header value can carry a secret** (`{env:NAME}`). Defense: the
  names-only display rule, pinned at the one card that renders the
  field; the calibration rows and route records carry provider/model,
  never headers. The GUI log's hardcoded secret sweep
  (`src/gui/server.rs:4557`) cannot know owner-chosen names — the
  display rule is the load-bearing defense, and the plan states it.
- **A misconfigured header breaks a provider that worked.** The refusals
  fire at write time and at build time, both naming the header; the
  edit op is reversible (write `headers` as `{}` / remove the field).
- **A gateway chokes on an unexpected header.** The field is per
  provider and defaults to absent; existing rosters send exactly what
  they sent before.

## Dependencies

- `docs/plans/2026-11-01-settings-ux.md` — U9's account card is the
  future GUI home of the field; this plan ships the config, CLI, and
  `fleet providers` surfaces only. No build-order dependency (the CLI
  field lands independently of the card).

## Notes

- **One mid-implementation amendment (2026-11-02, built as amended):**
  the env-miss failure shape. The plan's first draft had an unresolved
  `{env:NAME}` fail the router build outright — but a build failure on
  one candidate's header would brick every walk, including runs whose
  chain never reaches that provider. The built shape carries the named
  reason in the endpoint (`Endpoint::Http(Result<Client, String>)`, the
  lane's own `Result<ServerSpec, String>` shape) and fails the walk at
  that provider's first attempt, so the other candidates still serve.
  C3's wording was updated before the landing commit; everything else
  built as drafted.

- Alternatives considered and rejected: (a) hardcoding
  `x-opencode-session` unconditionally — a gateway-specific header has
  no place in a general client, and the roster is where per-provider
  facts live; (b) threading a per-request session id through
  `chat_stream`/`one_attempt` signatures — churn across the walk, lane
  boundary, and every caller for granularity the gateway's contract
  does not require; (c) `Vec<(String, String)>` over `BTreeMap` — the
  map keeps the writer's JSON native and the order deterministic; the
  value of ordering here is reproducible requests, which the map gives.
- The `{session}` id is generated per client instance, so every router
  preparation gets a fresh one — a run-scoped id, stable for the run's
  whole walk, which is the caching granularity the gateway asks for.
- Evidence for the exposure this plan closes: the 2026-11-02 check
  (conversation record) — `src/api.rs:614` and `:759` set
  `Authorization` only; no header field exists on
  `src/route.rs:215`'s `Candidate`; the upstream body already survives
  to the log (`src/api.rs:916`, `src/agent.rs:2609`).
