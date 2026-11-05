---
name: fleet-roster-migrated-from-router-kit
description: "Fleet's roster was migrated from the zcode-router-kit 2026-10-16 — all keys sourced into ~/.fleet/.env, stepfun pin removed (KI-047 closed), sys1 judgment keys present; stepfun heads every workload by the kit's own table (mimo is fallback-only); live-proven with `walk: stepfun served`. The migration then enabled the owner's ruling that Fleet reads no other harness's config at runtime"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The roster migration (2026-10-16, done at the owner's request) — Fleet needs no provider configuration and the router runs judged walks:

- **Keys**: every roster key lives in `~/.fleet/.env`, sourced from the kit's `~/.zcode/router/.env` plus its `localToken` (as `AGNOSTIC_ROUTER_TOKEN` — KI-047 closed). Includes `ZAI_CODING_API_KEY`, `XIAOMI_MIMO_API_KEY`, `TOKEN_PLAN_API_KEY`, `DEEPSEEK_API_KEY`, `STEPFUN_API_KEY`, and sys1's `FASTINO_API_KEY`/`TYPESAFE_API_KEY`, so route selection is a real judgment, not the fail-open default. Pre-migration backup: `~/.fleet/.env.bak-migration`; file is chmod 600. Liquid has no key anywhere in the kit — honestly unkeyed.
- **Pin removed** via `fleet settings set pin ""` (empty is no pin, through the writer). `approvals` stays `yes-guarded` — which the 2026-10-16 fix made delegation-safe. **Reaffirmed later the same day (2026-10-16), after a transient deepseek pin briefly appeared in `routing.json` and the TUI bar rendered "pinned to deepseek": the owner keeps NO pin — "deepseek is the provider of last resort and that logic is in the router itself." Verified live: `routing.json` carries only `gates.min_confidence: 0.72`, `fleet providers` shows no pin. The pin seam stays as the deliberate reserve-use override (the KI-214 payg guard: deepseek serves only on an owner pin), and with no pin the TUI routing label reads `route auto` — see [[fleet-tui-status-bar-routing-label]].**
- **The owner questioned whether mimo was ever preferred in the kit — their memory is correct**: all three kit sources (`~/.zcode/router/config.json`, its `.bak-kit`, `~/Projects/zcode-router-kit/roster.json`) agree that **stepfun heads every workload**; mimo is only a mid-chain fallback. The kit's internal `omniModel`/`wideModel` head `mimo-v2.6-pro`, but identically in live and backup — kit shipping shape, not drift. Fleet's compiled-in workload port matches the kit's live table exactly (including the stepfun dedupe), so no `routing.json` override was needed.
- **Live proof**: smoke run after the change → `walk: stepfun served`, 1 attempt, one fresh sys1 judgment, route row on the stream.
- **Deliberately not migrated**: the kit's localhost converters `codebuddy` (:8787) and `opencodex` (:10100) are NOT in Fleet's roster — owner can add via `fleet settings provider.add:`. The `web` leg fails open until `TINYFISH_API_KEY` is set.
- Note: Fleet walks each workload's chain directly, so the old pin only restated the table. `~/.local/bin/fleet` symlinks the release binary ([[fleet-dogfooding-viability-verdict]]), and CLAUDE.local.md's quirks section now describes the un-pinned state; kit config facts are KI-116's territory (actual runtime home is `~/.zcode/router`, not the documented `~/.agnostic-router-kit`).
- **The ruling this migration enabled (2026-10-16):** with every key — `DEEPSEEK_API_KEY` included — now in `~/.fleet/.env`, the owner ruled that Fleet's router never reads a config file from a different harness. The DeepSeek Harness's `~/.dsh/.credentials.yaml` `refs` leg was removed outright (helpers deleted, not unwired; a negative regression test pins it). The kit's and dsh's files are one-time copy sources, never runtime reads. The boards' read-only API seam this note once left "under a separate ruling" is now ruled too: the kit entry is gone from the roster entirely and the router board reads Fleet's own state — see [[fleet-routes-through-its-own-router]].
