---
name: fleet-sessions-archive-groups-by-project
description: "2026-10-20 — owner asked mid-turn for sessions grouped by project; shipped f6593d6 (atlas grammar, foldable project groups, open/resume/copy per row). Grouping is recorded-or-inferred from paths, never guessed from embeddings. Router and delegation revamps were NOT done in the same pass"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The request.** "let's do also are revamp of the sessions, router, and
delegation pages", then mid-turn: **"I think sessions should be grouped by
project"**. The grouping note reordered the work; sessions shipped first.

**Shipped, `f6593d6` (pushed, release rebuilt, face restarted via
`launchctl kickstart -k gui/$(id -u)/com.fleet.gui`).** Sessions now uses the
Atlas grammar ([[fleet-second-brain-visual-redesign-awaiting-pick]]): the
`surface-head` instrument + figures strip, a searchable table, foldable
**project group headers** (run count, open count, newest date, an *inferred*
badge), and **open / resume / copy** on every row. Selecting a row still opens
the replay in the 7/5 split.

**Where the project comes from.** `Session::create` records the process's
working directory in the start record; `summarize` fills `workspace_inferred`
by scanning the log's first 256 KB for `/Projects/<name>` when the field is
absent. Live state: 147 logs → **15 projects**, 81 attributed (all inferred —
every existing log predates the field), 66 in "no project". Do not invent a
project for the rest; the honest bucket is the feature.

**Router and delegation: shipped next, `7353073`** (same turn flow — commit,
push, release rebuild, `launchctl kickstart`). Both moved to the shared
`surface-head` + instrument + figures strip, with the existing cards kept
intact in a left column plus a rail — a composition change, not a rewrite:
- **Router**: instrument = pin + judge lane + tier count, actions "ask sys1"
  and "open settings"; strip = calls / ok / errors / success / tokens / judge.
  Left = live decision, declared chain, today's route rows. Rail = today's
  ledger, hourly bars, model mix, workload mix, cache (+ guidance). All
  charts and the hour/model filters still work.
- **Delegation**: instrument = live state + depth cap; strip = depth cap /
  in this run / past runs / graph nodes / refused reads. Left = live-run note
  and past trees; rail = the read's counts and the refusals; the declared
  graph keeps full width below when one exists.

**Live state that made both look quiet (a data fact, not a layout bug).**
Router's ledger for the day read 64 calls / 0 ok / 64 errors, every row
"never attempted" with the reason naming the hard chain; delegation had one
past run, no live run, and no `graph.json`. Worth saying plainly in a recap
so a sparse panel is not mistaken for a broken one.

**The composition lesson.** `.surface-column` (span 8) + `.surface-rail`
(span 4) both collapse to span 12 under 900px; the rail cards must be given
`grid-column: auto` or they keep `span 12` and blow the column apart.

**Gotchas met.** The parked owner-scope `fleet-sessions-archive` helper stores
`workspace` but not `inferred`; the TUI picker auto-anchors the newest session
on load, which is why a replay can appear without a click. Also the
`fleet-sessions-archive` commit touched `session.rs`, `session_recall.rs`,
`tui/app.rs`, `tui/ui/mod.rs` for the new field — a structural change that
every `SessionSummary` literal must follow.

Related: [[embeddings-cannot-label-a-session-project]],
[[fleet-memory-and-skills-knowledge-tables]], [[rebuild-is-not-commit-push-or-ship]].

**Owner verdict 2026-10-23: project-first grouping does not work as the work
surface.** Five reports in one turn — sessions mixes the owner's work with
background work, no way to find what needs attention or is mid-run, opening a
row is a coin toss between the replay and the composer, the conversation has
no exit and no switcher, no way to start a new one, and the transcript is
plain text with wasted space. Diagnosis and the redesign contract at
[[fleet-work-and-the-conversation-revamp]]. The project grouping above is not
wrong, it is subordinated: project becomes the grouping *inside* "Earlier",
and attention comes first.
