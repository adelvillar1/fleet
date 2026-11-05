---
name: fleet-gui-editorial-desk-redesign
description: "owner direction 2026-10-19/20 — the GUI is an editorial desk (paper-first, NOT dark-by-default), a home briefing answering four questions from real reads, a "no dead objects" interaction contract, a live route/think/act/verify process card, and a docked selected-object inspector with embedding-ranked related objects; COMPLETE 2026-10-20 — 497b60b (slice) then ddb8724 + cd8ad1a (completion plan), pushed, gate 1286"
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

The owner's 2026-10-19/20 verdict on the shipped GUI: "functionality is fine but
design wise is really meh, underwhelming… it looks like amateur hour", plus a
sharpening of intent across the same turn — "the UI should invite the user to
keep on using the harness, it should feel like I belong here", "make me want to
go back to the harness every day, I spend more than 12 hours a day", "I cannot
approve what I have not seen", "no dark mode as default, that's a lazy and
amateurish way of designing ui", "every object you are displaying needs to have
a purpose and an interaction layer", "if we are running a loop or a workflow I
would like to see that on some sort of card", "objects selected should have a
corresponding right side collapsible card", and "even suggestions driven from
the embeddings".

**The design ruling: the editorial desk.** Paper is the default ground (warm,
slightly green-grey; near-black ink; one signal blue), with `console`,
`graphite` and `contrast` as remembered options. Three type voices — New York /
`ui-serif` for display only, the system sans for every readable label, SF Mono
for every measurement, ID and fact name. `themes.rs` base is `paper`, base ink
pinned `(24,26,29)`. The stored theme key is `fleet-theme-2` so an old stored
`console` cannot override the default. Owner complaint that drove the whole
pass: "it looks like amateur hour", and the durability requirement — "make me
want to go back to the harness every day" — so the brief is a place the owner
returns to, not a page that is merely clean.

**The home briefing answers four questions from live reads**: what fleet knows,
the work and projects, the workflow (Plan → Delegate → Verify with the graph's
next ready node), and how it cuts the grind (review today's changes, resume,
the lanes). Backed by one bounded `GET /api/home` assembled from the panel reads
concurrently (`tokio::join!`) and cached 30s in `Bridge.home_cache`.

**The dock is the inspector's home** (completion, ddb8724). Above 1180px the
`.face` grid is `nav | content | dock`; the dock holds the process card, the
selected object, then the session and trust cards. At 1181–1599 opening the
dock auto-folds the nav to icons (the owner picked this over a bare overlay);
expanding the nav while the dock is open turns it into a fixed overlay, and
closing restores the prior nav mode held in memory (`face.railBeforeDock`), not
localStorage. At ≥1600 all three columns stand. At ≤1180 the dock is gone: the
process becomes the strip above the composer and objects use the fixed drawer.
Dock and pane are mutually exclusive (`body[data-pane=open] .dock {display:none}`).
Sessions split list/replay only while the content track ≥1000px; below that the
replay becomes the dock's object card, and at ≤1180 it renders full-width.
Layout trap fixed here: `.dock-object-body` now uses `grid-template-columns:
minmax(0,1fr)` with `overflow-wrap:anywhere` and the dock has `overflow-x:hidden`
— long fact paths and form values were clipping at the window edge.

**The process card** (owner: "I'm a visual person… show where we are"):
Route → Think → Act → Verify as a vertical stepper, reduced from the same event
log (`face.process`, `advanceProcess`); plan mode uses plan → review → build.
Completion added: **live delegation branches** (`p.branches` keyed by child
session, three newest, click opens the child's replay in the dock), **the gate**
from `/api/state` (`gate: {name:"sys1", informs_failover}` read from
`route::calibration_config`; verify reads `gate on · sys1`), a finished-run
**"ask sys1 for a review"** that fills the composer without sending, and
**stop/steer** in both the card and the narrow drawer.

**Stop/steer is the KI-248 stop + inject half, shipped for the browser**
(via the agent's existing control-file protocol, `delegate::write_control` →
`<sessions>/<session>.control`). `Bridge::stop(steer: Option<&str>)` returns
409 when no run is in flight or no session log exists; `POST /api/stop` and
`/api/steer` sit beside `/api/run`; the control lands in the event log as a
`control` card. The thread calls `clear_control` (the log path with `.control`)
after `agent.run` returns and on every early-continue, so a stop that lands on
the last step cannot end the next run — there is a test for exactly that.
Latency is one agent step. The core reports an owner stop THROUGH `failed`, so
the client detects `/stopped by the owner/i` and renders "stopped by you"
everywhere (end card, header pill, facts, process card) instead of a failure.
Still open from KI-248: the side conversation with the orchestrator, and
targeting a child branch. KI-249 (timeouts) untouched. 2026-10-20: the same
control surface (plus a run card, plan mode, checkpoints/diff, a palette and
knowledge panels) was then built into the terminal face — see
[[fleet-tui-revamp]].

**Related objects are one server-side index** (`src/gui/related.rs`), replacing
the browser's 12-candidate shortlist + per-request `/api/rank` embed.
`GET /api/related?kind&id&limit` compares across fact | skill | session |
project | route | cell and returns raw cosine with `weak` under 0.72 (KI-058's
measured gap: unrelated 0.58–0.66, related 0.77–0.93), plus `scores`
cosine|lexical and `index` warm|warming. Facts reuse `Memory.vector_of` (the
recall index; new accessor) and sessions reuse the session index through a new
cache-only `SessionIndex::cached_vectors` (split out of `vectors_for`).
`<FLEET_HOME>/.related-index.json` owns the cold kinds; skills + projects warm
once in `spawn_blocking` at launch, and the warm ends by pre-building the corpus
so the first click is a read. Rules learned the hard way: **never embed a cold
corpus on a request** (one per session ≈ 1 min, and status's archive was cold —
the first version took 64s); **build the corpus with no lock held** (the first
version held the store's mutex across 400 embeds); and the scoring mode must be
decided by the SELECTED object's vector, not the corpus's (`any()` returned
cosine for an unembedded skill and answered an empty list). Warm corpus ~13
entries, lookup 20–45 ms.

**"No dead objects" is now audited, not claimed.** Every panel has filter chips
with counts (memory all/live/protected/retired; skills all/used/unused/
proposals/skipped; calibration all/fit/informing; tower all/dirty; sessions as
before) and the matching KPI tile applies its filter via `tile(value,label,
tone,run)`. Router hour columns, donut segments/legend, and the tower activity
days filter their lists through transparent `.chart-hit` SVGs and leave a
removable `filter-chip-active`. Feed rows open their project; claims, gate
verdicts, recall hits, skips, and curation candidates open their record.
`tile()` stamps `data-act`, and a Playwright pass asserts every visible
`.tile .badge .rrow .rank-row .cal-cell .stream-line .cand .chart-col
.related-row` has `data-act`, role=button, or a title.

**Usage adaptation** (`src/gui/usage.rs`): `<FLEET_HOME>/ui.json`, counters and
`last` stamps only — suggestions {shown,accepted,dismissed,last}, panels
{opens,last}, actions {uses,last}. Atomic temp+rename, 400/entry and 256 KB caps
with least-recently-touched eviction, malformed/foreign-version reads empty, ids
refused past 120 chars or with control chars. `GET/POST /api/usage`;
`ui.adapt` (default on) is a normal setting in `settings.rs`, so the writer
still owns it. Accepted-over-shown multiplies offers and related rows by at most
1.35; nothing hides. Verified live: accepting "router" moved it from last to
first after reload.

**Process lessons.** The owner twice corrected my process — "you should have
created a plan first instead of jumping into development" and "I need as part
of the plan mockups of the ui" — so a redesign plan must ship ASCII mockups.
A third statement the same rule arrived 2026-10-20 while scoping the
second-brain surface ("before implementation, let's brainstorm and write a plan
first"); it lives in [[plan-brainstorming-before-development]].
The owner also chose between two dock options when asked. Two late bugs came
from races between a reply and the run's own end record (stop/steer handlers
overwrote a finished run's card); both now guard on `face.process === p`.

**State at completion (2026-10-20).** Pushed: 497b60b (slice), ddb8724 (the
completion plan), 9ad62d9 (recall-count bookkeeping), cd8ad1a (narrow-drawer
stop/steer). Gate: 1155 lib + 103 integration + 6 blend1 + 2 eval + 7 improve +
13 install + 2 mock ≈ **1286 passed, 1 ignored**; release built and the running
GUI on :3080 restarted. Test changed in src/memory.rs (corpus-growth, not this
work): `the_history_channel_separates_the_corpus_that_saturated_it` asserted
every distinct recall count maps to a distinct term, but a fact now sits past the
128 ceiling where `history_term` deliberately clamps to 1.0 — now it asserts
strict increase below the ceiling and saturation at/above it.
**Operational gotcha:** a fresh GUI launch warms ~400 skill vectors against the
embedding endpoint; running the full gate at the same time starves it and
`gui::server::tests::the_rank_route_orders_candidates_by_what_was_typed` flaked
twice on zeros. Kill the live GUI before the full gate (system_reminder memory
indexed this too).

**How to apply:** paper stays the default; dark themes are options. Any new
rendered object needs an entry in the interaction contract and a `data-act`
before it ships. Verify by rendering (Playwright) at 1920/1600/1440/1280/1181/
1024/900/390, never by CSS inspection; the Playwright module lives at
`/Users/alejandrodelvillar/.hermes/hermes-agent/node_modules/playwright-core`
with the executable at `.../ms-playwright/chromium_headless_shell-1248/
chrome-headless-shell-mac-arm64/chrome-headless-shell`. **2026-10-20:** that
browser build was missing and `require` of another project's copy pinned a
different version (`chromium_headless_shell-1234`), so launch died with
"Executable doesn't exist" — the recovery that needed no install was
`chromium.launch({ channel: 'chrome' })` against `/Applications/Google Chrome.app`,
passed in as an absolute path
(`require('/Users/alejandrodelvillar/Projects/mahjong-vtt/node_modules/playwright')`).
`page.click` fails on
elements under `[hidden]` panels and on 0-size SVG nodes — scope to
`#panel-x` or `.evaluate(() => el.click())`. Rebuild after asset edits (the
`include_str!` page is embedded). Related:
[[gui-revamp-browser-face-decision]], [[verify-what-renders-not-what-commits]],
[[gui-design-research-2026-10-17]], [[gui-client-verified-by-a-vm-harness]],
[[fleet-run-control-and-timeout-gaps]], [[fleet-memory-corpus-dirties-the-tree]].
