# Deep dive: oh-my-hermes, read against Fleet — 2026-11-20

Source: [rlaope/oh-my-hermes](https://github.com/rlaope/oh-my-hermes) (MIT,
v4.0.8), read from a full clone at `tmp/omh` — 2,791 Python files, 1,217
markdown files, 228 MB. Every fleet-side claim below was read at its file:line
this session; the OMH-side claims are the repo's own files. Where this doc
infers beyond either source, it says so.

> **Correction, 2026-11-20 (the same session that built from this dive).**
> Finding 1 below overstated its case: it claimed the 404 skills outside the
> declared 60 were **unloadable by name**. They are not. `Work::UseSkill`
> passes the whole library (`src/agent.rs:5595`), so `use_skill` always
> reached every loaded skill. The real defect is narrower and still real —
> the prompt **advertised** only the newest 60 by file mtime, so the model
> was never told the other 404 existed. Everything else in finding 1
> stands; the "unloadable" sentence does not. Recorded as KI-576, and the
> plan increment it shaped was withdrawn as void.

OMH in one line: **a deterministic orchestration layer that sits beside
another agent** — it installs next to Nous Research's Hermes Agent and never
patches it, never calls a model in its core, and spends almost all of its
engineering effort on one question: *what does the system actually know, as
opposed to what has it merely been told.*

That question is Fleet's own (README: "verifiable facts drive everything it
does"). So this dive is not "what features does OMH have that Fleet lacks" —
on feature-count Fleet is far ahead, and OMH has no memory substrate, no
router process, no calibration stream, no face of its own beyond a TUI over a
foreign gateway. The interesting finding is narrower and sharper: **in four
places OMH's answer to its central question is measurably better than
Fleet's, and one of those places is costing Fleet real capability right now.**

## The four findings, ranked

| # | Finding | Fleet today | Severity |
|---|---|---|---|
| 1 | Skill selection is by recency, not by meaning | 464-skill library cut to the 60 newest by mtime; 404 never *advertised* (still loadable) | **High — a discovery loss** |
| 2 | Declared write scope is a dispatch lock, not a boundary | Plan nodes declare `paths`; enforced against concurrent siblings, never against the writing child | **High — the delegation graph's main safety claim** |
| 3 | Budgeted values drift from their published documents | One live instance already: the guard doc says 10,050, the code says 10,800 | **Medium — silent, and it has already happened** |
| 4 | A "done" mark is a claim, not an evidence link | `todo` renders `[x]` on the model's word alone | **Medium — an unbacked "done" is Fleet's cardinal sin** |

Three more are worth naming but are adaptation, not adoption: OMH's
two-axis evidence label, its `define`-time graph refusals, and its routing
prep chain. They are section 5 below.

## Where Fleet is already far ahead

Worth stating plainly, because it bounds what is borrowable. Fleet has, and
OMH has no equivalent: a git-tracked memory corpus with six-channel recall
and a feedback loop (`src/memory.rs`); an in-process multi-provider router
with sys1 judgment, failover classes, and demotion (`src/route.rs`); a
calibration stream with graded judge/delegate/explain beats (`src/calib.rs`);
a plan-tied graph generated only from an approved plan, with a monotone cost
score and an acceptance-criteria roll-up (`src/plan_graph.rs`, `src/graph.rs`);
a graded eval corpus run headless against a mock (`src/eval.rs`); and 156
text-scan guard sites (`grep -roh 'include_str!' src --include=*.rs | wc -l`
→ 156) that pin invariants the compiler cannot see.

OMH, by contrast, is 611k lines of Python (`find src -name '*.py' -exec cat {} +
| wc -l` → 611,358) building an evidence layer around another agent's loop,
and its
own docs are candid about the cost: `src/maintenance/drift.py:1-20` describes
a session that spent **twelve full-suite cycles** chasing one catalog change's
knock-on effects. Fleet's problem is not feature poverty. It is that four of
its own load-bearing mechanisms answer "what does the system know" less well
than a Python tool that only does that.

---

## 1. Skill selection: recency is standing in for meaning (High)

### What Fleet does

`src/agent.rs:319` declares `SKILLS_PROMPT_CAP = 60`. `src/agent.rs:388-410`
(`declared_skills_priority`) sorts the whole library by the skill file's
**mtime, newest first**, then truncates to 60. `src/agent.rs:415-423` logs the
one honest line that admits what happened:

> `fleet: {library} skills in the library; {held} held back from this run's prompt (the newest {SKILLS_PROMPT_CAP} are declared)`

Measured this session:

- `ls ~/.agents/skills | wc -l` → **467** skill directories; `ls
  ~/.agents/skills/*/SKILL.md | wc -l` → **464** carrying a `SKILL.md`.
- `grep -n 'SKILLS_PROMPT_CAP: usize' src/agent.rs` → `319:pub const SKILLS_PROMPT_CAP: usize = 60;`
- So **404 skills are held back from every run's prompt** (464 with a
  `SKILL.md`, less the 60 declared), and which 60 depends on file modification
  time.

The failure this produces is not theoretical. The mtime distribution is
lumpy — 224 skills share a single mtime day (2026-08-30), because a bulk
install stamped them — and the newest 60 are dominated by the HyperFrames
video family, which was written last:

```
skills at ranks 55-65 (newest-first):
  55 hyperframes-studio   56 hyperframes-registry  57 hyperframes-keyframes
  58 hyperframes-creative 59 hyperframes-core      60 hyperframes-cli
  61 hyperframes-audio    62 hyperframes-animation 63 general-video
```

Ranks 56-62 are seven sibling video-composition skills that no ordinary
coding turn will ever select, and they occupy over a tenth of the entire
declared surface. Meanwhile a skill at rank 61+ that a coding session might
genuinely want is invisible to the prompt.

The consequence is worse than a bad ranking, because `use_skill` cannot
recover it. `src/skills.rs:860` (`use_skill_result`) looks the name up in the
**run's declared slice**, not the library:

```rust
match skills.iter().find(|skill| skill.name == name) {
    Some(skill) => skill.body.clone(),
    None => { /* "error: no skill named `{name}`" or the retired message */ }
}
```

and the slice is `self.skills.clone()` at `src/agent.rs:5572-5575`. So a skill
outside the 60 is never **advertised**: the model is not told it exists, so it
cannot ask for it by name unless it happens to guess. (It *can* load the name
if it has one — `use_skill_result` is handed the whole library, which the
correction above records; this dive first claimed otherwise and was wrong.)
The loss is a discovery loss, not a hard ceiling.

Note this is *also* the largest context cost in the file: at a 400-char
description clip, the 60 declared skills measure ~15,760 chars ≈ 3,940 tokens
per request (measured on the live library this session). Fleet already
publishes a tool-declaration budget (`MAX_DEFINITION_TOKENS`, 10,800) and a
tool-count budget (`MAX_VISIBLE_TOOLS`, 36), but the skills index — a larger
surface than either — has no token budget at all. It has a count budget that
was chosen when the library was small.

### What OMH does differently

`src/plugin_bundle/omh/skill_shortlist.py:1-30` is the whole argument, and it
is an argument from measurement:

> "The rule table in `awareness.py` names a workflow only when a message says
> one of its cue phrases, and a person who does not know a skill's vocabulary
> describes the situation instead ("our week-one retention dropped").
> Measured on a live model, that table named the intended skill for **14%** of
> paraphrased work requests, while a **BM25 ranking over each skill's name,
> triggers, situations, and description held it in its top four for most of
> them.**"

The mechanism is small and entirely portable:

- A **committed sidecar index** (`tools/skill_shortlist.json`, 130 KB) holding
  a per-skill bag of words: `anchors`, `terms` (numbered stem groups),
  `situation`, `label`. 156 skills, one entry each — the same number as
  Fleet's guard sites above, a coincidence of two unrelated inventories.
- A **BM25 scorer** with declared `k1`/`b` and a **`score_floor` of 5.0**
  (`skill_shortlist.py:229-236`, `:510`), so a request that matches nothing
  well returns nothing rather than forcing a bad candidate.
- **Two scripts, two rankings** — ASCII words, and Hangul syllable bigrams
  with their own separate admission floor, because the catalog serves Korean
  users. Other scripts get no line.
- The output is **one line of candidates, only when the request reads as
  work**, shown once per session per candidate set
  (`skill_shortlist.py:26-30`). It names skills the model *may* load; it
  selects nothing, loads nothing, and the model still calls the loader.
- The sidecar is a **generated artifact under drift control**
  (`src/maintenance/drift.py:1918-1919` names the path and its renderer), so
  it cannot silently fall behind the catalog.

Three properties make this the right borrowing rather than a nice-to-have:
the floor means silence is a valid answer; the ranking is deterministic,
offline, and model-free; and the index is generated from the catalog, so
there is one source of truth.

### The borrowing, concretely

Fleet already owns every piece except the ranking with a floor.

1. **Rank the declared skills by the brief, with a floor.** Fleet has
   `src/tool_router.rs` with exactly this shape for *tools*: a declared
   `FLOOR = 0.68`, a `MARGIN = 0.02`, an `ACTIVATE_MAX = 5`, and a
   `Selection::{Confident, Ambiguous, None}` return. The skills path should
   call the same geometry instead of sorting by mtime. The module doc at
   `src/tool_router.rs:1-6` says "Every function here is pure, so the whole
   selection can be tested without a lane, a network, or a model call" — which
   is precisely what this needs.
2. **Keep `use_skill` library-wide.** Even with better ranking, the loader
   should not refuse a name the library holds. The `None` arm in
   `use_skill_result` should fall back to a library lookup (and keep the
   retired-skill message, which is good). This is the smallest change with
   the largest effect: it turns a hard ceiling into a soft one.
3. **Give the index a token budget, not a count budget.** Replace (or
   supplement) `SKILLS_PROMPT_CAP` with a `SKILL_INDEX_TOKENS` bound in the
   style of `MAX_DEFINITION_TOKENS`, and let the number of declared skills
   fall out of it. Ranked selection then fills the budget with the most
   relevant skills instead of the newest.

Blast radius: `SKILLS_PROMPT_CAP` and `declared_skills_priority` are read from
`src/agent.rs`; `use_skill_result` from `src/skills.rs`; the prompt assembly
at `src/agent.rs:2289`. A grep for `SKILLS_PROMPT_CAP` finds it in
`src/agent.rs` only; `use_skill_result` is called from `src/agent.rs:7642`
and tested in `src/skills.rs`. The `skills` session-log record already pins
what was declared, so hard rule 5 (model-visible ⟺ logged) is satisfied by
construction — the declared list is already logged, whatever selects it.

---

## 2. Declared write scope is not enforced (High)

### What Fleet does

Fleet's plan-tied graph lets a node declare the paths it admits. The module
doc at `src/graph.rs:263-266` states the intent:

> "The admitted paths. Two nodes naming one path never run together…"

and the mechanism that honors it is real, and `graph.rs` calls it
"enforced": `Graph::dispatch` refuses to start a node while a running sibling
holds one of its paths (`src/graph.rs:725-745`, the `path_held` check), and
`holds_on` (`src/graph.rs:1499-1535`) is the consultation that backs it.

But read what it enforces: **"may these two run at once."** It is a mutual
exclusion lock at dispatch, and it never answers "did this child write outside
what it declared." Once a node is running, its child holds an ordinary
`write_file` tool and can write anywhere in the folder. A grep for a
write-time check against a node's declared paths finds only three
sites, all of them constructing a `Node`:

```
src/tools.rs:1247:        .with_paths(node.paths.clone())
src/plan_graph.rs:562:     .with_paths(declared.paths.clone())
src/plan_graph.rs:1049:    .with_paths(child.paths.clone())
```

There is no fourth site where a `write_file` / `edit` / `apply_patch` call is
compared against them. Fleet's write path bounds a child to its **working
folder** (`src/agent.rs:1468`, `:1478`, `:1490` refuse absolute paths,
escapes, and symlinks leaving the root) — a boundary of one directory, not of
one node's assignment. So a plan node declaring `paths: ["src/route.rs"]` is
protected against a *concurrent* sibling touching `src/route.rs`, and not at
all against itself rewriting `src/agent.rs`.

Fleet's delegation scope (`src/subagents.rs:43-48`) is explicit that it is a
declaration:

> "7. **The declaration is the scope.** A delegated node names the
> capabilities its subtree needs, and that list *is* its admitted set: a
> declared tool is admitted and an undeclared one is refused by name."

That is true, and it is enforced — but the vocabulary is **tool names**
(`Scope::admits(&self, name: &str)`, `src/subagents.rs:296`). A child admitted
`write_file` is admitted to every file in the folder. So the declaration is
enforced at exactly one granularity less than the one the graph already
records.

Fleet's own alternative is LayerFS (`src/layerfs_tools.rs:1-20`): concurrent
children work in separate writable Workspaces over one shared history, and
overlap surfaces as a typed `Conflict` — "concurrent agents answer a named
`Conflict` instead of stepping on each other's code." That is a good
mechanism for *detecting* the collision at merge. It is not the same thing as
preventing the write, and it is off unless `FLEET_LAYERFS_PROJECT` is set.

### What OMH does differently

OMH enforces the scope at tool-call time, and its module doc
(`src/plugin_bundle/omh/dag_scope_guard.py:1-40`) states the gap it closed in
almost Fleet's words:

> "`omh_dag define` refuses two nodes that share a write scope with no edge
> between them, and that is the whole of what the scope did: once the children
> ran, nothing looked at what they wrote. A child told `SCOPE: src/api.py`
> that also rewrote its sibling's `src/ui.py` was found, if at all, by the
> verify node after both had finished."

The enforcement is a `pre_tool_call` gate (`dag_scope_directive`) that:

- fires on file-mutating tools only, and only for a session the delegation
  ledger binds to a live node;
- resolves the target against the run's **pinned** `workspace_root`, so a
  child that `cd`s elsewhere is still judged against the same root;
- compares **literally first**, then treats an entry holding `*`/`?` as an
  `fnmatch` glob over the workspace-relative path (with brackets always
  literal, so `app/[slug]/page.tsx` covers itself and not `app/s/page.tsx`);
- walks the **parent chain to the outermost node binding**, so a grandchild
  cannot escape its ancestor's scope, and follows a child that Hermes'
  compaction renamed through its earlier session segments;
- has a distinct, tighter rule for a `read_only` node — every write in the
  workspace is blocked except its own handoff report;
- **passes a target outside the workspace** (scratch files, another checkout)
  and **passes and warns once** on an unresolvable lookup, so the guard fails
  open rather than wedging a run;
- and its refusal message is written as an instruction, not an error
  (`dag_scope_message`):

> "Do not retry this write or route it through the shell; finish what your
> scope covers and name the file you could not change in your handoff report,
> so the parent can repair or extend the run."

That last property is the one most worth copying. A refusal that names the
workaround it is closing ("or route it through the shell") is a refusal the
model cannot rationalize around.

At `define` time OMH additionally **refuses** a graph whose nodes overlap
without an edge (`dag_run.py:709-716`, `write_scope_overlap`) and refuses a
node whose prompt names none of the five planning markers
(`dag_run.py:719-745`, `prompt_markers_missing`). Fleet's `plan_graph.rs` has
the analogous hook and uses it for prerequisites — an unknown prerequisite is
refused by name (`:537-539`) — but does not refuse overlapping paths, because
it never needed to: they never blocked anything.

### The borrowing, concretely

There is one natural seat, and it is a place Fleet already runs on every
write: the approval gate. `src/approvals.rs:528` (`Rule::for_call`) already
classifies every tool call; `src/approvals.rs:349` (`is_destructive`) already
inspects its arguments. A scope rule joins that vocabulary:

1. **A `ScopeRule` beside the approval rules**, consulted once per mutating
   call, which compares the resolved target against the enclosing node's
   declared `paths`. Fleet already has the resolution helpers this needs —
   `resolve_write_in_root` and `resolve_in_root` (`src/tools.rs:4240`,
   `:4288`) are exactly the literal-then-normalized comparison OMH hand-rolls.
2. **The node is already known at the call site.** A delegated child carries
   its node identity from the plan graph; `src/plan_graph.rs:1049` is where a
   child's `Node` (with its `paths`) is constructed. What is missing is
   carrying that node id into the child's `Agent` so the write path can
   consult it — one field, not a new subsystem.
3. **Copy the refusal's shape, not just its rule.** A blocked write should
   name the file, the node, the declared scope, and the handoff escape hatch.
4. **Add the `read_only` class.** Fleet has `read_only` as a tool-surface
   concept (`src/tool_router.rs`, `src/plugins/face.rs`) but not as a node
   role. A verify or review node that cannot write is a stronger guarantee
   than a verify node that is asked not to, and Fleet's verify gate
   (`src/verify.rs:1-20`) would be the direct beneficiary: a verification
   that cannot mutate what it verifies is a verification worth the name.

Blast radius: the approval gate is called from one place in the loop, so a new
rule is contained. The child-side plumbing (carrying the node id into
`Agent`) touches `src/subagents.rs` and `src/plan_graph.rs`. The `paths`
vocabulary already exists in `src/graph.rs` and `src/plan_graph.rs` and needs
no schema change.

---

## 3. Budgets drift from their documents, silently (Medium)

### What Fleet does

Fleet pins many invariants with text-scan tests (156 `include_str!` sites).
Those catch *presence* — "this module must not reference the judge beat" —
extremely well. They do not catch **a number appearing in two places**.

This has already happened, and it is live right now:

| Where | Value |
|---|---|
| `docs/architecture/structure-guards.md:59` | `MAX_DEFINITION_TOKENS` **(10,050)** |
| `src/agent.rs:216` | `MAX_DEFINITION_TOKENS: usize = 10_800` |

The doc is the one the repo tells a session to read before its first compile
("`scripts/edit-preflight.sh` names the ones an edit will meet… this doc is
the one home for what they are"). It is stale by 750 tokens. The constant's
own comment records the whole history — five moves, to 10,000, 10,050,
10,300, 10,700, and 10,800 (`src/agent.rs:186-215`) — while the guard doc
names only the first two; its highest figure is 10,050, and
`src/agent.rs:202` still carries that same stale sentence in the comment
text. The comment's last three crossings never reached the doc.

Nothing failed. That is the point: the code is right, the guard is right, the
tests pass, and the document a session reads first is wrong. A session that
reads 10,050 and trims prose to fit will trim against a bound that does not
exist.

### What OMH does differently

`src/maintenance/drift.py` is a single-purpose answer to exactly this, and its
docstring (`:1-20`) is an evidence-based argument for why it exists:

> "Adding a single skill invalidates counts, budgets, and generated artifacts
> at once, but the test suite reveals them a few at a time: you fix what the
> run showed, rerun the whole suite, and learn about the next one. An observed
> session spent twelve full-suite cycles that way — four of them chasing the
> same byte budget down 305 -> 303 -> 299 -> 264…"

Its shape is worth copying almost verbatim:

- **`DriftMetric`**: a name, a description, a `live` callable that **calls the
  real producer**, an `expected` value, and a tuple of `sites` — file paths,
  **never line numbers, "because line numbers drift"** (`drift.py:20-22`).
- **`BudgetMetric`**: the same, plus a `limit`, a `limit_site`, and two
  declared **kinds** — a `ratchet` (raised to exactly what the producer
  measured, with the reason beside the constant) and a `ceiling` (standing
  headroom, derived by `derive_ceiling(measured, percent, step)`). Plus a
  named `reviewed_exception` so an accepted overage is explicit rather than a
  silent limit bump.
- Every entry says **what the budget is paid for**, in the units a reader
  cares about: "Per request: full-profile skill index lines Hermes renders",
  "Eager ceiling, per request only if tool_search is off" (`drift.py:1731-1745`).
- `tests/test_drift_registry.py` keeps the index honest by checking that every
  declared site still contains the value it claims.
- It is the **budget ledger too**: the same report judges every per-turn and
  footprint budget, so one command answers "what did my change invalidate."

### The borrowing, concretely

Fleet already has the machinery; it lacks the registry.

1. **A `src/drift.rs` with the same two row types** — a metric (live producer,
   expected value, sites) and a budget (limit, limit site, `ratchet` vs
   `ceiling`). Fleet's producer for the token figure already exists: the
   `declaration_tokens` function the bounds test uses
   (`src/agent.rs:14206-14213`).
2. **A `fleet drift` CLI verb** beside `fleet doctor`, reporting every
   invalidated value in one pass with its sites — the thing OMH's twelve
   full-suite cycles paid for.
3. **Seed it with the budgets Fleet actually has**: `MAX_VISIBLE_TOOLS` (36),
   `MAX_DEFINITION_TOKENS` (10,800), `SKILLS_PROMPT_CAP` (60),
   `DELEGATION_CONCURRENCY` (3), `DEFAULT_MAX_TURNS` (100), the tool-router
   floors (`FLOOR` 0.68, `MARGIN` 0.02, `ACTIVATE_MAX` 5). Six rows would have
   caught the structure-guards drift the day it happened.
4. **A test that the doc and the constant agree**, in the style Fleet already
   uses for text scans: `include_str!` the guard doc, assert the constant's
   current value appears in it. That alone fixes the instance; the registry
   fixes the class.

Blast radius: a new module plus a CLI verb, both additive. The doc-and-constant
test is one function in `src/agent.rs`'s test block or a new `src/drift.rs`
test. Nothing existing changes behavior.

---

## 4. "Done" is a claim, not an evidence link (Medium)

### What Fleet does

`src/tools.rs:3555-3598` (`run_todo`) replaces the whole list, maps a status
word, and renders marks: `[ ]`, `[~]`, `[x]`. A `completed` item contributes
to the final `{done}/{total} done` line. Nothing about the mark is checked
against anything that happened. Fleet's system prompt asks the model to be
honest ("`Done` means every acceptance criterion is tool-verified",
`src/agent.rs:62`) and its reporting contract (`REPORTING_CONTRACT`,
`src/agent.rs:79`) structures the closing message — but the `todo` tool
itself accepts the word.

Fleet has one retry for the specific pathology of stopping without reporting:
`is_silent_stop` and the single `repairs` retry (`src/agent.rs:3628-3637`).
That catches *silence*. It does not catch *a false tick*.

### What OMH does differently

`src/plugin_bundle/omh/todo_evidence.py:1-40` makes the done mark resolvable:

> "A done mark on the plan todo is a declaration: it says done was claimed,
> not that anything ran. The continuation rule stops a plan when every item is
> done, so **a run that marked an item done over a failed command, or ticked
> several items off one command, stopped its own loop without the work behind
> it.**"

The mechanism:

- Each item's `done` carries a **typed reference** `{"kind", "ref"}` which
  must resolve against a record the host persisted for a tool call:
  `tool_call` (a `terminal` call whose recorded `exit_code` is `0`) or
  `file_write` (a write carrying `bytes_written`, or a patch with
  `success: true`, neither with an `error`).
- **The model does not supply the reference** — "Models do not reliably see
  tool call ids… so the `omh_todo` tool binds it from the records at the
  moment of the done write and **ignores any it is sent**." Each item has its
  own **window** (opened at its own last transition, closed by the done
  write), and the binding takes the latest evidence-capable call in that
  window that no other item holds — which is what prevents one command from
  closing five items.
- An item whose reference cannot resolve stays **`done_unverified`** rather
  than silently becoming done.
- `pr`, `ci_run`, and `team_check` are **accepted and stored, and resolve to
  nothing yet** — declared so every lane writes one vocabulary, and named as
  unresolved rather than faked.

That last choice is the honest one and the one Fleet should copy: a reference
kind that cannot be verified is *stored and marked unresolved*, not dropped
and not assumed.

### The borrowing, concretely

Fleet is better placed than OMH here, because Fleet's session log already is
the evidence base — hard rule 5 guarantees every tool call is in it, and
`src/session.rs` replays it. Two increments:

1. **Stamp the `todo` result with the calls it can see.** When `run_todo`
   processes a `completed` item, it is inside the turn and can be handed the
   tool results the loop already holds. Bind each tick to the nearest
   evidence-capable result in its own window, and render
   `[x] step (verified)` vs `[x] step (unverified — no passing command)`.
   The mark stays; the word stops being free.
2. **Make the stop criterion read the marks.** Today the `done` count is
   decorative text in the output. If the run's continuation rule ever reads it
   (OMH's does), it must read the verified count. If Fleet does not read it
   at all, the honest alternative is to say so in the tool's own description —
   but a tick that lies is worse than a tick that does not exist.

This is the smallest of the four borrowings and the one most aligned with
Fleet's stated thesis. It is also the one where Fleet's architecture makes the
job *easier* than OMH's: OMH had to reverse-engineer ids out of a foreign
host's sqlite; Fleet wrote its own log.

---

## 5. Three worth adapting, not adopting

**The two-axis evidence label.** `src/evidence/labels.py` renders a status as
`Phase · Confidence` — `Plan · not run`, `Code · reported done`,
`Test · verified` — and the reasoning (`:10-40`) is that collapsing the axes
is the failure the vocabulary exists to prevent: "`Code` alone cannot say
whether OMH watched an executor run or only wrote a prompt for one." Every
label on the phase axis is a **noun, never a gerund** ("Coding reads as
*someone is coding right now*… Code does not"), and the confidence mapping
**fails closed** — an unrecognized wire value renders as `not run`. `ready` is
excluded because OMH's own guard would reject it as an unbacked claim.

Fleet's status vocabulary is spread across surfaces — the TUI, the boards, the
calibration rows — and its `--conf` style already carries confidence in
places (judgment). The adaptation worth doing is narrow: **one place that maps
a wire value to what a person reads, failing closed**. Not the whole ladder.

**The `define`-time graph refusals.** Beyond `write_scope_overlap`, OMH
refuses a node whose prompt names none of five planning markers
(`dag_run.py:719-745`) and *advises* on graphs where most build work is one
chain (`serial_graph`, `dag_run.py:781-786`) or where no node verifies and no node
reviews (`no_verification_node`, `no_review_node`, `dag_run.py:787-794`). Fleet's
`plan_graph.rs` already has the refusal shape for prerequisites (`:537-539`)
and already computes `unserved` criteria (`:717`). Adapting the *advisory*
half — a graph whose nodes all serve nothing, or whose work is entirely
serial, or that declares no verification — is cheap and catches planning
mistakes before a dispatch.

**The routing prep chain.** `src/routing/query.py:1-25` builds one
`RoutingQuery` from a message — `raw`, `executable`, `scrubbed`,
`routing_text`, `normalized`, `tokens` — once, and every reader takes the
stage it needs: "Readers that need an earlier stage alone… are different
stages, not copies of this chain." Fleet's router judges per task through sys1
and caches on `src/route_cache.rs`, which is a different and stronger answer
for *model* routing. The borrowable idea is the smaller one: a prepared query
type so several readers (tool router, skill selection, the face's panels) do
not each re-derive the same normalization. This becomes materially more
valuable the moment finding #1 lands, because skill ranking and tool ranking
would then share a prepared input.

**Two to reject outright.** OMH's reasoning-effort axis (a model chain per
work category, each entry a `(model, effort)` pair,
`src/plugin_bundle/omh/hermes_delegation.py:50-120`) does not fit Fleet's
transport: `ChatRequest` (`src/api.rs`) carries `model`, `messages`, `tools`,
and `max_tokens` — there is no effort parameter to set, and `reasoning_content`
is display material Fleet deliberately ignores on the way back
(`src/api.rs:412-415`). And OMH's `test_sharding` apparatus
(`tools/test_sharding/`) solves a problem Fleet does not have yet. Measured
from the last recorded gate (`/tmp/gate-*.log`, 2026-11-19): the release build
finished in 4m42s, and the test phase then ran **2,410 lib tests in 58s** plus
172 integration tests in 24s — every other binary under 4s, for a total of
**2,624 passed, 0 failed, 11 ignored**. Nothing in that run was slow enough for
sharding to buy back its own complexity. Revisit when the *test* phase alone
(after a warm build) crosses ten minutes, not before.

---

## The one structural difference worth naming

OMH's central design bet is that **the agent it sits beside is untrustworthy
about what it did**, so the wrapper builds its own evidence base and never
accepts the agent's word. Fleet's bet is that **the harness is the agent**, so
its own loop is the evidence base.

Fleet's bet is better — it is why Fleet has memory, routing, calibration, and
a graph that OMH can only reach through hooks. But it has one blind spot that
this dive keeps landing on: when the harness *is* the agent, nothing forces
the harness to check its own claims. OMH cannot report a lane's success
without a receipt, because it does not own the lane. Fleet can report its own
skill library as "60 newest" and call that a selection policy, because nothing
between Fleet and Fleet asks for the receipt.

All four findings are the same finding from four angles: **the places where
Fleet grades itself are the places a second opinion pays.** Findings 1 and 2
are capability, 3 and 4 are honesty, and all four are cheap because the
machinery — the tree-sitter index, the tool router's floor-and-margin
geometry, the approval rule vocabulary, the session log — is already built and
already is the argument Fleet's README makes.
