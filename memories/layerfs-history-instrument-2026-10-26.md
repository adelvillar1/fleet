---
name: layerfs-history-instrument-2026-10-26
description: "2026-10-26/27 — the owner called fleet's layerfs graph "really underwhelming"; the measured comparison (upstream ships NO visual; the reference is the separate dsh-layerfs plugin) and the history-instrument plan IMPLEMENTED C0–C5 (472df99…6e3f062): one 8ms get_history read replaced a 4.8s fan-out, the tree in the reference's vocabulary, the live feed, the files instruments, the storage accounting — plus the pin that caught the spawn_with pipe-buffer deadlock"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Owner, 2026-10-26: "the graph and visual pieces of layerfs as they
were implemented on our end are really underwhelming, can we have a
second look at what the original layerfs has and what we have?"** The
second look agreed with them, and its first finding corrects the
premise: **the pinned upstream ships no visual at all.**
`Ephemeral-AI-Lab/layerfs@ad78914` (the sealed bundle we drive) is
verbs and JSON — `project` / `branch` / `fs` / `workspace` / `sandbox`
— plus `@layerfs/backend` and an `layerfs-sdk-observe` skill for
tracing/storage/daemon logs. No graph, no UI, no ASCII tree. The
visual experience lives in a DIFFERENT repo, `dsh-layerfs` (the
DeepSeek Harness plugin) whose README says it plainly: "the **Work**
and **History** tabs show every agent's Branch, live." Fleet's four
panes are named after those tabs — **we copied the nouns, not the
instrument.** This is the reusable lesson for any "match the original"
request: find which artifact actually carries the capability before
grading against the vendor repo.

**What the reference draws** (`packages/view/src/tree.js` 283 lines +
`treezoom.js` 133 + `history.js` 346 + `files.js` 289, fed by
`model/src/{derive,attribution,events}.js`): a vocabulary, from its own
legend — **band** on main = a Layer, **leaf** = a Commit, **bough** = a
Branch sprouting from a fork node at its ACTUAL base, **ring** = a merge
grafting back, **dashed stem** = a publish into a band, **bare twig** =
a Branch nobody is on. Time is event slots, not wall clock; lanes are
packed nearest-parent and keep their lane; per-branch colour; agent
heads at the lane end. Upright or sideways, 25–64% zoom + fit + pan, a
**minimap overview** with a position window, hover cards, `F` to follow.
The Work tab: branch chips with counts, a sandboxes board
(`944 calls · 133 cmds`), rows reading `C913 · ↓ L8`, a **store.stats**
panel (`8.90 GB on disk for 4.93 TB of history`, dedupe/compression,
the Init record, per-commit average/largest + histogram) and a **live
call feed** (`$ branch publish report-final2 · 2 ms`). Screenshots in
that repo: `demo/quarry/assets/{history-tree,history-sideways,work-store-stats}.png`,
`docs/tutorial/14-history-blame.jpg`.

**What ours did, measured on the live store** (not impressions):
`historyDag` was 76 lines — a spine, one fixed 250px column per
branch, every fork edge from one fixed Y, grey dots; no zoom, no
minimap, no author, no vocabulary. Four defects: (1) **silent
truncation** — `list_branches` with no page size meant the CLI default
50 against a store of 89+; (2) **clipped** — content width ≈ 12.4k px
against a card-width viewBox, so ~46 lanes off-canvas; (3) **forks
wrong** — `forked_from` parsed and never used; (4) **4.8 s** for 7 KB,
one `branch list_commits` subprocess per branch. And the data was
already in ONE call: `get_history()` — 8 ms — returns branches with
`base`/`forkedFrom`, 19-field commits, layers with `sourceCommit`, all
nine storage figures + `init`, and a `version` for a 1 ms
`{kind:"unchanged"}` refresh. **Merge evidence lives in the LAYER**
(`sourceBranch: "merge-…"` + `sourceCommit`), not the commit — a graph
built from commit parents draws no merges. **`list_commit_storage("main")`
refuses `NoSuchBranch`** (main is a Layer chain).

## IMPLEMENTED 2026-10-26/27 — plan `docs/plans/2026-10-26-layerfs-history-instrument.md`, `completed`, six increments

- **C0** `472df99` — one `get_history` read, all fields parsed, `since`
  fast path, the cut named in words. Measured: the old fan-out 4,808 ms
  → **140–165 ms warm** (cold first read 693 ms = worker/node startup);
  the installer went copy-if-absent → **copy-if-different** (the scripts
  evolve in place; the operator's override is the env seam, not the
  home's copy).
- **C1** `1153dc2` — the tree in the reference's vocabulary: bands on
  main's spine, lanes from fork nodes at actual bases, leaves, dashed
  graft stems from `sourceCommit`, idle branches as ONE chip (the
  store's 89 near-empty branches were the worst case for a fixed-column
  DAG); `--lane-0…5` in all four theme blocks; zoom 25–200% + fit, pan,
  minimap with window, hover cards, click-through, sideways/upright,
  **`F` follows the newest node**.
- **C2** `88eb6b2` — the arm sets `LAYERFS_TRACE=<home>/traces/<session>.jsonl`
  per child; a Rust tail with per-file offsets, truncation reset,
  torn-line holds, 200-cap with the drop named; `/api/workspace-history/calls`;
  Work rows `C<ordinal> · ↓ L<k>` + state + activity + child session.
- **C3** `963deac` — one route, four modes: text (400-line windows,
  256 KB cut named, binaries named), blame (2,000-line cap), revisions
  (backend-only via the script's `--revisions`), diff (both ends
  through `get_comparable` via `--comparable`, NotComparable named).
- **C4** `d3f5c0a` — the accounting in the substrate's terms: on disk =
  `file`, metadata labelled a SUBSET never added, SQLite overhead =
  `file − free − stored` (471,628 on the live store), ratios computed
  from bytes, Init record or its absence named, per-commit bars from the
  history's own commits.
- **C5** `6e3f062` — `layerfs-live.sh` step 6 reads the seam back
  THROUGH the instrument (green: **56 ms warm, 9 grafts**, feed + text
  live); the four docs rewritten; plan status `completed`.

**The pin caught a real driver defect (the general lesson).** The
256 KB text fixture reddened on the FIRST run: `spawn_with` waited
with `try_wait()` while the child wrote into a piped stdout nobody
read — **any CLI answer past the OS pipe buffer (64 KiB) blocked the
child and burned the whole 120 s `CONTROL_BUDGET` on a call that
answers in milliseconds** (a big `fs read` window or `fs list` would
hit it live). The pipes now drain while the child runs; the fixture
went 120 s → 1.3 s. Two criterion lines left undone mid-increment (the
`F` key, three cap pins) were closed in the close-out rather than
papered over — the same "named leftover" discipline as
[[a-green-gate-covers-what-was-built-not-what-was-left-out]].

**Two process facts from the same session.** (1) The other session's
commit `7d2c81e` **swept six of my staged C5 files** into its own
message (docs + script) while I was mid-gate — content all in, but the
split sits under their commit; my follow-up carried the rest and said
so (see [[shared-file-commits-can-sweep-another-sessions-work]] for
the hunk-level staging + staged-tree-compile discipline that kept the
REST mine). Their `delegate::`/`judge::`/`explain::` tests failed at
their own HEAD — proven by running one there — so the honest gate
statement was **the staged tree green minus their named WIP**: fmt 0,
clippy `-D` 0, lib 1450/0, face 252/0. (2) The relaunch next morning
(2026-10-27): every route probed **healthy** (history 82 ms, calls 8
ms, `/events` holding open is its normal SSE shape) while the owner
saw it "stuck" — the relaunch was still right (fresh PID, 200), but
the evidence said the failure was the BROWSER TAB: a page loaded
before a restart runs the old JS against the new API, so
**hard-refresh before theorizing** (cf. [[verify-the-binary-is-newer-than-the-source]]).
The relaunch itself is mechanical now: `launchctl kickstart -k` fails
with exit 78 on this LaunchAgent, so it's `launchctl bootout
gui/$(id -u)/com.fleet.gui` then `bootstrap` the plist — and the
service prints a fresh pairing code each start, so an old QR/URL is
superseded.

**The general lesson.** We built a *summary* of the substrate — four
cards restating what the Store holds — where the reference built an
*instrument*: the graph teaches the model, the Work tab shows what is
happening now, the storage panel does the accounting. The verdict
"underwhelming" was about ambition, not styling, and the honest fix was
to measure what the substrate already reports before drawing anything.
Links: [[layerfs-expert-study-2026-10-25]] (the substrate study and
the seamless/closing-the-seams/live-fork-base plans),
[[fleet-second-brain-visual-redesign-awaiting-pick]] (the same owner
word, "underwhelming"), [[verify-what-renders-not-what-commits]],
[[a-fixture-corpus-flatters-a-retriever]], [[fleet-anti-hermes-constitution]]
(no new knob).
