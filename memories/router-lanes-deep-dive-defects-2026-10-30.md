---
name: router-lanes-deep-dive-defects-2026-10-30
description: "Deep dive 2026-10-30 over the router, dev-decisions and the six lanes found three verified live defects and one systemic key-universe split — none on the KI ledger"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Deep dive 2026-10-30 (read-only; nothing fixed). Router = model/provider routing in `src/route.rs` (sys1 judges a workload from a hash-keyed verdict cache, then the workload's preference list is walked with failover); `tool_router.rs` is deferred-tool activation (sys1 class + sem1 vectors, FLOOR 0.68). Lanes are six child processes over stdio MCP (`src/lanes.rs:72`), not five — `browser` (render) sits between gen1 and blend1. `dev-decisions` is three different things: an external repo, a blend1 per-project corpus at `~/.local/share/blend1/vectors/dev-decisions/embeddinggemma-2/` (142 items, 12 docs, 768d, ~590 KiB), and the `corpus-refresh-dev-decisions` cron job.

**Three defects verified by reading the code, none on the ledger (KI tail was 374):**

1. **Vision is dead.** `src/agent.rs:4575` builds `Client::routed(...)` then calls `client.chat(...)`, but `Client::chat` (`src/api.rs:603`) has no router branch — only `chat_stream` does (`:628`). `Client::routed` sets `base_url: String::new()` (`src/api.rs:575`), so `chat` POSTs to the relative URL `/chat/completions` with an empty Bearer key and always errors. The guard above it (`resolved.filter(...)`) correctly refuses a routerless run, so the failing path looks healthy. `vision_roster` (`src/agent.rs:1100`) and the comment on `Client::router()` both show the design intended the roster to feed a real router walk. Test `the_vision_workload_has_its_own_chain` (`:9487`) only tests `workload_decision`, never dispatch.

2. **`tool_route` is not a sys1 task.** `src/agent.rs:4059` calls `sys1_classify` with `task: "tool_route"`; `services/sys1/src/sys1/tasks.py` registers `_ROUTE` (id `route`) and `grep tool_route services/sys1/` returns nothing. `_resolve_task` raises `KeyError` → RPC error → `parse_capability` yields `None`, so the sys1 half of deferred-tool search is silently dead and sem1 ranks without a class.

3. **`measure` is starved while blend1 works.** Live `fleet lanes`: sdm1 reports BOTH backends "not configured — set TABPFN_API_KEY", yet blend1's doctor says "tabular — prior labs api ok" in the same run. Cause: `TABPFN_API_KEY` lives in `~/.config/sdm1/env`; blend1's `env.py:60` reads that file into its process env, but the sdm1 lane child only sees what fleet allowlists (process env + `~/.fleet/.env` + vault), where it is absent. So `fleet measure` fails while blend1's tabular leg succeeds.

**Systemic: two key universes.** `~/.fleet/.env` holds only FASTINO/TYPESAFE/DEEPSEEK; the lanes' private files hold the rest — `~/.config/dev-decisions/env` (DREX/LIQUID/…), `~/.config/sdm1/env` (TABPFN), `~/.config/gen1/env` (DASHSCOPE/STEPFUN), `~/.config/blend1/env` (TINYFISH/SCALEDOWN). Only blend1 reads its siblings' files, so *the same sys1 code has different providers depending on who imports it* — fleet's sys1 child gets glide/jev, blend1's embedded sys1 also gets liquid/drex. Any lane whose health depends on a key only in a private file reports starved.

**Also live:** `corpus-refresh-dev-decisions` failed 2026-10-30T00:50Z (runs 7, ok 6) — the only one of 14 corpus jobs to fail that night. Store itself is current (142/12, store mtime 2026-10-28 12:38 newer than every source).

**Follow-on 2026-10-30 (same dive, answered in the next turn):** the owner then asked whether `mem1`/`tool1` lanes would improve decision-making. No — and the mapping is in [[memory-and-tool-selection-fire-once]]: memory assembles once against the opening task and freezes into the system prompt, tool activations never expire, dead lanes keep declaring. The lane pattern's real home is a sys1 `curate`/`tool_route` task, which would also repair defect #2 here.

Next free KI: 375. See [[fleet-settings-abstraction-search]] for the rail work, [[the-right-pane-2026-10-29]] for the pane.
