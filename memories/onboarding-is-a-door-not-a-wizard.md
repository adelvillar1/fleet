---
name: onboarding-is-a-door-not-a-wizard
description: "owner direction 2026-10-29 — first-run must be frictionless, explain the benefit before the mechanism, and abstract every unnecessary complexity away; the accepted design is two states of the home (a benefit-language door only when fleet cannot answer, then the work home with one quiet optional \"set up more\"), proven by the product answering rather than by checkmarks"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 32
superseded-by:
valid-until:
---

The owner's direction (2026-10-29), verbatim: "I really want this to be
frictionless and to drive the user through the process explaining first
what the steps are for and the benefits. Keep the unnecessary
complexities away and abstracted from the user."

**Why:** fleet's first run was a power-user screen plus a technical
error — a fresh home showed the method strip, the lanes, four offers,
and then the first send wrote `routing config: <error>` into the
transcript. The owner's standing test ([[fleet-anti-hermes-constitution]]:
"does the owner need to know this knob exists?") applies hardest here,
because a first-time user has no context to filter jargon with.

**How to apply:**
- **Explain the benefit before the mechanism, always.** A step's headline
  says what the user *gets*; the env var, endpoint, model id, roster, or
  tier belongs in an optional "how this works" disclosure or nowhere.
- **One decision per screen, one primary button.** A card with two
  equally-weighted actions punishes reading.
- **Never ask what can be discovered.** Probe first (an existing key in
  the environment or `<FLEET_HOME>/.env`, an existing `~/Projects`),
  offer the found answer as a single press, and only ask when probing
  found nothing.
- **Every refusal names a benefit, never a name.** "Fleet can't answer
  yet — it needs a model to think with" is a refusal; a list of
  `*_API_KEY` names is not.
- **The proof is the product**: a real answer in the conversation, a
  real recall, a real project row — never a tick or the word
  "configured".
- **The setup surface shrinks to nothing**: optional capabilities live
  behind one quiet entry that disappears when every card is done or
  declined; never nag.
- **Keep the blocking path minimal**: ask only what the very next step
  cannot work without (one model key for a chat turn); defer passphrases,
  installs, folders and lanes to the moment they are actually needed.
- **A new surface must speak the console's existing language** ("make
  sure that the design language is consistent with what we have in the
  current gui"). Compose it from the primitives that already exist
  (`.empty`, `.offer`, `.card`, `.method`, `.notice`, `.foot-btn`) before
  inventing any chrome; a first-run field is the product's own
  `.composer`, because the first act should be the act the user keeps
  doing. No new colour, radius, font, spacing unit, or icon set.

**It is built** — `e838241` (2026-10-29, "/goal implement plan as
designed") and completed by `8419da3` once the audit caught two criteria
marked met early, plan completed, gate green, harness 375/41, the door
and the strategy preview verified live in Chromium. The strategy step is
where the "strategy, not a fact" rule shows: its preview presents the
chain, the two everyday workloads, the ceilings and the window fallback
and ends in a confirmation to READ, because there is no right answer to
"which model for which work" — only a choice the owner owns. The rules above are what the code
now enforces, and the harness asserts the copy ones directly (no
mechanism word on the default path; no new token or literal colour in
the onboarding's CSS; one primary button and one real "not now" per
card; a settled fleet shows no setup).

See [[guided-onboarding-implemented-e838241]] for what landed and
[[the-screenshot-is-the-visual-review]] for the check that caught the
door still showing the work home's furniture beneath it.
