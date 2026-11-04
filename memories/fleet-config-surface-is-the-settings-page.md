---
name: fleet-config-surface-is-the-settings-page
description: "2026-10-20 — the GUI Settings page is fleet's one configuration surface (router, provider roster, sys1/sdm1/sem1/gen1 lane cards, roles, vault, spend caps), shipped as adfd353; the write seams it added are the ones to reuse (settings::catalog, router_config, set_lane, set_secret, roles::write, lanes::configured_spec) and its three deliberate limits"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's request (2026-10-20) was that the GUI's settings stop being a
generic knob list plus a raw JSON textarea — "it should have something like
what we did for the zcode router kit" — and then, wider: "a similar
configuration surface for providers, for sys1, sdm1, sem1, and gen1 models. I
shouldn't be writing json files when I can have a more friendly approach."
Shipped as **`adfd353`**, pushed, gate **1,306 lib + 103 integration = 1,437
passed, 1 ignored**, release build clean, live Playwright audit at 1440 and
390 with no console errors and no overflow.

**The page**: section chips → router · providers · lanes · roles · vault ·
spend · knobs · rules · cron · identity · environment. Every value field
follows [[no-form-asks-for-a-value-fleet-already-has]].

- **Router** — the spend guard (`gates.allow_api_models`, previously no
  surface at all), tier cards with target select + ordered fallback chips, a
  **resolved-now** column computed by the router's own rules (gate → enabled →
  key, `route::admissible` made `pub(crate)`), thresholds, a pin select. The
  `hard` tier is badged `walker` because the orchestrator walks it whatever was
  judged ([[orchestrator-routes-through-hard]]).
- **Providers** — field editors over the roster; validation stays the
  loader's `deny_unknown_fields` schema. Key presence is a name-level badge,
  never a value.
- **Lanes** — a fleet-owned `<FLEET_HOME>/lanes.json` whose values
  `lanes::configured_spec` injects into the lane child at launch (precedence
  process env > `.env`/vault > lanes.json; an undeclared name never crosses,
  so KI-031's allowlist is intact). Per-call defaults merge *under* the
  model's arguments. The lane repos and sys1's TOML are never rewritten. The
  defaults follow the bundled MCP schemas exactly: `sem1_embed`
  (provider/model/endpoint), `sdm1_classify` where the provider rides the
  **`model`** argument, and `gen1_imagine` only — the bundled gen1 server
  exposes no speak/transcribe tool. Check a lane's `tools/list` before
  designing a default for it.
- **Roles** — `roles::catalog` names broken files instead of stderr-skipping
  them; `roles::write` keeps unknown frontmatter keys.
- **Vault/secrets** — write-only fields (send once, clear, never in a read or
  in `face` state); seal into the vault when the process holds
  `FLEET_VAULT_PASSPHRASE`, else one atomic `.env` line. Bots gained the
  Telegram token and a paired-devices card over the existing `/api/access`.
- **Spend/runtime** — `Usd` and `CountTo` setting kinds that can only tighten
  the compiled ceiling; the binary loads `fleet.env` at startup (skips
  `*_KEY/_TOKEN/_SECRET`, never overrides a set variable) so those writes take
  effect at the next launch.

**Three limits worth remembering:** resolution is per tier, not the run's
route (the orchestrator walks `hard` and calibration can demote); that column
does not apply calibration. `lane_arguments` merges defaults for a
`config_tools` field. `fleet.env` is loaded at startup, so its knobs apply
after a restart. Fleet has **no mixture-of-agents, capability chains, or
picker profiles** — those are the zcode kit's concepts, and the surface
deliberately does not fake them.

Not built and named as follow-ups: improve-proposal queue and graph editing
outside Settings. Related: [[fleet-second-brain-is-a-tool-plugin]] (built in
the same tree by a parallel session and deliberately excluded from adfd353),
[[fleet-vault-credentials]], [[gui-client-verified-by-a-vm-harness]].

**How the vault card was verified, and what cost time.** The vault's KDF is
210k PBKDF2 iterations, so unlock/init/seal requests take seconds. The
server test's default read budget is **3 s**, so `a_settings_write…`
failed under the full suite (passed alone) until the vault requests and the
`/api/settings` GETs after them were given `post_within`/`get_within` with
a 30 s budget — the same lesson as
[[a-test-that-passes-on-an-idle-machine-has-not-been-tested]], now for a
deliberately-expensive crypto path rather than a race. Live audit shape
that worked: locked card → unlock → create → three names appear → drop one
plaintext → lock; assert no secret string in `/api/settings`.
