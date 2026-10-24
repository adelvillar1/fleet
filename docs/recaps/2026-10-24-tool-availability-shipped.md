# The missing tool categories: shipped

**2026-10-23 → 24** · plan `docs/plans/2026-10-23-tool-availability-gaps.md`
· status completed, 28 of 28 criteria · gate green at **1488 lib +
144 integration** tests, `fmt`/`clippy`/`--release` clean.

The surface audit found capabilities the harness owned and could not
reach: three vendored lanes with no model-facing path, a background
command with no handle, no code intelligence beyond text search, a
document parser and an SSRF-guarded fetcher that only the GUI could
call, memory that only ran one direction, MCP resources and prompts
never spoken, and the file and search operations that still fell back to
a shell. It also found no lifecycle at all: no record of what a run
declared, so no way to say a tool does not pay for itself.

## What a run can do now that it could not

| | before | now |
|---|---|---|
| images, tables, a browser | three lanes nobody could name | `measure`, `imagine`, `browse` — declared on every run, lazy-joined, zero spawns when unused |
| a background command | a path to an output file | `process output/wait/kill`, with a per-run registry and a survivor named at the end |
| a multi-file change | many `edit` calls | `apply_patch` — content-anchored, atomic per file, a stale hunk refused by name with its nearest line |
| orientation in a repo | read files one at a time | `repo_map` — reference-ranked, token-budgeted, deterministic, and it says it is heuristic |
| a document | `bash` + a parser | `read_document` — local parsers, the parser named, no egress ever |
| a known URL | a search | `web` with `url` — the same guarded fetcher the pane uses |
| handing over a file | a path in prose | `send_file` — one `Artifact` event, a card in the face, a path in the log |
| what the harness knows | pre-emptive assembly | `recall` — ranked facts with score/kind/basis/date plus matching skills, no model call, no hit recorded |
| an MCP server's resources | invisible | `mcp_resource` / `mcp_prompt` — cursors followed, a missing capability named |
| search and files | shell | `grep` with `context`/`type`/`multiline`, `glob` through `rg --files`, `file_change` with the destructive class read from the op |

## The lifecycle

Every run writes one `Declarations` record before its first request, so
the denominator exists: `fleet tools scan` has declared runs, calls,
successes, errors, tokens per call and age; `fleet tools coverage` reads
the owner's own turns for intent the surface missed; `fleet tools sweep`
disables at most three per day, non-destructively, and `fleet lifecycle
promote` is the mirror that suggests what repetition has earned. Tools,
skills, loops and the router's workload chains share one adapter and one
sweep.

## Where the work changed course

The plan was written quickly and the audit found four claims that passed
their unit tests and failed in use. All four are in the close-out, and
two were bugs a user would have hit:

- `fleet tools coverage` **panicked on every machine** — `block_in_place`
  on the current-thread runtime every headless command builds, and the
  wrong wire name for the embed tool. The first run of a new command is
  the only test that counts.
- `file_change`'s move and copy resolved their destination as a *read*,
  so creating a file that did not exist yet failed. The common case.
- `grep` never stripped a prefix through a symlinked folder (macOS
  `/var` → `/private/var`), answering scoped searches with absolute
  temp paths; and with `context`, the neighbourhood was counted as
  matches.
- `--dry-run` was refused as redundant. The safe direction.

Three decisions worth keeping:

**The definition-token bound moved.** The native surface with memory
measures 8,034 tokens across 28 declarations, so it crossed the 8,000
bound and every run would have been tiered behind `tool_search`. The
bound had not been revisited since the surface was nineteen
declarations. Raising it is the same argument the plan makes for the
count bound — and a test now pins the surface against both, so a silent
crossing is a red gate.

**The descriptions lost their repetition rather than their rules.**
`remember`, `subagent`, `improve`, `deck`, `supersede` were rewritten
shorter; two phrases a test pins came back verbatim when the trim
dropped them.

**The tools card reads the same list the run declares from.** It listed
`ALL_TOOLS` alone, so `recall` had no row and a disabled tool had no
enable action anywhere in the face. Both the card and `fleet tools` read
`Agent::base_declarations` now.

## Open, recorded rather than claimed

- `docs/images/settings-tools.png` is one card behind — the data is
  verified live (27 native rows over `/api/settings`) and the image
  wants a capture pass.
- There is no session index, so `recall`'s session channel reports
  `NotWarmed` by name and will until an index exists.
- `repo_map` is a heuristic, not a parser; the LSP stays deferred behind
  `apply_patch` as the plan says.

## The commits

```
5897c86 feat(capabilities): measure, imagine, and browse
0556510 feat(lifecycle): one sweep over tools, skills, loops, and workflows
f8aeb20 feat(tools): a background command gets a handle
a6ae9fa feat(tools): a patch across files, and a map of the repo
3c099f6 feat(tools): documents read locally, URLs read through the guard, files handed over
ba7effb feat(tools): recall what the harness holds, and speak MCP's other two halves
763175a feat(tools): search that says what it can do, and a guarded file_change
82e2e5b fix(tools): the audit's four gaps
345fa3f feat(cli): `fleet lifecycle promote`, the mirror of the sweep
18b75c7 feat(face): the tools card and `fleet tools` read the whole native surface
```
