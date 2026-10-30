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

- [ ] **B1 — the manifest is walked once, shallow, whole.** Every
      probe in the manifest carries a verdict in its pass table —
      held (with what held), find (with its KI row), or skip (with the
      reason) — and no row is left unattempted without a named skip.
- [ ] **B2 — every find is a row.** Each bug carries a KI row with its
      repro and evidence before any fix is written; the ledger checker
      stays green.
- [ ] **B3 — crashers and data-loss fix first.** A find that loses
      work, corrupts a file, or wedges the face is fixed (with its pin)
      before cosmetic finds are touched.
- [ ] **B4 — the deep pass re-runs the fixed rows.** A fix's repro is
      re-run live and the harness carries its pin; the matrix's row
      moves to "held".
- [ ] **B5 — the beta ships.** The pass ends with the matrix whole, no
      open crasher, and the shakeout's summary in the recap — the beta
      gate the owner's declaration set.

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

## Notes

- The live face runs under the LaunchAgent — a find that wedges it is
  restarted with `launchctl kickstart`, and the find is filed with the
  restart named.
- The phone's relay origins are the owner's device; the matrix's relay
  rows wait for the owner's hand or a scratch device.
