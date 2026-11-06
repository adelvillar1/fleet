---
name: fleet-vs-opencode-missing-session-header
description: "2026-11-02 — fleet shared zcode #840's wire exposure (no x-opencode-session, no custom-header path); the provider-session-headers plan was drafted, approved, IMPLEMENTED and LANDED whole at origin/master 01f6760 (plan completed, gate 1,930/0)"
type: project
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

The owner asked (2026-11-02) whether fleet suffers zcode's issue zai-org/feedback#840: opencode's gateway (`https://opencode.ai/zen/go/v1`) rejects session-gated models (free tier, gpt-6-luna, qwen3.8-max) with HTTP 400 `MissingSessionID` when the client never sends `x-opencode-session`.

**Fleet's verdict, four legs (the check that started it):**
- **Same wire exposure** (now CLOSED, see below): both chat paths set only `Authorization` (+`Content-Type` when streaming) — `src/api.rs:614` (chat), `src/api.rs:759` (stream); no per-provider custom-headers field existed.
- **NOT the diagnostic failure** that made zcode's bug hard: `api_error` extracts `error.message` or keeps a 300-char body excerpt (`src/api.rs:916-928`), `AgentError::Api` displays it verbatim, and `Record::TurnFailed { stage: "model" }` lands in the session log before the error propagates (`src/agent.rs:2609`).
- **Fleet-specific wrinkle (NARROWED 2026-11-05, KI-451)**: a 400 → failure class `client-error`, **not failover-eligible** (`src/route.rs`), on the premise that every candidate would fail it identically — the owner declined a blanket change, because burning the chain on a genuine bad-request bug is the cost. That premise is false for two body classes, and both now fail over: `names_model_unavailable` (the model is absent on this provider) and `names_account_state` (this candidate's own plan/subscription/account is exhausted or unpaid). A genuinely malformed request still ends the walk, and the pin `an_account_state_400_fails_over_instead_of_ending_the_walk` drives both halves — including a `plan` noun in a validation message, which must NOT qualify.
- **Same protocol gap**: `/chat/completions` hardcoded, no `/responses` support, so the Luna models stay unreachable even with the header.

**LANDED 2026-11-02 (owner said "implement as designed")** — plan `docs/plans/2026-11-02-provider-session-headers.md`, status **completed**, all seven criteria ticked with evidence, on origin/master at **`01f6760`** (commits `3d246a1` the field+client+gates, `16c30e3` the card, `3c1d32b` docs+plan):
- `Candidate.headers`/`ProviderEntry.headers` are `BTreeMap<String,String>` (serde-default — every existing config parses byte-identical); merge arm beside base_url/key_env.
- Values compose `{session}` (stable random id per client instance, generated in `Router::build`; distinct across clients, stable within one walk) and `{env:NAME}` (substituted in `resolve()` from the dotenv chain). A reserved name (authorization, content-type, host, content-length — case-insensitive), an empty name, or a malformed placeholder is refused TWICE: at the settings writer (inside `validate_provider_entry`, so the CLI and the slice-12 proposal gate both catch it) and at `Router::build` for hand-edited files.
- **The one amendment (recorded in the plan's Notes BEFORE the landing commit)**: an unresolved `{env:NAME}` does NOT fail the router build — it rides `Endpoint::Http(Result<Client, String>)` (the lane's own `Result` shape) and fails the walk at that provider's first attempt, so one misconfigured header cannot brick walks that never reach it. General lesson: a build failure on ONE candidate's config error must not kill the whole walk — mirror the lane precedent.
- `Client::with_headers(base, key, pairs)`; both paths loop `.set()` after Authorization.
- Masking is load-bearing: values can carry env-sourced secrets, so every surface shows NAMES + count only (pinned at the card); the GUI log's hardcoded secret sweep (server.rs:4557) can't know owner-chosen names.
- The GUI face got nothing — the headers row rides settings-ux U9's account card.

**Process lessons from the landing:** plan-gate's two sub-0.3 MISSING flags (C5 docs, C6 gate) survived three runs + verification blocks + approach steps — waived with dispositions at approval rather than reworked (judge floor noise). Mid-work, origin/master moved (the parallel session's archify merge + `67671e9` clearing the shared memories/ recall counters); resolution: **stash the stale counter bumps named** (never re-apply — the deltas are relative to a pre-clear base), merge, re-run the test leg on the merged tree, push `HEAD:master`. The stash `stale recall-count bumps, superseded by 67671e9's clear` is left in place deliberately. See [[two-sessions-one-repo-master-worktree]].

**The sequel, also LANDED**: deep research against the gateway landscape found five gaps in this implementation (resume instability, machine-fact id shape, no CRLF rule, dropped response request-ids, the permitted static-value trap) — the parity plan was approved and implemented whole the same day; see [[session-header-parity-plan-drafted-2026-11-02]]. The third chapter — the Responses protocol as a stateless wire dialect, `{trace}`, and the face's headers row waiting on settings-ux U9 — is [[session-parity-remainders-plan-drafted-2026-11-02]], landed at `5c0c5ab`.
