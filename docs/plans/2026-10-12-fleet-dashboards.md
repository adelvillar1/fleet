---
status: completed
created: 2026-10-12
updated: 2026-10-12
slug: fleet-dashboards
resolution: |
  Both boards shipped as dual-face bundles and mounted in the fleet
  profile; acceptance criteria met. See docs/recaps/2026-10-12-fleet-dashboards.md.
---

# Fleet dashboard plugins — router board and second-brain board

Status: complete — both boards shipped and mounted in the fleet profile; acceptance criteria met. Plan-gate: this document.

## Goal

The owner's direction (2026-10-12): the visual parts of the agnostic-router-kit and the full functionality of second-brain must exist as harness plugins, and their dashboards must be genuinely visual — "the current control tower view is just text and honestly not very helpful."

Two new fleet plugins, each a dual-face (host + browser) bundle following the `@fleet/dsh-tower` pattern, mounted in the fleet profile:

- `@fleet/dsh-router-board` — a live dashboard for the agnostic-router-kit plane (`127.0.0.1:8300`): usage and delegation mix, tier chains, judge and cascade counters, workflow runs.
- `@fleet/dsh-second-brain` — the second-brain tower as a plugin: the gates surface (earned-autonomy board, HITL inbox, calibration), harness outcomes, quality loops, report rows and their conflicts, maturity weeks, tower projects.

Both read their sources read-only; neither launches, writes, or proxies anything. `@fleet/dsh-tower` stays as-is (git-facts portfolio); a later chunk may retire or restyle its text panel once these boards land.

## Data sources

Router board — read-only over the operator API (bearer token from the roster `router.localToken`): `GET /api/state`, `GET /api/usage`, `GET /api/suggest`, `GET /api/workflow-runs`. Defaults: `routerUrl http://127.0.0.1:8300`, roster discovery under `~/.agnostic-router-kit` with the checkout `roster.json` as the token source; machine-local overrides via the profile patch layer.

Second-brain board — read-only file reads of the derived state at `~/.second-brain` (symlink to the project): `reports_state.json`, `gates_state.json`, `harness_state.json`, `maturity_state.json`, `loops_state.json`, `tower_data.json`. No pipeline is triggered; jobs stay in second-brain itself.

## Design (per plugin, the dsh-tower seam)

- Host: `Config` + `defaultConfig()` (machine-overridable), a cached-snapshot service with a self-rescheduling `.unref()` timer, a `WebRoute` `GET <path>` behind the `requestRejection` fence with `cache-control: no-store`, and a `shared.ts` wire contract imported by both faces. Route paths: `/fleet-router-board/state`, `/fleet-second-brain/state`.
- Browser: client plugin with `inject = ['slots', 'locale']`, an English-only typed dictionary, a fetch-and-poll source (validate-don't-cast parsers, single in-flight read, error keeps the last snapshot), and a `main` panel + `sidebar.panellist` icon registration.
- Visual language, no chart library: hand-rolled SVG (donut, bars, heatmaps, sparklines) and CSS-module cards using `--dsw-alias-*` tokens — same approach as the router's own dashboard and second-brain's pages, in the harness client's idiom.
- Mounting: insert-only `cordis.patch.yml` per package; `link:` dependency + `dsh.profile.bundles` entries in `~/.dsh/profiles/fleet/package.json`.

## Verification

Per plugin, mirroring dsh-tower's four spec files: host service + parser spec, route spec (real `node:http`, fence-first, no-store, GET-only), client source validation spec, and a jsdom slot render spec through `SlotTestRuntime`. `verify-export-jsdoc`, `verify-client-ui-i18n`, `verify-client-packages`, `verify-client-route-resolution` must pass for the new packages; run the matched gates, not the full suite. Live check: the fleet profile boots with both panels in the sidebar and both boards render against the running router and the second-brain tree.

## Acceptance criteria

- C1: both plugins mount in the fleet profile; each adds a sidebar panel whose board renders real data from its source.
- C2: each board is visual — cards, bars/heatmaps/sparklines, status dots — with no plain-text-table-only surface.
- C3: every wire payload is validated (malformed source → named error state, never a cast or a crash), and a refused read keeps the last snapshot visible.
- C4: matched gates pass for both packages; unit/slot tests green.
- C5: both plugins stay read-only toward their sources.
