---
title: the cache instrument — what each model's prompt cache actually saved
date: 2026-11-15
status: completed
---

# The cache instrument

The owner asked (2026-11-15): *"is there a way to calculate the cache
performance for each model used by the router, using the same pattern as
the telemetry plugin approach?"* Answer: yes — the accounting already
exists on the wire and in the ledger; what is missing is the per-model
read and a surface that shows it.

## What the tree already has

- `Usage::cache_read_tokens` (`src/api.rs:523-591`) reads DeepSeek's
  `prompt_cache_hit_tokens`, OpenAI's nested `prompt_tokens_details.
  cached_tokens`, and Anthropic's `cache_read_input_tokens` into one flat
  field, documented as a **subset of `prompt_tokens`**, never an
  addition.
- Every route row in the calibration stream carries `provider`, `model`,
  `usage`, `latency_ms`, `ts` (`calib::ReadRow`), so the per-model rate is
  derivable from rows already written.
- Existing reads: `fleet calib`'s spend card sums cache-read **per
  provider**; `anomalies.rs` computes a first-half/last-half share per
  provider and flags a collapse; the face shows one `cache_read` number
  per run. Nothing groups by **model**, and no surface draws it.

## The shape

1. **`src/cache_report.rs`** — a pure aggregation over route rows:
   per (provider, model) the calls, the accounted prompt volume, the
   cache-read tokens, the share (`cache ÷ the reporting rows' prompt`),
   the cached/fresh latency percentiles, a per-day series, and the
   first-half/last-half shares that name a collapse. Honest about what it
   cannot see: rows without usage, and models whose provider never
   reports cache, are counted as such rather than rendered as zero.
2. **The bridge query `cache`** — `read_days_recent` over the stream's
   newest 14 day-files, answered as one document. Added to
   `READ_ONLY_QUERIES` so an untrusted plugin may declare it; the plugin
   host enforces the declared set exactly as it does for telemetry.
3. **`plugins/cache/`** — a self-contained panel in the telemetry
   plugin's pattern (static HTML, `postMessage` reads, no other channel)
   with its own visual language: a volume river where each model's
   segment is filled to its share, per-model cards with a giant share
   figure, a daily share spark, and a cached-vs-fresh latency dumbbell.
   Motion is one short entrance (numbers count, paths draw), disabled
   under `prefers-reduced-motion`.

## Acceptance criteria

- [x] C1: `cache_report::document` groups route rows by (provider, model)
  with share, sums, per-day series, halves, and latency splits; empty
  input answers empty rather than invented; the share is `null` when a
  model's rows never report cache.
- [x] C2: the split latencies report only when each side has enough rows
  to mean anything (≥ 4), and carry their row counts.
- [x] C3: `plugin_query` answers `cache` and `READ_ONLY_QUERIES` names it;
  an undeclared query still refuses by name.
- [x] C4: `plugins/cache` renders the document — heroes (share, from
  cache, fresh), the river, one card per model (share, calls, spark,
  latency dumbbell), the caveats line — and degrades to "no route rows
  carry cache accounting yet" when the document is empty.
- [x] C5: tests pin the aggregation (grouping, share math, halves,
  latency threshold, empty input) and the module is reachable from the
  gate.
- [x] C6: gate green, the plugin installed into the home's `plugins/` and
  registered, and the live face answers the query with the real numbers
  the stream holds.
