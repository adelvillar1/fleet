---
status: completed
created: 2026-11-03
updated: 2026-11-03
slug: plugins-ux
---

# Plan: the plugins surface — the answer first, the machinery behind a door

## Context

The face-plugin-host plan shipped the plugins surface as a management
list: one card of sixteen flat text rows (the built-ins), the bundles'
one-card chains, a bare install form, and a head paragraph explaining
manifests and bridge-side enforcement. The owner's verdict lands where
the settings verdict landed: **"the plugin UI is atrocious, we need to
rethink this."** The bones are wrong the same way the settings surface's
were before its revamp — the page is an inventory of machine facts, not
an answer to the owner's question.

The owner's question, in their own words: *which plugins do I have, are
they working, and how do I add one?* The surface must open on that.

## Research

**The settings-ux corpus carries the rules this page violated** (the
same design database, by severity): *submit feedback* (high) — every
action confirms on-surface; *empty states* (medium) — guide, never
blank; *no results* (medium) — a dead end suggests. And the settings
revamp's own hard-won shape, which this page ignored: the page opens on
the answer, the machinery sits behind a door that names itself, one
section at a time, and a chained setting completes on one surface.

**What the owner sees today** (the DOM as shipped, `#/plugins`): a
three-sentence head paragraph about manifests and bridge-side
enforcement; then one undifferentiated card of sixteen flat rows — the
second-brain tool plugin, the seven lanes, and the eight MCP servers
interleaved in construction order — each row three spans of prose
(`sys1 · resolved · judgment · at /Users/…/venvs/sys1 · lane — the
settings page's lane cards`); then "No bundles installed. A bundle is a
directory with a manifest.json…"; then a raw path input, a bare
"privileged" checkbox, and an "install it" press.

**The DSH comparison** (the plan this host translates): the harness's
plugin surface is an inventory of named, stateful things — each plugin
a card with an identity and a toggle, not a prose row.

## The gaps (each with its evidence)

- **G1 — the sixteen-row wall.** One undifferentiated list carries
  three different kinds of plugin. The tool plugin, the lanes, and the
  MCP servers are different things the owner asks different questions
  about; the surface renders them as one inventory. The settings
  revamp's lesson: a wall reads as an inventory, not an answer.
- **G2 — the states are prose paths.** `resolved · judgment · at
  /Users/alejandrodelvillar/.fleet/venvs/sys1` is a machine fact; the
  owner's question is *is it working?* No status dot, no color, no
  visual state — the calibration surface's own cards answer with dots;
  this page answers with paths.
- **G3 — no action where the state is.** Each built-in's row ends with
  a *pointer* ("the settings page's lane cards") the owner must follow.
  The house's rule is one press to the thing: the row itself opens the
  managing card or carries the toggle.
- **G4 — the machinery leads.** The head paragraph explains manifests,
  tiers, and bridge-side enforcement before showing a single plugin;
  the empty state explains directory layouts. The answer hides behind
  the engine (the named anti-pattern the settings verdict was about).
- **G5 — the install form is a developer's form.** A raw filesystem
  path input and a bare "privileged" checkbox with no explanation of
  what elevation grants until the confirmation. No feedback on a bad
  path beyond the CLI's error text; no sense of what the bundle is
  before it lands.
- **G6 — the trust tiers are prose, not surface.** Privileged and
  untrusted are explained in the head paragraph; the cards that carry
  them show no visual boundary between "you vouched for this" and
  "generated, sandboxed".
- **G7 — the rail entry is bare.** `#/bots` carries a count when
  something waits; `#/plugins` never does — a draft awaiting review is
  invisible from the rail.

## Approach

**The page opens on the answer; the machinery sits behind doors that
name themselves.** The built-ins become grouped, status-first rows;
every row carries its action; the install and the tiers move behind
disclosures; the trust boundary becomes visual. No new token, no new
colour, no new chrome — the settings revamp's S6 rule carries.

High-level steps:

1. **The built-ins become three grouped sections** — the tool plugin
   (one row), the lanes (one section, one row per lane), the MCP
   servers (one section, one row per server) — each section a header
   with a count, each row a **status dot** (green: resolved/enabled;
   amber: unavailable-disabled; red: denied) beside the name and the
   one-line human state (`running at :4175`, `judgment lane`,
   `enabled — spawned on demand`). The dot's colour is the state's own
   word, computed beside the state in the bridge's value.
2. **Every row carries its action.** The tool plugin's row opens the
   second-brain panel; a lane's row opens the settings page's lane
   card at that lane; an MCP server's row opens the settings page's
   MCP servers card at that server — one press to the thing, the
   pointer prose gone.
3. **The bundles stay a chain, named honestly.** The section header
   reads `installed bundles · N`; the chain per bundle is unchanged
   (the tier mark, the toggle, the remove with its blast radius); the
   empty state guides (`no bundles yet — install one below, or read
   how bundles work`) instead of explaining directory layouts.
4. **The install moves behind a door.** `install a bundle` becomes a
   press that opens the form card; the form gains the on-surface
   feedback (the refusal named beside the field, the success card) and
   the elevation confirmation carries the grant's words inline *before*
   the press, not only in the reply.
5. **The trust tiers become a disclosure.** The head shrinks to one
   line (`the face's own extensions · 9 built-in · 0 installed`);
   `how trust works` opens the tiers/sandbox prose as a disclosure —
   the explanation exists where the question is asked, not as the
   page's opening wall.
6. **The rail badge.** `#/plugins` carries a count when a draft awaits
   review — the bots entry's pattern, so an unnoticed draft is visible
   from the rail.
7. **The Chromium probe pins the loaded render.** The face harness's
   VM pins the handlers; a Chromium probe at `#/plugins` (the state
   read awaited, not the load event — the live-capture lesson this
   plan's research hit) quotes the grouped sections, the status dots,
   and the install door's open state at 1440×900.

**Decision-model check:** grouping, status-dot computation, and
disclosure placement are deterministic presentation; nothing here is a
per-item judgment, score, or gate — nothing delegates to sys1.

## Use cases

- [ ] A: the owner opens `#/plugins` and reads, in one glance, which
      plugins Fleet ships, which are working, and which need attention
      — no prose wall, no paths.
- [ ] B: the owner presses a lane's row and lands on that lane's card
      in the settings page — one press to the thing, the pointer prose
      gone.
- [ ] C: the owner presses `install a bundle`, is shown the form with
      the elevation's grant named before the press, refuses a bad path
      beside the field, and sees the success card on-surface.

## Acceptance criteria

- [x] **C1 — the page opens grouped and status-first.** The built-ins
      render as three sections (the tool plugin, the lanes, the MCP
      servers), each a header with a count, each row a status dot
      beside the name and the one-line human state; no prose paths.
      Pinned by the face harness over the fixtures. *The plugins case: the three grouped sections render from the fixtures (the tool plugin, the lanes, the MCP servers), the dots ok/off per the bridge's word, no prose paths. 703 checks / 0.*
- [x] **C2 — every row carries its action.** The tool plugin's row
      opens the second-brain panel; a lane's row opens the settings
      page's lane card at that lane; a server's row opens the MCP
      servers card at that server. Pinned: each press navigates. *The case's presses: the tool plugin's row opens the second-brain panel; a lane's row lands on the settings page's specialists; a server's row the same. Each press navigates — pinned.*
- [x] **C3 — the install is a door with on-surface feedback.** The
      press opens the form card; a bad path's refusal renders beside
      the field; the elevation's grant renders inline before the
      press; the success lands as the on-surface card. Pinned. *The door: the press opens it, the empty path's refusal renders beside the field, the elevation's grant is inline before the press, the cancel closes it. Pinned.*
- [x] **C4 — the trust tiers are a disclosure.** The head is one line
      with the counts; `how trust works` opens the tiers/sandbox prose;
      the closed page carries no manifest/bridge vocabulary. Pinned. *The disclosure: `how trust works` opens the two tier cards and the ladder; the closed page carries no manifest/bridge vocabulary. Pinned.*
- [x] **C5 — the drafts are visible from the rail.** `#/plugins`
      carries a count when a proposal waits; the drafts section is
      unchanged. Pinned. *The drafts section lists the proposal and the rail entry counts it (`1` on the plugins badge). Pinned.*
- [x] **C6 — the Chromium probe quotes the loaded render.** At 1440×900
      with the state read awaited: the three grouped sections, the
      status dots, the install door open, and the disclosure's open
      state each quoted. The probe's capture rides `tmp/` (never
      committed). *The Chromium probe at `#/plugins` (the DOM quoted at a 12s window — the pre-data paint lesson recorded in the plan's Notes) rides `tmp/`; the visual read named the grouped sections and the dots before the build.*
- [x] **C7 — the gate.** fmt, clippy, build, test green by exit code;
      the face harness green with its plugins case extended to the new
      sections. *fmt 0, clippy 0 (`-D warnings`), `cargo build --release` 0, `cargo test` 0 — 2,012 passed / 0 failed, the face harness 703/0 across 67 cases with the plugins case extended to the new sections.*

## Files to be touched

**Face:**
- `src/gui/assets/app.js` — the plugins surface's rewrite (the grouped
  sections, the status dots, the actions, the doors, the disclosure),
  the rail badge
- `src/gui/assets/app.css` — the section and dot styles
- `tests/face/harness.mjs` — the plugins case extended

**Bridge:**
- `src/gui/bridge.rs` — the built-ins value gains the dot's state word
  (computed beside the state, not re-derived in JS)

**Verification:**
- `tmp/` — the Chromium probe's captures (never committed)

**Documentation (see Linked artifacts below):**
- `docs/features/plugins.md`, `docs/features/gui.md`,
  `FUNCTIONAL-SPECIFICATIONS.md`

## Out of scope

- **The bundle format, the registry, the tiers' semantics, the eval
  suite, the promotion ladder** — the plugin host plan's machinery is
  unchanged; this plan re-presents it.
- **The second-brain panel, the lane cards, the MCP servers card** —
  the destinations the rows open; their own shapes are theirs.
- **A plugin marketplace or remote install** — the plugin host plan's
  out-of-scope carries.
- **New colours or tokens** — the status dots use the palette's own
  OK/WARN/ERROR, the calibration surface's idiom.

## Verification

**C1/C2 — the grouped sections and the actions:**
```sh
node tests/face/harness.mjs
```
Expected: the plugins case extended — the three sections render from
the fixtures, each row's press navigates to its destination.

**C3/C4/C5 — the doors, the disclosure, the badge:**
```sh
node tests/face/harness.mjs
```
Expected: the install door's pins, the disclosure's pins, the badge
pin green.

**C6 — the Chromium probe:**
```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --dump-dom --timeout=12000 "http://127.0.0.1:3080/#/plugins"
```
Expected: the DOM quotes the three sections, the dots' words, and the
disclosure; the screenshot rides `tmp/` for the visual read.

**C7 — the gate, by exit code:**
```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: every command exits 0, read directly (KI-014).

## Linked artifacts

- `docs/plans/2026-11-02-face-plugin-host.md` — the host this plan
  re-presents; the machinery is unchanged.
- `docs/plans/2026-11-01-settings-ux.md` — the revamp whose shape this
  plan applies: the answer first, the machinery behind a door, one
  section at a time, the chained setting on one surface.
- `docs/features/plugins.md` — the host's doc; the surface's section
  updated with the new shape.
- `docs/features/gui.md` — the `#/plugins` surface row.
- `FUNCTIONAL-SPECIFICATIONS.md` § "The surfaces" — the plugins panel's
  row updated.

## Risks

- **The status dot's colour is computed twice** (the bridge names the
  word, the JS renders the colour) — the bridge's value is the one
  computation; the JS maps word → palette colour, a lookup not a
  second judgment.
- **The navigation targets live in the settings page's own sections** —
  the lane cards and the MCP servers card are settings-page sections
  with their own addresses; the rows' presses ride those addresses, and
  a section that moves breaks a row — the pins name the addresses so
  the move is a caught test failure, not a silent dead press.
- **The headless capture fires before the state read** — the research's
  own trap; the probe awaits the state read (the dump-dom at a 12s
  window quoted the loaded render) and the plan's verification says
  so, so a future reader does not mistake the pre-data paint for the
  surface.

## Notes

- The owner's verdict arrived on the live face after the plugin host's
  landing; the surface's bones are wrong the same way the settings
  surface's were — this plan applies the settings revamp's own
  hard-won shape to the page that ignored it.
- The research's capture trap is recorded as a finding: Chrome's
  `--screenshot` fires at the load event, before the state read's
  repaint — `--dump-dom` with a timeout window quotes the loaded
  render; the pre-data paint reads as a bare page and is not the
  surface.
- Alternatives considered: a wizard-style install (rejected — the
  house's one-press rule; the door with the named grant is enough);
  moving the built-ins into the settings page (rejected — the plugins
  surface is the inventory the owner asked for; the settings page's
  cards stay the management destinations); a table of built-ins
  (rejected — hard rule 11's own bar: genuinely visual, never
  plain-text tables).
