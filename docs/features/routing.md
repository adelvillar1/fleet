# Routing (slice 5)

Multi-provider routing: a roster of OpenAI-compatible gateways in the
owner's declared chain, sys1-judged per-task selection, failover across
failure classes, and a calibration decision row per run.

## The roster

Six candidates in the owner's declared order (directive 2026-10-13), stored
as compiled-in defaults in `src/route.rs` and overridable per entry from
`<FLEET_HOME>/config.json` (merge, not replace). Every endpoint, key
env-var *name*, and model id is sourced from the fork's
`agnostic-router-kit/roster.json`; no key value is ever stored.

| # | id | base_url | key env var | model |
|---|---|---|---|---|
| 1 | `router-kit` (HEAD) | `http://127.0.0.1:8300/v1` | `AGNOSTIC_ROUTER_TOKEN` (optional) | `auto` |
| 2 | `zai-coding-plan` | `https://api.z.ai/api/coding/paas/v4` | `ZAI_CODING_API_KEY` | `GLM-5.3-Flash` |
| 3 | `xiaomi-mimo` | `https://token-plan-sgp.xiaomimimo.com/v1` | `XIAOMI_MIMO_API_KEY` | `mimo-v2.6-flash` |
| 4 | `stepfun` | `https://api.stepfun.ai/step_plan/v1` | `STEPFUN_API_KEY` | `step-5-preview` |
| 5 | `token-plan` | `https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` | `TOKEN_PLAN_API_KEY` | `qwen3.8-flash` |
| 6 | `deepseek` (reserve) | `https://api.deepseek.com` | `DEEPSEEK_API_KEY` | `deepseek-chat` |

The router kit is the primary route, not the escape hatch; DeepSeek is a
working account held in reserve. The chain order is compiled in and starts
declared — slice 6 grades outcomes and reorders candidates over time.

## Selection

Before a run, `route::select` asks `sys1_classify` (task `route`) over the
slice-4 MCP client for a workload verdict (`quick`, `standard_code`, `hard`,
`prose`, `deep_context`) and maps it to a candidate. The map honors "the kit
is primary": routine work routes through the kit, and the specialized plans
are the judged targets for hard/prose/deep_context. An unusable, absent, or
failed verdict — or a mapped candidate with no key — falls back to the
declared HEAD and says so. Routing never blocks a run: sys1 that cannot
launch or cannot judge is a fallback, not an error. A configured `pin`
bypasses judgment entirely.

## Failover

`Router::chat_stream` walks the declared chain from the selected candidate.
401/402/403/408/429/5xx, network errors, timeouts, and decode failures all
advance to the next usable candidate (400/404 do not — every candidate would
fail them). A candidate that starts streaming is committed: a mid-stream
failure fails the run typed, with no silent restart. Each attempt and the
final choice are recorded.

## The decision row

Every routing decision appends one `devdec`-vocabulary JSONL row to
`<FLEET_HOME>/logs/events-<UTC date>.jsonl` — `op: "route"`, the selected
and served provider, `how` (judged/fallback/pinned), the workload and
verdict, every attempt with its failure class, and a content hash of the
task (never the text). The append is serialized process-wide and
best-effort: a full disk is logged, never fatal. Slice 6 (`src/calib.rs`)
formalizes this writer and aligns the log root with dev-decisions so the
`devdec` CLI reads these rows beside its own.

## Surfaces

- `fleet providers` — the roster as cards in chain order, per-provider state
  (configured/unconfigured, key present/absent — never the value), the head
  marked. No network, no spawn.
- The TUI status bar shows the active provider for the current/last run and
  how it was chosen.
- A headless run prints `route: <provider-id> (<judged|fallback|pinned>)` at
  start.

## Config

`<FLEET_HOME>/config.json` overrides the defaults per entry:

```json
{
  "pin": null,
  "providers": [
    { "id": "router-kit", "enabled": true },
    { "id": "zai-coding-plan", "model": "GLM-5.3-Pro" }
  ]
}
```

An invalid config file fails startup with a typed, named error — never
silently ignored. A new provider id must be fully specified (base_url,
key_env, model) to be added; a partial one is a named error.
