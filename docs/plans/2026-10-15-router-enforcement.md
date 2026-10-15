# Slice 21 — the router as intended: the kit's pattern, internal and plugin-shaped (2026-10-15)

status: in-progress (the routing core landed 2026-10-15: the ported table, the plugin config, the order-walk with the gates, the pin semantics, and the honest surface are committed and gate-green; the blend1 lane-serving path — AC4's client-boundary half and AC9's composed leg — is the remaining increment)

## The architecture this slice lands (the owner's rulings, 2026-10-15)

1. **Fleet is a self-contained harness on the deepseek-harness philosophy:
   everything is a plugin.** The router's parts — the provider roster and
   the preference of execution — are plugin config inside fleet, loaded
   from `<FLEET_HOME>` with compiled-in defaults, validated on load.
2. **The routing pattern is internal, not a separate component.** The
   pattern the zcode-router-kit (the running zcode-only variant of the
   agnostic-router-kit, `~/.zcode/router`) executes is ported natively
   into fleet. The kit is not consulted at run time.
3. **blend1 is a provider — the grounding one.** The composed ask
   (Liquid's hosted lfm 24B synthesizer over the specialized legs sys1 /
   sdm1 / sem1 / gen1) exists to give better grounded responses from
   smaller models. The router targets it when the workload makes sense
   — the `composed` shape — through the vendored lane as a plugin.
4. **The plan-quota-first, deepseek-in-reserve directive becomes
   structural.** The kit's `allowPayg: false` is ported as policy:
   deepseek is declared in the roster (payg) and targeted by no
   workload, so it cannot serve unless the owner pins it explicitly.
   The original complaint — deepseek serving as the silent last
   option — is closed by construction, not by a warning.

## The pattern being ported (verified against the live kit, 2026-10-15)

The live kit's config (`~/.zcode/router/config.json` → `routing`) is the
source for the compiled-in defaults; the repo roster
(`~/Projects/zcode-router-kit/roster.json`) is the upstream base. The
live table is the owner's tuned state and wins:

- **Per-workload preference at (provider, model) granularity**, each
  workload an ordered candidate list — the target first, then the
  fallbacks in preference order:
  - `quick` → stepfun/step-3.7-flash, xiaomi-mimo/mimo-v2.6-flash, zai/GLM-5.3-Flash
  - `standard_code` → stepfun/step-3.7-flash, zai/GLM-5.3-Flash, xiaomi-mimo/mimo-v2.6-flash
  - `hard` → stepfun/step-5-preview, xiaomi-mimo/mimo-v2.6-pro, zai/GLM-5.3-Flash
  - `prose` → stepfun/step-3.7-flash, zai/GLM-5.3-Flash, token-plan/qwen3.8-flash
  - `deep_context` → stepfun/step-5-preview, zai/GLM-5.3-Flash, xiaomi-mimo/mimo-v2.6-pro
  - `composed` → blend1/blend1, stepfun/step-3.7-flash, zai/GLM-5.3-Flash, xiaomi-mimo/mimo-v2.6-flash
  - (`quick`'s live list carries a duplicated stepfun entry — deduped in
    the port, noted here so the divergence from the source is explicit.)
- **The gates**: `minConfidence 0.6` — a judgment below it routes to
  `defaultWorkload` (`standard_code`) rather than to the guessed shape;
  `wideChars 1_000_000` for the wide/omni picks (named, not built — see
  Out of scope).
- **The roster**: each provider carries id, base URL, key env-var NAME
  (never a value), its model list, `billing` (plan | payg), and
  `enabled` — the kit's roster.json shape, which fleet's compiled-in
  default already mirrors in part.

## What changes in the crate

- **The walk becomes the workload's own list.** Today `select` maps a
  workload to ONE candidate and the failover walks the global chain
  order. The port: the workload table names the ordered
  (provider, model) candidates, the first usable one is the target, and
  a failure walks THAT list — per-workload preference, no global chain.
  An exhausted list is a named failure carrying the last error, never a
  silent fall-through to a candidate no workload asked for.
- **The roster gains per-provider models.** A `Candidate` carries its
  model list; the preference picks the model. `RouteDecision` carries
  the (provider, model) it will serve (already landed in-tree).
- **blend1 is a lane-kind candidate.** Its surface is the vendored
  lane's `/v1/ask` (services/blend1/service/main.py, port 8420,
  `BLEND1_BEARER_TOKEN`) and its MCP tool — not OpenAI
  chat-completions. The candidate abstraction gains a `kind`
  (`http` | `lane`); a lane candidate is driven through fleet's own MCP
  client against the vendored lane (self-contained: no separate
  component, no checkout), with the ask payload mapped to and from the
  chat shape at the client boundary.
- **The plugin config.** `<FLEET_HOME>/providers.json` (the roster) and
  `<FLEET_HOME>/routing.json` (the workload table + the gates), both
  optional, both validated with typed named errors on load (the
  existing `ConfigError` discipline — a misspelled field fails loud,
  never silently ignored), both merging over the compiled-in defaults
  per entry. The existing `config.json` pin stays as the owner's
  explicit override on top.
- **The gates land in `select`.** A sys1 verdict below
  `minConfidence` routes to `defaultWorkload`'s list with the reason
  naming the gate; the decision row carries the confidence.

## Acceptance criteria

1. **The compiled-in default is the kit's live table** — the six
   workloads above, at (provider, model) granularity, deduped, with the
   divergence from the source named in this plan.
2. **The plugin config loads and validates.** `providers.json` and
   `routing.json` under FLEET_HOME override the defaults per entry; an
   unknown field or a bad shape is a typed named error that fails
   startup loud; a key env-var NAME may appear in config, a value never.
3. **The walk is the workload's list.** A judged workload targets its
   first usable candidate and fails over through its own list in
   order; an exhausted list fails the run by name with the last error;
   no candidate outside the workload's list is ever reached by the
   walk. Pinned by unit tests over a scripted roster.
4. **blend1 serves the composed workload.** A `composed` judgment
   drives the vendored blend1 lane through the MCP client (kind `lane`),
   the ask payload mapped at the client boundary, the answer returned
   in the chat shape; a lane that fails open is a named refusal that
   advances to the workload's next candidate.
5. **The gates.** A verdict below `minConfidence` routes to
   `defaultWorkload` with the reason naming the gate and the
   confidence; the decision row carries both.
6. **deepseek cannot serve unpinned.** No default workload targets it;
   a pin is the only path to it, and `fleet providers` shows it as
   declared-payg-untargeted. Pinned by a test that asserts no default
   workload's list contains it.
7. **The pin is a provider, and the router still decides the model.**
   The owner's pin names a PROVIDER (`stepfun`, `zai-coding-plan`,
   `token-plan`, `xiaomi-mimo`, `blend1`); the router then chooses the
   model from that provider's available models per the judged workload —
   pinned to stepfun, a quick-shaped task runs step-3.7-flash and a
   hard-shaped one runs step-5-preview, because those are stepfun's
   models for those shapes. **The pin's purpose is quota**: it is how
   the owner points the harness at a vendor carrying a large unused
   allocation (the z.ai coding plan, the stepfun plan) and spends that
   down before the pay-as-you-go tail. A pin naming a `provider/model`
   pair pins both exactly; a pin naming an unknown provider is a named
   refusal, never a silent default. This is what makes "pin to
   step-5-preview" and "pin to stepfun" different requests, both
   answerable.
8. **The honest surface** (already partially landed in-tree,
   uncommitted): the decision carries provider+model; `fleet run`
   prepares the route before creating the session, so the first line
   and the session log's `start` record name the routed provider and
   model, never the CLI default; the walk line reports the attempts in
   order with the no-key skips naming the env var each wanted.
9. **Verified with real runs**: a quick-shaped task lands on
   stepfun/step-3.7-flash; a hard-shaped task on stepfun/step-5-preview;
   a composed-shaped task on blend1 through the lane; a low-confidence
   judgment routes to standard_code's list; a provider pin routes every
   shape through that provider's own models — each with the honest
   attribution in the output and the session log, and the route rows in
   the stream agreeing.
10. **Gate green** (`cargo test`, `cargo build --release`; rustfmt and
    clippy are absent on this machine and are not run), this plan's
    status updated in the same turn, recap landed, committed with
    explicit paths.

## Out of scope

- `omniModel` / `wideModel` / `mixture` / the workflow registry: named
  in the plugin config's shape, not built — the workload table is the
  preference of execution this slice ports.
- The kit's own judge (typesafe jev / fastino cascade): fleet's judge
  stays sys1, as slice 5 settled. The gates are ported; the judge is
  not swapped.
- The external kit as a run-time component: declared in the roster,
  targeted by no default workload, re-enableable by the owner's config
  and nothing else.
- Automatic edge/capability inference and slice 20's wiring — both
  unchanged and still pending.
