---
status: active
created: 2026-11-05
updated: 2026-11-05
slug: open-ledger-closeout
---

# Plan: the open ledger, closed by class

## Context

`docs/known-issues.md` carries **28 open rows** (`python3 scripts/ki_ledger.py`
→ `rows: 456   open: 28`, read 2026-11-05). Twenty-five were the queue this
plan was asked to address; three (KI-454, KI-455, KI-456) are findings this
plan's own drafting produced and are named below rather than left for a later
sweep. They are not 25 instances of one
thing — they are nine unrelated classes spanning the nomenclature write guard,
the routing walk, the plan/supersede tooling, test isolation, the native shell,
and the delivery seam. Planned as 25 independent items this would be an
itinerary, not a contract.

This plan is the single home for that queue, following
`2026-11-02-open-work-closeout.md` — the precedent for exactly this ask. The
difference it must argue: that plan absorbed *plan residuals*, work that had
been specified and not built. This one absorbs *ledger rows*, defects and gaps
recorded as they were found, so the rows arrive with findings attached and the
work is deciding what each finding actually costs.

Two things triage found before any work was planned:

1. **One row is already fixed** (KI-326, proven below) — so the ledger's `open`
   status is not a reliable premise. Every row is re-verified against the code
   before it is worked, and the verification is the plan's first criterion.
2. **Two rows are not fleet's code** (KI-396 in `~/Projects/dev-decisions`,
   KI-403 an upstream rustup layout), and **two are decisions rather than
   builds** (KI-406, KI-412 — the rows themselves say "the owner's call").

## Findings this plan's own drafting produced

**KI-454 — the doc understates the guard.** `TECHNICAL-DOCUMENTATION.md:474-476`
says `nomenclature.rs` "names the project's established terms and warns on
drift". The guard returns three verdicts and the doc names the mildest: Refused
blocks the write (`src/agent.rs:5999`), Flagged rides a note on a novel
near-neighbour, Clean says nothing. Arch-gate flagged it as one DRIFTS claim
against this plan. Correcting that sentence is C0's doc half, and it is the
same sentence C3's fix must leave true.

**KI-455 — the cap truncates in silence, and it bit this plan live.** The
plan-gate reads only the first 2000 chars of each section
(`devdec/gates.py:228,418`). A revision that added the committed step for four
MISSING criteria moved coverage not at all — 15/19 before and after, the same
four criteria back as MISSING. Splitting the body into sections under the cap
took the next run to 19/19. KI-455 is KI-396's second sighting, and this plan
is the reproduction.

**KI-456 — the gate counts the wrong checkboxes.** `_parse_plan_criteria`
(`devdec/gates.py:256-266`) reads every `- [ ]` line in the file with no
section filter, so this plan's five mandated `## Use cases` contracts became
C0-C4 and every real criterion shifted down by five: the gate reported
`19 criteria` for 14, and its MISSING list named C4/C5/C8/C9 as C9/C10/C13/C14.
`scripts/plan_check.py` already scopes to the acceptance-criteria heading and
says in its docstring that counting the rest inflates the list by 46. Reading a
coverage verdict here means reading the criterion's text, never its number.

## What is in scope

Every open row, grouped by the class that decides its work — not by severity,
because the class is what makes several rows one job.

| Class | Rows | Route |
|---|---|---|
| **A. Nomenclature guard false positives** | KI-395, KI-415, KI-428 | one matcher fix; KI-415/428 are instances of KI-395 |
| **B. Test isolation & load sensitivity** | KI-405, KI-426, KI-452, KI-446, KI-326 | one discipline, four fixtures; KI-326 already fixed |
| **C. Promise-vs-behaviour tooling** | KI-402, KI-409, KI-396 | a tool whose declared shape and executed shape disagree |
| **D. Delivery seam** | KI-412, KI-431 | the installed binary, the agent's post-state |
| **E. Child-report trust** | KI-320, KI-327 | one render remedy |
| **F. Decision rows (no build until ruled)** | KI-406, KI-412, KI-404, KI-414 | present the call, build on the answer |
| **G. Routing & compute** | KI-451 | one arm beside `names_model_unavailable` |
| **H. Record-keeping & environment** | KI-317, KI-323, KI-398, KI-403, KI-252 | discipline, a checker hook, and a documented perpetuity |

## Approach

**Fix the classes, close the instances.** Six of the 25 rows are the same defect
observed on a different surface (KI-395/415/428 are one guard; KI-405/426/452
are one load-sensitivity discipline). Working row-by-row would build the same
fix six times and leave the class alive to produce row 454. Each class gets one
fix and one pin, and its instances close together.

**Verify before working.** KI-326 is already fixed in `src/memory.rs:6036-6049`
— the lookup shadows the var under test — and was proven fixed this session by
running it with `FLEET_EMBEDDING_ENDPOINT` exported to the value that used to
red it. A row that reads `open` may be a stale label, and a plan that builds a
fix on top of one wastes the build. So C0 re-verifies all 25 before C1 starts,
and the ledger is corrected for every row already fixed.

**Separate what can be decided from what can only be asked.** KI-406 (the
native shell's verification seam), KI-412 (whether install reconciles the
installed binary) and KI-404 (a board's digest binding default) are
architectural choices the owner owns. They get a written recommendation and a
named default, and the build follows the answer — they do not hold the classes
that need no ruling.

The order of work, and the reason for it, is in **Sequencing and why**. What
each class commits to is specified row by row in the two sections that follow
this one.

## The classes, row by row (A)

**A — the nomenclature guard (KI-395, KI-415, KI-428).** The matcher refuses a
case or separator variant of an established term, which is right for drift
(`judgeRoute` beside `judge_route`) and wrong when the "variant" is the
canonical spelling itself. KI-395 is the struct name `Usage`; KI-415 is the SF
font family's mono variant, the capitalised `One` at a sentence start, and the
serde derive `Deserialize`; KI-428 is the same refusal arriving through
`apply_patch` instead of `write_file`. One fix: a case-insensitive match that
admits the exact spellings the vocabulary indexed, because a canonical form is
never a variant of itself. The pin must show a genuine drift still refused.

## The classes, row by row (B, C)

B — test isolation and load sensitivity (KI-405, KI-426, KI-452, KI-446,
KI-326). Four fixtures bound an observable in wall-clock seconds while the
subject is not timing, so a loaded machine reds them and a quiet one passes:
the probe retry (`src/membership`… the probe at `src/memory.rs:6054-6112`), the
layerfs stub trio, and the blend1 doctor pair (`tests/blend1.rs:348`). One
discipline: the observable counts attempts or polls, proven green with four CPU
burners running. KI-446 is separate plumbing — the mock server resolves from
`CARGO_BIN_EXE_mock-server`, then from the test binary's own directory, then
the manifest path, so a shared `CARGO_TARGET_DIR` stops breaking eleven
`agent::tests` cases. KI-326 is already fixed; C0 confirms it and closes it.

**C — tooling that announces a path it does not take (KI-402, KI-409).**
`src/tools.rs` reads `replacement` as an optional string, so a caller supplying
`description` and no `replacement` meant the modify path and is retired as a
forget instead: the parse refuses that pair and names the missing
`replacement`, the forget's result sentence says the name is dead rather than
"expired with its retirement noted", and an unknown `phrases` key is accepted
or rejected by name rather than ignored. `parse_remember` (`src/cli.rs:2102`)
reads every value as one shell word, so a `--body` carrying a bare
double-quoted JSON object cannot be filed at all: the parser accepts the value
from a file path the way a doc is read, pinned in `tests/cli.rs`.

## The two caps that are not fleet's code (KI-396, KI-455, KI-456)

**KI-396 / KI-455 — the truncation.** The plan-gate truncates each
section at `_PLAN_GATE_SECTION_CHARS = 2000`
(`~/Projects/dev-decisions/scripts/devdec/gates.py:228`, applied at `:418`) and
says nothing about what it cut. This plan hit it: its own Approach section was
4230 chars, and a revision landing past char 2000 moved no verdict at all. The
fix is a truncation note in that gate, so it belongs to `dev-decisions`, not to
fleet — fleet files the row against that repo, with the file and line, and its
own ledger row points there. The fleet-side mitigation that needs no other
repo is already in this plan's shape: no section here exceeds the cap.

**KI-456 — the criteria index.** The same file's `_parse_plan_criteria`
(`gates.py:256-266`) collects every checkbox in the plan, so the template's
`## Use cases` block is consumed as criteria. Fleet's own
`scripts/plan_check.py` already draws the line correctly — boxes under an
`## Acceptance criteria` heading, and its docstring names the 46-row cost of
the other reading — so this is also a fleet-side convention to copy, which is
why C11 files it rather than patching around it.

## The classes, row by row (D, E, F, G, H)

**D — the delivery seam (KI-431, KI-412).** KI-431 is a build, not a choice: a
restart's bootout was not followed by a re-bootstrap, so the face and both
relays stayed down and the owner read it as "fleet is down". The reload path
verifies the agent's post-state — `launchctl print` shows a pid and the port
answers — and re-bootstraps by name when the domain lacks the service. KI-412
is the owner's call (C12): whether install reconciles `~/.fleet/bin/fleet` with
the built binary, or the agent runs the worktree's `target/release/fleet`.

**E — child-report trust (KI-320, KI-327).** `render_workspace` already rides
a child's result with branch, merge outcome and conflict notes. What is
missing is the project name and the `fs read main /<path>` command that reads
the work back, so a parent never restates a sandbox path as a host one.
KI-327 closes with it: a zero-tool-call child's "merged to main" is template
noise, and the render that carries the project and read command makes that
visible instead of plausible.

## The classes, row by row (F, G, H)

**F — decision rows (KI-406, KI-412, KI-404, KI-414).** Each carries a written
recommendation and a named default in C12. No build lands before the answer,
because each row says in its own words that the call is not the session's.

**G — routing (KI-451).** `failure_class` (`src/route.rs:2656`) classes every
400/404 "client-error" and therefore not failover-eligible, on a premise an
account-state body falsifies. A second keyword arm beside
`names_model_unavailable` (`src/route.rs:2698`) reads the body for account,
subscription, or plan state and returns failover-eligible, so the walk reaches
its next candidate instead of ending at its first.

**H — record-keeping and environment (KI-317, KI-323, KI-398, KI-403,
KI-252).** KI-317: a run writes its driver under a project-scoped, run-unique
name and runs the work in the same shell invocation that writes it. KI-323:
plans cite `docs/` artifacts as evidence homes and treat memory facts as
pointers carrying an inline quote. KI-398: `scripts/plan_check.py` already
detects a completed plan with an unticked criterion and exits 1, but nothing
invokes it — the check is named in `docs/plans/README.md` as a precondition of
`completed` and wired into the session's pre-commit list. KI-403 and KI-252 are
recorded as documented perpetuity with their re-check triggers named.

## Decision-model check

Two fits, both narrow. **KI-451** is a classification of an HTTP failure body
into failover-eligible or not — the row itself says it is "one arm beside
`names_model_unavailable`", a keyword test, and that is the right shape: a
message-body heuristic that must never call a model to decide whether to try
the next one. Escalating it to sys1 would put a network call inside the
failover path it is meant to unblock. **KI-396** reports which plan sections a
gate truncated — a mechanical count over a body, no judgement. Nothing else in
this plan classifies, scores, or routes per item; the rest is fixtures,
plumbing, and record-keeping, and inventing a lane for them would be the
Hermes shape the anti-goal names.

## Use cases

- [ ] A: A session writes a file naming a struct whose name is a canonical
      spelling, and the write lands without a work-around.
- [ ] B: A full `cargo test` on a loaded machine does not red a test whose
      subject is not timing.
- [ ] C: A caller asks `supersede` to modify a fact and is never silently
      retired into a forget.
- [ ] D: A rebuild cannot leave the served face behind, and a restart that
      unloads the agent says so.
- [ ] E: A parent reads a child's work back without restating a sandbox path
      as a host one.

## Progress

The triage C0 completes. Every verdict below was read out of the code or a live
run on 2026-11-05, not inferred from the row's own status cell.

| Row | Verdict | Evidence |
|---|---|---|
| KI-326 | **already fixed** | `src/memory.rs:6036-6049`; proven live — `FLEET_EMBEDDING_ENDPOINT=http://127.0.0.1:8901/v1/embeddings cargo test --lib an_env_write_is_readable_by_the_runtime` → `1 passed` |
| KI-395, KI-415, KI-428 | confirmed open | the same nomenclature matcher; KI-415/428 are instances |
| KI-396, KI-456 | **not fleet's code** | `~/Projects/dev-decisions/scripts/devdec/gates.py:228,418` and `:256-266` |
| KI-403 | **not fleet's code** | an upstream rustup layout, not a fleet build |
| KI-406, KI-404, KI-414, KI-412 | **a decision** | each row names the call as the owner's own |
| KI-402, KI-409, KI-405, KI-426, KI-452, KI-446, KI-431, KI-320, KI-327, KI-451, KI-317, KI-323, KI-398, KI-252 | confirmed open | per-row file:line read; each is named in its class section |
| KI-454, KI-455, KI-456 | filed by this drafting | see "Findings this plan's own drafting produced" |

C0 remains unticked: the re-verification is done and recorded here, but C0 also
carries KI-454's doc correction, so its box stays open until that sentence
lands.

## Acceptance criteria

Each criterion is a verifiable assertion; the work is done when every box is
checked. **Order is identity** — never reorder after gating.

- [x] C0 — all 28 open rows (the original 25 plus KI-454/KI-455/KI-456) are
      re-verified against the code and recorded in this plan's Progress table
      as confirmed-open / already-fixed / not-fleets-code / a-decision, the
      ledger corrected for every row found fixed, and KI-454's doc sentence
      corrected to the guard's three verdicts. **Two rows were already fixed**
      (KI-326, and KI-454's doc sentence) and were closed here rather than
      rebuilt. KI-454's close also corrected the entry's own citation: it
      pointed at `src/agent.rs:8286-8291`, the Refused precondition, not at the
      return; `WriteGuard::Refused` is constructed at `:8304-8310`.
- [x] C1 — the three load-sensitive fixtures (`the_probe_retries_after_a_pause…`,
      the layerfs stub trio, the blend1 doctor pair) each bound their
      observable in attempts or polls rather than wall-clock seconds, and each
      is proven green with four CPU burners running beside the suite.
      **Landed:** `EMBED_ATTEMPTS_PER_CALL = 2` in `src/memory.rs` with the pin
      `one_embed_call_spends_exactly_its_attempt_count`; `VERSION_BUDGET = 10s`
      with `VERSION_ATTEMPTS = 3` and `run_version_with`/`retry_version`/
      `parse_version` in `src/layerfs_tools.rs`, four pins including a ~10.13 s
      real read; and KI-452, whose load-sensitivity diagnosis measured FALSE —
      the real mechanism was a busy-retry in `http_llama.embed` taking 184 s
      against `src/mcp.rs`'s `CALL_TIMEOUT` of 60 s, now bounded by
      `DOCTOR_PROBE_DEADLINE = 20.0` in sem1 and blend1, measured 184.1 s → 20.0 s.
## Acceptance criteria (C2–C3)

- [x] C2 — the mock server resolves from `CARGO_BIN_EXE_mock-server`, then from
      the directory the test binary lives in, and only then the
      manifest-relative path, and the `agent::tests` cases that spawn it pass
      under a `CARGO_TARGET_DIR` outside the manifest. **Landed:** step 1
      cannot carry the unit tests — cargo stamps no `CARGO_BIN_EXE` into a lib
      test target — so step 2 is the load-bearing one; the failing set was
      twelve spawned, eleven red, and the row had missed
      `the_run_report_carries_the_per_turn_curve`. Detached worktree at
      `62814a4`, `CARGO_TARGET_DIR` outside it: red 97/11 failed, green
      108/0.
- [x] C3 — a `write_file` naming an exact canonical spelling the vocabulary
      indexed is admitted, and the KI-415 instances (the SF font family's mono
      variant, `One`, `Deserialize`) and the KI-428 `apply_patch` case are
      pinned as admitted, so the guard refuses no canonical form of itself.
      LANDED 2026-11-05: one shared rule, `nomenclature::spelling_is_written`,
      asked by both surfaces — the write guard (`src/agent.rs`) skips the
      token, and `convention` (`src/tools.rs`) answers "is a spelling this
      project writes" in its near-exact arm instead of "rename". The bar is
      MEASURED, not chosen: a spelling written by at least
      `SPELLING_CONVENTION_FILES` = 2 distinct files is a canonical form of
      its own term (real tree: `Usage` 22, `One` 34, `Mono` 4, `Deserialize`
      31, `Projects` 40 — every KI-415 instance clears it); it is 2 and not 1
      because the drift spelling sits in exactly one file, a guard-test module
      built to prove the refusal. Five pins, two of which read their premise
      back before asserting; three red-proofs
      (delete the guard call; the bar at 1; `if false &&` in `tools.rs`).
      A single-word case exemption was added, measured, found to admit
      `Projects` in a language-keyword fixture, and removed.
## Acceptance criteria (C4–C5)

- [x] C4 — a `supersede` call carrying a `description` and no `replacement` is
      refused by name as the modify path it meant, rather than taking the
      forget path, and a forget's result sentence says the name is dead rather
      than "expired with its retirement noted". LANDED 2026-11-05:
      `Supersede::parse` refuses a call carrying any replacement-shaped
      argument with no `replacement`, through the new
      `WriteError::Replacement`, and the message names what it found and what
      would have happened. A forget's own result says the name is DEAD, that
      `remember` is refused while it is taken, and that a correction wants
      `supersede` with `replacement`. Both sentences read out of a replayed
      session, not the source. `phrases` joined the row's third defect as a
      real field and a declared property with the companions' inherit-or-clear
      rule. Every caller in the tree was carried: `src/improve.rs`,
      `src/gui/bridge.rs` (whose delete path was passing a replacement-only
      `description` with no replacement — the exact silent forget, now
      refused), and `src/gui/server.rs`'s management-verb test, which had
      asserted `superseded-by:` against a frontmatter field that stands empty
      on every live fact, so the check never proved a stamp. Five pins, three
      red-proofs. The token bound was measured, not raised: the additions
      pushed the surface to 10,018 against `MAX_DEFINITION_TOKENS = 10_000`
      and tiered `apply_patch` behind `tool_search`; the text was trimmed.
- [x] C5 — `fleet remember` can file a fact whose `--body` or `--description`
      carries a bare double-quoted JSON object. LANDED 2026-11-06, and the
      criterion's own file did not exist: `tests/cli.rs` is not in this tree
      and never was (`remember_parses_its_structured_flags` lives in
      `src/cli.rs mod tests`), so the pin landed beside it there. The row's
      shell diagnosis was measured and does not reproduce — a single-quoted
      outer with JSON inside files and reads back — but the gap it names was
      real: there was no shell-safe route, so a caller had to guess how many
      quoting layers a blob sat behind. Two now exist: `--description-file
      <path>` and `--body-file <path>` read the value VERBATIM out of a file
      (modelled on `fleet graph --file`, with duplicate detection, a "not
      both" refusal against the literal flag, and a named error carrying the
      flag and the path). Underneath sat a worse defect, filed as KI-458 and
      fixed here: the writer emitted INVALID YAML for those same values —
      `description: "he said "hi""` — so 128 of 625 live facts were refused by
      any standard reader while fleet's own hand unquoter survived and nothing
      reddened. `quote_scalar` escapes `\` and `"`, `flow_scalar` quotes any
      item a plain scalar would misread, `split_flow` reads them back
      quote-aware, and `unquote` unescapes exactly what the writer emits. The
      corpus was repaired through the same rules: 192 facts rewritten,
      625/625 strict-parseable, and fleet's view of all 625 byte-identical
      across the repair. Three pins, three red-proofs (one of which exposed a
      vacuous assertion of my own — `written.phrases` came from the request,
      not the file — and was rewritten to read the reloaded fact).
## Acceptance criteria (C6)

- [x] C6 — the reload path verifies the agent's post-state (a pid from
      `launchctl print`, the port answering) and re-bootstraps by name when the
      domain lacks the service, so a restart cannot leave the face down.
      LANDED 2026-11-06, and the criterion's premise had to be built first:
      the tree had NO reload path in code at all, only the runbook block in
      `docs/features/install.md` and a scratch `tmp/deploy447.sh` — which is
      precisely how KI-431 happened, because a sequence of shell lines a reader
      must complete is not a path that can verify anything. It is now a verb:
      `fleet reload [--from <path>] [--host] [--port]` in the new `src/reload.rs`,
      wired through `src/cli.rs` and `src/main.rs`, with the install doc's
      block rewritten to invoke it. The post-state is decided from the two
      facts the machine can be asked (a pid, and a TCP connect on the port)
      by the pure `post_state`; `repair_for` picks the verb the observed world
      calls for (bootstrap when the domain lacks the service, restart when it
      holds one); a first repair that does not take escalates to the
      bootout/bootstrap pair. The EXIT CODE follows the proof, not the
      commands — the old failure was that every line succeeded while the face
      was down. `--from` installs through a new inode (KI-418's rule, now code
      instead of prose) and refuses a source with no exec bit by name, since
      installing one produces exactly the face-stays-down shape this row is
      about. Proved LIVE against a scratch agent (`com.fleet.reloadprobe`,
      :55454) driven from KI-431's own precondition — bootout run,
      `Could not find service` from `launchctl print` — which the reload
      bootstrapped to `state = running pid 50987` with the port answering 200,
      confirmed independently of the tool's own report; the owner's face was
      never touched (pid 94983, :3080 200 throughout) and the scratch plist is
      removed. Two red-proofs: bootstrap disabled → the escalation caught it;
      both disabled → the run printed `the face is DOWN` and exited non-zero,
      the old outcome. Eleven module pins, one parser pin, one help pin.

## Acceptance criteria (C7-C8)

- [x] C7 — a child result's render carries its project name and the
      `fs read main /<path>` command that reads its work back, so no parent
      restates a sandbox path as a host one.

      **What landed.** Two rows, one surface. KI-320: `WorkspaceReport`
      gained `project` as its first field, so the store address the parent
      needs is a reported fact instead of archaeology — a child's tools
      resolve relative paths against its *sandbox*, and a path in its answer
      is not a host path. `render_workspace` now opens with
      `workspace: project {p}` and closes with the command that reads it back
      (`subagents.rs`: `read_back_cli`, `read_back_command`,
      `render_workspace_with`). The CLI half resolves through the gate
      (`Gate::command_line` → the new pure `program_line`), so a machine that
      runs the bundle prints `node "<bundle>/…/layerfs.js"` with the
      space-bearing path quoted — the old `Gate::command_line` joined parts
      unquoted, which splits the moment a path has a space in it.

      KI-327: a publish the store *refuses* never moved `main`, but the old
      arm recorded `Merged` because the local merge had succeeded — the header
      printed `merged to main` over a trunk that never moved. `MergeOutcome`
      gained `NotPublished { integration, reason }`; `merge_back`'s error arm
      sets it, `settle` matches it explicitly (not by wildcard, so a future
      verb is a compile error rather than a silent note), the render prints
      `NOT merged to main (the publish to the integration branch was refused: …)`,
      and `apply_home` withholds the home apply with the same reasoning a
      Conflict gets — nothing is walked home through a publish that did not
      happen. `read_back_command` points at the child's branch, not `main`,
      whenever the work did not land.

      Pins: `subagents::tests::the_workspace_header_names_the_project_and_the_command_that_reads_it_back`
      (merged → `main`, unmerged → the branch, empty project → no command at
      all), `a_refused_publish_is_not_a_merge_to_main`,
      `the_read_back_command_names_the_cli_this_machine_runs` (bare exe and
      the space-bearing bundle path), and
      `layerfs_tools::tests::a_refused_publish_is_carried_as_its_own_outcome_and_applies_nothing_home`
      (a live settlement against a stub store that lands the merge and
      refuses both publish attempts).

      Red-proofs, each watched by name: `Merged(serde_json::Value::Null)`
      restored in `merge_back`'s error arm →
      `a refused publish is not a merge to main: Some(Merged(Null))`, and the
      settlement pin fails the same way; `holds_the_work` hardcoded to `main`
      → `the unmerged child is read from its branch: … read back: layerfs
      --project shop fs read main /<path>`; the empty-project guard dropped →
      the same pin fails on the fabricated address. Restored, and both pin
      groups re-verified green (49 subagents, 1 settlement).
- [x] C8 — a 400/404 whose body names account, subscription, or plan state is
      classified failover-eligible and the routing walk continues to the next
      candidate instead of ending at its first.

      **What landed.** `src/route.rs`: `names_account_state` sits beside
      `names_model_unavailable`, and the 400/404 arm of `failure_class` reads
      it into a new `account-state` class. The predicate is two-sided on
      purpose — a plan/subscription/account noun AND a state word
      (`no active`, `inactive`, `expired`, `unpaid`, `out of credit`,
      `insufficient`, `suspended`, `deactivated`, …) — so a request-validation
      message that merely mentions a `plan` parameter stays a client error.
      The model arm keeps precedence over the account arm when a body says
      both, since which one is wrong is what the surface prints.

      The change is not only the classifier. `judge.rs` had the coupling
      problem: `AVAILABILITY_CLASSES` lists the classes that mean "the
      provider could not serve", and a class the walk advances on but the
      table omits grades a covered walk as a *wrong selection* — the exact
      inverse of what happened. `account-state` joined the table (now 8), and
      `judge::tests::intern_class` learned both `account-state` and the
      pre-existing `model-availability`, which it had been reading as `None`
      (a route row read back through it lost its class entirely).

      Pins: `route::tests::an_account_state_400_fails_over_instead_of_ending_the_walk`
      (the live instance, six account-state bodies across 400 and 404, and
      five negatives including a `plan` *parameter* validation message),
      `the_model_arm_wins_over_the_account_arm_and_names_its_own_fact`,
      `an_account_state_failure_lets_the_walk_reach_the_next_candidate` (the
      criterion's own sentence, driven through the real
      `failover_eligible_class` decision), and
      `judge::tests::every_class_the_walk_advances_on_grades_as_availability`
      (the table, its two deliberate omissions, and a full `grade` on an
      account-state-covered walk).

      Red-proofs, each watched by name: the `names_account_state` arm removed
      → `left: "client-error", right: "account-state"`; the two-sided gate
      replaced with `true` → `invalid request: \`max_tokens\` must be positive`
      graded `account-state`; `account-state` dropped from
      `AVAILABILITY_CLASSES` → "`account-state` advances the walk, so a
      covered failure on it is the provider's availability". Restored, and
      the full lib suite re-verified: **1914 passed, 0 failed, 3 ignored**.

      Corpus and docs corrected with the code: the memory fact
      `fleet-vs-opencode-missing-session-header` argued the blanket
      "400 is not failover-eligible" position and now names the two narrowed
      exceptions, and `TECHNICAL-DOCUMENTATION.md`'s `route.rs` section
      states the class table rather than leaving it to the code.

## Acceptance criteria (C9–C13)

- [x] C9 — `scripts/plan_check.py` runs as part of a named check the session
      actually executes before calling a plan complete, so a completed plan
      with an unticked criterion cannot be committed silently.

      **What landed.** Two enforcement points and the instruction that names
      them. (1) `docs/plans/README.md` (the file that already owns the
      `active → completed` rule) now carries the run-before-complete step and
      the one-per-clone `git config core.hooksPath .githooks`. (2)
      `.githooks/pre-commit` (new, executable) runs the checker only when a
      `docs/plans/*.md` path is staged — a code commit pays nothing — and on a
      non-zero exit prints the KI-398 message and returns 1. `CLAUDE.md`'s
      session protocol names both one-line checks beside the ledger's.

      **The checker itself was widened, because two holes made it a no-op in
      the shapes that matter.** `open_criteria` read only `line.startswith(
      "- [ ] ")`, so a nested `  - [ ]` criterion under an
      `## Acceptance criteria` heading — a shape a plan uses for sub-criteria
      — was invisible; it now matches column 0 through six spaces and the
      three list markers, skipping fenced spans. And a plan with no `status:`
      line was skipped outright, which is KI-398's defect with the cell empty
      instead of wrong; it now reports a statusless plan that still carries
      open criteria. Measured against the real corpus before and after: 0 live
      offenders either way, so the widening changed no verdict on today's
      plans — it closes shapes that were never exercised.

      **Proofs, all watched.** Planted `- [ ]` under a completed plan's
      criteria → `git commit` refused, exit 1, HEAD unchanged at `9b2d660`,
      message naming the plan and the criterion. Same plant as an indented
      box → refused. `sh -x` on a code-only staged tree → the checker never
      invoked, exit 0. The real plan staged as-is → "every completed plan's
      acceptance criteria are ticked", exit 0. Fixture set under
      `/tmp/pchole{,2}`: a fenced `- [ ]` inside the section does not count; a
      `## Use cases` box does not count; a nested box does; a statusless plan
      with an open box is named.
- [x] C10 — every row this plan calls perpetuity names its re-check trigger,
      and the trigger was READ rather than assumed. KI-403 is recorded as
      documented perpetuity with its trigger (any stable-toolchain update) and
      was re-verified live on 2026-11-06 — `otool -L` shows `@rpath/libLLVM.dylib`,
      `otool -l` the two-levels-deeper rpath, `readlink` the repair in place.
      KI-252's trigger FIRED (the corpus crossed 500 facts), so its perpetuity
      was falsified and re-measured instead of re-deferred: recall@10 agreement
      is 1.000 at this size, the cost half is unreadable and filed as KI-459,
      and the row was narrowed to the one piece genuinely unbuilt — semantic
      clustering — with the setting left DEFAULT OFF. The criterion's original
      wording ("so neither reads as work waiting to start") was false for
      KI-252, which is why it is rewritten here rather than ticked as it stood.
- [x] C11 — KI-396 and KI-456 are filed against `~/Projects/dev-decisions`
      (`scripts/devdec/gates.py:228,418` and `:256-266`) with fleet's ledger
      pointing at them, since neither gate is fleet's code. That repo had no
      defect surface at all, so the filing opened one — `docs/known-issues.md`
      there, in the same table shape, with D-001 (the silent section cap) and
      D-002 (the criteria index). Both were MEASURED with that repo's own
      parsers before being written down, not restated from fleet's rows:
      D-001 by driving `_parse_plan_sections` and assembling the state the way
      `:418` does (a 2098-char section carrying a sentinel at char 2048 produces
      a state with no sentinel and no truncation note), D-002 by running
      `_parse_plan_criteria` on this very plan (19 returned for 14 declared,
      `C0..C4` being the use cases, `C13` being the author's C8). Committed
      `fb6b456` on branch `fix/plan-gate-truncation-and-criteria-index`, pushed.
      Fleet's KI-396/KI-455/KI-456 now name D-001/D-002 in their next-action.
- [ ] C12 — the four decision rows (KI-406, KI-412, KI-404, KI-414) each carry
      a written recommendation with its default named, are presented to the
      owner as one block, and each is built or explicitly deferred on the
      answer.

      **The block, with each recommendation's evidence read out of the code.**
      Presented 2026-11-06 to the owner; the box ticks when the answers land.

      **KI-412 — deliver the built binary, defaulting to reconcile on install.**
      *Recommend:* `fleet install` (and `fleet reload`, which already exists)
      copies `target/release/fleet` over `~/.fleet/bin/fleet` — `reload.rs`'s
      `install_binary` already does the copy, the executable-bit check, and the
      atomic staged rename, and `reload()` wires it. A correction read out of
      the code before putting this to the owner: `grep -n "LaunchAgent\|plist\|
      launchctl" src/install.rs` returns nothing — install owns layout, config,
      lane wiring, venvs and layerfs, while `src/reload.rs` is the only module
      that names a plist or calls launchctl. So the gap is not "install forgot a
      line": install has no part in the service's lifecycle at all, and the
      reconcile step is the first one. Two shapes fit — install ends by
      calling `reload::install_binary` then `reload::reload()` (one command,
      what the owner would expect from "install"), or install only reconciles
      the binary and leaves the restart to the documented `fleet reload`. The
      recommendation is the first, because a copied binary that the running
      service has not re-executed is the KI-431 symptom one step earlier.
      *Reject:* pointing the LaunchAgent at the worktree's
      `target/release/fleet` — a `cargo clean`, a branch switch, or a second
      worktree leaves the service pointing at a path that no longer exists, and
      the symptom (the face is down) is the one KI-431 just fixed.
      *Default if the owner says nothing:* reconcile. *Half done already:*
      `fleet reload` is the manual seam, and `docs/features/install.md` now
      names it. *Measured while putting this to the owner:* the gap is latent
      today, not biting — `md5 -q ~/.fleet/bin/fleet target/release/fleet` is
      `1e3fcebf9ef0a13cb3f087de59c7d391` for both, and
      `launchctl print gui/$(id -u)/com.fleet.gui` reports
      `program = /Users/alejandrodelvillar/.fleet/bin/fleet`, `pid = 94983`. The
      two agree because the copy was made by hand on 2026-11-05; the next
      `cargo build --release` breaks the agreement with no signal, which is the
      KI-412 symptom exactly.

      **KI-404 — bind a board to the project it was created in.** *Recommend:*
      a board created from the face's canvas inherits the cwd's project key, so
      `refresh_digest` (`src/canvas.rs:476-479`) stops returning early on the
      `None` its own `create` writes at `:592`. The seam is one field: `create`
      takes a `home` and a name and knows neither cwd nor request, so the
      caller supplies the binding — the GUI's `"create"` op (`src/gui/bridge.rs:8812`)
      and the CLI's board verb both know the folder. *Reject:* writing the
      digest unkeyed and dropping the jobs' glob — that deletes K6's whole
      read path (`design_context_section`, `src/canvas.rs:507`) rather than
      feeding it. *Default:* bind. This is the only one of the four where the
      row's own "until then" is a real daily cost: every board on this machine
      is unbound, so K6 reads nothing at all.

      **KI-406 — accept the HTTP API as the shell's test surface, and say so.**
      *Recommend:* the honest option, because the alternative needs a private
      API. Read out of the pinned crates rather than asserted: `wry 0.57.1`
      sets `isInspectable` only under `#[cfg(any(debug_assertions, feature =
      "devtools"))]` AND only after `attributes.devtools` is true
      (`wry-0.57.1/src/wkwebview/mod.rs:579-589`), and its own comment says the
      inspector route is Safari's Web Inspector, not an in-process handle a
      harness can drive; tauri's `devtools()` (`tauri-2.12.5/src/webview/mod.rs:1192`)
      documents that "macOS: This will call private functions". So a debugging
      path exists, but it hands the owner a human inspector — not a driver the
      face harness or Computer Use can script. *Default:* HTTP API as the
      tested surface, with the window declared a thin view over it in
      `FUNCTIONAL-SPECIFICATIONS.md`. What that costs: a regression in the
      window's own rendering (a Tauri-only CSP, a webview quirk) has no
      automated witness — and it is bounded, because the window loads the same
      served origin the browser does.

      **KI-414 — keep the `srcdoc` mount; already the built shape.**
      *Recommend:* no work. The row is an environmental constraint recorded so
      the next mount does not rediscover it, and the mount was built the right
      way: `src/gui/assets/app.js:17108` sets `frame.srcdoc = html` on a frame
      with `sandbox="allow-scripts"`, and the bundle route still sends
      `frame-ancestors 'none'` (`src/gui/server.rs:1923`, the bundle route's own
      head at `:1902-1927` — the same phrase at `:1775`/`:1846` belongs to the
      design routes, a correction the first draft of this paragraph got wrong).
      *Default:* keep as is; close it as recorded-below-the-line rather than
      open work. **The default was taken (C13):** KI-414 is closed on that
      record, with the measured line numbers in its status cell.
- [x] C13 — every row that closes carries its evidence read out of the code or
      a live run in its status cell, and `python3 scripts/ki_ledger.py` exits 0
      with fewer open rows than the 25 originally open before this plan.

      **Landed 2026-11-06.** The ledger reads `rows: 461   open: 8` and exits 0,
      with `zero rows missing ID/severity/status/next-action; all cell counts
      agree with the header` — 25 open before this plan, 8 now, and every closed
      row's status cell names what was read to close it (a line range for a
      code claim, a command and its output for a live one).

      **The gate, on the tree being committed:** `cargo build --all-targets` exit
      0 with zero warnings; `cargo test` exit 0, `1915 passed; 0 failed; 3
      ignored` in the lib and `168 passed; 0 failed; 4 ignored` in the
      integration binary; `node tests/face/harness.mjs` 945 checks across 105
      cases; `python3 scripts/ki_ledger.py` exit 0; `python3
      scripts/plan_check.py` exit 0.

      **The gate earned its keep on this criterion.** The first full run reddened
      one integration pin: `a_layerfs_child_walks_the_coherence_sequence`
      asserted the pre-C7 header (`workspace: branch fleet-child-`) against a
      render C7 had changed to carry the project, and C7 had been ticked on an
      integration run recorded BEFORE that change reached `subagents.rs`. Filed
      as KI-460, fixed, red-proofed by reverting the needle, and closed.

      **A second defect came out of the same audit, and it is the more serious
      one.** C5's YAML repair had silently collapsed the phrase list of
      `memories/bsd-mktemp-suffix-trap.md` from five items to three — the file
      parsed, carried the field, and had lost data, which is invisible to the
      strict-parse check that repair was judged by. Root cause: `flow_scalar`
      quoted a plain `literal X's`, and `split_flow` opened a quoted item at any
      quote character, so the quote the writer added ate the rest of the line.
      Measured against `yaml.safe_load` (an interior quote is plain data) and
      swept across all 132 C5-touched facts for fields where the two readers
      disagree — exactly one, the repaired one. Fixed on both sides, the fact
      restored, the 23 over-quoted fields rewritten as the shipped writer now
      writes them, pinned and red-proofed. Filed as KI-461 and closed.

      **Two rows closed on the record rather than on a build,** each with its
      own reason stated in the row: KI-323 (the plan-evidence convention now
      written under Rules in `docs/plans/README.md`) and KI-414 (the `srcdoc`
      mount is already the shape its constraint forces; the row is a recorded
      constraint, not open work).

      **What remains open is the owner's:** KI-412, KI-404, KI-406 carry
      recommendations with named defaults in C12 and wait on the call. KI-252,
      KI-396, KI-455, KI-456 and KI-459 are open on purpose — KI-252 is narrowed
      to the one piece genuinely unbuilt, the gate rows are dev-decisions' to
      fix (D-001/D-002), and KI-459 is the instrument that makes KI-252's next
      measurement meaningful.

## Files to be touched

**Backend / API:**
- `src/agent.rs` — the nomenclature guard's matcher (`WriteGuard`, ~7910-8330);
  the mock-server resolution at 9332
- `src/route.rs` — `failure_class` (2656) and one arm beside
  `names_model_unavailable` (2698)
- `src/tools.rs` — the `supersede` argument contract (~1404) and its refusal
- `src/memory.rs` — the probe-retry fixture (6054-6112)
- `src/cli.rs` — `parse_remember` (2102) and `RememberOptions` (356)

**Scripts:**
- `scripts/plan_check.py` — the invocation seam C9 names
- `scripts/ki_ledger.py` — only if the perpetuity shape C10 needs it

**Tests:**
- `tests/layerfs.rs` — the stub trio's budget
- `tests/blend1.rs` — the doctor pair's wait (348)
- `tests/cli.rs` — the quoted-body pin for C5
- `src/agent.rs` `mod tests` — the `CARGO_TARGET_DIR` case for C2

**Documentation:**
- `docs/known-issues.md` — 28 rows re-verified, then closed with evidence
- `docs/features/install.md` — the reload block's post-state check (C6)
- `TECHNICAL-DOCUMENTATION.md` — the nomenclature guard's admission rule and
  the delivery seam, once built

## Out of scope

- **Building on the four decision rows before they are ruled.** KI-406, KI-412,
  KI-404 and KI-414 get recommendations in this plan; their builds wait for the
  owner's answer, because each names a call the row itself says is not the
  session's.
- **The `dev-decisions` fix for KI-396.** Fleet files it and points at it; the
  cap is another repo's code and changing it from here would be the wrong
  repository.
- **Re-engineering the nomenclature guard.** C3 admits the canonical spelling;
  it does not rebuild the vocabulary derivation. If the class needs more than
  the admission rule, that is a new row and a new plan.
- **The pending KI-252 vector work.** Its own plan
  (`2026-10-24-deck-vector-moves-2-3.md`) holds that scope and its measurement
  gate has not been met (0.830 against a 0.98 floor at 50 facts, 500 the floor).
  This plan records it as perpetuity, not as work to start.
- **Rebuilding or relaunching the native `.app`/dmg** unless C6 or C12 changes
  the delivery path.

## Sequencing and why

1. **C0 first.** A stale `open` row wastes every later build. KI-326 already
   proved the premise wrong once.
2. **Class B (C1, C2) before anything else that runs the suite.** Every later
   criterion is proven by a `cargo test`, and a load-sensitive fixture makes
   that proof unreliable exactly when this plan adds parallel work. Fixing the
   instrument before using it.
3. **C3 before C12's document work.** The guard's false positives drove this
   session's own writes to `edit` and bash — it refused this very plan file
   once — and leaving it open while writing four decision documents invites the
   same detour.
4. **C4, C5 together.** Both are tool contracts that announce a path they do
   not take; one review pass.
5. **C6, C7, C8** in any order — independent surfaces.
6. **C9, C10, C11 last of the builds.** Record-keeping that describes the work
   this plan did, so it can be written from what landed.
7. **C12 threaded, not saved for the end.** The four decisions are the owner's
   queue, not this session's; presenting them early costs nothing and unblocks
   whatever the answers permit.

## Verification

**C0 — the triage is evidence, not a claim.**
```bash
python3 - <<'PY'
import sys; sys.path.insert(0,'scripts')
import ki_ledger as k
from pathlib import Path
header, rows = k.parse(Path('docs/known-issues.md'))
print(len([r for r in rows if k.status_word(r[2][4]).lower()=='open']))
PY
```
Expected: 28 today (25 originally open plus KI-454/KI-455/KI-456); the triage table
names each with the file and line that decides it.

**C1 — load sensitivity.** Run with the burners the KI-385 discipline names:
```bash
for i in 1 2 3 4; do (yes > /dev/null &) ; done
cargo test the_probe_retries_after_a_pause a_failing_command_still_checkpoints \
  a_hung_command_is_discarded_at_its_budget a_refusal_carries_its_own_code_and_message \
  a_doctor_call_names_every_lanes_availability_never_a_hang
```
Expected: green under load. A fix that only passes on a quiet machine has not
fixed the class.

**C2 — shared target dir.** Expected: green with
`CARGO_TARGET_DIR=$HOME/.cache/fleet-gate` and no `target/` in the manifest.

**C3 — the guard.** Expected: `write_file` on a file naming `Usage`, the mono
variant, `One`, `Deserialize` lands, and a genuine drift (`judgeRoute` beside
`judge_route`) is still refused — the admission must not disarm the guard.

### Verification, continued (C4–C13)

**C4 — supersede.** Expected: a `description` with no `replacement` is refused
naming the missing key; a real forget's sentence says the name is dead.

**C5 — the quoted body.** Expected: the fact files and reads back byte-equal.

**C6 — the reload.** Expected: after the two-command reload,
`launchctl print gui/$(id -u)/com.fleet.gui` shows a pid and
`curl -s localhost:3080/ -o /dev/null -w '%{http_code}'` answers 200.

**C7 — the child render.** Expected: a child's header carries its project and
an `fs read main /<path>` the parent can run unchanged.

**C8 — the walk.** Expected: a 400 whose body names account state is
failover-eligible and the walk reaches the next candidate.

**C9 — the hook.** Expected: `python3 scripts/plan_check.py` is named in the
session's checklist and exits 1 on a planted unticked criterion.

**C10, C11 — the record.** Expected: the ledger's status cells name the
trigger; KI-396's row points at the sibling repo's file and line.

**C12 — the decisions.** Each row carries a recommendation and a default; the
owner's answer flips it to a build criterion or an explicit defer.

**C13 — the ledger.** Expected: `python3 scripts/ki_ledger.py` exits 0, every
cell count agrees with the header, and open is below 25.

## Linked artifacts

- `docs/known-issues.md` — 28 rows re-verified and closed with evidence; KI-403
  and KI-252 restated as perpetuity with their triggers
- `docs/features/install.md` — the reload block gains the post-state check
  (C6) and the reconcile stance the owner rules on (KI-412)
- `TECHNICAL-DOCUMENTATION.md` section "nomenclature.rs" (line 474) — the
  section says the module "names the project's established terms and warns on
  drift", but the guard REFUSES the write (`src/agent.rs:5999`, `Refused` → the
  tool returns not-landed), so the sentence is wrong today and says the fix for
  C3 is the admission rule; plus the delivery seam (C6/C12). Correcting this
  sentence is C0's doc half.
- `FUNCTIONAL-SPECIFICATIONS.md` — only if C12 changes what the owner is
  promised about the served face
- `docs/plans/README.md` — the plan-completion check C9 names

## Risks

- **The load-sensitivity class is a moving target.** KI-385, KI-405, KI-426 and
  KI-452 are four sightings of one discipline failing to hold. C1 fixes the
  three live fixtures and pins the rule; a fifth sighting means the rule is not
  being applied at review time, and the answer then is the checklist, not
  another fixture.
- **The nomenclature guard is load-bearing.** It refuses writes for a reason
  (KI-226, KI-284, KI-333). C3 admits exact canonical spellings only — and its
  verification requires a genuine drift to *still* be refused, so the fix
  cannot quietly disarm the guard.
- **KI-326 shows rows go stale.** If more than one row is already fixed, the
  triage shrinks the plan. C0 exists to find that before the build order is
  committed to, and the Progress table records it.
- **KI-412 is the owner's call and blocks a real symptom** — the served face
  lagged the code once already. It is presented first among the decisions.
- **Gate cost.** The full suite is ~2078 tests and takes minutes; this plan runs
  it more than once. Match depth to the change per the standing ruling: full
  suite for the class fixes, the touched module's tests otherwise.

## Dependencies

- `docs/plans/2026-11-02-open-work-closeout.md` — the precedent shape; its
  remainder rows are already closed, so there is no overlap.
- `docs/plans/2026-11-01-ledger-tooling.md` and `scripts/ki_ledger.py` — the
  checker this plan's C13 runs.
- `docs/plans/2026-10-24-deck-vector-moves-2-3.md` — holds KI-252's real scope
  and its unmet measurement gate.
- `~/Projects/dev-decisions` — `scripts/devdec/gates.py:228,418`, KI-396's home.

## Notes

- All 28 rows were read from the ledger's own parser
  (`scripts/ki_ledger.py`), not by hand-splitting the markdown — this session
  already made that mistake once, reading five rows as malformed when the
  checker was evaluating them correctly.
- KI-326's live proof:
  `FLEET_EMBEDDING_ENDPOINT=http://127.0.0.1:8901/v1/embeddings cargo test --lib
  an_env_write_is_readable_by_the_runtime` → `1 passed`. The ambient value that
  used to red it is exactly what was exported.
- **This plan's own drafting is evidence for KI-415.** Writing this file was
  refused once by the nomenclature guard for naming the SF font family's mono
  variant — the identical false positive KI-415 records. The file was written
  through bash, which is the documented work-around, and the refusal is the
  clearest available proof that C3 is real work and not a tidiness ticket.
- KI-446's row notes a detached-worktree gate is the standing rule and a shared
  target dir the obvious way to avoid a second full build — which is why C2 is
  sequenced early: it makes every later gate run cheaper.
- Four rows name their own blocker as an owner decision. They are grouped in C12
  rather than scattered, so the owner answers once.
