---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: skills-rail
---

# Plan: the skills revamp — the rail, the categories, and the approve that works

## Context

The skills surface is the knowledge group's long page: the library card
(search, filter chips, a 30-row budget), then four cards stacked under it
(the decisions queue, declined, curation, retired) on one scroll, actions
scattered across hover rows. The library behind it is real — **453 skills**
in the canonical `~/.agents/skills`, of which the prompt declares the
newest 60 — and the surface treats it as one undifferentiated list.

The deep dive (2026-10-30) measured the substrate the revamp stands on:

- **The files carry no categories.** The frontmatter vocabulary across all
  453 libraries is `name` + `description` (451 `name:`, one `title:`,
  **six** `category:`). Any category set must be **fleet-declared data**,
  not something read from the files.
- **The approve is broken, structurally.** `bridge::skill_apply` runs
  `improve::Gate::local` — four cargo steps — for a Skill target whose
  staged change carries `repo_paths: Vec::new()` (a home-side write, no
  commit, pre-image stash for the undo). The face runs under the
  LaunchAgent whose PATH has no cargo (cargo lives only in the rustup
  toolchain dir, per `CLAUDE.local.md`), so `Command::new("cargo")` fails
  NotFound on every step and **every approve of every proposal is
  refused**, always, on this machine — the owner's report, now diagnosed.
  The gate is also the wrong depth for the change: a full release build
  plus test suite is the price of writing one markdown file into the
  library (the verification-depth ruling, applied here).
- The other actions are sound: retire/unretire write
  `<home>/skills-retired.json` through the one writer and never the
  library (KI-215), the usage scan counts approved `use_skill` calls, the
  drafter and the merge curation file proposals deterministically.

The owner's direction, 2026-10-30: arrange the skills surface the way the
settings and bots revamps arranged theirs — the work rail's idiom, the
shared `sectionRail`, one section at a time, the right pane for details —
and **make sure every action button works**, the approve first.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the pattern.** *"I want [to] use the same pattern to
   arrange skills into a reasonable set of categories"* — the settings/
   bots shape: rail → one section → one-line rows → the item in the pane.
2. **2026-10-30 — the buttons.** *"make sure that all action buttons on
   skills work because right now the approval of suggested skills is not
   working"* — the approve defect is this plan's first increment, not a
   footnote.

## Approach

The category set is **declared data in the repo**, derived once from the
real library and reviewed by the owner — the nomenclature vocabulary's
shape, not a runtime inference. Assignment is `category_of(name)` over an
exact-name table (`src/skills-categories.json`, `include_str!`d) with an
explicit **everything else** section whose rail badge makes every
unmatched name visible; a skill added to the library tomorrow lands there
until the table names it. The classification is classify-shaped, but it is
one-time editorial curation with an owner-reviewable artifact, not a
per-run judgment — no sys1 leg in this plan (a sys1 *suggester* for
unmatched names is named as a possible follow-up, not built).

The proposed sections, named in the owner's language and ordered by what a
person arrives to do (exact memberships and counts land in this file when
the table does):

| Section | Holds | Rail badge |
|---|---|---|
| **needs you** | the proposals awaiting a decision | n proposals |
| **building** | writing and verifying code: debugging, refactors, tests, reviews, git, CI | n |
| **web** | frontend frameworks, styling, browsers, accessibility | n |
| **design** | interface and visual design, brand, mockups | n |
| **media** | video, 3D, image, audio, motion | n |
| **documents** | docx / pdf / xlsx / pptx, decks, reading and OCR | n |
| **data** | databases, pipelines, LLM/AI engineering | n |
| **platforms** | integrations and external services: github, stripe, railway, MCP, email, social | n |
| **machine** | this Mac and the owner's personal workflows | n |
| **everything else** | the honest fallback — unmatched names | n |
| **declined** | the loader's skips, reason named | n |
| **curation** | the merge candidates, similarity named | n |
| **retired** | the names off the prompt, one click back | n |

The surface consumes what the settings and bots revamps already proved:
`sectionRail()` (the shared helper — never a lookalike), the `cfgItem`
one-line row, `budgetRows`, and the pane card (`openDrawer`, tabs stood
down, the dirty-close guard). The search stays at the surface head reading
across every section; the used/never-used filter chips retire in favor of
marks on the rows (uses count, never-selected), the way settings' rows
carry their marks.

The approve fix is **one rule in `improve::apply`**, not a face bypass:
the cargo gate runs only when the staged change touches repo paths
(`!landed.repo_paths.is_empty()`); a home-side target lands on its stage
validations (the draft parses, the name matches, `proposed: true` is
refused) plus the pre-image stash, records one honest gate step
(`home-side write — no repo gate`), and stays reversible by
`fleet improve undo`. Repo-side targets keep the full gate, pinned. This
fixes the face and the CLI with one rule — the CLI apply of a skill
proposal fails the same way today from any shell without the PATH line.

High-level steps:

1. **S0 — the approve, fixed at the root.** The gate-depth rule in
   `improve::apply`, its unit pin (a skill proposal applies green with
   cargo absent from PATH), KI-374 filed in the same commit.
2. **S1 — the declared table.** `src/skills-categories.json` +
   `category_of` + its tests; the real library measured through it once,
   counts recorded in this file.
3. **S2 — the rail and the sections.** The skills stage gains the rail
   nav; `paintSkills` becomes one section painter; the hash route
   `#/skills/<section>`; badges from the read.
4. **S3 — the rows and the pane.** One-line rows with budget and marks;
   a row's press opens the pane card with the body, the uses, the path,
   and the actions; proposals render kind + evidence + body + approve.
5. **S4 — the buttons, proven.** A harness check per action button
   against its named element; a live browser approve through the face;
   the measurement and the docs.

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### S0 — the approve works

`improve::apply` gains the depth rule: `ctx.gate.run()` only when the
staged change names repo paths; otherwise one recorded `GateStep` naming
why no gate judged it. The face's `skill_apply` is untouched — the rule
lives where the gate lives, so the CLI inherits it. `finish` still
records the Propose row and stamps the proposal file. KI-374 (the face's
approve could never succeed: a repo gate over a home-side write, cargo
absent from the face's PATH) filed in the same commit.

*Checks:* K1, K2.

### S1 — the category table

`src/skills-categories.json` declares the sections and their memberships
(exact names; at most a handful of prefix rules where a family is real);
`skills::category_of` reads it; `boards::read_skills` stamps `category`
per skill in the face's document. The library is measured through the
table once and the per-section counts are recorded below.

*Counts (recorded when the table lands):* — pending S1.

*Checks:* K3.

### S2 — the rail and the sections

`index.html`'s skills stage gains `<nav class="section-rail"
id="skills-rail">`; `paintSkills` becomes the rail painter plus one
section painter (the `paintSettingsSection` shape); `#/skills/<section>`
addresses a section, bare `#/skills` resumes the remembered one; the
needs-you row leads; declined, curation and retired close the rail.

*Checks:* K4, K5.

### S3 — the rows and the pane

Category sections render one-line `cfgItem` rows (name, description
value, the uses mark, never-selected mark) under a 30-row budget with the
count named; the search box stays at the surface head across sections. A
row's press opens the pane card (tabs stood down, dirty-close guard):
description, body, uses, path, modified, related, and the actions (copy
name, copy path, ask the curator, retire / return). A proposal row's card
carries kind, evidence, the proposed body, and approve.

*Checks:* K6, K7.

### S4 — every button proven, the measurement, the docs

A harness case per action button (open, path, ask, copy, retire,
unretire, approve, read, search, budget) asserted against a named
element; one real proposal approved through the live face in a browser
(build stamp visible); the Chromium probe quoting the before/after
heights and control counts; the docs.

*Checks:* K8, K9, K10.

## Use cases

- [ ] A: The owner reaches any of 453 skills through a category section instead of one long scroll.
- [ ] B: A drafted proposal is approvable from the face — today every approve refuses.
- [ ] C: A skill's detail and actions live in the right pane, the same as a setting's.
- [ ] D: A retired skill returns in one click; a declined skill names its reason; a merge candidate names its pair.

## Acceptance criteria

- [ ] **K1 — the depth rule.** `improve::apply` runs the cargo gate only
      for a staged change carrying repo paths; a home-side target records
      one gate step naming why no gate judged it; a repo-side target
      still refuses on a red step. Pinned by a unit test that applies a
      skill proposal with cargo absent from PATH and lands it green, and
      one that still refuses a patch target with cargo absent.
- [ ] **K2 — the live approve.** A proposal approved through the face's
      `POST /api/skill` returns applied and the library holds the skill;
      a refused apply names its reason in the notice. Proven in a real
      browser through the face (a scratch home's proposal suffices).
- [ ] **K3 — the table is declared and total.** `src/skills-categories.json`
      parses; every section is non-empty; `category_of` is deterministic
      (same name, same section); the live library measured through it
      lands every name in a category or in `everything else`, with the
      per-section counts recorded in this plan.
- [ ] **K4 — the rail is the shared helper.** The skills panel renders
      `sectionRail` (store `fleet-skills-rail`), collapses to icons,
      survives a reload, marks `aria-current`, and takes the arrow keys;
      only the open section's cards and rows exist in the DOM.
- [ ] **K5 — a section is an address.** `#/skills/<section>` opens it;
      bare `#/skills` resumes the remembered one; an unknown segment
      falls back to `needs you`.
- [ ] **K6 — the rows are one line with a budget.** A category's rows are
      `cfgItem` one-liners with the uses and never-selected marks; the
      first 30 show and the press names the rest; the search box reads
      name and description across every section.
- [ ] **K7 — the pane is the item.** The row's press opens the card with
      description, body, uses, path, modified, related, and the actions;
      a proposal's card carries kind, evidence, body, and approve; Escape
      closes, closing with an in-flight action asks; the row refreshes on
      a landed action.
- [ ] **K8 — every button works.** A harness check per action button —
      open, path, ask, copy, retire, unretire, approve, read, search,
      budget — each asserted against its named element and its observed
      effect, in the same commit as the element.
- [ ] **K9 — the retire law holds.** Retire and return write only
      `<home>/skills-retired.json` and never the library (KI-215's pin
      stays green through the revamp).
- [ ] **K10 — the gate and the measurement.** fmt, clippy, build, test
      green by exit code; the Chromium probe quotes the before/after
      heights and control counts; KI-374 filed in S0's commit.

## Files to be touched

**Backend / API:**
- `src/improve.rs` — the depth rule in `apply`; the home-side gate step; the two unit pins
- `src/skills.rs` — `category_of` over the included table; the tests
- `src/skills-categories.json` — new: the declared table
- `src/boards.rs` — `read_skills` stamps `category` per skill
- `docs/known-issues.md` — KI-374

**Frontend:**
- `src/gui/assets/index.html` — the skills stage's rail nav
- `src/gui/assets/app.js` — `paintSkills` as rail + section painter; the pane cards; the hash route

**Tests:**
- `tests/face/harness.mjs` — the per-button case (K8) and the section/route/pane cases
- `tests/integration.rs` or the module tests — the depth rule's pins

**Documentation (see Linked artifacts below):**
- `docs/features/skills.md` — the surface section rewritten
- `TECHNICAL-DOCUMENTATION.md` — the skills module reference
- `FUNCTIONAL-SPECIFICATIONS.md` — the skills surface passage

## Out of scope

- **A deny/dismiss button for proposals** — only approve exists today; refusing by leaving the draft waiting is the current contract, and a deny is its own decision.
- **Editing skill files from the face** — the library stays read-only to fleet; the drafter and the owner's editor are its writers.
- **Per-skill provenance** (KI-215's named follow-up) — the usage scan still reads fleet's sessions only.
- **The prompt-side cap and trigger** — the 60-cap, the trigger shaping, and `use_skill` are unchanged.
- **A TUI skills surface** — still none, by the standing decision.
- **A sys1 category suggester** — named as a possible follow-up for unmatched names; the table is the source of truth.

## Verification

**K1** — the two unit pins run with PATH stripped of cargo:
```bash
cargo test the_home_side_apply_lands_without_cargo
cargo test the_repo_side_apply_still_needs_its_gate
```
Expected: both pass with `cargo` absent (the pin manipulates the gate's
cargo path, not the machine's).

**K2** — in a browser at the face: open needs you, press approve on one
proposal, read "the proposal landed" and the skill in its category.

**K3** — the table's test plus one measured pass:
```bash
cargo test the_category_table_parses_and_assigns
```
Expected: green; the counts recorded under S1 in this file.

**K4–K8** — the face harness (scratch home, the skills cases bumped):
```bash
node tests/face/harness.mjs
```
Expected: every new check green; the count line quoted in the commit.

**K10** — the gate, whole, exit codes read directly; the Chromium probe
at 1440×900 quoting before/after, the screenshots read as the review.

## Linked artifacts

- `docs/features/skills.md` — the surface section: the rail, the categories, the pane, the depth rule
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — `src/skills.rs`'s entry documents only the CLI surface today (the arch-gate's DRIFTS reading: the face's skills panel is undocumented and lags two revamps); the entry gains the face surface, `src/skills-categories.json`, and `src/improve.rs`'s depth rule
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the skills passage: sections, the pane, the approve

## Risks

- **The category table is editorial and will disagree with the owner's mental model somewhere.** Mitigation: the everything-else badge makes mismatch visible; membership is one reviewed JSON file; the owner's edits are data, not code.
- **Dropping the gate for home-side targets widens what approve can land.** Mitigation: the stage's validations (parse, name match, `proposed: true` refusal) and the pre-image stash remain; repo-side targets keep the full gate, pinned by K1's second test; the calibration row still records every apply.
- **Thirteen rail rows** is more than settings' eight. Mitigation: the rail folds to icons by design; the badge counts do the finding.

## Dependencies

- The settings revamp (`2026-10-29-settings-rail.md`, completed) — `sectionRail`, `cfgItem`, `budgetRows`, the pane card idiom.
- The bots revamp (`2026-10-29-bots-rail.md`, completed) — the needs-you-first ordering this plan's rail follows.

## Notes

- The approve diagnosis, for the record: `bridge::skill_apply` (bridge.rs) builds `Gate::local(repo)` whose cargo is the bare `"cargo"` PATH-resolved at `Command::new` time; the LaunchAgent's PATH (`/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin`) holds no cargo and the rustup shim is keg-only, so all four steps fail NotFound and `apply` refuses before the (already-staged) write is judged. The staged Skill write carries `repo_paths: Vec::new()` — no commit, the stash is the undo — which is exactly the signal the depth rule reads. Resolving cargo into the gate was considered and rejected: it would fix the ENOENT and keep a minutes-long release build as the price of one markdown write.
- The six `category:`-bearing files in the library: their word is a reasonable seed for the table but the table, not the files, stays the source of truth (the loader keeps unknown keys opaque by design — parsing them back for six files is not worth a dialect).
- Proposal names in the queue today (six drafts awaiting): `deck-recovery`, `procedure-bash-bash`, `procedure-read-file-bash`, `read-file-recovery`, `remember-recovery`, `turn-failure-network` — S0 un-sticks all six.
