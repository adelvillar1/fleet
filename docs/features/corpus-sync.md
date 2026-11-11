# The shared corpus sync — the distributed mind (armada L4)

One corpus, two feeders: harnesses feed it locally (the cross-harness pull),
armada distributes it globally (this layer). The corpus rides **git** — the
transport — while fleet owns the **semantic merge** (D23).
`fleet memories --sync` is fetch → merge → push; with no remote configured it
refuses by name and fleet stays exactly as it is today (the daily script
keeps committing locally). Sync is opt-in and additive (D6).

## Why naive pull/push is wrong, and the merge rules

- **The recall-count** is a grow-only rank signal under a local-only flock.
  Two installs conflict on the same `recall-count:` line constantly; it
  merges by **sum/delta** over the merge base (`ours + theirs − base`), never
  last-writer-wins (D24) — a fact recalled on A bumps its rank for B.
- **KI-285's two-writer lost update goes distributed**: auto-generated
  replacement serials carry the install-id's first 8 hex (D25), so two
  installs never mint the same name (`<base>-r<serial>-<install8>`). A
  supersede collision resolves to **one live frontier** by the basis ladder
  (D26): higher basis wins, tie → later date, tie → greater install-id; the
  loser folds with a named retirement note, never hard-deleted.

The recall index, `*.lock`, `*.tmp`, and per-install state never sync (D29) —
only the tracked fact tree moves. Every sync settles a `corpus_sync` journal
event through L2 and is graded by read-back before it reports done (D28). An
unresolvable conflict aborts the merge and curtails the sync — the corpus
stays at local HEAD, never a half-merge pushed.

## The contract (for the layers above)

```text
corpus::sync(home, remote_override?) -> MergeReport   // fetch → merge → push
corpus::merge_recall_count(base, ours, theirs) -> u64 // sum/delta
corpus::resolve_supersede(a, b) -> candidate          // basis ladder winner
corpus::scoped_replacement(base, serial, install)     // install-scoped serial
corpus::SyncState { remote, corpus_ref, last_revision, settles }  // <home>/corpus-sync.json
MergeReport { merged, summed, resolved, refused }
RemoteInstance::new(project, root): Source            // reads the remote at git HEAD
```

The control plane's `corpus-sync` verb (L8) calls `corpus::sync`; the guard's
`corpus` dimension (L3) gates it. `RemoteInstance` plugs into the unchanged
`pull()` import path, provenance-tagged `armada:<project>:<commit>`. See
`docs/plans/ARMADA-CONTRACTS.md` §5 and `ARMADA-DESIGN-DECISIONS.md` D23–D29.

## Surface

- **`fleet memories --sync [--remote <url>]`** runs the driver and prints the
  report (merged/summed/resolved/refused counts, named degradations).
- **`fleet memories --pull --source armada`** reaches the live remote feeder.

## Verification

`cargo test -p fleet --lib corpus::` pins: the sum/delta rule (base 5, ours 8,
theirs 7 → 10), the basis ladder, the supersede winner (basis → date →
install), distinct install-scoped serials, the unconfigured refusal, the sync
state round-trip, and sync-with-no-remote writes nothing. The memory_pull
import pin updates to the install-scoped replacement. A live two-clone
exercise confirms the no-remote refusal, the no-op sync advancing the cursor,
and the `corpus_sync` journal event carrying the install-id. The full gate
(2227 lib + 172 integration + 16 install) and the face harness confirm
single-owner behavior is unchanged.