# The sinkhole audit — information fleet produces and cannot resurface

Audit, 2026-11-14. The question: where does fleet **produce** information
that can never be **resurfaced** — read back, displayed, recalled, acted
on? Sweeps: the face's event/document consumption against the bridge's
production, the calibration stream's op vocabulary against its readers,
FLEET_HOME's JSON files against their consumers, the session-log
Record variants against the surfaces, and the runtime documents
(spine, checkpoints, skills).

The calibration stream is the house's measurement core — trust earned by
measurements — which makes its dark ops the most consequential finding.

## The sinkholes, ranked

### 1. The calibration stream: 18 of 21 op types are written and never resurfaced

The stream records 21 op types (`calib.rs` OP_* constants): footprint,
lifecycle, embed_timing, route, tool_screen, turn, shape, recall_hit,
verify, consolidate, propose, eval_corpus, explain, route_judged,
curate, validate, demand, learn_probe, learn_check, delegation — plus
spend. The readers (`boards.rs`) key specifically on **three**:
recall_hit, route, tool_screen. Everything else lands in the
append-only stream and stays there. The measurements the house's
whole epistemology runs on — the verify verdicts, the consolidation
cycles, the demand signals, the learn probes — are produced and
never shown to the owner or fed back into a decision. *Fix sketch: the
calibration lens groups the day's rows by op with the three known ops
first — the stream already carries everything the render needs.*

### 2. `.session-index.json` — the inverted sinkhole

`fleet recall`'s session channel checks one file's **existence**
(`recall.rs:106`) to decide the channel is "warmed" — and **no writer
exists anywhere in fleet**. The recall-over-past-sessions channel is
permanently closed: every recall answers "not searched — the session
index is not warmed on this machine" unless a file is hand-created.
A gate nothing can open. *Fix sketch: write the index when the archive
read lands, or drop the gate and let the recall build it on first use.*

### 3. Checkpoint size facts — unread on both surfaces

`Checkpoint.bytes` is never read on the face or the TUI; the TUI drops
`files` too; the checkpoints document's `at`/`bytes`/`unread` per
snapshot are computed per read and shown nowhere. The owner judging
rewind cost (disk, scope) has no numbers. *Fix sketch: the checkpoint
strip's title gains the bytes; the checkpoints panel shows at/size.*

### 4. Memory.root — which corpus a run recalled from, unread

The Memory event carries the adopted corpus root (KI-535's fix made it
travel); neither surface reads it. Directly relevant now that corpus
resolution follows the session's folder: the owner can't tell WHICH
corpus a run's recall came from. *Fix sketch: the face's memory strip
names the root's basename on hover; the TUI status shows it.*

### 5. The workspace spine computes its graph twice, and more, unread

The spine document carries `nodes` + `edges` (a full graph walk) that
no surface reads — the connections column reads the separate tissue
endpoint instead, so the same graph is computed twice per workspace
read. Also unread: `threads` (the run→transcript index), `deliverables`
(with computed needs/stage), and the `bounded`/`omitted` truncation
honesty labels (in both the spine and tissue documents). *Fix sketch:
either the tissue read consumes the spine's graph or the spine stops
computing it; `bounded`/`omitted` surface as a footer line.*

### 6. turn_budget — the cap invisible while the counter climbs

`/api/state` computes the run's configured turn cap per read; no
surface reads it. The owner watching the turn counter has no cap line
beside it. *Fix sketch: the pill's hover names "N of cap M turns".*

### 7. CronFire's facts — folded away in the browser face

The event carries minute, catch_up, detail, and the fire's session;
the face's strip reads job+outcome only. Whether a fire was a
catch-up, and a click-through to the session, are folded away. (The
TUI renders all four.) *Fix sketch: the strip's row gains the
catch-up mark and the session click-through.*

### 8. SkillsLoaded.loaded — how many skills the run carried

The face reads only `skipped`; `loaded` (the count of skills the run
actually carried) is unread. *Fix sketch: the strip reads both —
"12 loaded · 3 skipped".*

### 9. Deck.fix_actions — the auto-fit repairs, computed and never shown

The deck event carries the ordered list of auto-fit repairs; zero
reads in the face, dropped in the TUI. *Fix sketch: the deck card's
faults section lists the applied fixes.*

### 10. The Routed event's duplicate `order` key — a real bug

`describe()` emits the `order` key **twice** (bridge.rs:13110 and
13111-13114): the tuple-array form is computed and then shadowed by
the object form (json! last-key-wins). Dead computation and a
shape ambiguity for any consumer. *Fix sketch: delete one arm.*

### 11. Smaller named drops

- `/api/skills`: `skill.modified` and the document's `canonical`
  directory — computed, never read.
- TUI field drops (deliberate thinness, named for completeness): live
  thinking and closed thought cards (incl. seconds), the Tasks pane
  card, GoalDone, tool call arguments and the resolution method line,
  usage's budget/folds/share/cache-read, the route decision's
  verdict/judged_from/reason/execution, Recommended's kind,
  Checkpoint files, Deck's html/faults/fix_deck/fix_actions,
  CronFire's detail, Memory's root, SkillsLoaded's loaded,
  ToolResult's resolution.

## Checked and healthy (coverage)

UiEvent variants: 22 of 26 fully consumed on the face (the four TUI
`{}` no-ops are the deliberate thinness). The memory document's 17
fact fields (basis, recall_count, valid_until, superseded_by,
preview) — all consumed. The settings scalars' source badges, the
armada reliability, the tower projects, the delegation graph's
reason/delta/unlock/critical-path/blocks-most-work/revisions — read.
FLEET_HOME's ~30 JSON files: every one traced writer→consumer except
the two named above (`tokens.json` in canvas bundles is an
external-consumer export by design; `.session-index.json` is the
inverted sinkhole).

## The shape of the fix

Most of these are one-line reads in an existing painter — the
information is already transported; only the render is missing. The
two structural items (the calibration lens's op grouping, the
session-index gate) are small plans. The audit's own finding about
itself: the sinkholes cluster where a producer was added for
completeness and no consumer was built the same day — the house rule
"every slice ships its user-facing surface with the feature" applies
to reads too.
