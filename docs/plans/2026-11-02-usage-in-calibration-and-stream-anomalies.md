---
status: draft
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

The mockups — the voice to approve before a line is built:

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

- [ ] **C0 — usage parses.** The `Usage` struct carries the three
  optional fields; an API answer with them records them, one without
  records none, and every existing test, fixture, and mock passes
  unchanged. Pinned by unit tests over both answer shapes.
- [ ] **C1 — the row carries what served.** A `RouteRow` written after
  a served attempt carries the serving attempt's usage; a failed walk
  (no attempt served) writes the row with usage absent; a row from
  before the field reads as absent. Pinned by unit tests over the
  writer and a golden old-row parse.
- [ ] **C2 — the stream says the spend.** `fleet calib`'s default
  surface renders the usage columns when present and the per-provider
  closing card; rows without usage render exactly as before. Pinned by
  a surface test over one new-shape row and one old-shape row.
- [ ] **C3 — detection is deterministic and silent.** Each of the six
  detectors fires on its scripted day and stays silent on a quiet day;
  every threshold is a named `const`; no detector makes a lane ask.
  Pinned by per-class fixture tests.
- [ ] **C4 — the lane narrates, never finds.** On a scripted firing
  day, `--anomalies` makes at most two lane asks (explain + triage),
  and on a dry run (`--dry-run` on the flag, judge's rehearse shape)
  it makes zero. Pinned by the counting session seam from
  2026-10-23-judgment-beats-one-lane (C0's seam, reused).
- [ ] **C5 — the surface is visual and honest.** `fleet calib
  --anomalies` renders findings as cards with class, window, row
  count, evidence, and narration; a quiet day prints its one line.
  Zero findings is a finding. Pinned by a surface test on a firing day
  and a quiet one.
- [ ] **C6 — the docs say it.** `docs/features/calibration.md` names
  the third surface and the six classes; `FUNCTIONAL-SPECIFICATIONS.md`
  carries the flag and the usage columns; the ledger gains a KI row if
  any provider's live answer omits the cache fields (checked, not
  assumed, during verification's live run).

## Files to be touched

`src/api.rs` (the `Usage` fields, the parse), `src/calib.rs`
(the `RouteRow` field, the writer), `src/cli.rs` (`CalibOptions.anomalies`,
the parse), the calib surface rendering,
`src/anomalies.rs` (new), `src/judge.rs` (the session seam reuse, if
the shape differs from the beats'),
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

## Verification

The tests named per criterion; the gate — all four legs, exit codes
read directly, the rustup export first. Increment A adds one live leg:
one real provider call, read back from the stream with its cache
fields present, confirming the shape the fixture assumed — and
answering, per provider, how cache-read and prompt tokens relate (the
`billed_tokens()` semantics). Increment B's live leg: `fleet calib
--anomalies` over the most recent past day with real rows, findings
read off the cards.

## Linked artifacts

- `memories/super-agent-explorations-for-fleet.md` — the article's
  benchmark, the mapping, and the rejected branches.
- `docs/plans/2026-10-23-judgment-beats-one-lane.md` — the lane
  session seam C4 reuses, and the row-vocabulary discipline both
  increments follow.
- `src/calib.rs`'s module header — the stream's advisory-by-construction
  rule, which the anomaly pass inherits: detection never blocks,
  delays, or fails a run.
- `docs/features/calibration.md` — the surface this plan extends.
