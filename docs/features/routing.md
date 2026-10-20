# Routing (slice 5)

Multi-provider routing: a roster of OpenAI-compatible gateways in the
owner's declared chain, sys1-judged per-task selection, failover across
failure classes, and a calibration decision row per run.

## The roster

Seven candidates in the owner's declared order, stored as compiled-in
defaults in `src/route.rs` and overridable per entry from
`<FLEET_HOME>/config.json` (merge, not replace). The endpoints, key
env-var *names*, and model ids were ported from the fork's
`agnostic-router-kit/roster.json` when the precursor's decision core was
absorbed; the roster is Fleet's own now, and no key value is ever stored.

| # | id | base_url | key env var | model |
|---|---|---|---|---|
| 1 | `zai-coding-plan` (HEAD) | `https://api.z.ai/api/coding/paas/v4` | `ZAI_CODING_API_KEY` | `GLM-5.3-Flash` |
| 2 | `xiaomi-mimo` | `https://token-plan-sgp.xiaomimimo.com/v1` | `XIAOMI_MIMO_API_KEY` | `mimo-v2.6-flash` |
| 3 | `stepfun` | `https://api.stepfun.ai/step_plan/v1` | `STEPFUN_API_KEY` | `step-5-preview` |
| 4 | `token-plan` | `https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` | `TOKEN_PLAN_API_KEY` | `qwen3.8-flash` |
| 5 | `deepseek` (reserve) | `https://api.deepseek.com` | `DEEPSEEK_API_KEY` | `deepseek-chat` |
| 6 | `liquid` | `https://api.liquid.ai/openai/v1` | `LIQUID_API_KEY` | `lfm2.5-24b-a2b` |
| 7 | `blend1` (lane) | `http://127.0.0.1:8420` | `BLEND1_BEARER_TOKEN` (optional) | `blend1` |

**Fleet's router is this module.** The owner's 2026-10-16 ruling is that
Fleet is a self-contained app with a dedicated router it owns: `select`
asks sys1 for a workload verdict, the workload preference table picks the
provider, and failover walks the declared chain. There is no router
process behind it, no port, and no operator API. The
`agnostic-router-kit` entry that once sat at the head of this roster is
gone — the kit is the precursor this crate supersedes, a separate process
Fleet would have had to reach over loopback for it to re-route, and the
precursor is not Fleet's router. Every candidate above is dialed directly
by `Router::chat_stream`. The port 8300 on this machine belongs to the
zcode harness's own router, which is not Fleet's business.

The chain order is compiled in and starts declared — slice 6 grades
outcomes and demotes a candidate with three consecutive failed attempts
to the end of the session's walk, so the order becomes *calibrated*
without ever overriding sys1's judgment.

## Selection

Before a run, `route::select` asks `sys1_classify` (task `route`) over the
slice-4 MCP client for a workload verdict (`quick`, `standard_code`, `hard`,
`prose`, `deep_context`) and maps it to a candidate. Every workload's own
candidate list is the walk, and stepfun heads five of the six in the
compiled-in table. An unusable, absent, or
failed verdict — or a mapped candidate with no key — falls back to the
declared HEAD and says so. Routing never blocks a run: sys1 that cannot
launch or cannot judge is a fallback, not an error. A configured `pin`
bypasses judgment entirely.

**The orchestrator walks `hard`.** The first model a run loads is the
planner — it assigns the work and supervises it — so the walk leaves the
judged workload at one point: the chain is `ORCHESTRATOR_WORKLOAD`'s
(`hard`), whatever the task was judged, per the owner's 2026-10-16
ruling. The judgment still runs and the decision row still records its
label — `fleet judge` grades the judgment, and a relabeled row would
corrupt that grading — while the reason names both halves when they
diverge ("sys1 judged `quick`; the orchestrator walks `hard`'s chain").
An owner override of `hard`'s candidates in `routing.json` governs the
walk, which is what "whatever is assigned to the hard category" means.
The route-decision cache's key names the walked workload, so a cache
written under an older rule serves nothing: the key format changed, the
cache invalidates whole, and the first run re-judges. Delegation
children inherit the parent's model (CLAUDE.md hard rule 9), so the
strengthened head is the whole run's.

## Failover

`Router::chat_stream` walks the declared chain from the selected candidate.
401/402/403/408/429/5xx, network errors, timeouts, and decode failures all
advance to the next usable candidate (400/404 do not — every candidate would
fail them). A candidate that starts streaming is committed: a mid-stream
failure fails the run typed, with no silent restart. Each attempt and the
final choice are recorded.

There is no per-candidate budget — each attempt inherits the client's single
overall timeout (600 s). The worst case is therefore bounded but large:
every candidate hanging to timeout costs (chain length − start) × 600 s,
about an hour for a full chain. In practice a dead candidate fails in
milliseconds (connection refused); the hour case needs every candidate to
accept a connection and then stall.

## The decision row

Every routing decision appends one `devdec`-vocabulary JSONL row to the
calibration stream — `op: "route"`, the selected and served provider, `how`
(judged/fallback/pinned), the workload and verdict, every attempt with its
failure class, and a content hash of the task (never the text). The
append is a single `write_all` under O_APPEND, so two fleet processes
cannot interleave a row, and crosses a bounded non-blocking queue drained
by one writer thread: a full disk or a full queue is logged and counted,
never fatal.

Slice 6 (`src/calib.rs`) owns that writer, and the rows now land in
dev-decisions' own log root — `<user home>/.local/share/dev-decisions/logs/<YYYY>/<MM>/<DD>/events.jsonl`
— so the external `devdec` CLI reads them beside its own. Full detail:
[docs/features/calibration.md](calibration.md).

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
    { "id": "zai-coding-plan", "model": "GLM-5.3-Pro" },
    { "id": "xiaomi-mimo", "enabled": false }
  ]
}
```

An invalid config file fails startup with a typed, named error — never
silently ignored. Unknown fields are rejected the same way: a misspelled
override (`keyenv`) fails the load rather than vanishing. A new provider id
must be fully specified (base_url, key_env, model) to be added; a partial one
is a named error.

The gates live in `<FLEET_HOME>/routing.json` and are all optional:
`gates.min_confidence`, `gates.default_workload`, and
`gates.allow_api_models`. The last is the spend guard: `false` keeps every
`Http` candidate out of every order the router builds — the judged chain,
the fallback, the head walk, and a pin. Vendored lanes still serve. The
GUI's Settings page exposes it as a switch, the thresholds as a select and
an input, and the tiers as a target select plus ordered fallback chips; the
`resolved now` column is the first candidate the walk would actually serve
(the gate applied, the provider enabled, its key present). The page writes
through the same `settings.rs` writer the CLI uses.

An empty `"pin": ""` reads as no pin: the loader trims it away, so the GUI's
"auto" and a hand-written empty value mean the same thing.
