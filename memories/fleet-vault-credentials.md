---
name: fleet-vault-credentials
description: "fleet's credentials live in an AES-256-GCM vault at <FLEET_HOME>/vault.json keyed by FLEET_VAULT_PASSPHRASE; resolution is env → vault → plaintext .env. 2026-10-19 audit — the router could not read the vault (KI-250); the vault now lays over the dotenv at every construction site via home::read_dotenv_with_vault + Vault::open_all, so a sealed roster key answers everywhere a plaintext key did"
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

Owner directive 2026-10-17: "supplied keys and passwords are never printed and
they are stored securely on an encrypted vault or hashed." Shipped as
**`d3fff77`** (`src/vault.rs`, plus `fleet vault` in cli.rs/main.rs).

**Encryption, not hashing** — every secret fleet holds must be *recoverable*,
because an API key is sent to a provider. Hashing is right only for a secret
that is *verified* rather than used, and fleet has none; when one appears it
gets its own module rather than bending this one. Say this plainly when the
owner asks for "encrypted or hashed" — the "or hashed" half covers a case that
does not exist yet.

**The shape.** `<FLEET_HOME>/vault.json` carries a PBKDF2 salt, an iteration
count, and one base64 entry per secret laid out as `nonce(12) || tag(16) ||
ciphertext` — a fresh random nonce per write, so two seals of the same secret
are never the same bytes. The key is PBKDF2-HMAC-SHA256 over the passphrase.
AES-256, GCM and PBKDF2 are hand-rolled (no new crate, per the crate
contract) — see [[verify-crypto-against-published-vectors]] for what that
cost and what it caught.

**The CLI:** `fleet vault init` (creates it and migrates the plaintext
`.env` into it), `set NAME VALUE` (or `--value` so the secret is not a bare
positional), `get NAME`, `rm NAME`, `list`. **`list` prints names and never
values**; `get` is the one deliberate exception, because a secret has to be
able to reach a pipe. A wrong passphrase refuses without saying whether the
entry exists, and a tampered entry does not open.

**Resolution order at runtime — the order is the design.** The process
environment wins (an operator's override), then the vault, then the plaintext
`.env`. The vault sits *between* the two because that is the migration order
the owner wants: an encrypted entry is preferred over a plaintext line, so
migrating a home stops the plaintext copy from being the one that answers. A
home with no vault — or a passphrase that does not open it — falls through to
the plaintext leg rather than failing the run, because a run that cannot
start is worse than a run reading the key the owner already had there.

**2026-10-19 audit — the router could not read the vault (KI-250, closed
`5dbc996`, gate 1122).** The resolution order above was true only of
main.rs's `setting()` — three names (the DeepSeek key and base URL,
Tinyfish's). The router's `route::resolve_key` went through
`home::resolve_setting`, which is env → dotenv and stops, so every
roster key `fleet vault init` migrates would silently stop resolving the
moment the owner deleted the plaintext lines the init output told them
to delete. The to_pptx pattern: the feature existed, its largest
consumer was not wired. **The fix lays the vault over the dotenv at
every construction site:** `home::read_dotenv_with_vault(home,
passphrase)` opens the vault with one derivation (`Vault::open_all` —
one KDF per run, not one per entry) and sealed names answer over stale
plaintext lines, so init's leave-the-lines migration is honest — the
sealed copy is the one that answers and deleting the plaintext is
housekeeping. The GUI's agent boots overlay too, so sealed roster keys
reach the router from the browser face. Pinned by
`the_vault_lays_over_the_dotenv_and_a_missing_passphrase_leaves_it_plain`
and `open_all_opens_everything_with_one_derivation_and_nothing_with_a_wrong_one`
(gate green 1122). The "never printed" half audited clean: doctor masks
to the last four, GUI settings shows presence only, the pane refuses
`.env` by name, `no_payload_carries_a_credential` scans the payloads,
and `get` printing to stdout is the one deliberate exception (piping).

**The "every construction site" claim was short of true (2026-10-25,
c1e3caf).** The ledger's second pass re-verified the closed claim against
the code: the GUI's boots overlaid, but four headless paths still read the
plain dotenv — `sys1_judges`' key gate, `prepare_with_pin`'s roster
resolution, the scheduled runs' keys, and the mid-run re-route — so a
machine that sealed its keys answered keyless from the CLI while the face
served. All four now read `read_dotenv_with_process_vault`, pinned end to
end by `a_sealed_credential_answers_the_router_through_the_vault_overlay`
(seal a key, drop the plaintext line, unlock the process, and the roster's
own resolution marks the candidate usable). The lesson is
[[ki-ledger-second-pass-method]]: a closed row's status is a claim, and
"every construction site" is exactly the kind of claim a partial fix
leaves standing.

**The GUI writes secrets through this vault (adfd353, 2026-10-20).**
`settings::set_secret` is the one credential writer: a credential-shaped name
(`*_KEY|*_TOKEN|*_SECRET`) is required, the value is sealed into the vault when
the process holds `FLEET_VAULT_PASSPHRASE` (creating the vault on the first
seal), and otherwise written as one atomic `.env` line preserving every other
line. `remove_secret` removes from whichever store holds it. The page's fields
are write-only — the value is sent once, the input clears, and no read carries
it (`/api/settings`, `/api/channels` both proved clean in the live audit).
`secret_state` deliberately does **not** count `fleet.env` as a secret source:
the startup loader skips credential-shaped names and the channel resolver
never reads one from there, so a token hand-written into `fleet.env` is not a
token any reader would answer.

**Three named limits, disclosed in the commit rather than papered over:**
1. The passphrase rides `FLEET_VAULT_PASSPHRASE`, so it is only as safe as the
   environment it sits in, and an unattended run (cron, the GUI) needs it
   exported somewhere. The stronger design is the macOS Keychain via the
   `security` CLI (no new crate) — not built, because it is macOS-only and
   this is the portable baseline.
2. PBKDF2 is 210,000 iterations, not OWASP's 600,000, because the derivation
   runs on every invocation that needs a key and a CLI that takes two seconds
   to start is a CLI that gets worked around. It is a named knob.
3. `init` migrates but does not delete the plaintext `.env` — removing the
   lines the owner has sealed is the one step that cannot be done for them.

**A key can exist on the machine and still not resolve (2026-10-21).** The
document-attachment client reads `NACE_API_KEY`/`DREX_API_KEY` through fleet's
readers; the owner answered a refusal with "there is, in sys1" — the credential
lives in **sys1's** env file, which fleet does not read. That is the own-home
rule working as designed, and the fix is a copy into `<FLEET_HOME>/.env` or the
vault, not a cross-store read. Before saying "no key", say **which store each
name was read from** — "not in fleet's env/vault/.env" is a different fact from
"not on this machine", and only the first is true here.

**The passphrase is a process unlock, not just an env var (4fe9d41,
2026-10-21).** The GUI's vault card shipped in `adfd353` as a read-only
status line, and the owner came back with "the credentials vault has not
surface to interact with it" — on a face launched without a passphrase the
card listed names and offered nothing at all, so the vault was unusable
from the browser. The fix: `home` owns a `VaultUnlock` state (`Unset` →
read `FLEET_VAULT_PASSPHRASE`; `Unlocked(passphrase)` → hold in memory;
`Locked` → ignore the env var), and **every vault reader in the process**
goes through `home::vault_passphrase()` — main.rs's setting resolution, the
router's key resolution, lanes, channels, the related index, `set_secret`.
`read_dotenv_with_process_vault` is the one overlay helper. That is what
makes an in-place unlock usable by the *next run*, not just by the card.
The card now unlocks (wrong passphrase refused by name via `vault_verify`,
which opens the existing entries to check), creates the vault and migrates
the credential-shaped `.env` names, seals one plaintext entry at a time
server-side (`vault_seal_env` — the value never reaches the page), drops a
plaintext line (`vault-remove-env`), seals/removes an entry, and locks
again. `vault_init_with`/`vault_seal_env_with` take an explicit passphrase
so tests never touch the process global; `vault-init` refuses to replace a
non-empty vault.

**Write-only means the card cannot show you the fixed form.** After
locking, the card shows the unlock form and the sealed names only — the
create/seal/drop actions disappear with the unlock. That is correct (a
locked process cannot write) but it reads as "the surface vanished" in a
screenshot; capture the unlocked state when verifying the card.

**First run, and the forgotten passphrase (1d6f946, 2026-10-21).** The
owner's next question was "I have no idea what the passphrase is — we tested
it during development but not onboarded the feature." The check first: on this
home `vault.json` did **not exist** and all ten credential-shaped names were
still plaintext in `~/.fleet/.env` (StepFun, Z.ai, xiaomi, token-plan,
DeepSeek, Fastino, TypeSafe, Ollama, TinyFish, the router token) — the dev
passphrase only ever lived in scratch homes, so nothing was locked and
nothing needed recovering. The real gap was **onboarding**, now shipped:
with no vault the card is the first-run form (the `.env` names that will be
sealed, a passphrase + confirmation, and a plain *there is no recovery —
store it before sealing anything*), and one action creates the vault,
migrates those names, and unlocks the process (`vault_onboard`). For an
existing vault, a forgotten passphrase has its own path: `vault_reset`
archives the old file as `vault.json.bak-<stamp>` — never deletes it — and
onboards a new vault over the plaintext `.env` copies. Never invent or
generate the owner's passphrase for them: it is the one secret only they can
hold, and the card hands them the moment to choose it.

**The vault is its own destination (fea955c, 2026-10-21).** The onboarding
form shipped inside Settings and the owner still said "there should be a vault
settings section so that a vault can be onboarded" — see
[[a-top-level-surface-needs-an-address]]. It is now `#/vault` under **System**
in the rail and the palette (the same `vaultCard()` renders both, with an
optional card id), showing the first-run form, the sealed names, and the
`.env` ledger. On this home at the time: no `vault.json`, ten credential-shaped
names still plaintext. That is the expected pre-onboarding state — do not read
it as a broken vault.

**2026-11-02 — the missing step, not a missing recovery.** The owner's real
home carried `vault.json` with **10 sealed entries** under a passphrase
nobody remembered choosing, while 5 credential-shaped names sat plaintext
in `.env`. The lesson (the owner's own words): *"there is nothing in that
vault that I cannot recover, but the onboarding or lack of it is what
matters."* Onboarding now has a `vault` step (`src/onboard.rs`,
`STEP_VAULT`, commit `5a65a6a`) whose proof is `settings::vault_state`:
done only when the vault holds credentials **and** no `.env` credential is
still `sealed: false`, so it re-arms itself when a new key is added. On
this home it reads done=false, evidence `10 sealed · 5 still plaintext` —
the truth, and the same sentence a fresh home gets with no vault.
`vault_onboard_with(home, passphrase)` seals every credential-shaped
`.env` line (all 15 here). See
[[a-setting-nobody-asked-for-is-onboarding-debt]]; the recovery path for a
forgotten passphrase (`vault_reset` archives `vault.json.bak-<stamp>`) is
unchanged and must stay the OWNER's press.

**How to apply:** when a key does not resolve, check the order — env, then
vault (is `FLEET_VAULT_PASSPHRASE` set, or has this process unlocked?),
then `.env`. When adding a
secret-consuming surface, read it through the same `setting()` helper in
main.rs so it inherits the order rather than re-deriving it. Related:
[[verify-crypto-against-published-vectors]],
[[fleet-roster-migrated-from-router-kit]], [[gui-revamp-browser-face-decision]].
