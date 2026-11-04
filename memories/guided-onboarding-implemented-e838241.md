---
name: guided-onboarding-implemented-e838241
description: "project 2026-10-29 — docs/plans/2026-10-29-guided-onboarding.md was examined, re-designed around the door, given the house design language, then IMPLEMENTED as e838241 and completed by 8419da3 after the audit found two criteria marked met early (the cards' visible proof, the strategy preview): src/onboard.rs (six derived steps, narration, probes, preview, seeded fact), `fleet onboard`, /api/onboard + the door + the prove op, the capability cards; gate green, harness 375/41, the door and the preview verified…"
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

2026-10-29: the owner drafted **docs/plans/2026-10-29-guided-onboarding.md**
in a separate session (guided first-run: first LLM key → the five lanes in
lane order → the projects folder → the GitHub PAT → the model roster, one
sentence "speak, measure, remember, prove, choose", a derived-content rule,
writes only through existing seams, a two-rung proof ladder, skip-with-named-
consequence, resumability from doctor's checks) and asked me to examine it,
then re-design it frictionless, then make it speak the console's design
language, then **"/goal implement plan as designed"**.

**What verified clean in the draft:** `set_secret` refuses
non-credential-shaped names and the GUI's settings `"secret"` op already
calls it; LANES carries per-lane passthrough key env names;
CONTEXT_WINDOWS + the undeclared-model floor back the roster step;
KI-074/KI-087 are closed rows whose fix (the memory line names the adopted
root) is what the projects step extends; `TOWER_LOG_PROJECTS_DIR` with
`~/Projects` is the tower's own root; the live mcp.json's github card
carries `GITHUB_PERSONAL_ACCESS_TOKEN` as an env NAME.

**Six findings, all recorded in the plan's own `Examined 2026-10-29`
section (finding 2 marked SUPERSEDED rather than rewritten):**
1. `fleet install` was missing — the lanes' live verify spawns venvs only
   install builds.
2. The GUI cards were promised in Files and never specified.
3. The completion probe's "one recalled fact" is unmeetable on an empty
   corpus.
4. "Flash tier by default" misnames the chain — a one-key home WALKS TO
   its key through the failover tail.
5. **There is no first-run at all**: a keyless home renders the full
   power-user welcome and the first send fails IN THE TRANSCRIPT with
   `routing config: <error>`; the CLI is worse (`route::preflight` prints
   every acceptable `*_API_KEY` name). A chat turn needs ONE model key and
   no lanes (judgment fails open) and `set_secret` with NO passphrase
   writes the plaintext `.env` line — so install and the vault passphrase
   came OFF the blocking path (this corrected finding 1's first fix).
6. No house design language — every element now maps to an existing
   primitive (`.empty`, `.empty-eyebrow`, `.empty h1`, `.offer`,
   the product's own `.composer`, `.method`, `.notice`) with a
   may-not-introduce list.
The same pass caught the plan contradicting itself ("not a wizard" vs the
word used six times) — **"wizard" retired everywhere**; it names a modal
metaphor the design rejects.

**What landed (e838241, 22 files):**
- `src/onboard.rs` + `src/onboard/tests.rs`: STEPS = home, first-llm, lanes,
  projects, github, roster; `Facts::read` derives everything (keyed
  candidates, fan-in tail via `Billing::Payg`, `boards::resolve_sources`
  tower root, LANES names+roles, the compaction floor, the GitHub card's
  own token name); `state`/`first_incomplete`/`to_json`; `skip` and
  `confirm` ride the calibration stream's EXISTING `lifecycle` rows;
  `remember_onboarding` writes the fact the completion probe reads back;
  `completion` = four reads (credential gate, a route row today, the
  recalled fact, a tower project).
- `fleet onboard [--step|--verify|--skip|--json|--key NAME=value|--confirm]`;
  the CLI's keyless refusal gained one line pointing at `fleet gui`.
- `/api/onboard` GET (state + narration + completion) and POST (ops:
  home, key — which sets the secret then proves it with ONE bounded
  routed call —, skip, confirm, verify, projects, remember).
- `UiEvent::NeedsSetup` replaces the routing-error transcript write: a
  keyless send raises the door and restores the typed words.
- The face: the door INSIDE `.empty` (eyebrow/title/lede swapped), the
  field is `class="composer onboard-key"` with `.foot-btn.send`, the
  discovered-key press is an `.offer` row, the mechanism is a `.method`
  disclosure, the capability cards are `.card`+`.offer` rows in
  `#onboard-cards`. `el.empty.dataset.onboard = 'door'` + CSS hides the
  work home's furniture (counts, method strip, lanes, offers, footnote).

**What verification caught (both worth remembering):**
- The **Chromium screenshot was the decisive check**: the door painted
  correctly but the power-user furniture still showed below it (77 facts,
  the method strip, the lanes chips). One state attribute + a CSS rule
  later, a first-run home is three sentences and one decision. The second
  screenshot caught the opposite bug — the `.method` hide rule swallowed
  the door's OWN disclosure (fixed with `.method:not(.onboard-mechanism)`).
- My new `case 'needs_setup':` landed immediately after `case 'finished':`
  in the reducer switch, so every finishing run fell into the door
  handler and eight receipt checks went red — a misplaced `case` inside a
  fall-through block is silent for the case before it.

**Proofs:** gate fmt 0 / clippy 0 / build 0 / test 0 (lib 1+7+16,
integration 152 passed 3 ignored); harness **365 checks / 40 cases**
(the door's shape, the abstraction words absent from the default path,
the onboarding's CSS declaring no token the console lacks and no literal
colour, the cards one-primary-one-not-now, a settled fleet showing none);
three integration tests over a scratch home (resume twice, the seeded fact
read back, the document's door copy). Docs: `docs/features/onboarding.md`
(new), README's "commissioned, not built" note replaced, a "The first run"
section in `docs/harness-orientation.md`.

Related: [[onboarding-is-a-door-not-a-wizard]] (the rules),
[[fleet-onboarding-composer-proposal]], [[fleet-settings-abstraction-search]],
[[fleet-anti-hermes-constitution]],
[[no-form-asks-for-a-value-fleet-already-has]],
[[fleet-gui-editorial-desk-redesign]],
[[fleet-vault-credentials]] (set_secret with no passphrase → plaintext
`.env`; vault seals later).
