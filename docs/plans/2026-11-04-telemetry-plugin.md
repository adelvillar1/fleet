---
status: completed
created: 2026-11-04
updated: 2026-11-04
slug: telemetry-plugin
---

# Plan: the telemetry plugin — the face's internals, as a live flow map

## Context

The owner asked for a telemetry visualization built through the plugin
creator pattern: what Fleet is doing internally — model, memories,
skills, sessions, huddles — and how information moves between the app's
layers, "appealing and graphical," strong enough to stun.

The plugin host exists (`src/plugins/face.rs`, `src/plugins/eval.rs`): a
bundle is a directory with a `manifest.json` and a static asset, the
registry is `<FLEET_HOME>/plugins.json`, two tiers — privileged (the
owner's reach) and untrusted (creator mode's output, sandboxed to the
manifest's declared read-only queries, enforced bridge-side at
`POST /api/plugins/query`). Three facts measured this session shape the
work:

1. **The data is all there and all live.** `/api/state` carries the
   session, the event log, the knowledge block, the approval rules and
   the turn budget; `/api/memory` 162 facts + 21 archived + 64 recall
   hits; `/api/skills` 452 skills + 6 proposals; `/api/sessions` 200
   sessions with a state histogram; `/api/router` the tier chains, the
   cache, the judged counts and the recent route rows with
   milliseconds; `/api/delegation` the depth cap and the child tree;
   `/api/huddles` 3 active + 2 inactive rooms with their personas and
   knowledge; `/api/usage` the panel attention counts. Every number a
   telemetry map wants is already computed by the bridge.
2. **`huddles` is not in the read-only query set.** `READ_ONLY_QUERIES`
   (`src/plugins/face.rs:23`) names twelve queries and huddles is not
   one of them, though `/api/huddles` answers. The owner named huddles
   as part of the ask, so the host's set gains it.
3. **An installed bundle has no page in the face.** The rail row
   mounts (`buildRail`), but pressing it does nothing: `showRoute`
   resolves an address through `ROUTES`, a plugin name is not one, so
   the hash falls back to the conversation, and no element ever loads
   `/plugins/<name>/`. The bridge serves the bundle and relays queries;
   the face's half of the mount was never built (the plugins plan's P1
   shipped the rail row and the management surface only). The demo
   fixture's postMessage protocol (`tests/fixtures/plugin-demo`) has no
   parent-side listener anywhere in `app.js`.

## Approach

**One bundle, one host leg, verified live.** The bundle is authored in
this repo at `plugins/telemetry/` (manifest + one self-contained
`index.html`), installed into the home as an **untrusted** bundle — what
creator mode produces, sandboxed to its declared queries — and rendered
by the face through a new mount panel that relays exactly those queries,
nothing else. No privileged elevation: the ask is a visualization, and
sandboxed read-only is sufficient for it, so the honest tier is the one
the manifest declares.

The visualization is **eight strata of Fleet's real pipeline**, drawn as
an instrument rather than a dashboard: door (the turn enters) → recall
(memories + hits) → skills (library + proposals) → route (sys1's
judgment, tiers, cache) → model (lanes, calls, milliseconds) → tools
(the approval gate, delegation depth) → rooms (huddles) → corpus (the
write-back, the consolidation preview, attention). A canvas spine
carries a particle current between strata whose density and speed are
driven by the live counts, so the picture moves at the rate the harness
actually works; the right column names the real entities of whichever
stratum is focused. Every figure comes from a query answer; a query the
bridge refuses renders as a named refusal rather than a zero.

## Steps

1. **P0 — the query set gains huddles.** `READ_ONLY_QUERIES` and
   `bridge.plugin_query` answer `huddles`; a test pins it.
2. **P0 — the bundle.** `plugins/telemetry/manifest.json` +
   `plugins/telemetry/index.html`: one file, inline everything, no
   external request, under the 2 MB bound, no credential shape (the
   static pass's sweep). Data arrives only through the relay.
3. **P0 — the mount.** `src/gui/assets/index.html` gains the bundle
   panel; `app.js` gains `bundleMount` (fetch the bundle's page, inject
   it as a sandboxed `srcdoc` frame — the canvas's own idiom), the
   declared-query relay (`event.source` checked against the frame, the
   answer posted back in the demo's protocol), and the route: a plugin
   name resolves to the bundle panel instead of falling back to chat.
4. **P1 — install and grade.** `fleet plugins install plugins/telemetry`,
   then `fleet plugins eval telemetry` — the static pass and every
   declared query driven through the bridge's enforcement.
5. **P1 — live verification.** The bundle rendered in Chromium against
   the running face's real numbers, screenshotted; the rail row pressed
   and the panel painted.

## Acceptance criteria

- [x] `READ_ONLY_QUERIES` contains `huddles`, `plugin_query` answers it,
      and a test pins both. — `src/plugins/face.rs:31` (the set is now
      `[&str; 13]`), `src/gui/bridge.rs:974` (the arm), `src/plugins/face.rs:564`
      (the pin: the set carries the name and a manifest declaring it parses
      with its queries in order).
- [x] `plugins/telemetry/` parses as a `FaceManifest` (untrusted, group
      `system`), its asset is non-empty, under 2 MB, and names no
      credential shape. — `fleet plugins install plugins/telemetry` installed
      it into `~/.fleet/plugins/telemetry/`; `fleet plugins eval telemetry`
      passed the static pass (which is the size bound and the credential
      sweep) and every declared query.
- [x] `fleet plugins eval telemetry` passes the static pass and all
      declared queries. — 8 declared queries, 8 answered, 0 refused.
- [x] Pressing the installed rail row paints the bundle in the face (not
      the conversation), and the bundle's queries come back answered. —
      Chromium at `http://127.0.0.1:3091/#/telemetry` paints `#panel-bundle`
      with the check's head, chip and declared-set note and a live frame;
      the face harness's BUNDLE case pins the route, the `srcdoc` frame's
      `sandbox="allow-scripts"`, the relay's ask/answer round trip with its
      sequence, and that a message from any other window is not relayed
      (`tests/face/harness.mjs`, 792 checks / 84 cases).
- [x] An undeclared query is still refused bridge-side with the manifest's
      set named — the mount adds no reach. — `POST /api/plugins/query` with
      `{"name":"telemetry","query":"vault"}` answers 403 naming the declared
      set; the harness's BUNDLE case drives the same refusal through the
      relay and asserts the page receives `ok:false` with the reason, and
      that the refusal counter reads `1 refused`.
- [x] `cargo build --all-targets` and `cargo test` pass, exit codes read
      on the cargo invocation. — `BUILD-EXIT=0`, `TEST-EXIT=0`
      (`test result: ok. 1828 passed; 0 failed; 3 ignored`), 2026-11-04.

## Status pass — 2026-11-04

All six criteria ticked with the path and the command that proves each.
Two things the criteria did not name and the work produced anyway: the
face harness's DOM stub learned `contains` and the frame's own window
(the mount asks whether its frame is still mounted, and the relay's gate
reads `event.source`), and KI-416 records the loaded-suite flake the gate
found — a bounded wait that could not say it had given up, which read as a
defect in the answer path rather than as a slow seat.

## Files to be touched

```
src/plugins/face.rs              READ_ONLY_QUERIES gains huddles (+ test)
src/gui/bridge.rs                plugin_query answers huddles
src/gui/assets/index.html        the bundle panel
src/gui/assets/app.js            bundleMount, the relay, the route
src/gui/assets/app.css           the bundle panel's own rules
plugins/telemetry/manifest.json  the bundle's manifest
plugins/telemetry/index.html     the visualization
docs/features/plugins.md         the mount and the new query, documented
docs/known-issues.md             the gaps found on the way
```

## Out of scope

- Privileged plugins and the promotion ladder (no elevation is asked for
  or taken here).
- Creator mode's conversation flow (the `creator` mention); this bundle
  is authored in the repo, reviewed in git, installed by the owner's
  command — the draft ground stays what it is.
- Any write path: the bundle reads, never writes, and the relay refuses
  everything outside the manifest's eight names.
- Telemetry persistence: no history, no sampling store — the map shows
  what the bridge answers now.
