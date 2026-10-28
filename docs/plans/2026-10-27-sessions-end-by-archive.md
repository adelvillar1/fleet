---
status: completed
created: 2026-10-27
updated: 2026-10-27
slug: sessions-end-by-archive
---

# Plan: sessions end only by archive

## Context

The owner's ruling (2026-10-27, in three steps the same afternoon): a
user-launched session is never ended by the agent. A model handback
("completed") and the turn cap ("max-turns") are not ends — the loop
records what it factually knows, and the session stays open to the
owner. The only end is the owner archiving the session. And because the
face had no archive verb, the archive is this plan's feature.

The trigger was the dogfood triage the owner asked for directly: the
sessions rail showed nothing after 2026-10-23, the owner's bot-PWA
session (`20261026T181734Z-76405ddd`) was unlisted, and fleet's own
diagnosis run had stamped itself `completed` while its last message was
a question back to the owner ("you restart Fleet once"). Two defects
one ruling: the rail's cap kept the wrong end of the archive (KI-334),
and the loop's end vocabulary claimed a completion the harness had not
verified.

Ground truth the design stands on (verified by code read this session):

- `summarize` reads `ended` from the log's **last line only**
  (`src/session.rs`), and a follow-up run appends to the same log
  (`Session::open`) — so an end record is already structurally soft;
  only the face's words treated it as final.
- The loop stamped `completed` purely on reply shape: a model reply
  with no `tool_calls` wrote `End { stop_reason: "completed" }`
  (`src/agent.rs`), taught by the system prompt, checked by nothing
  except the silent-stop repair (a bare "done" gets one re-ask). A
  fluent handback sailed through as completion.
- Interactive logs carry no `kind` on their start record; scheduled
  (`kind: scheduled`) and delegated (`kind: subagent`) logs do. That
  field is the discriminator everywhere below — no start-record format
  change.

The decisions the owner made when asked: archived sessions render as a
collapsed **Archived** group at the rail's bottom (rows stay findable,
never invisible), and the Archive verb sits on **any row** —
conversations and scheduled/subagent runs alike. The state-semantics
change (handback is not an end) applies to interactive sessions only.

## Increments

**C0 — the rail keeps the newest logs (predecessor).**
`sessions_read` capped the archive with `take(SEARCH_BOUND)` over an
oldest-first list — the OLDEST 200 logs, everything after 2026-10-23
cut from the rail and from `?q=` search alike (KI-334). The cap now
takes from the tail (reverse / truncate / reverse) so it keeps the
newest 200 and restores oldest-first order for the reverse render.
Search rides the same slice and inherits the fix. Landed with the test
`the_rail_keeps_the_newest_search_bound_logs`: 201 logs, the newest
leads, the oldest falls past the bound, a needle in the newest is found
with the bound named honestly.

**C1 — the log says what the loop knows: "handed back".**
`end_stop_word` (`src/agent.rs`): for an interactive session (no
`kind`), a model stop with no tool calls stamps **`handed back`**
(silence after the report repair: **`handed back without a report`**);
scheduled/subagent keep `completed` — there it is true, bounded work
whose report the caller consumed. `max-turns` is unchanged for all
kinds. The in-memory `StopReason` enum is untouched, so cron's outcome
card and subagent outcomes are unaffected. `Session` now carries the
kind its start record declares (written by `open_new`, recovered from
the log's first line on `open`), so a resumed log answers for the kind
it was born with. Recall's `outcome_word` (`src/session_recall.rs`)
signs `handed back*` as **Incomplete** — without the arm every
handback would sign Divergence and poison retrieval; a bounded run's
`completed` still signs Resolution. The face's `work_state` maps
interactive + ended + `completed`/`completed without a report`/
`handed back*` → **handed back** (an open word, rendered in the
owner's groups) — the ruling honored retroactively on the display
side, one match arm.

Named non-goal: recall's historical signs are not re-derived for
legacy interactive `completed` logs (a measured corpus change, its own
task). The display re-render above is the retroactive half.

**C2 — the archive: the one end.**
Registry `sessions-archived.json` in the home (`id → the archived
stamp`), loaded/saved exactly on the titles pattern (atomic
writer-unique temp rename, entries pruned to living logs, an empty
registry is no file). `POST /api/session/archive`
`{session, archived}` (beside `/api/session/title`; an absent
`archived` reads as archive): sets or clears the entry, records a face
event, returns the row's new state. Refusals name themselves: a run in
flight ("stop it first, then archive" — the stop verb is for that) and
a name the archive does not hold. `work_state` consults the registry:
an archived row is **`archived`** — the only terminal word a
user-launched session can wear — unless a run is actually in flight in
it (a resume), in which case the live word wins until that run is
back. Opening/replaying an archived session still works: an end, not a
deletion. Counts gain `archived`; `earlier` and `yours` exclude
archived rows; `total` stays everything scanned. Archiving clears the
session's standing goal — a goal on an ended session is debris.

**C3 — the face.**
`workStateWord` gains handed back / archived; the handback bar reads
the accent (quiet, open — not the down-gray of a died run). The rail
partitions archived rows into a collapsed **Archived** group at the
bottom, one line with the count on the background-runs toggle's
pattern, each row carrying **Unarchive**. Every live row carries a
small **archive** chip (keyboard-reachable, `stopPropagation` so the
row's own click still opens the conversation), and the conversation
head gains the archive button beside rename, flipping on the row's
current state. Refusals surface as a notice card in the stream.

## Acceptance criteria

0. A session created after the cap is the rail's first row; a needle
   in the newest log is found by `?q=` with the bound named
   (`the_rail_keeps_the_newest_search_bound_logs`).
1. The end word follows the log's kind: interactive → `handed back` /
   `handed back without a report`; scheduled/subagent → `completed` /
   `completed without a report` (`the_end_word_names_the_kind_the_log_was_born_with`).
2. An opened log answers for the kind it was born with
   (`an_opened_log_answers_for_the_kind_it_was_born_with`).
3. A handback signs Incomplete in recall, never Divergence; a bounded
   `completed` still signs Resolution
   (`a_handback_signs_incomplete_never_a_divergence`).
4. The state words: interactive legacy `completed` and any handback
   render **handed back**; scheduled/subagent render **completed**;
   archived wins over every log word except a run in flight; an
   approval outranks all
   (`the_work_state_names_what_needs_the_owner_first`).
5. The archive verb: archive → row `archived` + registry on disk +
   goal cleared; unarchive → computed state restored + registry gone;
   unknown id and in-flight run refused by name
   (`archiving_is_the_one_end_and_the_owners_verb`).
6. The face: the handback reads open, the collapsed group opens to
   unarchive chips, the row chip and head button post the verb, a
   refusal names itself in the stream (face harness case
   `theArchiveIsTheOneEndAndTheOwnersVerb`, 18 checks).
7. The gate is green: plain `cargo test` (debug profile), the
   `test result:` lines summed — the exit code alone is not the
   result.

## Files touched

- `src/gui/bridge.rs` — C0 cap, `work_state` + `handback_reason`,
  registry trio (`load_archived`/`save_archived`/`archived_path`),
  `session_archive`, counts, tests.
- `src/gui/server.rs` — `POST /api/session/archive`.
- `src/agent.rs` — `end_stop_word` + the end-write site, test.
- `src/session.rs` — `Session.kind` + `start_kind` + accessor, test.
- `src/session_recall.rs` — `outcome_word` arm, test.
- `src/settings.rs` — `set_tools_disabled_for_test` / `clear_tools_disabled_for_test`
  (KI-335's thread-local pin) and the structural half: `Agent::new` reads
  the disabled set from the run's own home (the session's), hermetic by
  construction.
- `tests/integration.rs` — the six run-loop tests' end vocabulary updated
  to the handback words (C1's ruling reaching the integration pins).
- `src/gui/assets/index.html` — the head archive button.
- `src/gui/assets/app.js` — word map, groups + Archived toggle, row
  chip, head handler, `archiveSession`.
- `src/gui/assets/app.css` — handback bar, archive chip.
- `tests/face/harness.mjs` — the stub key + the ARCH case.

## Out of scope

- Recall's historical re-signing (see C1's non-goal).
- Any CLI surface change: `fleet sessions` prints the summary's
  `stop_reason` raw, and `handed back` prints as itself.
- The LayerFS outage row and its self-recovery proposal: fleet's own
  parked session's work (`20261027T135549Z-70d7a54d`), whose cap-fix
  brief this plan supersedes. Its other owed work is untouched.
- Deleting, hiding, or auto-pruning archived logs: the archive is an
  end and a grouping, never a deletion.

## Verification

- `cargo test --lib` for the six new/rewidened Rust tests, then the
  full gate.
- `node tests/face/harness.mjs` for the face suite (263 checks / 28
  cases after this plan).
- Live read-back after the server restarts onto this build: the rail
  lists the 2026-10-26/27 sessions including the bot-PWA night
  session; yesterday's rows read `handed back`, not `completed`.

## Risks and open questions

- Legacy interactive rows whose `kind` recovery cannot parse (a log
  whose head is unreadable) read as the owner's own — the safe
  direction under this ruling (they render open, never completed).
- The remote face's `ALLOWED_POST` does not carry
  `/api/session/archive` yet: a paired phone cannot archive until the
  bot-PWA plan's allowlist work lands. Loopback (the console) is the
  surface this plan serves.
