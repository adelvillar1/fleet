# The landing rethought, the details at hand, and names that follow the work

**2026-10-30 (third session)** · the documentation re-shot from the
rail-enabled face and the idiom named in the README · the three
owner-reported frictions fixed under one plan — the rail's collapse at
the head, the instrument that fits, and the landing rethought to greet
the owner's own work · and conversation names gone dynamic: a
four-rung ladder that names a conversation by what it **is** — the
owner's rename, the standing goal, the last plan — not the words it
started with. Both plans completed; the face harness closed at **474
checks across 51 cases**; the suite at **1,842 passed / 0 failed**;
the ledger checker green at **377 rows, 33 open**. Everything pushed
through `d8b3447`.

## The session, in order

- **The documentation refresh** (`6699961`, `1274634`) — all thirteen
  gallery images re-taken with Playwright at 1440×900 from the live
  face (the gallery had gone stale for six pages, not four — it
  predated the settings and bots revamps too), the README naming the
  idiom the long pages share, `surfaces.md`'s four stale passages
  rewritten in its own What-it-is / What-you-do / Why / State shape
  with the states measured that day.
- **`landing-and-rail-details`** (`d1289ad`) — the three
  owner-reported frictions, each diagnosed on the live face before
  written: the collapse moved from the rail's foot to the head (its
  center measured exactly on the topbar's, 28.5px), the instrument's
  buttons wrapping as whole units instead of breaking their labels
  inside fixed heights, and the landing greeting **what needs you**
  (runs at approval seats or paused) and **what is waiting** (skills
  proposals, the memory cycle's queue, calibration's short cells) —
  with `owner_recent` tightened to exactly `class == "yours"`, the
  unclassed-run leak closed at its third surface.
- **`dynamic-conversation-names`** (`d8b3447`) — the title ladder:
  the owner's rename first (never overridden), then the standing goal
  (what the conversation is *for*; it moves when the work moves),
  then the last plan's first line, then the first message.
  Read-time derivation from sources the substrate already carries —
  no model call, no new store — with `title_from` naming the rung
  that served and the header's tooltip saying what named a
  conversation. Verified live: one conversation already names itself
  by its goal rung, one by the owner's rename.

## Plans worked on

### `docs/plans/2026-10-30-landing-and-rail-details.md` — the landing and the rail's details

Status after this session: **completed** (was `active`).

| Acceptance criterion | Status | Notes |
|---|---|---|
| L1 the recent list is the owner's, exactly | ✅ met | the predicate pin; the build caught the empty-title rows reading as noise — "a conversation · <when>" |
| L2–L3 the collapse at the top, the icons head | ✅ met | 28.5px == the pane expand's center, measured |
| L4 the instrument fits | ✅ met | whole buttons at 1440 and 1100; the card grows |
| L5–L7 the greet's rows, the queues, the grammar | ✅ met | the LAND harness case; the greet said 6 when the queue held 3 — the count reads the queue's own statuses now |
| L8 the gate and the measurement | ✅ met | the suite green; the probe's shots read as the review |

### `docs/plans/2026-10-30-dynamic-conversation-names.md` — dynamic conversation names

Status after this session: **completed** (was `active`).

| Acceptance criterion | Status | Notes |
|---|---|---|
| D1 the summary carries the plan rung | ✅ met | the LAST plan's first meaningful line; a plan-less log answers None |
| D2–D3 the ladder's order, the rename wins | ✅ met | two pins over the extracted pure fn |
| D4–D7 the surfaces inherit, the rung rides, the buttons work | ✅ met | the NAME harness case; the header's tooltip |
| D8 the gate | ✅ met | the four exit codes + the harness count quoted |

## Commits

| Hash | Message |
|------|---------|
| `6699961` | docs(readme): the gallery re-shot from the rail-enabled face, and the idiom named |
| `1274634` | docs(architecture): the four rail surfaces' passages carry the idiom, the verbs, and the honest states |
| `d1289ad` | feat(face): the collapse at the head, the instrument that fits, and the landing that greets the owner's work |
| `d8b3447` | feat(face): dynamic conversation names — the title ladder, read-time |

*(The curator's arc — `7b96232` through `e099d40` — is another
session's work with its own recap,
`2026-10-30-the-curator.md`.)*

## What was added

- **The landing's greet** — what needs you (the owner's runs at
  approval seats or paused, one row each, straight to the
  conversation) and what is waiting (the queues the rail revamps made
  legible, each only when non-empty, each addressing its surface).
  The home read carries `needs_yours` and the waiting counts, cached
  30s, so the door and the surfaces cannot disagree.
- **Dynamic conversation names** — the four-rung ladder derived at
  read time; `summarize` carries `last_plan` in the pass it already
  ran; the rail, the sessions list, the landing's recent rows, and
  the header all read the same field, and `title_from` makes a
  derived name diagnosable.
- **The gallery and the architecture atlas at the truth** — every
  surface screenshot from the current build, `surfaces.md` carrying
  the rail idiom and the measured states (449 skills, three proposals
  waiting; 114 live facts; thirteen calibration ops).

## What was fixed

- **The unclassed-run leak** — `owner_recent` passed `class == None`,
  KI-372's exact hole at its third surface; the machine's sessions
  landed on "pick up where you left off" as bare ids. The predicate is
  exactly `class == "yours"`, pinned.
- **The empty-title rows** — the owner's own runs carried empty titles
  and read as machine noise; an untitled conversation reads as
  "a conversation · <when>" now.
- **The instrument's clipped buttons** — flex-shrink squeezed three
  buttons past their labels inside fixed heights; whole buttons wrap
  with a row gap and the card grows, verified at two widths.
- **The greet's honest count** — it said 6 proposals when the queue
  held 3; the count reads the queue's own statuses (pending/waiting).

## What was changed

- The left rail's head carries the brand mark and the collapse on one
  row, the toggle's center exactly level with the topbar's pane expand
  icon; the icons mode keeps the mark centered with the chevron
  floating on the edge. The foot keeps the where-line and the jump.
- The functional spec gained "The landing" — the greet's contract —
  and "Conversation names" — the ladder's.

## Files changed

**Backend / API**
- `src/gui/bridge.rs` — `owner_recent`'s predicate, `title_ladder`, the home read's `needs_yours` + waiting counts, `title_from`
- `src/session.rs` — `summarize`'s `last_plan`; `SessionSummary` grows

**Frontend**
- `src/gui/assets/index.html` — the rail head, the greet's slot
- `src/gui/assets/app.js` — the greet builder, the recent labels, the header's tooltip
- `src/gui/assets/app.css` — the head row, the greet, the instrument's wrap

**Tests**
- `tests/face/harness.mjs` — 472 → 474 checks across 51 cases (the LAND and NAME cases)
- `src/gui/bridge.rs`, `src/session.rs` tests — the ladder's three pins

**Documentation**
- `README.md` — the gallery + the idiom
- `docs/images/` — thirteen re-shot surfaces
- `docs/architecture/surfaces.md` — the four rail passages
- `docs/features/gui.md` — the landing's greet, the naming ladder
- `FUNCTIONAL-SPECIFICATIONS.md` — "The landing" and "Conversation names"
- `docs/plans/2026-10-30-landing-and-rail-details.md`, `docs/plans/2026-10-30-dynamic-conversation-names.md` — completed with Landed sections

## Doc updates applied

Both plans' linked artifacts were applied in the same commits as the
work — `docs/features/gui.md`, `FUNCTIONAL-SPECIFICATIONS.md`. Nothing
deferred.

## Open questions / next steps

- **The three genuinely-pending skill proposals** still wait the
  owner's approve; the landing's greet now names them.
- **KI-367's disposition** and the relay's one-tap install acceptance
  remain the owner's calls.
- **The curator's work** (another session's, landed today) — its own
  recap carries its next steps.
- **A model-named session** remains the named follow-up if the
  ladder's rungs prove too coarse.
- **The strays** in the working tree — keep/commit/retire, the
  owner's call.

## Notes

- The gate held all session: the suite closed at 1,842 / 0 / 5, the
  face harness at 474 / 51, the ledger checker green (377 rows, 33
  open — the curator session filed its rows).
- The measurement habit caught three defects the harness could not:
  the toggle's 7px sag before the height calc was leveled, the
  buttons' clipped labels, and the greet's 6-vs-3 count — the probe
  after every face change stays load-bearing.
- Cache sweep at the close: debug 20.5 GiB → 18.7 GiB, 64.5 GiB free;
  the process registry empty.
- The face is live at `http://127.0.0.1:3080` serving the session's
  build; the greeting, the collapse, and the dynamic names are all on
  it.
