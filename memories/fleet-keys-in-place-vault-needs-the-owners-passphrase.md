---
name: fleet-keys-in-place-vault-needs-the-owners-passphrase
description: "project/reference 2026-11-01 — every keyed fleet provider already resolves on the real home; the kits carry no keys (roster names only), the lane keys live in sys1/.env (DREX, SUPERSONIC imported into ~/.fleet/.env, modes tightened to 600); sealing into the vault is blocked without FLEET_VAULT_PASSPHRASE, which exists in no process here (the source slug `fleet-keys-are-already-in-place-and-the-vault-needs-the-owners-passphrase` exceeded the name grammar's 64 characters; this legal name carries it)"
type: feedback
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

The owner's 2026-11-01 ask — *"make sure that keys are imported…
they are working so there is no point for me to re-add them again.
when you find them add them to the vault"* — audited by NAME only
(never a value):

- **The real home needs nothing.** Its `/api/settings` read reports
  `key_present: true` for zai-coding-plan, xiaomi-mimo, stepfun,
  token-plan, deepseek, liquid and ollama-cloud (23 models); only
  blend1 is keyless, by design (a vendored lane). So there is no
  re-entry problem on this machine.
- **The kits carry no keys**: `agnostic-router-kit/roster.json` and
  `zcode-router-kit/roster.json` are `{providers: {...}}` maps with no
  `key_env` field at all; dev-decisions has no env file.
- **The lane keys live in `~/Projects/sys1/.env`**:
  `DREX_API_KEY`, `SUPERSONIC_API_KEY` (plus FASTINO/TYPESAFE/LIQUID
  which fleet already had). I imported DREX + SUPERSONIC into
  `~/.fleet/.env` and tightened `~/.fleet/.env` and `vault.json` from
  `644` to `600` — the key file was group-readable.
- **The vault could NOT be written.** It is sealed by
  `FLEET_VAULT_PASSPHRASE`; `fleet vault set` refuses when it is
  unset, and no process here (my shell, either face) carries it. Five
  names live only in the dotenv: `LIQUID_API_KEY`, `DREX_API_KEY`,
  `SUPERSONIC_API_KEY`, `FLEET_RELAY_JOIN_SECRET`,
  `FLEET_BOT_RELAY_JOIN_SECRET`. Sealing them needs the owner's
  passphrase exported first.

**2026-11-02, after the catalogs were really discovered:** the picture
holds — the real home's `/api/settings` reports `key_present: true` for
all seven keyed providers (blend1 is the lane); the import is exactly
the two sys1 lane keys already written to `~/.fleet/.env`. Everything
still runs from the dotenv; the vault is still unsealed here.

**A key pasted into the chat is a key to rotate (2026-11-02).** The owner
pasted a live `liquid_…` bearer in plaintext beside a curl example; it is now
in that conversation's transcript (and in shell history if they ran the curl
themselves) even though nothing in the repo or the vault holds the literal.
Say so plainly and give the rotation path (`fleet vault set LIQUID_API_KEY
--value …` plus the `~/.fleet/.env` line) rather than quietly using it — and
use the stored key from the dotenv for any probe, never re-paste theirs.

**"Can you add them to the vault?" — asked twice, and twice the honest
answer is no without the passphrase (2026-11-02).** The second time the
owner had already been told so; the check to re-run is cheap and
conclusive: `grep -q FLEET_VAULT_PASSPHRASE` across `~/.zshrc`,
`~/.zprofile`, `~/.zshenv`, `~/.fleet/fleet.env`, `~/.fleet/.env` and
`~/Library/LaunchAgents/*.plist` (not found anywhere), and the live
read's `vault.passphrase: false` (the running face is locked — if it
ever reads `true`, the process holds it and the seal can be done).
`/api/settings` already generates the exact work list: every
`vault.plaintext[]` entry with `sealed: false` — on this home the same
five names — so answer with that list, not a generic refusal. The app
owns the flow (trust → the vault: unlock holds the passphrase in memory,
then each .env row has `seal it`, and once sealed `drop the .env line`),
so the owner either presses five rows or says "it is unlocked" and the
five go through `/api/settings` (`vault-seal-env`). Never start a new
vault to get around a missing passphrase — `vault-reset` archives the
current one and its entries become unreadable.

**"I have never set a passphrase to begin with" (2026-11-02) — the real
answer is: you did, at some point, or an onboarding did.** The vault at
`~/.fleet/vault.json` holds **10 sealed entries**, so it was created and
keyed; there is no way to derive or recover that passphrase, and I must
never try. The catch is real but the app already ships the exit, and it
is safe on this machine because **nothing lives only in the vault** —
all 10 sealed names also resolve from `~/.fleet/.env`:

- Settings → trust → the vault → *"I do not know the passphrase"* →
  choose a new one → **start a new vault** (`vault-reset`).
- That RENAMES the old `vault.json` to `vault.json.bak-<stamp>` (archived,
  never deleted) and creates a new vault keyed by the new passphrase,
  **sealing every credential-shaped `.env` line into it** (all 15 here,
  including the five that were plaintext). Then `drop the .env line` per
  name removes the plaintext copies one at a time.
- Warn with it: the new passphrase becomes the only way in (same catch
  otherwise), and the launch agent's plist still holds the two relay
  join secrets in plaintext, so sealing them does not remove the copy the
  agent passes at launch.
- Present it as the owner's press, not mine: I cannot key a vault the
  owner did not key, and I should not perform a destructive-seeming
  reset on their behalf.

**How to apply:** on any "reuse my credentials" ask, first read which
names already resolve (`/api/settings` → `catalog.providers[*].key_present`)
and say so — the import is usually small. Inventory `~/Projects/*/` env
files and the two router kits before concluding a key is missing, and
report names only. Never write a vault entry without the owner's
passphrase, and say plainly that the dotenv is the working store
already. See [[fleet-vault-credentials]],
[[api-key-env-file-map]], [[settings-ux-plan-2026-11-01]].
