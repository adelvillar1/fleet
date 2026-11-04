---
name: a-setting-nobody-asked-for-is-onboarding-debt
description: "feedback 2026-11-02 — the owner's vault had 10 sealed entries and no one had ever been asked for a passphrase; the fix was not a recovery path but a MISSING ONBOARDING STEP ("there is nothing in that vault that I cannot recover, but the onboarding or lack of it is what matters")"
type: feedback
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

Late in the 2026-11-02 settings session the owner said, of the vault:
*"I have never set a passphrase to begin with, so it's a bit of a catch
22… how do we fix that?"*, and after I laid out the reset path:
*"there is nothing in that vault that I cannot recover, but the onboarding
or lack of it is what matters."*

**Why:** I had spent two turns answering "can you add them to the vault?"
with *no, no passphrase* — a true statement about the DATA and a useless
one about the PRODUCT. The real defect was structural: fleet shipped a
vault with a first-run form on its own card, but nothing in the onboarding
ever asked for a passphrase, and nothing ever told a settled machine that
its keys were sitting in `.env` in the clear. So `vault.json` accumulated
ten entries under a passphrase nobody chose to remember, five names stayed
plaintext, and the surface that could have asked (the trust section) was
never reached. The owner's frame is the constitutional one — the standing
test is *"does the owner need to know this knob exists?"*, and a knob whose
question is only asked on a screen nobody visits fails it. A config that
exists but was never introduced is a first-run bug, not a user error, and
"the app has a card for it" is not onboarding.

**How to apply:** when a feature's flow dead-ends on something the owner
says they never supplied, stop looking for a recovery path and ask **which
onboarding step would have asked that question** — then add it, with the
platform's own rules ([[onboarding-is-a-door-not-a-wizard]]): benefit
before mechanism, one action, a real "not now" that names the cost, and a
proof. Built the same turn as `5a65a6a`: `onboard.rs` gained
`STEP_VAULT = "vault"` between `first-llm` and `lanes`, whose narration is
one benefit sentence ("the account keys this home holds are readable by
anything on this machine until they are sealed") and whose skip names the
cost ("the keys stay in .env in the clear"). Its proof is REAL state, not a
tick: `settings::vault_state(home)` → done only when the vault holds
credentials **and** `plaintext[].sealed == false` is empty, the evidence
reading `10 sealed · 5 still plaintext` on a half-migrated home and
`N credentials sealed` when clean — so a credential added after the seal
turns the step back on by itself, and the card's action deep-links to the
vault's own form (`#/settings/trust`, then clicks the `vault` row) because
a step that lands on a section rather than its form is another half-ask.
Pinned by `the_vault_step_is_done_only_when_nothing_is_left_in_the_clear`
(absent vault → false and empty evidence; `vault_onboard_with` sealing two
names → true; a third plaintext line → false naming `1 still plaintext`).
The honest limit to state aloud: the step can tell that a vault exists and
that plaintext remains, never whether the owner *knows* the passphrase.
Related: [[fleet-vault-credentials]],
[[fleet-keys-are-already-in-place-and-the-vault-needs-the-owners-passphrase]],
[[onboarding-is-a-door-not-a-wizard]], [[fleet-anti-hermes-constitution]].
