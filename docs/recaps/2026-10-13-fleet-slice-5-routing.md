# Recap — slice 5: multi-provider routing

Date: 2026-10-13
Slice: 5 of the everyday-harness plan (`docs/plans/2026-10-13-fleet-everyday-harness.md`)

## What landed

A native routing layer: a roster of OpenAI-compatible gateways in the
owner's declared chain, sys1-judged per-task selection, failover across
failure classes, and a calibration decision row per run.

- **`src/route.rs`** (new): the roster (`Candidate`/`Roster`) with
  compiled-in defaults matching the declared chain and a per-entry JSON
  merge from `<FLEET_HOME>/config.json`; sys1-judged selection (`select`,
  spawning the lane and reading the `route` task's workload verdict);
  failover (`Router::chat_stream`, the failure-class table, the committed
  mid-stream rule); the `devdec`-vocabulary `DecisionLog`; and the
  `fleet providers` card render.
- **`src/api.rs`**: `Client` gained an optional `router`; `Client::routed`
  and `Client::route_decision`. `chat_stream` delegates to the router when
  routed and is byte-for-byte the single-endpoint path otherwise (the plain
  body split into `chat_stream_plain` so the router can call it per
  candidate without an infinitely-sized recursive future).
- **`src/agent.rs`**: `UiEvent::Routed` emitted once at the start of a
  routed run; an unrouted client emits nothing, so the headless and unrouted
  paths are unchanged.
- **`src/cli.rs` / `src/main.rs`**: `fleet providers`; `run`/`chat` prepare
  the route (once per session) and record the decision; `doctor` unchanged.
- **`src/tui/*`**: the status bar shows the active provider and how it was
  chosen; the agent thread prepares the route on the first run and reuses
  the committed router.
- **`src/session.rs`**: `iso_utc_stamp` exposed `pub(crate)` for the row's
  timestamp (one home for the calendar math).

## The chain and its sources

Router kit (HEAD, `http://127.0.0.1:8300/v1`, model `auto`) → Z.ai Coding
Plan (`GLM-5.3-Flash`) → Xiaomi MiMo (`mimo-v2.6-flash`) → StepFun
(`step-5-preview`) → Qwen Token plan (`qwen3.8-flash`) → DeepSeek direct
(`deepseek-chat`, reserve). Every endpoint, key env-var name, and model is
from the fork's `agnostic-router-kit/roster.json`; the router kit's token is
referenced by name only (`AGNOSTIC_ROUTER_TOKEN`), never stored.

## Verification (local gate, exit codes read — never piped)

- `cargo build --all-targets` → exit 0, **0 warnings**.
- `cargo test` → **171 unit + 46 integration passed, 0 failed, 1 ignored**
  (150 + 43 pre-existing, all still green; +21 route unit, +3 integration).
- Integration: `a_failed_head_fails_over_to_the_next_candidate` (head 402 →
  second serves, the row names both attempts),
  `sys1_judges_the_route_through_the_injected_spec` (the fake lane's `hard`
  verdict routes to zai), and `the_real_sys1_lane_exposes_the_classify_schema`
  (the vendored sys1 booted and its `sys1_classify` schema matched).
- Live e2e against the real kit: with `AGNOSTIC_ROUTER_TOKEN` set the run
  served on `router-kit (fallback)`; without it the head 401'd and the walk
  served on `zai-coding-plan`, both attempts in the row. One decision row
  per run landed under `<FLEET_HOME>/logs/events-<UTC date>.jsonl`.

## Ledger

KI-044 (per-session sys1 spawn cost when its providers are unconfigured),
KI-045 (run/chat no longer pre-flight a credential), KI-046 (the
workload→candidate map is the one tunable), KI-047 (the kit's `/v1` is
token-gated; modeled key-optional, token by name only).

## Next

Slice 6 (`src/calib.rs`) formalizes the decision writer into the
calibration stream, aligns the log root with dev-decisions, and grades route
outcomes so the chain reorders on evidence.
