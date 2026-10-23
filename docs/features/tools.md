# The tool surface

What the model can do, how a call is resolved, and how a large MCP
catalog stays out of the prompt until it is needed.

Analysis and the full build order: `docs/architecture/tool-surface.md`
and `docs/plans/2026-10-23-tool-surface-and-lane-resolution.md`.

## The eight built-ins

| Tool | What it does | The guarantee |
|---|---|---|
| `bash` | runs a shell command | `timeout_ms` (1s–600s, default 120s); `run_in_background` returns an output file; a timeout promotes the command instead of killing it; output over 64 KiB spills to an artifact |
| `read_file` | reads a text file | line numbers, a `path (N lines)` header, `offset`/`limit` (default 2,000 lines), a continuation line, and named empty/past-end/binary notices |
| `write_file` | creates or replaces a file | atomic temp+rename; the parent must exist |
| `list_dir` | lists a directory | `depth` (1–8), directories marked `/`, capped at 1,000 |
| `edit` | replaces one exact passage | requires a prior `read_file` of the path; a unique match unless `replace_all`; zero matches name the closest line; several name their lines; atomic write and a diff |
| `grep` | searches contents | files/content/count modes, a cap with "narrow the query", a match and file count; `rg` when installed, `grep` otherwise |
| `glob` | finds files by pattern | `*`, `?`, `**`, newest first, capped at 100 |
| `todo` | replaces the run's task list | full-list replacement, `pending`/`in_progress`/`completed` with `todo`/`doing`/`done` aliases |

`edit` is content-anchored rather than line-anchored: line numbers shift
under a one-line change, and an anchor that no longer matches fails
loudly instead of corrupting the file.

## Resolving a call

1. **Exact name.** A declared tool runs as named.
2. **Alias.** Names other harnesses taught a model
   (`str_replace`, `search_replace`, `apply_diff`, `run_command`,
   `view`, `find`, …) map onto the tool here; the progress line says
   the mapping happened.
3. **Recovery.** An unknown name returns the closest declared name
   ("did you mean `bash`?") or the full tool list — never a dead end.
4. **Validation.** Arguments are checked against the declared schema
   before execution: required, type, enum, and integer bounds. A bad
   argument is `error[validation]: field \`timeout_ms\` expects integer;
   got a string`, which the model can correct.

## A large catalog: two tiers

Native tools are always declared. Joined MCP tools are declared too
while the request fits **28 tools and 8,000 definition tokens**. Past
either bound the native set stays (byte-stable, so the prompt cache
survives), the MCP tools are held back, and `tool_search` is declared.

`tool_search(query)` returns the strongest candidates and activates up
to five for the next request. Its selection is a decision plus
geometry, and neither is a language model:

1. **sys1 classifies** the request into a closed capability set —
   `read`, `search`, `edit`, `execute`, `web`, `memory`, `vision`,
   `media`, `delegate`, or `none` — with a confidence. `none` is a real
   answer: the search says no tool is needed and activates nothing.
2. **sem1 ranks** the candidate cards by cosine, with the class text
   folded into the query and a small deterministic keyword boost. Card
   vectors are cached by a hash of the catalog, so one batch embed
   serves every search until the catalog changes.
3. **A floor and a margin decide.** Below the floor the result is "no
   tool fits" plus the nearest two, and nothing is activated. When the
   top two are within the margin both are activated and the model
   chooses by name — a grounded choice between two named candidates.
4. **If sem1 is unavailable**, a lexical ranker runs and the result
   says so. No search fails because a lane is cold, and the resolution
   completes with the model client unreachable.

The sys1 lane gained one task for this: `tool_route` in
`services/sys1/src/sys1/tasks.py`, a single `capability` choice head
over the closed set. It is a vendored-file change; the upstream core
needs the same task to survive a re-vendor.

## The advisor

`fleet tools` lists the eight built-ins. `fleet tools scan --sessions N`
reads the most recent N session logs and counts, per name: calls,
errors, denials, alias recoveries, unknown names, repeated
(name, arguments) calls, `tool_search` results that found nothing, and
the bash commands whose first word is `rg`/`grep`/`find`/`fd`/`bed`/`awk`
— work `grep`, `glob`, or `edit` already covers.

`fleet tools advise` turns those counts into four kinds of suggestion:

- **build** — a name the model called that nothing answers, or a
  `tool_search` that found no tool.
- **fix** — a tool that fails at least a quarter of its calls over at
  least three calls.
- **adopt** — an alias the model reached for, or bash doing search and
  edit work a native tool owns.
- **retire** — enabled servers nobody called, in one row.

The scan and the advice are deterministic: they read files and spend
nothing. On this machine's last 30 sessions the first run found a
`write_file` failing 18 of 22 calls, `remember` failing 10 of 18, 117
bash calls doing grep/glob/sed work, and thirteen enabled servers with
no calls — which is the surface doing its job.

## The ledger and the audit

Every call writes a `ToolResolution` before its result: the name it
asked for, the tool that answered, the method (`exact`, `alias`,
`retrieved`, `refused`), the reason, the catalog size, and the tier.
`fleet tools audit [<session>]` reads one log and prints the
schema-valid share, error classes, p95 result bytes, truncation,
redundant calls, recovery after a failure, and resolutions by method and
tier. The transcript's tool card shows the method; the detail shows the
whole reason.

## The limits

At most 40 calls run in one turn; the rest are refused by name. A result
past 64 KiB spills its full text to `<session>/calls/<id>.out` and names
the path. Unknown arguments are dropped with a note. A fold elides tool
results older than the last six to a byte count, and re-injects the
todo ledger, so the step in progress survives the summary.

## The profile

The route verdict the run already holds reorders the native tools: a
`standard_code` run sees read, edit, grep first; `prose` sees read,
write, glob. A profile reorders and never removes. The resolution's
reason names it.

## Not built yet

Named here rather than implied by the plan: a live second provider for
the dispatch policy; MCP tool annotations; and catalog-backed enums
where a tool takes a model, provider, or workload value. `ALL_TOOLS` is
the native registry (an enum plus a spec and a validator per tool, not a
trait object). An image result is written to a file and named, so
`vision` can read it.

## Files

`src/tools.rs` (specs, dispatch, validation, aliases), `src/fileops.rs`
(window read, atomic write, anchored edit), `src/search.rs` (grep,
glob, glob matching), `src/tool_router.rs` (sys1 class parsing, sem1
ranking, floor and margin, lexical fallback), `src/agent.rs` (tiering,
`tool_search`, the read tracker).
