---
status: active
created: 2026-10-31
updated: 2026-10-31
slug: quota-failover-and-compaction-count
---

# Plan: the quota failover unlocks the walk, and the turn count retires

## Context

Two owner reviews from the beta shakeout's first real use, each
diagnosed in the source:

- **The router does not switch providers when the committed one reports
  quota exhausted.** `chat_stream` walks the fallback chain on any
  *uncommitted* failure — quota included — but the moment a candidate
  has served, `state.committed` is true and the next turn's failure is
  fatal with no walk at all. A provider that goes out of money after
  one good turn fails the run typed, every turn, until the owner
  re-pins. Quota is not a transient fault: it is a persistent state
  that switching recovers, which is the whole reason the chain has a
  tail. The walk's `failover_eligible` already advances on `quota`
  within a walk; the committed path just never reaches it.
- **The turn count reads as a freshness measure that it is not.** Turn
  count says how many model calls a run made — a measure of the loop's
  iteration, not the conversation's depth. A conversation folded five
  times is deep; a ten-turn conversation that never compacted is
  shallow. `SessionSummary` carries `compacted: bool` (folded at least
  once) but no count, and the surfaces that lean on turn count as a
  size marker — the replay line, the delegation tree cards, the
  delegation sub-line, `branchNote` — show a number that does not
  answer "how much conversation is here".

The owner's direction: *"the router is not switching providers
automatically from the fallback chain if there is one reporting that
quota is exhausted. also we need to remove the turn count, if anything
compaction count is the real measure of a conversation freshness."*

## Decisions (the owner's, dated and quoted)

1. **2026-10-31 — the quota unlock.** The committed path unlocks and
  walks when the committed candidate reports quota, instead of failing
  fatal. Everything else stays fatal-on-committed.
2. **2026-10-31 — the freshness measure.** Turn count retires from the
  surfaces that used it as size; compaction count replaces it.

## Approach

**The quota unlock, at the one place it belongs.** In the committed
path, a `quota` failure that has not streamed unlocks the walk: the
record is written as it is now, `state.committed` clears, and the walk
begins at the position *after* the candidate that just reported — so
the exhausted provider is not retried in the same walk. Every other
class on the committed path stays fatal (that is the "no silent
restart" design, and it is right for a transient fault); a mid-stream
failure stays fatal whatever the class, because a partial answer
cannot be resumed. Quota is the one class whose recovery is a
different provider, so it is the one class allowed to unlock.

**The compaction count rides the summary, and the swap is surface-level.**
`SessionSummary` gains `compactions: Option<u64>` — the count of
`Compaction` records in the log, read in the pass `summarize` already
runs. Wherever a turn count read as "how much conversation", the
surface now reads the compaction count; where the count is a budget
(`runTurns`, "turns left") or a compaction event's own words ("N turns
folded"), it stays — those are not freshness claims.

High-level steps:

1. **Q0 — the quota unlock.** The committed path unlocks on `quota`;
  the walk starts past the exhausted candidate; the pins.
2. **Q1 — the count.** `SessionSummary.compactions` in the existing
  pass; the pin.
3. **Q2 — the surfaces.** The turn-count surfaces swap to compaction
  count; the budget and the compaction-event lines are untouched; the
  harness case.

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### Q0 — the quota unlock

`Router::chat_stream`'s committed branch: on an error whose
`failure_class` is `"quota"` and whose `streamed` flag is false, clear
`state.committed`, set the walk's start to `position + 1`, and fall
into the walk instead of returning. The recorded attempt's outcome and
class are unchanged; the decision row's reason names the unlock (the
walk's clause or the class) so a reader sees why the order moved.

The pins: a committed quota failure unlocks and the next candidate
serves (the walk's decision row records two attempts); a committed
non-quota failure stays fatal; a committed mid-stream failure stays
fatal whatever the class; a quota failure on the *first* turn (not yet
committed) still advances the walk as it does today.

*Checks:* Q1.

### Q1 — the compaction count

`SessionSummary.compactions` carries the count of `Compaction` records
the log holds — one more count in the pass that already walks the
lines. `None` when the log is unreadable. The pin: a log with three
compaction records answers three; a log without answers zero (a real
zero, not an absent one, because "never folded" is a fact worth
stating).

*Checks:* Q2.

### Q2 — the surfaces

The turn-count-as-freshness surfaces swap to the compaction count:
`branchNote`'s done line, the delegation tree cards' `said` and the
root card's note, the delegation sub-line, and the replay line's turn
fallback. Each reads the compaction count as "N folds" (or the word the
surface's register wants), and a conversation with zero folds says so
rather than showing a dash. The turn *budget* (`runTurns`, "turns
left") and the compaction event's own line ("N turns folded") are
untouched — they are not freshness claims.

*Checks:* Q3, Q4.

## Use cases

- [x] A: A committed provider runs out of quota and the run advances to the next candidate without the owner re-pinning.
- [x] B: A conversation's surfaces answer how many times it has been folded, not how many model calls it made.
- [x] C: A never-folded conversation says so rather than showing a zero that reads as empty.

## Acceptance criteria

- [x] **Q1 — the committed quota unlocks the walk.** A committed
      candidate's quota failure (class `"quota"`, not streamed) clears
      `committed` and the walk advances past it to the next candidate,
      which serves; the decision row records both attempts and the
      reason names the unlock. Pinned by a unit test over the walk's
      decision and an integration test over the real HTTP walk.
- [x] **Q2 — every other committed failure stays fatal.** A committed
      non-quota failure (auth, server, timeout) returns the error with
      one attempt recorded; a committed mid-stream failure stays fatal
      whatever the class. Pinned by tests.
- [x] **Q3 — the summary carries the compaction count.** `compactions`
      counts the log's `Compaction` records in the existing pass; a
      three-fold log answers three, a fresh log answers zero. Pinned by
      a test.
- [x] **Q4 — the freshness surfaces swap.** The four turn-count-as-
      size surfaces read the compaction count; a zero-fold
      conversation reads as never folded, not as a dash; the turn
      budget and the compaction event line are untouched. Pinned by a
      harness case.
- [x] **Q5 — the gate.** fmt, clippy, build, test green by exit code;
      the harness count quoted in the commit.

## Landed 2026-10-31

Q1: `a_committed_quota_failure_unlocks_the_walk` — a committed 402
unlocks the walk and the next candidate serves, with the decision row
reading as two attempts; the walk starts past the exhausted candidate
(`commit_for_tests` is the integration test's seam for the committed
state, since driving it live needs a tool call to force a second turn).
Q2: the existing `a_committed_failure_records_the_provider_that_was_committed`
still pins the fatal path (a network class stays fatal on commit). Q3:
`the_summary_counts_the_compactions` — three folds answer three, a
fresh log answers a real zero; `SessionSummary.compactions` rides the
pass `summarize` already runs. Q4: the delegation tree's nodes carry
`compactions` (counted in `session_shape`'s whole-log read), the replay
read carries it, the sessions row carries it, and the four surfaces
read `foldsWording` — the tree cards' `said` and the root note, the
replay's turn fallback, and the live branch line drops the turn count
rather than inventing a number its event never carried. The face
harness, **485 checks across 53 cases** — the FOLD case pins the tree
cards reading "3 folds" / "never folded" and no "turns", and the live
branch line naming its outcome without the count. Q5: the gate whole —
fmt 0, clippy 0, build 0, **1,847 passed / 0 failed / 5 ignored**.

## Files to be touched

**Backend / API:**
- `src/route.rs` — the committed quota unlock; the pins
- `src/session.rs` — `SessionSummary.compactions`; the pin

**Frontend:**
- `src/gui/assets/app.js` — the four freshness surfaces

**Tests:**
- `tests/face/harness.mjs` — the compaction-count case
- `src/route.rs` tests — the unlock pins
- `src/session.rs` tests — the count pin

**Documentation (see Linked artifacts below):**
- `docs/features/routing.md` — the quota unlock beside the failover's own ruling
- `docs/features/gui.md` — the freshness measure
- `FUNCTIONAL-SPECIFICATIONS.md` — the run behavior

## Out of scope

- **A quota cache** — the router does not persist "this provider is
  out of quota" across runs; the walk recovers within the run. A
  cross-run cache is a named follow-up.
- **The mid-stream rule** — a partial answer cannot be resumed, and
  that stays fatal whatever the class.
- **The turn budget** — "turns left" is a remaining-budget telemetry
  field, not a freshness claim; it stays.
- **The compaction event's own line** — "N turns folded" describes the
  compaction, not the conversation; it stays.

## Verification

**Q1–Q2** — the walk pins:
```bash
cargo test --lib route
cargo test --test integration quota
```
**Q3** — the count pin:
```bash
cargo test --lib the_summary_counts_the_compactions
```
**Q4–Q5** — the face harness and the gate:
```bash
node tests/face/harness.mjs
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: every new check green; the count line quoted in the commit.

## Linked artifacts

- `docs/features/routing.md` — the quota unlock beside the failover
- `docs/features/gui.md` — the freshness measure beside the work list
- `FUNCTIONAL-SPECIFICATIONS.md` § "Run behavior" — the quota switch

## Risks

- **The unlock retries the exhausted candidate's position once if the
  start is not advanced past it.** The walk starts at `position + 1`,
  so the exhausted provider is not re-attempted in the same walk.
- **A quota failure that is actually a decode failure in disguise**
  (a provider reporting quota in a 2xx body) stays a decode failure and
  is fatal — the class table reads the status code, and a 2xx answer
  that does not parse is not a quota signal.

## Notes

- The diagnosis: `chat_stream`'s committed branch returns on any error
  without consulting `failover_eligible`; the walk's own loop does
  consult it. The asymmetry is the bug, and quota is the class whose
  recovery is a different provider.
- The turn count's surfaces were found by grep for `' turns'` across
  `app.js`; the budget and the compaction-event lines were read and
  excluded by the freshness rule, not by omission.
