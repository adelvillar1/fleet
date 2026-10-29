# Workspace history — the worker revived, the absence named, the visuals made true

Owner ask (2026-10-29): "review workspace history — it's giving this error … I also need to make sure that the visualizations stay true to the LayerFS project."

## Diagnosis

The refusal was environmental, not a panel bug: a Docker Desktop reset had taken the `layerfs-worker` container, its runtime image, and every `layerfs-*` Store volume, leaving the endpoint file pointing at a dead port. The SDK's uncaught `LayerFSError` was captured verbatim by `backend_answer` and painted as "the script failed: …". A second, independent audit compared the panel's drawing against the pinned substrate (`ad78914`) and found four material drifts from its own model, plus a handful of count/wording lies.

## Rulings

1. **Recover the substrate; the history that preceded the reset is gone by fact.** The runtime image is official node pulled by digest (the bundle's manifest names it); the worker returns with `fleet install`; the Store restarts at its Init layer. Recorded in KI-364.
2. **Name the failure class before the script runs.** A recorded worker with nothing answering reads "the LayerFS worker is not running — start it with `fleet install`"; a Store volume the reset took reads "the Store volume is missing — re-register the folder; its history restarts empty". The script's own words ride after the named cause as bounded evidence, never as the whole story.
3. **A merge is work, not idleness.** `get_history` skips a head-null merged branch's commits, so filtering on `head` read finished work as "forked, no work" and dropped the one visual that proves a merge. A branch whose derived state is `merged` keeps its lane, its Work row, and its proof: the ring stands on the band, named by the Layer's `sourceCommit`, with the dashed stem when that commit is on the page.
4. **The fork sits where the substrate says it forked.** `forkedFrom` (a Commit) is the fork point when the page carries it; the Layer's `base` is the fallback — `base` alone collapsed every fork inside one layer onto a single spine slot.
5. **Lanes follow ancestry, not arrival.** Each lane's body walks the commits' own `parent` links; a parent outside the page starts a new segment, so the drawing never implies a link the read did not carry. Merge evidence stays where the substrate puts it — the Layer (`sourceBranch`/`sourceCommit`) — which the graft already draws.
6. **Counts describe the page honestly.** The instrument says "the newest N" when the substrate's cap cut it, the Files tabs carry the same note, commits on cut branches are named under the tree so the strip's count and the leaves agree or the note says why not, `lww` parses the wire's object shape (`{path, kept, deleted, rule}` — it was parsed as strings and always empty), the graft null-guards `source_branch`, dedupe is computed from the byte totals instead of reading the substrate's nullable string, and `↓ L<k>` says `(page)` when the layer list was cut.

## Increments — all landed

- **Step 0 — recovery**: `docker pull node@sha256:3d27e5c1…`, `fleet install` (worker answering · image present), six volumes recreated, the fleet folder re-registered through the DECLARED path (git ls-files staged; 498 files / 20.3 MB — the raw script on the repo would have imported the whole 24.9 GB tree, and a first run did exactly that before the declared route was used), config's volume/store re-recorded. The panel answers: registered, no refusal, Init 563 entries / 498 files, the nine figures.
- **Step 1 — named refusal** in `layerfs_view.rs`: `worker_gap` (a recorded endpoint with nothing answering) checked before the spawn, `named_refusal` translating the two Docker-reset failures. Pinned by `a_recorded_worker_that_answers_nothing_is_named_before_the_script`, `a_dead_worker_is_named_before_the_history_script_runs`, and `a_substrate_failure_is_named_in_the_panels_language`.
- **Step 2 — the visuals** (`app.js`): merged head-null branches as worked lanes with ring-only grafts; forks at `forkedFrom`; parent-chain lanes; honest counts and words; the `lww` object shape in the hover (Rust parse + client mapping, pinned by `a_last_writer_wins_entry_reads_as_its_object`).
- **Step 3 — pins**: the C5 harness case gains a truth fixture (merged head-null branch, forkedFrom ≠ base, out-of-page merge, cut-branch commit, null dedupe) and nine checks: 293 checks across 30 cases.
- **Step 4 — docs**: `docs/features/workspace-history.md` states the merge lane, the fork-point rule, the parent-chain lanes, the subtree count rule, the cut wording, and the named refusals; KI-364 records the incident and the recovery.

## Named remainder (not this cut)

- Drawing `units`, `workspaces`, commit/layer `root`, `new_metadata`, `displaced`, or the `branch diff_branches` content — parsed-but-undrawn fields the docs do not promise.
- `main` serializes with a derived state (`idle`) though it is the trunk; the face never renders main's state, and the document's word for it is a separate nit.
- Merge diffs are still first-parent diffs; labelling them so is a one-line follow-up.
- The other five registered projects' Stores restarted empty after the reset and re-initialize when next used.
