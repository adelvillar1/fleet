---
name: fleet-memory-and-skills-knowledge-tables
description: "2026-10-20 — owner: memory and skills \"also look meh\" and \"there are no actions in the cards\"; both rebuilt as knowledge tables with per-row actions and a rail (e7e36c2), sharing the Atlas head/strip/rail classes; the file action forced a real pane-containment fix"
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

**The request, verbatim:** "let's improve the memories and skills pages, the
also are a bit underwhelming and there are no actions in the cards and now
compared to second brain they are look like meh." So the bar is now the Atlas
surface ([[fleet-second-brain-visual-redesign-awaiting-pick]]) — every
knowledge surface is expected to have a compact instrument, a figures strip,
actions on the rows themselves, and a rail.

**What shipped, `e7e36c2`, pushed.** Memory: a corpus table (name, type,
basis, recalls, state) with **edit / ask / file** on every row; a rail with
today's recall (this run's recalls + the day's hits) and the corpus mix by
type and basis. Skills: a searchable library table (442 rows, first 30 by
default, show-all on demand) with **open / path / ask / copy** per row; a rail
with the decisions queue (approve + read per proposal), the loader's declines,
and the curation note. The head/strip/instrument/rail CSS was made shared
(`surface-*`) and the Atlas reuses it.

**The lesson the file action taught.** The pane is contained to the launch
directory (`pane_path` canonicalizes and refuses outside `cwd`), so passing a
fact's absolute path produced `No such file or directory` — concatenated onto
the root. `memory_read` now also carries `relative` (the fact's path stripped
of the launch dir) and the row opens that; a fact outside the working directory
copies its path instead. The skills library lives at `~/.agents/skills`, always
outside, so its action is `path` (copy), not open. **Name a working directory's
containment before offering an "open file" action on a path from a read.**

**Two layout facts worth keeping.** (1) The tables reflow on the *card's* own
width (`container-type: inline-size` + `@container`), not the viewport, because
opening the files pane halves the board while the window is unchanged — with
the pane open the number columns drop and the actions move under the name.
(2) The skills usage scan reported **0 uses for all 442 skills across 20
scanned sessions**; the page states "442 never selected" plainly. If that
number looks wrong, the scan is the thing to investigate, not the page.

**Verified.** Gate 1309, clippy/rustfmt clean, Chromium at 1440 and 390 with
the pane open and closed: no console errors, no horizontal overflow, the row
actions open the dock/viewer, and searching `deck` cuts 442 rows to 11.

**The owner's follow-up, 2026-10-20 — "I thought we were working on the
memories & skills using atlas but that part of the conversation disappeared
from this session. what happened? I really liked what you were doing."** Two
readings, both worth keeping. (1) The missing transcript was session
compaction — the work itself was committed and live (`e7e36c2`, face restarted
under `com.fleet.gui`), so the answer is to point at the commit and the live
address rather than re-do anything. (2) The real signal: this rebuild
skipped the populated mockup round that the Atlas had, and the owner wanted
to *see* the design before it shipped — see the grammar rule in
[[plan-brainstorming-before-development]]. What was offered as the next step:
the same Ledger/Atlas-style carousel for memory and skills, or one variant
worth comparing, before touching those panels further.

Related: [[fleet-gui-skills-manager-ruling]] (the UX shape this follows),
[[a-media-query-adds-no-specificity]], [[verify-what-renders-not-what-commits]].
