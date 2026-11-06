---
name: fleet-second-brain-visual-redesign-awaiting-pick
description: "owner 2026-10-20 — the second-brain panel was \"rather underwhelming\"; two mockups (Ledger, Atlas) on real tower_data.json; the owner picked Atlas and it shipped as a85472e (attention queue, project pulse, commit ribbon, method band)"
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

**The owner's verdict, 2026-10-20, right after the plugin shipped:** "what
about the visual side, what we have in fleet now is rather underwhelming."
The shipped panel ([[fleet-second-brain-is-a-tool-plugin]]) was a tool card plus
six identical board cards — gates, loops, harness, maturity, tiles — so the
method telemetry was the whole page and the project landscape (the reason the
tool exists: "my attempt at making sense of an ever growing project landscape
as a solo developer") was absent.

**Diagnosis, so it is not re-derived:** flatness is structural, not cosmetic.
Every section was a full-width card at the same weight, and the content that
matters (attention queue, per-project pulse, handoff next lines) was data
`boards.rs` threw away — see the parse gaps in
[[fleet-second-brain-is-seven-fields-in-the-gui]].

**Two directions built as one interactive carousel mockup** (real data, no
source touched):
`~/.zcode/v2/visualizations/d94efb0a.../second-brain-design.html`
- **Ledger** — attention queue leads with class filter tabs, six-week pulse as
  the counterweight, project roster whose right column is each project's own
  handoff line.
- **Atlas** — landscape leads: per-project six-week sparkline + its last
  session's next line; the queue as a right rail of class bars; a commit ribbon.
- The owner's pick, verbatim: "I like the Atlas mockup better than Ledger."

**Status: Atlas shipped 2026-10-20 as `a85472e`, pushed.** The read widened
(plan rows with `stale_days`, `live_handoff` + its `next` score, `drift_days`,
per-day commits, conflicts with project/status/detected_at/key; the 8-row
timeline cap and the 6-row conflict cap removed) plus three tested pure
reductions — `attention_queue`, `atlas_projects`, `commits_by_day`, with
`pulse_windows` anchoring six seven-day buckets on the tool's build day. The
panel: tool instrument in the page head, figures strip, atlas (8 cols) beside
the queue rail (4), commit ribbon, ruled method band. Class rows filter the
rail; a project row opens the dock; a claim opens the claim. Gate 1309.

**Two defects the live pass caught.** (1) A finished run's pipeline steps
rendered under the instrument and pushed the page title the full card height
down — steps now render only while a job runs or after a failure. (2) The
gates' progress bars were empty because `graded_rows` is not carried by the
tool's board rows; the state's own phrase (`3/20`) is the authority — parse the
left side, fall back to the numeric field.

**2026-10-20, later: the Atlas absorbed the retired tower console** (`05fd1ec`)
— the report feed (newest twelve filings) and each harness's trust-precision
line now render as "the report feed" below the commit ribbon, because the owner
called `#/tower` redundant. See [[fleet-gui-surface-grammar-rollout]] for what
was superseded (the directory scan) and the leftover unreachable tower helpers
in `app.js`.

**Tower retirement (`05fd1ec`, 2026-10-21).** The owner: "given that second
brain is fantastic it seems like the tower page is redundant." The Atlas
absorbed the tower's two unique facts — the report feed and the harness
trust-precision lines — as a "the report feed" card; the directory scan was
superseded; `#/tower` was removed from the routes, the rail and the page. The
brain payload now carries `feed` and `trust` inside `reports`. The tower's
renderer helpers remain unreachable in `app.js` (one helper is shared).

**Numbers moved a lot mid-session** (the tool rebuilt at 22:22): the mockup's
33 attention items became 102 (3 drift + 7 stale + 18 handoffs + 74 claims),
28 projects became 32. The commit ribbon is labelled "the tool's newest 400
timeline rows" — the timeline cap makes the left edge empty, and calling it
45 days would be a lie.

**How to apply:** when the owner says a surface is underwhelming, lead with the
information architecture (what leads, what is demoted) before palette or
spacing; populate the mockup with real data (invented figures hide exactly the
empty columns and zero bars that expose the gaps); render the real face at
desktop AND phone widths after shipping. Related:
[[verify-what-renders-not-what-commits]], [[gui-revamp-browser-face-decision]],
[[fleet-gui-editorial-desk-redesign]], [[fleet-memory-and-skills-knowledge-tables]].
