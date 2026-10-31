---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: beta-shakeout
---

# Plan: the beta shakeout — find the bugs the features' own gates could not

## Context

Fleet is beta by the owner's declaration (2026-10-30): what they wanted
is there, it looks and works fine, and *"time to find bugs"*. The
suite's 1,842 tests and the face's 474 checks prove what their authors
thought to assert; a shakeout hunts where that empathy ends — the edges
between features, the inputs nobody typed twice, the states a surface
can be in when another acts. The find is the deliverable: every bug
lands in `docs/known-issues.md` as a row with its evidence, the ledger
discipline the repo has run since slice one. Fixes follow the ledger's
own triage — crashers and data-loss first.

## Approach

**A manifest, walked and re-walked — not a checklist of features.** A
find is the intersection the features' own tests never reached: the
surface × the flow × the hostile input. The manifest below is the
contract a shakeout run executes — every probe carries its exact
action, its input, and what "held" means, so a fleet run (or a human)
walks rows, records verdicts, and files finds without re-deciding what
thorough means.

### The hostile corpus (shared inputs, defined once)

| Tag | Input |
|---|---|
| H1 | the giant: 100 KB of mixed prose + emoji + `<script>` + quotes + newlines |
| H2 | the controls: `\u0000`, `\u0007`, an ANSI escape, an RTL override |
| H3 | the regex: `[]{}()*+?^$|\\` and a catastrophic backtracking pattern |
| H4 | the path-y: `../../../etc/passwd`, `%2e%2e`, `C:\\`, a NUL |
| H5 | the unicode: Zalgo combining marks, zero-width joiners, CJK, a 20k single word |
| H6 | the JSON-ish: `{"nested": {"deep": [1, 2, {"x": null}]}}` where a string belongs |
| H7 | the numbers: `-1`, `0`, `4294967296`, `NaN`, `Infinity` as ids, counts, days |
| H8 | the days: `2026-13-45`, `2026-02-30`, `1970-01-01`, `9999-12-31` |
| H9 | the empty: `""`, whitespace-only, a bare newline |

### The manifest

Verdicts: **held** (the probe's action completed without breaking its
surface; name what held), **find** (file the KI row before any fix),
**skip** (the row needs the owner's device or a state this machine
cannot reach — say which).

**A — the conversation and its run loop**

| # | Probe | Action | Held means |
|---|---|---|---|
| A1 | the giant task | send H1 as a run's task (scratch home) | the run starts or the input refuses with a named bound; the log records it |
| A2 | the draft survives | type H5 into the composer, reload | the draft replays byte-identical |
| A3 | the goal at the ceiling | POST a 1,999-char goal, then H1 | the 1,999 lands; H1 is refused naming 2,000 (KI-378's pin, live) |
| A4 | the goal cleared mid-run | set a goal, start a run, clear it | the running run finishes with its own goal; the next runs goalless, named |
| A5 | the steer with H3 | send H3 as a steer mid-run | queued or typed verbatim; the log carries it |
| A6 | the whitespace send | send H9 | refused or a no-op — never an empty run |
| A7 | two sends in one second | fire two sends | one drives, one queues, the names right |
| A8 | the seat with its session gone | approve after a face restart killed the seat's session | a named refusal, no hang |
| A9 | the verdict controls | like, dislike, fork, hook on one reply; reload | the sidecar keeps them; the fork's log prefixes byte-identical |
| A10 | the controls paste | paste H2 into the composer | the draft renders or names it; nothing wedges |

**B — the sessions rail and the archive**

| # | Probe | Action | Held means |
|---|---|---|---|
| B1 | the hostile rename | the pencil with H1, then H2, then H5 | 200, served verbatim, restorable; the row clips |
| B2 | the cleared rename | rename to empty | the ladder's next rung serves |
| B3 | the archive of a running session | archive the working row | refused or named — never silently |
| B4 | the resume of an archived session | resume the archived row | named, the conversation opens |
| B5 | the 20k query | H6 as the content query | held, the body intact |
| B6 | the backtracking query | H3 as the content query | held, no hang |
| B7 | the seat's session gone | open a needs-you row after its restart | a named refusal (KI-286's seam) |
| B8 | the fork of a one-turn session | fork the shortest owner row | the fork's log, the prefix rule byte-identical |

**C — memory**

| # | Probe | Action | Held means |
|---|---|---|---|
| C1 | the giant description | remember with H1 as the description | the writer's bound refuses, named |
| C2 | the giant body | remember with a 10k body | the body bound refuses, named |
| C3 | the hostile name | remember with H4 as the name | the name rule refuses, named |
| C4 | the hostile edit | edit a fact's description to H5 | the rewriter holds; the reload reads it back |
| C5 | the empty justification | supersede with an empty justification | the parser's refusal, named |
| C6 | the confirmed supersede | supersede a confirmed-by fact from the pane | the click is the approval; the row lands |
| C7 | the unreadable valid-until | a fact with H8 as valid-until | not an expiry — the cycle leaves it |
| C8 | the Zalgo search | H5 as the corpus search | held |
| C9 | the empty preview | the needs-you queue on an empty corpus | the honest empty |
| C10 | the archive serials | a corpus with `<name>.2.md` collisions | the read serves both, named |

**D — skills**

| # | Probe | Action | Held means |
|---|---|---|---|
| D1 | the double approve | approve the same proposal twice | the second names "already applied" |
| D2 | the gone proposal | approve after the file left | "no proposal named" — the named refusal |
| D3 | the regex search | H3 in the library search | held |
| D4 | the retire race | retire + unretire in one second | the one-writer holds; the last write wins, named |
| D5 | the hostile skip | a SKILL.md with H5 in its frontmatter | the skip reason named in its own words |
| D6 | the unmatched name | a skill the table does not carry | `everything else` serves it, visible |

**E — calibration**

| # | Probe | Action | Held means |
|---|---|---|---|
| E1 | the hostile day | `?day=` with H8 | the CLI's words, named (pinned) |
| E2 | the midnight edge | the stepper across 23:59 UTC | the day math holds |
| E3 | the lane down | the gate review ask with sys1 unreachable | the named absence, never a hang |
| E4 | the zero-confidence day | a day of rows without confidence | the cells name their zero rows |
| E5 | the 512 cap | a busy day (512+ rows) | the strip names "the day's last 512", the sections agree |

**F — workspace history**

| # | Probe | Action | Held means |
|---|---|---|---|
| F1 | the path-y project | `?project=` with H4 | refused by name |
| F2 | the escaping path | the files read with `..` out of the root | refused by name (the pane's own rule) |
| F3 | the NotComparable ref | a diff whose ends are Layer and Commit | the substrate's named refusal |
| F4 | the dead worker | a read with the worker stopped | the substrate's own words, before the script runs |
| F5 | the idle-chip click | click a branch nobody worked | named or no-op — never a broken pane |

**G — the seams**

| # | Probe | Action | Held means |
|---|---|---|---|
| G1 | the CLI and the face, one home | `fleet remember` while the memory panel repaints | the flock holds; no torn read |
| G2 | two writers, one home | two fleet processes writing the same sidecar | serialized, the last wins, no corruption |
| G3 | the restart mid-flow | kickstart the face with a drawer open and the phone polling | the page recovers on its poll; the session re-handshakes |
| G4 | the relay's hostile path | H4 as a session path at the relay origin | refused at the gate (the owner's device, or the live probe's role) |
| G5 | the credentials absent | a run with no key anywhere | the named absence before any model call |
| G6 | the read-only home | a face over a read-only `<home>` | every read holds; every write names the refusal |
| G7 | the clock edge | a run across UTC midnight (the day files) | the rows land in the day they belong to |

**H — the CLI, headless**

| # | Probe | Action | Held means |
|---|---|---|---|
| HH1 | the hostile day | `fleet calib --day` with H8 | the typed refusal (pinned) |
| HH2 | the path-y draft | `fleet skills --show` with H4 | the bare-name refusal (pinned) |
| HH3 | the giant remember | the CLI remember with H1 | the bounds refuse, named |
| HH4 | the eval on the mock | `fleet eval` | the score lands, the row written |
| HH5 | the second onboard | `fleet onboard` on an onboarded home | named, idempotent-ish |
| HH6 | the scratch doctor | `fleet doctor` on an empty home | the seven facts, the absences named |
| HH7 | the hostile day archive | `fleet memories --consolidate` with H8 valid-untils | the unreadable dates left alone, named |

The TUI's rows stay owner-driven (a terminal surface a headless run
cannot drive); the phone's relay rows need the owner's device or the
live probe's role — marked **skip** with the reason until one is at
hand.

### Running it through fleet

A shakeout run is a fleet run whose task is this manifest:

- **The task**: *"walk the beta shakeout manifest
  (docs/plans/2026-10-30-beta-shakeout.md) pass N, rows <the ids named>,
  recording each verdict in the plan's pass table and filing every find
  as a KI row with its repro and evidence."*
- **The isolation**: state-changing probes (renames, goals, retires,
  supersede) run against the owner's rows only with an immediate
  restore, or against a scratch home (`FLEET_HOME` pointed elsewhere)
  when the probe does not need the live face. A probe that cannot
  restore skips and says so.
- **The budget**: probes that would spend model calls (A1's giant send)
  run against the scratch home's failover tail, or wait for the
  owner's go at the live face.
- **The filing**: one KI row per find, its repro and evidence quoted;
  `python3 scripts/ki_ledger.py` after each batch — the pass owns the
  ledger during its walk (two writers filing at once is KI-285's shape).
- **The passes**: pass one walks fast and shallow, whole; pass two goes
  deep on whatever shook loose; pass three re-runs the fixed rows
  (B4).

## Acceptance criteria

- [x] **B1 — the manifest is walked once, shallow, whole.** Every
      probe in the manifest carries a verdict in its pass table —
      held (with what held), find (with its KI row), or skip (with the
      reason) — and no row is left unattempted without a named skip.
      *Pass one (2026-10-30, eight shallow probes) and pass two
      (2026-10-31, the whole manifest deep) both walked; the pass-two
      table below carries every verdict with its evidence.*
- [x] **B2 — every find is a row.** Each bug carries a KI row with its
      repro and evidence before any fix is written; the ledger checker
      stays green. *KI-378, KI-380, KI-381 — all three filed, fixed,
      and closed in the walks that found them; the checker reads 381
      rows, 34 open, zero problems.*
- [x] **B3 — crashers and data-loss fix first.** A find that loses
      work, corrupts a file, or wedges the face is fixed (with its pin)
      before cosmetic finds are touched. *KI-380 (the zombie tab, the
      owner's own real-use find) fixed at four surfaces with the face
      harness's case; KI-381 (onboard unreachable, a crasher) fixed
      with its end-to-end pin within the session that found it.*
- [x] **B4 — the deep pass re-runs the fixed rows.** A fix's repro is
      re-run live and the harness carries its pin; the matrix's row
      moves to "held". *KI-378's goal bound re-run live in pass two
      (A3); KI-380's recovery paths pinned by the harness's KI-380
      case (482 checks / 52 cases); KI-381's walk pinned end to end.*
- [ ] **B5 — the beta ships.** The pass ends with the matrix whole, no
      open crasher, and the shakeout's summary in the recap — the beta
      gate the owner's declaration set. *The matrix is whole and no
      open crasher remains; the recap is this session's, and pass
      three (the re-walk after the owner's restart of the face and
      their own use) is what closes the box.*

## Files to be touched

**Everything is in scope for a find; fixes land where the bug lives.**
The plan touches only: `docs/known-issues.md` (the finds), this file
(the matrix), and the recap.

## Out of scope

- **New features** — the bug budget comes first; an idea found while
  hunting is a note, not a branch.
- **Refactors** — a fix reshapes the minimum the pin needs.

## Verification

The matrix table in this file, walked; the ledger checker green; the
gate per fix.

## Pass one — the matrix, walked 2026-10-30

| Probe | Surface × class | Outcome |
|---|---|---|
| S1 a 100 KB paste (emoji/script/quotes) into the composer | conversation × enormous inputs | held — the draft carried 131k chars; the task's send bound filed beside KI-378 as the same class |
| S2 a 10k-char goal through `POST /api/goal` | conversation × enormous inputs | **FIND — KI-378**: accepted and stored; the standing objective rides every run's prompt. Fixed: the writer refuses past 2,000 chars, named; pinned |
| S3 a 2 KB emoji/newline/HTML rename | sessions × hostile characters | held — 200 and restored 200; served verbatim (the earlier 409s were the probe's own wrong shape) |
| S4 regex + control characters into the memory search | memory × hostile characters | held |
| S5 four rapid section flips mid-read | all rails × state collisions | held — no page errors |
| S6 a pane card open while the panel flips | all rails × state collisions | held — the card closed with the panel |
| S7 the stepper four days back | calibration × boundaries | held — the honest empty state ("2026-10-26", no rows named) |
| S8 a 20k-char search | memory × enormous inputs | held — the body intact |

The find's fix: `Bridge::GOAL_MAX_CHARS` (2,000) at `set_goal`, the
refusal naming the bound and why; pinned by
`the_goal_refuses_past_its_ceiling`. The task's send bound is the
owner's triage (a one-shot spend, not a standing tax). Pass two goes
deep on whatever the owner's own use shakes loose.

## Pass two — the matrix, walked deep 2026-10-31

Walked by the session itself (the runbook's "a shakeout run is a fleet
run" taken literally), scratch-home isolation except where the row
needs the live face; model-spend rows rode the keyless scratch face so
every failure was a named absence, not a bill. Two finds, both fixed
with pins in the same session; everything else held, with what held
named. Rows covered by a pinned test rather than a live probe say so.

| # | Probe | Outcome |
|---|---|---|
| A1 | the giant task (100 KB H1) | held — the wire cap refuses: "a body of 156751 bytes; the cap is 65536" (the scratch face; zero spend) |
| A3 | the goal at the ceiling | held live — 1,999 lands; H1 (102 KB) refused at the wire cap first, then the 2,000 bound is the second wall (KI-378's pin re-run, B4) |
| A4 | the goal cleared | held — the clear answers 200, `text: None`; the run-side is the bridge's per-run read |
| A5 | the steer with H3 | held — idle steer refused named: "no run is in flight to stop or steer" |
| A6 | the whitespace send | held — "a run needs a task" (409) |
| A7 | two sends in one second | held — one 200 accepted, one 409 "a run is already in flight" (the face queues; the API refuses) |
| A8 | the seat with its session gone | held — "no approval is waiting" (409), no hang; `AnswerError::NoSeat` pinned in the bridge tests |
| A2/A9/A10 | draft, verdict controls, controls paste | held — pinned by the face harness (the draft case, the TURN case, pass one's S1) |
| B1–B2 | hostile rename, cleared rename | held live on my own probe session — 2 KB emoji/HTML/control titles served verbatim and restorable; the clear falls to the ladder's next rung (`title_from: "first"`, rendered as "a conversation · when") |
| B3 | archive a running session | held — the guard names it: "has a run in flight — stop it first"; it even caught this probe's own wrong-shaped fork aimed at the owner's live session |
| B4 | resume the archived session | held — the body-shaped resume reopens it; restore via `archived: false` |
| B5–B6 | the 20k query, the backtracking query | held — `sessions --recall` with a 40 KB emoji/JSON query and `(a+)+$`: exit 0, ranked, no hang |
| B7 | the seat's session gone | held — "no session named" pinned (the bridge's `nobody` test) |
| B8 | the fork's byte-identical prefix | held — the 0-owner fork's records match the source's byte for byte; `forked_from` rides the start line |
| C1–C2 | the giant description / body | held — description cut at the marked 512; body refused past 64 KB named (the CLI writer path, HH3's live probes) |
| C3 | the hostile name | held — `../../../etc/passwd` and `..` refused by the name rule, named |
| C4–C5 | the hostile edit, the empty justification | held — sanitize + marked cut pinned (`the_writer_bounds_the_type_and_the_justification`); whitespace justification → the typed error pinned |
| C6 | the confirmed supersede click | held — the bridge's `fact_supersede(.., true, ..)` is the click-is-approval path |
| C7 | the unreadable valid-until | held live — `2026-13-45` left live by the cycle (HH7); not an expiry |
| C8 | the zalgo search | held — the pool ranked, the surface fallback named |
| C9 | the honest empty | held — 0 facts reads "no facts yet · a fact is memories/<name>.md …" |
| C10 | the archive serials | held — `<name>.md` and `<name>.2.md` both served |
| D1–D2 | the double approve, the gone proposal | held — already-disposed refused with its status label; gone named "no proposal named … awaiting" (pinned in `improve`'s match) |
| D3 | the regex search | held — `skillMatches` is an `indexOf` (linear; no engine to hang), verified with the catastrophic patterns |
| D4 | the retire race | held by construction — pid-suffixed temp + rename = whole-file swap; last writer wins, never torn |
| D5 | the hostile frontmatter | held — skip reasons named in their own words (the live library's skips) |
| D6 | the unmatched name | held — `use_skill` names the miss; over-cap skills named by the growth line |
| E1 | the hostile day | held both faces — CLI exit 2 with the typed refusal (HH1); the face read refuses with the CLI's own words |
| E2 | the far dates | held — 1970-01-01 and 9999-12-31 serve the honest empty |
| E3 | the lane down | held live — the scratch face's run: "sys1 gave no usable verdict … walks hard's chain … no usable roster candidate remained" — degraded and named, never hung |
| E4 | the zero-confidence day | held — the scratch day's failed turn renders its counters honestly |
| E5 | the 512 cap | held live — 2026-10-26 (3,600+ rows) reads `rows_read: 512, cap: 512`, the document names the cap |
| F1 | the path-y project | held — "no registered project at `../../../etc/passwd`"; the encoded form too |
| F2 | the escaping path | held — `view_path` refuses the escape by name (pinned); the live read hits the substrate's named absence first (F4's shape) |
| F3 | the NotComparable ref | held — "the left ref layer:abc cannot ride a diff: ref must be a saved Commit or Layer id" |
| F4 | the dead worker | held — the substrate's own refusal before any script runs |
| F5 | the idle-chip click | held — the 0-commit branch serves an honest files page |
| G1–G2 | two writers, one home | held live — 8 distinct + 4 same-name concurrent writers and a reader loop: 9/9 facts whole, one winner, three named "already exists; the modify path is supersede", no torn file, no `.tmp` litter |
| G3 | the restart mid-flow | held — pinned by the face harness's poll/relay cases; the live face restart stays the owner's hand (KI-380's fix makes the recovery loud now) |
| G4 | the relay's hostile path | held at the gate — the relayed request rides the same route allowlist, refused by name before session resolution; the device rows stay the manifest's named skip |
| G5 | the credentials absent | held live — the keyless scratch face ran a real task to "no usable roster candidate remained" before any model call, zero spend |
| G6 | the read-only home | held — reads hold (exit 0); the write names the os error at its own tmp file |
| G7 | the clock edge | held — the day files rollover verified (10/30 and 10/31 both populated; the day is computed once at record time) |
| HH1 | the hostile day | held — typed refusal, exit 2 (the pin stands) |
| HH2 | the path-y draft | held — "not a proposal name: a name carries no path separator", exit 1 |
| HH3 | the giant remember | held — the marked 512 cut, the body under its bound; the fact lands whole |
| HH4 | the eval on the mock | held — 21 scenarios, score 1.00, green |
| HH5 | the second onboard | **FIND — KI-381**, fixed: the missing `needs_runtime` arm; the walk + resume pinned end to end |
| HH6 | the scratch doctor | held — the seven facts with the absences named; exit 1 is the honest missing credential |
| HH7 | the hostile archive | held — the unreadable date left alone, the far date untouched, the expired compressed→archived, every action named |

**Pass two's tally: 2 finds (KI-380's disposition + KI-381), 52 rows
held, 1 named skip (the device-side relay rows).** The beta's open
crasher count is zero.

## Notes

- The live face runs under the LaunchAgent — a find that wedges it is
  restarted with `launchctl kickstart`, and the find is filed with the
  restart named.
- The phone's relay origins are the owner's device; the matrix's relay
  rows wait for the owner's hand or a scratch device.
