# The idle cycle — housekeeping on spare capacity

Vision, 2026-11-15. Companion to
[the self-healing review](2026-11-15-self-healing-review.md), which
classifies healing by who may act; this doc asks the adjacent question:
**what should the harness do to the house when it has nothing else to
do, and how do we know it is working?**

The position: `spawn_homekeeping` is already a heartbeat — hourly, one
deterministic pass and one reading pass. The vision is to make it an
**idle cycle**: work that runs on spare capacity, yields the instant a
turn arrives, and resumes where it stopped. Ordered stages, one rate
budget, a ledger of what each stage decided, and a feedback edge that
tunes the gates that admit its own writes. The small models are not the
point; they are the instrument that lets a stage read a corpus of 900+
facts without a flagship model on the tab. The point is that the estate
gets measurably fitter from capacity that was already paid for,
unattended, and every action is reconstructable.

The second constraint is citizenship. A harness that works while nobody
is watching has the shape of the thing you least want on a machine: it
holds resources, it survives interruption, and it decides for itself
when to act. So the idle cycle inherits the run's resource contract and
**tightens** it — no owner to ask, no resume to rely on, nothing left
for someone to find later.

## Why idle and not nightly

A nightly job competes with a night that may hold a long run, wakes a
machine that may be asleep, and has one shot with a fixed budget. An
idle job spends capacity that is otherwise wasted, never contends with
the owner's work, and can spread a stage over as many gaps as it needs.

It also removes an assumption the nightly design was quietly making:
that housekeeping is safe to run for minutes at a time. It is not — a
turn can arrive mid-pass. So the idle frame forces two properties the
night frame hid:

- **Preemption.** Every stage is a cursor over a ranked queue, not a
  loop. A turn arriving mid-pass yields at the next item boundary, the
  cursor persists, and the next idle window resumes. No stage holds a
  lock across an item; every action is atomic on its own.
- **Starvation.** Under sustained load idle never comes. The house must
  not be quietly neglected by a busy owner, which is what every
  background scheduler is most likely to do.

## What exists today (the vision extends this; it does not replant it)

| Mechanism | Shape | Where |
|---|---|---|
| The consolidation cycle | hourly, **deterministic**: `compress` / `supersede` / `forget`, one summary fact per pass | `memory::spawn_consolidator`, `features/memory.md` |
| The dream pass | ≤ `DREAM_PASS_BUDGET` (8) inferred facts per pass, validated against their `source` transcript through sys1's `validate` head (`supported` / `contradicted` / `unverifiable`), auto-landing at `VALIDATE_AUTO_LAND_CONFIDENCE` (0.80) | `dream.rs`, `memory::dream` |
| The provenance permission model | `inferred` supersedable on justification; `confirmed-by` needs the Approver; `validated` is the pass's only product | `features/memory.md` |
| Calibration rows | every action appends a row with its verdict and its reason | `calib.rs` |
| The heartbeat | cycle + pass on one thread, once at start, then hourly, in chat / TUI / face | `spawn_homekeeping` |

That is a strong floor — and the hourly thread is already *shaped* like
an idle worker with the wrong trigger. It is also narrow in six named
ways.

## The gap, named

1. **Nothing checks a fact against another fact.** The dream pass reads
   a fact against its *source*; the corpus of 908 facts can contradict
   itself forever and no pass would see it.
2. **Nothing merges by meaning.** `supersede` retires on age and
   recall-count; two facts can say the same thing at full health
   indefinitely.
3. **No representation.** Recall is flat over the corpus. The
   "dreaming builds representations" half — the subject pages, the
   linked rounds the deep dive names as the article's centre — is
   unbuilt (`deep-dive-2026-11-07-memory-dreaming.md`).
4. **No injected-layer budget.** Facts are protected or not; nobody
   earns a line in the always-injected block, and nobody loses one for
   silence.
5. **The budget is a count, not a rate.** `DREAM_PASS_BUDGET = 8` facts
   is a shape, not a spend. Idle capacity has a *rate*, and nothing
   enforces one.
6. **The thresholds are opinions.** 0.80 was measured once, on a
   measured set, and nothing grades it again. A pass that is
   mis-calibrated stays mis-calibrated.

## The trigger: what idle means

Idle is a conjunction, held for a grace window so a starting run is not
cut into:

```
idle  ⇔  no run in flight
      ∧  no delegated child running
      ∧  no approval waiting on the owner
      ∧  no turn queued
      ∧  the grace window has elapsed
```

The hourly timer stops being the scheduler and becomes the **floor**: a
lower bound on how often an idle window is attempted, so a machine that
is never idle still gets a probe. The claim is atomic — the same
one-step claim the cron store already uses — so two processes cannot
both enter a pass. The probe itself needs a home, and the obvious one is
not available yet: `start_health_loop` has exactly one call site
(`gui/mod.rs:331`), so **the health loop is the face's** — chat and TUI
spawn homekeeping, reclaim and cron, and nothing else. Either the idle
cycle spawns its own probe, or health is hoisted out of the face. That is
the first prerequisite, not a detail: v0.8.6/0.8.7 put the substrate
auto-heal, the API probe, the lane handshake, the MCP probe and
`corpus::drift` inside that loop, so the two interactive surfaces are
missing five detections rather than one
([deep dive](deep-dive-2026-11-15-router-and-self-healing.md), H-1).

## The rate: spend capacity, never the owner's

- **`IDLE_RATE`** bounds the pass as a fraction of idle wall-clock, not
  a token count alone — the machine's heat and the owner's battery are
  as real as the invoice.
- A pass stops at the rate, **persists its cursor**, and the next
  window resumes highest-value first. Unspent work is **idle debt**,
  recorded and shown. A window that skipped the corpus says so.
- **Interactivity outranks housekeeping, always.** The disk reserve
  refuses a run before its first turn when space is low; the idle
  cycle mirrors it — it refuses to start a stage when a turn is queued,
  and yields at the next item boundary when one arrives. There is no
  setting where housekeeping may delay a run.

## Starvation is a first-class failure

Idle-triggered schedulers starve under sustained load, silently. So
every queued item carries its age, and two things follow:

- An item that has waited past `IDLE_STARVE_MAX` is **named on the
  board** as waiting. The owner sees a house that is not being cleaned
  rather than inferring it from a corpus that has drifted.
- Past a second bound, the highest-value stage is permitted to run in
  the **inter-turn gap** — still bounded, still preemptible, still at
  the rate — because between turns is also idle, just briefly.

## Citizenship: the resource contract, extended to unattended work

A run has an owner in the loop. An idle pass does not, which makes every
exemption the run earns unavailable to it. The contract is the same one
`agent.rs` already enforces — the reserve, the tree's ceiling, the
deterministic stop — with the asking removed and the disposal made
explicit.

**1. The same reserve, and nobody to ask.** A run that breaches the
reserve stops deterministically; an overage asks (`permit_past_reserve`).
The idle cycle has no one to ask, so a breach **stops the pass and
records the debt** — no prompt, no exemption, no "continue anyway." The
`IDLE_RATE` is a ceiling *under* the reserve, never a way around it, and
the pass reserves scratch headroom before its first item so the work is
not what fills the disk.

**2. It never wakes anything.** `wake_runtime` opening Kiri/Docker is
autonomous healing *for a run*, which is correct: the owner asked for
the run. For housekeeping it is not — waking a container to tidy a
markdown corpus is the host-abuse pattern precisely. If a stage needs a
lane and no runtime is up, the stage waits for a window where something
else woke it. This is why the stages are ordered the way they are:
clearance, demotion, and the link round read what is already on disk and
can run in any window; only `relate` and the subject page need a live
lane, and only they can be blocked by a sleeping machine.

**3. It owns what it opens, and yields by returning.** Every acquisition
— lane session, MCP child, scratch directory — is a scoped handle
closed on drop. The forgotten path is the yield, not the error: a pass
that yields **returns** rather than unwinding, so it closes as it
leaves, and the cursor it persists is data, not a held resource. At
window start a reaper sweeps children tagged with a previous pass id —
the same liveness read `processes.rs` already uses (`ps stat`, zombies
counted as finished) — because a crashed pass is exactly the one that
leaves something behind.

**4. It leaves no residue.** Scratch lives at
`$FLEET_HOME/idle/<pass-id>/`, swept at the end of the window and again
at the start of the next. Nothing the cycle creates outlives it in
`/tmp`, no daemon outlives the window, and **no new listening port is
ever opened** — the face's loopback rule is the model: the address is
the whole authentication, and a housekeeper that listens is not a
housekeeper. Route-cache entries it writes are budgeted by the namespace
they already have, not grown without bound.

**5. It reports its delta, and the delta is the test.** One line per
window that did work: tokens, wall-clock, peak RSS against
`MEMORY_CEILING_KIB`, process count against `PROCESS_CEILING`, disk
delta, children opened and closed. `footprint.rs`'s 30-day standing
reading then sees the cycle as part of the tree, so growth shows up
where growth already shows up.

The falsifiable form, which is the point: **twenty idle windows against
a fixture home with nothing to do must produce a net disk delta within a
bound, zero orphaned children, zero new ports, and a process count that
returns to baseline.** If a housekeeper cannot pass that, it is not
housekeeping. It belongs in `eval.rs`'s corpus as its own family — it
costs no model call, and it catches the failure the metaphor invites.

One debt this pays off on arrival, stated accurately because the earlier
review overstated it: cache bloat is serviced by an external
`daily-maintenance.sh`, **and** `cache-sweep.sh` is already invoked from
`src/` when a writer hits ENOSPC (`health.rs:596-605`). So the sweep
exists and reacts to the worst case; what stage 1 adds is a schedule, so
the sweep stops depending on a failure to happen.

## The steward: the cycle's principal

The cycle is not anonymous background machinery. It is **a principal** —
the thing `identity.rs` calls "the unit of authority an action acts for"
— and it has a role file like every other persona. Its charge is not
memory housekeeping; it is **keeping fleet fit**: metering the estate,
tending what is reversible, and telling the owner what is not. The corpus
is one of the things it keeps, not the thing it exists for.

This is not a naming convenience; it is where three problems dissolve.

**1. The role file is already a citizenship contract.** `roles.rs` gives
a persona exactly the four knobs the contract needs: `tools` (what it
may touch), `shapes` (which recall partitions it may read), `workload`
(which routing chain its runs walk, so it cannot route itself up to
`hard`), and a bounded overlay — `OVERLAY_CAP` is 4,000 bytes because
"a role is standing instruction, not a document." A housekeeper whose
blast radius is declared in one auditable file, in the same format as
`clerk` and `tower`, is a housekeeper an owner can actually govern.

Proposed `roles/steward.md`:

```
---
name: steward
description: Keeps fleet fit — meters the estate every window, tends what
  is reversible, and tells the owner what is not. Owns its tasks, its run,
  and its cleanup.
tools: read_file, list_dir
skills:
shapes:
workload: housekeeping
---
You are steward, fleet's housekeeper. You keep the harness in condition
rather than the corpus tidy: you meter first, and you judge one item at
a time. You run on spare capacity, you never wake anything, and you leave
nothing behind. You hand the verdict back; the harness owns the loop, the
verb, and the write. You never author a fact's body, you never delete,
and you never touch a fact the owner confirmed. You may not write a number
you are graded on. Your only unprompted word is the starvation line, when
the house is not being kept.
```

The smallest tool set of any persona is the point. **The one persona
that runs while you are not looking should have the least power, not
the most** — the steward reads, judges, and reports. Every write it
causes goes through a path the harness owns (`Memory::supersede`, the
clearance verbs), so the verbs carry the permissions and the persona
carries the accountability.

**2. Authority makes the virus structurally impossible.** As a
non-owner principal the steward cannot act with the owner's authority,
cannot retire a `confirmed-by` fact, and **can never hold a latched
approver** — a `--yes` on an unattended persona is the host-abuse shape
exactly, and the authority model already has the vocabulary to refuse
it. Where a run's overage asks the owner (`permit_past_reserve`), the
steward's asks have no one to address: they stop. That is the whole of
its privilege: it may spend what is spare and it may not ask for more.

**3. Ownership makes citizenship attributable.** The triad, literally:

- **Owns the tasks** — the queue, the cursor, and the idle debt ledger
  are the steward's, per principal. A pass that skipped the corpus is
  the steward's debt, not an anonymous app's.
- **Owns the run** — `processes.rs`'s registry is already per-run, so a
  steward run's children are scoped to it; its spend cap follows
  `delegate.rs`'s beat pattern (a daily USD ceiling, sized on probed
  rates, reserved per ask); its workload is pinned in the role file.
- **Owns the cleanup** — the reaper at its next start is *its* reaper,
  and a leaked child is a row *against the steward*. Precision scores
  include citizenship failures, so a persona that leaves residue is a
  persona whose stages lose priority. A leak stops being an invariant
  someone has to notice and becomes a defect with a name on it.

Two refusals guard the personification, because a persona invites the
wrong instincts:

- **It is a principal, not an agent.** The steward owns the pass; the
  harness owns the loop. Giving a 1.7B an agent loop over the corpus
  re-imports the recovery measurement the repo already made — 29.3%
  against the loop's 54.2%. The overlay is standing instruction, not a
  conversation.
- **It never chats.** No housekeeper window, no "ask steward," no
  greeting. It speaks once, unasked, and only about starvation.

## Wide eyes, small hands

A role that keeps fleet fit must *see* everything and *touch* almost
nothing. Three rules make that coherent rather than contradictory.

**The verbs are the harness's, not the persona's.** The steward does not
get a shell in order to, say, re-run the local gate on spare capacity —
and that is a genuinely good use of an idle window, since CI is disarmed
by budget and the gate is manual by design. So the capability lives in a
named harness verb (`fleet gate`, `fleet memories --validate`,
`fleet idle sweep`), each with its own resource contract, its own bound,
and its own row. The persona requests a verb; the harness executes,
measures, and cleans up. Capability moves into the harness, which is
where the contract already lives.

**Recall is broad; action is narrow.** The role's `shapes` stays empty —
every partition — on purpose. Seeing everything is what lets it meter
the estate; the risk is in the hand, not the eye. `tools:
read_file, list_dir` is the whole grant, and it grows only by a ruling
with a reason on file.

**It may not write the numbers it is graded on.** Every meter reads a
record the steward does not produce: calibration rows come from the
beats, footprint from `footprint.rs`, gate results from the gate, recall
hits from the recall path. This is the structural defence against
Goodhart — a housekeeper that can improve its own score by writing it is
a housekeeper that will. The steward's only writable outputs are the
queues, the cursors, and its own findings.

## The recommendation, not the repair

The steward does not heal. It **recommends**, and the healing components
of fleet consume the recommendation. That is the difference between a
meter with opinions and a member of the repair layer — and it is also
what makes the whole thing governable, because the recommendation is a
record with a name on it, not an action with an alibi.

**A recommendation is a typed record, addressed to a consumer.** Not
prose, not a paragraph in a journal:

```
{
  dimension:  "freshness" | "routing" | "surface" | …,
  subject:    the thing that is wrong, by its own id (path, KI, lane, key),
  repair:     a name from the healing catalog — never a free-form action,
  evidence:   the records read, by reference, so the claim is checkable,
  class:      the catalog's registration — never the model's,
  lane:       { task, head, noul, confidence } — the second signature,
  effective:  the stricter of the two,
  state:      raised | healed | asked | answered | rejected | suppressed
}
```

**The catalog owns the class.** The steward selects a repair *by name*
from the self-healing review's own catalog, and the catalog carries the
class. The model never assigns one. This is not a stylistic rule: a
model that can label its own action "autonomous" has converted the gate
into a suggestion, and a 1.7B's confidence is not a permission. The
closed set also keeps the judgment inside the measured envelope — the
steward is not choosing among consequences, it is naming one of a few
dozen registered repairs against evidence it read.

As of v0.8.7 that catalog is **complete** — every candidate in the
self-healing review is shipped or explicitly by-design — so every name the
steward can select resolves to a real repair. That is what makes the
closed set a set rather than a wish list, and it is why this design got
cheaper this week rather than more speculative.

**Three outcomes, one per effective class** — the stricter of the
catalog's registration and the lane's verdict (below):

- **autonomous** → the healer acts in the window the recommendation is
  raised. The recommendation *is* the trigger; the health loop, the
  consolidator, and the reaper are its consumers.
- **approved** → it becomes an ask at the approvals seat (an inbox card
  naming the repair, the subject, and the evidence), and the owner
  presses. It never becomes a chat message.
- **destructive** → a human, always, with the card saying plainly why it
  cannot be undone. There is no confidence high enough to skip this.

**Identity, dedupe, suppression.** Every recommendation carries a stable
key — dimension, repair, subject digest — so the healer cannot act twice
on one finding, and a rejection **suppresses that key for a stated
number of windows**. A refused recommendation that returns every ninety
seconds is not diligence; it is a process abusing the one resource the
contract never mentions. **Attention is the scarcest resource on the
machine**, and the citizenship rules apply to it: at most one ask per
dimension per window, no two asks sharing a subject prefix, and nothing
re-raised while its suppression holds.

**It is graded on outcomes, not activity.** A recommendation the healer
executed that moved its meter in the right direction is a hit. A
recommendation rejected, or executed with the meter flat, is a miss.
That is the number that tunes the steward's thresholds — and it is the
only defence against a housekeeper that reports a great deal and fixes
very little, which is what an ungraded recommender always becomes.

## The second signature: the lane's verdict

One party should never certify its own proposal. So a recommendation
carries **two claims about danger, from two parties**, and the effective
class is the stricter of them:

- **the catalog's class** — a static claim about the *kind* of repair:
  this registered repair is, in general, autonomous.
- **the lane's verdict** — a dynamic claim about *this instance, now*:
  applying it to this subject would, or would not, destroy something no
  other thing carries.

The gap between them is the point. Demoting a fact is autonomous in
general; demoting the one fact that is the sole evidence for another
fact's justification is destructive *here*. A static class cannot see
that. A per-instance verdict can.

**The primitive is the typed head, not a prompt.** sys1's `task_spec`
already carries both shapes the job needs: `choice` (2–255 options) for
anything categorical and **`noul`, a yes/no head whose value is P(yes)
from the distribution**, with `confidence` as the margin. So the safety
question is one noul head per recommendation — inside the measured
comfort of "two to three heads per call," and nowhere near the 27-head
collapse.

The question has to be answerable by a small lane, which means concrete
and instance-shaped, never "is this safe?":

> *Would applying this repair lose information that no other alive fact
> carries?* → noul = P(yes) is the destructive probability.

**It fails closed.** No verdict, a `verdict: "fail"` envelope, a margin
too thin to read, or a noul above `STEWARD_NOUL_FLOOR` — each of those
escalates. An abstention is not a permission, and the floor is the one
number in this design that is deliberately pessimistic until its record
earns better.

**The ratchet is asymmetric on purpose.** The lane may **tighten** a
class — autonomous to approved, approved to destructive — and it may
**never loosen** one:

| Catalog | noul | Effective |
|---|---|---|
| autonomous | below the floor, margin clear | autonomous — the healer acts |
| autonomous | above the floor | **approved** — the instance beat the static class |
| approved / destructive | anything | unchanged — the stricter already holds |
| any | absent, failed, or thin | **approved** — escalate |
| `certifiable` | none owed | autonomous — the precondition is code |

One case carries no second signature: an entry the catalog marks
`certifiable`, whose safety is a **deterministic precondition** rather
than a judgment — disk below the standing threshold, the sweep's own
cooldown passed, build artifacts only. No model input exists in that
decision, so no model verdict is owed, and asking for one would be
theatre. The rule this preserves: **the second signature is for
judgments, not for arithmetic.**

A false destructive costs one ask. A judge that could promote an
approved action to autonomous would be a vulnerability dressed as a
safety layer. Asymmetry is what makes the second signature worth having.

**The recommendation is itself a claim, so it is judged like one.** Three
axes, two of which are available today and one of which is not:

- **relation** — what the pair is: `same` / `contradicts` / `unrelated`,
  on the registered `validate` head with the peer fact as the evidence;
- **warrant** — does the evidence support *this* repair? The
  recommendation is a claim and the records the steward read are its
  transcript, so the same registered head answers `supported` /
  `contradicted` / `unverifiable` over it;
- **safety** — the `steward_safety` noul, absent until upstream lands.

The write path requires relation **and** warrant to agree, from two asks;
the safety axis resolves to escalate until it exists. That is the fanout,
and it is bounded: **two judges per item, 2–3 heads each, paid only where
a write would land** — the read path takes one ask. Disagreement is never
averaged: it is a row and a named skip. Cross-vendor pairing is worth
paying for as a **disagreement detector**, not a better scorer — the
cross-vendor transplant measured identical to control, so a fanout buys
doubt, not accuracy, and it is priced accordingly.

**Different failure domains, in series.** Five links, none of them able
to cover for another:

1. the **recommender** names a repair from a closed catalog;
2. the **catalog** carries the static class, authored by a human;
3. the **judges** rule on the instance — relation and warrant, two asks,
   on a different vendor's model from the recommender's — and must agree;
4. the **ratchet** takes the stricter of the two, and cannot be talked up;
5. the **owner** owns destruction, with the evidence and the reason it
   cannot be undone.

**The upstream channel, since we own it.** sys1's core files are
byte-exact copies in fleet, and a task name must be registered — so the
change goes where it belongs: a first-class `steward_safety` task
upstream, then a new vendored cut. That is better than riding an
existing task's prose, because a registered task carries typed heads:

```
task: steward_safety
  destroys : noul            — P(yes) to "would this repair lose
                               information no other alive fact carries?"
  why      : choice [sole_evidence | cross_reference |
                     provenance_loss | orphan | none]
```

Two heads, both inside the measured comfortable range — and one
discipline: an upstream task is a **contract**, so it lands with its
heads, a version, and no growth by convenience. Head creep is the
27-head collapse arrived at politely, one reasonable addition at a time. And note what
the second one buys: **the ask's explanation is a closed label, not
generated prose.** The inbox card reads "retiring this would lose the
sole evidence for *X*" from a `why` value, so a justification cannot be
hallucinated by anything — it is selected from five options and the
evidence pointer rides beside it.

**The three repos share the work, and it lines up.** `sys1` gains the
task and its heads; `dev-decisions` gains the row vocabulary for a
`steward` dimension (catalog class, lane noul, effective, outcome) so
the tightening and conversion rates are gradeable the way every other
beat is; fleet gains the workload row, the ratchet, and the
recommendation record. No repo gets a private dialect.

The probe still happens — a new task id and head names on the wire are
probed before anything depends on them, the way `lfm3-1.7b` was. The
difference is that a failed probe is now a bug to fix upstream, not a
design constraint to live with.

**It is measured like everything else.** Both verdicts are on the row, so
the tightening rate is visible — how often the lane overruled the
catalog — and so is the ask-conversion rate: of the asks it raised, how
many the owner approved. A judge that never tightens is noise; one that
tightens wrongly moves its own floor.

## The stages: meter first, then the tenders

Fitness is a scorecard before it is a repair. Stage 0 measures the
estate every window; the five tenders turn what it finds into
recommendations — never repairs — in the order that keeps the machine
healthy rather than the order that keeps the corpus tidy.

The class each stage names below is the **catalog entry's** class, never
the steward's judgment: the stage selects a repair by name and the class
comes with it.

### 0. Meter — no model, every window

Deterministic, cheap, and the reason the role is real. Four of the eight
readers already exist and are merely unscheduled: `anomalies::pass`
(six deterministic classes — CLI-only, and it throws its report away),
`lifecycle::sweep` (unused tools/skills/loops/workflows, daily 02:30),
`demand::collect` (families, rates, trends from calib), and `footprint`
(tree ceilings plus the 30-day standing reading). The deep dive's Part 3
maps each dimension to one. Each is read from a record the steward does
not write:

| Dimension | Read from | A failure looks like |
|---|---|---|
| **Grounding** | calibration rows, the judge beats | a turn whose claim did not stand on a computed fact |
| **Recall** | recall-hit rows, the `recall-count` distribution, the injected layer's tokens per turn | facts nobody recalls occupying the layer; a layer that grew without earning it |
| **Freshness** | `known-issues` ages, plan statuses against landed commits, doc mtimes against the code they describe | a comment naming `LFM2.5-24B` while `lfm3-1.7b` ships |
| **Routing** | probe rows, demotion expiries, failover rates | a candidate that has failed since Tuesday and still sits first in a chain |
| **Surface** | `lifecycle.rs`'s unused-tool and unused-skill scan, `mcp_registry` probes, lane opens | a configured MCP server nobody can reach, invisible until a run tries it |
| **Gate** | the local gate's last result, the ignored tests, eval-corpus coverage | CI disarmed by budget, so a red gate nobody was watching stays red |
| **Citizenship** | `footprint.rs`'s lines, orphans, ports, cache size | the delta the fixture bounds |
| **Cost** | spend by workload from the beat caps | housekeeping's share of the quota creeping |

A dimension with no reader is itself a finding. The meter's output is
one row per window and one aging finding per dimension that moved the
wrong way — never a chart, never a prompt.

Dial `quick` exists for a reason: the meter is a judgment of *shape*,
and the cheap chain is enough to say which dimension moved. The tenders
that need to read text walk the prose leg.

### 1. Clearance — no model, ever

Deterministic, idempotent, reversible: dangling `superseded-by`,
recall-graph edges pointing at retired names, frontmatter fields that
never got filled (`shape`, `phrases`), facts whose `source` names a
session no longer on disk, archive serials, rotated logs, pruned
checkpoints, the cache sweep the `scripts/` still hold, stale doc
claims by git mtime.

**Class: AUTONOMOUS.** The largest stage by file count, the cheapest by
token, and the only one that is safe to run in a 200ms gap. If a model
is ever proposed here, the proposal is wrong.

### 2. Replay — two judges, both 2–3 labels

Today it is one judge (`validate`, fact vs. source). It becomes two:

- **`validate`** — unchanged, against the transcript.
- **`relate`** — **new**: a *pair* of facts, three labels —
  `same` / `contradicts` / `unrelated`, with a confidence.

Both are **registered sys1 tasks with typed heads, not prose prompts**.
`relate` is a `choice` head of three options — exactly the primitive's
2–255 range — so the pair judge is d1's work, not the text leg's, with
the LFM leg as the named fallback. That also keeps the measured
hierarchy intact: d1 kept the judgments it measured well, and the
rejections were against *selection over a catalog*, which a pair is
not.

Pairs are never enumerated. They come from `sem1`'s shape-first
candidates and the recall graph's existing edges, ranked by recent
access, then capped. The lane's own measurement is the design
constraint: **two to three heads per call is the comfortable range; 27
ordinal heads collapse** (`TECHNICAL-DOCUMENTATION.md`) — so a pass is
one pair, one question, one verdict, and the fan-out happens across
items, not inside a call.

The judged shape must stay inside the measured envelope. That is why
`relate` is a pair decision and not "find this fact's duplicate":
selection over a catalog measured 20.2–30.3% (LFM3) and 28.6% (d1), and
both were rejected on measurement. Pairs are what survived.

### 3. Consolidation — the writes, gated by provenance

`relate`'s verdicts become actions through paths that already exist:

- `same` above the bar → `Memory::supersede` with the two facts' names
  and the verdict as the justification. Exactly one survivor; the
  retired fact keeps its file and its stamp.
- `contradicts` above the bar → retire the **inferred** one through the
  same path, the session and the verdict in the justification. A
  `confirmed-by` fact is never retired here — it waits for the owner,
  as the permission model already rules.
- `unrelated` → a row and a named skip.

**Class: APPROVED for merging two facts, AUTONOMOUS for retiring an
inferred contradiction** — the second is already slice 12's rule, and
this stage only supplies a better justification than age.

### 4. Representation — the half that is unbuilt

Two products, different risk classes:

- **The link round** (cheap, fan-out): for facts that share a shape
  partition but no edge, ask the 2-label question `related` /
  `unrelated`. A yes writes an edge, not prose. Edges are reversible,
  inspectable, and cost one token to judge. Surviving edges come from
  **agreement** — see the economics below. This is the stage that runs
  best in many short windows, because an edge needs no context.
- **The subject page** (bounded, prose): one page per recurring shape
  cluster, authored by the hosted text leg (`lfm3-1.7b`, the probed
  default in `generate.rs`), over facts the harness selected. Prose is
  the one thing the small model is trusted with today — compaction
  folds, titles, archive summaries ride it in production — and the page
  body is exactly that shape: supplied facts in, one page out, checkable
  line by line. A page is the one stage that prefers a long window, so
  it takes the largest one available.

**Class: APPROVED.** A page is content; it lands as a proposal and
applies through the gate. The refusal is named: **no model writes a
fact body.** The measured precedent is the article's 413-file failure,
and the corpus is not going to repeat it.

### 5. Demotion — the waking check

The always-injected layer has a size and a rate. Each pass: rank the
layer by provenance (`PROVENANCE_VALIDATED` 0.75 and up), by
recall-count earned in the window, and by whether a summary already
carries the content. Facts that have not earned a line are **demoted** —
moved to a linked summary, still recallable, still on disk.

**Class: AUTONOMOUS.** The least destructive action in the program (a
demoted fact is one link away) and the highest-leverage one, because the
layer's cost is paid on every turn.

## The economics: cascade across items, stack only on writes

The instinct is to run several models in parallel on every item. The
measurement says otherwise, and the design follows it:

- **Fan out across items.** An idle window's work is many cheap
  judgments, one item each: ~200 pair probes at a 700M-class call is a
  rounding error against one flagship turn. Breadth is bought with
  count, not with heads.
- **Stack only where a write happens.** Two independent judges on the
  `same` / `contradicts` path — the hosted small leg and a d1 head, or
  two LFM sizes — and the action lands only on agreement above the bar.
  Disagreement is not averaged; it becomes a row and a named skip. This
  is where cross-vendor diversity is worth paying for: not as a better
  scorer (the cross-vendor transplant measured identical to control),
  but as a **disagreement detector**.
- **Never fan out across a catalog.** The rejection is on file twice.
- **The budget is a rate.** `IDLE_RATE` bounds the spend as a share of
  idle capacity; the debt is visible, which is what keeps the program
  honest.

## The router work: a chain, not a constant

Today the small-model leg is invisible to the router, twice over.

`generate.rs` carries `TEXT_MODEL = "lfm3-1.7b"` as a constant and
passes it into blend1's `generate` tool; blend1 calls Liquid with its own
probed default. So the leg that already writes the harness's folds,
titles and archive summaries has **no chain, no failover through
`route::select`, no per-workload spend row, and no calibration row naming
the model that served.** Meanwhile every judged workload — `quick`,
`standard_code`, `frontend`, `hard`, `prose`, `deep_context`, `composed`
— walks flash and mid tiers. None of them names a 1–2B leg. The prose the
housekeeper needs is written by a model the router has never heard of.

Four changes, in the order they matter.

**1. A `housekeeping` workload, small-first.**

```
liquid/lfm3-1.7b → liquid/lfm2.5-24b-a2b → deepseek/deepseek-flash (the tail)
```

Two roster facts this row depends on, both verified: `liquid` declares
only `lfm2.5-24b-a2b` today (`route.rs:534-550`), so `lfm3-1.7b` must be
declared before the chain can walk it; and nothing validates a model
against the declared list, which is how `generate.rs`'s constant gets
away with naming an undeclared one.

It does **not** join `JUDGED_WORKLOADS`, and does not need to: that list
is the closed set the upstream `route` task can answer, and a workload
outside it is reachable by a pin — which is exactly what the steward's
role file is (`workload: housekeeping`). So this row needs no upstream
change at all, unlike the safety head, and the difference is worth
noticing on the way past.

**2. Deferrable work never escalates.** The router has one failure
policy today: walk the chain to the tail, because someone is waiting on
the other end. Idle work inverts it. A fold not folded, a title not
written, a page not generated — each is fine tomorrow. A flagship turn
spent on a changelog line is not. So the workload carries a **ceiling**:
candidates above the cheap class are never walked, and an exhausted
chain **defers** — records idle debt and returns. Interactive work
fails over; idle work fails down, defers, and retries. That is the
policy difference that makes unattended work affordable, and it belongs
in `routing.json` beside `allow_api_models` and `local_tail`.

**3. `generate.rs` asks the router.** Replace the constant with
`route::select("housekeeping")`, pass the resolved id into the tool's
`model` argument, and write the decision row every other walk already
writes. The served model then appears in the session record and the
calibration stream — which is what makes this leg's cost and quality
measurable, and today neither is.

**4. A daily cap in the beat's shape.** `delegate.rs` sizes a $0.05/day
ceiling on a probed rate and reserves it per ask. Housekeeping follows
the pattern: a cap sized on the window's plausible volume, reserved
before the call, and the debt line when it is reached.

Two symmetries make this cheap. The ceiling is the router's version of
the resource contract's reserve — spend what is spare, never ask for
more — and the pin is the same mechanism the orchestrator already uses
for `hard`. The steward's chain is then declared in one place and
enforced by machinery that exists.

## The edge that makes it improve

Every stage already writes calibration rows; the idle cycle adds one
thing — **the rows are graded, and the grades move the gates.**

A merged fact that is recalled and re-validated later is a merge that
worked. A demoted fact that is recalled twice and re-promoted is a
demotion that was wrong. A `same` verdict that a later session
contradicts is a false positive with a date on it. Those outcomes are
already the row vocabulary's business; the cycle's job is to feed them
back so that each stage's threshold — 0.80 today — becomes the number
its own record justifies. That is the difference between a housekeeper
and a housekeeper that learns its own taste.

It also gives the program a stopping rule. A stage whose precision sits
below its cost keeps getting demoted in priority until it is off. The
passes compete on their own record, not on their ambition — and that is
the only reason to let one run unattended.

## Governance, per action and per principal

The self-healing review's four classes, applied to recommendations
rather than stages — one stage can raise all four — with the steward's
id on every row, the class carried by the catalog entry, and the
effective class set by the ratchet against the lane's noul:

| Action | Class | Why |
|---|---|---|
| Clearance sweep | AUTONOMOUS | idempotent, reversible |
| Stamp `supported` past the bar | AUTONOMOUS | an upgrade; a fact that stays inferred is unchanged |
| Merge `same` past the bar | APPROVED | two facts become one; content survives in the retired file, but the corpus changed |
| Retire an inferred `contradicts` | AUTONOMOUS | slice 12's rule, better justification |
| Touch a `confirmed-by` fact | APPROVED | the permission model already says so |
| Write a subject page | APPROVED | content; proposal until pressed |
| Demote from the injected layer | AUTONOMOUS | one link away, still on disk |
| Run the local gate on spare capacity | AUTONOMOUS | read-only effect on the tree; build artifacts share the sweep |
| Delete anything | never | nothing is deleted; archived and diffable |

Two hard refusals sit above the table. **No model authors a fact body**
— the write path refuses `validated` for the same reason: a write that
claims what it did not do forges the one basis the corpus earned. And
**no pass edits what another pass touched** — a fact changed by a pass
is frozen for a stated number of windows, so the program cannot thrash
against itself.

A third sits under them, and it is the one the principal exists for:
**every action names the principal that took it.** An idle action that
cannot name its principal is an action the owner cannot govern, and a
row without one is a row the calibration stream cannot score against
the persona that caused it.

## What the owner sees

Nothing, by default: no toast, no modal, no asked question. One journal
line per window that did work, one board row (stages run, items judged,
actions taken, idle debt, thresholds in force), and `fleet idle` for the
owner who wants the transcript — plus the starvation line, which is the
one thing the cycle is allowed to say without being asked.

Proposals surface the way skills proposals do: lazily, when the
conversation is already about that fact, never as a queue to clear.

## Phases, with the bar for each

P0 lands the principal before any stage runs — the role file, its id,
and its row attribution — because every later phase's evidence carries
that name, and retrofitting it would invalidate the measurements P0
exists to take.

| Phase | Delivers | Passes when |
|---|---|---|
| **P0 — meter and baseline** | the scorecard's eight readers wired read-only, the recommendation record's schema (dimension / subject / repair / evidence / catalog class / lane noul / effective / state) with its dedupe key, `steward_safety` upstreamed, cut into fleet, and **probed** live, the `housekeeping` workload row with its ceiling and daily cap, `generate.rs` bound to it, plus the existing `validate` pass replayed over a labeled fixture of 50 facts; cost and wall-clock per window recorded | every dimension has a number from a record the steward does not write; a recommendation is a typed record the healers can consume; the safety task answers on the wire; the prose leg's model, spend and served-model row come from the router; precision and cost are known numbers, not estimates; the fixture is committed |
| **P1 — clearance** | the deterministic sweep as its own stage, with a `clearance` calib row per action | zero model calls; every action reversible; a fixture proves idempotence (two passes, no second change); **and the citizenship fixture passes** — twenty empty windows, net disk delta within bound, zero orphans, zero new ports, process count back to baseline |
| **P2 — `relate`** | the pair judge as a registered sys1 task (`choice`, three options), d1 primary with the LFM leg as the named fallback, agreement-stacked on writes only, proposals out | precision on the fixture's known pairs beats the trivial baseline by a stated margin, and disagreement exceeds a second stated bound |
| **P3 — merge path** | `same` and inferred-`contradicts` land through `Memory::supersede` | a merged pair survives re-validation by the dream pass; a retired contradiction names its verdict in the justification |
| **P4 — demotion** | the injected-layer budget | the layer's token cost drops by a stated fraction with no measured recall loss over the window |
| **P5 — representation** | link rounds and subject pages | the round's edges are more useful than the recall graph's own, and a page survives review line by line |
| **P6 — self-tuning** | thresholds move on their own grades | each stage's gate is a number its record justifies; a stage below its cost goes quiet by itself |

P0 through P2 are the whole program's proof. If a three-label pair judge
cannot beat a trivial baseline on 50 fixtures, the rest of the vision is
the Borges map: a representation larger than the thing it abstracts.

## Named rejections

- **A model that selects from a catalog.** 20.2–30.3% (LFM3) and 28.6%
  (d1) on the same shape; the 27-head collapse. Pairs only.
- **A pass that rewrites fact bodies.** The 413-file failure; the
  corpus is not a draft to be re-edited.
- **An ensemble on every item.** Cost with no measured benefit; the
  stack belongs on writes.
- **A pass without a calibration row.** Rule 5's invariant, applied to
  spare capacity: an action that cannot be reconstructed did not happen.
- **Unpinned model ids.** A provider renaming a model under a stable
  name would silently change the housekeeper; ids are pinned, and the
  fallback is named.
- **Deletion.** Archive, stamp, demote — never delete.
- **A clock as the trigger.** The timer is the floor, not the
  scheduler: a housekeeper that runs while the owner is working is a
  housekeeper that delays the owner.
- **A stage that wakes a sleeping runtime.** Waking a container to tidy
  markdown is the host-abuse pattern. Housekeeping takes the machine as
  it finds it, or waits.
- **Anything that outlives its window.** No daemon, no child, no scratch
  file, no port, no cache entry without a TTL. A pass that leaves
  something behind has not finished, whatever its journal says.
- **A latched approver on an unattended persona.** `--yes` belongs to an
  owner who is present. A steward that can answer its own destructive
  question is the host-abuse shape with a name on it. Note this is a
  ruling on a mechanism that exists: `cron::Job` already carries
  `auto_approve: bool` (`cron.rs:656`) and a job without it has its calls
  skipped (`cron.rs:613`). The default is safe; the steward inherits the
  same shape and may never hold the flag.
- **A persona with power it does not need.** The steward's tool set is
  the smallest of any role, and it grows only by a ruling with a reason
  on file.
- **A model that labels its own action safe.** The class belongs to the
  catalog entry; a recommender that grades its own consequences has
  made the gate a suggestion.
- **A recommendation that repeats.** A suppressed key that re-raises
  before its window is the harness nagging its owner — attention is a
  resource, and the contract covers it.
- **A verdict that can loosen a class.** The lane may tighten and may
  never loosen; a second signature that can promote an approved action
  is a vulnerability wearing a safety layer's coat.
- **An abstention read as permission.** A thin margin, a failed
  envelope, or a missing noul escalates. The floor is pessimistic until
  its own record earns better.
