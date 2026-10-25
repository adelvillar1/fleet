---
status: draft
created: 2026-10-23
updated: 2026-10-23
slug: capability-pitch
---

# Plan: the capability pitch — onboarding as just-in-time configuration

## Context

The owner's ruling, 2026-10-23: an onboarding screen with the composer
interface that says "hey, if you set up a system-one model like jev,
julia1, d1, gliner, glide, typellm, you could get this. If you give me
the API key I can set it up for you" — and the same for the other
lanes.

This is also the answer to the settings-abstraction search: the board
stays shaped like the five questions for the owner who goes looking;
the front door gains configuration **pitched as capability, just in
time** — work now; fleet names, at the moment it matters, what would
make it better, and does the setup when the key is handed over. The
anti-Hermes guard is built into the shape: the composer stays the
interface, the offer is one sentence, the action is one paste, and "I
can set it up for you" means the agent writes through its own writers.

Every mechanism exists in the tree, verified 2026-10-23:

- The catalog knows every roster provider's `key_env`
  (`src/route.rs:247` onward) and reports each name
  present/unset with its source (`catalog_env_names`,
  `src/settings.rs:2034`; `key_present` on the roster entries).
- The vault has first-run onboarding that creates, seals, and migrates
  the plaintext `.env` entries — "give me the key" is one paste
  through a writer that exists.
- The tiers resolve live: the route bar flips from "no route yet" to a
  judged route the moment the key lands, so the promise is kept in the
  same surface the promise was made in.
- The MCP registry proved the pattern: scan, offer, one action —
  extended here from servers to intelligence.
- The empty state (`paintEmpty`) and the offers grid (`paintOffers`,
  the usage-boosted order) are the display grammar; the OFFERS
  constant carries the existing "try this command" offers.

## What we should do

**The capability pitch: offers over the roster's absent keys, ordered
by what they unlock, written in the offers' grammar.** One new source
of offers beside the static OFFERS list — computed from the catalog,
not hardcoded, so a provider the roster no longer carries never
pitches, and a machine with everything configured pitches nothing.

**The pitch names the capability, not the vendor.** Not "add
LIQUID_API_KEY" but "a system-one model judges every task before it
runs — routing, delegation, even a second opinion on plans; give me
the key and I am set up in one paste." The copy is per-provider,
written once, and the order is the unlock order: the judge first
(because it upgrades everything else), then the composed ask (blend1),
then the media and live-web lanes, then the reserve. The local lane
never asks for a key — it offers to check whether its server is up.

**The action is one paste, and the paste is the composer's.** The
offer fills the composer with a sentence ("here is the key for jev:
···"), because the owner is already typing there; the send path
detects a key-shaped paste, routes it through the vault's seal (or the
home `.env`, per the existing credential ruling), and the catalog
re-read flips the offer off. No form, no wizard, no settings visit.

**The screen is the front door, once.** The offers grid already lives
there and already learns (usage-boosted); the pitch rides the same
grid, above the command offers, and stops appearing once configured.
A returning owner with everything set sees the front door exactly as
today.

**One decision to confirm before increment 1** (recommendation
first): **where pasted keys land.** Recommendation: the vault when a
passphrase exists, the home `.env` otherwise — the exact rule the
vault's own onboarding already implements, reused verbatim rather than
invented. The alternative — always `.env` — is simpler but splits the
credential story the trust trio established.

## Approach

**D1 — the pitch is derived, never hardcoded.** The offers compute
from the catalog's `key_present`/`source` per roster entry; the copy
per provider is a static table keyed by provider id (the words are
editorial, the presence is live). A provider absent from the roster
never pitches; a present key never pitches; an unset one pitches in
unlock order (judge → composed → media → web → reserve), the order the
routing table itself would use them.

**D2 — the paste is a credential write through the existing writers.**
The send path recognizes a message carrying a key for a pitched
`key_env` (the offer's insert makes the association explicit: "the key
for jev: <paste>"), and routes to the vault seal / `.env` write the
trust trio already owns. The composer then reads the catalog again;
the offer disappears, the route bar resolves, and the strip gains a
credential line — the same trace any secret write leaves.

**D3 — the empty state carries the pitch only for what is missing.**
A fresh machine sees the judge pitch first; a fully configured machine
sees today's front door unchanged. The pitch is never a modal, never a
wizard, never blocks the composer — it is one offer card among the
commands, ranked first while it is true.

**D4 — the local lane's offer is a health check, not a key.** sem1's
embedding server is LaunchAgent-owned; its pitch is "the local
embedding lane is asleep — want me to check it?" riding the doctor's
probe, not a credential form.

## Increments

Each increment closes green — the four-command gate, exit codes read
directly — with a harness case and a live verification.

1. **The derived capability offers.** The provider-pitch table, the
   catalog-driven offer source in `paintOffers`' grammar, the unlock
   order, the gone-when-configured rule. Harness case over stubbed
   catalogs (fresh machine: judge pitches first; configured machine:
   no capability offers; one key set: exactly the rest pitch).
2. **The paste path.** The composer recognition, the vault/.env
   routing per the confirmed decision, the catalog re-read, the offer
   clearing, the strip's credential line. Harness case for both
   landing paths + the refusal shape (a paste that matches no pitched
   name is just a message).
3. **The local-lane health offer.** The doctor-probe-backed pitch and
   its action. Harness case.

## Acceptance criteria

- [ ] C1 — a machine with no roster keys sees capability offers in
  unlock order (judge first), each naming the capability the key
  unlocks in the offers' grammar; a machine with every key sees none;
  a machine with some sees exactly the missing ones. Pinned by a
  harness case over stubbed catalogs.
- [ ] C2 — accepting an offer fills the composer with a sentence that
  names the provider; sending a message carrying that provider's key
  lands the key through the vault (passphrase set) or the home `.env`
  (none), the catalog re-reads, the offer clears, and the route bar
  resolves. The key value never appears in the session log. Pinned by
  a harness case for both landing paths.
- [ ] C3 — a message that matches no pitched provider is sent as an
  ordinary task, and a paste whose key shape matches nothing pitches
  nothing. Pinned by the same case.
- [ ] C4 — the local embedding lane's offer is a health check backed
  by the doctor's probe, with no credential form. Pinned by a harness
  case.
- [ ] C5 — the gate is green: `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  exit codes read on the cargo invocation itself.

## Files to be touched

- `src/gui/assets/app.js` — the derived offer source beside OFFERS,
  the composer's key recognition and routing, the local-lane offer
- `src/gui/assets/app.css` — the capability offer's emphasis (ranked
  first while true), nothing structural
- `src/gui/bridge.rs` — one read: the pitched providers with their
  `key_env` and presence, if the catalog document does not already
  carry enough (it does: `env_names` + roster `key_present`; the plan
  expects no bridge change, and says so rather than inventing one)
- `tests/face/harness.mjs` — the three cases

## Out of scope

- A wizard, a modal, a settings visit, or any multi-field form — the
  anti-Hermes guard is the design.
- Any new credential store or transport; the vault and `.env` writers
  are the only landing places.
- Pitching lane *tools* (the model-facing side) — the lanes' tools
  already declare; this pitches only the keys and health that gates
  them.
- The settings board's reorganization — the five-questions shape is a
  separate future plan; this plan only adds the front door's pitch.

## Verification

- C1–C4: harness cases over stubbed catalog documents (fresh,
  partial, full, local-down) driving the offers and the composer the
  way a pointer and a paste would.
- Live pass on the loopback face: the real catalog's pitch order read
  against this machine's actual key state, one paste landed through
  the confirmed path, the offer cleared, the route bar resolved.
- C5: the full gate, exit codes read directly.

## Audited 2026-10-25 — against commit history

Fully pending: the only commit touching this work is this plan's own
drafting (`6be8096`). No capability-offer surface exists in the tree —
the bridge's `capability` hits are the pairing/channel capability, an
unrelated sense. C1's no-roster-keys offer, C2's accept-fills-composer,
and C4's health-check-backed embedder offer are all unbuilt; C4's
health check now rides the probe machinery KI-296's retry hardened,
which makes the offer's honesty easier than when this was drafted.

## Linked artifacts

- `docs/features/gui.md` — the home briefing section carries the
  capability pitch
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation/front-door section
  gains the just-in-time configuration sentence
- `docs/known-issues.md` — KI-308's `sk-` constraint applies to the
  copy: the pitch names key *names*, never key shapes

## Risks

- **A paste misread as a key is a lost task.** The recognition is
  keyed to the pitched provider's name appearing in the message
  beside a long token — the offer's own insert makes the association
  explicit — and the fallback is the ordinary send, visible in the
  transcript, correctable by edit-and-resend.
- **Key-shaped content in the composer.** The draft is local
  storage as always; the value enters the log only if the run is
  actually sent, which is why the recognition must route before the
  send, not after.
- **The pitch order is editorial.** The unlock order is the routing
  table's own, so the copy's order can be wrong only where the table
  is; the calibration stream will say so.
