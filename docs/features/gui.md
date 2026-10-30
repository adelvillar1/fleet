# The browser face — `fleet gui`

Fleet's interface is a local web application served by the fleet binary
itself. `fleet gui` starts three things in one process — the HTTP
server, the agent bridge, and the browser — and the conversation is the
app: the router's decisions, the memory recalls, the skill loads, and
the delegation spawns render as compact cards inside the transcript as
they happen, so nothing that makes this harness valuable is a pane you
must remember to open.

Plan: [`../plans/2026-10-16-gui-revamp.md`](../plans/2026-10-16-gui-revamp.md)
(the GUI revamp, G1–G10, complete).

## The seam

One binary, zero new crates, loopback-only. The server is hand-rolled
HTTP/1.1 on the tokio this crate already carries; the assets are
hand-written HTML/CSS/JS embedded with `include_str!`; every write rides
the one writer the CLI shares. The boundary is the machine — loopback
bind, Host and Origin checks — and the address alone opens the face. A
per-launch token used to gate every route; it is gone because it made
the face's own reload a refusal, which is not what an owner's browser is
for. Authentication returns when fleet becomes a multi-user app. A
non-loopback `--host` is refused by name.

The face is light-first. `paper` is the default ground — a warm, slightly
green-grey paper with near-black ink and one signal blue — because a
console is read in daylight, beside a document, for hours. `console`,
`graphite`, and `contrast` are remembered options, not the house style.
The type has three voices: New York/`ui-serif` for display only, the
system sans for every readable label, and SF Mono for every measurement,
ID, and fact name.

One event log drives the whole face. `/api/state` is the replay and
`/events` is the live half of the same log, so a refresh and a stream
are the same derivation run twice. Every number in the header — spend,
context, route, exchanges, stop reason — is reduced out of the log rather
than reported, and the inspector shows the reduction beside the event it
came from. An **exchange** is one owner message and the run that
answered it; steer frames and scheduled prompts ride the log as user
records but are not counted, so the total reads the same live and after
a reload.

## The home briefing

The conversation's empty state is the harness's home, not a blank page.
It answers four questions from live reads:

- **What does fleet know about me?** Live and protected fact counts, and
  the names of the two most-recalled facts, from the memory corpus.
- **What work and projects do I do?** The workspace, the run archive's
  size, the last three runs, and the kind of work each was.
- **What is my workflow?** Plan → Delegate → Verify, with the
  dependency graph's next ready node offered as the first step.
- **How does this make me efficient?** One-click review of today's
  changes, a resume of the last run, and the lanes (sys1 judge, memory,
  skills, web, deck) within reach.

`GET /api/home` is one bounded read assembled from the same panel reads
and cached for thirty seconds, so the home and a panel cannot disagree
and a reload does not walk the corpus, the library, and every session
log again. Each card fills progressively; a refused read keeps the last
good value and names its reason. A `since your last visit` line counts
the runs and facts that landed while the owner was away.

## The process card

A run is a loop, and the face shows where it is. The dock opens with
**the run**: route, think, act, verify, with the current step, the
exchange, the tool, the facts, the delegations, the checkpoints, and the
elapsed time. It is reduced from the same event log the transcript
reads, so it cannot claim a step the run did not take. A plan-mode run
uses its own three-step loop (plan, review, build).

- **Live branches.** Each `delegation` event updates a branch keyed by
  the child session: its goal, stage, call count, last tool, outcome,
  and turns. The three newest render under the stepper; clicking one
  opens that child's replay in the dock over the live conversation.
- **The gate.** The verify step names the failover gate from
  `/api/state` (`gate on · sys1` or `gate shut`). A finished run offers
  **ask sys1 for a review**, which fills the composer with the review
  task and leaves the send to the owner.
- **Stop and steer.** While a run is in flight the card carries a stop
  button and a one-line steer field. Both write the control file the
  agent's turn loop reads before its next model call
  (`<sessions>/<session>.control`); a stop shows `stopping…` until the
  run's own end record lands, and a run the owner stopped renders as
  **stopped by you**, not as a failure. The control file is cleared
  when the run returns, so a stop that lands on the last step cannot
  end the next run in the same session.

The topbar carries a compact process chip on every panel, so a run is
visible while the owner is reading the router or a board; the strip
becomes one line above the composer when the pane is closed.

**The pane is the one right surface.** One icon opens it; there is no
second version. Four tabs: **run** (the live loop — route, think, act,
verify — with the stop and steer controls, and the session and trust
cards beneath it when no object is selected), **files** (the working
directory's tree beside one file's code), **preview** (the deliverable
and the documents, see below), and **web** (an address bar that
renders the site live when the site allows framing — the pane checks
the site's own `x-frame-options` and CSP before rendering, names the
refusal in the site's words when it does not, and stands the prose
reader and the owner's browser beside it as the two doors; a framed
page cannot navigate the pane away). Selecting an object opens the
pane with the object's card above the tab body, the run's cards
standing down while it speaks; at and below 1180px the fixed drawer
keeps that job.

**The preview renders the common documents.** Markdown (the house's
own renderer, table support, the output through DOMPurify before it
touches the DOM), CSV/TSV as real tables (quote-aware), JSON and NDJSON
as collapsible trees, images and SVG as `<img>` — an SVG's scripts are
inert there and nowhere else — PDF pages through vendored PDF.js with
scripting disabled (the document is data), XLSX as real tables with
sheet tabs from a hand-rolled zip reader (values only, named), PPTX as
a per-slide outline (the honest partial: nothing renders a deck
client-side, and the pane says so), DOCX through vendored docx-preview
inside the sandboxed frame, and `.drawio` as its XML with the `.svg`
companion rendered when one sits beside it. The deck a run builds
renders inside the sandboxed frame too — it is the model's HTML. The
fences: a contained bytes route (`/api/preview`) with the same
credential refusal and working-directory bounds the text read has, the
content type derived from an extension allowlist and nothing else,
`nosniff`, and `Content-Disposition: attachment` so a navigation to the
route can never execute a document on the face's origin. The vendored
libraries (DOMPurify, PDF.js, docx-preview, JSZip) carry their
provenance in `assets/vendor/VENDOR.json` — versions, upstreams,
SHA-256s, licenses — and lazy-load per format from the face's own
origin.

**The owner's width.** The pane's left edge is a grip: drag it and the
width follows between a 300px floor and the stage's own, the choice
persists across reloads, a double-click resets, and the grip is a
separator the keyboard steps with the arrow keys. While a drag is in
flight a live page's pointer events stand down, so the frame cannot
swallow the move.

## Selected objects and related suggestions

Selecting an object opens the same right-hand inspector, non-modal, so
the owner can keep working while it stays open. Above 1180px it is the
dock's object card, above the session and trust cards; at and below
1180px it is the fixed drawer. It carries the object's facts, its
actions, and its related objects.

- A **fact** opens its type, tier, basis, recall count, path, its edit
  form, and actions to ask the agent, copy the name, or filter by shape.
- A **skill** opens its uses, path, body, and actions to copy the name
  or ask the curator.
- A **route row** opens its walk, outcome, latency, and a sys1 second
  opinion.
- A **project** opens its git facts, plans, path, and the ask action.
- A **graph node** opens its state, score, and three unlock conditions,
  with the step handed to the composer.
- A **calibration cell** opens its fit, counts, confidence span, and the
  gate-review action.
- A **KPI tile** opens its derivation and the list below it.

**Related suggestions are one server-side index.** `GET
/api/related?kind&id&limit` compares the selected object against every
other kind — facts, skills, sessions, projects, route rows, and
calibration cells — by the same embedder the recall uses. Facts reuse
`memories/.recall-index.json` and sessions reuse
`<sessions>/.recall-index.json`; the sidecar `<FLEET_HOME>/.related-index.json`
owns the kinds that had no index (skills, projects, routes, cells).

- The percentage is the raw cosine, not a score this surface invented.
  A score under 0.72 is marked **weak** rather than filtered: KI-058
  measured unrelated pairs at 0.58–0.66 and related ones at 0.77–0.93.
- The cold half — every skill description and every project's identity
  — warms once at launch, off the request path. A click that arrives
  first answers from what is ready and says `warming`; the card retries
  once after two seconds. A request never pays the archive's cold embed.
- With no embedding backend the card answers lexically and says
  `keyword match · no embedder`. Facts and sessions are cache-only on
  this path, exactly so a model swap cannot turn a click into a
  minute-long wait.
- Related rows exist on sessions, routes, and calibration cells as well
  as facts, skills, and projects.

**The counters adapt the order.** The browser stores counters — never
text — in `<FLEET_HOME>/ui.json`: offers and related pairs shown,
accepted, and dismissed; panels opened; actions used. An accepted-over-
shown ratio raises an offer or a related row by at most a third; nothing
is hidden and the primary lanes never move. The knob `ui.adapt` (default
on) switches the re-ranking off. The store is atomic, capped, and a
malformed or foreign-version file reads as empty.

## The interaction layer

No object is decorative. Every row, tile, badge, chart segment, pill,
and node has a purpose and an action, and the same inspector serves
them all. The rule is audited, not assumed: a Playwright pass asserts
every visible `.tile`, `.badge`, `.rrow`, `.rank-row`, `.cal-cell`,
`.stream-line`, `.cand`, `.chart-col`, and `.related-row` carries a
`data-act`, a button role, or a title.

- **The work list** opens in place. Every row — whoever its state — runs
  one verb: it resumes that log into the conversation beside the list,
  with its composer, so reading and continuing are the same act and
  there is no separate read-only replay. `#/sessions/<id>` is the
  address of the open row and survives a reload; `#/chat` is the home —
  the welcome with the counts and "pick up where you left off", and the
  one place every surface's Escape returns to; arriving there detaches
  from the conversation being read (nothing server-side changes, and
  the recent list's live row is the way back). The rail renders the
  owner's work only: system-initiated sessions — scheduled jobs,
  delegated children, peer channels — live in the log (`fleet cron`,
  `fleet sessions`), never on the user screen, and a log with no words
  and no kind is not a conversation at all. A **probe** — verification
  traffic, birthed by `fleet run --probe` or marked by `fleet sessions
  --probe <id>` in the archive's sidecar — is the same: the log keeps
  everything, the rail renders none of it. Rows are grouped by what
  needs the owner (*needs you*, *working*, *earlier* per project, and
  **archived** closed behind one line at the bottom), and the state
  words come from the bridge's derivation:
  `needs-approval`, `paused`, `working`, `active`, `handed back`,
  `completed`, `capped`, `stopped`, `archived`, `interrupted`, `new`,
  `resumed`. The rail's sessions badge counts the owner's waiting work
  — needs plus working — from every surface, and hides when the archive
  is quiet. **New** is an intent, not a row: it walks home with a fresh
  compose, and the session is created when words are sent, so an
  abandoned compose leaves no shell in the archive.
  Sessions end only when the owner archives them: a run that stopped —
  the model handed the conversation back, or the turn cap hit — reads
  as open (`handed back`, `capped`), never as finished, and every row
  carries an **archive** chip (an archived row's chip reads
  **unarchive**). The conversation head carries the same verb for the
  session it anchors. Archiving a run in flight is refused by name —
  stop it first. Where there is no room for
  the column it overlays as a sheet, opened from the conversation's
  title or back control and closed by picking a row. The column folds
  too: one control in its head closes it to a 48px strip — the control
  that brings it back, and the archive's count of what needs you — and
  the conversation takes the rest of the width. The choice survives a
  reload (`localStorage["fleet-work"]`), and the column's left edge
  follows the navigation, so folding the rail onto its icons leaves no
  gutter between the two.
- **The conversation's head** carries the way back to the work list, the
  title (press for the switcher, the pencil to rename), the state in
  words, and **New** — which writes a real session (`POST
  /api/session/new`), so the next run lands in it. Replies render as
  markdown when their block closes: streamed text while it arrives, the
  rendered block at the boundary, and the receipt's closing report the
  same way. Every message then carries its own controls under its words
  as icons — **copy**, **like**, **dislike**, **fork**, and **hook** on
  a reply, **edit** and **copy** on the owner's own — the words in the
  tooltip and the accessible name, the glyph in the row, visible
  without a hover, because a hover-only control does not exist on a
  touch face. A verdict is keyed by the exchange the reply answered and
  lands in the feedback sidecar beside the log, never in it; the
  pressed verdict is the accent plus `aria-pressed` and the tooltip's
  past tense; a fork opens a new
  conversation carrying this point's history with the source left
  whole; a hook seeds a scheduled job from the ask the turn answered,
  and the owner arms it in the drawer before anything is written. The
  composer's foot carries the run's two beside-verbs as icons beside
  attach and plan: the **steering wheel** fills the composer with
  `/steer` and the **bifurcation** with `/btw` — the owner types the
  instruction, and the slash parser, the only parser, takes it from
  there. The icon buttons carry the face's own hover tooltip — instant
  and themed, the words in `data-tip`, the accessible name in
  `aria-label` — where the native title needed a long hover and
  rendered browser-default. A send beside driving work queues above the
  composer, and each queued row carries its three verbs as icons the
  reply footer's way: **send** moves the message to the head (the head
  is what the next run sends), **edit** returns its words to the
  composer and takes the row out, **drop** removes it; rows still drag
  to reorder, and an emptied queue leaves no rows behind.
- **The activity rows.** One line per thing the run did, each opening on
  a press: `Thought · Ns` (the provider's `reasoning_content`, kept in a
  `reasoning` log record and never sent back), and a tool call, whose
  expanded detail carries the full command or path from the call's own
  arguments, the elapsed time, the output preview, and a `raw record`
  button. The tool card's press opens that detail rather than the raw
  event. The run strip's folded lines open their records the same way.
- **The conversation's text size is the owner's.** `A−`/`A+` in the
  conversation's head — and `Alt+↓`/`Alt+↑` from the composer — step a
  multiplier (85% to 170%, 100% by default) on the transcript. The
  multiplier keeps the hierarchy: a reply, its headings and its code grow
  together, and the transcript's column widens with it so the measure
  stays roughly the same characters per line. Everything inside the
  transcript scales — your messages, the reply, the thought rows, the
  command cards with their expanded detail, the receipt and the run line
  — because all of it is text the owner reads. Outside it nothing moves:
  the composer, the run strip, the conversation head, the rail and the
  panels keep their measure, and a card's own furniture (its metric and
  caption) stays put. Each end dims its own press rather than pressing
  into nothing, and the choice survives a reload
  (`localStorage["fleet-chat-size"]`, a step index).
- **Facts** open their detail and edit; their type and tier badges
  filter the corpus.
- **Every panel has visible filters with counts.** Memory: all, live,
  protected, retired. Skills: all, used, unused, proposals, skipped.
  Calibration: all, fit, informing. Tower: all, dirty. Sessions: all,
  open, ended, delegated, compacted. The matching KPI tile applies its
  filter, and the chosen chip is the only one pressed.
- **Chart segments filter.** A router hour column narrows the route
  rows to that hour; a donut segment (or its legend row) narrows them to
  that model.
  Each leaves a removable chip above the narrowed list.
- **Route rows** open their walk and offer a sys1 second opinion;
  chain candidates copy the model id.
- **Calibration cells** open the fit and the rule, with a gate-review
  action and related objects.
- **Graph nodes** open their unlock conditions and hand the node to the
  composer.
- **Project rows** open their git facts; a report-feed row opens its
  project's card, and claims, gate verdicts, and scan skips open their
  record.
- **Tiles** open the metric's derivation and the list below it; the
  tiles that count a filterable set apply the matching filter.
- **Topbar chips** open the surface that explains them; the route opens
  Router, the state opens the archive at this run, and spend/context
  open the derivation.
- **Method and lanes** are actions: Plan toggles plan mode; Delegate
  opens the graph; Verify asks sys1 for a gate review; memory, skills,
  web, and deck open or dispatch their lane.

The inspector is one detail surface with two homes: the dock's object
card above 1180px, the fixed drawer at and below it. Neither navigates
away from the panel behind it, and both carry the object's actions.
Escape clears the object, never the dock.

## The endpoints

| Route | What it does |
|---|---|
| `GET /api/state` | the session, model, gate, budget, ordered log, seat, the session this face is *driving*, and remembered rules |
| `GET /api/home` | the welcome's bounded read, cached 30s |
| `GET /events` | the live half of the same ordered log (SSE) |
| `POST /api/run`, `/api/plan` | hand a task to the agent thread; the body may carry `folder`, which is validated and stored for the chat before dispatch |
| `POST /api/folder` | set or clear a chat's working folder without a run; the reply is the transcript line it recorded |
| `POST /api/pair/window` | mint a single-use device pairing window and the QR card document (loopback only) |
| `POST /api/stop`, `/api/steer` | write the run's control file; 409 when no run or no log |
| `POST /api/approve` | answer the standing seat |
| `GET /api/related` | cross-kind related objects for one selected id |
| `GET/POST /api/usage` | the browser's counters and the adaptation knob |
| `GET /api/router`, `/api/delegation`, `/api/memory`, `/api/skills`, `/api/sessions`, `/api/session`, `/api/calibration`, `/api/tower`, `/api/second-brain`, `/api/settings` | the panels' own reads |
| `GET /api/tree`, `/api/file`, `/api/web`, `/api/web/frame`, `/api/preview` | the pane's contained reads: the working directory, a credential-refusing file read, a guarded fetch, the framing verdict the browser tab renders by, and the document bytes the preview renders from |
| `GET /api/channels` | the channel control plane: the service's status (and whether its heartbeat still beats), every channel's paired and pending peers, and the newest fifty activity lines with the bound named |
| `POST /api/channels/{start,stop,pair,capability,revoke,forget}` | the control plane's writes. `pair` answers with the token exactly once; there is no read that returns it again |
| `GET /api/access`, `POST /api/access/revoke` | the paired devices and channel peers, by name and digest only, and the revocation that forgets one |
| `POST /api/session/new` | create a conversation: the log exists before its first message, and the next run posted with its id lands in it. The body may carry `{persona, goal}` — the bot's creation (see [`bot-app.md`](bot-app.md)); a persona that no role file carries is refused by name |
| `POST /api/session/join`, `/api/session/leave` | one persona joins or leaves a conversation: a row in the log, refused by name when the persona does not resolve, is already present, or is the lead others still follow |
| `POST /api/fact`, `/api/skill`, `/api/resume`, `/api/rewind`, `/api/feedback`, `/api/session/fork`, `/api/diff/reject`, `/api/settings`, `/api/deck/render` | the writes, each through the writer the substrate already owns |

The channel control plane's routes are **not** on the remote face's
allowlist: a paired phone answers 403 by name, the same rule settings
has. A phone that could pair a peer would be a phone that can widen the
door it came through.

**Two shells, two ports, one bridge.** Every face serves this console:
the loopback root and the paired-device face on the machine's address.
The bot face — `fleet bot`, the assistant-first PWA whose personas, goal
and destructive seat are specified in [`bot-app.md`](bot-app.md) — binds
its own port (`--bot-port`, default 3081) as a second window. Both faces
read the same logs and answer the same seat: a phone and the desktop are
two clients of one conversation.

## The surfaces

The keyed panels under hash addresses, four destinations in the rail
(work, intelligence, knowledge, system), one flat level beneath each,
and ⌘K reaching all of them so the grouping is a convenience rather than
a maze. The rail's collapsed state is remembered across launches.

| Panel | What it reads |
|---|---|
| `#/chat` | the conversation: the transcript, the inline intelligence cards, the approval seat |
| `#/router` | the router board's own reads plus the live decision, which rides the log |
| `#/delegation` | the live run's tree and the dependency graph with the unlock conditions the code requires |
| `#/memory` | the fact corpus, the day's recall hits, search; an edit rides the substrate's own rewriter |
| `#/skills` | the library with its loaded/skipped split, the proposals, the curation candidates |
| `#/sessions` | the archive newest-first; a row opens its replay, and the replay's resume re-anchors the face to that log. A row also names the project it was born in and, when a channel started it, `via <channel> · <peer>`; `via a channel` is a filter chip |
| `#/calibration` | the day's stream rows by op and the judged quality's fit cells, the same computation `fleet calib --calibration` rides |
| `#/second-brain` | the project atlas: the attention queue, the six-week pulse, the commit ribbon, the report feed with the harness trust lines, and the method band (gates, loops, harness, maturity) |
| `#/bots` | the channel control plane: the service and its heartbeat, the write-only Telegram token, the paired devices with revoke, who can reach the fleet with an editable capability, the peers waiting at the door, the pair form with its callback and one-time token reveal, and what they have done. The rail entry carries a count only when something is waiting |
| `#/vault` | the vault as its own surface: first-run onboarding (choose the passphrase, create, migrate the plaintext `.env` entries), the sealed names, the plaintext ledger with seal/drop, lock, and start-over for a forgotten passphrase |
| `#/settings` | every configurable surface: the router (spend guard, tiers with resolved-now, thresholds, pin), the providers as field editors, the lane cards, the MCP servers card (`docs/features/mcp.md`): the registry's rows with their probe dots and enable/probe/remove, the scan's discovery rows with their badges and import, the roles editor, the vault, the spend caps, the remaining knobs, the rules, the cron jobs, the identity slot, and the env names |

## The configuration surface

The Settings page is one page with section chips: **router · providers ·
lanes · roles · vault · spend · knobs · rules · cron · identity ·
environment**. Every write goes through the one writer the CLI uses
(`settings.rs` for the router, providers, lanes, secrets, and spend; the
roles writer for the roles), and every value field is a pick from a
catalog first.

**The catalog.** `settings::catalog` is returned in `/api/settings` and
is the one source every picker reads: every `provider/model` from the
resolved roster plus every value the current `routing.json` names (a
value the roster no longer carries stays as a `custom` entry), the
provider states, the workload names, the env names with presence and
source but never a value, the resolved paths, the admissible tools, the
skills library, and the memory shapes. The lane cards add the declared
defaults each lane publishes.

**The input rule.** A closed set is a `<select>`; an open set is a
combobox whose datalist carries the catalog and whose text stays free for
a new value; an ordered list (a workload's fallbacks, sys1's chain) is a
chip editor with move-up/move-down; an unordered set (a role's tools,
skills, shapes) is a multi-chip editor. A value already present is
preselected or a chip, and an unknown current value is shown rather than
dropped. Free text is reserved for a genuinely new value: a description,
an overlay, a cron expression, a provider id, or a secret.

**The router.** The spend guard writes `gates.allow_api_models`; each
tier's target is a model select and its fallbacks are ordered chips; the
resolved column is the first candidate admissible under the gate and
enabled with its key present (or not required), or the first blocking
reason. The orchestrator's `hard` tier carries a `walker` badge. The
thresholds write `gates.min_confidence`, `gates.default_workload`, and
`pin`; the pin is a select over provider ids and pairs.

**The providers.** Each roster entry is a field editor — base URL, key
env (a name), models as chips, billing, kind, enabled, key required,
disabled reason — and the save builds the entry the loader validates.
The raw JSON textarea is gone. An override of a compiled-in provider
reverts; an entry the file added needs the destructive confirmation.

**The lanes.** sys1, sdm1, sem1, gen1 (and the browser and composer)
each have a card: the home path, the env names the lane declares, and
the per-call defaults for the tools whose schemas accept them. The write
is a fleet-owned `<FLEET_HOME>/lanes.json`; the lane's own repository and
sys1's TOML are never rewritten. `lanes::configured_spec` injects the
values at launch with the precedence process env > `.env`/vault >
`lanes.json`; an undeclared name never crosses. The agent merges a
lane's per-tool defaults under the model's arguments, so an explicit
value always wins.

**The roles.** A role is `<home>/roles/<name>.md`; the editor writes the
loader's own frontmatter, preserves unknown keys, and picks tools,
skills, and shapes from the catalog.

**Secrets and the vault.** A secret is write-only: the value is sent
once, the input is cleared, and no read carries it back. The vault card
is a working surface, not a status line. With no vault yet, the card is
the first-run setup: it names the credential-shaped `.env` entries that
will be sealed, asks for a passphrase and a confirmation, and says
plainly that there is no recovery — the passphrase keys the vault and is
never stored, so it belongs in a password manager before anything is
sealed. One action then creates the vault and migrates those entries.

For an existing vault, a locked process can unlock it in place: the
passphrase is sent once, held in memory by the process (never written),
and a wrong passphrase is refused by name. Once unlocked, the card can
seal one plaintext entry at a time (server-side, so the value never
reaches the page), drop a plaintext line after its value is sealed, seal
a new entry, remove a sealed one, or lock the process again. The
`in .env` section marks each credential `plaintext` or `sealed`, so the
migration has a visible finish. A forgotten passphrase has its own
`I do not know the passphrase` path: starting a new vault archives the
old one as `vault.json.bak-<timestamp>` — never deletes it — and seals
the plaintext `.env` credentials under the new passphrase. Launching
fleet with `FLEET_VAULT_PASSPHRASE` remains the way to unlock from the
start; the in-place unlock is the same passphrase, held the same way,
without a restart.

**Spend and runtime.** The daily caps (judge, delegate, explain) are
`Usd` settings that can only tighten the compiled ceiling; the empty box
is seeded with the cap in force. `FLEET_CONCURRENCY`, the web cache TTL,
the skills directory, and the services root are env knobs. The binary
loads `<FLEET_HOME>/fleet.env` at startup, skipping credential-shaped
names and never overriding a set variable, so a write there applies at
the next launch.

## The trust trio

Three capabilities that ride the conversation rather than adding a
panel:

- **Checkpoints.** Each turn's file state is captured under the home's
  own `checkpoints/<session>/<turn>/`, bounded by file count and bytes,
  with the version-control and build directories skipped and the cap's
  cut named. A capture that fails names itself and the run continues —
  the trio is a safety net, not a gate on the work.
- **Rewind.** A checkpoint card's rewind restores the tree to that
  turn's state, appends the `rewind` record the next reseed cuts at, and
  reseeds the face. The conversation continues from the turn the owner
  rewound to; the abandoned records stay in the log, because a log is
  append-only.
- **Plan mode.** The composer's second door. A plan-mode run refuses the
  write-class tools by name at the gate — its child delegations inherit
  the floor — and its final message records as a `plan` the browser
  renders editable, with Build posting the edited text as an ordinary
  run.
- **Diff review.** `/review` measures the tree against a checkpoint and
  renders each file's changes as hunks; a reject puts that hunk's
  checkpoint lines back and re-reads the diff rather than pretending the
  file is now clean.

## The working folder

The composer's run bar carries three standing choices for the chat: how
its tool calls are approved, the provider its route is pinned to, and
where it works. The first two ride the next run; the third is a grant
the agent's own tools enforce. **No folder means no filesystem access at
all** — access is granted by choosing a folder and by nothing else.

- **The choice is per chat.** A folder is stored against the session in
  `<FLEET_HOME>/sessions-workdirs.json` — a surface sidecar, not a new
  session-log record — so a reload restores it and a second chat starts
  with none. A pending launch session's choice moves to the id when the
  log is created.
- **The tools, not the prompt, enforce it.** With a folder set, `bash`
  spawns with it as its working directory, and a relative `read_file`,
  `write_file`, or `list_dir` resolves under it. A `..` walk, an
  absolute path outside, and a symlink that resolves outside are each
  refused with a reason naming its class. With no folder, the four are
  refused by name before the approval gate — nothing is read, written,
  or executed — and the refusal tells the model to ask the owner for a
  folder. The prompt says the same thing before a call is made.
- **Full access needs a folder.** The mode is "nothing asks", and only a
  working folder can bound it, so the control is disabled on a no-folder
  chat, a mode left on full falls back to ask, and the run route refuses
  `mode: full` with no folder as a 400 naming the rule.
- **The boundary is the launch directory.** The picker browses it one
  level at a time through `/api/tree`; a folder outside it needs a
  settings allowlist and is out of scope. Choosing the launch directory
  itself is a real grant — the bar shows its name like any other — and
  "remove the folder" is the only thing that clears access.
- **A run captures its own root.** The folder is resolved when the run
  is accepted and rides the command, so a change made during a run
  applies to the next one. The turn checkpoint captures the chat's
  folder; a no-folder chat owns no tree and captures nothing, and a
  subagent inherits the parent's access the way it inherits the model.
- **A stored folder that has gone missing** falls back to *no folder*
  at run time — never to the launch directory — and the run's first
  event is a transcript line naming the missing folder and saying the
  filesystem tools are unavailable until one is set. The stale entry is
  cleared. A folder named in a request is validated strictly instead:
  an escape or a missing path is a 400 naming the reason, never a
  silent fallback.
- **The other surfaces are unchanged.** The CLI, the TUI, and headless
  runs are launched from a directory; that launch is the owner's
  explicit choice, so they keep the unrestricted root they always had.

## Documents

The composer's Document control attaches a document from the workspace,
not from the desktop: the picker walks `/api/tree` one level at a time,
the same containment the working folder uses.

**Local first.** A clean `.docx`, `.xlsx`, `.pptx`, `.csv`, `.html` or
text file is read on this machine by the documents lane
(`services/documents/extract.py`): Office formats as their OOXML parts,
legacy binaries through LibreOffice `--safe-mode`, PDFs through
`pdftotext`, with MarkItDown used when its package is installed. No key,
no upload, no bill. A PDF whose text layer is empty is the escalation
case, not an error.

**Perception second.** A scan, an image, an email, audio/video, or a local
failure is read by Nace Perception's Parse (`POST /v1/documents/parse`).
The first request answers with what would be sent and why; the document
leaves the machine only when the owner presses "send to Nace". The key is
`NACE_API_KEY`, falling back to `DREX_API_KEY`, read from the vault or the
home's env files and never sent to the browser. The parse is cached by
content hash under `<FLEET_HOME>/perception-cache`, so attaching the same
file twice makes one billed call.

**The record.** The chip and the framed turn name the parser, the pages,
the units where Perception billed, the job id, and a truncated preview's
`retained X of Y`. The turn, and the parsed text, go to the model
provider that serves it — the same egress every turn has; a document that
went through Perception names that second recipient too.

**Bounds.** Five attachments a turn, 64 MB a document, a 120-second local
extraction and a 300-second job budget. A missing lane or tool is a named
refusal; macros are never executed and embedded objects are never opened.
The lane's tools and licences are listed in `services/documents/README.md`.

## The bots surface

The bots page is assistant-centric (the 2026-10-28 ruling): personas are
the axis, and the plumbing is one section rather than the page.

- **The rail** carries one row per persona — derived from the roles
  registry, with the count of conversations that need the owner — then
  `unassigned`, then `channels`. It is the settings rail's own
  implementation, collapsed and remembered the same way.
- **A persona's section** orders what to do: *needs you* (a seat waiting
  on an approval, with the state word on the row), *turned back* (the
  run ended its turn — the row carries the message it ended on), then
  *recent*, budgeted when long.
- **A conversation opens in the pane** as a card with three parts, all
  from the replay and the state the face already reads: the **details**
  (persona, model, turns, created, ended, workspace), the **decisions**
  (the goal it served, the facts it chose to remember, the ask it ended
  on), and the **history** (the whole transcript, newest first, tool
  rows one line, the replay's own omitted count named).
- **The channels section** keeps the transport: the service, the pairing
  card, the Telegram token, the peers and the activity line.
- **The addresses**: `#/bots/<persona>` and
  `#/bots/channels`; a conversation has its own id in the row.

Nothing on the page invents a state: `needs-approval` and `handed back`
are the archive's own words, and no unread notion exists because the log
cannot support one.

## The settings surface

Settings is a rail of eight sections and one section's items — the work
rail's own idiom, one fold in the product rather than two:

- **The sections**: models (the tiers, the gates, the provider accounts),
  specialists (the lanes, the tools, the MCP servers), roles, trust (the
  approvals mode, the remembered rules, the vault), money (the caps),
  schedule (the cron jobs), interface (the UI knobs and the identity
  slot), environment (the env names and the remaining scalars).
- **The row is one line**: the item's name, its current value, and the
  marks worth a glance — `set` when the value comes from this home's own
  config, `env` when the environment supplies it, `default` for the
  compiled value, a live mark on a rule or a job. A section reads as a
  table; its count sits on the rail's row, and a section holding an
  override carries a dot.
- **The item opens in the pane**: the same object card the facts and
  sessions use, the pane's tabs standing down while it is open. It
  carries the fields, the provenance in plain words, the compiled
  ceiling, the revert where the document offers one, and the save
  through the writer the CLI uses. Closing returns the tabs; an edited
  card that is closed without saving asks first.
- **Long lists take a budget**: the first ten or twelve rows and one
  press for the rest, the count named ("24 rules · 14 rules"). No stored
  state, and the number is honest about what is behind the press.
- **The filter at the head of the rail** searches every section's items
  at once and lists the matches with the section that owns each.
- **The addresses**: `#/settings/<section>` opens a section (bare
  `#/settings` resumes the remembered one), and `#/settings/<section>/<item>`
  is the item's own address — which is what lets the onboarding's cards
  land on the setting they are about.

Everything writes through `writeSettings` and the same ops as the CLI;
the surface is the presentation, not the contract.

## The face's own tests

`tests/face/harness.mjs` boots the real `src/gui/assets/app.js` against
a stubbed DOM and the bridge's document shapes, then drives it the way a
pointer or a keystroke would and asserts on what it built: the work
list's group order and the server's counts, one open verb across
completed/working/interrupted rows, the conversation's head with its
switcher and New, a send beside driving work (queued, named), a run's
end clearing the way for the next send, a reply that is text until
its block boundary and rendered once after, the work column's fold and
its following the rail, and the conversation's text size stepping,
dimming at its bounds and surviving a reload. `tests/face.rs` runs it from
`cargo test` when `node` is on PATH and skips by name where it is not;
the live passes in the recaps cover what a stub cannot — layout, the
event stream over the wire, and a real run's records.

## The reading rules

- Below 900px the shell is one column whatever the rail, dock, or pane
  state says: the rail becomes a drawer, every board folds to one column,
  and the stage keeps the full width instead of landing in the rail's
  track. A media query adds no specificity, so the narrow rules answer
  the desktop state rules at their own specificity — the failure mode
  there was a 236px stage beside 664px of empty canvas.
- A refused read keeps the last good numbers with its reason beside
  them — the console is never a blank screen because a read did not
  land.
- A bounded read names its bound: a replay previews the newest 400 cards
  and says what it left out; the calibration read names its 512-row cap;
  the tower scan names what it left unread.
- No credential value ever reaches a payload. Names and presence only,
  the doctor's masking rule.
- The client is verified two ways, both scratch and uncommitted because
  the repo carries no JS runner: the real `app.js` runs in a Node `vm`
  against a stubbed DOM fed the bridge's own documents, and since
  2026-10-19 a Playwright pass drives a real Chromium at the shipped
  widths for rendering, interaction, overflow, console errors, and the
  `data-act` audit. The stub is not the truth about what renders; the
  browser is.

## The install shell

The face is installable. A manifest and two PNG icons (the serif **f** on the graphite console
ground, with one signal-blue rule) let a browser add it to the home
screen, and a small service worker keeps the two script assets and gives
an unreachable face a readable retry page instead of a blank tab.

- **The assets are content-addressed.** `build.rs` hashes the served
  assets and exposes the stamp; the page asks for `app.css?v=<stamp>` and
  `app.js?v=<stamp>`, and only those URLs are served `immutable`. The bare
  paths, the HTML, the manifest, and the worker itself revalidate, so an
  upgraded binary cannot be hidden behind yesterday's script.
- **The worker caches exactly two URLs** — the versioned stylesheet and
  script — and prefetches them at install. Its fetch handler returns
  early for every non-GET, every cross-origin request, `/api/*`,
  `/events`, `/pair`, and any request asking for an event stream. The
  remote face's credential is a cookie, which the browser attaches after
  the handler sees the request, so the boundary is by path, never by an
  `Authorization` check.
- **HTML is never cached.** A navigation is network-first; when the face
  cannot be reached the worker answers a 503 retry page (themed, with a
  reload action). An installed face still reads its session live — there
  is no offline conversation.
- **Installation needs a secure context.** Loopback and an HTTPS remote
  face qualify; a phone over plain-HTTP LAN does not, and the relayed
  face under `/s/<sid>/` is per-session and is not installable. Those
  paths run as ordinary pages, with no worker and no retry shell.
- **A failed boot is visible.** An ES5 guard names the browser floor
  fleet's CSS actually needs (`color-mix` and `100dvh`: Chrome/Edge 111+,
  Firefox 113+, Safari 16.2+), and a 15-second watchdog shows one themed
  card with a retry if `app.js` never marks the shell mounted. The
  browser chrome follows the chosen theme through `theme-color`.

## Pairing from the face

The topbar's QR icon — shown only on the loopback face — mints a fresh
pairing window and shows the URL, the code, and a QR a phone can scan.
The URL is chosen in one place, in this order:

1. `remote.public_url`, when the owner has named the HTTPS origin — the
   relay origin (`https://fleet.opsh.io`) is that value. When the face
   dials a relay, the card's URL also carries the relay session's
   secret as a fragment (`#k=…`), which browsers never send: the paired
   page files it away and seals its frames with what it derives from it.
2. A detected Tailscale name (`https://<tailnet name>`), **verified** by
   asking it for `/api/state`: the paired face answers 401 without a
   cookie, the loopback face answers 200, and a proxy pointed at the
   loopback face is therefore not offered as a pairing origin.
3. This machine's remote address, with a note that it is plain HTTP and
   that iOS refuses cleartext to a tailnet address.

The newest window wins: opening the card again mints a new code and the
previous one stops working, which the card says. The mint is loopback
only — the remote allowlist does not carry `/api/pair/window`, and a
paired phone does not pair phones.

## The launch

```sh
fleet gui [--port N] [--no-open] [--host H]
```

Default `127.0.0.1:3080` (the fork's port), `--port 0` for an OS pick.
The address prints; the browser opens unless `--no-open`.
Ctrl-C stops it. The prompt line the model sees names the GUI and its
URL, so the agent knows its interface.
