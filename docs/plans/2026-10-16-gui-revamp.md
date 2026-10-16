---
status: active
created: 2026-10-16
updated: 2026-10-16
slug: gui-revamp
---

# Plan: The app's GUI — the browser face lands

## Context

Fleet's interface today is a ratatui TUI, a `fleet chat` REPL, and about
twenty CLI subcommands. The intelligence that makes this harness what it
is — the router's decisions, the delegation tree, the memory substrate,
the 441-skill library, the calibration stream — is reachable but hidden.
It lives in CLI output, transient tool cards, and a 42-column board pane.
The owner's directive for this session: revamp the GUI — the **full
interface of the app**, not only the TUI — make the router, the
delegation and dependencies graph, the memories, and skill management
visible, and give the GUI a way to configure every configurable surface.
The app was originally meant to resemble the DeepSeek harness UI, and
since no web UI ever shipped in Rust, the owner granted a clean slate:
design the best interface possible, unconstrained by what exists.

The owner then set the bar for what "best" means: "it also needs to be
easy to use, more like codex and zcode and less than hermes, hermes UI
is horrible" (2026-10-16). **Ease of use is the governing
requirement.** Codex's quiet, keyboard-first minimalism and ZCode's
conversation-first chrome are the positive references; hermes's
cluttered style is the anti-pattern to avoid. The deep research behind
this session (the converged spec of ~12 harness interaction patterns,
saved as the `harness-landscape-converged-spec-2026-10` memory) folds
into the criteria below — the trust trio (checkpoints, plan mode, diff
review), the approval allowlist, the context meter, and spend
visibility — each subordinate to ease, never as a cockpit.

The owner then named the one surface that earns console density, with
a screenshot of a macOS network-analysis dashboard as the reference:
"this is a dashboard example of a good design, and what I would expect
from the control tower" (2026-10-16). The tower — the observability of
the whole system: what every project shipped, which harnesses are
reporting, which claims conflicted with evidence, what is open right
now — is the deliberate exception to the quiet rule. Chat stays the
app; the tower is where the cockpit lives, because a control tower
that isn't one would betray the name.

The reference is on disk at `~/Projects/deepseek-harness`. Its GUI was
`dsh fleet` — a web profile that started a loopback HTTP server
(127.0.0.1:3080), opened the browser, and rendered a React SPA in a
three-column frame: a 280px sidebar (brand, new session, sessions,
bottom-pinned settings, board icons), a keyed main panel with chat as the
default, and a 45%-viewport right panel. Only its three dashboards
ported to Fleet's TUI; the chat surface, the model selector, the right
panel, and the settings seat never did.

`docs/features/boards.md:25-28` deferred the browser face because "a
server needs dependencies this crate does not carry." That reason no
longer holds: tokio is already a dependency (`Cargo.toml`), and this
crate hand-rolls its JSON-RPC transport, its cron parser, and its boards
refresher. The settings-surface plan anticipated this session explicitly:
"the GUI session builds on a finished contract rather than inventing one"
(`docs/plans/2026-10-15-settings-surface.md:19-20`).

## Approach

**The decision.** The app's GUI is a local web application served by the
fleet binary itself: loopback-only, zero new crates, hand-written assets
embedded in the binary, live over server-sent events, every write through
the one settings writer the CLI shares. The TUI remains the terminal face
over the same state and loses nothing. The crate contract — "One static
Rust binary; no Node runtime" (`TECHNICAL-DOCUMENTATION.md:14`) — is
honored, not bent: the assets compile in, the server is ours.

**The governing principle — the conversation is the app.** The first
screen is a conversation with a focused composer, not a dashboard. The
intelligence is ambient: the router's decision, the memory recalls, the
skill loads, and the delegation spawns render as compact cards inside
the transcript as they happen (ZCode's tool-card pattern), so nothing
that makes this harness valuable is ever a pane you must remember to
open. The panels are destinations for deep work, reached from a rail
that collapses to icons; the inspector drawer is optional and closed by
default. Every surface is two actions from the conversation and Esc
always comes home. This is the Codex/ZCode shape — one obvious action
per screen, quiet chrome, keyboard-first — and the explicit rejection
of hermes's clutter. One surface is the named exception: the tower is
a console because the owner asked for a console, and every other
surface stays quiet.

### The seam

- `fleet gui` starts three things in one process: the HTTP server, the
  agent bridge (the `UiEvent` fan-out the TUI's `spawn_agent` already
  owns), and the browser.
- Server: hand-rolled HTTP/1.1 on tokio's `TcpListener`. GET for pages
  and JSON reads, `GET /events` for the SSE stream, `POST /api/*` for
  actions. No WebSocket — nothing needs client-originated streaming.
- Assets: hand-written HTML/CSS/JS under `src/gui/assets/`, embedded with
  `include_str!`. No bundler, no framework, vanilla ES modules.
- Auth: a per-launch token printed with the URL and required on every
  route; the bind is loopback-only and a non-loopback `--host` is refused
  by name.
- The model-visible prompt names the face, as the fork's did: "You are
  interacting with the user through the Fleet GUI at \<url\>."
- Credentials: no key value ever enters a payload — names and presence
  only, the doctor's masking rule. A test mirrors the TUI's no-token
  board test.

### The information architecture

The frame at rest — one conversation, an icon rail, nothing else:

```
┌────┬──────────────────────────────────────────────┬──────────────┐
│ ◈  │ chat — session · route auto · $0.42 · ctx ▓▓░│ ▸ inspector  │
│    │ ────────────────────────────────────────────│   (closed)   │
│ ▣  │ ▸ you    triage the KI ledger               │              │
│ ▤  │ ▸ fleet  …                                   │              │
│ ▦  │   ╭ tool · read ───────────────────╮         │              │
│ ▧  │   │ done · preview…                │         │              │
│ ▨  │   ╰────────────────────────────────╯         │              │
│ ▩  │ ▸ fleet 84 rows are open…                    │              │
│    │   ╭ ⚡ route · code → step-5-preview ╮       │              │
│ ⚙  │   ╰──────────────────────────────────╯       │              │
│    │ ── message ──────────────────────────────── │              │
└────┴──────────────────────────────────────────────┴──────────────┘
```

- **Sidebar** (56px icon rail, expands to 280px on click or Cmd+B):
  collapsed at launch. The fleet brand, new session, the sessions list,
  the surface list, settings pinned at the bottom, and a footer carrying
  the routing mode and the working dot (the TUI status bar's KI-217
  rules). Navigation only — the rail is never a content surface.
- **Main panel** (keyed; chat is the default and the home): one panel
  per surface.
- **Inspector drawer** (~380px, right, closed by default, toggled with
  `i` or the header button): the details behind the cards in view — the
  full route row, the delegation subtree, the recall hits behind a
  memory card. The fork's always-on 45% rail became this drawer: live
  intelligence rides *inside* the transcript as compact cards, so the
  default view already answers "it's hidden" and the drawer is where
  you go deeper, not where you must live.

The panels — chat is the app; the rest are destinations you choose:

- **chat** — the conversation and the default screen. Header (session,
  routing, this session's spend, the context meter), the transcript with
  its inline intelligence cards, tool cards with their four states, the
  compaction, lane, skills, and cron cards, streaming tokens, the
  composer, and the approval seat inline when a tool asks. The inline
  cards are the load-bearing part: a route decision, a memory recall
  set, a skill load, and a delegation spawn each render as one compact
  card at the moment it happens, expandable to the inspector. The
  intelligence is visible without opening anything — the Codex/ZCode
  lesson that the interesting thing must never require a detour.
- **router** — the declared chain, the workload preference table, the
  live decision of the run in flight, today's route rows, failover
  history, cache stats. Cards, bars, a day column chart, a model-mix
  donut — hand-rolled SVG in the fork's board card language.
- **delegation** — the delegation and dependency graph: the live run's
  tree (parent to children, the depth-2 cap visible, model and status per
  node) and the dependency graph with the unlock conditions the code
  actually requires. SVG nodes and edges — a graph, not a table.
- **memory** — the corpus (one fact per file), the recall hits of the
  last run, recall counts, search. A fact edits in place through the
  substrate's own rewriter, never around it.
- **skills** — the library with its loaded/skipped split, the proposals
  awaiting a decision, the curation candidates. Approve and retire go
  through skills.rs's own functions.
- **sessions** — the archive, a session replayed in the browser, resume.
- **calibration** — the day's stream rows, the fit samples, the gate
  verdicts, the budget lines.
- **settings** — every configurable surface: the ten scalar knobs with
  their source (default / file / env), the provider and workload lists
  with structured editors, cron jobs (a surface they have never had),
  SOUL.md, and the env status by name only. Every write goes through the
  one writer the CLI shares.
- **tower** — the control tower, and the one console surface (the
  owner's named bar). Six zones, every number from a read: a KPI strip
  (projects tracked, open tasks, open conflicts, reports today, dirty
  worktrees — each a big number, a 12px label, one context line); a
  hero two-series chart (reports filed per day against commits shipped
  per day, with a legend); the harness-mix donut; the ranked projects
  (trailing-30-day commits, bar, share, branch, head, a dirty badge,
  ahead/behind, a health dot by lifecycle); the claims-vs-evidence rows
  (each open conflict's claim against the reality that rejected it,
  color-coded); and the report feed (every filed report, newest
  first). A status bar names the snapshot's build time, the malformed
  lines, the coverage, and the suppressed count. The feed is as fresh
  as the derived state's last build and says so — it never pretends to
  be live. A refused read keeps the last good snapshot and names why; a
  bounded scan names what it left unread.
- **second-brain** — the derived state: the gates board, the loops,
  the maturity series, the harness runs, and the knowledge corpus with
  its classified rows. The same card language, quieter — cards, bars,
  and a week grid, never a plain-text table.

### The visual language

Quiet by default. One accent, generous whitespace, and decoration only
where it carries information — the Codex reference: a surface that says
nothing it doesn't mean. The conversation follows the ZCode reference:
markdown-native rendering, compact tool cards, a composer that owns the
bottom of the screen. The fork's token chain in one stylesheet:
`--dsw-static-*` palette →
`--dsw-alias-*` semantics → component tokens; light by default, dark via
a theme attribute; the DeepSeek blue accent (rgb(65,118,230)); the fleet
gradient brand; 12px card radius, 0.5px neutral hairlines, the SF text
stack with SF Mono for code, tabular numerals for values, 22px card
values over 12px tertiary labels. Cards, dots, bars, and graphs — never a
plain-text table, the house UX bar (`CLAUDE.md:39`). The tower is where
this density finally lands in full: KPI cards, tabular numerals
throughout, bars and shares and health dots, the donut and the
two-series chart — a console in the same card language, and the only
surface allowed to be one.

### What the TUI keeps

Everything. The TUI is the instant, keyboard-first, ssh-able face; it
shares the state layer and regresses on nothing. The GUI is where the
surfaces a terminal cannot hold honestly — a dependency graph, a
441-skill library, a structured provider editor — get room. The settings
pane's known gap (deep-dive finding 1: the pane cannot reach the
list-shaped surfaces its `commit_setting` already branches for) closes
here with the structured editors; until then the honest minimum is the
JSON buffer the CLI already accepts.

### Increments

- **G1 — the seam.** `fleet gui` boots the loopback server, serves an
  embedded page, streams a heartbeat over SSE, prints the URL with the
  token, opens the browser unless `--no-open`, and injects the GUI prompt.
- **G2 — the conversation.** The home surface over the `UiEvent` stream:
  transcript, tool cards, composer, streaming, the inline intelligence
  cards (route, memory, skills, delegation), the inspector drawer, the
  context meter and the session spend line in the header, and the
  approval seat round-tripped over POST — with the persistent allowlist
  ("yes, and don't ask again") writing rules through the one writer, so
  prompts thin out over time. This increment is where the Codex/ZCode
  bar is met or missed; it ends with the owner running a task from the
  browser and touching nothing else.
- **G3 — router.** The router panel: the board's reads plus the live
  decision and the cache stats.
- **G4 — delegation.** The tree and the dependency graph for the live run
  and its history; the graph.rs unlock-condition doc lag is fixed while
  the graph renders the code's actual conditions.
- **G5 — memory and skills.** The corpus browser, recall hits, fact
  editing through the substrate's rewriter; the library, the proposals,
  the curation.
- **G6 — sessions and calibration.** The archive, replay, resume; the
  stream rows and gates.
- **G7 — the boards.** The tower console (the six zones above, from the
  boards' own reads, the snapshot contract holding) and the
  second-brain panel. The tower_data parse grows to carry the console's
  aggregates — parse-only, the read contract untouched.
- **G8 — settings.** The full config surface through the one writer; the
  structured provider and workload editors; the allowlist rules as a
  third list surface; cron jobs; SOUL.md; and the fleet.env defect
  (below) closed or disclosed by name.
- **G9 — the trust trio.** Checkpoints (a per-turn snapshot of the run's
  file state, rewindable — the converged spec's most-requested gap),
  plan mode (read-only planning, then an editable plan, then Build —
  Cursor's plan-as-document shape), and diff review with per-hunk
  accept. All three ride the conversation; none adds a permanent panel.
- **G10 — hardening and docs.** Token UX, error surfaces, the eval
  scenarios, the docs, the TUI-parity check, the recap.

Each increment ends the same way: the four-leg gate, an explicit-path
commit, and **the owner opens it** — the settings-surface plan's
disclosed gap ("the TUI pane has never been opened by a human,
`docs/plans/2026-10-15-settings-surface.md:153-157`) does not repeat.

## Use cases

- [ ] A: The owner opens `fleet gui` and is in a running task within
  seconds — composer focused, one keystroke to send, nothing to
  configure first — and watches the router decide, memory recall, and
  skills load as inline cards in the conversation. The intelligence
  that was hidden is visible without opening anything.
- [ ] B: The owner configures every surface from the browser — a knob, a
  provider, a workload, a cron job — and the CLI and `fleet doctor` see
  the change without a restart or a second source of truth.
- [ ] C: A delegated run shows its tree, and the dependency graph names
  what unlocks what.
- [ ] D: No credential ever reaches the browser — names yes, values
  never; the server binds loopback only and refuses anything else.
- [ ] E: The TUI is unregressed — every existing test passes and the
  board pane and settings pane behave as before.
- [ ] F: The face the model sees names the GUI, so the agent knows its
  interface.
- [ ] G: A tool asks for approval mid-run; the owner answers from the
  keyboard, picks "don't ask again" for the routine class of call, and
  the rule persists through the one writer — later runs of that class
  never prompt, and the active rules stay visible where the trust state
  lives.
- [ ] H: A run goes sideways; the owner rewinds to the turn's
  checkpoint, switches the next stretch into plan mode, edits the plan
  the agent wrote, and builds — then reviews the resulting diff hunk by
  hunk, accepting some and rejecting the rest.
- [ ] I: The session header always shows the spend so far and the
  context meter, so "where am I" never requires opening a panel.
- [ ] J: The owner opens the tower and reads the whole system at a
  glance — what each project shipped in the last thirty days, which
  harnesses are reporting, which claims conflicted with evidence, what
  is open right now — one screen, no panel-hopping, every number
  traceable to a read.

## UX routes

No URL routing — keyed panels with hash addresses, as the fork's panel
ids worked:

| Route | Panel | Data source |
|---|---|---|
| `#/chat` | conversation | the `UiEvent` stream; the session log on resume |
| `#/router` | router | `boards::read_router`'s reads plus the live `Routed` event |
| `#/delegation` | delegation + dependency graph | the live run's `Subagent` events; `graph.rs`'s unlock conditions |
| `#/memory` | memory corpus + recall hits | the memories tree; the `Memory` event |
| `#/skills` | library, proposals, curation | `skills.rs` load report, proposals, curation |
| `#/sessions` | archive, replay, resume | `session::list` / `session::replay` |
| `#/calibration` | stream, fit samples, gates | `calib::read_day` and the gate state |
| `#/settings` | every configurable surface | `settings::states`, the list surfaces, `jobs.json`, SOUL.md |
| `#/tower` | the control-tower console | `boards::read_tower` plus `read_brain`'s tower_data section, extended parse-only to carry the reports, conflicts, harness, and report_daily aggregates |
| `#/second-brain` | the derived state + knowledge corpus | `boards::read_brain` unchanged |

`#/chat` is the launch route. The expanded sidebar lists the surfaces in
the order above — chat and the four intelligence surfaces first, the
boards last, settings pinned at the bottom — and the inspector drawer is
a view state over any route, not a route of its own.

Launch: `fleet gui [--port N] [--no-open] [--host H]`, default
127.0.0.1:3080 (the fork's port), `--port 0` for an OS pick.

## Acceptance criteria

- [ ] C0: `fleet gui` boots, binds loopback only, prints the URL with a
  token, and opens the browser unless `--no-open`; a non-loopback host is
  refused by name.
- [ ] C1: the chat surface streams a real run over SSE — tokens, tool
  cards, an approval round-trip — verified headless against the mock
  server, preserving the standing zero-real-network-in-tests invariant.
- [ ] C2: the router panel renders the chain, the workload table, and the
  live decision from the board's own reads; a refused read keeps the last
  snapshot and names why.
- [x] C3: the delegation panel renders the live tree and the dependency
  graph with the code's actual unlock conditions, and the graph.rs doc
  lag is fixed.
- [x] C4: the memory and skills panels render the corpus, the recall
  hits, and the library with its skips; a proposal is approved from the
  browser through skills.rs's own function.
- [ ] C5: the settings surface writes every knob and both lists through
  the one writer; the CLI and the TUI pane see the change; the fleet.env
  defect is closed or disclosed by name.
- [ ] C6: no credential value reaches any payload, the token gates every
  route, and tests assert both — mirroring the TUI's no-token test.
- [ ] C7: the four-leg gate is green (fmt, clippy, build, test), read by
  exit code, never through a status-losing pipe (KI-014).
- [ ] C8: the docs land and the owner has opened every shipped surface.
- [ ] C9: the app opens on the conversation with the composer focused;
  a message sends with one keystroke and zero prior configuration; the
  first-run screen has no dead end and no required tour.
- [ ] C10: every surface is reachable from the conversation in at most
  two actions, every panel has a visible way back, and Esc returns to
  the conversation from anywhere — including from a running turn.
- [ ] C11: the router decision, memory recalls, skill loads, and
  delegation spawns render as inline cards in the transcript without
  opening a panel; the inspector drawer defaults closed; the panels are
  deep-dive destinations, not the default view.
- [ ] C12: an approval is answerable without the mouse and offers a
  persistent "don't ask again" that writes an allowlist rule through the
  one writer; the active rules are visible from the conversation's
  header and editable in settings; a re-run of an allowed call never
  prompts.
- [ ] C13: a turn's file state is checkpointed and rewindable; plan mode
  produces an editable plan before execution and Build executes it; a
  diff renders with per-hunk accept and reject.
- [ ] C14: the session spend and the context meter render in the chat
  header from the run's own data — no panel required, no fabricated
  numbers.
- [ ] C15: the tower renders the console from the boards' own reads —
  the KPI strip, the two-series activity chart, the harness donut, the
  ranked projects with bars, shares and health dots, the
  claims-vs-evidence rows, the report feed, and the status bar. Every
  number traces to a read and none is fabricated; a refused read keeps
  the last good snapshot and names why; a bounded scan names its unread
  count rather than hiding it; the feed names its build time. The
  console density is the tower's alone — no other surface inherits it.

## Files to be touched

**New (Rust):**
- `src/gui/mod.rs` — the module, the `serve` entry, flag parsing
- `src/gui/server.rs` — the hand-rolled HTTP/1.1 loop, routing, the token gate
- `src/gui/sse.rs` — the broadcaster over the `UiEvent` stream
- `src/gui/api.rs` — the JSON actions: run, approve, resume, settings writes
- `src/gui/state.rs` — the GUI's view state fed by `UiEvent` (the browser's app.rs)
- `src/checkpoints.rs` — the per-turn file-state snapshots and the rewind
- `src/gui/assets/` — `index.html`, `app.css`, `app.js` and the per-panel
  modules, embedded with `include_str!`

**Touched:**
- `src/main.rs` — the `gui` subcommand dispatch
- `src/cli.rs` — the grammar for `fleet gui [--port N] [--no-open] [--host H]`
- `src/agent.rs` or a new `src/bridge.rs` — the `UiEvent` fan-out both
  faces share; the GUI face's prompt injection
- `src/settings.rs` — the structured provider and workload editors
  (fields through the writer's own validation) and the fleet.env fix
- `src/approvals.rs` — the allowlist rules as a third list surface
  through the writer; the ask/yes/yes-guarded gate consults the rules
  before it prompts
- `src/home.rs` — the resolution chain walks `fleet.env` for the two
  GUI-writable keys (the defect close), or the writes stop; G8 records
  which
- `src/graph.rs` — the unlock-condition doc correction
- `src/boards.rs` — the tower_data parse carries the reports, conflicts,
  harness, and report_daily aggregates the console renders (parse-only;
  the read contract, the bounds, and the snapshot rules unchanged)
- `docs/features/gui.md` (new), `README.md`,
  `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
  `docs/features/boards.md` (its deferred-browser-face paragraph is
  superseded — the doc says so)

**Tests:**
- `src/gui/*` — loopback-only bind, the token gate, no credential in any
  payload, SSE ordering
- `tests/integration.rs` — a headless run against mock-server streaming
  over SSE, and an approval round-trip
- the existing TUI tests stay green, untouched

## Out of scope

- Serving beyond loopback — the delivery-seam ruling; the bind refuses
  non-loopback by name.
- Replacing or regressing the TUI.
- A component framework, a chart library, or any new crate.
- Auth beyond the launch token; multi-user; remote access.
- Mobile layout — desktop browser first, the sidebar collapses.
- Attaching the GUI to a run owned by another process; a TUI session and
  a GUI session are separate processes over the same files, and the
  calibration stream already gives cross-process visibility.

## Verification

**Criterion C0: the seam binds and gates**
```bash
cargo run --bin fleet -- gui --port 0 --no-open
curl -s http://127.0.0.1:<port>/            # without the token
```
Expected: the URL line prints with the token; the bare curl is refused
(401); `--host 0.0.0.0` is refused by name.

**Criterion C1: a run streams headless**
```bash
cargo test --test integration gui::
```
Expected: a scripted run's tokens, tool cards, and approval round-trip
arrive over SSE against mock-server; exit 0.

**Criterion C2/C3/C4: the panels render the real reads**
```bash
cargo test --lib gui::
```
Expected: the router, delegation, memory, and skills panel tests pass
against fixtures derived from the modules' own readers.

**Criterion C5: settings round-trip**
```bash
cargo run --bin fleet -- settings set approvals yes
cargo run --bin fleet -- settings list
```
Expected: a GUI write and a CLI write are the same file; both faces see
both.

**Criterion C6: no credentials, token everywhere**
```bash
cargo test --lib gui::no_credential
```
Expected: no key value in any payload; every route answers 401 without
the token.

**Criterion C7: the gate**
```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo fmt --all --check; echo "fmt=$?"
cargo clippy --all-targets -- -D warnings; echo "clippy=$?"
cargo build --release; echo "build=$?"
cargo test; echo "test=$?"
```
Expected: every leg 0, read directly.

**Criterion C8: docs and the human open**
Inspect `docs/features/gui.md`, README, and the two specification docs;
the owner's own open of each shipped surface is the criterion.

**Criteria C9-C11: the ease bar (the owner's open)**
The owner opens `fleet gui` fresh and: sends a message with Enter alone;
reaches every surface from the conversation in ≤2 actions; Esc returns
home from every screen, including a running turn; the router, memory,
skill, and delegation events appear as inline cards with no panel
opened; the inspector stays closed until asked for. A first-time open is
the criterion — no tour, no dead end.

**Criterion C12: the allowlist round-trip**
```bash
cargo test --lib gui::allowlist
cargo run --bin fleet -- settings list
```
Expected: an approved "don't ask again" rule lands through the one
writer, the CLI sees it, and the covered call never prompts again.

**Criterion C13: the trust trio**
```bash
cargo test --lib gui::checkpoint
cargo test --lib gui::plan_mode
cargo test --lib gui::diff_review
```
Expected: a checkpoint rewinds the run's file state; a plan-mode plan
edits and builds; a diff accepts and rejects per hunk.

**Criterion C14: spend and context, visible**
```bash
cargo test --lib gui::header
```
Expected: the chat header renders the session spend and the context
meter from the run's own counters, with no panel opened.

**Criterion C15: the tower console**
```bash
cargo test --lib gui::tower
```
Expected: the console's zones render from a fixture derived from the
boards' own readers (TOWER_DATA_FIXTURE and a tower snapshot); the KPI
numbers equal the fixture's counts; a truncated scan names its unread
count; the feed names its build time; exit 0.

## Linked artifacts

- `~/Projects/deepseek-harness` — the IA and visual-language reference
  (read-only; nothing is imported)
- `docs/plans/2026-10-15-settings-surface.md` — the finished TUI
  settings contract this builds on
- `docs/architecture/deep-dive-2026-10-15.md` — the surfaces inventory
  and finding 1
- `docs/features/boards.md` — the snapshot contract the panels reuse

## Risks

- The hand-rolled server's scope creep. The contract is GET pages and
  JSON, GET SSE, POST JSON actions; anything past that — uploads,
  WebSocket, range requests — is a signal to stop and re-decide, not to
  grow the server.
- Asset size. Embedded assets ship inside the binary; the budget is a few
  hundred KB of hand-written code, and the gate's build time is the
  tripwire.
- The dogfooding session is executing the KI-triage plan in this same
  tree. The GUI lands in new files plus small, named edits, gated and
  committed with explicit paths to stay clear of it.
- rustfmt and clippy were absent on this machine at the last deep dive;
  if that recurs, style is matched by hand and the absence is disclosed.
- Control-room creep. Ten panels of live intelligence are a cockpit,
  and the owner named hermes as the anti-pattern for exactly this. The
  guard is structural, not vigilance: intelligence renders inline first
  (C11), the drawer defaults closed, and any new surface must justify
  itself against C9 and C10 before it earns a rail slot. The tower is
  the named exception — the owner asked for a console there by name —
  so the guard is that no other surface inherits the tower's density.

## Dependencies

- `src/boards.rs` — the three boards' reads land as panels unchanged.
- `src/settings.rs` — the one writer every settings write goes through.
- The fork's web roster at `~/Projects/deepseek-harness`
  (`apps/web`, `packages/client/*`, `packages/fleet/dsh-*`) — read for
  this plan's IA and token mapping.

## Notes

- Recon provenance: this plan's design reads the fork's web roster, the
  TUI (`src/tui/*`), `src/settings.rs`, `src/boards.rs`, the
  settings-surface plan, and the 2026-10-15 deep dive.
- The boards.md deferral is superseded with its reason stated: tokio is
  already carried and the server is hand-rolled, so "dependencies this
  crate does not carry" no longer applies. G10 corrects the doc.
- G1 dependency disclosure: `TcpListener` needs tokio's `net` feature and
  a clean Ctrl-C shutdown needs `signal`. Both are features of the crate
  tokio this project already carries — no new crate enters the tree, so
  the zero-new-crates claim stands. The features were added in G1 and
  named here rather than discovered later.
- **The owner's 2026-10-16 course correction, verbatim: "it also needs
  to be easy to use, more like codex and zcode and less than hermes,
  hermes UI is horrible."** Codex and ZCode are the positive references;
  hermes is the anti-pattern. The correction reshaped the IA: the
  always-on right rail became an optional inspector drawer, chat became
  the default and dominant surface, and the inline intelligence cards
  became the default answer to "it's hidden."
- **The owner's 2026-10-16 tower reference, verbatim: "this is a
  dashboard example of a good design, and what I would expect from the
  control tower"** — sent with a screenshot of a macOS network-analysis
  console (the TCP-Viewer pattern: KPI stat cards with context lines, a
  hero time-series, a donut, ranked list cards with bars and shares,
  color-coded analysis rows, a dense sidebar, a live status bar). The
  tower is this plan's one console-density surface; the quiet rule
  holds everywhere else, and the control-room-creep risk is amended to
  name the exception.
- Recon grounding for the tower's data. The tower-log ledger
  (`~/.second-brain/events.jsonl`, one JSON line per report: ts, type,
  task, project, project_known, harness, summary, and the optional
  commit, commit_verified, and session) is derived by
  `control_tower.py` into `tower_data.json`: the reports (doing,
  pending_verification, sessions, reports_recent, the conflicts with
  their claim-vs-reality, open_conflict_count, suppressed, trust,
  coverage, report_daily), the harness mix, the projects, and the
  shipped timeline. `boards::read_tower` adds the live git facts per
  project (branch, head, when, commits_30d, dirty, ahead/behind,
  notes, and the scan's beyond_bound and unread). Recon found
  `TowerDataRead` carrying only generated_at, projects, and timeline —
  the console's aggregates need the parse-only extension named in
  Files. The tower's feed is as fresh as the derived state's last
  build; the panel names its generated_at rather than pretending to be
  live.
- The harness-landscape research (memory:
  `harness-landscape-converged-spec-2026-10`) folds into this plan as
  the trust trio (G9: checkpoints, plan mode, diff review), the approval
  allowlist (G2 and G8, C12), and the context meter and spend line (G2,
  C14). All of it is subordinate to ease — the meter is a hairline in
  the header, not a dashboard tile.
- **The fleet.env defect, found during recon:** `settings::write_env`
  persists `FLEET_EMBEDDING_ENDPOINT` and `FLEET_CUA_DRIVER` into
  `<FLEET_HOME>/fleet.env`, but nothing in the runtime reads that file —
  `memory.rs` and `agent.rs` read the process env only. An edit from the
  TUI pane (or the GUI) changes what the pane shows, not what a run uses.
  G8 closes it — the resolution chain walks fleet.env for those two keys
  — or discloses it by name; it must not ship papered over.
- **A G4 gate flake, disclosed rather than fixed inside the increment.**
  The first full gate run failed one test —
  `gui::bridge::tests::a_remembered_rule_stops_the_prompt_and_rides_the_rules_event`
  — with `rule` coming back null. It is not a delegation defect: the
  `bridge()` test fixture keys its temp home on the process id alone
  (`src/gui/bridge.rs:1094`), so every test in the binary shares one
  home and one test's `remove_dir_all` can delete the directory out
  from under another test's `settings::allow_tool` write. It is
  timing-dependent and passes on re-run (three clean full runs
  followed, 1070 passed). The fixture needs a per-test home, which is
  a test-hygiene change of its own; it is named here so it is not
  mistaken for a product defect later.
- **The skills-manager reference, adopted as a concept and refused as a
  dependency.** The owner pointed at
  `github.com/xingkongliang/skills-manager` (a Tauri 2 desktop app: React
  19, TypeScript, Vite, SQLite/rusqlite, react-i18next) that manages and
  syncs skills across 50+ coding tools. What transfers is the UX shape,
  and G5 is built on it: a card library with per-item badges (loaded /
  skipped, kind, usage count), preview-before-acting in the inspector, a
  decisions queue as a first-class card, an activity trail (fleet already
  has one — the calibration stream's `propose` and `recall_hit` rows),
  and search. The memory panel is the same concept extended to facts,
  and it gets something skills-manager has no analogue for: the
  recall-hit trail. What does not transfer: the Tauri + React + SQLite +
  Node dev toolchain against "one static Rust binary, no Node runtime, no
  checkout dependency"; SQLite as a second store when the substrate is
  the filesystem (one fact per file, one directory per skill) and a
  second store would be a second truth; and marketplace/git install plus
  deploying to 50 harnesses, which is the opposite of the own-home
  ruling — fleet is a consumer of `~/.agents/skills` and never writes
  another harness's config. A second desktop window would also fragment
  the one console surface the owner asked for.
- **The memory edit rides the substrate's public modify path, and the
  plan's "in place" wording is narrower than the code allows.** The
  panel spec says a fact "edits in place through the substrate's own
  rewriter". `memory::rewrite_field` — the one in-place rewriter, which
  re-renders a named frontmatter line and preserves every other byte
  through a temp file and a rename — was private. G5 widens it into a
  public `memory::edit(root, &FactEdit)` that rides it for every field
  and adds `rewrite_body` for the body, so the browser's edit is the
  corpus's own write path rather than a file write of the GUI's. What
  the read cannot do is rename a fact: the name is the fact's identity
  and its file's stem, so a rename is a supersession
  (`Memory::supersede`), which records why. The name, the tier and the
  basis are therefore not editable from the browser, and the form says
  so.
- **The skills approve rides improve.rs, because skills.rs has no
  approve.** `skills.rs` loads the library, the skips and the proposals;
  it applies nothing. The apply path is `improve::apply(name, &Apply)`,
  which stages the change, runs the four cargo steps in the repo, and
  keeps the change only when every step is green. `POST /api/skill`
  rides exactly that function with `approved: true` — the click is the
  approval — so an approve takes as long as the gate does, and the
  button says so while it runs. A skills write is home-side, so it
  creates no commit; the pre-image stash beside it is what
  `fleet improve undo` restores. Two consequences are named rather than
  papered over: the eval-corpus step is unwired because a source target
  is refused by name before the gate (this is the skills panel's
  approve, and a source edit's corpus step is a CLI step), and the gate
  runs on the process's own PATH, so a `fleet gui` launched from an
  environment without cargo refuses the approve and names why — the same
  exposure the CLI has.
- **Retirement curation is a standing no-op, and the card says so.**
  `skills::curate` files merge-only proposals; a retirement draft would
  measure use across fleet's sessions while the canonical library is
  cross-harness, so a fleet-side count would retire another harness's
  skill (KI-215). The read therefore carries the KI-215 refusal in
  `curation_note` rather than offering a retire button that could not
  work. The merge candidates ARE offered, computed read-only with the
  substrate's own tokenizer at its own threshold — the same pairs
  `curate` would file, without the write.
- **A read that did not land, found by building it.** The first skills
  read scored 97,020 description pairs by re-tokenizing both sides per
  pair and took 8.4 seconds — far past a 5-second refresh interval. G5
  adds `skills::TokenSet`, which tokenizes each description once and
  scores a pair by a merge over two sorted sets, and `description_similarity`
  now delegates to it so there is one rule and not two. The read lands
  in 275 ms against the real 441-skill library.
- **The corpus root is resolved once and shared.** The browser's memory
  panel and the agent thread's consolidator both walked for a corpus
  root. `Bridge` now holds the root resolved by the agent's own rule
  (`memory::resolve_root` off the process's working directory) and hands
  the same root to the consolidator, so the two cannot disagree about
  which corpus this face is looking at. The server tests point the panel
  at a corpus they built through a `#[cfg(test)]` seam, which keeps the
  production rule out of the test's way.
- Increments G1+ follow this document; each lands gated and committed
  with explicit paths, and this file's `updated` field moves with the
  work.
