# blend1: the grounded lane (slice 16)

`blend1` is the fifth vendored lane — the blended lane of the sys1 house
pattern, and the one that exists because **LLMs do not do math; TabPFN
does**.

One `ask()` runs the d1 router (sys1) **solo first**, as the front door;
then the fact lanes — the local EmbeddingGemma-2 semantic lane (sem1)
and the Prior Labs TabPFN tabular lane (sdm1) — run concurrently; and
Liquid's LFM2.5-24B-A2B synthesizes **only when the router clears
`ROUTE_FLOOR` (0.70)**. A validation gate (GLiDE → d1) rates the
synthesized answer before it ships, with one bounded retry and a critic
chain behind it. The result is a cheap model whose reasoning stands on
computed facts rather than invented ones.

The lane composes its three siblings; it does not reimplement them. Its
own core is stdlib-only, and the siblings resolve as path dependencies
through the house `<SVC>_PYTHONPATH` mechanism.

Upstream: `~/Projects/blend1` (branch
`feature/escalation-and-sys1-score`, **not merged to `main`** — merging
requires the owner's approval, Apache-2.0), vendored here at
commit `ab4ac55` — *"feat: escalation leg + sys1-scored reprocess — two
bounded legs beside the gate"* — under `services/blend1/` with the core,
the service, the LICENSE, and `VENDOR.json` recording that commit: the
same shape the other four lanes carry. The MCP adapter
(`service/mcp_server.py`) is **new, written in the vendored tree**: the
other four lanes each shipped one, blend1 shipped a library plus an
optional FastAPI service, and MCP stdio is fleet's lane contract. The
upstream carry is logged.

## The lane's tools

The adapter speaks MCP 2024-11-05 over stdio and exposes five tools.
Every argument is validated with a **named** error (a `-32602` reply
naming the offending field), never a traceback.

| Tool | Arguments | What it does |
|---|---|---|
| `ask` | `prompt` (string, required, non-empty); `table` (array of row objects, optional); `target` (string, required when `table` is given); `task_type` (`"classification"` \| `"regression"`, default `classification`); `retrieve` (bool, default `true`) | one blended ask — the grounded path end to end |
| `doctor` | — | per-lane availability snapshot, reasons named |
| `search` | `query` (string, required); `project` (string, optional); `k` (int ≥ 1, default 3) | semantic search over a project's corpus; `[{key, score, text}]`, best first |
| `refresh` | `project` (string, optional); `sources` (array of paths/globs, optional) | docs-derived corpus re-derivation, hash-gated |
| `seed` | `items` (non-empty array of `{key, text}`, required); `project` (string, optional) | the corpus's write path; idempotent by key |

Project resolution is the lane's own rule, applied once per call:
explicit parameter > `BLEND1_PROJECT` > the cwd git-repo name > `None`
(the legacy store). The library validates the name itself (one path
component, no separators) and its `ValueError` is the wire's named error.

**Over MCP the cwd is the lane's home, not the caller's.** The launch
spec sets the child's working directory to `services/blend1/`, so
`resolve_project(None)` there resolves to **`fleet`** — the repo hosting
`services/` — never the legacy store. An omitted `project` on `search`,
`refresh` or `seed` therefore indexes or queries fleet's own corpus; a
different project's corpus needs its name passed explicitly (or
`BLEND1_PROJECT` set). Likewise `refresh`'s default globs resolve against
that same cwd, where `services/blend1/` holds no docs, so an omitted
`sources` list derives nothing and reports `action: "no-source"` — the
scheduled jobs carry explicit absolute globs for exactly this reason.

`ask`'s result is the `BlendResult`'s fields **as they are** (structured
content), so a result key added upstream rides through without an
adapter change. The adapter adds two keys of its own beside them:

- `escalated` — the library's property: `true` when any lane failed open
  or the validation gate failed the answer.
- `failed_lanes` — every lane report whose status is `error`, scanned
  generically so a lane added upstream is surfaced too.

## The grounded path, end to end

```
prompt ──► route lane (sys1 + Liquid d1: needs_generation? needs_live_facts?)
                │
   ┌────────────┼────────────────────┐
   ▼            ▼                    ▼
semantic      tabular [if table]    web [if the router fires
(sem1:        (sdm1 + TabPFN:        needs_live_facts, or live=True;
 embed +       exact predictions)    never when live=False]
 neighbors)          │
   └────────────────┴────────────────────┘
                     ▼
               merge facts
                     │
         needs_generation? ── no ─► direct answer (no 24B call)
                     │ yes
                     ▼
               generate lane (LFM2.5-24B-A2B)
                     │
               validation gate (GLiDE → d1)
                     │
               final answer (+ optional voice)
```

1. **Route, solo.** d1 rates the ask: does it need open-ended generative
   synthesis? `needs_generation` is a noul whose value *is* the
   probability, and the floor is 0.70. Below the floor the answer is
   **direct** — composed from the computed facts, with no paid call at
   all. A router that cannot answer fails *toward* synthesis (quality
   over cost), with its error riding in the result.
2. **The fact pool, concurrently.** semantic embeds the prompt and
   retrieves the project's corpus neighbors; tabular scores the query
   rows of `table` with TabPFN, exact. Both fail open with their reason
   named. The web lane (tinyfish search + fetch) fires only when the
   router's `needs_live_facts` clears the floor, or when `live=True` —
   it fails **closed**, because an unfired free lane costs latency only.
   Fetched text rides the S2 prompt framed as untrusted reference data,
   never as instructions.
3. **Gated synthesis.** When the router clears the floor, the 24B is
   called once. TabPFN's number reaches it as a **given** in the system
   prompt (`S2_SYSTEM_TEMPLATE`): the model is instructed never to
   invent, round, or alter it.
4. **Validation.** GLiDE rates the answer against the ask and the givens
   (`answers_the_question`, `contradicts_facts`, plus a deterministic
   number check over the metric-shaped numbers). A definitive fail gets
   **one bounded retry** with the verdict injected as feedback (worst
   case ≈ 2× generate cost, failures only); an **inconclusive** outcome
   (`passed: null` — no critic answered) ships **ungated and does not
   retry**; a retry that does not clear the gate ships the **original**
   answer, flagged. The gate never blocks.
5. **The correctness score and the escalation.** sys1 (d1) scores the
   answer's correctness beside the gate; a wrong score reprocesses once.
   An unresolved gate — inconclusive, or a retry that did not clear —
   escalates once to a larger configured model, and the escalated answer
   ships flagged. Neither leg blocks.
6. **The result.** One shape regardless of which lanes answered.

**The two legs' order is a named limitation.** The score leg runs
*after* the gate and can replace `generated`, while the escalation
re-synthesizes from the givens with **only the gate's feedback** — it
never sees the sys1 verdict. So an escalation can replace an answer the
score had already corrected, and the reprocessed answer is never
re-gated. The spend is bounded and both outcomes are flagged in the
result (`escalated_answer`, `score.shipped`), so nothing is hidden; the
ordering itself is a design limitation of the vendored core, not a
fleet-side choice.

### The result keys

`prompt_sha256`, `prompt_chars`, `path` (`direct` \| `synthesized` \|
`decompose`), `route`, `semantic`, `tabular`, `generated`, `validation`,
`escalation`, `score`, `voice`, `web`, `final_answer`,
`total_latency_ms`, `escalated_answer` — plus the adapter's `escalated`
and `failed_lanes`. **The result keys are an additive contract**: they may
only grow. Renaming or removing one is a breaking change and needs a plan.

Two keys deserve their meaning spelled out, because they are easy to
conflate:

- **`escalated`** means exactly **"a lane failed open"** — any lane report
  whose status is `error`, or a validation gate whose verdict did not
  clear. The gate's own computation is `not passed`, so a **definitive
  fail after the retry** and an **inconclusive verdict** (`passed: null`
  — no critic answered) both set it: `None` is not truthy. The answer
  ships, but the caller deserves to know it rests on fewer lanes than
  requested, or on a verdict the gate never reached.
- **`escalated_answer`** flags the **shipped** answer as the escalated
  one. The escalation leg fires only on an unresolved gate, so in
  practice the two keys **always co-occur**: the gate's unresolved
  verdict is what sets `escalated`, and `escalated_answer` is what says
  the answer that shipped is the escalation's rather than the
  pre-escalation one — which rides on, retained, in `generated`.

The two legs the plan names as B10 and B11 are in the vendored core:

- **the escalation** — when the validation is inconclusive (`passed:
  null`, no critic answered) **or** the bounded retry does not clear the
  gate, the answer escalates to a larger configured model with the same
  givens (the untrusted-source framing intact). The escalated answer
  ships **flagged** (`escalated_answer: true`, and
  `escalation.shipped: true`). Bounded like the retry: **one attempt per
  ask** — the worst case is the initial generate + the gate's one retry +
  this one escalation ≈ 3× the generate cost, on failure paths only, and
  the sys1 reprocess can add one more (≈ 4×), also failures only. The
  gate never blocks: an escalation that is **unconfigured** (no
  `BLEND1_ESCALATION_MODEL` / `BLEND1_ESCALATION_BASE`), **keyless**, or
  **failed** is named with its reason, and the original answer ships.
- **the sys1-scored reprocess** — sys1 (d1) scores the generated answer's
  correctness against the givens, on the same state shape and the same
  evidence the GLiDE critic reads. A wrong score (`correct: false`)
  reprocesses with the score's verdict as feedback, bounded by the
  retry's own discipline. The score carries `reprocessed` and `shipped`
  (`"reprocessed"` or `"original"`); a reprocess that fails ships the
  scored answer with `reprocess_error` named. The GLiDE gate stays; the
  sys1 score is the correctness judgment beside it. An unavailable sys1
  (the route lane is the sys1 front door, so a route error means sys1 is
  down) reports `status: "skipped"` with the reason named — never a
  fabricated score.

## Fail open, and name it

Every lane fails open. A lane that cannot answer records its reason in
its own lane report, and the orchestrator composes the best answer from
the lanes that did answer. `BlendResult.escalated` is `true` when any
lane errored, **or** when the validation gate's verdict did not clear —
a definitive fail after the retry, **or an inconclusive verdict** (`None`
is not truthy, so the gate reads as failed) — the answer ships, but the
caller deserves to know it was rated unsatisfactory or never rated at
all.

Nothing degrades silently. The adapter's `failed_lanes` names every lane
report whose status is `error`; a generative lane that is down produces a
fallback answer composed from the computed facts, with
`Generative lane unavailable: <reason>` at its head; a down critic ships
the answer **ungated**, with the uncertainty recorded. No key material
reaches the wire or a log: `doctor` reports key *names*, and blend1's
own telemetry redacts structurally (sha256 + counts, never prompt text,
answer text, or key material).

## The install's venv and path wiring

`fleet install --lanes` builds blend1's venv at
`<home>/venvs/blend1/`, like every other lane: idempotent, receipt-backed,
degrade-without. Its package set is the **union of its three siblings'
sets** — `gliner2[local]`, `structured-data-models`,
`sentence-transformers` — because blend1's own core is stdlib-only and
its venv exists to serve the sibling cores its lanes compose, not blend1
itself.

The cores themselves are **path-wired, not pip-installed**. The adapter
reads the house mechanism in resolution order, each inserted only when
its variable is set (a sibling that is absent fails open inside its own
lane with the reason named; only blend1's own core is mandatory):

| Variable | Points at |
|---|---|
| `BLEND1_PYTHONPATH` | `services/blend1/src` — blend1's own core (mandatory) |
| `BLEND1_SYS1_PYTHONPATH` | `services/sys1/src` — the judgment core (the d1 router) |
| `BLEND1_SEM1_PYTHONPATH` | `services/sem1/src` — the embeddings core |
| `BLEND1_SDM1_PYTHONPATH` | `services/sdm1/src` — the tabular core |
| `BLEND1_GEN1_PYTHONPATH` | `services/gen1/src` — the media core (the voice leg) |

The install prints **no export line for blend1**, and that is
deliberate: the path wiring is not ambient state the owner exports. The
launch spec sets every `BLEND1_*_PYTHONPATH` at spawn — blend1's own
core beside the four siblings' — so a lane launched by fleet (through
`fleet lanes`, the probe, or the agent's tool loop) is wired correctly
with nothing in the shell. What the install's line names instead is the
mechanism, for the one case that needs it: a **hand-run** adapter, where
the owner supplies the names themselves (and where a venv root would be
the wrong value for `BLEND1_PYTHONPATH` — it addresses
`services/blend1/src`, not `~/.fleet/venvs/blend1`).

What does ride through the passthrough is the **credential names** — the
ambient variables the lane's providers read, copied only when present:
blend1's passthrough is the union of its legs' credentials —
`LIQUID_API_KEY` (one vendor, one key: the d1 router and the 24B
generative lane), `FASTINO_API_KEY` (the GLiDE gate), `TOKEN_PLAN_API_KEY`
and `OLLAMA_API_KEY` (the critic chain), `TINYFISH_API_KEY` (the
optional web leg), `DASHSCOPE_API_KEY` / `STEPFUN_API_KEY` (the
optional voice leg), and `TABPFN_API_KEY` (the tabular leg) — beside the
escalation leg's wiring (`BLEND1_ESCALATION_MODEL` / `_BASE` / `_KEY_ENV`
/ `_API_KEY`), the per-critic overrides
(`BLEND1_CRITIC_1/2_{BASE,MODEL,KEY_ENV}`), the telemetry root
(`BLEND1_LOG_DIR`) and the corpus locators (`BLEND1_VECTORS_DIR`,
`BLEND1_PROJECT`). `DEEPSEEK_API_KEY` is on no lane's list.

## The doctor card

`doctor` answers one row per lane, each with `available`, `reason`, and
`provider` — reasons named, a missing key naming its **variable**, never
its value:

| Row | What it names |
|---|---|
| `route` | Liquid d1 via sys1 — the front door |
| `semantic` | llama-server (EmbeddingGemma-2) — dims and latency when reachable |
| `tabular` | tabpfn-hosted (Prior Labs) |
| `generate` | `lfm2.5-24b-a2b` via api.liquid.ai — whether `LIQUID_API_KEY` is set |
| `validate` | glide → d1 (oversized states) — whether `FASTINO_API_KEY` is set |
| `critics` | the escalation chain, each `{critic, configured, key_env}` |
| `voice` | gen1's speak cascade (opt-in) |
| `web` | tinyfish search + fetch (opt-in) |

`fleet lanes` renders this on blend1's card, at the house visual bar:
the lane's name and role (`grounded`), its probe status dot, and one
named row per sub-lane with its own status dot — green with its backing
provider when the lane can answer, dim with the named reason when it
cannot. The lanes modal (**Ctrl+L**) renders the same rows beside the
other four lanes' cards, and a refused probe keeps the card and names
the error rather than dropping the section — the boards'
snapshot-and-name contract, on the lanes surface.

The card renders the **five lanes an `ask()` composes its answer from**
— `route`, `semantic`, `tabular`, `generate`, `validate`, in the blend's
own order. `critics`, `voice`, and `web` ride in the same report and are
read from it directly (`blend1 doctor` on the command line); they are the
opt-in and escalation legs, not lanes an `ask()` composes from, so the
card leaves them to the report.

The probe calls `doctor` only for a lane that lists it in `tools/list`,
and only after the handshake has answered: a doctor that cannot answer
is an empty report, never a failed probe, because a doctor that cannot
answer is not a lane that broke.

## The Liquid provider

`route.rs`'s registry declares Liquid as a candidate:

| Field | Value |
|---|---|
| `id` | `liquid` |
| base | `https://api.liquid.ai/openai/v1` |
| model | `lfm2.5-24b-a2b` |
| key env | `LIQUID_API_KEY` (key required) |

It is appended **at the end** of the declared default order on purpose.
The declared chain — z.ai → mimo → stepfun → token plan → deepseek —
is the owner's standing directive, and this slice does not reorder it. Liquid is *declared*: the owner's `config.json` decides its
real position, and moving it into the chain is a one-line config change
whenever the owner wants it.

Two notes that belong beside the declaration. The base URL's shape
matters: Liquid's chat completions live under `/openai/v1`, **not**
`/v1` (a `/v1` path 404s), and one key serves both the d1 router and the
24B generative lane. And **d1's judgment path already rides inside
sys1** — the route lane calls `sys1.classify("liquid", …)`, so the d1
decision model reaches fleet through the judgment lane, and this slice
makes no change to sys1.

## The derived problem shapes

The use cases below are derived from the problem shapes that recur in the
owner's work. Each names the shape, **the naive path that fails today and
how it fails**, the pieces that solve it — including the combinations,
because the combinations are the point — and its fleet surface.

| # | Shape | The naive path that fails | The pieces that solve it | Fleet surface |
|---|---|---|---|---|
| PS1 | the cold-context question | invents (an ungrounded model), or doesn't scale (reading the repos' docs by hand) | the per-project corpus + the grounded ask; `search` when a quote is enough | `ask`, `search`; the `blend1-corpus-recall` and `blend1-grounded-fact` scenarios |
| PS2 | the number that must be right | confident, plausible, **wrong** — LLMs do not do math | the tabular lane + the sys1-scored reprocess + the number check | `ask` with `table`/`target`; the `blend1-grounded-fact` and `blend1-reprocess` scenarios |
| PS3 | the docs that drifted | discovered at the worst moment — a warmup that reads a stale recap | the scheduled per-project refresh (hash-checked) + the tower board's git facts | `refresh` under a `fleet cron` job; the tower board |
| PS4 | the cheap answer that might be wrong | paying frontier prices for everything (waste), or shipping maybe-right (worse) | the ladder: router gate → GLiDE retry → reprocess → escalation | the `ask` result's keys; the `blend1-escalation` and `blend1-reprocess` scenarios |
| PS5 | the ecosystem's currency | watching twenty repos by hand, or not knowing | a corpus per repo + the web lane + the project-resolved search | `search`, `ask`, the refresh job |
| PS6 | the claim that must be checked | a checker whose own answer is ungrounded — verification as theater | the grounded ask over the corpus + the tabular counts + reprocess + escalation | `ask`, `search`; the `blend1-grounded-fact` and `blend1-reprocess` scenarios |
| PS7 | the context that doesn't survive | re-reading, or inventing, every cold session | fleet's `memories/` + the corpus + the scheduled refresh | `memories/`, the corpus, the refresh job |

### PS1 — the cold-context question

*"What does project X say about Y?"* asked of a repo whose docs have not
been read this month.

**The naive path fails** by not scaling: reading the ecosystem's repos'
docs by hand is a morning, every time.

**The pieces.** The per-project corpus holds the project's facts,
semantically indexed; the grounded ask retrieves the neighbors and lets
d1 decide whether synthesis is needed at all — a **direct** answer
composed from retrieved facts costs nothing, because the router does not
clear the floor. `search` is the cheaper door when a quote is enough:
the neighbors, scored, best first, no synthesis leg.

**Combination with PS3:** the cold question is answered from a corpus
that stays current, not one seeded once and left.

**Fleet surface.** `mcp__blend1__ask` (with `retrieve`, default true) and
`mcp__blend1__search`. Graded by the `blend1-corpus-recall` scenario (the
seeded fact quoted unaltered) and the `blend1-grounded-fact` scenario
(the semantic neighbors riding beside the tabular fact).

### PS2 — the number that must be right

*"How many / what rate / which segment / what's the forecast?"* — the
answer is a computed fact, and **LLMs do not do math; TabPFN does.**

**The naive path fails** confidently and plausibly and wrongly: a
language model asked for a churn rate produces a number shaped like a
churn rate, and nothing downstream can tell it was invented.

**The pieces.** The tabular lane computes the number — TabPFN scores the
query rows of `table` against the labeled context, exact — and the number
rides into the synthesis prompt as a **given** the model may not invent,
round, or alter. Behind it, two guards: the validation gate's
**deterministic number check** (every metric-shaped number in the answer
must appear in the givens; percentages canonicalize to probabilities, so
`p=0.924` and `92.4%` compare equal), and the **sys1-scored reprocess** —
if the model altered the number anyway, `score.correct` is `false`, the
answer is re-synthesized with the verdict as feedback, and the corrected
answer ships (`score.shipped: "reprocessed"`).

**One honest limit on the reprocess:** `SCORE_FLOOR` (0.70) is
**unfitted** until sys1 has feedback rows to calibrate against — the
repo's own gate-floor convention, and the value rides in the result
either way. So the score is a judgment signal, not a calibrated
probability, until the lane has history.

**Combination with PS4:** the computed fact with the confidence ladder
behind it.

**Fleet surface.** `mcp__blend1__ask` with `table` and `target`. Graded
by `blend1-grounded-fact` (the fact reaches the answer unaltered, never
rounded) and `blend1-reprocess` (a sys1-wrong score reprocesses and the
corrected answer ships).

### PS3 — the docs that drifted

Derived knowledge — `CLAUDE.md`, recaps, plans — drifting from its source,
the code, discovered at the worst moment: a warmup that reads a stale
recap and reasons from it.

**The naive path fails** silently and late. Nothing announces the drift;
the first symptom is a decision made on last month's architecture.

**The pieces.** The **scheduled corpus refresh** per project re-derives
the corpus from the docs and rebuilds only when the text hashes changed —
the steady state is file reads, not embedding, so a cron cadence is
cheap — and a rebuild names what changed. Beside it, the **tower board's
per-project git facts** put the drift next to the commits that caused it:
dirty count, head commit, trailing-thirty-day count, ahead/behind.

**One honest limit on the staleness check:** the hash comparison is over
**text** only. If the embedding server's model or dims change behind the
fixed store slug, the hashes still match and the refresh reports
`unchanged` — model drift is undetectable without an embedding call. A
model change therefore needs an **explicit re-seed** (`blend1 seed`), not
a refresh.

**Combination with PS1:** the cold question answered from a corpus that
stays current.

**Fleet surface.** `mcp__blend1__refresh` under a `fleet cron` job
(`auto_approve: true` — declared, idempotent, hash-gated), and the tower
board in the TUI's board pane (Ctrl+B).

**Where the jobs live.** The fourteen per-project refresh jobs
(`corpus-refresh-<project>`, daily at staggered minutes from 00:10 to
01:15 UTC, `auto_approve: true`) are **machine state, not repo state**:
the parent created them on this machine with `fleet cron add`, and their
shape is recorded in the slice-16 plan's B13. No repo file or test pins
them, so a **fresh `FLEET_HOME` has no refresh jobs** — re-create them
from that record. Declaring them repo-side is out of scope for this
slice.

### PS4 — the cheap answer that might be wrong

Routine questions that mostly do not need a frontier model — except when
they do, and you do not know which until you have looked. Paying frontier
prices for everything is waste; shipping maybe-right is worse.

**The naive path fails** in one of two directions, and both are expensive:
overpay on every ask, or under-verify and ship an answer nobody checked.

**The pieces — the ladder.** Each rung is cheap until it is needed, and
each exception is named:

1. **the router gate** — d1's `needs_generation` noul against
   `ROUTE_FLOOR` (0.70). Below the floor the answer is **direct**,
   composed from computed facts, and the paid leg never runs: $0.
2. **the GLiDE gate with one bounded retry** — a definitive fail gets one
   retry with the verdict injected as feedback; worst case ≈ 2× generate
   cost, failures only. An **inconclusive** verdict ships ungated and does
   not retry.
3. **the sys1-scored reprocess** — wrong → corrected before it ships,
   bounded by the retry's own discipline (`score.reprocessed`,
   `score.shipped`).
4. **the escalation** — an unresolved gate → one larger call, and the
   shipped answer is **flagged** on `escalated_answer`, with the gate's
   unresolved verdict riding beside it in `escalation`. Unconfigured,
   keyless, or failed → named, and the original answer ships.

The gate never blocks: a down critic ships the answer ungated with the
uncertainty recorded, and if the escalation itself fails the original
answer ships with the failure named.

**The combination is the point:** cheap by default, correct on exception,
every exception named in the result.

**Fleet surface.** the `ask` result's own keys — `path`, `validation`,
`escalated`, `failed_lanes`, and the additive keys the escalation and
reprocess legs ride in on. Graded by `blend1-escalation` (inconclusive →
escalated → flagged, the pre-escalation answer retained) and
`blend1-reprocess` (wrong → reprocessed → corrected, the wrong answer
retained).

### PS5 — the ecosystem's currency

~20 repos, the lanes and the kit moving under him. *"What changed, and
does it affect me"* is not answerable by watching.

**The naive path fails** by omission: the change that matters is the one
nobody was watching when it landed, and the local docs are the last to
hear about it.

**The pieces.** A **corpus per repo**, each kept current by its own
refresh, so the local half is always answerable. The **web lane**
(TinyFish search + fetch) covers what is outside the local docs — it
fires only when the router's `needs_live_facts` clears the floor, or when
`live=True`, and fetched text rides the synthesis prompt framed as
**untrusted reference data**, never as instructions. The
**project-resolved search** asks the right repo's corpus rather than a
blend of all of them.

**Combination with PS4:** cross-repo currency, cheap unless it isn't —
the router decides when the paid leg and the web leg are worth it.

**Fleet surface.** `mcp__blend1__search` (with `project`),
`mcp__blend1__ask`, and the per-project refresh job.

### PS6 — the claim that must be checked

The standing rule: *done* means tool-verified. A claim about a system —
"the gate is green", "the lane serves" — must be checked against evidence,
and the checker's own answer must be grounded, or the verification is
theater.

**The naive path fails** by laundering: a model asked to confirm a claim
confirms it, fluently, and the confirmation looks exactly like the
evidence would.

**The pieces.** The **grounded ask over the project's corpus** — the
claim is checked against what the project says, not against what a model
believes. The **tabular lane** for the counts: a claim that is a number is
checked by computing it. The **reprocess** when the answer contradicts the
evidence, and the **escalation** when the evidence is thin — the answer
ships flagged, so a thin check never reads as a firm one.

**Combination with PS2:** verification where the number *is* the claim.

**Fleet surface.** `mcp__blend1__ask` and `mcp__blend1__search`. Graded
by `blend1-grounded-fact` and `blend1-reprocess`.

### PS7 — the context that doesn't survive

Every session starts cold; the agent re-reads, or invents. Durable project
knowledge must survive session boundaries — and the agent's own history
and the project's facts are different things with different homes.

**The naive path fails** by conflating them: one store that holds both
what the agent learned and what the project says answers the wrong
question with the right-looking fact.

**The pieces.** Fleet's `memories/` holds the **agent's** durable facts —
git-tracked, provenance-carrying, recalled into a turn's context by the
memory slice. The corpus holds the **project's** facts — docs-derived and
TinyFish-sourced, semantically indexed under
`~/.local/share/blend1/vectors/<project>/`, retrieved by cosine
similarity. The **scheduled refresh** keeps the project's half current
without a session.

**The division is documented, not merged.** One is what the agent has
learned; the other is what the project says. `CLAUDE.local.md` is never
in a corpus — the library's own guard, inherited.

**Fleet surface.** `memories/` (the memory slice), the corpus
(`search`/`seed`), and the refresh job.

## The eval scenarios

Four deterministic scenarios in `src/eval.rs`'s fixed corpus, family
`grounded-lane`, each a scripted seam with no paid live call:

| Scenario | What it grades | Serves |
|---|---|---|
| `blend1-grounded-fact` | a tabular fact reaches the model's context and the final answer **unaltered** — never invented, never rounded | PS1, PS2, PS6 |
| `blend1-escalation` | an inconclusive validation escalates, the escalated answer ships **flagged**, and the pre-escalation answer is retained | PS4 |
| `blend1-reprocess` | a sys1-wrong score reprocesses and the **corrected** answer ships, with the score and the reprocess outcome in the result | PS2, PS4, PS6 |
| `blend1-corpus-recall` | a seeded project fact is retrieved by `search` and quoted **unaltered** | PS1 |

They drive the lane through the `mock-mcp` binary over a script — the
Rust-side form of the lane's own `_lane_overrides` test seam — so the
lane's `ask` reply is a fixture whose keys follow the vendored core's own
additive contract. Each scenario therefore grades fleet's side of the
contract: the fact that reaches the model's context, the answer that
ships, and the keys that name what happened.

## What is not built

- **The blend1-shaped subagent engine.** Route gate → parallel lanes →
  synthesis → validation → escalation as the general template for
  fleet's own subagents. The follow-on design, named not built.
- **The FastAPI service surface** (port 8420, `/v1/ask`, `/v1/search`,
  `/v1/health`). The MCP adapter is fleet's path; the service stays the
  other languages' door.
- **blend1's telemetry in the boards.** Its redacted JSONL log beside the
  calibration stream — a small follow-on once the lane is live.
- **The escalation target's own configuration.** The leg reads
  `BLEND1_ESCALATION_MODEL`, `BLEND1_ESCALATION_BASE`, and
  `BLEND1_ESCALATION_KEY_ENV` (default `BLEND1_ESCALATION_API_KEY`) from
  `~/.config/blend1/env`; unset, it reports `status: "skipped"` naming
  the variables. Fleet declares the provider; the owner's env file
  configures the target.
