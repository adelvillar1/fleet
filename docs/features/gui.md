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
context, route, turns, stop reason — is reduced out of the log rather
than reported, and the inspector shows the reduction beside the event it
came from.

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
turn, the tool, the facts, the delegations, the checkpoints, and the
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
visible while the owner is reading the router or the tower. The dock is
collapsible from the topbar; when it is closed, the same state becomes
one line above the composer.

**The dock's layout.** From 1600px the nav, content, and dock stand as
three columns. From 1181–1599 opening the dock folds the nav to its
icon rail; expanding the nav while the dock is open turns the dock into
a pinned overlay, and closing it restores the nav mode the owner had.
At 1180px and below there is no dock column: the process becomes the
strip above the composer and selected objects use the fixed drawer.
The dock and the file/preview pane are mutually exclusive columns.

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

- **Sessions** select in place. The list and the selected replay stand
  side by side; arrow keys move the selection, Enter/click reads it,
  Escape returns to the list, and `#/sessions/<id>` makes the selection
  a link. Below a 1000px content track the replay becomes the dock's
  object card instead of a fourth column.
- **Facts** open their detail and edit; their type and tier badges
  filter the corpus.
- **Every panel has visible filters with counts.** Memory: all, live,
  protected, retired. Skills: all, used, unused, proposals, skipped.
  Calibration: all, fit, informing. Tower: all, dirty. Sessions: all,
  open, ended, delegated, compacted. The matching KPI tile applies its
  filter, and the chosen chip is the only one pressed.
- **Chart segments filter.** A router hour column narrows the route
  rows to that hour; a donut segment (or its legend row) narrows them to
  that model; a tower activity day narrows the report feed to that day.
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
| `GET /api/state` | the session, model, gate, budget, ordered log, seat, and remembered rules |
| `GET /api/home` | the welcome's bounded read, cached 30s |
| `GET /events` | the live half of the same ordered log (SSE) |
| `POST /api/run`, `/api/plan` | hand a task to the agent thread |
| `POST /api/pair/window` | mint a single-use device pairing window and the QR card document (loopback only) |
| `POST /api/stop`, `/api/steer` | write the run's control file; 409 when no run or no log |
| `POST /api/approve` | answer the standing seat |
| `GET /api/related` | cross-kind related objects for one selected id |
| `GET/POST /api/usage` | the browser's counters and the adaptation knob |
| `GET /api/router`, `/api/delegation`, `/api/memory`, `/api/skills`, `/api/sessions`, `/api/session`, `/api/calibration`, `/api/tower`, `/api/second-brain`, `/api/settings` | the panels' own reads |
| `GET /api/tree`, `/api/file`, `/api/web` | the pane's contained reads: the working directory, a credential-refusing file read, a guarded fetch |
| `GET /api/channels` | the channel control plane: the service's status (and whether its heartbeat still beats), every channel's paired and pending peers, and the newest fifty activity lines with the bound named |
| `POST /api/channels/{start,stop,pair,capability,revoke,forget}` | the control plane's writes. `pair` answers with the token exactly once; there is no read that returns it again |
| `GET /api/access`, `POST /api/access/revoke` | the paired devices and channel peers, by name and digest only, and the revocation that forgets one |
| `POST /api/fact`, `/api/skill`, `/api/resume`, `/api/rewind`, `/api/diff/reject`, `/api/settings`, `/api/deck/render` | the writes, each through the writer the substrate already owns |

The channel control plane's routes are **not** on the remote face's
allowlist: a paired phone answers 403 by name, the same rule settings
has. A phone that could pair a peer would be a phone that can widen the
door it came through.

## The surfaces

Eleven keyed panels under hash addresses, four destinations in the rail
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
| `#/tower` | the control tower — the one console surface: the KPI strip, the hero activity chart, the harness donut, the ranked projects, the claims against the evidence, the report feed |
| `#/second-brain` | the derived state, quieter: the gates, the inbox, the loops, the harness runs, the maturity week grid |
| `#/bots` | the channel control plane: the service and its heartbeat, the write-only Telegram token, the paired devices with revoke, who can reach the fleet with an editable capability, the peers waiting at the door, the pair form with its callback and one-time token reveal, and what they have done. The rail entry carries a count only when something is waiting |
| `#/settings` | every configurable surface: the router (spend guard, tiers with resolved-now, thresholds, pin), the providers as field editors, the four lane cards, the roles editor, the vault, the spend caps, the remaining knobs, the rules, the cron jobs, the identity slot, and the env names |

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
once, the input is cleared, and no read carries it back. It is sealed
into the vault when the process holds `FLEET_VAULT_PASSPHRASE`, else it
lands as one line in the home's `.env`. The vault card lists names and
whether a passphrase is present; sealing and removing need the
passphrase.

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

1. `remote.public_url`, when the owner has named the HTTPS origin a proxy
   serves the remote face on.
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
