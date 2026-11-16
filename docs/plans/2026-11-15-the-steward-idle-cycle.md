---
title: the steward — an idle cycle that reads, meters, recommends, and escalates
date: 2026-11-15
status: draft
---

# The steward — an idle cycle that reads, meters, recommends, and escalates

The owner's direction, in seven rulings:

1. **"Housekeeping on spare capacity"** — the harness works when it is not
   busy, yields the instant a turn arrives, and resumes where it stopped.
   Named by capacity, never by a clock.
2. **"A persona (bot) owns the tasks, the run and the cleanup — the
   housekeeper."** The cycle is a principal, not anonymous machinery.
3. **"It keeps fleet in tip-top performance"** — the charge is the whole
   estate, not the memory corpus.
4. **"It provides actionable recommendations to the self-healing
   components, and escalates destructive actions to a human."** It does
   not heal; it recommends, and the class travels with the repair.
5. **"A suggestion must be paired with a sys1 judgment or a noul verdict
   of safe/destructive"** — with checks and balances, so no party
   certifies its own proposal.
6. **"If this is a housekeeping role, why wouldn't it call a model to
   begin with — especially if one is already available."** It does. A
   steward that only counts files is a `wc -l` with a persona.
7. **"You can pick any LiquidAI model that is available and not set up
   yet."** The seats are named below, and the mechanism that admits a new
   model is the roster plus a workload — never a vendored lane's
   allow-list.

The architecture is [the idle cycle](../architecture/2026-11-15-idle-cycle.md);
the ground truth is [the router/self-healing deep dive](../architecture/deep-dive-2026-11-15-router-and-self-healing.md).
This plan is **slice 1: the loop's spine, with real reading on the model
fleet already pays for.**

## Three seats, and which one is missing

Every recommendation carries two claims, answered by different parties.
The seats:

| seat | work | model today | state |
|---|---|---|---|
| **judgment** | the pair question (*is this fact supported, contradicted, or unrelated to that one*) and the warrant question (*does the evidence support this repair*) | sys1's registered `validate` head over the lane roster (d1/SystemOne, the same `LIQUID_API_KEY`) | **exists** |
| **prose** | the ask's one-line explanation, and later the subject pages | a Liquid LFM text model through a router workload | **half exists** — `generate.rs` runs `lfm3-1.7b`, but outside the router and behind a two-model allow-list |
| **safety** | would applying this repair destroy something no other thing carries | `steward_safety`, a new noul head | **missing** — upstream |

That is the honest version of the earlier scope: **the model is in from the
first window**; what is missing is one *head*, not a model. So the shape is
— **the model reads and proposes; the catalog classifies; a code
precondition may act; the owner writes what is content or destructive** —
and no write lands on a work verdict alone.

## The model seats, chosen

The owner's ruling opens the roster, so the seats are picked rather than
inherited.

- **`liquid/lfm2.5-24b-a2b`** — already declared (`route.rs:534-550`),
  already probed, and the largest Liquid text leg the harness knows. It
  takes the **owner-facing** prose: inbox card explanations, the words a
  human reads. 2B active on a 24B MoE, so the cost is small-model shaped.
- **`liquid/lfm3-1.7b`** — probed live 2026-11-10 and used by
  `generate.rs`, but **not declared in the roster**. Declaring it is part
  of this plan. It takes the **internal** prose: finding summaries, the
  meter's narrative, anything that a human reads only after a failure.
- **any further Liquid model** — admitted only by **probe**
  (`settings::discover_provider_models`, `settings.rs:3576`, the same call
  the API health line makes), then declared with its window. No id is
  guessed: the repo already paid for that lesson, and the guard is that a
  declared model is validated against the roster while a pinned one is
  not (deep dive R-3).

The mechanism matters more than the pick. `blend1`'s `generate` tool
hard-refuses anything outside `("lfm2.5-24b-a2b", "lfm3-1.7b")`
(`services/blend1/service/mcp_server.py:343`), so a vendored lane cannot
be the place the steward's model choice lives. **The roster plus a
`housekeeping` workload is.** That is why a minimal slice of the router
chain is in scope here, and the rest of it is not.

## The seams this stands on (verified)

- **The judgment head exists.** `validate` is a registered sys1 task
  (`dream.rs:112-115`), already a three-way choice
  (`supported` / `contradicted` / `unverifiable`) asked of a fact against
  evidence. Asking it of one fact against a **peer fact** is the same
  question with a different evidence body — `same` / `contradicts` /
  `unrelated` in the corpus's own words.
- **The prose leg exists** (`generate.rs`, `lfm3-1.7b`, probed), with
  `lfm2.5-24b-a2b` as its fallback and every failure a typed `Err`.
- **The healing catalog is complete** — every candidate in the
  [self-healing review](../architecture/2026-11-15-self-healing-review.md)
  shipped or is by-design (v0.8.6/0.8.7).
- **Four meters exist** and are unscheduled or unrecorded:
  `anomalies::pass` (six classes, CLI-only, **throws its report away**),
  `lifecycle::sweep`, `demand::collect`, `footprint`.
- **The ask surface exists** (`inbox.json`, five live classes, decision
  keys); the propose→gate→apply path exists (`improve`).
- **The principal exists** (`identity::PrincipalId`); background work
  passes `None` today (`cron.rs:1571`, `delegate.rs:354`).
- **The supervisor pattern exists** (`cron.rs:2133-2143`); homekeeping
  (`memory.rs:6073`), reclaim (`checkpoints.rs:73`) and the health task
  (`gui/bridge.rs:631`) do not.
- **The prerequisite:** `start_health_loop` has one call site
  (`gui/mod.rs:331`), and v0.8.6/0.8.7 put five detections inside it
  (deep dive H-1).

**The load-bearing rules.** The class belongs to the catalog entry, never
to the model. The effective class is the **stricter** of catalog and lane;
the lane may tighten and may never loosen. An abstention is not a
permission. A rejected recommendation is **suppressed**, not re-raised.
Nothing the cycle creates outlives its window. A work verdict is not a
licence to write. And a model choice lives in the roster, never in a
vendored lane.

## Acceptance criteria

- [ ] **C1 — the principal, and its chain.** `roles/steward.md` lands with
  the smallest tool set of any persona (`read_file, list_dir`),
  `workload: housekeeping`, empty `shapes`, and the overlay carrying "you
  may not write a number you are graded on." **The pin is the binding**:
  `Role.workload` walks `Walk::Named` (`subagents.rs` resolves the role
  before `prepare_child`), so the steward's runs walk the Liquid chain by
  name rather than by the judgment's label — and C8 creates the row the
  name resolves to. Until it exists the pin names no chain and the judged
  workload stands, which is the named degradation the router already gives
  a bad pin; the pin is corrected in the same step, not after it. A
  `steward` `PrincipalId` slug resolves
  through `identity`. Every idle run writes it on the session `Start`, the
  journal event, and every calib row it causes. It never carries
  `auto_approve` (`cron.rs:656`) and never holds a latched approver.
- [ ] **C2 — the trigger.** An `idle` module owns the conjunction — no run
  in flight, no delegated child running, no approval waiting, no turn
  queued, grace window elapsed — behind the one-step atomic claim the cron
  store already uses. The hourly timer demotes to **the floor**. Yield is
  a **return**: a turn arriving mid-pass stops at the next item boundary,
  persists the cursor, and the next window resumes. No stage holds a lock
  across an item.
- [ ] **C3 — the health hoist.** `start_health_loop` moves out of the face
  into a shared spawn that chat, TUI and face all call, so the five
  detections v0.8.6/0.8.7 added are not face-only. A move plus spawn
  sites; no new probe logic. *(Closes deep dive row 6.)*
- [ ] **C4 — the supervisors.** The shipped `catch_unwind` supervisor is
  applied to homekeeping, checkpoint reclaim and the health task.
  *(Closes row 7.)*
- [ ] **C5 — the meter (deterministic, no model).** One `steward` meter row
  per window over the dimensions the four readers supply: anomalies' six
  classes, lifecycle's unused counts, demand's rates, footprint's lines,
  health's lines. Every value is read from a record the steward does not
  write. `anomalies::pass` gains a timer **and a sink** — today it
  computes and discards. **The meter calls no model**; a dimension with no
  reader is named as a finding, not rendered as zero.
- [ ] **C6 — the record.** The recommendation is a typed record, not
  prose: `{dimension, subject, repair, evidence, class, lane, effective,
  state}` with a stable dedupe key over `(dimension, repair, subject
  digest)`. One store under `<FLEET_HOME>/steward/`
  (`recommendations.jsonl`, `cursor.json`, `debt.json`), append-only,
  crash-safe through the repo's filelock pattern, monotonic ids. States:
  `raised | healed | asked | answered | rejected | suppressed`.
- [ ] **C7 — the catalog and the ratchet.** A code table of registered
  repairs, each with its class and its evidence shape. Effective class =
  the stricter of catalog and lane. **With no safety verdict in slice 1,
  every judgment entry resolves to `approved`** — fail closed. One
  exception, named and narrow: an entry may declare `certifiable: true`
  when its safety is a **deterministic precondition in code**, not a
  judgment — no model input exists in the decision, so no second signature
  is owed. `cache-sweep` is the only certifiable entry here. The
  principle: **the second signature is for judgments, not for
  arithmetic.**
- [ ] **C8 — the model seats, declared and routed.** `liquid` declares
  `lfm3-1.7b` beside `lfm2.5-24b-a2b`; any further Liquid model is
  admitted by probe (`settings.rs:3576`) and declared with its window, and
  a model not in the declared list is refused rather than sent. A
  `housekeeping` workload lands: `liquid/lfm3-1.7b →
  liquid/lfm2.5-24b-a2b → deepseek/deepseek-flash (the tail)`, with the
  **ceiling** (deferrable work never escalates above the cheap class — an
  exhausted chain records idle debt and returns) and a daily USD cap in
  `delegate.rs`'s reserved-per-ask shape. The steward's prose calls
  resolve through `route::select("housekeeping")` and write the route row,
  so the leg's model, cost and usage are recorded — which they are not
  today. The row carries the **walked workload as a field**, not only as a
  clause in `reason`: the personas plan claims `walked_as` on the row and
  the row type does not have it (deep dive, Part 1), and a persona's model
  binding is unauditable without it.
- [ ] **C9 — the judges, and the fanout.** Every recommendation is judged
  before it can act, by parties that did not write it. Candidates come
  from `sem1`'s shape-first recall and the recall graph, ranked and capped
  — never enumerated. Two axes ride the registered `validate` head:
  **relation** (the peer fact as the evidence body: `same` / `contradicts`
  / `unrelated`) and **warrant** (the recommendation as a claim and the
  records the steward read as its transcript: `supported` / `contradicted`
  / `unverifiable`). The safety axis (`steward_safety`) is absent in this
  slice and resolves to escalate. **The write path requires both axes to
  agree**, from two asks; the read path takes one. Two judges per item at
  most, 2–3 heads each (the measured comfort range), the `validate`
  namespace of the route cache consulted before any spawn, one row per
  judgment with axis, verdict, confidence, subject digest and cost. A
  disagreement is **a row and a named skip, never an average**; a lane
  that cannot answer is a skip, never a guess. Cross-vendor pairing (d1
  beside the prose chain) is available and is priced as a disagreement
  detector, not a better scorer. **No judge writes.**
- [ ] **C10 — the ask, with the judges' verdicts on it.** A `same` or
  `contradicts` relation **and** a `supported` warrant, both above
  `STEWARD_RELATE_FLOOR` and both above their margins, become one inbox
  card naming the pair, both verdicts, the confidences and the evidence —
  and the card's one-line explanation comes from the **prose seat**
  (owner-facing: `lfm2.5-24b-a2b`) over the supplied facts, so the text is
  checkable line by line and its failure falls back to the closed label.
  The owner's press runs `Memory::supersede` with the verdict as the
  justification — the path the permission model already allows for an
  `inferred` fact. A `confirmed-by` fact is **never** offered.
- [ ] **C11 — the autonomous path, certified by code.** The meter's disk
  dimension raises `cache-sweep` past the threshold; the certifiable entry
  runs `health::sweep_once` in the window; the result is recorded, and a
  failed sweep is never recorded as healed. *(Closes row 8 for the paths
  it drives.)*
- [ ] **C12 — the ask discipline.** At most one ask per dimension per
  window; no two asks sharing a subject prefix; a rejection suppresses the
  key for a stated number of windows; the ask carries its evidence by
  reference. `inbox.json`'s decision keys are reused, not a second store
  invented.
- [ ] **C13 — citizenship.** Scoped handles closed on drop; scratch under
  `$FLEET_HOME/steward/<pass-id>/`, swept at the end of the window and the
  start of the next; an orphan reaper keyed on the pass id using
  `ps stat`; **no new listening port**; nothing wakes a sleeping runtime;
  a footprint line per window that did work (tokens **bounded and
  reported**, wall-clock, peak RSS against `MEMORY_CEILING_KIB`, process
  count against `PROCESS_CEILING`, disk delta, children opened/closed).
  **No item needing a reading means no call**: a tidy corpus costs zero
  tokens.
- [ ] **C14 — the fixtures.** Two families in `eval.rs`'s corpus, neither
  needing a live provider: (a) *citizenship* — twenty idle windows on a
  fixture home with nothing to do yield a net disk delta within bound,
  zero orphans, zero ports, process count back to baseline; (b) *the pair
  judge* — a fixture of known duplicate, contradictory and unrelated pairs
  scores the verdicts, and every card raised lands **zero writes without a
  press**.
- [ ] **C15 — the gate.** `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test` all
  exit 0 (read the exit code, never through a pipe — KI-014), plus the face
  harness. Docs updated in the same pass: the vision's ratchet gains
  `certifiable` and names `validate` as the interim work head;
  `docs/features/` gains the steward's page.

## Sequence

| step | criteria | why this order |
|---|---|---|
| S1 | C8's roster + workload row, then C1, C2 | a persona pinned to a workload needs the workload to exist; the chain is four lines of table and one roster field, so it goes first |
| S2 | C3, C4 | the meter reads health, and the loop must not die silently |
| S3 | C5, C6 | the meter and the record, before any repair is attached |
| S4 | C7, C8 | the catalog, the ratchet, and the model seats |
| S5 | C9, C10 | the model's first real job, and the write it may cause |
| S6 | C11, C12, C13 | the autonomous path, the ask discipline, citizenship |
| S7 | C14, C15 | fixtures, the gate |

## Verification

The full gate at every step, then live:

1. **Idle, empty home.** Twenty windows with nothing to do: cursor rows and
   no actions; **zero model calls**; footprint delta within bound; no
   orphans; no ports.
2. **A corpus with a known duplicate and a known contradiction.** Two cards
   and nothing else; the prose line matches the facts it was given; a press
   supersedes the right one; the other card's dismissal suppresses it for
   the window; **no write happens without a press.**
3. **The judges disagree.** Force the relation judge to `same` and the
   warrant judge to `contradicted` on the same pair: **no card**, one skip
   row naming both verdicts, and no write. Then force a thin margin on one
   axis: same result — escalate to nothing rather than to a write.
4. **A Liquid outage.** `LIQUID_API_KEY` refused: the judgment is a named
   skip, the prose falls back to the closed label, the chain records idle
   debt rather than escalating to a flagship, and the window still ends
   clean.
5. **Disk pressure on a fixture home.** `cache-sweep` raises and runs in the
   window, the output is recorded, and the next meter shows the space back.
6. **Preemption.** A held turn starts mid-window: the pass stops at the item
   boundary, the cursor persists, the next window resumes at the next item.
7. **The starved machine.** A continuous run past `IDLE_STARVE_MAX`: the
   board names the waiting item, and no stage runs under a queued turn.

## Out of scope (named, not built)

- **`steward_safety` upstream.** The `destroys` noul and the `why` choice.
  Until it lands, the safety verdict is *absent*, so every judgment entry
  escalates and no model verdict writes anything.
- **The rest of the router arc.** The ceiling's full semantics beyond the
  housekeeping chain, per-workload spend reporting on the board, rebinding
  `generate.rs` to the workload (deep dive R-2), and `frontend`'s tail
  defect (R-6) — all real, none on this slice's path.
- **Representation and subject pages**, **demotion**, **self-tuning** — all
  need the record to have a history first.
- **The spine reading the principal.** U50/U51 still classify background
  runs by heuristic (`session::start_kind`); this slice *records* the
  field, and the filter's migration is its own change.
- **Any live destructive recommendation**, and **any autonomous write from
  a model verdict.** The pair judge's writes are owner-pressed by
  construction.

## Decisions that do not move

This block exists because a full-file rewrite dropped two of these once
(the persona's workload pin, and the judges' agreement rule) and the loss
was invisible until it was caught by hand. Every revision of this plan
diffs against this list; a change to any line is a **ruling**, not an
edit, and the check is a grep over these phrases before the revision
lands.

1. The persona's pin is `workload: housekeeping`, and the pin **is** the
   model binding (`Role.workload` → `Walk::Named` → the preference row).
2. Three axes judge a recommendation: **relation**, **warrant**,
   **safety**. Two exist today; safety escalates until upstream.
3. The write path requires the two existing axes to **agree**; two judges
   at most; disagreement is a row and a skip, never an average.
4. The effective class is the **stricter** of catalog and lane; the lane
   may tighten and may never loosen.
5. `certifiable` is the only autonomous path, and it means a
   **deterministic precondition in code** — the second signature is for
   judgments, not for arithmetic.
6. **No write lands on a work verdict alone**: the model proposes, the
   catalog classes, the code precondition or the owner writes.
7. The steward is a **non-owner principal** with the smallest tool set, no
   `auto_approve`, and no conversation.
8. A model id lives in the **roster, admitted by probe**; `blend1`'s
   allow-list is never the place a model choice lives.
9. The meter writes no number it is graded on; idle is **capacity**,
   preemptible, with starvation named on the board.
10. Nothing is deleted; nothing outlives its window; no port; no waking a
    runtime for housekeeping.
11. Out of scope, named: `steward_safety`, the rest of the router arc,
    representation, demotion, self-tuning, the spine's principal migration.

## The line held

Nothing is deleted — archived, stamped, demoted, or swept. No model
authors a fact body. No write lands on a work verdict alone: the model
proposes, the catalog classes, the code precondition or the owner writes.
No class without a catalog entry. No abstention read as permission. No
recommendation repeated while suppressed. No model id outside the declared
roster. No daemon, no child, no port, no scratch file outliving its
window. And no stage runs under a queued turn: a housekeeper that delays
the owner is not a housekeeper.
