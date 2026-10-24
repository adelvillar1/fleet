# The external-repo rows: seven closed, one opened, a provider born

**2026-10-24** · plan `docs/plans/2026-10-24-external-repo-ki-rows.md`
· status completed, 9 of 9 criteria · fleet gate green by exit code
(**1495 lib + 144 integration**, fmt/clippy/release clean) · CI green
in agnostic-router-kit (run 40107568736).

The burn-down's out-of-scope list said the external-repo rows wanted
their own session; the owner called for it mid-afternoon, then doubled
the scope mid-session with a directive: **include TypeLLM as a sys1
provider** — "check the skill to understand the capabilities" — and
handed over the key in conversation.

## The rows, and what they had become

Every row was a write-time snapshot and the repos had moved underneath
them. The survey before the plan found: gen1's remote **added and
pushed by the owner** (KI-004 closed on evidence alone); sys1's
plan-surface work **landed** — three of the four uncommitted files were
owner commits now, leaving one two-line `provider or []` guard; and
the router-kit failure log matched KI-001's diagnosis verbatim.

| row | end state |
|---|---|
| KI-004 | closed on the owner's own remote; `origin/master..master` empty |
| KI-003 | the guard landed (`d144912`), branch pushed even with origin |
| KI-029 | re-vendored at `f0faebc` — `VENDOR.json` names a commit, not a working tree |
| KI-001 | CI green after four layers (below) |
| KI-005 | sem1's `CLAUDE.md` landed on the sibling pattern (`8bdc26f`, pushed to main) |
| KI-143 | the 566-line adapter filed upstream on `feat/mcp-stdio-adapter` (`92c49ac`, pushed) |
| KI-207 | the owner ruled **update**: driver 0.37.0, doctor all-green, and a live MCP round trip answering 62 tools where every call previously died on the contract mismatch |

KI-311 opened: the corpus derive-dedupe (KI-297's fix) is a fleet-side
carry exactly the way the adapter was — upstream main still re-embeds
on repeated headings. Same filing move, next session.

## TypeLLM, the provider that came from a skill

The skill carried everything the wire needed but the wire itself: the
SDK hid the endpoint, so the wire was learned twice — once from the
installed SDK's runtime (`POST https://api.typellm.ai/v1/generate`,
Bearer, `{context, questions, images, options}`), then live, where the
API taught its own vocabulary: enum fields are `type: "string"`, noul
is `type: "boolean"` (`"bool"` is refused outright), and every head
rides `return_probabilities`. The provider mirrors the ScaleDown shape
end to end — wire, head mapping, parser, Jev-class head-set
inheritance, config, core roster, mock-wire tests — and the proof ran
through sys1's own `classify()`: the change task's three heads answered
with labels and full distributions.

Design choices worth keeping: score heads answer as an enum of
**absolute** level digits (the wire has no ordered-score type, so
nothing shifts — unlike the 0-indexed drex/glide/scaledown wires); the
noul carries P(yes) from the distribution, the scaledown semantic; the
key rode the env chain only, per sys1's own hard rule, and the repo's
zip-strict convention caught the rubric builder at ruff.

## The router-kit onion: four layers to green

KI-001 was one diagnosis wearing four coats, each hidden beneath the
last, and CI could only peel one per run:

1. **Node 24, not 20** — the probes import `.mts` modules.
2. **`npm install`** — `workflow-plane` is a `file: lib/workflow`
   dependency only an install links.
3. **`npm install --prefix router`** — `kit apply` renders the scratch
   runtime by copying `router/node_modules/@typesafe-ai` from the
   checkout, and that directory is gitignored like every
   node_modules; a fresh clone's five scratch-router suites died on it.
   Reproduced locally by removing the directory (33 suites / 5 fail)
   and restored by the install (33 / 0).
4. **The named-absence law crossed platforms** — a binary that exists
   but cannot run is ENOEXEC-as-sync-throw on macOS, EACCES-async
   everywhere, and glibc's execvp `/bin/sh` fallback made the old
   no-shebang fixture untestable on Linux (the garbage file *ran*).
   The fixture became exists-not-executable, EACCES joined the law,
   and the suite went green on the runner (`7fd0aac`, run
   40107568736).

The rebase detour (a pushed commit amended locally, then rebased
rather than force-pushed) is recorded for the process ledger: resolve
forward, never force.

## The re-vendor's hang: a carry the first pass dropped

The sys1 re-vendor's first pass (rsync at the landed sha) deleted more
than it should have. `sys1/perception.py` — the documents session's
parse_document — lives only in the upstream **working tree**, never on
`feature/plan-surface` (KI-003's pattern on a new file), and the old
adapter carried the `sys1_perceive_document` tool beside it. The
perception tests caught both: three failed on the missing module, and
`a_refused_job_names_the_vendors_code` **hung** — the lane booted
without the tool, `parse_via_lane` errored before any vendor
connection, `expect_err` passed vacuously on the early error, and the
test then blocked forever at the served double's `join()`, waiting for
four phantom connections. The mask (a test that should fail, hanging
instead) cost the better part of an hour: orphaned cargo locks, two
colliding test binaries, and a stack sample before the instrumented
double showed zero connections ever arriving. The landed state carries
both files explicitly, named in `VENDOR.json`, with the adapter
degrading to classify-only when the carried module is absent upstream.
rsync's `--delete` is now a named hazard for every future re-vendor.

## Also worth knowing

- **A stale build lock ate ~25 minutes.** An orphaned `cargo test`
  from an interrupted command held fleet's `target/` flock while no
  rustc ran; two background suites sat waiting on nothing. `lsof` named
  the holder, the kill freed the gate. When cargo hangs with no
  compiler alive, look for the orphan before looking at the code.
- The environmental gaps recur across repos: sys1's and blend1's venvs
  both lack their `[service]` extras (fastapi), so their service-test
  modules error at collection — recorded in each recap line, not
  hidden.
- blend1's local `main` carries unpushed owner commits (the checkout
  says so); left untouched.

## The commits

Fleet: `6334a80` (plan), `33a4fa3` (the TypeLLM amendment), `7a3f1d7`
(ledger closes + KI-311). sys1: `d144912` (guard), `f0faebc`
(TypeLLM provider), pushed. agnostic-router-kit: `632c589` →
`735547c` → `58db923` → `a469e4f` → `7fd0aac` (the four layers),
pushed, CI green. sem1: `8bdc26f`, pushed. blend1:
`feat/mcp-stdio-adapter` `92c49ac`, pushed.
