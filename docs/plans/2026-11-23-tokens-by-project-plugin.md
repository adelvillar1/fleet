---
title: tokens by project — a second view of the accounting the cache instrument already reads
date: 2026-11-23
status: draft
slug: tokens-by-project
---

# Tokens by project

The owner asked (2026-11-23): *"plan a new plugin … token accounting per project,
using the information we already use from the cache plugin, just a different view
— tokens by model per project, a donut type visualization with total tokens and on
hover over all the information reported by the model provider."*

This is a sibling of `plugins/cache`, not a second accounting: one read of the
calibration stream, sliced by the project the run belonged to instead of by model
across everything.

## What the tree already has (and the one thing it does not)

- **The scope**: a session is scoped to one project folder at birth — its Start
  record's `workspace` (`src/session.rs:231`), the fact the rail groups by. So
  attribution needs no heuristic; it needs the accounting row to name its session.
- **The accounting**: every route row carries `provider`, `model`, `latency_ms`
  and `usage: Option<Usage>` (`calib::RouteRow`, `src/calib.rs:64-116`), and
  `Usage` normalizes the providers' own fields — `prompt_tokens`,
  `completion_tokens`, `total_tokens`, `cache_read_tokens` (a *subset* of the
  prompt, never an addition), `cache_creation_tokens`, `reasoning_tokens`
  (`src/api.rs:516-566`). `Usage::billed_tokens()` is the one defined reading
  (`prompt + completion`, `src/api.rs:572-580`).
- **The reader**: `cache_report::read_document` already walks the newest 14
  day-files and groups by (provider, model), with `calls`, `accounted`,
  `reported`, `prompt`, `completion`, `cache_read`, `fresh`, `share`, latency
  splits and a per-day series (`src/cache_report.rs:277-260` region). Its
  honesty rules are the ones this view must inherit: cache-read ⊆ prompt, the
  denominator is the *reporting* rows, and a missing rate is `null`, never zero.
- **The project fact**: a session's Start record carries `workspace`
  (`src/session.rs:231`, written from `current_dir()` at birth), and the archive
  surfaces it as `SessionSummary.workspace` with `workspace_inferred` for old
  logs (`src/session.rs:1385-1389`). It is the same fact the rail groups by
  (`src/gui/bridge.rs:10593`, `10647`).
- **The gap**: a route row carries **no session**. It has `model` + `usage`, but
  nothing that joins it to a project. And `Prepared.session_id` is *not* the
  conversation id — its own doc says it is "opaque and UUID-shaped, derived from
  the preparation's session seed or drawn fresh" (`src/route.rs:3970-3981`). The
  seed itself (the conversation id) is passed to `prepare_with_pin` by the
  callers (`options.session`, `self.options.session_id`) and then dropped.
  Turn rows carry `usage` but neither a session nor a model
  (`src/calib.rs:136-155`); the session log's own `Usage` record carries
  `turn`, `prompt_tokens`, `completion_tokens` only (`src/session.rs:683-695`).

So the plan adds one key at the writer and resolves the project at read time:
**the route row records the conversation it belongs to**, and the project is the
session's own `workspace` — never the current working directory at read time,
never guessed.

That the project is a session fact is the point, not a caveat: a session is born
in one folder and its Start record fixes it, so every row of that session belongs
to that folder by construction, and a delegated child has its own Start (its own
workspace) and therefore its own project. Nothing here infers a project from a
path, a hash or an id: the row carries the id, the session carries the folder.
The only honest buckets are the ones no session can answer — a row written before
rows carried the id, a seedless run, a session whose log is gone, a session born
without a workspace.

## Approach

### C0 — the mockup gate (before code)

Draw the page in the cache instrument's own language (near-black field, mono
numerals, one accent per model), measured in WebKit at 1440, 820 and 390: a
grid of **project cards**, each card carrying the project's donut — its slices
the models that worked across **all** the sessions scoped to that project, the
centre the project's reported total — and, below the donut, the legend of every
model with its number. The gate's open questions live in the mockup's README:

1. **The card, not the meaning.** The donut is per project and its composition
   is fixed: every session the project's runs belong to, aggregated by model —
   a session is never a slice, and two sessions that used the same model are one
   slice. The numbers per model are readable **without hovering** (the legend
   below the donut carries tokens and share); hovering a slice or a legend row
   adds the full provider breakdown. What the gate picks is the card's density
   and order (billed tokens descending) at 1440, 820 and 390 — how many cards
   per row, and whether the buckets are cards of their own or one footnote
   strip.
2. **Slice metric**: `billed_tokens` (prompt + completion), the reading the
   cache instrument already uses — the cache half is a subset of the prompt, so
   cache-read is a *field*, not a slice.
3. **The three honest buckets' words**: `unattributed` (a row written before
   rows named their conversation — including every row for 14 days after this
   ships), `unbound` (the session exists and has no workspace), and
   `unknown session` (the log is no longer in the archive).
4. **The hover panel's fields**: prompt, completion, billed, the provider's own
   `total_tokens` when it differs, cache read, cache write, reasoning — each
   "not reported" where the provider sent none, with calls/accounted beside them.
5. **Name and home**: plugin `tokens`, rail group `system`, label `tokens`.

No code lands before those answers; the mockup is the same gate D3 used.

### C1 — the route row names its conversation

- `calib::RouteRow` gains `#[serde(default)] pub session: String` — the
  conversation id, empty on rows written before the field and on seedless runs
  (cron, onboarding probes, one-shot CLI: those are not projects).
- `Prepared` keeps the seed it was given (`session: Option<String>`) beside the
  derived `session_id`; `record_route` stamps it. The derived HTTP value and its
  affinity behaviour are untouched.
- A seedless run keeps an empty field — the view buckets it, never invents one.

### C2 — the document gains the project dimension

`cache_report` reads the same rows twice into two groupings, from one walk:

- **`models`** stays exactly as it is (the cache plugin's contract is not
  changed).
- **`projects`**: `[{ project, label, inferred, models: [...same ledger fields
  plus cache_creation, reasoning and reported_total...], totals: {...} }]`,
  ordered by billed tokens descending.
- **Attribution**: the reader collects the distinct non-empty `session` ids from
  the window's rows and reads each session log's Start record (the first line —
  `session::session_path` exists; one small helper returns
  `Option<Option<String>>`: absent log / workspace). Empty id → `unattributed`;
  log missing → `unknown session`; workspace `None` → `unbound`. Every bucket is
  a real entry in `projects` with its own sentence, never merged into a project.
- **The provider's own total** is accumulated when reported; a project/model
  whose `total_tokens` differs from `billed_tokens` carries the mismatch flag, so
  a contradicting provider is named rather than silently reconciled (the usage
  plan's own rule).
- The bridge passes the home into the read so the session logs resolve; the
  document carries `window_days` (14, the cache instrument's own) and `read_at`.

### C3 — `plugins/tokens/`

- `manifest.json`: untrusted, version 1.0.0, entry `{group: "system", label:
  "tokens"}`, `queries: ["cache"]`, provenance `creator:fleet repo ·
  plugins/tokens`. No new query name and no sandbox widening: the data arrives
  through the query the cache plugin already declares.
- `index.html`: the cache plugin's static-HTML + `postMessage` pattern (one
  `ask('cache')`, no other channel), the instrument vocabulary shared with its
  sibling, `prefers-reduced-motion` honoured.
- The card: **one per project**, the project's name (the basename, the full
  path in the tooltip), the donut, and the legend below it. The donut's slices
  are composed across every session scoped to that project — the models that
  worked, aggregated, never one ring per session and never one slice per call.
  Slices by billed tokens, one stable colour per model across every project, the
  centre carrying the project's reported total; the caption says how many calls
  reported (`accounted` of `calls`); a model with no usage block draws no slice
  and is named in the caption.
- The legend below the donut is the always-on surface: every model with its
  tokens and share, readable without hovering (and the only path on touch).
  Hovering a slice or a legend row opens the full provider breakdown; the same
  panel opens on keyboard focus of a legend row, so nothing is hover-only.
- The buckets render as their own entries with their sentences, not as
  zero-token projects.
- Empty window → a sentence naming the window; a refused read → the refusal's
  own words and the query name.

## Acceptance criteria

- [ ] C0: the mockup exists under `docs/plans/mockups/2026-11-23-tokens-by-project/`,
  measured at 1440/820/390 with no horizontal overflow, its README records the
  five gate answers, and the owner has answered them.
- [ ] C1: `RouteRow.session` is written from the conversation seed (a test on
  `Prepared::record_route` asserts the id it was prepared with lands on the row);
  a seedless run writes an empty field; rows without the field decode empty.
- [ ] C2: `cache_report` answers `projects` from the same walk; a fixture with two
  sessions in two workspaces groups by project and by model; `unattributed`,
  `unbound` and `unknown session` each appear as their own bucket from their own
  fixture row; the total per project equals the sum of its model rows; a model
  with no usage block carries `accounted` 0 and `prompt` 0 (not a null share);
  the provider's differing `total_tokens` carries the mismatch flag.
- [ ] C3: the existing `models`/`totals` keys are unchanged (the cache plugin's
  document test still passes byte-for-shape), and the `cache` document test adds
  the `projects` assertions.
- [ ] C4: `fleet plugins eval tokens` passes both the static pass (manifest,
  credential sweep, asset sanity) and the declared pass (`cache` answered through
  the bridge's enforcement); an undeclared query still refuses by name.
- [ ] C5: the bundle asks `cache` and nothing else (a grep of the bundle's
  `ask(` calls and the manifest's set agree).
- [ ] C6: a project is one card carrying the donut and, below it, the legend
  of every model's tokens and share — readable without hovering; the donut's
  slices sum to the project's reported total and aggregate every session of the
  project (a fixture of two sessions in one workspace, two models, draws ONE
  donut with both models — a session is never a slice or a ring); a zero-token
  model never draws a slice; the caption names `accounted` of `calls`; the three
  buckets render as sentences, never as projects with 0%.
- [ ] C7: hover and keyboard focus open the same panel; every provider-reported
  field is shown, and an absent one reads "not reported" — never 0.
- [ ] C8: measured in WebKit at 1440/820/390 — no overflow, the cards reflow
  (a grid at 1440 and 820, one column at 390), the donut and its legend stack
  inside each card at ≤640, the legend rows' touch targets ≥44px,
  reduced-motion honoured.
- [ ] C9: the gate is green — fmt, clippy, release build, `cargo test` (all
  binaries), the face harness and the console harness on the pinned Node 26.

## Files to be touched

- `src/calib.rs` — `RouteRow.session` (defaulted).
- `src/route.rs` — `Prepared` keeps the seed; `record_route` stamps the row.
- `src/cache_report.rs` — the project grouping, the buckets, the totals.
- `src/session.rs` — a small `start_workspace(home, id)` helper (first line only).
- `src/gui/bridge.rs` — `cache()` passes the home so the session logs resolve.
- `plugins/tokens/{manifest.json,index.html}` — the bundle.
- `docs/plans/mockups/2026-11-23-tokens-by-project/` — the C0 gate.
- Tests: `src/cache_report.rs` unit fixtures, `src/route.rs` row test, the plugin
  eval suite's declared pass, and any document-shape pin the face harness
  already runs for plugin queries.

## Out of scope

- No new name in `READ_ONLY_QUERIES`, no widening of the plugin sandbox.
- No price or cost estimate — providers do not report a price here, and the
  cache instrument's rule (no capture, no estimate, no price) is inherited.
- No per-day series in the project view (the cache view owns time); no per-call
  drill-down (the query carries aggregates).
- No backfill: rows written before C1 are `unattributed` for their 14-day life,
  and the surface says so rather than predicting where they belonged.
- No change to the derived HTTP `{session}` value or its affinity behaviour.

## Verification

- The C0 mockup measured in WebKit (the house mockup method) and the five answers
  recorded before code.
- Unit fixtures for the grouping and every bucket; the cache plugin's own test
  still green; `fleet plugins eval tokens` on a scratch home;
- The real read on this machine, reported with its numbers: projects found, the
  unattributed bucket's size during the transition, and the read's wall time over
  the 14-day window.
- The full gate, and a single live probe of the installed bundle in the face.

## Linked artifacts

- `plugins/cache/` and `docs/plans/2026-11-15-the-cache-instrument.md` — the
  sibling this view reuses.
- `src/plugins/face.rs` (`READ_ONLY_QUERIES`) and `src/plugins/eval.rs` — the
  bundle's contract and its evaluation.
- `docs/plans/2026-11-23-the-surface-pass.md` — the grammar the plugin's own
  controls follow (a press is explicit; empty is not refused).
