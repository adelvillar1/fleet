# Session recap — 2026-11-06: the open ledger, closed by class

**Summary.** The owner asked for a plan to address every open KI ledger row,
then "implement plan as designed". The plan
(`docs/plans/2026-11-05-open-ledger-closeout.md`, **14 of 14 criteria
ticked**) landed in `a1502af`, merged master into `face-plugin-host`
(`80df123`), and then the merged-tree gate found **two real defects the
plan's own gate runs had never reproduced** — fixed in `f0c4a4b`.

The ledger went **28 open → 5 open**, and every row that closed carries the
code read or the live run that closed it.

## What the plan did

Twenty-seven rows read as **nine classes**, not twenty-five independent
jobs, following the `2026-11-02-open-work-closeout.md` precedent. The
difference it had to argue: that plan absorbed *plan residuals*, work
specified and not built; this one absorbed *ledger rows*, defects and gaps
recorded as they were found, so the rows arrived with findings attached and
the work was deciding what each finding actually cost.

- **A** (KI-395/415/428) — the nomenclature write guard refuses a spelling
  written across two files (`MAX_SPELLINGS_PER_TERM=8`,
  `SPELLING_CONVENTION_FILES=2`).
- **B** (KI-405/426/452/446/326) — the load-sensitive fixtures: one attempt
  count, a version read bounded in **seconds not attempts**, a doctor probe
  bounded at 20 s (was 184.1 s measured), and `CARGO_BIN_EXE` resolution
  through `sibling_binary`/`walking_up` so an integration test finds the
  mock binaries.
- **C** (KI-402/409/396) — `supersede` refuses a missing replacement and
  keeps phrases; `fleet remember` had **no file route at all**, so
  `--body-file`/`--description-file` landed beside a writer that emits valid
  YAML.
- **D** (KI-431/412) — `src/reload.rs` + `fleet reload`: install the binary,
  restart the agent, then **verify the post-state** (a pid and an answering
  port). Install copies; it does not restart.
- **E** (KI-320/327) — the child report names its project and the command
  that reads its work back; a refused publish is its own outcome.
- **G** (KI-451) — an account-state 400 fails over instead of ending the
  walk.
- **H** (KI-317/323/398/403/252) — the plan check wired to a pre-commit hook,
  the plan-evidence convention written down, KI-403/252 restated with the
  triggers that make them re-checkable.

## The two defects the gate found afterwards

Both reddened **only under the full suite**, both passed in isolation, and
both traced to `ef0dc96` — pre-existing, an ancestor of both master and this
branch. Neither file had been touched by the plan's work.

**KI-463 — the related endpoint built its whole corpus on the request path.**
`RelatedIndex::corpus` called `build_corpus`, which reached
`Memory::load_with` with the process embedder; `load_with` embeds every fact
whose cached digest does not match. The index is constructed against
`resolve_root(cwd)`, so the corpus it rebuilt was **the checkout's own
`memories/`** — KI-462's shape, one day later. The module doc already
promised the opposite ("the warm task owns the cold half").

It hid for a nameable reason: the process-global probe answers `KeywordOnly`
until some earlier test resolves it. Idle, the build embeds nothing and the
click looks fast. Under the suite the vector path is live, the first click
runs a whole corpus build inside `spawn_blocking`, and
`the_related_route_answers_one_document_and_refuses_a_foreign_kind` blows
its 60 s reply deadline and panics on an empty message.

`corpus()` now takes **no embedder at all** — the cold builder is
unreachable from it by signature rather than by discipline — and reads the
new `Memory::load_cached` (vectors whose cached digest matches; nothing
embedded, nothing written). `build_corpus` keeps the cold half and now ends
by calling the same read, so both paths rank one set.

**KI-464 — the telegram test waited on half its own assertion.** It polled
`fake.sent` then immediately read the activity row that `note_reply` writes
**after** the send (`src/channels/telegram.rs:200-202`). The row had not
landed when the send became visible: a race an idle machine loses by
microseconds and a loaded suite wins. Raising the 10 s budget would have
hidden it — the loop was watching only half of what the assertion reads.
It now polls both observables and exits only when both exist, on a **poll
count**, not a wall clock.

## The verification

- **The plan's own run** (`a1502af`): 1927 lib / 168 integration.
- **After the two fixes, on the committed tree** (`f0c4a4b`, detached
  worktree, own target dir): `build --all-targets` exit 0 with **zero
  warnings**; lib **1929 passed / 0 failed / 3 ignored**; integration
  **168 / 0 / 4**; face harness **962 checks across 110 cases**; ledger
  **465 rows, 5 open**; `plan_check` exit 0. All three named tests confirmed
  `ok` in that log, including the two that had failed.
- **The red-proofs**: restoring `build_corpus(...)` inside `corpus()` failed
  the new pin with the offending body printed (`FAILED. 0 passed; 1
  failed`); KI-460's needle reverted to its pre-C7 form returned the
  identical integration failure; KI-461's old any-quote rule returned the
  exact phrase collapse.

## Found on the way

- **KI-465, a gate-procedure flaw mine to own.** A full-suite run piped
  through `tail -40 | grep` kept the summary line and dropped the
  `---- name stdout ----` block, so the run's failing test names were
  **destroyed by the command that reported them** — "two failures" with no
  way to name them short of a whole re-run. The exit code survived only
  because it was read with `${PIPESTATUS[0]}`. The standing rule covered the
  *status*; a second, unexamined pipe discarded the *names*. A gate run's
  output now goes to a file whole and the summary is read from that file.
- **KI-462 — the board-fact root leaked into the owner's corpus.** Under
  `cargo test` an unpinned root fell through to the checkout's real
  `memories/`; seven `design-board-*` facts and two recall-count bumps were
  written. Fixed at the class (an unpinned root returns
  `home.join("board-facts")`), leaked facts deleted, counters restored.
- **The disk filled twice again** (`/System/Volumes/Data` 425 Gi of 460);
  ~70 GiB reclaimed from `/tmp` targets. The checkpoint-retention leg is
  still the real fix and is still open.
- **A `bash` call with `run_in_background: true` orphans its grandchild** — a
  `cargo test` started that way died mid-run and wrote a truncated log. Full
  suites run in the foreground.

## Where it stands, honestly

- **Shipped:** `f0c4a4b` pushed to `origin/face-plugin-host` and
  `origin/master` (both at `f0c4a4b`).
- **Delivered:** the release binary built from the committed tree and
  installed to `~/.fleet/bin/fleet`, **byte-identical** to the source build
  (sha256 match, 18:48).
- **The ledger:** 465 rows, **5 open** — KI-252 (semantic clustering,
  genuinely unbuilt), KI-396/KI-455/KI-456 (filed against
  `~/Projects/dev-decisions`), KI-459.
- **Two corrections worth keeping.** My first two attempts at the
  request-path pin were bad: one asserted `!x.is_empty() || x.is_empty()`
  (vacuous), and the next depended on the probe, so it would have passed
  *with the bug present* — exactly how the defect hid. The landed pin is a
  source-shape check in the `explain::tests` mould, because the runtime
  observable genuinely cannot see this on an idle box. And my first draft of
  the fix's doc comments stated a mechanism I had not read (`build_corpus`
  embedding skills); reading the HEAD source showed skills are the warm's,
  and the comments were corrected to the facts.
