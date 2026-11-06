---
status: completed
created: 2026-11-02
updated: 2026-11-02
slug: usage-in-calibration-and-stream-anomalies
---

# Usage in the calibration stream, and the stream's own anomaly pass

## Context

The offloop article on dsh-superagent's design
(https://offloop.org/learn/deepseek-super-agent-design) prices every
architectural claim in cache-read tokens; this plan is the two
explorations that survive contact with fleet's code. The mapping and
the rejections (event triggers, persistent workers, multi-repo scope)
are recorded in `memories/super-agent-explorations-for-fleet.md`.

- **Fleet cannot measure what the article measures, or what the owner
  rules on.** The `Usage` struct (`src/api.rs:491-500`) carries
  `prompt_tokens`/`completion_tokens`/`total_tokens` only; providers
  return `cache_read_input_tokens` and `cache_creation_input_tokens`
  beside them, and serde's ignore-unknown-fields default drops both
  silently — the live-format fixture proves they arrive
  (`src/api.rs:1165`). The calibration stream's `RouteRow`
  (`src/calib.rs:64-97`) records provider, how, verdict, attempts,
  latency, input_chars — and no token field of any kind. Spend is the
  owner's binding constraint (the quota rulings are protected-tier
  facts), and the one stream whose job is recording what actually
  happened cannot answer "am I spending the other plans first".
- **Nothing watches the stream for trouble.** There is no
  `src/anomalies.rs`; grep finds no run-level anomaly detection — every
  stall/heartbeat hit is memory-trajectory vocabulary. The classes the
  stream already makes visible: consecutive 429 fallbacks walking the
  whole roster, a repeated identical tool failure, both local lanes
  down (lane + embeddings — today each degrades silently *by design*,
  but both down at once has no name), a delegation that always eats its
  max-turns, a run whose only end is `max-turns`. KI-066's recurrence
  (a smoke that must assert its mock before `--yes`) is exactly the
  class this catches after the fact.

The article's own benchmark is why the shape matters: the
delegation-first prompt closed ~90% of the latency gap at under half
the token cost of a router-plus-workers architecture. That argument is
arithmetic over token columns fleet does not record. These two
increments give fleet the columns, then give the stream a voice about
what they show.

A third exploration joins them (2026-11-02, recorded in
`memories/harness-compositional-generalization-paper-vs-fleet.md`): the
Zhang/Khattab harness paper names **context rot** — appended tool
outputs bloating history until every call is out of distribution — as
the failure mode of ReAct-shaped harnesses, and its Figure 8
instrumented the **trajectory shape** to see it. Fleet's answer to rot
already exists (delegation with fresh contexts, windowed reads, capped
outputs, compaction), but whether rot actually bites fleet's own
traffic is unmeasurable today: the stream has no per-run token curve.
Increment C is the paper's one adoptable idea, taken as instrumentation
per KI-249's law — measure before extending — and nothing in it
changes when or whether compaction fires.

## Approach

**Increment A — usage rides the rows.** Three optional,
serde-defaulted fields on the `Usage` struct: `cache_read_tokens`,
`cache_creation_tokens`, `reasoning_tokens`. Absent fields read as
zero and serialize as omitted, so every existing fixture, mock, and
recorded row parses unchanged. One named method, `billed_tokens()`,
defined once with its own semantics documented at the definition —
whether cache-read tokens double-count against `prompt_tokens` is a
provider-by-provider question the live check in Verification answers,
and the method is where that answer lives, so no call site re-derives
it.

`RouteRow` gains one `Option<Usage>` field, serde-defaulted: the usage
of the attempt that served, absent on attempts that failed and on rows
written before the field existed — the `judged_from` pattern exactly.
Rows that served from cache-as-prompt still carry it; nothing pretends
a cached turn was free. `Attempt` stays untouched: per-attempt token
counts are not returned by the providers' error paths and inventing a
zero would be a lie in the vocabulary.

`fleet calib`'s stream surface names the new columns when rows carry
them — provider, model, how, latency, prompt/completion/cache-read,
and a small per-provider spend summary as the surface's closing card.
Rows without the fields print as they always did. The user-facing
lines land as the increment lands: `docs/features/calibration.md`
gains the usage columns, `FUNCTIONAL-SPECIFICATIONS.md` the flag and
columns — C6's work is Increment A's, not a pass afterwards.

**Increment B — the stream judged.** `src/anomalies.rs`: a
deterministic pre-pass over a day's calibration rows that names what
the day did unusually, in the row vocabulary, with the evidence and no
spend. Six first classes, each with its threshold as a `const` beside
its detector:

| Class | Reads | Fire condition |
|---|---|---|
| `fallback-walk` | `RouteRow` | ≥3 consecutive rows whose `providers_used` walks the same direction, ending in the same fallback |
| `tool-failure-loop` | `ToolScreenRow` × session log | ≥3 screenings of one tool with the same `input_hash` in one run |
| `lanes-down` | tool screenings + the embeddings probe record | both lane calls and the embeddings path failing in the same window |
| `delegation-maxed` | delegation rows | ≥2 delegations in one day ending `max-turns` |
| `run-failure-cluster` | `TurnRow` | ≥3 `outcome: "failed"` in 15 minutes |
| `cache-collapse` | `RouteRow`'s usage field (Increment A's) | a provider's cache-read share dropping to zero across a day that previously showed hits |

Only two classes ask the sys1 lane, and only when a deterministic
class has already fired: `explain` for the fired cluster's narration,
`triage` for a severity. Anomaly detection stays deterministic; the
lane adds prose to findings, never finds alone. Detection is silent
when the day is quiet — the pass finding nothing *is* a finding, and
it prints one line saying so.

**The surface.** `fleet calib --anomalies [--day <YYYY-MM-DD>]`: the
day's findings as cards at the "genuinely visual" bar — class, the
rows it keys on (timestamps + hashes, never task text), the evidence
line, and when a lane was asked, its one-line narration. A third
`CalibOptions` flag in the existing flag grammar; the three surfaces
read different rows and do not combine. Both increments carry their
own user-facing lines the way every slice ships its surface: the
feature doc gains the usage columns with Increment A and the anomaly
surface with Increment B, and the functional spec gains the flag
beside them — C6 pins what these two approaches built, not a docs
pass afterwards.

**Increment C — the run's shape rides the stream.** The paper's one
adoptable idea, taken as measurement and nothing else. The agent
already computes the growth curve every turn and throws it away:
`last_prompt_tokens` (`src/agent.rs:2630`) is the turn's context size —
the provider's own count when it reports one, the estimate otherwise,
the exact reading `maybe_compact` uses — and it survives only until the
next turn overwrites it. Increment C keeps it.

The `RunReport` gains the per-turn curve: one snapshot per turn of
(prompt tokens, completion tokens), captured where the usage event
already fires, plus the compaction count the run experienced. A new
`ShapeRow` in the calibration vocabulary — serde-defaulted, the
`judged_from` pattern — is written once per run beside the route row,
so a run's trajectory becomes a row and the stream's oldest discipline
holds: rows written before the field read as absent, and nothing
pretends a curve existed where none was recorded.

`fleet calib --shape [--day <YYYY-MM-DD>]`: the day's runs as curve
cards at the genuinely-visual bar — the run's per-turn prompt sizes as
a one-line sparkline-like sequence, its compactions and outcome named —
and a day-level closing card: the median per-turn growth ratio, the
runs whose context doubled mid-run, the compaction counts, the runs
that ended `max-turns`. A day with no runs prints its one line. The
reader speaks in numbers and names; it makes no sys1 ask and fires no
anomaly — Increment B's classes stay the voice, and if the shapes ever
justify one (a run whose context doubled N times), that class is a
follow-up ruled on by the data this increment collects.

The mockup — the voice to approve before a line is built:

```
a shape card
  14:02 · hard · stepfun/step-5-preview · 6 turns · compactions 1 · ended stop
  4.1k → 7.8k → 12.4k → 18.9k → 11.2k → 13.0k
the day's closing card
  14 runs · median per-turn growth 1.4× · 2 runs doubled mid-run ·
  3 compactions · 1 run ended max-turns
```

```
the usage columns, on a stream row
  stepfun/step-5-preview · judged · hard · 1.2s · 4,102 + 312 · cache-read 3,655
```

```
an anomaly card
  FALLBACK-WALK · 14:02–14:19 · 4 rows
  the walk: liquid → deepseek → token-plan → stepfun, every row
  ending `fallback`; the same `input_hash` in all four.
  severity: high (the roster exhausted on one task shape)
```

## Acceptance criteria

- [x] **C0 — usage parses.** The `Usage` struct carries the three *`usage_reads_the_providers_cache_fields_and_their_absence` — the wire names record, the absence records nothing (not zeros), and the absence serializes as omitted.*
  optional fields; an API answer with them records them, one without
  records none, and every existing test, fixture, and mock passes
  unchanged. Pinned by unit tests over both answer shapes.
- [x] **C1 — the row carries what served.** A `RouteRow` written after *`the_decision_row_carries_the_serving_usage` — a served walk's row carries the usage; a walk that never served writes it absent; the reader's defaults keep old rows parsing.*
  a served attempt carries the serving attempt's usage; a failed walk
  (no attempt served) writes the row with usage absent; a row from
  before the field reads as absent. Pinned by unit tests over the
  writer and a golden old-row parse.
- [x] **C2 — the stream says the spend.** `fleet calib`'s default *`the_surface_names_the_usage_columns_only_when_they_ride` — the tokens line and the spend card render for the new-shape row and stay absent for the old-shape day.*
  surface renders the usage columns when present and the per-provider
  closing card; rows without usage render exactly as before. Pinned by
  a surface test over one new-shape row and one old-shape row.
- [x] **C3 — detection is deterministic and silent.** Each of the six *Six per-class pins (fallback-walk, tool-failure-loop, lanes-down, delegation-maxed, run-failure-cluster, cache-collapse) — each fires on its scripted day, and `a_quiet_day_finds_nothing` pins the silence; every threshold a named const.*
  detectors fires on its scripted day and stays silent on a quiet day;
  every threshold is a named `const`; no detector makes a lane ask.
  Pinned by per-class fixture tests.
- [x] **C4 — the lane narrates, never finds.** On a scripted firing *`the_pass_asks_at_most_twice_and_never_on_a_dry_run` — the counting seam: two asks on a firing day (triage + explain), zero on a dry run, the severity and narration riding the answers.*
  day, `--anomalies` makes at most two lane asks (explain + triage),
  and on a dry run (`--dry-run` on the flag, judge's rehearse shape)
  it makes zero. Pinned by the counting session seam from
  2026-10-23-judgment-beats-one-lane (C0's seam, reused).
- [x] **C5 — the surface is visual and honest.** `fleet calib *`the_surface_renders_a_quiet_day_and_a_finding` — the finding card (class, window, rows, evidence, severity, narration) and the quiet day's one line.*
  --anomalies` renders findings as cards with class, window, row
  count, evidence, and narration; a quiet day prints its one line.
  Zero findings is a finding. Pinned by a surface test on a firing day
  and a quiet one.
- [x] **C6 — the docs say it.** `docs/features/calibration.md` names *calibration.md's three sections, the FS flag rows, the tech doc's evidence spine. The live leg — one real provider call reading the cache fields back — is the plan's own verification step and stays the named pending half; no live run this session means the conditional ledger row (a provider omitting the fields) is untriggered, not assumed.*
  the third surface and the six classes; `FUNCTIONAL-SPECIFICATIONS.md`
  carries the flag and the usage columns; the ledger gains a KI row if
  any provider's live answer omits the cache fields (checked, not
  assumed, during verification's live run).
- [x] **C7 — the curve is captured, not re-derived.** A run's *`the_run_report_carries_the_per_turn_curve` — one snapshot per turn on a scripted run, the estimate carrying when the provider reports nothing (the maybe_compact reading).*
  `RunReport` carries one prompt/completion snapshot per turn — the
  reported prompt tokens when the provider reports, the estimate
  otherwise (the `maybe_compact` rule, not a second one) — and the
  compaction count. Pinned by an agent-loop test over a scripted run.
- [x] **C8 — the shape rides the vocabulary.** A `ShapeRow` written *`the_shape_row_round_trips_and_old_rows_read_as_absent` — the shape row's round trip, and an old route row reading its fields as absent.*
  once per run beside the route row carries the curve; a row from
  before the kind reads as absent; the writer emits nothing for a
  zero-turn run. Pinned by writer tests and a golden old-stream parse.
- [x] **C9 — the surface answers the rot question.** `fleet calib *`the_shape_surface_answers_the_rot_question` — the curve cards and the closing aggregate (median growth, doubled runs, folds, capped endings); an empty day prints its one line.*
  --shape` renders the day's curve cards and the day-level closing
  card (median growth, doubled runs, compactions, max-turn endings);
  a day with no runs prints its one line; no lane ask, no anomaly
  fired. Pinned by surface tests over a scripted day and an empty one.

## Files to be touched

`src/api.rs` (the `Usage` fields, the parse), `src/calib.rs`
(the `RouteRow` field, the writer, the `ShapeRow` kind),
`src/agent.rs` (the per-turn curve on the `RunReport`),
`src/cli.rs` (`CalibOptions.anomalies` and `CalibOptions.shape`,
the parse), the calib surface rendering,
`src/anomalies.rs` (new — the day's deterministic pass; the lane-ask
seam is reused from the judgment beats' implementation per
`2026-10-23-judgment-beats-one-lane.md`, wherever it lives),
`docs/features/calibration.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
`docs/known-issues.md` (the live provider-shape check's finding, if
any).

## Out of scope

Any runtime watchdog — a daemon, a monitor thread, a cron that acts:
this plan is the analysis half; the acting half is a delivery-seam-style
recorded-and-unbuilt ruling for the owner. Automatic KI filing from an
anomaly class (the finding surfaces; the owner or the session files
it). Per-attempt token accounting. Any change to routing, the walk, or
the preference table this evidence might eventually justify — the
evidence lands first. The sys1 batch class (one call, N rows), which
stays the named follow-up from the beats plan.

And for Increment C specifically: any change to when or whether
compaction fires (the curve measures; `maybe_compact`'s budget is
untouched); cross-run trajectory-similarity metrics (the paper's
Levenshtein/Jaccard apparatus is training-eval tooling); an
anomaly class built on the shape before the shapes exist to rule on
(a doubled-context class is the named follow-up, written only if the
day cards earn it); per-delegation curves inside the run's row (the
delegation rows already say what the children did).

## Verification

The tests named per criterion; the gate — all four legs, exit codes
read directly, the rustup export first. Increment A adds one live leg:
one real provider call, read back from the stream with its cache
fields present, confirming the shape the fixture assumed — and
answering, per provider, how cache-read and prompt tokens relate (the
`billed_tokens()` semantics). Increment B's live leg: `fleet calib
--anomalies` over the most recent past day with real rows, findings
read off the cards. Increment C's live leg: one real run on the
scratch home writes its `ShapeRow`, and `fleet calib --shape` reads
the curve back off the card — the paper's rot question asked against
fleet's own traffic for the first time.

## Linked artifacts

- `memories/super-agent-explorations-for-fleet.md` — the article's
  benchmark, the mapping, and the rejected branches.
- `memories/harness-compositional-generalization-paper-vs-fleet.md` —
  the harness paper's mapping; Increment C is its one adoptable idea.
- `docs/plans/2026-10-23-judgment-beats-one-lane.md` — the lane
  session seam C4 reuses, and the row-vocabulary discipline both
  increments follow.
- `src/calib.rs`'s module header — the stream's advisory-by-construction
  rule, which the anomaly pass inherits: detection never blocks,
  delays, or fails a run.
- `docs/features/calibration.md` — the surface this plan extends.
