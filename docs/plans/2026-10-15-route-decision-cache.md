# The route-decision cache

Status: complete
Date: 2026-10-15

## Why

`select()` judges every route from scratch: it spawns the sys1 MCP
server, initializes it, calls `sys1_classify`, and closes it
(`src/route.rs:1105`). Nothing caches the verdict. The zcode-router-kit
caches exactly this decision and its board reports a 27:1
cached-to-fresh ratio — the same judgment repeats constantly in practice.

The justification is not latency. It is that a recorded judgment is
evidence the credit loop can read: the owner's law is that the log
comes first and the loop closes later, and a cache that serves a prior
verdict while still recording the new row is the first place that
recording becomes reusable.

## What is deliberately NOT cached, and why

- **API responses.** The prompt changes every turn — the memory block
  alone decays by wall-clock and bumps recall counts. A hit would
  require comparing the full assembled prompt, and a miss costs the
  same as no cache. Serving an answer assembled under a different
  memory state would violate recall-as-steering.
- **The fact corpus and the skills tree.** Each is loaded once per
  process in a one-shot CLI shape. A cache would add invalidation
  complexity to save an operation that happens exactly once.
- **The graph score memo already covers its own case** in-process.

## Acceptance criteria

- **C1** `select()` consults the cache before spawning the judge lane;
  a hit skips the spawn entirely.
- **C2** The cache is keyed to the task's hash AND the judging lane's
  identity. A mismatch invalidates the whole cache — a model swap must
  not serve a verdict the new lane never produced. The recall-index's
  discipline, applied to routing state.
- **C3** A hit still writes its `RouteRow`. The `latency_ms` is honest
  (the cache read's own time, not the judgment's), and the row names
  that the verdict was served from the record.
- **C4** A miss writes through once the judgment lands.
- **C5** An absent or malformed cache reads as empty and never errors;
  the write is atomic (unique temp file plus rename).
- **C6** `fleet calib` can report the cached/fresh split, so the hit
  rate is a number the owner can read rather than an assumption.
- **C7** Tests: hit, miss, key-mismatch invalidation, row-on-hit,
  atomicity, corrupt-file tolerance.
- **C8** No new dependency; no runtime consultation of the kit (hard
  rule 6).

## Design

`src/route_cache.rs` — small, beside the recall-index in shape:

- `Verdict { workload: String, confidence: f64, verdict: String }`
- `Cache { key: String, entries: HashMap<String, Entry> }` where the
  key is the judging lane's identity and each entry is a verdict with
  the stamp it was recorded at.
- `read(home) -> Cache`, `keyed_for(&self, key) -> bool`,
  `write(home, key, entries)`, `get(task_hash) -> Option<&Verdict>`,
  `put(task_hash, verdict)`.

Storage: `<FLEET_HOME>/route-cache.json`, beside the roster it
describes. A cache beside the config it derives from, the way
`memories/.recall-index.json` sits beside the corpus.

`route.rs::select` consults it at the `judge()` call site. The row's
marker rides a new `judged_from` field on `calib::RouteRow`
(`fresh` | `cached`), not the reason string — the reason is prose for
humans and the field is what `fleet calib` counts.

## Out of scope

- Cross-machine sharing. The cache is local state, like the roster.
- Shape-normalized keys (mapping free-form text to a workload shape
  before the lookup). The key is the task's own hash; a normalizer is
  itself a judgment and would need its own evidence.
- Eviction by age. The stamp and the key are the invalidation
  discipline; a TTL would expire evidence the credit loop wants.
