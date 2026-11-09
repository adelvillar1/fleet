# Deep dive: the open KI ledger, 2026-11-09

A current-state survey of the ledger's open rows — `docs/known-issues.md`
at master `15fa424`: **501 rows, 11 open** (the checker's own count,
`scripts/ki_ledger.py`). Every row was re-read in full and re-verified
against the live tree and environment this session; where the row's
evidence has aged, the delta is named. One parse trap worth recording up
front: a naive column split on `|` misreads rows whose cells carry pipes
(KI-331 reads as open on the shifted column and is in fact closed — the
checker's right-anchored parse is the honest one), so the dive trusts the
script's count, not a grep.

The severity mix of the eleven: 7 defects (KI-396, KI-455, KI-456,
KI-459, KI-476, KI-493, KI-494), 2 gaps (KI-252, KI-492), 2 improvements
(KI-466, KI-480). They fall into four clusters by cause and fixer, and
the clusters are the useful unit of work: three rows are one upstream
defect pair, two rows are one feature pair, three rows are yesterday's
live-surface finds with repair paths already scoped, and three are
owner rulings wearing row numbers.

## Cluster A — the plan-gate pair upstream (KI-396, KI-455, KI-456)

Three of the eleven rows are **two** defects, both in
`~/Projects/dev-decisions` (`scripts/devdec/gates.py`), both filed there
as D-001 and D-002 on 2026-11-06, both **still open there** (verified
this session — both rows carry `| open |`):

- **D-001 = KI-396 + KI-455** — the per-section character cap
  (`_PLAN_GATE_SECTION_CHARS = 2000`, applied at `gates.py:418` as a
  silent slice) truncates a plan's work sections without naming the cut,
  so a coverage fix landing past the cap cannot move the verdict and
  reads as ignored. KI-396 was the first sighting (a C6 miss pinned at
  p=0.31 across two plan versions differing only past char 2000);
  KI-455 the second, with the sharper reproduction (a 4230-char
  Approach gated 15/19 twice; splintering the body into `###` sections
  under the cap moved the same run to 19/19 — the gate's own parser
  splits on both `## ` and `### `, so the splinter is a real workaround
  and a `## `-only hand count reports wrong sizes).
- **D-002 = KI-456** — `_parse_plan_criteria` matches every checkbox
  line in the file with no section filter, so a plan's template-mandated
  `## Use cases` contracts are consumed as the first criteria and every
  real criterion's id shifts by their count (a 14-criterion plan gated
  as 19; a MISSING flag on "C9" names the author's C4). Fleet's own
  `scripts/plan_check.py:39-48` already scopes to boxes under `##
  Acceptance criteria` and says in its docstring why — the fix is to
  port that scoping and print each criterion's leading words beside its
  index.

**Cost and value.** Both fixes are small, well-scoped, and guarded by
the next-action cell's selftest requirement (a section past the cap
whose note names it; a use-cases block whose parsed count equals the
declared count). No vendored copy of the gate lives in fleet's tree
(verified: no `gates.py` under `services/`), so the repair stays purely
upstream and closes three fleet rows at once. This cluster is the
highest leverage of the eleven: the gate reads every plan the house
writes, and its two blind spots corrupt exactly the verdicts (coverage
MISSING flags, criterion ids) that plans act on.

## Cluster B — yesterday's live-surface finds (KI-492, KI-493, KI-494)

All three opened 2026-11-09 from the owner's failed chat; all three
re-verified live this session.

**KI-493 (LayerFS substrate down) is live right now.** Port 49473 —
the persisted port in `~/.fleet/layerfs-home/worker/endpoint.json` —
answers REFUSED, `docker ps -a` and `docker images` are header-only
(the store is still empty, the registered volume `layerfs-fleet-store-*`
among the gone), so the pinned `node@sha256:3d27…` container cannot
exist and both delegated children's death shape stands. The row's own
triage is right and unchanged: this is provisioning, not code — `docker
pull node@sha256:3d27…` then `fleet install` (the install spawns the
worker and re-records the endpoint) — and it is owner-gated only by the
pull's size. Until it runs, **every delegation dies at workspace open**;
`fleet doctor` is the one surface that names it. The code-side nicety
(a refusal that says "the substrate is down, `fleet install` is the
repair" instead of relaying ECONNREFUSED) is a small bridge change that
can ride any later face cut.

**KI-492 (the silent no-folder grant) stands in code.** The none arm of
`run_folder` (`src/gui/bridge.rs:4644`) still returns `Ok(None)` at
`:4670` without emitting the folder event the fallback arm emits, so a
run without a grant produces a transcript of one-sentence tool refusals
and no cause, while the run bar paints the attached session's cached
`effective_folder` — the mismatch the owner saw. The fleet-default half
of the incident is already discharged (the owner's default now exists in
`sessions-workdirs.json` under `""`, so new chats inherit a grant); what
remains is exactly the row's three-line fix path: the absent-folder arm
records the same event the fallback arm does, the bar paints the folder
the *next run* will use, and a grant-less run names its state in the
transcript before the first refusal does. Harness-testable end to end.

**KI-494 (the two dead MCP lanes) has aged in one detail.** The row's
evidence says the `computer-use` entry carried args and **no command**;
the live `~/.fleet/mcp.json` now carries a `program` — but it is
`./Codex Computer Use.app/…/SkyComputerUseClient`, a **relative** path
whose bundle exists nowhere findable (not `/Applications`, not `$HOME`,
not fleet's cwd; Spotlight returns nothing for the binary), so the entry
still cannot spawn and the row's fix instruction stands with its evidence
cell amended: point the entry at the driver `fleet doctor` already
resolves (`~/.hermes/tools/cua-driver-0.21.0-darwin-arm64/.../cua-driver`,
verified present and executable this session) or remove the entry, since
fleet's own desktop path answers. `fli` is unchanged — the Python 3.13
framework binary whose server import fails inside its own environment —
and the fix remains repair-or-disable. This is minutes of config work
plus one owner decision; it clears the per-session `lane_failed` noise
that made the failed chat's tool surface look emptier than it was.

## Cluster C — the deck-vector pair (KI-252, KI-459)

One feature and its instrument, both parked on honest grounds.

**KI-252's remainder is genuinely unbuilt.** The row stays open for one
thing only: semantic clustering of long sources into infographic blocks —
`grep -ci cluster` is still 0 in both `src/deck.rs` and `src/memory.rs`
(re-run this session). Moves 1–3 of the 2026-10-19 blueprint landed (the
template index at `src/deck.rs:1211`, the asset library at `:1457`, the
Matryoshka two-stage behind `memory.vectors.two_stage`), and the
2026-11-06 re-measure at the 500-fact trigger printed `mean recall@10
agreement 1.000` over 625 facts — recall is settled; only cost (KI-459)
and clustering remain. The setting is confirmed **default OFF**
(`src/memory.rs:2658`). Clustering is feature-shaped work: it needs a
plan, not a triage.

**KI-459 is a small, sharp instrument fix that unblocks a standing
decision.** The two-stage measurement harness (`the_two_stage_measurement`,
`src/memory.rs:7406`) wraps the query's own embedding call inside the
timed region, so its wall clock cannot compare ranking strategies — the
four 2026-11-06 runs showed two-stage "never cheaper" with deltas
(54/47/1/1 ms) inside one embed call's own spread, while the same
benchmark's per-instruction cost *fell* as the corpus grew sevenfold,
which is the tell that the ranking is not what was timed. The mechanism
is confirmed in code: `vector_prefilter` (`src/memory.rs:1759`) pays a
full slice-scored pass and prunes nothing from `Memory::score`'s
every-fact loop. The fix is the row's own: a vector-taking seam so each
instruction embeds once outside the timed region, then single-pass vs
two-stage over the same vectors, repetitions, median reported. Until it
lands, `memory.vectors.two_stage` stays default-OFF **on an unreadable
measurement** — the honest state, but one small harness change away from
being decidable.

## Cluster D — the owner rulings (KI-476, KI-480, KI-466)

**KI-476 (Send button contrast) re-measured identical.** White on
`--dsw-alias-gradient` recomputed this session from the live tokens:
console `rgb(96,143,255)→rgb(66,112,230)` = 3.06→4.48, graphite
`rgb(94,148,255)→rgb(62,112,232)` = 2.93→4.48, light 4.81, contrast
9.92 — byte-identical to the row's numbers, so the two dark themes'
stops have not moved since 2026-11-08 and the failure stands. The
control is unchanged too: `.foot-btn.send` (`src/gui/assets/app.css:3195`)
still paints `#fff` on the gradient — the same button the 0.6.18
composer revision moved into every work surface, which makes it the
app's most-pressed control on every route. The cheaper fix path is as
the row scoped it: darken only the two dark themes' gradient stops until
the whole sweep clears 4.5:1 (light and contrast already pass), and pin
with the mockup's per-theme contrast pass — the measurement is what
survived this long only because nobody had run it. Natural ride-along:
the next face cut (0.6.18 is committed but not yet built/installed, so
its face is still in the pipe).

**KI-480 (a long conversation named by its first line) awaits triage
with candidates already scoped.** The shape is unchanged: `title_ladder`
(`src/gui/bridge.rs:7947`) still has exactly four rungs — owner, goal,
plan, first — and no last-owner-message rung; the session-fact digest is
still the log's head-truncated first 4000 chars of tool calls. The
instance is fixed (the owner renamed the session). The row's three
candidates (join compaction summaries beside the head / a content-match
line without `q` or a last-owner rung / the CLI fact drawing from
whole-conversation text) are a design decision first, code second.

**KI-466 (the dotted fact-name spelling) awaits a one-line ruling.** The
grammar sentence is confirmed shared by all three write paths (now at
`src/memory.rs:4210`, `:4382`, `:4960` — line numbers moved from the
row's, the file grew, the sentence is one). The twice-observed bar is
met (KI-394 + this row), every corpus-refresh run hits the refusal once
and writes the fact dotless — harmless, never lost, and the ignore rule
matches the FILE regardless of the NAME (the row's own analysis, which
holds). Either the scheduled task strings name `corpus-<project>-<date>`
as the file it becomes and the fact name dotless, or the grammar admits
`.`; until ruled, the one-call refusal is cosmetic noise, not a defect.

## The reading

By what unblocks what:

1. **KI-493's pull** is the only row blocking a live capability today
   (every delegation dies at workspace open). One command, one install,
   owner-gated on the pull's size.
2. **Cluster A** closes three rows with two small upstream fixes and
   repairs the instrument every plan gate runs through.
3. **KI-459** is the smallest code change with a real decision behind it
   (the two-stage default).
4. **KI-492** and **KI-476** are well-scoped face fixes that can ride
   the same cut (0.6.18's face is already in the pipe unbuilt).
5. **KI-494** is minutes plus one decision; **KI-466** and **KI-480**
   are decisions wearing row numbers; **KI-252** is a plan-shaped
   feature, correctly parked.

Nothing in the eleven is stale enough to close without work: the two
rows whose evidence aged (KI-494's no-command detail, KI-466's line
numbers) are still true in substance, and the three rows whose triggers
were armed on corpus size (KI-252's 500-fact floor, KI-459's re-measure)
have already fired and been answered — what remains is the fix each row
names, not a re-triage.
