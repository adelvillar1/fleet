# The router rules everything but the leader

Status: completed
Date: 2026-10-21

The owner read the router deep dive — judgment paid on ~72% of runs and
moving nothing, one model serving 409 of 409 runs under a pin, a reserve
that could only ever serve when pinned — and ruled, in four messages: no
provider's model is hardcoded anywhere, the router invokes lanes and
models by difficulty, a pin narrows the router to that provider's models,
and deepseek-flash is the failover when every routed model fails. Asked
how "by difficulty" squared with the 2026-10-16 orchestrator ruling, the
owner narrowed it: *the session that leads is set to hard; everything
else the router rules.* The plan is
`docs/plans/2026-10-21-router-rules-everything-else.md`, approved before
the build.

## What changed

- **The failover tail.** `route::with_tail` closes every walk order the
  router builds — judged, fallback, provider-pinned, pair-pinned, and the
  vision side-route — with the roster's deepseek entry and its default
  model, filtered by the same `allow_api_models` gate as every candidate,
  present once (a `routing.json` workload that places the reserve keeps
  the owner's position), and absent when the reserve is disabled. When it
  serves, `Router::row` ends the reason with `; served by the failover
  tail deepseek-flash`; a reserve chosen by a pin carries no clause. One
  edge found by an existing test and fixed before it shipped: an empty
  preference table decides its head walk *before* the tail is appended,
  or the reserve becomes the head of a walk nobody declared.
- **The leader keeps `hard`; children are the router's.** `select` is now
  `select_walk(.., Walk::Leader)`; `Walk::Judged` walks the judged
  workload's own chain with no orchestrator override. `subagents::run`
  prepares a child's route with `route::prepare_child` over the child's
  composed brief, under the parent's pin (which now rides the `Router`
  and is read through the parent's client), runs the child on its own
  routed client, and records the child's decision row in the same
  calibration stream. The rule has one boundary, chosen when a roles test
  showed it: a parent on a direct, unrouted client — every test double —
  or a run with no home hands the child the same client, because there is
  no router to rule it. Every production surface routes, so children
  route.
- **No nominal model.** `api::DEFAULT_MODEL` is `api::DEEPSEEK_MODEL`,
  referenced only by the roster's deepseek entry and the window table.
  `route::AUTO_MODEL` (`auto`) is what `AgentOptions::default()`, the TUI
  and the GUI launcher seed; a scheduled run's session takes the model of
  the route `run_job` prepared before it; `fleet run` already opened its
  session with the decision's model and still does. A `fleet chat` log
  opens before its first message exists, so it starts as the pinned
  pair's model or `auto` — the plan's C5 was amended mid-build to say so,
  per the plans README.
- **`--model` is a pin.** `route::resolve_model_pin`: `provider/model` as
  written (an unknown provider refused by name), a bare id to the one
  roster provider that serves it in the roster's own spelling, and a
  refusal listing every `provider/model` spelling when none or several
  do — `glm-5.3-flash` lives on both zai and ollama-cloud in the owner's
  config. `fleet run` and `fleet chat` resolve it before the route is
  judged; before this it changed nothing but the start record.
- **The docs.** CLAUDE.md's hard rule 9 and its DeepSeek state bullet,
  `docs/features/routing.md` ("Children", "The failover tail", "Pins and
  the nominal model"), `docs/features/subagents.md`, and the router's own
  comments on the reserve.

Also landed this session, under the same plan's C0 (committed at
`9c72bbb` before the plan existed, disclosed in the plan and ratified by
its approval): `deepseek-chat` → `deepseek-flash` everywhere in living
code and docs, and the context-window table grown from 11 to 36 rows —
ollama-cloud's 23 models from the provider's own `/api/show` declaration,
liquid's 131K, deepseek's 1M for the V4.1 flash generation the old name
now serves. The ollama-cloud roster entry was expanded to all 23 cloud
models as a `config.json` override through `fleet settings provider add`.

Tests: the tail in every order and the owner-placed reserve; the row
clause only when the tail served; leader vs child walks with a dead
judge and with a scripted sys1 verdict; `resolve_model_pin`'s five
resolutions; the CLI's `None` default. Gate: fmt, clippy -D warnings,
release build, `cargo test` — 1488 passed, 1 ignored. The gate's first
run failed on `No space left on device`: `target/debug` held 159 GiB of
accumulated test binaries (668k files); `cargo clean --profile dev`
freed it, and the release binary the face runs was untouched.

## What is left

- **KI-278:** a scheduled run that delegates still hands its child the
  parent's client — `improve_home` is `None` on purpose for cron and
  doubles as the child's route home. The fix is a route home distinct
  from the improve home.
- **C8, deferred and named:** `composed` is unreachable by judgment
  (sys1's `route` task has five labels; the lane is vendored, the label
  is upstream's), and effort-per-workload is its own plan — the research
  is recorded in this session's notes.
- The live smoke named in the plan's Verification — `fleet run --model
  ollama-cloud/glm-5.3-flash "say hi"`, and a run with every plan key
  unset serving on the named tail — spends plan tokens and is the
  owner's call.
