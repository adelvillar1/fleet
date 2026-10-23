# The tool surface

What the model can do, how a call is resolved, and how a large MCP
catalog stays out of the prompt until it is needed.

Analysis and the full build order: `docs/architecture/tool-surface.md`
and `docs/plans/2026-10-23-tool-surface-and-lane-resolution.md`.

## The eight built-ins

| Tool | What it does | The guarantee |
|---|---|---|
| `bash` | runs a shell command | `timeout_ms` (1s–600s, default 120s); `run_in_background` returns an output file; over 64 KiB is cut with the head/tail kept |
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

## Not built yet

Named here rather than implied by the plan: the `ToolResolution`
session record and `fleet tools audit`; tool-result elision with spill
artifacts; the todo ledger's re-injection after a compaction; the MCP
protocol stage (annotations, pagination, structured content, image
artifacts, cancellation); and the route-verdict tool profile.

## Files

`src/tools.rs` (specs, dispatch, validation, aliases), `src/fileops.rs`
(window read, atomic write, anchored edit), `src/search.rs` (grep,
glob, glob matching), `src/tool_router.rs` (sys1 class parsing, sem1
ranking, floor and margin, lexical fallback), `src/agent.rs` (tiering,
`tool_search`, the read tracker).
