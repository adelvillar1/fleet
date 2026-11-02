---
status: completed
created: 2026-11-02
updated: 2026-11-02
slug: session-header-parity
---

# Plan: session-header parity — what the gateway landscape does that Fleet does not

## Context

The provider-session-headers plan landed today: a roster entry can carry
`headers`, `{session}` resolves to a stable per-client id, and a
session-gated gateway stops 400ing. This plan asks the next question:
**how do the gateways and providers of this landscape actually use
session ids, and where does Fleet's shape fall short of what they reward
or require?** The answer comes from the OpenCode Zen docs, the owner's
own fork's migration discussion (deepseek-harness #5495), and the
gateway documentation of the tools that operate this exact mechanism
(LiteLLM, vLLM Production Stack, SGLang, Vercel, Cloudflare, Helicone,
Portkey).

## Research

**The mechanism's two jobs.** Every implementation found uses a
session id for one or both of: (1) **affinity** — a cache- or
replica-routing key, so a conversation's turns land on the same backend
and its prompt/KV cache is reused (OpenCode Zen's "route a conversation
consistently and reuse its prompt cache"; vLLM Production Stack's
`session` routing logic hashing a client-chosen `sessionKey` header;
SGLang's consistent-hash session affinity over the session-aware radix
cache; Vercel AI Gateway's session affinity; LiteLLM's cache-aware
deployment affinity reading `metadata.session_id`); (2) **grouping** —
an observability key (Helicone's `Helicone-Session-Id`/`-Session-Path`,
Portkey's `x-portkey-trace-id` + `x-portkey-metadata`, Cloudflare AI
Gateway's `cf-aig-metadata`, LiteLLM's spend-log `session_id` column).

**The id's lifecycle contract, converged across sources**: stable per
conversation — across turns, resume, compaction, and retries — and
fresh for new conversations, forks, and subagent sessions (the fork's
#5495 resolution names this list; OpenCode's harnesses doc says
"stable-id-per-conversation"; Anyscale notes the operational burden of
requiring clients to send one at all).

**The static-value trap, measured.** The fork's discussion diagnosed
the static-header workaround — the shape Fleet's config permits today —
as harmful: a constant value collapses all conversations into one
affinity bucket, degrading cache locality, with users reporting slower
calls and higher costs and one confirming session switches still hit
the same replica. The community adopted instead a `sessionHeader` config
field naming the header, with the runtime supplying the per-conversation
value from the session's id.

**Format and validation.** OpenCode's gateway requires the header's
presence and does not strictly validate the value (an early "UUID
enforced" claim was retracted as a deployment artifact), but an
RFC-4122-shaped UUID is the documented-safe choice; implementations
derive an opaque per-conversation id rather than sending the harness's
internal session id, and validate header-safe characters to avoid
CRLF injection. LiteLLM accepts any `x-<vendor>-session-id` header,
body metadata, or W3C baggage, and can auto-generate when none is sent.

**The response side.** Providers and gateways return per-request
diagnostic ids — Anthropic's `request-id` (plus Bedrock's
`x-amzn-Requestid`), LiteLLM's `x-litellm-call-id`, OpenCode's
`x-opencode-log-id` ("attach to upstream reports"), Cloudflare's
`cf-ray`. The fork's guidance is to quote the log id when reporting a
refusal upstream. OpenAI's Responses API carries session state at the
protocol level (`previous_response_id`, the Conversations API) — a
different mechanism from headers, and chat-completions-only clients
(Fleet included) are outside it.

**Protocol-level statefulness** (the Responses API's
`previous_response_id`) is the one session mechanism a header cannot
replicate; it stays out of scope (see Out of scope).

## The gaps, against Fleet as built

Each gap names its evidence in Fleet's tree as of today.

- **G1 — the id is stable across a conversation's turns but not across
  its resume.** `{session}` is generated per client instance at router
  preparation (`fresh_session_id`, `src/route.rs`): the GUI caches one
  preparation per driving conversation (`bridge.rs`'s `routes` map), and
  `fleet chat` prepares once per session — but a **resumed** session
  (`run --session <id>`, a reopened conversation after a face restart)
  re-prepares and gets a fresh id, moving the conversation to a new
  affinity bucket. The fork's contract names resume explicitly.
- **G2 — the id's shape leaks machine facts and misses the safe
  format.** `fresh_session_id` is `pid ^ nanos` hex: process-local
  facts, not the RFC-4122 shape every source calls the safe choice, and
  a resumed conversation cannot reproduce it even in principle (G1's
  root). The fork's privacy note also applies: the wire value should be
  opaque — derivable from the conversation's id without being it.
- **G3 — header names and values are not injection-safe.**
  `validate_headers` checks emptiness, reservation, and placeholder
  syntax — but nothing refuses a control character: a config literal or
  an env-sourced `{env:NAME}` value carrying `\r\n` would inject headers
  into the request line (the fork's CRLF note; ureq does not sanitize).
  The build's gate runs on post-substitution values, so the check needs
  to exist and to sit after substitution — the writer's half sits before
  it.
- **G4 — the response's diagnostic ids are dropped.** `api.rs` reads
  status and body only; `ApiError` carries no request id, so a refused
  request cannot be reported with the id the provider asked to see
  (the fork: attach `x-opencode-log-id`; Anthropic: quote `request-id`).
  The `TurnFailed` record inherits the omission.
- **G5 — the static-value trap is silently permitted.** A literal
  constant in a session-named header parses, validates, and ships — the
  exact shape the fork measured as harmful. Fleet's surfaces never say
  so; the writer has no warning, the docs no trap note.

## Approach

**Close the lifecycle contract, the injection hole, and the
diagnostic loop — in that order — without touching the walk's
signatures.** The session seed rides the preparation that already
exists; the id becomes a deterministic, opaque, UUID-shaped derivation
of that seed; the validator learns the character rules; the client
learns to keep what the provider tells it.

High-level steps:

1. **The seed rides the preparation (G1).** `prepare_with_pin` and
   `prepare` gain an optional `session_seed: Option<&str>` — the
   conversation's id when one exists. `run` passes the resumed
   session's id (`--session`), `None` for a fresh conversation (fresh
   id, per the contract); `chat` passes its session's id; the GUI's
   prepare passes the driving session's id — so a re-preparation after
   a pin flip or a face restart rebuilds the *same* bucket for the
   same conversation. Delegated children keep fresh ids (new
   conversations, per the contract) and `prepare_child` is untouched.
2. **The id becomes an opaque UUID-shaped derivation (G2, with G1).**
   `{session}` resolves at build time to a deterministic UUID-shaped
   string hashed from the seed — two `calib::hash16`-style passes over
   the seed, rendered 8-4-4-4-12 — so the internal session id never
   rides the wire and a resumed conversation reproduces its id
   byte-for-byte. No seed: a fresh random UUID-shaped id, as today.
   `fresh_session_id` becomes the no-seed arm of that one helper.
3. **The validator learns injection safety (G3).** `validate_headers`
   refuses a control character (`C0`–`1F`, `7F`) or non-token character
   in a **name**, and a control character in a **value** — each refusal
   naming the header. The writer's half checks the config literals;
   the build's half already checks post-substitution values, so an
   env-sourced value with a carriage return is caught there. Pinned at
   both gates.
4. **The client keeps the provider's diagnostic ids (G4).** On a
   non-200 answer, both request paths capture a bounded allowlist of
   response headers — `x-request-id`, `request-id`, `x-opencode-log-id`,
   `x-litellm-call-id`, `cf-ray` — into `ApiError.request_id` (first
   present wins, value bounded), the Display names it
   (` — request id …`), and `TurnFailed` records it with the message.
   Success answers stay untouched.
5. **The trap is named where it is set (G5).** The settings writer, on
   saving a `headers` map, prints a non-blocking warning when a header
   whose name contains `session` carries a value with no placeholder —
   the one-line cost of the collapsed affinity bucket, in the fork's
   own measured words. `docs/features/routing.md` gains the trap note
   beside the field's passage.
6. **The docs.** routing.md's headers passage gains the lifecycle
   contract (resume stability, the opaque derivation, the static-value
   trap, the request-id capture); TECHNICAL-DOCUMENTATION.md's Keys
   bullet and FUNCTIONAL-SPECIFICATIONS.md's verbs row gain the same
   facts — the third gate walks with the feature.
7. **The gate.** The four-leg gate by exit code — fmt, clippy, build,
   test — before the push that carries this code; the face harness
   green unchanged.

**Decision-model check:** nothing here fits sys1 — the seed derivation
is deterministic hashing, the character rules are a fixed table, the
capture is an allowlist read; there is no per-item judgment to delegate.

## Use cases

- [ ] A: a conversation resumed after a restart (CLI `--session`, a
      reopened face conversation) presents its gateway the same session
      id it presented before, so the affinity bucket and its cache
      survive the restart.
- [ ] B: a provider or gateway refusing a request is reportable
      upstream: the refusal's message carries the request id the
      provider returned, and the log's `TurnFailed` row records it.

## Acceptance criteria

- [x] **C0 — the id is opaque, UUID-shaped, and resume-stable.**
      `{session}` resolves to an 8-4-4-4-12 hex string containing no
      process fact and never the raw session id; a seed produces the
      same value on every preparation; two different seeds differ; no
      seed produces a fresh value per preparation. Pinned: determinism,
      distinctness, shape, and the seed's ride through `prepare` →
      build → wire for a resumed session. *`the_session_id_is_uuid_shaped_opaque_and_resume_stable` (determinism, distinctness, shape, idempotence, the raw id's absence) and `a_resumed_sessions_seed_rides_the_preparation` (the seed's prepare → Prepared ride, same id across preparations, distinct across conversations) — route.rs; the mid-run re-route re-seeds through `AgentOptions.session_id` (agent.rs `apply_pin`).*
- [x] **C1 — headers are injection-safe at both gates.** A name with a
      control or non-token character is refused naming the header; a
      value with `\r`, `\n`, or `\0` is refused naming the header — at
      the writer (config literal) and at the build (an env-sourced
      value substituted before the check). Pinned at both. *`a_control_character_in_a_header_is_refused_naming_it` — non-token names, control characters in values, and the env-substituted value caught at the build — route.rs, with the writer's half inside `a_provider_entry_validates_through_the_rosters_own_schema` (settings.rs).*
- [x] **C2 — a refusal carries the provider's request id.** Against the
      stub answering 400 with `x-request-id` set, both request paths'
      `ApiError` carry the captured id and the Display names it; an
      answer without the header carries `None`; the allowlist header
      present on a 200 is not captured. Pinned. *`a_refusal_carries_the_providers_request_id` (both paths, the Display's words, the 200 non-capture) and `an_answer_without_the_header_names_nothing` — api.rs, stub-driven.*
- [x] **C3 — the static-value trap is named at the writer.** Saving a
      headers map whose session-named header carries a constant value
      prints the affinity warning and still saves; a placeholder value
      prints nothing; the routing.md passage carries the same words.
      Pinned. *The writer's arms inside `a_provider_entry_validates_through_the_rosters_own_schema` (settings.rs): the constant warns (advisory), the placeholder and the unrelated header stay silent, the save lands.*
- [x] **C4 — the docs carry the lifecycle contract.** routing.md's
      headers passage names resume stability and the opaque derivation;
      TECHNICAL-DOCUMENTATION.md's Keys bullet and
      FUNCTIONAL-SPECIFICATIONS.md's verbs row match. *routing.md's headers passage (resume stability, the opaque derivation, the static-value trap, the refusal's id) + the Surfaces bullet; TECHNICAL-DOCUMENTATION.md's Keys bullet; FUNCTIONAL-SPECIFICATIONS.md's verbs row.*
- [x] **C5 — the gate.** fmt, clippy, build, test green by exit code;
      the face harness green unchanged (no face change beyond the
      prepare call's seed argument). *fmt 0, clippy 0 (`-D warnings`), `cargo build --release` 0, `cargo test` 0 — 1,935 passed / 0 failed across every target, the face harness green unchanged inside the run (tests/face.rs, 16/0). Every exit code read directly.*

## Files to be touched

**Router / client:**
- `src/route.rs` — the seed parameter on `prepare`/`prepare_with_pin`,
  the UUID-shaped derivation helper replacing `fresh_session_id`'s arm,
  `validate_headers`' character rules, the pins
- `src/api.rs` — `ApiError.request_id`, the allowlist capture on both
  error paths, the Display arm, the stub pins

**Callers of prepare:**
- `src/main.rs` — `run`/`chat` pass the resumed session's id
- `src/gui/bridge.rs` — the three prepare sites pass the driving
  session's id where one exists

**Settings:**
- `src/settings.rs` — the writer's session-value warning

**Tests:**
- `src/route.rs`, `src/api.rs`, `src/settings.rs` `#[cfg(test)]` — C0–C3
  pins

**Documentation (see Linked artifacts below):**
- `docs/features/routing.md`, `TECHNICAL-DOCUMENTATION.md`,
  `FUNCTIONAL-SPECIFICATIONS.md`

## Out of scope

- **The Responses protocol** (`previous_response_id`, the Conversations
  API) — protocol-level session state is an adapter plan of its own; a
  header cannot carry it, and Fleet speaks chat-completions only.
- **`{trace}` or per-run observability ids** — Fleet's one preparation
  per conversation makes `{session}` serve both jobs for the gateways
  it targets; a distinct per-run trace id would thread a value through
  every walk signature for no wire-visible gain today. Named so it is
  not silently reopened.
- **Auto-generation for gateways that fingerprint** (Vercel's fallback)
  — Fleet's client cannot omit a configured header on some requests;
  the config decides what is sent.
- **The GUI's headers row** — settings-ux U9's account card, as before.

## Verification

**C0 — the id's shape and resume stability:**
```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo test --lib route::
```
Expected: the determinism/distinctness/shape pins green; the seed's
prepare→build→wire pin green for a resumed session.

**C1 — injection safety:**
```sh
cargo test --lib route:: settings::
```
Expected: the control-character refusal pins green at the writer and at
the build, each naming the header.

**C2 — the request id capture:**
```sh
cargo test --lib api::
```
Expected: the 400-with-`x-request-id` pins green on both paths, the
absent-header and 200-non-capture pins green.

**C3 — the writer's warning:**
```sh
cargo test --lib settings::
```
Expected: the constant-value warning pin green, the placeholder pin
green, the save still landing.

**C5 — the gate, by exit code:**
```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: every command exits 0, read directly (KI-014).

## Linked artifacts

- `docs/features/routing.md` — the headers passage gains resume
  stability, the opaque derivation, the static-value trap, and the
  request-id capture; the providers-card bullet gains the request id
- `TECHNICAL-DOCUMENTATION.md` section "Environments and dependencies"
  ("Keys" bullet) — the seed derivation and the capture allowlist
- `FUNCTIONAL-SPECIFICATIONS.md` section "The commands as verbs" — the
  provider headers row gains resume stability and the request id

## Risks

- **The prepare signature grows a parameter** — five call sites plus
  children; the `None` default keeps every existing caller compiling
  unchanged, and the seed is advisory (an id nobody asked for behaves
  exactly as today).
- **A deterministic id across a shared home** — two fleet processes
  driving the *same* conversation derive the same id; that is the
  contract's point (one conversation, one bucket), not a collision.
- **Captured response headers are provider-controlled strings** — the
  allowlist bounds what is read, the Display bounds what is shown, and
  the value is length-capped before it enters any record.

## Dependencies

- `docs/plans/2026-11-02-provider-session-headers.md` — this plan
  builds on its landed field, placeholders, and two-gate validation.

## Notes

- The research sources, for the next reader: OpenCode's harnesses and
  Go docs (opencode.ai/v2/docs/console/harnesses, /go); the fork's
  migration discussion (deepseek-harness #5495 — the static-value
  measurement, the `sessionHeader` pattern, the lifecycle contract,
  the CRLF note); zai-org/feedback #840 (the original rejection);
  LiteLLM's request-headers and cache-aware-routing docs; vLLM
  Production Stack's router config (`sessionKey`); SGLang's
  session-aware radix cache; Anyscale's KV-token-aware routing post
  (the client-burden note); Vercel's session-affinity docs
  (`x-session-id`, the fingerprint fallback); Cloudflare AI Gateway's
  custom metadata; Helicone's header cheatsheet; Portkey's headers
  reference; Anthropic's `request-id`; OpenAI's Responses migration
  guide (the protocol-level statefulness that stays out of scope).
- The fork's resolution landed as a wrapper injecting from
  `options.sessionId` — the same shape this plan's seed rides: config
  names the header, the runtime supplies the per-conversation value.
  Fleet's field already names the header; this plan supplies the
  conversation-derived value.
- Alternatives considered: threading the session id through every
  walk signature (rejected — the churn the first plan measured for no
  granularity the gateways require, since preparation-per-conversation
  already holds in all three surfaces); post-construction header
  mutation on the Arc'd endpoints (rejected — shared mutable state the
  walk's locks do not cover); hashing with `DefaultHasher` (rejected —
  its keys are randomized per process, which would break determinism).
