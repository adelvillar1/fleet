---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: guided-onboarding
---

# Plan: guided onboarding for first-time users

## Context

Everything a first-time user needs to make Fleet work currently lives
in the owner's head: which key makes the judgment lane answer, which
endpoint quirk Liquid carries, why the projects folder matters, what
the GitHub PAT unlocks. The house docs carry the rulings as rulings,
written for people who were in the room. This plan commissions the
guided onboarding that turns that knowledge into a sequence.

The arc was ruled in one conversation (2026-10-29) and is
dependency-ordered; each step unlocks the next:

0. **The home, silently.** The home layout and the first key are all
   the critical path needs: `fleet install`'s provisioning (lane venvs,
   doctor's gate) runs only when a step that needs the lanes is opened,
   and the vault's passphrase is an *offered upgrade* ("Seal your keys")
   rather than a first-run wall — the key lands in `<FLEET_HOME>/.env`,
   which is already the second resolution step, and the vault seals it
   later with the benefit named. A chat turn needs one model key and no
   lanes (the judgment fails open), so nothing else may block the door.
1. **First LLM.** One provider key landing in the vault, and the
   narration reads the chain the roster builds from it — for a
   one-key home every walk closes on that key through the failover
   tail, and the onboarding says that in the chain's own words rather than
   calling any tier a default. Before the lanes, because every lane
   composes with an LLM, not instead of one.
2. **The lanes, in lane order.** sys1, sdm1, sem1, gen1, blend1. The
   order is the tutorial: judgment, math, meaning, media, synthesis.
3. **The projects folder.** The watched landscape: any repo under it
   whose agents file tower reports is tracked; the last action on a
   project is the tower's freshest row. The second-brain repo is one
   watched project among the rest.
4. **The GitHub PAT.** The tower's evidence access: claims are
   reconciled against git, not recorded on faith. Last among the
   provisioning steps because it turns the trust model on.
5. **The model roster.** The strategy table: head, tiers, billing,
   spend ceilings. The only step where the user supplies opinions
   rather than facts, and the step the whole machine reads back
   (tier chains, windows, failover, calibration cells).

The sequence reads as one sentence: speak, measure, remember, prove,
choose — spoken in the room step zero builds.

## Approach

### The derived-content rule

The onboarding is narration over a registry, not a second place where
truth lives. Every env name, base URL, model id, context window, and
fail-open behavior a step presents is read from the structures the
runtime already reads: the lane registry (`src/lanes.rs`), the
compiled-in roster and context-window table (`src/route.rs`), the
settings vocabulary (`src/settings.rs`), and the MCP server spec
(`src/mcp.rs`). No step's copy may restate a fact the registry owns;
if a lane's key name changes, the onboarding follows without an edit.

### The GUI: the door, then the work — a designed journey

The GUI onboarding is not a wizard and not a card stack on the welcome.
It is **two states of the home**, decided by detected truth, and the
rest is a door the owner opens when they want it.

**The find: today there is no first-run at all.** A fresh home renders
the full power-user welcome — the method strip, the lanes, four offers,
a footnote about Perception — and the first send fails in the transcript
with `routing config: <error>` (the CLI's version is worse: every
acceptable env var name printed at once). Nothing guides, nothing
explains, and the first experience of the product is a technical error.

**State one — the door (only when the machine cannot answer).** When
no routing candidate resolves a key, the home is not the work home: it
is one screen with one decision.

- **What it says, in this order:** what fleet is for (one sentence),
  what it needs (one sentence), what happens next (one sentence). Then
  one field and one button.
- **The benefit, not the mechanism:** "Fleet answers in this window,
  reads and writes files in a folder you choose, and remembers what you
  decide." No env var name, no provider list, no model id, no port, no
  word `vault` or `endpoint`, ever, on this screen.
- **It looks before it asks.** The door scans the environment and
  `<FLEET_HOME>/.env` for any key the roster knows and, when it finds
  one, offers it as a single press: "There is already a key on this
  machine — use it?" The paste field is for nothing-found only.
- **The proof is the product.** The press makes one bounded chat call
  and shows the answer in the conversation, word for word, with one
  line under it naming what just happened in plain words: "that answer
  came from a model the router chose — the chain is on the route bar if
  you're curious." No rosters, no tiers, no green checkmarks.
- **The failure path becomes the door.** A send attempted with no key
  raises this same screen inline instead of writing `routing config:`
  to the transcript; the typed words are kept in the composer. The CLI
  keeps its honest refusal for scripts, and gains one line pointing at
  `fleet gui` for the guided path.
- **Nothing else is asked.** No passphrase, no install, no folder
  choice: the key lands in `<FLEET_HOME>/.env` (the existing first
  resolution step), `fleet install` runs only when a later step needs
  the lanes it provisions, and a working folder is chosen when the user
  first wants file work — the run's own refusal ("no folder means no
  filesystem access") already teaches it at the moment it matters.

**State two — the work home, with one quiet door.** Once fleet can
answer, the home is the work home: the welcome, the counts, the offers.
One entry — in the settings surface and in the rail's "set up more" —
opens the capabilities in benefit order, each a single card on demand,
never blocking, never nagging:

| Card | What it says it gives you |
|---|---|
| Remember what you decide | "Fleet recalls decisions from last week instead of asking again." |
| Check its own work | "A second model grades the evidence before you trust an answer." |
| Watch your projects | "Point fleet at the folder your repos live in; it reads their git facts and never edits them." |
| Reconcile its claims | "Fleet checks a claim against the commit, not against a promise." |
| Seal your keys | "Keys encrypted at rest with a passphrase you hold." |
| Set the cost ceiling | "A daily limit, so an agent loop cannot surprise you." |

Each card: one sentence of benefit, one action, one visible proof, and
**"not now" that is a real button with no follow-up nag.** A finished
card leaves the list. When every card is done or declined, the entry
itself disappears — a fleet that needs no setup shows no setup.

### The design language: built from the console's own primitives

The door is not a new surface with new chrome — it is **the welcome in
another state**, assembled from the parts the console already owns. The
house language (read from `app.css` and the welcome markup):

- **The frame is `.empty`**: `min(760px, 100% − 56px)`, centred, padded
  24px, no card border — the welcome's own measure and ground.
- **The eyebrow is `.empty-eyebrow`**: lowercase in source, uppercase by
  CSS, 10px/600, `0.13em`, `--dsw-alias-accent-lift`, with the 22px
  leading rule its `::before` draws. The door's reads `fleet · first
  run`; the work welcome's `fleet · local console` stands unchanged.
- **The title is `.empty h1`**: `--font-display`, weight 500,
  `clamp(36px, 4.4vw, 52px)`, `-0.025em`, `max-width: 16ch`. It states
  what is about to happen, declaratively — the house never says
  "Welcome!" and never uses an exclamation mark.
- **The lede is `.empty p`**: 14.5px, `--dsw-alias-ink-secondary`,
  line-height 1.65, `max-width: 54ch`. The benefit sentence lives here.
- **The discovered-key press is an `.offer` row** — the same object as
  the welcome's four offers, because it is the same kind of thing: one
  press takes you somewhere. Icon mark in its 26px raised square, a
  12.5px/600 title, an 11.5px tertiary `said`.
- **The paste field is the composer** — not a new input. The door
  renders the product's own `.composer` with the primary action as its
  `.foot-btn.send`, because the first thing a fleet user does must be
  the thing they keep doing. Character-for-character the same field,
  radius and foot.
- **The mechanism disclosure is a `.method` block**: 9.5px/600
  `0.11em` uppercase label, a `border-top` hairline, quiet body. Every
  word the abstraction rules forbid on the default path lives here and
  only here, behind a press.
- **The capability rows are `.offer` rows inside a `.card`** in the
  settings surface: `.card-head h2` at 13px/600, a `.card-note` for the
  one-line framing, then the rows. "Not now" is a `.row-rewind` quiet
  chip — never a second primary.
- **A proof is a `.notice`** (the existing toast) plus the real artifact
  in the surface that owns it: a fact on the memory line, a project row
  in the tower, a route row on the router. No new confirmation screen,
  no checkmarks, no "step 1 of 6", no progress bar — the house shows
  state in words (`handed back`, `working`) and dots with `data-state`.

**What the build may not introduce:** a colour outside
`--dsw-alias-*`/`--lane-*`, a radius outside the card/pill set, a font
outside `--font-display`/sans/`--font-mono`, a spacing unit outside the
existing rhythm, an icon set beyond the inline stroke SVGs already in
`index.html`, or a component class that is not a reuse of `.empty`,
`.offer`, `.card`, `.method`, `.notice`, `.foot-btn`, `.ghost`, or
`.row-rewind`. If a new class is needed for the door's one-column
stack, it is layout only and names nothing about appearance.

### The abstraction rules (testable)

The copy rules are criteria, not taste, because each has been violated
in the first-run surface as it stands:

1. **No mechanism on the default path.** The words `key env var`, a
   `*_API_KEY` name, `endpoint`, `roster`, `tier`, `vault`, `port`, and
   any model id may appear only inside a per-card "how this works"
   disclosure the owner opens — never in a card's headline, body, or
   button.
2. **One decision per screen, one primary button.** A card with two
   equally-weighted actions is a card that punishes reading.
3. **Never ask what can be discovered.** Keys, `~/Projects`, an
   existing `.env`, an existing git repo: probe first, offer the answer
   as a press, and ask only when probing found nothing.
4. **Every refusal names the benefit, not the name.** "Fleet can't
   answer yet — it needs a model to think with" is a refusal; a list of
   env var names is not.
5. **The setup surface shrinks to nothing.** Completion removes the
   entry; a returning owner sees the work home and no reminders.
6. **The proof is the product.** Every step's evidence is a real answer,
   a real recall, a real project row — never a checklist tick, never
   the word "configured".

### Write through the existing seams only

The onboarding adds zero new storage and zero new trust surfaces.

- Keys ride the existing secret path the GUI already uses
  (`set_secret`). With no passphrase it writes the plaintext
  `<FLEET_HOME>/.env` line — the door's zero-ceremony path; "Seal your
  keys" is the upgrade (`vault_onboard` + `vault_seal_env`), offered
  with its benefit and never required. A key is write-only in the UI:
  sent once, never rendered back, shown as set or unset, sealed or
  plaintext. Credential-shaped names are enforced (_KEY, _TOKEN,
  _SECRET) so a typo is a named refusal — and the door never shows the
  name it enforces, only the refusal in plain words.
- Endpoints and provider overrides ride the roster's own schema.
  A malformed field is a named refusal, never a silently ignored key.
- The GitHub PAT is a vault write whose name is already declared by
  the GitHub server card (GITHUB_PERSONAL_ACCESS_TOKEN); the MCP
  registry's rule stands: env NAMES only, never values.

### The proof ladder

Every step ends in a proof, per the claim-time verification rule. Two
rungs, and the second is always opt-in because it may cost money:

1. **Presence, free.** The key resolves, the endpoint parses, the
   folder resolves, the token is named in the vault state. Same
   checks `fleet doctor` runs.
2. **Live verify, one bounded call, opt-in.** sys1 classifies one
   fixture and returns a verdict with a confidence; sdm1 scores one
   fixture table; sem1 embeds one string; gen1 renders one brief to
   pixels and judges those pixels; blend1 composes one ask with the
   synthesis standing on the gathered legs; the first LLM answers one
   bounded chat turn. The user sees the lane work before moving on.

Completion for the whole onboarding is one green probe, not a checkbox
list: `fleet doctor` green end to end, one chat turn that routes, one
recalled fact visible on the memory line, and one project the tower
can name. The recalled fact exists because the sequence puts it there:
at the end of the steps, it remembers the onboarding itself
through the existing remember path — the date, the head the roster
serves, the projects root the tower watches — so a fresh home's corpus
holds one true fact, and the probe's recall check reads back what the
sequence wrote rather than pretending a corpus existed.

### Skippable, with the cost named

The lanes fail open; onboarding may not imply all five are mandatory.
Each step offers skip, and the skip names the consequence in the
consequence's own words: without sem1, recall leans on keyword only;
without blend1, composed work answers with evidence and not prose;
without the PAT, tower claims stay unverified. A skip is recorded in
the calibration stream like any other decision.

### Resumability

`fleet onboard` holds no session state of its own; it recomputes what
is already true at each launch (keys present, lanes answering, folder
resolving, token named) and starts at the first incomplete step. The
doctor's checks are the state.

### Step five's different epistemics

Steps one through four end in a decisive green because there is a
right answer. Step five ends in a confirmation preview instead: the
chain as built, what a quick task does, what a hard one does, the
spend ceiling. The roster is a strategy, and the onboarding says so. The
preview also names the compaction consequence the context-window table
implies: a model the roster does not declare gets the window floor,
and the user hears it at pick time, not in week three.

## Acceptance criteria

- [x] `fleet onboard` runs the home step and the five in the declared
      order and resumes at the first incomplete step
      (`the_onboarding_resumes_at_the_first_incomplete_step`, driven
      twice); the vault is an offered upgrade rather than a blocking
      step zero, per the deeper look (finding 5).
- [x] A keyless home is the door: one screen, one decision, benefit
      before mechanism, and the discovered-key press when one exists
      (`OB the door is the welcome in its first-run state`); the press
      proves itself with one bounded call whose answer lands in the
      conversation (`/api/onboard`'s `key` op → `onboard_prove`).
- [x] A send with no key raises the door inline — `UiEvent::NeedsSetup`
      replaces the routing error, the face's `needs_setup` case restores
      the typed words, and the CLI's own refusal names `fleet gui`.
- [x] The abstraction rules hold under test: the harness asserts the
      door's default path names no mechanism word, each card carries one
      primary button and one real "not now", and the door's copy is
      asserted twice (harness and
      `the_onboarding_document_is_the_door_when_it_cannot_answer`).
- [x] No new design chrome: the door is inside `.empty`, the press is
      an `.offer` row, the field is `class="composer onboard-key"` with
      a `.foot-btn.send`, the disclosure is a `.method` block, and the
      harness asserts the onboarding's CSS declares no token the console
      lacks and no literal colour (`OB the door is the welcome in its
      first-run state`).
- [x] The capabilities appear as benefit cards on demand
      (`OB the capability cards are on demand`), and a settled fleet
      shows none.
- [x] Every narrated fact is derived —
      `the_narration_is_derived_from_the_lane_registry` walks the
      registry's own names, roles, root, and window floor.
- [x] Keys go through `settings::set_secret` only, never read back; the
      refusals are named (`refusals_are_named_in_plain_words`).
- [x] The lanes step's presence is the resolve check and `--verify` /
      the `verify` op asks them live; every declinable step's skip names
      its consequence from the narration.
- [x] The projects step names the root the tower scans (derived) and
      takes an override through the settings writer; the memory line's
      root naming is KI-074/087's existing fix, which the step's copy
      points at.
- [x] The PAT step reads the name the GitHub card declares
      (`mcp_registry::github_token_name`, from the registry never a
      constant) and writes through the secret path; with no card the
      step refuses in plain words rather than inventing a name.
- [x] The roster step ends in the confirmation (a `lifecycle` row) and
      its disclosure names the window floor for undeclared models, read
      from `compaction::COMPACTION_TOKEN_BUDGET`.
- [x] Completion is the one green probe over four real reads, and the
      recalled fact is the sequence's own
      (`the_onboarding_seeds_the_fact_its_completion_reads`).
- [x] The gate ran whole with exit codes read directly.

## Files

- `src/onboard.rs` (new) the sequence, its narration, and the probes.
- `src/cli.rs` the `fleet onboard` command and its options.
- `src/lanes.rs`, `src/route.rs` registry accessors the narration reads,
  if a field the narration needs is private today.
- `src/gui/bridge.rs` the derived onboarding state (`/api/onboard`: the
  per-capability state read from the same registries and probes), the
  key press and its bounded proof, and the no-key run's door event.
- `src/gui/assets/app.js`, `index.html`, `app.css` the door, the
  capability cards, and the copy rules' shape.
- `docs/features/onboarding.md` (new) what each step does and proves.
- `docs/harness-orientation.md` and `README.md` the quick start points
  at `fleet onboard`.
- `tests/integration.rs` the end-to-end onboarding over a scratch
  FLEET_HOME and the mock API, mirroring the install suite's shape.

## Out of scope

- No vault redesign; the vault's existing first-run onboarding (the
  passphrase, the migration of plaintext .env entries) is step zero
  by invocation, not by rewrite.
- No calibration changes; the stream's vocabulary carries onboarding
  decisions as rows and nothing more.
- No new config grammar, no new file format, no background service.
- The second brain itself is not built here: the step confirms the
  projects root the tower already watches and nothing beyond it.

## Verification

Per criterion above: the resume behavior is driven twice in the
integration suite over a scratch home; derived content is pinned by a
test asserting a registry field renders in narration; refusals are
asserted by message; the completion probe is the suite's final
assertion. The chat-turn verify stays out of the suite (not
deterministic) and is proven by hand once, recorded in the recap. The
GUI criteria are driven by the face harness over the bridge's document
shapes, the way the pane's and the work rail's checks are.

## Examined 2026-10-29 — six findings incorporated

The plan was examined against the code the same day it was drafted
(the registries, the secret path, the tower root, KI-074/087, the
vault's first run all verified as described). Six findings landed:

1. **The install dependency** — the lanes' live verifies spawn venvs
   only `fleet install` builds; the arc gained step zero and the
   onboarding invokes install rather than re-implementing it.
2. **The GUI was promised, never specified** — superseded by finding 5:
   the surface is no longer a card stack but the home's two states,
   with `/api/onboard` and criteria of its own.
3. **The completion probe needed a first fact** — the onboarding now
   remembers itself through the existing remember path,
   and the probe reads that back.
4. **"Flash tier by default" misdescribed the chain** — the narration
   reads the chain the roster builds; the failover tail is the walk's
   end for a one-key home, not a default tier.
5. **The GUI had no designed journey** — the deeper look found no
   first-run at all (a fresh home shows the full power-user welcome, and
   the first send writes `routing config: <error>` into the transcript)
   and that a chat turn needs one key and no lanes. The design became
   two states of the home — the door and the work home — with the vault
   and install moved off the critical path.
6. **The design had no house language** — every element is now mapped to
   an existing primitive (`.empty`, `.offer`, `.card`, `.method`,
   `.notice`, `.foot-btn`) with an explicit list of what the build may
   not introduce, and the plan's own vocabulary was unified: "wizard"
   retires everywhere, because the GUI design rejects the metaphor the
   word carries.
