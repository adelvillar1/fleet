---
name: api-key-env-file-map
description: "Where the estate's API keys actually live (verified 2026-10-27): OLLAMA_API_KEY in ~/.fleet/.env (fleet's own env — route.rs:357 reads it there); TYPESAFE/FASTINO/LIQUID/DREX keys in ~/.config/dev-decisions/env; SCALEDOWN/TINYFISH/BLEND1_BEARER_TOKEN in ~/.config/blend1/env; sys1's own ~/.config/sys1/env carries none of them."
type: reference
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

The model-estate API keys are split across four files — verified 2026-10-27 while wiring the clef provider (listing KEYS only, never values):

- **`~/.fleet/.env`** — the fleet home's own dotenv: `STEPFUN_API_KEY`, `ZAI_CODING_API_KEY`, `XIAOMI_MIMO_API_KEY`, `TOKEN_PLAN_API_KEY`, `DEEPSEEK_API_KEY`, `FASTINO_API_KEY`, `TYPESAFE_API_KEY`, `AGNOSTIC_ROUTER_TOKEN`, **`OLLAMA_API_KEY`**, `TINYFISH_API_KEY`.
- **`~/.config/dev-decisions/env`** — sys1's hosted decision-model keys: `TYPESAFE_API_KEY`, `FASTINO_API_KEY`, `LIQUID_API_KEY`, `DREX_API_KEY`.
- **`~/.config/blend1/env`** — `SCALEDOWN_API_KEY`, `TINYFISH_API_KEY`, `BLEND1_BEARER_TOKEN` (and `BLEND1_PROJECT`-class settings).
- **`~/.config/sys1/env`** — none of the above; sys1's providers read the process env (its own env file adds none of the decision keys here).

**Re-verified 2026-11-01, with a new source**: `~/Projects/sys1/.env` (the repo's own dotenv, not `~/.config/sys1/env`) carries `DREX_API_KEY`, `SUPERSONIC_API_KEY`, `FASTINO_API_KEY`, `TYPESAFE_API_KEY`, `LIQUID_API_KEY`. DREX and SUPERSONIC were imported into `~/.fleet/.env` (fleet uses both: `src/lanes.rs` declares them as lane choices), giving that file fifteen names — the ten above plus `LIQUID_API_KEY`, `DREX_API_KEY`, `SUPERSONIC_API_KEY`, `FLEET_RELAY_JOIN_SECRET`, `FLEET_BOT_RELAY_JOIN_SECRET` (the two relay proofs were written 2026-11-01 by the rotation — see [[relay-join-secret-rotation-2026-11-01]]; they are also in the com.fleet.gui plist's EnvironmentVariables, which is what launchd exports). The two router kits' `roster.json` files carry **no keys** (no `key_env` field at all — six provider entries on the 2026-11-01 re-check: deepseek, xiaomi-mimo, token-plan, stepfun, zai-coding-plan, blend1), so "the keys are in the router kit" is never true. The lane repos' own dotenvs: `~/Projects/sys1/.env` carries DREX/SUPERSONIC/FASTINO/TYPESAFE/LIQUID (the only one with files); `~/Projects/blend1`, `sem1`, `sdm1`, `gen1` carry none; `~/Projects/dev-decisions` carries none. `blend1` as a roster provider needs `BLEND1_BEARER_TOKEN`, which resolves from `~/.config/blend1/env` for blend1 itself but not for fleet's home — the real-home catalog showed `blend1 key_present: false`, and it is a LANE provider, keyless by design for the lane path. `~/.fleet/.env` is now `600` (it was group-readable). See [[fleet-keys-are-already-in-place-and-the-vault-needs-the-owners-passphrase]].

**Sub-2026-11-01 addendum, verified on the real home through the running face**: the reload of every compiled provider (zai 11, xiaomi 6, stepfun 6, token-plan 20, deepseek 2, liquid 5 after the base fix, ollama-cloud 25 — 76 in total) ran green against every key FROM `~/.fleet/.env` alone, with no vault unlock; only deepseek's listing declares settings (1M window / 393k output). So the fleet home's dotenv is sufficient for the whole roster, and a missing vault seal does not block discovery.

**2026-11-02:** a key pasted into a chat (the owner pasted a live
`liquid_…` bearer beside a curl example) is compromised by publication —
nothing in a repo or the vault holds the literal, but the transcript does.
Never write a pasted key into a memory or a file, probe with the stored
dotenv value instead, and name the rotation path to the owner. See
[[fleet-keys-are-already-in-place-and-the-vault-needs-the-owners-passphrase]].

Practical consequences: a sys1 provider whose `key_env` is `OLLAMA_API_KEY` will not arm from the sys1/blend1/dev-decisions env files — source it from `~/.fleet/.env` (as `ab_clef_shadow.py`'s `arm_key()` does); blend1's env.py resolves blend1 → sdm1 → dev-decisions, so the decision keys reach blend1 through the dev-decisions file; fleet's server process gets `OLLAMA_API_KEY` from its own home's dotenv. Related: [[fleet-vault-credentials]], [[clef-decision-model-assessment-2026-10-27]].
