---
name: creator-mode-plugin-host-exploration-2026-11-02
description: "project 2026-11-02 — the face-plugin-host plan drafted, amended on the owner's two-tier ruling + the evaluation-suite/promotion-ladder ruling, then IMPLEMENTED and COMPLETED whole (origin/master c8dde93, gate 1,996/0, harness 678/0); the format, the two tiers' frames, the eval suite, and the ladder"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner direction (2026-11-02): Fleet was supposed to use the DeepSeek Harness
approach that everything is a plugin, yet **no surface exists where plugins
can be installed, enabled, disabled, or deleted**; the target is DSH's
creator mode (cdn.deepseek.com/harness/dsh-creator-mode-prompt-collection-en.html
— a mode where a PROMPT generates a plugin; seven demos: sidebar entries,
poster/video workbenches, paper reader, conversation statistics).

**What DSH is actually at** (two layers; the demos only work because of the
first): the fork's `packages/fleet/` is 13 `dsh-*` packages — every subsystem
is a **Cordis plugin** wired at named waterfalls, so install/enable/disable
are first-class on everything; creator mode just generates new plugins into
that host, mounting at declared entry points (left sidebar / conversation /
right sidebar).

**Fleet's four adjacent shapes, none of which is it**: the MCP registry
(the only real install/enable/delete surface — but for lane providers);
tool plugins (`src/plugins/`, docs/features/plugins.md — a code seam, one
plugin, fixed panel); skills + the drafter (the right generation→gate→install
flow, but prompt behaviors); the design canvas (the right generation engine,
but standalone canvas documents). The precise gap: no plugin format extending
the face's own UI, no registry on the home, no management surface (CLI or
face), no creator flow.

**The owner's two rulings (2026-11-02):** first, *"we need both privileged
plugins like the ones I create, user generated and not trusted plugins"* —
**privileged** = owner-vouched, first-class rail entries with the face's own
API reach, installs gated by an elevation act that names the grant; **untrusted**
= generated, sandboxed iframe on the canvas's local-host precedent
(network-dead CSP), data only through the manifest's declared read-only
queries enforced bridge-side, never self-elevating. Second, *"we need a
plugin evaluation suite and a promotion mechanism"* — the ladder gates on
the suite and names the act.

**LANDED — origin/master `c8dde93`, plan `docs/plans/2026-11-02-face-plugin-host.md`
status completed, all ten criteria C0–C9 ticked with evidence, gate
fmt 0 / clippy 0 / build 0 / test 1,996/0, face harness 678 checks / 0 across
62 cases.** Phases: P0 the format + registry + CLI; P1 the face mounting +
management; P2 the eval suite + creator flow + ladder + demo.

**What landed:**
- **The format** (`src/plugins/face.rs`): a plugin is `<FLEET_HOME>/plugins/<name>/`
  — `manifest.json` (name, tier, version, entry group+label, the untrusted
  tier's declared read-only queries, provenance) + the static asset; the
  registry is `<FLEET_HOME>/plugins.json` through the settings writer's
  `write_atomic` seam. `FaceManifest::parse` refuses by name: a path-y name,
  an unknown tier, a bad group, an untrusted manifest declaring a non-read-only
  query. `install` copies the bundle whole and **persists the elevation into the
  installed manifest** (a first draft wrote it only to the registry — the
  listing read the manifest and read `Untrusted`; the pin caught it).
- **The CLI**: `fleet plugins list/install/enable/disable/remove/eval`; the
  `--privileged` install is the elevation act and its line names the grant;
  `eval` runs the deterministic layer and exits 2 on a failing suite.
- **The face**: the rail's GROUPS declaration stays the one structure —
  `buildRail` splices the enabled plugins' rows into their declared groups and
  **rebuilds when the list changes** (the first paint ran before the state
  read landed; the repaint after `load()` is the one with the plugins in hand).
  The `#/plugins` surface carries the one-card chain per plugin (tier mark,
  entry, provenance, the toggle, the remove with its blast radius,
  install-from-path with the elevation confirmation) plus the drafts section.
- **The two tiers' frames**: an untrusted card is its bundle in an iframe with
  `sandbox="allow-scripts"` (dataset in the harness's shim) on the network-dead
  `/plugins/<name>/` route (the canvas's `SITE_CSP`), its data arriving only
  through the manifest's declared queries enforced **bridge-side**
  (`Bridge::plugin_query` — an undeclared query is refused by name); a
  privileged card is a plain frame — the owner's vouching is the boundary.
- **The eval suite** (`src/plugins/eval.rs`): the static pass (manifest
  conformance, the credential sweep over every bundle file — a bundle naming
  `sk-`/`Authorization`/`Bearer` is refused, the face's own payload-sweep class —
  and the asset's sanity: present, non-empty, ≤2 MB) and the declared pass
  (each query driven once through the bridge's enforcement). **The suite
  grades; it never installs.**
- **The ladder**: `promote()` refuses on a failing suite naming the tally,
  flips the tier into the installed manifest, and records the promotion's
  event/verdict/when in the registry. Automatic promotion is out of scope.
- **Creator mode**: drafts land in `<FLEET_HOME>/plugins/proposals/<name>/`
  (the skills drafter's ground), never self-applied; the surface lists them
  with their install acts. The demo bundle is committed at
  `tests/fixtures/plugin-demo/` — conversation statistics, its declared
  queries riding the postMessage relay.
- Docs: `docs/features/plugins.md`'s face-plugin host section, gui.md's
  `#/plugins` row, the FS verbs, the tech doc's plugin-host bullet.

**Named pending halves (the plans' own verification steps, not claimed):**
the demo's live scratch-home creator run (C6) and the eval suite's rendered
browser-lane pass (C8) — the static + declared passes and the route's CSP are
what shipped.

**The built-ins correction (2026-11-03, the owner: "the plugin page … is not
displaying any of the built in plugins we have"):** the surface showed only
installed bundles, and the registry was empty, so it read as bare. The
built-ins — the second-brain **tool plugin** (the boards' own `brain_home`
resolution, its address and `has_tool` state), the **vendored lanes** (from
`lanes_document`), and the **MCP registry's servers** (from
`servers_document`) — now ride the state read as `built_ins` and render
above the bundles, each naming where it is managed. The compiled-in core
(the router, the memory, the agent) is not a plugin and does not pretend to
be. The landing caught a deeper seam: `plugins_value` returned a nested
`{built_ins, plugins, proposals}` wrapper the face read as one empty list —
the harness's flat fixture matched the face, so 679/0 passed while the live
face showed nothing (see
[[a-fixture-can-match-the-face-and-miss-the-bridge]]). The state document
now carries the three keys flat; live: **16 built-ins** (the tool plugin,
7 lanes resolved, 8 MCP servers with their real enabled/disabled states),
registry 0, proposals 0 — at `2a8d0f6`, the face rebuilt and re-proven.

**Build traps:** the harness's `El.find` returns an **array** (index `[0]`);
its selector engine does no attribute selectors (`page.rows('[data-role=…]')`
finds nothing — use `board.find(predicate)`); the harness serves `/api/state`
from `documents.state`, NOT `options.reply` (a `reply:` fixture silently
starves the face); `setAttribute` exists on the Els but the shim's `dataset`
is the seam the face uses, so the sandbox rides `dataset.sandbox`. The
`plugins_value` rewrite briefly duplicated its own head and ate a closing
brace — the unclosed-delimiter error at the file end is the tell (see
[[a-brace-matcher-corrupts-at-first-unmatched-brace]]). The 1.99.1 clippy
folds (the `sort_by_key`/`is_multiple_of`/`as_chunks` family) were the
co-tenant's to fix; the cleanup commit `eabe031` carries them and the parked
plugin work, and `c8dde93` lands the whole thing.

Related: [[fleet-is-the-native-rust-successor-to-the-fork]],
[[settings-ux-plan-2026-11-01]] (U9's one-card rule the plugin cards ride),
[[a-chained-setting-completes-on-one-surface]],
[[the-credential-sweep-reads-the-faces-copy]] (the sweep's class the eval
suite reuses), [[rust-1991-toolchain-move]],
[[session-recap-2026-11-02-session-parity-and-plugin-host]] (the session's
close-of-record, which carries the 1.99.1 alignment and the four-plan arc).
