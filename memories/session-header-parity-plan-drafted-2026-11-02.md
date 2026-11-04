---
name: session-header-parity-plan-drafted-2026-11-02
description: "project 2026-11-02 — the session-header-parity plan (research on the gateway session-id landscape, five gaps in the first headers work) was approved and IMPLEMENTED whole, landed at origin/master a17e115 (plan completed, gate 1,935/0): resume-stable opaque UUID ids seeded per conversation, injection-safe headers, request-id capture, the writer's trap warning"
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

The owner asked (2026-11-02, after [[fleet-vs-opencode-missing-session-header]] landed) for deep research on session-id headers across LLM gateways, a gap analysis against Fleet's fresh implementation, and a plan — then said **"implement plan as designed"** and the whole thing landed on origin/master at **`a17e115`**, plan `docs/plans/2026-11-02-session-header-parity.md` status **completed**, all six criteria ticked, gate 1,935/0 (face harness 16/0 inside).

**The landscape in one line each** (sources cited in the plan's Notes): session ids serve *affinity* (OpenCode Zen prompt-cache routing; vLLM Production Stack hashes a client-chosen `sessionKey` header; SGLang radix cache; Vercel `x-session-id` with a fingerprint fallback; LiteLLM cache-aware deployment routing) and *grouping* (Helicone/Portkey/Cloudflare `cf-aig-metadata`). The converged lifecycle contract: **stable per conversation across turns, resume, compaction, retries; fresh for new conversations, forks, subagents**. The fork's own discussion (deepseek-harness #5495) **measured the static-value workaround as harmful** — one constant id collapses all conversations into one affinity bucket — and adopted config-names-header / runtime-supplies-per-conversation-id. Gateways validate presence, not shape, but UUID is the documented-safe choice; values must be header-safe (CRLF) and opaque.

**What landed:**
- `session_seed: Option<&str>` on `prepare`/`prepare_with_pin` (nine call sites seeded: run `--session`, chat's first task, the GUI's three bridge sites (driving id; the `btw` aside rides its conversation), the TUI's `current_session`; compact/cron/onboard-proof stay `None`; `prepare_child` untouched — children fresh per contract).
- `session_id_for(seed)`: an **opaque UUID-shaped (8-4-4-4-12) hash of the conversation's id** (two `calib::hash16` passes), fresh OS entropy (`RandomState`) when unseeded — and **idempotent on its own output** (`is_uuid_shaped` seeds ride verbatim). One resolved id per preparation, shared by every candidate's client, carried on the new `Prepared.session_id` → `AgentOptions.session_id` (a new field, ~30 literals patched) → `apply_pin`'s re-route re-seeds the SAME value, so the bucket survives a mid-run provider switch too. One id per conversation across providers is deliberate (opaque, vendors can't correlate).
- Injection safety: header **names** must be RFC-7230 token chars, **values** carry no control chars — at the writer (`validate_provider_entry`) and again at `Router::build` on the *env-substituted* values, so an env-sourced `\r\n` is caught.
- `ApiError.request_id`: a five-name response-header allowlist (`x-request-id`, `request-id`, `x-opencode-log-id`, `x-litellm-call-id`, `cf-ray`) captured on non-200 only, in both paths + `map_send_error`, displayed as `… — request id …`, riding `TurnFailed` via the message.
- The writer's `session_constant_warning` (non-blocking eprintln in `set_provider`) names the static-value trap.
- Docs: routing.md's headers passage carries the lifecycle contract, the trap, and the capture; TECH Keys bullet + FS verbs row match.

**Build traps hit:** `u8::is_control()` does not exist — it is `b.is_ascii_control()`; `RandomState::build_hasher()` needs `use std::hash::{BuildHasher, Hasher}` in scope; adding a field to `AgentOptions` cascaded to ~30 struct literals (a guarded python insert-after-`goal: None,` handled them, but the script's regex must match `AgentOptions \{$` at END of line, not as the whole line); the chat edit broke braces by replacing a single-line `.await` chain with a multi-line call and dropping `.await {`. Pins' assertions must quote the error message's OWN wording (`header-name token`, not "header name"). plan-gate's stable sub-0.3 blind spot (docs/gate criteria) and arch-gate's one-run DRIFT that didn't reproduce were both waived with dispositions.

**The session's later arc:** this plan's landing (`a17e115`) was followed the
same day by [[session-parity-remainders-plan-drafted-2026-11-02]] (5/6, then
6/6 at `d816704`), the usage plan ([[usage-plan-amended-trajectory-shape-2026-11-02]]),
and the plugin host ([[creator-mode-plugin-host-exploration-2026-11-02]]) —
all landed and completed, with the 1.99.1 cleanup folded in between. See
[[session-recap-2026-11-02-two-plans-landed]].
