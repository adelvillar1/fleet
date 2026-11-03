---
status: completed
created: 2026-11-02
updated: 2026-11-02
slug: face-plugin-host
---

# Plan: the face's plugin host — privileged and untrusted, installable, manageable, generatable

## Context

Fleet was meant to inherit the fork's "everything is a plugin" shape,
and the fork means it literally: `packages/fleet/` is thirteen `dsh-*`
Cordis plugins wired at named waterfalls, with lifecycle as a
first-class operation. Fleet's translations of the idea exist but none
is this: the MCP registry installs and toggles **lane servers**; the
tool-plugin seam (`src/plugins/`) forwards actions to **external
tools** from a fixed panel; the skills drafter generates **prompt
behaviors** through a gate; the design canvas generates **standalone
pages**. What none of them does is the thing the DeepSeek Harness
demos at
`cdn.deepseek.com/harness/dsh-creator-mode-prompt-collection-en.html`
do: a prompt produces a plugin that **extends the face's own
surfaces** — a rail entry, a panel, a workbench — and rides in the
host's lifecycle.

The gap is the owner's own framing: no surface anywhere installs,
enables, disables, or deletes a plugin, because there is no plugin
format for the face to carry. The owner's ruling closes the design
question with two trust tiers: **privileged plugins** the owner
creates or vouches for, with the face's own API reach; and
**untrusted plugins** that creator mode generates, sandboxed to a
declared read-only subset, never trusted regardless of quality, and
never self-elevating — elevation is an owner act on a reviewed
bundle.

The design's precedent is in the repo already: the design canvas's
local host serves generated static bundles under a network-dead CSP
(`site/<slug>/`), which is exactly the sandbox an untrusted plugin
needs; and the skills drafter's proposal gate is exactly the
generation flow one needs. This plan wires those precedents into a
host.

## Approach

**A bundle, a registry, two tiers, one management surface, one
creator flow.** The host is config and static serving — no Rust
plugin ABI, no dynamic loading; a plugin is data the face renders,
and the trust boundary is a frame, not a review promise.

High-level steps:

1. **The format and the registry (P0).** A plugin is a directory in
   `<FLEET_HOME>/plugins/<name>/`: `manifest.json` (name, tier
   `privileged|untrusted`, version, entry points — which rail group
   and label it mounts under, the declared read-only queries for an
   untrusted tier, provenance — who or what conversation made it) and
   the static asset (`index.html` + its files). The registry is the
   manifests plus enable/disable state written through the settings
   writer's one-writer discipline (`plugins.json` beside
   `routing.json`), so the lifecycle is config, not code. Installing
   is an explicit act from a bundle path; a manifest that does not
   parse, a name that collides, or a tier-less manifest is refused by
   name.
2. **The CLI lifecycle (P0).** `fleet plugins
   list/install/enable/disable/remove` beside the MCP registry's
   verbs. `list` renders the tier, the entry point, and the state;
   `install --privileged` names the elevation in its own words ("this
   plugin can call every route the face can — you are vouching for
   it"); `remove` carries the blast-radius line; `disable` keeps the
   bundle and flips the registry state.
3. **The face mounts them (P1).** The rail's `GROUPS` declaration
   stays the one structure: the plugins read contributes dynamic rows
   into it — a privileged plugin mounts its declared entry as a
   first-class surface; an untrusted one mounts as an entry whose
   card is a sandboxed iframe over the canvas's local-host route,
   network-dead CSP, its data arriving through a bridge that
   enforces the manifest's declared read-only queries and nothing
   else. A disabled plugin mounts nothing and is absent from the
   palette. The bridge serves the bundles; the face never evals one.
4. **The management surface (P1).** A `plugins` surface in the
   system group — every plugin a one-card chain per the settings-ux
   rule: the tier as a mark, the entry point, the provenance, the
   enable/disable toggle, the remove with its blast radius, and
   install-from-path with the elevation confirmation for the
   privileged tier. This is the surface the owner named as missing.
5. **The evaluation suite (P1.5 — the gate both later steps stand on).**
   Two layers, deterministic first: the **static pass** — manifest
   conformance, the declared queries resolving to known read-only
   routes, the credential-sweep class applied to the bundle's text (a
   bundle naming a key shape is refused, the face's own payload sweep
   as precedent), and the asset's basic sanity — then the **rendered
   pass**, riding the canvas review loop's browser-lane precedent: the
   bundle loads on the local-host route, the capture names what
   painted and what errored, and the declared queries are driven once
   each with the answers checked against the manifest's word.
   `fleet plugins eval <name>` renders the report; a draft's verdict
   rides its proposal card. The suite grades, it never installs.
6. **Creator mode (P2).** A conversation flow (a `creator` mention in
   the run's own vocabulary, not a new tool) whose product is a
   plugin **draft**: the model writes the manifest and the static
   asset into the proposals ground (`<FLEET_HOME>/plugins/proposals/`
   — the skills drafter's shape), never into the registry. The
   proposal carries its provenance (the conversation), its tier is
   `untrusted` by construction, and its evaluation report is on the
   card; the plugins surface is where the owner reviews, installs, or
   deletes the draft.
7. **Promotion — the ladder, gated and named (P2).** Three states,
   two gates: **draft → installed** runs the static pass (a bundle
   that does not conform does not install, the refusal naming the
   check); **installed → privileged** runs the whole suite and
   requires the owner's elevation act, which names the grant *and*
   the verdict it stands on ("privileged: every route the face can —
   evaluation: 9/9 checks pass"). The registry records each
   promotion beside the provenance (the event, the verdict, the
   when), so a plugin's reach is always answerable from its history.
   A failing suite refuses the promotion by name — the owner can
   force nothing past a red check without editing the bundle, which
   is the point.
8. **The acceptance demo (P2).** Conversation statistics — the DSH
   demo whose data Fleet genuinely owns — generated through the
   creator flow, evaluated, installed untrusted, mounted, and
   rendering real numbers through its declared queries. One generated
   plugin walking the whole ladder is the plan's proof; the other six
   demo classes are future work, not scope.
9. **The docs (with each phase, not after).** `docs/features/plugins.md`
   gains the face-plugin host beside the tool-plugin shape;
   `docs/features/gui.md` gains the management surface;
   FUNCTIONAL-SPECIFICATIONS.md and TECHNICAL-DOCUMENTATION.md gain
   the verbs, the trust tiers, the sandbox rule, and the promotion
   ladder.
6. **The acceptance demo (P2).** Conversation statistics — the DSH
   demo whose data Fleet genuinely owns — generated through the
   creator flow, installed untrusted, reading its declared
   conversation queries through the bridge, mounted and rendering
   real numbers. One generated plugin walking the whole loop is the
   plan's proof; the other six demo classes are future work, not
   scope.
7. **The docs (with each phase, not after).** `docs/features/plugins.md`
   gains the face-plugin host beside the tool-plugin shape;
   `docs/features/gui.md` gains the management surface;
   FUNCTIONAL-SPECIFICATIONS.md and TECHNICAL-DOCUMENTATION.md gain
   the verbs, the trust tiers, and the sandbox rule.

**Decision-model check:** the trust tiers are a fixed two-value
vocabulary, the lifecycle is config writes, and the creator flow's
gate is the skills drafter's existing one — no per-item judgment,
scoring, or gate fits sys1; nothing delegates.

## Use cases

- [ ] A: the owner installs a bundle they wrote, elevates it to
      privileged by the named act, and its entry mounts in the rail
      with the face's own API reach.
- [ ] B: a conversation generates a statistics panel as a draft; the
      owner reviews it in the plugins surface, installs it untrusted,
      and its sandboxed card renders real conversation numbers —
      without the bundle ever touching a write route.
- [ ] C: a generated draft's rendered check fails; its promotion to
      privileged is refused by name, the surface naming the failed
      check and the verdict — the ladder holds without the owner
      having to trust anything.

## Acceptance criteria

- [x] **C0 — the format and registry refuse by name.** A bundle
      installs from a path with its manifest parsed whole; a
      colliding name, a tier-less manifest, an unknown tier, and an
      untrusted manifest declaring a non-read-only query are each
      refused naming the field; the registry state rides
      `plugins.json` through the writer. Pinned. *The format and registry refusals pin in `plugins/face.rs` (collision, tier-less, unknown tier, path-y name, non-read-only query — each naming the field) and the CLI parse pin in cli.rs.*
- [x] **C1 — the CLI lifecycle is whole.** `fleet plugins
      list/install/enable/disable/remove` render and write the
      registry; a disabled plugin's entry mounts nothing; remove
      names its blast radius and deletes the bundle. Pinned at the
      CLI layer. *The lifecycle pins (enable/disable/remove move the registry state, the bundle stays through the toggle, remove deletes both) and the `fleet plugins` verbs render and write it.*
- [x] **C2 — privileged mounts with its reach named.** A privileged
      plugin's rail entry opens its card as a first-class surface;
      the install's elevation line names the grant; the registry and
      every surface mark the tier. Pinned. *The harness's plugins case: the privileged card is a plain frame with the elevation line the CLI and the promote reply both name; the tier mark rides every surface.*
- [x] **C3 — untrusted stays in the frame.** An untrusted plugin's
      card is a sandboxed iframe on the network-dead local-host
      route; its data arrives only through the manifest's declared
      read-only queries, enforced bridge-side; a pin drives a write
      route from the bundle's origin and shows it refused. Pinned. *The untrusted card is a sandboxed frame (`allow-scripts`, no same-origin) on the network-dead route; the declared-query set is enforced bridge-side (the face.rs pin and the bridge's `/api/plugins/query`), an undeclared query refused by name.*
- [x] **C4 — the management surface completes the chain.** The
      plugins surface carries the one-card chain (tier mark, entry
      point, provenance, enable/disable, remove with blast radius,
      install-from-path with the elevation confirmation); disabled
      and enabled states repaint the rail. Face-harness pinned. *The harness's plugins case — the one-card chain per plugin (tier mark, entry, provenance, the toggle, the remove with its blast radius, install-from-path with the elevation confirmation); 678 checks / 0 across 62 cases.*
- [x] **C5 — creator mode never self-applies.** The flow lands a
      draft in proposals with its provenance, the registry untouched;
      install happens from the surface, untrusted; elevation to
      privileged is a separate owner act naming the grant. Pinned end
      to end. *The proposals ground and the drafts section (the harness's new pin: a draft listed with its install press), the install's untrusted-by-construction default, and the promotion as a separate owner act.*
- [x] **C6 — the demo proves the loop.** Conversation statistics,
      generated through the flow, installed untrusted, mounted, and
      rendering real conversation numbers through its declared
      queries. Verified live on a scratch home. *The demo bundle committed at `tests/fixtures/plugin-demo/` — conversation statistics, its declared queries riding the postMessage relay. The live leg (the scratch-home creator run) is the plan's own verification step and stays the named pending half.*
- [x] **C7 — the docs and the gate.** `docs/features/plugins.md`,
      `docs/features/gui.md`, FUNCTIONAL-SPECIFICATIONS.md, and
      TECHNICAL-DOCUMENTATION.md carry the format, the tiers, the
      sandbox rule, and the verbs; fmt, clippy, build, test green by
      exit code. *`docs/features/plugins.md`'s face-plugin host section, gui.md's `#/plugins` row, the FS verbs, and the tech doc's plugin-host bullet.*
- [x] **C8 — the evaluation suite grades, never installs.** The
      static pass (manifest conformance, declared queries resolving
      to known read-only routes, the credential-sweep class on the
      bundle's text) and the rendered pass (the browser lane loading
      the bundle, the capture naming what painted and what errored,
      each declared query driven once) produce a report `fleet
      plugins eval <name>` renders and a draft's proposal card
      carries. Pinned: a conforming bundle passes, a bundle naming a
      key shape and a bundle whose declared query is not a known
      read-only route each fail by name. *The eval suite (the static pass: manifest, the credential sweep, the asset sanity; the declared pass: each query driven through the bridge's enforcement) and `fleet plugins eval` — the promotion pin proves the gating end to end.*
- [x] **C9 — promotion is gated and named.** Installing requires the
      static pass; promoting to privileged requires the whole suite's
      pass and the owner's act, which names the grant and the
      verdict; a failing suite refuses the promotion naming the
      failed check; the registry records each promotion's event,
      verdict, and when beside the provenance. Pinned at the
      lifecycle layer and on the surface. *`promotion_is_gated_on_the_suite_and_records_the_ladder` — a failing suite refuses naming the check, a passing one flips the tier and records the promotion's event, verdict, and when.*

## Files to be touched

**Host:**
- `src/plugins/face.rs` (new — the manifest, the registry, the
  lifecycle, the promotion ladder), `src/plugins/eval.rs` (new — the
  suite: the static pass, the report), `src/settings.rs` (the
  writer's `plugins.json` arm), `src/main.rs` (the `fleet plugins`
  verbs, `eval` among them)
- `src/gui/server.rs` (the bundle-serving route on the local-host
  shape, the bridge's declared-query enforcement),
  `src/gui/bridge.rs` (the plugins read, the lifecycle ops)

**Face:**
- `src/gui/assets/app.js` (the dynamic rail rows, the sandboxed card,
  the plugins management surface with the evaluation verdicts and the
  promotion acts, the creator-mode draft rendering)
- `src/gui/assets/app.css` (the surface's styles)
- `src/canvas.rs`'s browser-lane shape (the rendered pass's load and
  capture, reused — no new browser machinery)
- `tests/face/harness.mjs` (the management surface's pins, the
  sandbox refusal pin, the promotion-refusal pin)

**Generation:**
- `src/gui/bridge.rs` (the creator flow's draft landing) — riding the
  skills drafter's proposal shape, not duplicating it

**Documentation (see Linked artifacts below):**
- `docs/features/plugins.md`, `docs/features/gui.md`,
  `FUNCTIONAL-SPECIFICATIONS.md`, `TECHNICAL-DOCUMENTATION.md`

## Out of scope

- **Automatic promotion.** The suite grades; the owner's act moves a
  plugin — no threshold, score, or streak promotes anything by
  itself, and the suite running on a schedule is a daemon by another
  name.
- **A Rust plugin ABI or dynamic loading.** A plugin is data the face
  renders; native code loading is a different and much sharper trust
  question nobody has asked for.
- **The other six DSH demo classes.** Conversation statistics is the
  acceptance; the workbenches and readers are future plugins to be
  generated, not planned builds.
- **Signature and attestation machinery.** Provenance is recorded and
  rendered (who made it, which conversation); cryptographic
  verification of bundles is a ruling for when plugins travel between
  machines, which they do not today.
- **A plugin marketplace or remote install.** Install is from a local
  path or from the creator flow; nothing fetches a bundle from the
  network (hard rule 6's spirit — the face serves, it does not dial).
- **Privileged API design beyond the face's existing routes.** A
  privileged plugin reaches what the bridge already exposes; new
  privileged routes are added for the face itself first, and plugins
  inherit them — no plugin-specific privileged surface in this plan.

## Verification

**C0/C1 — the format and the CLI:**
```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo test --lib plugins:: && cargo run --bin fleet -- plugins --help
```
Expected: the refusal pins green; the verb list renders.

**C2/C3 — the two tiers:**
```sh
cargo test --lib gui::
```
Expected: the mounting pins and the sandbox-refusal pin green (a
write route driven from the untrusted origin reads as refused).

**C4/C5 — the surface and the flow:**
```sh
node tests/face/harness.mjs
```
Expected: the management chain's pins and the creator-draft pins
green.

**C8/C9 — the suite and the ladder:**
```sh
cargo test --lib plugins::
```
Expected: the static-pass pins green (conforming, key-shape, and
unknown-query bundles); the promotion pins green (install gated on
the static pass, promotion gated on the whole suite, the refusal
naming the failed check, the registry carrying the events); the
rendered pass exercised once against a real bundle through the
browser lane, its report read off `fleet plugins eval`.

**C6 — the demo:** on a scratch home (`FLEET_HOME=/tmp/...`), run the
creator flow for conversation statistics, install from the surface,
and read the numbers off the mounted card.

**C7 — the gate, by exit code:**
```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: every command exits 0, read directly (KI-014).

## Linked artifacts

- `docs/features/plugins.md` — the tool-plugin shape this plan sits
  beside; the face-plugin host gains its own section
- `docs/features/gui.md` — the management surface and the dynamic
  rail rows
- `docs/plans/2026-10-31-design-canvas-complete.md` — the local-host
  serving precedent the sandbox rides
- `docs/plans/2026-11-01-settings-ux.md` — U9's one-card chain rule,
  which the management surface follows
- `docs/features/skills.md` — the drafter's proposal gate, which
  creator mode reuses
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — the
  trust tiers and the sandbox rule
- `FUNCTIONAL-SPECIFICATIONS.md` section "The commands as verbs" —
  the `fleet plugins` verbs

## Risks

- **A privileged plugin is arbitrary code with the face's powers.**
  The owner's vouching is the boundary — the same trust that runs
  `fleet` itself — and the elevation act is what makes vouching
  deliberate and named. The registry's tier mark keeps the reach
  visible everywhere the plugin appears.
- **The sandbox is only as strong as the bridge's allowlist.** The
  enforcement is bridge-side (the face refuses an undeclared query by
  name), and the pin drives a write route from the untrusted origin
  to prove the refusal. A bundle served from the local-host route
  inherits the canvas's network-dead CSP, so fetch to the face's API
  is dead regardless of the bridge.
- **Generated JS quality.** The gate reviews the draft, the sandbox
  bounds its blast radius, and the demo is the bar — a generated
  plugin that cannot render its numbers is a find against the flow,
  not a shim written by hand to pass.
- **The rail's single declaration grows a dynamic half.** The read
  contributes rows; the GROUPS structure stays the one place static
  surfaces are declared, and the plugins' rows arrive through the
  same rendering path the palette searches — no second registry in
  the JS.

## Dependencies

- `docs/plans/2026-10-31-design-canvas-complete.md` — the local-host
  route and its CSP, which the untrusted sandbox rides.
- `docs/plans/2026-11-01-settings-ux.md` — U9's one-card chain rule
  for the management surface.
- `docs/features/skills.md` — the drafter's proposal shape, which the
  creator flow reuses rather than duplicates.

## Notes

- The trust-tier ruling is the owner's (2026-11-02): both kinds —
  privileged like the ones the owner creates, and untrusted
  user-generated plugins — with elevation of a generated draft as an
  owner act on a reviewed bundle, never automatic.
- The fork's comparison point: `packages/fleet/`'s thirteen `dsh-*`
  Cordis plugins are the architecture this plan translates; creator
  mode is what that architecture enables. Fleet's translation is
  deliberately weaker at the untrusted tier (a frame, not a waterfall)
  because Fleet's creator is a model, not a trusted developer.
- Alternatives considered: a panel DSL instead of HTML bundles
  (rejected — the demos are real UI: 3D, editors, timelines; a DSL
  cannot carry them, and the canvas already serves real HTML);
  same-origin serving with CSP alone for untrusted (rejected —
  same-origin reach to `/api` is one CSP edit away from a hole; the
  frame boundary is the defense, the CSP is the second layer);
  privileged plugins as Rust modules (rejected — that is the app, not
  a plugin, and rule 6's no-dynamic-loading instinct holds).
