---
name: gui-revamp-browser-face-decision
description: "owner ruling 2026-10-16 — the GUI revamp is a loopback web app served by the fleet binary over a conversation-first IA (inline intelligence, optional inspector, the tower as the one console); plan at docs/plans/2026-10-16-gui-revamp.md, increments G1-G10"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's 2026-10-16 directive: "revamping the gui… bring visibility to the
router, the delegation and dependencies graph, the memories, skill
management… that is what makes this harness shine and it's hidden" — then
corrected the scope twice: not the TUI, the app's FULL interface, "which
initially was meant to resemble the original deepseek harnes ui"; and "then
you have a clean slate to give me the best UI possible as you have no
baggage."

The finding that shaped the design: the fork's GUI was `dsh fleet`, a
loopback web app (default 127.0.0.1:3080) with a three-column React SPA —
sidebar, keyed main panel, right rail. Only its three dashboards ever ported
to fleet; the chat face, model selector, right panel and settings seat never
did. So the resemblance target is a web face, not a TUI restyle.

The decision, landed as a plan (docs/plans/2026-10-16-gui-revamp.md, commit
436e03e): build that face in Rust — a loopback-only HTTP server hand-rolled
on the tokio the crate already carries (zero new crates, per the crate
contract in TECHNICAL-DOCUMENTATION.md:14), hand-written assets embedded via
include_str!, SSE for live streaming, POST /api/* for actions, per-launch
token, loopback-only bind, credentials never in payloads, every write through
settings.rs's one writer. The TUI is retained and unregressed — it stays the
terminal face; deep-dive finding 1 (the settings editor cannot seed from the
provider/workload list surfaces) closes in the GUI, not the TUI.

The 2026-10-16 course correction that governs the shape: "it also needs to be
easy to use, more like codex and zcode and less than hermes, hermes UI is
horrible." So the IA is **conversation-first, not a control room**: chat is
the default and dominant surface, the fork's always-on 45% right rail became
an optional ~380px inspector drawer (closed by default), the sidebar is a
56px icon rail that expands to 280px, and the intelligence renders as compact
inline cards in the transcript (route decisions, memory recalls, skill loads,
delegation spawns — ZCode's tool-card pattern) so nothing valuable is ever a
pane you must remember to open. Esc always comes home; every surface is ≤2
actions from the conversation. Codex's quiet keyboard-first minimalism and
ZCode's conversation chrome are the references; hermes's clutter is the
anti-pattern.

The harness-landscape research ([[harness-landscape-converged-spec-2026-10]])
folds in, subordinate to ease: the trust trio (checkpoints, plan mode, diff
review — increment G9), the approval allowlist "yes, and don't ask again"
writing rules through the one writer (G2/G8), and the context meter plus
session spend as a hairline in the chat header (G2). Increments G1-G10, each
gated and each ending with the owner opening the shipped surface.

The 2026-10-16 tower ruling: "this is a dashboard example of a good design,
and what I would expect from the control tower" — sent with a screenshot of a
macOS network-analysis console (TCP-Viewer pattern: KPI stat cards with
context lines, a hero time-series, a donut, ranked list cards with bars and
shares, color-coded analysis rows, a live status bar). The tower is the ONE
console-density surface — the deliberate exception to the quiet rule; no
other surface inherits its density. Its data is grounded in recon: the
tower-log ledger (~/.second-brain/events.jsonl: ts/type/task/project/
harness/summary/commit/commit_verified) derived by control_tower.py into
tower_data.json (reports, conflicts, harness mix, report_daily, timeline),
plus boards::read_tower's live git facts. Recon found TowerDataRead carries
only generated_at/projects/timeline — the console's aggregates need a
parse-only boards.rs extension (increment G7, the boards). That extension was
too narrow, and the 2026-10-20 second-brain request re-opened it: the parse
still keeps seven fields per project while tower_data.json carries drift,
handoff next actions, open plan rows and a 400-row timeline — see
[[fleet-second-brain-is-seven-fields-in-the-gui]]. The owner then ruled the
seam is a tool PLUGIN with its own refresh actions, built as
`src/plugins/second_brain.rs` + `/api/second-brain/plugin|act` — see
[[fleet-second-brain-is-a-tool-plugin]]; that is the third plugin shape beside
channels and lanes.

Progress: G1 e4a73c8, G2 30f8294, G3 8eab88f, G4 b175ac5, **G5 dfdaefb**
(2026-10-16 — the memory and skills panels; C4 ticked). G5's two writes
ride the substrate, never around it: a fact edits in place through a
public `memory::edit` built on the substrate's own `rewrite_field` (the
name/tier/basis are not editable — a rename is a supersession), and a
proposal is approved through `improve::apply` (skills.rs has no approve
of its own), which runs the real four-step cargo gate. Operating facts:
every `fleet gui` launch mints a fresh token and prints its URL (default
127.0.0.1:3080) — there is no fixed link; and the memory panel's corpus
root resolves from the launch directory, so run `fleet gui` from a
project directory to browse that project's memories.

The looks pass began 2026-10-17 on the same owner directive ("we really
need to dig deep to make the gui more appealing and frictionless") and
has landed four increments. The research landed as
**docs/architecture/gui-design-research-2026-10-17.md** (14c3453,
distilled in [[gui-design-research-2026-10-17]]). The craft half landed
as 9c27573 — warm neutrals, 10%-white hairlines, tabular numerals, 180ms
motion, card-radius inheritance — and it fixed a latent
`var(--font-sans)` that was undefined, so two inputs had been rendering
in the browser default font. The IA landed as **cf10d41**: the ten hash
addresses grouped into four destinations (work / intelligence /
knowledge / system), one flat level beneath each, with a command palette
whose results are grouped by the same four — **one `GROUPS` declaration
builds both**, so there is a single structure and nothing to keep in
sync. The owner chose that over a flat palette and over
grouped-versus-flat as a setting, having asked whether the two could be
combined or switched: the answer given and accepted is that they are
layers (orientation vs direct access), not alternatives, and the one
switch worth offering is the rail's collapsed state, remembered in
localStorage. The light and dark themes landed as **e66740f** — the
whole light theme is one block of custom properties under
`:root[data-theme="light"]`, because a theme is the alias tier
re-pointed and nothing else; dark stays the default per the standing
ruling, the light ramp is warm paper with darkened status colors for
text contrast, and the hairline inverts with the ground (10% white on
dark, 10% black on light).

**Four themes shipped as `a882524`** (2026-10-17), and the guard that
keeps them readable is the load-bearing part. `src/gui/themes.rs` is a
`#[cfg(test)]`-only module that `include_str!`s the shipped `app.css`,
parses each `:root[data-theme="…"]` block, resolves every text token
through the `var()` chain, and fails the build when any of the seven
text tokens drops under 4.5:1 against any of the three surfaces — and
also when a theme is declared but absent from the picker, or in the
picker but undeclared, because a theme nobody can select is a theme
nobody verified. Measured worst-token-on-worst-surface: console 4.78:1,
paper 4.63:1, graphite 5.00:1, contrast 5.90:1. The guard immediately
earned its keep — console's and paper's tertiary text sat at 4.28 and
4.13 and paper's warn at 4.33, all on the *raised* surface, which is
exactly where a tile label lives. The four: **console** (warm dark, the
default), **paper** (warm light), **graphite** (cool neutral, for an
owner who finds the warm dark too brown), **contrast** (the
accessibility theme — pure ground, near-black ink). Note the attribute
is now `paper`, not `light`: the `:root` base *is* console, so the head
script sets `data-theme="console"` as the pre-JavaScript default and
the parser reads the base as console. The topbar toggle opens a picker
rather than flipping — with four themes a blind cycle is a guess — and
the picker reuses the commands' row shape so one interaction pattern
serves both.

Verified live: the picker boots on console, shows all four with their
descriptions, and choosing one sets the attribute and persists it.
Rendering the real stylesheet under each attribute gives four distinct
grounds — console (16,14,13), paper (247,244,241), graphite (13,15,18),
contrast (255,255,255). The rename from `light` to `paper` left one dead
value behind in the composer's `/theme` command, which still wrote
`data-theme="dark"` and silently reset the chosen theme; fixed in
`e05f8c3`, and the general lesson is
[[parallel-renames-leave-dead-values]].

A **right-hand collapsible pane** landed as **f4e31d5** (cmd+\\ or the
topbar button) — the first surface that lets the browser ask for a file.
It explores the tree one directory at a time, reads the actual code with
line numbers and a language, and previews a deliverable (markdown
rendered as nodes, never innerHTML, so a document cannot inject
markup). Its boundary is the load-bearing part: the root is the
directory `fleet gui` was launched from, and every path is checked
**lexically first** — so a `..` that walks out is refused for that
reason whether or not it exists — then again on the canonicalized path,
so a symlink pointing out is caught too, and a credential file is
refused by name *before anything touches the disk*. The ordering
matters: running the disk read before the checks made a nonexistent path
report "No such file or directory" instead of "outside the working
directory", a refusal that named the wrong reason. It also surfaced two
defects — the topbar's destinations and the stage's right-hand card rail
had shared the `.rail` class name (which would have stacked four
destinations vertically in a 52px bar; the topbar one is now
`.surfaces`), and the markdown renderer passed inline nodes through
`node()`, which sets `textContent`, so every heading and paragraph
rendered as "[object Object]".

**The composer's command type-ahead** landed as **d6b5c7f** (2026-10-17). A slash
opens the commands above the composer, in two passes. The lexical pass answers in
the same frame the key lands — exact name, then prefix, then alias, then
description substring, then a fuzzy character subsequence — because an owner who
knows the command expects it now, so that pass waits on nothing. The semantic pass
refines it, debounced at 180ms, ranking the command *descriptions* against the
whole intent: "make the screen darker" finds /theme, "show me the files" finds
/pane. The split of ownership is the design — the page owns the command
declarations (the actions only a page can perform), and `POST /api/rank` owns the
ranking, embedding the query and each candidate and ordering them by the cosine
`memory.rs` already uses, so there is one similarity rule in the crate and not two.
A machine with no embedding backend answers zero scores and the lexical ranking
stands. One caveat worth carrying: EmbeddingGemma's cosines do not separate well
(the right command scored ~0.79 against a ~0.70 runner-up), so the semantic pass is
a **refinement of** the lexical one, not a replacement — it earns its place when
lexical has nothing, which is exactly the case it was built for. Seven commands, all
actions rather than surfaces (/theme, /pane, /go, /compact, /clear, /approve,
/help), which is what separates them from the palette.

A coordination fact from the same day: the dogfooding session turned out to be
building the **same deck feature** from the other end — recorded in
[[fleet-deck-two-sessions-collide]], which is also the technique for holding their
hunks out of a co-edited file's commit.

The pane gained a **third tab — the web** — as **452dd60** (2026-10-17): an
address bar, a server-side fetch, and the page's prose rendered in the app's
own type. Recorded on its own in [[gui-web-pane-ssrf-guard]], because the
SSRF guard is the load-bearing part of it: a browser that can name any URL is
a browser that can name fleet's own loopback services, so every hop is checked
against the scheme and the addresses the host resolves to. That is also the
answer to "why not an iframe" — iframes cannot frame most sites at all.

Three conversation-surface directions were prototyped for the owner to
choose between — the ledger, the cards, the stream, along a density axis.
**The owner has still not picked one, and the one-renderer refactor is
blocked on that choice**, because the direction decides the renderer's
shape.

**The G4 test flake is FIXED — the plan's disclosure of it is now stale.**
`gui::bridge::tests::a_remembered_rule_stops_the_prompt_and_rides_the_rules_event`
failed roughly every other gate because the `bridge()` fixture keyed its
temp home on the process id alone, so every test in the binary shared one
home and one test's `remove_dir_all` could delete the directory out from
under another's settings write. Fixed in cf10d41 (unique home per call,
six consecutive clean runs). Do not re-diagnose it if the plan's notes
still describe it as open.

The owner's next directive after the looks pass (2026-10-17): fleet must
excel at **infographics and presentations** — a 3-stage research /
structure / render pipeline adapted to the crate contract. Recorded
separately in [[fleet-deck-visual-pipeline]].

The owner's verdict after opening G5 (2026-10-16): "good for an mvp, but
we need to work on the looks in a different session." That pass started
2026-10-17 and is recorded in [[gui-design-research-2026-10-17]]; the
guidance that came out of it still stands — the visual language is the
plan's three-tier token chain (`--dsw-static-*` → `--dsw-alias-*` →
component), dark by design per the stylesheet's own header, and the
skills-manager concept adoption already recorded in the plan's notes
(card libraries with badges, preview-before-acting, a decisions queue —
the shape is right, the finish is what the pass improves).

The owner reaffirmed the same day: "we really need to dig deep to make the
gui more appealing and frictionless." The curated sequence for that pass,
agreed in conversation: **design-an-interface** first (several radically
different directions on the table before any CSS changes — the "design it
twice" discipline, which fits the owner's clean-slate brief), then
**design-dials** to lock the direction, then **better-colors** +
**better-typography** for the token values and the type scale, then
**ai-interface-design** + **ux-writing** for the streaming states and the
microcopy, and **design-review** + **visual-output-verification** as the
acceptance gate — the GUI's equivalent of the four-leg cargo gate. All are
loadable skills at `~/.agents/skills/`, not baked-in taste; the results are
only as good as the owner's reactions to the three directions.

**G6 landed as `409910d` (2026-10-18 session).** The sessions panel
(work group) reads the archive newest-first, marks the log the face
anchors, opens a row's replay inline, and offers the RESUME — the plan's
one real architecture amendment: the browser's `Command` grew the
`Resume` arm the TUI always had, the anchored session id moved from a
Bridge field into `GuiState` because the face can now change it, and the
agent thread drops its prepared route on resume so the continued
conversation is judged fresh. One translator (records → the live
fan-out's card vocabulary) is shared by the replay preview and the
resume reseed, so the two cannot disagree; the route card is
deliberately absent from a replay because the route decision rides the
calibration stream, not the session log. Bounded reads name their
bounds: a replay previews the newest 400 cards and says what it left
out; a tool preview reuses the live path's own 80-char first line. The
calibration panel (system group) renders the day's stream grouped by op
and the fit cells straight from `calib::calibration()` — the same
computation `fleet calib --calibration` uses, so the two cannot
disagree — with the failover gate named on the board. `showRoute` now
shows panels from one `PANEL_EL` registry (the explicit hide-lists were
one surface away from showing one panel while forgetting another). Five
tests cover the archive/replay/resume/calibration reads; live smoke
answered 98 archived logs, today's 512-row tail, and named refusals.
Two G6 testing facts worth keeping: a `Server::bind` in tests answers
nothing until `tokio::spawn(server.serve())` runs (a bound-but-not-
served server returns empty replies), and a GET whose path already
carries a query must join the token with `&`, not the `get` helper's
`?token=` append.

**G7–G10 all landed 2026-10-18 and the plan is CLOSED** (status:
completed; the one criterion left is the owner's own open of every
shipped surface, C8 — the plan cannot claim that for the owner).

- **G7 `3a0e454`** — the tower console (the one console-density surface)
  and the second-brain panel, both reading the boards' own reads: git
  facts per project + the second-brain's derived state as one document
  with the plan's six zones. The boards' parses grew parse-only to carry
  the console's aggregates (reports: doing count, coverage, per-day fold,
  feed, suppressed/malformed; tower data: harness mix). The client's
  refused-read rule: a read that did not land keeps the LAST GOOD NUMBERS
  with its reason beside them — `face.tower = Object.assign({}, face.tower,
  {refused})` — never a blank console.
- **G8 `a5dbb04`** — the settings surface: the whole config as one
  document (knobs with their source named, both list editors, the rules
  as the third list, the cron jobs, the SOUL.md slot, env names by
  presence only) and every write through the writer the CLI rides. **The
  fleet.env defect is CLOSED, not disclosed**: `memory::embedding_endpoint`
  and the cua-driver resolution now walk `<FLEET_HOME>/fleet.env` behind a
  pure seam (`home::read_fleet_env`), with a round-trip test pinning
  write → read, so a GUI write finally changes what a run uses.
- **G9 `ebec815`** — the trust trio on one new module
  (`src/checkpoints.rs`): per-turn file-state capture (bounded, ignores
  named, the cap's cut named), a restore that makes the tree match the
  manifest exactly (including removing files the checkpoint never saw), a
  bounded LCS line diff with per-hunk reject, `Command::Plan` and
  `Command::Rewind` arms, and a `Record::Rewind` the next reseed cuts at
  (`session::rewind_cut`). Plan mode refuses the write-class tools by
  name at the gate and child delegations inherit the floor. Every face
  that drives the agent (GUI, TUI, headless `fleet run`) checkpoints
  under the home's own store.
- **G10 `f63e585` + `0a7a86f`** — docs: `docs/features/gui.md` (the seam,
  the ten surfaces, the trio, the reading rules), the module-reference
  rows, the `fleet gui` command in the functional spec and README, and
  boards.md's deferred-browser-face paragraph superseded with its reason.

G7–G10 testing facts: serde_json's compact serialization sorts object
keys alphabetically, so wire-form assertions must match that order
(`"count":3,"day":"…"`, not the source order); a test needing a
different cwd gets a `#[cfg(test)]` seam, never `set_current_dir`
([[tests-get-a-seam-never-a-process-global]]); and each increment's
client verification is its own harness file in `tmp/`
([[gui-client-verified-by-a-vm-harness]]). Still open from the research
brief: the one tool-card renderer with the AI SDK state machine, blocked
on the owner's pick among the three conversation-surface directions.

**The rail's tree was silently stripped by `f4e31d5` — diagnosed 2026-10-19.** The rail landed at `cf10d41` (10-17 10:20) rendering the design's full tree: group headings, a `rail-sub` div with a `rail-link` per surface, and a "⌘ jump to anything" opener, in a left sidebar (`.rail`, flex-column). `f4e31d5` — the right-hand pane commit, **43 minutes later** — rewrote `buildRail` to render only the four destination buttons as a topbar strip (`.surfaces`, flex-row) and deleted the headings, the nested links, the opener, and `markRail`'s link-marking loop. Three marks of a silent strip rather than a decision: the commit message describes only the pane; the comment above `buildRail` still promises "one flat level beneath each"; and `app.css` still styles `.surfaces-group`/`.surfaces-link`/`.surfaces-palette`, elements the JS never creates. The owner saw the shipped face on 2026-10-19 ("no left side rail, no grouping, nothing — not what I approved") and it matches the four-button strip exactly. What still works: the palette (⌘K, all ten surfaces grouped by destination), the pane (⌷/cmd+\, files/preview/web), themes, composer type-ahead, action card, the panels, the trust trio — and `GROUPS` still declares the full tree; only the rail's rendering of it is gone. Restore = cf10d41's `buildRail`/`markRail` verbatim + the sidebar CSS + the nav's class. **RESTORED 2026-10-19, commit `06ca78d`**: the rail is the face's first grid column (`--rail-width: 208px`, 56px collapsed) spanning every row — the mark at its head, per group a heading + a dest row with the approved mockup's merged names (`DEST_LABELS`) + the group's surfaces flat beneath (`SUB_LABELS` gives chat its "the run in flight" words display-only), and the foot's jump-to-anything (opens the palette) and collapse switch. The stylesheet's tree rules had survived the strip unused, so the restore was JS + layout; `buildRail` fills `#rail-tree` (a static child — the vm harness's DOM stub has no `document.querySelector`, so element lookups in the el map must be `getElementById`). Gate green 1121; verified in Playwright: 4 groups, 10 links, collapse to 56px icons, pane takes its own column beside the rail. Unrelated and by design: `fleet gui` opening a browser tab is plan G1's architecture (loopback server in the Rust binary + `open`, zero new crates), not a regression.

**The owner's feature-by-feature audit of the GUI session, opened 2026-10-19.** The rail strip broke the owner's confidence in the whole session's claims, so they are feeding me their original feature requests one at a time and I verify each LIVE (Playwright against a real `fleet gui` + curl at the API level), never from commit messages. **Feature #1 — "a right side collapsible pane: preview deliverables, view actual code, explore files" — VERIFIED FULLY IMPLEMENTED AND WORKING**: pane opens/collapses (⌷, cmd+\, ›, own 420px grid column → rail|stage|pane), tree explores one directory at a time with a breadcrumb and a 400-entry named cap, code viewer renders real line numbers + language labels (9 extensions, non-UTF8 refused), preview renders true DOM nodes from markdown (never innerHTML), web tab's SSRF guard refuses loopback by name and real pages fetch. Security fired for real: `.env` refused by name whether or not it exists, `../` escapes refused lexically before disk, symlinks caught by canonicalize. Two nits, unfixed: Esc closes the pane only when NO file is open (the code comment states the opposite — more comment drift), and the credential refusal (basename `.env`/`.env.*`) does NOT catch `.envrc`. The approved design mockup the owner approved against lives at `~/.zcode/v2/visualizations/2d1ddb4a88a5c288f85cc5637e413253a794b1b97ae59e69fb8a0d5daa0ff5f9/fleet-gui-combined.html` (variants: both-always / rail-collapsed / palette-grouped). More features (#2, …) expected.

**The 2026-10-19/20 editorial-desk redesign SUPERSEDES the dark-by-default look and the "owner has not picked a conversation-surface direction" blocker.** The owner ruled the shipped face "meh… amateur hour" and directed: paper first, not dark by default ("that's a lazy and amateurish way of designing ui"); a home that answers what fleet knows about the owner, their work, their workflow, and how it reduces the grind; every displayed object must have an interaction layer; and it must make them want to return daily (12-hour sessions). That pass is recorded in [[fleet-gui-editorial-desk-redesign]]. The three-direction pick still never happened — the redesign went straight to a conversation surface in the cards shape, so the one-renderer refactor is no longer blocked on a choice, it is subsumed by the new direction. Also: an unattended run committed the owner's dirty tree mid-pass ([[unattended-runs-can-commit-your-worktree]]).

**Why:** the intelligence that makes fleet valuable was invisible outside CLI
output and a 42-column board pane; the fork's own UI proved the shape; and a
web face is the only one that can render the delegation graph, memory facts
and skill curation visually without fighting crossterm.

**How to apply:** implement under the plan, not around it. Never serve
beyond loopback; never add a crate; never write config from the GUI except
through settings.rs; update the plan's `updated` field as increments land.
Related: [[fleet-memory-corpus-dirties-the-tree]], [[fleet-routes-through-its-own-router]],
[[canonical-skills-library-agents-skills]], [[deep-dive-2026-10-15-second-pass-findings]],
[[gui-design-research-2026-10-17]], [[harness-landscape-converged-spec-2026-10]],
[[fleet-deck-visual-pipeline]], [[fleet-deck-two-sessions-collide]].
