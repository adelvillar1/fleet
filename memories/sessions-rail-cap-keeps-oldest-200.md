---
name: sessions-rail-cap-keeps-oldest-200
description: "CORRECTS the retired 'frozen sessions set' diagnosis: /api/sessions rescans per request but its 200-cap keeps the OLDEST 200 (take over an oldest-first list), so everything after 2026-10-23 is cut from the rail AND ?q= search; restart cannot fix it."
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

The retired memory `sessions-api-set-frozen-at-200-since-2026-10-23` (memories/.archive/) inferred a set frozen at server start or a persisted index. Code read 2026-10-27 pins the real mechanism — nothing is frozen:

- `sessions_read` (src/gui/bridge.rs:4416) calls `session::list(home)` fresh on EVERY request, then `entries.into_iter().take(SEARCH_BOUND)` (bridge.rs:4435, SEARCH_BOUND=200 at bridge.rs:4253).
- `session::list` sorts by id ASCENDING — oldest first (src/session.rs:749, doc comment says so).
- So the cap keeps the **oldest** 200, which end exactly at 2026-10-23T00:25Z (head row `20261023T002500Z-af526386`). The comment at bridge.rs:4428 states the intent — "the newest SEARCH_BOUND logs" — the take is on the wrong end.
- `?q=` content search runs over the same truncated slice (bound:200 searched:200), so post-10-23 sessions are invisible to search too.
- Restart CANNOT fix: proven 2026-10-27 (server PID 8523 started 08:50 that day; set unchanged).

FIXED 2026-10-27 (plan `docs/plans/2026-10-27-sessions-end-by-archive.md`, KI-334): the cap takes from the tail now (reverse/truncate/reverse), search inherits the fix; pinned by `the_rail_keeps_the_newest_search_bound_logs`. The same plan carried the owner's follow-up ruling — see [[sessions-do-not-end-on-their-own]] — the loop stamps `handed back` (not `completed`) for interactive model stops, and the only end is the owner's new archive verb (`POST /api/session/archive`, registry `sessions-archived.json`, collapsed Archived group in the rail, any row archivable); six new Rust tests + the face harness ARCH case pin it. The owner restarts the gui to serve the fixed build.

Gate note from that session: the declarations test failure was FIXED the same day (KI-335, closed) — the machine's sweep state (`list_dir` in `~/.fleet/config.json` `tools.disabled`, written 08:35 by the lifecycle sweep) leaked into scratch-home Agent tests via the `tools_disabled` read on the fleet home; fixed structurally (the disabled set now reads the run's OWN home, derived from the session log path — production-identical since every production path passes its own home as `improve_home`) plus `settings::set_tools_disabled_for_test`/`clear_tools_disabled_for_test` for simulating lists. First attempt via `improve_home: Some(scratch)` was WRONG — `improve_home` gates the `improve` tool's declaration and pushed the surface over the `tool_search` bound. The six integration tests pinning the interactive end vocabulary were updated to the handback words. Full gate after: lib 1578/0, integration 149/0, blend1/improve/install/face green. The eval target's 2 failures were ALSO the machine reaching into tests — KI-336, closed permanently the same afternoon: `resolve_project` read the LayerFS substrate's provisioning through `read_machine`'s global fleet-home fallback, so the owner's provisioned machine made every scratch-home delegation try the (down) worker. Fixed the same way as KI-335: `layerfs_tools::read_machine_at(home)` and `resolve_project(progress, run_home)` against the run's own home (the sessions directory's parent, now computed once and shared with child_home); production byte-identical, the eval corpus grades the delegation contract deterministically with the worker up or down (proven: full gate green while the worker was still down). Pin: `a_delegation_resolves_no_substrate_outside_its_own_home`. `a_leftover_control_does_not_survive…` remains the known parallel-load flake class (passes alone).

LIVE STATE, 2026-10-27 evening: the rail issue "still there" was the served binary — the running gui was still the 08:50 process (PID 8523) on the old code; killed and relaunched onto the new build (PID 34016, `fleet gui --no-open --remote`, output appended to `~/.fleet/gui-3080.log`) and verified live: the PWA session listed (state `interrupted`, never closed by the agent), the dogfood triage session reads `handed back`, and the archive verb round-tripped end to end. An instance of the standing lesson [[verify-the-binary-is-newer-than-the-source]]: landed code is not served code until the process restarts. PENDING: the owner archived `20261027T134421Z-38ae3263` (18:55Z) and asked for its restore — the unarchive was REFUSED because that session has a run in flight (`needs-approval` outranks `archived` in work_state, and the archive verb refuses around a live run by design); the flag is still set in `sessions-archived.json`, so once the run ends the row reads `archived` again until unarchived. Restore = POST /api/session/archive `{archived:false}` for that id, or the row's Unarchive chip, after the run ends (stop or approve — the owner's call; its partial work survives a stop).

Recovery context: the owner's bot-PWA session is `20261026T181734Z-76405ddd` (interactive, fleet repo, 2026-10-26T18:17Z). Nothing is lost — replay reads the log by id straight from disk (`replay_read`, bridge.rs:3929 → GET `/api/session?id=<id>`), so it opens in the face at `#/sessions/<id>` despite the rail cut.

Dogfood state: fleet's own triage session `20261027T135549Z-70d7a54d` ENDED (completed) 15:24Z with a handoff, not hung — that self-declared "completed" is itself now the owner's named design defect, see [[sessions-do-not-end-on-their-own]] — its five delegation opens (14:09–14:29Z) each died as start-only subagent logs because the LayerFS worker was down (127.0.0.1:54277 refused; same Docker-reset outage as [[mcp-vizro-removed-2026-10-27]]; `fleet install` re-provisions). Its plan: delegation back → fix brief → code+gate → two owed KI entries (stale rail; LayerFS outage). File those KI rows carefully — KI-331's concurrent hand-append hazard applies while scheduled catch-ups run.
