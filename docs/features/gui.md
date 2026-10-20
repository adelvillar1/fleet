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

A run is a loop, and the face shows where it is. The right pane opens
with **the run**: route, think, act, verify, with the current step, the
turn, the tool, the facts, the delegations, the checkpoints, and the
elapsed time. It is reduced from the same event log the transcript
reads, so it cannot claim a step the run did not take. A plan-mode run
uses its own three-step loop (plan, review, build). When the run ends
the card keeps the final state and the last step; when no run is in
flight it says so and offers **start with Plan**.

The topbar carries a compact process chip on every panel, so a run is
visible while the owner is reading the router or the tower. The right
pane is collapsible from the topbar; when it is closed, the same state
becomes one line above the composer.

## Selected objects and related suggestions

Selecting an object opens the same right-hand inspector, non-modal, so
the owner can keep working while it stays open. The inspector carries
the object's facts, its actions, and its related objects.

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

**Related suggestions are embedding-driven.** The candidates are the
fact corpus, the skill library, and the project list; a lexical pass
shortlists the candidates worth embedding, and `/api/rank` compares them
by cosine. The percentage shown is the backend's cosine, not a score
this surface invented. With no embedder the card says no close match
rather than inventing an order.

## The interaction layer

No object is decorative. Every row, tile, badge, chart segment, pill,
and node has a purpose and an action, and the same detail drawer serves
them all.

- **Sessions** select in place. The list and the selected replay stand
  side by side; arrow keys move the selection, Enter/click reads it,
  Escape returns to the list, and `#/sessions/<id>` makes the selection
  a link. The archive's filters — open, ended, delegated, compacted —
  are visible chips with a count, never a hidden state.
- **Facts** open their detail and edit; their type and tier badges
  filter the corpus; the drawer can ask the agent about the fact.
- **Route rows** open their walk and offer a sys1 second opinion;
  chain candidates copy the model id.
- **Calibration cells** open the fit and the rule, with a gate-review
  action.
- **Graph nodes** open their unlock conditions and hand the node to the
  composer.
- **Project rows** open their git facts; feed rows are the evidence
  behind them.
- **Tiles** open the metric's derivation and the list below it; session
  tiles apply the matching filter.
- **Topbar chips** open the surface that explains them; the route opens
  Router, the state opens the archive at this run, and spend/context
  open the derivation.
- **Method and lanes** are actions: Plan toggles plan mode; Delegate
  opens the graph; Verify asks sys1 for a gate review; memory, skills,
  web, and deck open or dispatch their lane.

The drawer is the one detail surface. It never navigates away from the
panel behind it, and its footer carries the object's actions.

## The surfaces

Ten keyed panels under hash addresses, four destinations in the rail
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
| `#/sessions` | the archive newest-first; a row opens its replay, and the replay's resume re-anchors the face to that log |
| `#/calibration` | the day's stream rows by op and the judged quality's fit cells, the same computation `fleet calib --calibration` rides |
| `#/tower` | the control tower — the one console surface: the KPI strip, the hero activity chart, the harness donut, the ranked projects, the claims against the evidence, the report feed |
| `#/second-brain` | the derived state, quieter: the gates, the inbox, the loops, the harness runs, the maturity week grid |
| `#/settings` | every configurable surface: the knobs, both lists, the rules, the cron jobs, the identity slot, the env names |

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

- A refused read keeps the last good numbers with its reason beside
  them — the console is never a blank screen because a read did not
  land.
- A bounded read names its bound: a replay previews the newest 400 cards
  and says what it left out; the calibration read names its 512-row cap;
  the tower scan names what it left unread.
- No credential value ever reaches a payload. Names and presence only,
  the doctor's masking rule.
- The browser's client is verified by running the real `app.js` in a
  Node `vm` against a stubbed DOM fed the bridge's own documents. The
  harness lives in `tmp/` and is never committed — the repo has no JS
  runner, so the harness is the verification.

## The launch

```sh
fleet gui [--port N] [--no-open] [--host H]
```

Default `127.0.0.1:3080` (the fork's port), `--port 0` for an OS pick.
The address prints; the browser opens unless `--no-open`.
Ctrl-C stops it. The prompt line the model sees names the GUI and its
URL, so the agent knows its interface.
