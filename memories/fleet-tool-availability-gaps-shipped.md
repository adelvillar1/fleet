---
name: fleet-tool-availability-gaps-shipped
description: "2026-10-23/24 — the tool-availability gaps plan SHIPPED end to end in another session: 8 increments + setter, 11 built-ins, the lifecycle (sweep/promote), and the audit's four corrections; gate 1488 lib + 144 integration, plan completed 28/28"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

SUPERSEDES the "DRAFT… implementation under way in another session" status of
[[fleet-tool-availability-gaps-2026-10-23]] (which still holds the design
rationale: the four amendments, the `Surface` adapter, the archive bootstrap).

**Shipped 2026-10-23 → 24**, commits `5897c86 … 18b75c7` plus recap `de6878d`:
`5897c86` capabilities (measure/imagine/browse), `0556510` lifecycle sweep,
`f8aeb20` process registry, `a6ae9fa` apply_patch + repo_map + bound 32,
`3c099f6` read_document + web `url` + send_file/Artifact, `ba7effb` recall +
mcp_resource/mcp_prompt, `763175a` grep context/type/multiline + glob via rg +
file_change, `82e2e5b` the audit's four gaps, `345fa3f` `fleet lifecycle
promote`, `18b75c7` the whole native surface in the tools card and `fleet
tools`. Plan status `completed`, all criteria checked; gate **1488 lib + 144
integration**, fmt/clippy/`--release` clean.

**The built-ins are now eleven**, not eight: `bash`, `read_file`, `write_file`,
`list_dir`, `edit`, `apply_patch`, `grep`, `glob`, `repo_map`, `file_change`,
`todo` — plus a native side-list (measure, imagine, browse, recall,
mcp_resource, mcp_prompt, subagent, convention, plan, deck, vision, web,
read_document, send_file, computer_use, process) for 27 declarations on an
ordinary run, 28 with memory. Two bounds moved with that: `MAX_VISIBLE_TOOLS`
28 → 32 and (unplanned) `MAX_DEFINITION_TOKENS` 8,000 → **9,000**, because
the surface with memory measures 8,034 and every run would otherwise be
tiered behind `tool_search`. A test now pins the surface against both
bounds, so a silent crossing is a red gate.

**The audit was the interesting part — four claims passed their unit tests
and failed in use** (a live run of each new command is what found them):

- `fleet tools coverage` **panicked on every machine**: `block_in_place` on
  the current-thread runtime every headless command builds (KI-023), and it
  asked the sem1 lane for `embed` when the wire name is `sem1_embed` (the
  roster's prefix-stripping is for the model's eyes, not the wire).
- `file_change`'s move/copy resolved the DESTINATION as a read, so creating
  a file that did not exist yet failed — the common case.
- `grep` never prefix-stripped through a symlinked folder (macOS `/var` →
  `/private/var`), answering scoped searches with absolute temp paths; and
  with `context` it counted the neighbourhood as matches (rg marks a match's
  separator `:`, a context line's `-`).
- `--dry-run` on `fleet tools sweep` was refused as redundant.

**The card and `fleet tools` read one list.** Both listed `ALL_TOOLS` alone,
so `recall` had no row in the face — and no enable action. Both now read
`Agent::base_declarations`, the derivation a run declares from. The live
data check (27 native rows over `/api/settings`, bounds reporting 32/9000)
was done against a second GUI instance on its own port; **the screenshot
`docs/images/settings-tools.png` is one card behind** — a headless Chrome
capture never wrote a file, so the image needs a capture pass. Recorded in
the plan's close-out rather than claimed.

**recall's session channel has nothing to read**: fleet keeps no searchable
session index, and `recall` refuses to embed inside a tool call, so it
reports `NotWarmed` by name ("the session index is not warmed on this
machine, so past sessions were not checked"); a warm-but-empty index is a
different sentence; the probe is the file's presence.

Recap: `docs/recaps/2026-10-24-tool-availability-shipped.md`.

Related: [[fleet-tool-availability-gaps-2026-10-23]] (the design),
[[fleet-tool-surface-analysis-2026-10-23]] (the eight-tool surface this
grew from), [[a-tool-surface-must-measure-and-retire]],
[[fleet-gate-exit-code-trap]].
