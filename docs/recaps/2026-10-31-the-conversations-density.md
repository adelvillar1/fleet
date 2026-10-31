# The quota unlocks, the count retires, and the conversation gets dense

**2026-10-31 (this session)** · two owner reviews landed whole — the
router's committed quota now unlocks the fallback walk (`8b3564b`) and
the turn count retires from every freshness surface in favor of the
compaction count · then the conversation's density (`5b14bd9`): the
tool and thought cards are one line wearing type icons while the run
runs, the thinking streams to its card, the finished exchange folds to
its marks' summary with the answer full, and the working line reports
at the tail of the conversation — the composer keeps its idle words.
The gate: **1,848 passed / 0 failed / 5 ignored**, the face harness
**509 checks / 54 cases**, the ledger green at **384 rows, 36 open**.

## The session, in order

- **The quota unlock** (`8b3564b`) — the diagnosis: `chat_stream`'s
  committed branch returned on any error without consulting
  `failover_eligible`, so a provider that went out of money after one
  good turn failed the run typed, every turn, until re-pinned. Quota is
  the one class whose recovery is a different provider, so it is the
  one class allowed to unlock: the committed path now clears
  `committed`, records the attempt, and walks from *past* the
  exhausted candidate. Everything else stays fatal-on-committed, and a
  mid-stream failure stays fatal whatever the class. Pinned by
  `a_committed_quota_failure_unlocks_the_walk` over the real HTTP walk
  (`commit_for_tests` is the seam — driving it live needs a tool call
  to force a second turn).
- **The count replaces the count** — `SessionSummary.compactions`
  counts the log's `Compaction` records in the pass `summarize` already
  runs (a fresh log answers a real zero, because "never folded" is a
  fact worth stating). The four surfaces that used turn count as size —
  the delegation tree cards' `said` and root note, the replay's turn
  fallback, the live branch line — read `foldsWording`; the turn
  *budget* and the compaction event's own line are untouched, because
  they are not freshness claims.
- **The density plan** — drafted, amended twice under steer (the type
  icons; then the working line moving *off* the composer — "it's a
  property of the conversation"), gated (coverage 12/12), and built in
  one pass: P0 the two `ICONS` entries; P1 `UiEvent::Thinking` riding
  the reasoning deltas (transport-only — the `Reasoning` record and the
  log unchanged) to one live line the block's close replaces with the
  collapsed card; P2 the one-line cards, the summary moved into the
  detail beside the command; P3 the exchange's fold at the final output
  through the D8 `toggleTurn` state, each mark carrying
  `turn N · N tools · thought Ns`; P4 the `working-live` line with the
  composer's `busy` class and working/seat branches removed.
- **The screenshot caught the defect the harness could not** — the
  first live probe measured the tool cards at 47px: one line *plus*
  one, because the card body is a grid and the status stacked under
  the tool's name. The body is now a baseline flex row — title
  truncating, status at the right end — and the cards measure
  **32px, one line, at 1440 and at 1100** on five real tool rows.
- **The compaction commit's missed hunk** — `src/tui/app.rs`'s test
  constructor lacked `compactions: None`: master's committed test tree
  did not compile, while every local gate stayed green because the fix
  sat uncommitted in the working tree. Rode the density commit, named
  in its message; the lesson is saved (a green gate can ride an
  uncommitted hunk — diff any pre-modified file fully before staging).

## Plans worked on

### `docs/plans/2026-10-31-quota-failover-and-compaction-count.md` — the quota failover and the compaction count

Status after this session: **completed** (flipped this recap — the
work landed at `8b3564b` with its Landed section; the frontmatter had
been left `active`).

| Acceptance criterion | Status | Notes |
|---|---|---|
| Q1 the committed quota unlocks the walk | ✅ met | the integration pin walks the real HTTP chain past the exhausted candidate |
| Q2 every other committed failure stays fatal | ✅ met | the existing fatal-path test still pins a network class |
| Q3 the summary carries the compaction count | ✅ met | three folds answer three, a fresh log a real zero |
| Q4 the freshness surfaces swap | ✅ met | four surfaces read `foldsWording`; the budget and event lines untouched |
| Q5 the gate | ✅ met | fmt/clippy/build green, harness 485/53 at landing |

### `docs/plans/2026-10-31-conversation-density.md` — the conversation's density

Status after this session: **completed** (landed whole at `5b14bd9`,
the Landed section carrying the probe's measurement and the fix it
found).

| Acceptance criterion | Status | Notes |
|---|---|---|
| R1 the cards carry the icons | ✅ met | `tools`/`thinking` SVGs on both heads, harness + probe (`glyphSvg=true` × 5) |
| R2 the thinking streams | ✅ met | `UiEvent::Thinking` + `the_thinking_delta_shape`; the face half in D1 |
| R3 the completed thought is its collapsed card | ✅ met | D1: line gone, card collapsed, block one press away |
| R4 the cards are one line collapsed | ✅ met | 32px at 1440 and 1100, measured on the live face |
| R5 the completed turn summarizes | ✅ met | D1: rows folded, mark carries the counts, press expands |
| R6 the answer stays open | ✅ met | D1: the assistant row is never a fold target |
| R7 history is never re-folded | ✅ met | D1: a replayed log keeps its rows; folds restore, never invent |
| R9 the working line on the conversation, absent from the composer | ✅ met | D1 pins words/tick/seat/removal + the composer's silence; the chat case's stale hint check moved to the line |
| R8 the gate | ✅ met | fmt 0, clippy 0, build 0, 1,848/0/5; harness 509/54 |

## Commits

| Hash | Message |
|------|---------|
| `8b3564b` | fix(route,face): the quota unlocks the walk, and the fold count replaces the turn count |
| `5b14bd9` | feat(face): the conversation's density — one line while it runs, one summary at the end, the answer full |

(Same-day parallel-session commits — the pass-two shakeout recap, the
corpus refresh, the two settings plan drafts — are recorded in their
own recaps and plan files.)

## What was added

- **The working line** — the run's live state at the tail of the
  conversation: spinner, "working", the turn/exchange, the elapsed
  time from the same `face.process.at` anchor, ticked by the one-second
  timer; a standing seat swaps to its own wait. Display only, outside
  the stream's rows, gone the moment nothing is live.
- **The thinking stream** — `UiEvent::Thinking`, one send per reasoning
  delta (the `Token` precedent), the wire shape `type: thinking`, the
  TUI ignoring it beside `Reasoning`.
- **The turn's summary** — `summarizeRun` at the run's end folds the
  exchange's tool/thought rows under their marks with the counts; the
  per-visit seed clears folds so turn numbers cannot leak between
  conversations, and a replay restores applied folds rather than
  inventing them.
- **Two icons** — `tools` (a call at the prompt) and `thinking` (the
  drawn diamond) in the shared `ICONS` table.

## What was fixed

- **The committed quota lock-in** — a provider that ran out of quota
  after serving once failed every following turn fatally; the walk now
  recovers within the run.
- **The two-line tool card** — the grid body stacked the status under
  the name; the baseline row makes the card honestly one line (47px →
  32px, live-measured).
- **The compaction commit's uncompilable test tree** — the missed
  `compactions: None` constructor field, carried in `5b14bd9`.
- **The plan-mode hint clobber** — `paintComposer` used to overwrite
  plan mode's words on the next idle repaint; the hint now derives
  from `face.planMode` every paint.

## What was changed

- **The composer's hint** — idle words in every state; the `busy`
  class and the working/seat branches left `paintComposer` for
  `paintWorking`. One ticker, alive only while a run or seat is.
- **The freshness measure** — four surfaces read the compaction count
  (`foldsWording`, "never folded" at zero) where they read turns.
- **The tool card's collapsed composition** — the summary moved from
  the card's line into its detail beside the command.

## Files changed

**Backend / API**
- `src/route.rs` — the committed quota unlock; the walk pins
- `src/session.rs` — `SessionSummary.compactions`; the pin
- `src/agent.rs` — `UiEvent::Thinking` and its send beside the reasoning deltas
- `src/gui/bridge.rs` — the thinking wire shape + `the_thinking_delta_shape`; the delegation nodes' `compactions`
- `src/tui/app.rs` — the Thinking ignore arm; the missed `compactions` constructor field

**Frontend**
- `src/gui/assets/app.js` — the two icons, the live thinking line, the one-line cards, `summarizeRun`/`paintTurnSummary`, `paintWorking`, the replay guard and fold restore, the four `foldsWording` surfaces
- `src/gui/assets/app.css` — the baseline tool body, the working-live line, the thinking-live line, the glyph icons
- `src/gui/assets/index.html` — the `#working-live` element at the conversation's tail

**Tests**
- `tests/face/harness.mjs` — the D1 density case (24 checks); the chat case's exchange check moved to the working line; the FOLD case from the quota work
- `tests/integration.rs` — `a_committed_quota_failure_unlocks_the_walk`

**Documentation**
- `docs/features/routing.md` — the quota unlock beside the failover's ruling
- `docs/features/gui.md` — the density passage (the activity rows rewritten) and the working line
- `FUNCTIONAL-SPECIFICATIONS.md` — the transcript's density contract; the fold-count and quota-switch lines
- `docs/plans/2026-10-31-quota-failover-and-compaction-count.md` — Landed; status flipped to completed this recap
- `docs/plans/2026-10-31-conversation-density.md` — Landed with the probe's measurement and the fix it found

## Doc updates applied

- `docs/features/routing.md` § failover — the committed quota unlock, the mid-stream rule kept fatal
- `docs/features/gui.md` § interaction layer — the activity rows rewritten as the density contract; the new working-line bullet
- `FUNCTIONAL-SPECIFICATIONS.md` § run behavior — the transcript's one-line/one-fold/answer-full contract

## Doc updates deferred (debt)

- None from this session — both plans' linked artifacts were updated
  as part of each landing.

## Open questions / next steps

- **The shakeout's remaining passes** — the manifest's passes two/three
  ride the owner's own use; the beta work mode continues.
- **The open debts from earlier recaps stand** — the three pending
  skill proposals, KI-367, the right pane's P3/P4, the task-field send
  bound (KI-378's row).
- **The parallel session's plan drafts** (`91b7db1`, `81955e4` —
  provider models in settings) await their build.

## Notes

- The density plan was amended twice under steer before the build
  (icons on the cards; the working line off the composer) — the plan
  gate was re-run after each amendment, coverage 12/12.
- The visual review continues to earn its keep: the grid-body defect
  was invisible to 509 harness checks and died on the first
  screenshot's reading. Re-shoot after every state-dependent change.
- `cargo` is not on PATH in this shell — the toolchain bin dir exports
  before every gate command.
- The gate closed at 1,848 / 0 / 5 (`--lib` 1,657); the face harness at
  509 / 54; the ledger checker green (384 rows, 36 open — the
  parallel session's finds included).
- The face is live at `http://127.0.0.1:3080` serving the density
  build; the probe screenshots (`tmp/density-*.png`) are this
  session's evidence.
- Process sweep: registry empty (0 killed). Two pre-existing orphans
  reported, left alone — a 5-day `node-code-sandbox-mcp` (npx) and a
  3-day blend1 venv python; the owner's call whether to prune.
- Cache sweep: debug 21.4 → 20.8 GiB, the dev profile cleaned whole
  (over the 15 GiB backstop); release targets untouched, 54.6 GiB
  free.
