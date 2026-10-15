# The calibration loop: `fleet judge` / `fleet calib --calibration` (slice 18)

> The router picks a provider for every turn and records why; the
> calibration stream has carried those rows since slice 5. What has
> never closed is the loop: nothing graded whether the pick was
> **right**, so the stream recorded a history nobody read back. This
> slice closes it. The day's route rows are graded against their own
> outcome evidence through the vendored sys1 lane, the judgments land
> as rows beside the routes they grade, the calibration surface shows
> what the judgments say, and the failover's preference consults it —
> conservatively, per shape, behind a fit requirement, and never
> touching the declared chain.

## The shape rule (the owner's directive, 2026-10-14)

**Problem shape drives, words corroborate.** The owner's words: *"a
keyword or phrase in the wrong context is meaningless, while a problem
shape tied to the word phrases that were recorded during the resolution
are key information."*

Applied here, concretely: **the calibration is keyed by
(provider × workload shape), never by provider alone.** A provider that
is right for `hard` and wrong for `prose` must not be averaged into one
wrong-rate — that average is exactly the wrong-context failure the
owner names. Every computation in this slice is per **cell**:

- the confidence-weighted wrong-rate,
- the cost per correct turn,
- the fit requirement (rows + confidence span),
- the ride-back, which consults **the cell for the current workload's
  shape only**.

A provider judged wrong for shape X is skipped in the failover only
when the current task is shape X. Its shape-Y record never moves a
shape-Y walk. The route row's reason names the cell that caused the
skip — the provider, the shape, and the cell's own numbers — not an
average. On the surface the cells are cards, and a single averaged
per-provider score is never rendered.

## The loop, end to end

```
route rows land (op=route)   ──┐
                               │  `fleet judge [--day D] [--dry-run]`
                               ▼
                    one bounded ask per route row
                    to the sys1 lane (sys1_classify),
                    evidence only, never the task text
                               │
                               ▼
judged rows land (op=route_judged), keyed by sha256(route row)[:16]
                               │
              ┌────────────────┼─────────────────────┐
              ▼                ▼                     ▼
   `fleet calib          `fleet judge           the failover's
   --calibration`        --agree/--disagree/    preference consults
   per-cell cards        --waive <key>          the shape's own cell
```

**The judgment.** `fleet judge` reads a day's `route` rows and makes
one bounded ask per row to the vendored sys1 lane — the same lane the
router selects with (`judge::JUDGE_LANE`, `"sys1"`) over the MCP client.
Each ask carries the row's own redacted evidence and nothing else: the
workload shape, the verdict and its reason, the model, the
`providers_used` walk, the attempts and their outcomes, the input's
character count, the task's content hash, and the joined `turn` row's
outcome. **The task text never enters the prompt** — the route rows are
redacted by design, and the prompt says so. The prompt poses the
three-way question (below) and asks *was this selection right **for the
workload shape it was made for***, not *was this provider right in
general*.

**The judged row.** One `route_judged` row per graded route, keyed by
`sha256(<canonical route-row line>)[:16]` — the devdec join shape. The
row carries the verdict, its confidence (the noul), the reason, the
provider, **the workload shape**, the lane, the cost and the latency.
Additive by construction: every new field is `skip_serializing_if =
"Option::is_none"` and every reader field is `#[serde(default)]`, so a
reader from before this slice reads a judged row without the new
fields, and a reader from after reads an older stream.

**The human seam.** `fleet judge --agree|--disagree|--waive <key>`
records the owner's disposition on a judged row. The disposition rides
the **same append-only row** under the **same key**: the reader takes
the freshest write per key, so the owner's grading is ground truth
without rewriting what the lane said. The judge is a signal, not an
authority. In the math, `disagreed` inverts the verdict — an overridden
wrong becomes a right, and an overridden **insufficient** becomes a
right too, because the owner's disagree on an incomplete solution is
exactly the ground truth that says *the companion was there* — and
`waived` sets the row aside entirely: neither right nor wrong,
excluded from the rate.

A **second** disposition supersedes the first by the same rule — a
later `--agree` after a `--disagree` lands a later row for the key, and
the math follows the later one. There is no idempotence guard, by
design and by limitation: the stream is append-only, so recording the
same disposition twice lands two rows and keeps both, and the reader's
freshest-row rule is what makes the last one stand.

## The two kinds of negative vector (the owner's third vector)

The owner's framing (2026-10-14): a problem is a minima in a
multidimensional space, a solution is a vector driving toward it, and a
value that is a minimum for one dimension can be meaningless in the
whole. The corollary for grading: **there are two kinds of negative
vector, and they are not the same information.**

1. **A failure** — a trajectory that drove AWAY from the minima. In
   routing: a different candidate served while the required one was
   passed over or failed in a way that was its own. This is the
   anti-vector, and it is what the failover drives away from.
2. **An incomplete solution** — a vector whose DIRECTION was right but
   which never reached the minima because the companions it needed were
   absent. It did not diverge; it drove partway and stalled. Fleet's
   own history is full of them: the install's venv bootstrap was
   correct but unaccompanied by the torch stack; the launch spec's
   `BLEND1_PYTHONPATH` was right but the sibling path-wiring was
   missing; the adapter was correct but unaccompanied by the fail-open
   status parse.

So the judgment's verdict vocabulary is **three-way**, and the two
negative states are never folded together:

| Verdict | Meaning | The failover's read |
|---|---|---|
| `right` | The required candidate served, no failover preceded it, no escalation leg was called for, and the run completed where joinable. | nothing |
| `wrong` | A different candidate served while the required one was passed over or failed in a way that was its own — an anti-vector. | **skips** the provider in the preference |
| `insufficient` | The direction was right but the minima was not reached for want of a companion. | skips nothing; **names** the companions the minima needs |

The three insufficient signals, each readable from the row's own
outcome evidence:

- **a failover followed the served turn** — the required candidate was
  attempted and failed on an *availability* class (a key, a balance, a
  permission, throttling, an outage, a timeout, an unreachable host)
  and a later candidate served in its place;
- **the turn's answer needed the escalation leg** — the row's own sys1
  verdict was `escalate` and the record carries no evidence the leg ran;
- **the attempt succeeded but the task did not complete** — the joined
  `turn` row's stop reason is a non-completion that NAMES an absent
  companion: `max-turns` ran out on a longer turn budget, `failed` lost
  whatever the failed step needed to finish. Each reason names the
  companion it was missing, because "insufficient" without the absent
  companion is not actionable.

**`interrupted` does not grade insufficient.** In this crate it is the
word for a session log that never recorded an ending — an I/O failure,
or a run stopped from outside (`calib::TurnStop`). That is *absent*
outcome evidence rather than a named absence, and the third vector is
defined by the companion that was absent, so an interrupted run grades
on the selection evidence alone: the required candidate served, no
failover preceded it, and the selection is right. The ruling matters
because the insufficient-rate drives the companion NOTE — reading an
owner's own stop (or a wedged disk) as a missing companion would
amber-stain cells and trigger notes about companions that were never
absent, which is exactly the noise the note's floor exists to prevent.

A wrong direction outranks an incomplete one: when both readings are
available, the anti-vector is the actionable signal and the grade is
`wrong`. The judge's prompt asks the three-way question and defines all
three states, so the lane's workload judgment is made in the same frame
the beat grades in.

## The discipline (ported from slice 17)

| Bound | Behaviour |
|---|---|
| **Batch-only** | No turn's synchronous path names the judgment. `judge` runs from the CLI's dispatch in `main.rs`; `route.rs` never calls it — it reads the *calibration*, which is a pure read of the stream. The proof is a source grep over a fixed module list (`no_synchronous_path_references_the_judgment_beat`), not a walk of the call graph: it asserts no synchronous path names the beat, and names the two in-crate callers that legitimately do — `main.rs` (the CLI dispatch) and `eval.rs` (the graded scenario), beside `judge`'s own tests. |
| **Dry run** | `--dry-run` makes no ask and shows exactly what would be asked — the route keys, the shapes, the providers, and the composed evidence. |
| **Budget** | `JUDGE_BUDGET_USD_DAY` = $0.05/day. `FLEET_JUDGE_BUDGET_USD_DAY` can only **lower** it, never raise it. Each ask reserves `JUDGE_ESTIMATED_ASK_USD` before it is made; the cap holds to within one ask's real cost, because the lane's router picks the path — and therefore the cost — only once the ask has been made. |
| **Fail-open refusal** | A lane that fails open answers with its own fallback prose. The beat parses the judgment's status fields and treats a non-ok judgment as **no judgment** — a named refusal carrying the lane's own error, no row, no influence, the recorded decision standing. An answer that reports a workload but no confidence is refused the same way: the confidence is the weight the cell math reads, and a guessed weight fabricates the number the weighted rates, the span, and the fit requirement all rest on. |
| **Spend record** | The spend ledger lands **even when the row write fails** (slice 17's sidecar pattern), so the budget cannot be evaded by a failing disk. |

The real-binary check runs against the lane's **unconfigured** state:
the judgment refuses by name with exit 1 and lands no row.

## The fit rule

A cell's floor is trusted only when its judged sample carries enough
rows **and** enough confidence span. The thresholds are named constants,
sized for fleet's volume rather than inherited from the sibling:

| Constant | Value | Why this number |
|---|---|---|
| `calib::FIT_MIN_ROWS` | **8** | dev-decisions grades many heads across many tasks a day and asks for 20. Fleet is one owner's tool: a day's route rows are the runs they did — single digits on a quiet day, low double digits on a heavy one. Twenty rows would take a week and the calibration would never inform anything. Eight is reachable within a day or two. What eight buys, stated exactly: read **unweighted**, the floor cannot be crossed by fewer than **four** wrong judgments — three of eight is 0.375, below the 0.50 floor, and four is 0.50, the first crossing — so a bare small-sample swing never moves routing on the unweighted reading. In the **confidence-weighted** regime three IS enough: three wrongs at 0.95 against five rights at 0.55 weight to 2.85 / 5.60 = 0.509, over the floor, on a sample whose 0.40 span is fit. That is deliberate rather than a loophole — the weighting is the point, and `FIT_MIN_SPAN` is what keeps a flat high-confidence sample from degenerating into an unweighted rate. |
| `calib::FIT_MIN_SPAN` | **0.30** | The wrong-rate is weighted by the lane's own confidence, so a sample whose confidences are all the same number carries no information about how confidence relates to correctness — the weighting degenerates and the "confidence-weighted" claim is vacuous. The sibling asks for 0.4 over its volume; fleet's rows per day are fewer and a decision model answering over one compact evidence record clusters high, so 0.4 would rarely be reachable at eight rows and would refuse samples that are genuinely informative. 0.30 is the span at which the weighting does real work at fleet's sample sizes. |
| `calib::WRONG_RATE_FLOOR` | **0.50** | The sibling's calibration is a report and carries no such floor; fleet's ride-back needs one. 0.50 is the most conservative crossing that still means something: weighted by the judge's own confidence, the provider is wrong at least as often as it is right. A lower floor would move routing on a provider that is more right than wrong, which is not what "conservative" means. |
| `calib::INSUFFICIENT_RATE_FLOOR` | **0.50** | The trigger for the companion NOTE, never for a skip: a fit cell whose confidence-weighted insufficient-rate is at or over this names that the minima needs companions. It is the same conservative crossing read against the other negative vector — at least half the cell's confidence weight sat on selections whose direction was right but whose minima was not reached. Below it the cell is mostly reaching its minima and the note would be noise. |

A cell's math keeps the three states apart: the cell carries the
confidence-weighted **wrong**-rate and the confidence-weighted
**insufficient**-rate beside the three counts (right, wrong,
insufficient), and never folds them. There is no third weighted rate —
the right count is a count, not a rate. An insufficient row is not a
wrong one — its direction was correct — so folding the two would skip a
provider whose selections were right, and folding it into right would
hide the companions the minima needs.

A graded row that reported **no confidence at all** is counted apart
(`zero_confidence`) and named on the card: it weighs nothing in the
weighted rates, so the rates are the confident rows' alone — and when
every graded row is one, the weighting has nothing to weight by and the
rates are the plain fractions. The beat refuses a lane answer that
reports a workload with no confidence at all rather than grading it on
a guessed weight (above), so the count reads rows whose confidence the
lane itself reported as zero.

An unfit cell renders as **unfit with what it still needs** — how many
more rows, how much more span — and never as a score. The surface's own
words: *no score is drawn from this sample*.

## The ride-back, and its exact scope

**What moves.** The failover's **preference among the candidates that
follow the walk's start**. When the router walks its candidates after
sys1's selection fails, a candidate whose cell for the current
workload's shape is fit and whose confidence-weighted wrong-rate is at
or over `WRONG_RATE_FLOOR` is passed over — but only when a later
candidate in the declared chain is healthy, so the walk always has
something to try.

**What does not move.**

- **The declared chain's order.** The roster's order is the owner's
  standing directive and is untouched.
- **The head of the walk.** The candidate sys1 selected starts the
  walk and is never skipped.
- **Another shape's record.** A cell for shape X never moves a walk for
  shape Y.
- **An unfit cell.** However wrong its raw rate looks, an unfit cell
  moves nothing.
- **The last healthy candidate.** A skip that would leave the walk with
  nothing to try is not a skip.

Every skip names its calibration in the route row's reason:

```
; calibration skipped zai-coding-plan for shape `hard` in the failover
preference (judged wrong at 0.94 on a fit cell of 8 rows, span 0.35)
```

**The insufficient cell's read.** A cell whose selections keep stalling
short of the minima for want of a companion skips **nothing** — the
direction was right, so the provider is passed over by no one. What it
earns is a clause in the decision row's reason naming the companions,
so a later reader sees why the walk behaved as it did:

```
; calibration notes xiaomi-mimo for shape `hard`: 1.00 insufficient on a
fit cell of 8 rows (span 0.35) — the direction is right, the minima
needs companions, and the provider is not skipped
```

The two clauses are deliberately different words, and the verb is the
whole difference: a `skipped` clause moved a candidate out of the
walk's preference, a `notes` clause moved nothing. A later reader of a
decision row sees which of the two happened from the clause's own
first word, without having to re-derive it.

**The readers' bounds, named.** Both readers that can move money or
routing — the beat's day-spend read and the ride-back's cell lookup —
read the last `calib::READ_CAP` (512) rows of the day. On a day heavier
than 512 rows a cell's earlier judged rows fall out of the tail, the
cell reads as unfit, and the ride-back silently stops informing: the
KI-167 class, inherited by a second reader rather than introduced by
this one. The home-side spend ledger is unbounded, so an ask whose row
never landed is counted whatever the day's weight.

**The gate.** `calibration.informs_failover` in `~/.fleet/config.json`
(default **on**; absent = on). Off restores the pre-slice behaviour
exactly — no skip, no clause — which is pinned by a test, not asserted
by argument.

## The surface

```
fleet calib --calibration
```

Per-cell cards, never a plain-text table and never an averaged
per-provider score: the judged count, the right/wrong/**insufficient**
split (with the owner's overrides and waivers counted separately), the
confidence-weighted wrong-rate, the confidence-weighted
insufficient-rate, the cost per correct turn, and the fit state. The
gate's state, the floor, the fit thresholds, and the ride-back's scope
are named on the surface itself. `fleet calib` (today's rows by op) now
also reports the day's cells in its detail line.

A fit cell carrying insufficient rows renders its own rate line and,
when the rate is at or over `INSUFFICIENT_RATE_FLOOR`, the companion
note — *the minima needs companions … the provider is not skipped*. The
three states are three named numbers on the card, never a binary.

`fleet judge` renders the batch's card (the asked, the refused, the
right/wrong/insufficient tally, the spend against the budget), the
dry-run's card (what would be asked and what it would cost), the
fail-open refusal card, and the disposition card. Every row line on
every card reads the one verdict mapping the vocabulary carries —
wrong in red, **insufficient in amber**, right in green — so a card
never prints a green `right` above its own `8 insufficient` tally.

## Combinations

- **PS-B + PS-A (explain).** The combination the loop opens: a
  judged-wrong route is a verdict-emitting gate's failure, so
  `fleet explain` can ask the lane what the wrongness means, keyed to
  the same route key the judgment joined on. **The selector itself is
  not wired in this slice** — `explain::Selector` carries `Proposal`
  and `Scenario` only, and `--route <key>` lands beside them in the
  slice that adds it. What this slice builds is the join key that
  makes it a one-line addition: the `route_judged` row's key is the
  route row's key, computed by the same function the beat and
  `fleet judge --agree` both call.
- **PS-B + PS6 (eval).** Built here. The `calibration-loop-closes`
  scenario grades the loop — the judged rows beside the routes, the
  join key, the idempotent second batch, the dry run's no-spend, the
  fail-open refusal, the fit rule, and the conservative influence
  (a fit-and-wrong cell skipped, the same provider's right cell not
  skipped for the other shape, an unfit cell never moving routing, and
  the third vector's insufficient cell skipping nothing while its
  companion note names what the minima needs) — through scripted seams
  only, with no paid live calls. When the eval corpus regresses, the
  regression is itself a row the calibration reads.

## Not built

- **Answer-quality judging.** The judgment grades the *selection*, not
  the answer. Whether the turn's output was good is a different
  question with different evidence, and nothing here claims it.
- **`fleet explain --route <key>`.** The join key is built (above); the
  selector that reads it is not.
- **sdm1's cost-vs-value voice.** The sibling's calibration reports
  cost against value in its own vocabulary; fleet's surface reports
  cost per correct turn per cell and stops there.
- **Cross-session calibration.** The loop reads one day. Nothing
  accumulates a week's cells into a longer-horizon trend.
- **Automated dispositions.** The human seam is manual by design.
