# Workspace history (the history instrument)

The LayerFS substrate, read as fleet's own knowledge surface: one
content-addressed Store drawn as a TREE (a band per Layer, a lane per
branch, a leaf per commit), the work those branches carried, the
substrate's live call feed, a file's own instruments, and the Store's
accounting. This is the HOW for `docs/architecture/surfaces.md`'s
*Workspace history — knowledge* entry. It is the C4–C6 record of the
seamless plan plus `docs/plans/2026-10-26-layerfs-history-instrument.md`
(C0–C5), which replaced the per-branch fan-out with ONE read and the
summary cards with instruments.

```
src/layerfs_tools.rs    the driver: the machine facts and the folder
                        registrations, the composed environment, the
                        CLI's verbs (project/branch/fs/exec/run),
                        the Workspace cadence, the config writers
src/layerfs_view.rs     the view model: branches and commits, the
                        backend's trunk, the child join, the section's
                        document, the Files pane's page
src/subagents.rs        the arm: resolve the project from the folder,
                        the workspace handle, the branch fold, the
                        settlement's merge-back
src/install.rs          `fleet install`'s substrate phase: the decision,
                        the worker's start, the scripts' install
src/doctor.rs           the substrate line's seven facts
src/gui/bridge.rs       the section's two reads and the delegation
                        panel's branch join
src/gui/server.rs       the two routes
src/gui/assets/         the section: the rail entry, the four panes,
                        the delegation row's click-through
scripts/                layerfs-worker.mjs (start the worker),
                        layerfs-register.mjs (register a folder),
                        layerfs-history.mjs (the backend's one read,
                        plus its --revisions and --comparable modes),
                        layerfs-probe.sh and layerfs-live.sh (the
                        diagnostics)
```

## How the reads are composed

Nothing in fleet talks to LayerFS through a second implementation. The
whole tree comes from ONE backend call; the agent CLI answers only
where the backend cannot; and fleet's own delegation rows answer the
join from a branch back to the child that worked it.

**One read, not a fan-out.** `scripts/layerfs-history.mjs` runs
`backend.store.project.get_history` once and passes the answer through:
the branches (with `base` and `forkedFrom`), their newest commits
(nineteen fields — the graph and the accounting draw them all), main's
Layers (with `sourceCommit` — the merge evidence), the Store's nine
figures plus the Init record, and a `version`. Measured on the fleet
store: the old per-branch fan-out answered in 4.8 s (one CLI
subprocess per branch, silently capped at the CLI's page of 50); the
one read answers in **56–165 ms warm**, and `get_history(since)` — the
version the last read named — answers `{kind: "unchanged"}` in 1 ms,
so a refresh that found nothing moved re-reads nothing. The script
imports the bundle's packages by absolute path — nothing resolves from
fleet's own tree — and strips `LAYERFS_TRACE` first, so the face's
reads never land in a child's feed.

**The cut is named.** `moreBranches`, `moreLayers` and the per-branch
`more` cursor are the substrate's own "there is more" — the document
carries them as words (`truncated`) the face prints where the tree
stands, the instrument's line says "the newest N" when the page was
cut, and the Files tabs carry the same note rather than a shorter list
passing as the whole. A silent drop (the old 50-of-89) is the shape the
surface never draws.

**The CLI survives where the backend cannot.** `fs list` (the Files
pane's listing), `fs read`/`fs stat`/`fs blame` (the file's
instruments), `branch diff_commits`/`fs diff_files` (the diff), and the
live feed's tail over the machine's own trace files.

**The store path is the project's own.** Every read composes
`LAYERFS_STORE_PATH` from the folder's registration before it spawns
anything, the same composition the arm's spawns use. The seven manual
exports the recipe used to need are gone: `fleet install` records the
machine's facts, and the reads and the runs compose from them.

**A dead read is named, never an empty view.** A failed script, a
worker that does not answer — each is a named reason on the document,
and the two failures a Docker reset produces are named in the panel's
language *before* the script runs: a recorded worker with nothing
answering on its port reads "the LayerFS worker is not running — start
it with `fleet install`", and a Store volume the reset took reads "the
project's LayerFS Store volume is missing — re-register the folder; its
history restarts empty". The script's own words ride after the named
cause, bounded, as evidence. An empty list that reads as "nothing
here" is the one shape the surface never draws.

## The section

The surface is the work rail's idiom, shared with settings, skills, and
memory (2026-10-30): a rail of **projects** — the machine's registered
folders whose newest commit, or whose substrate-recorded creation, falls
inside the six-week activity window — one project's components at a
time, and the right pane for the two browsing cards. A project resolves
by its **folder path** (the control tower's and the second brain's
identity), the rail remembers the choice, and `#/workspace-history/<encoded
path>` addresses one. The rail document reads every registration once,
caches for thirty seconds, and a read that dies names the substrate's
own refusal on one compact card after the section — a broken read is not
inactivity, and an unexplained absence is the one shape this surface
never draws.

The board carries the **visual components** — the tree, the work, and
the calls feed (machine-wide, as the traces are) — and the pane carries
**the store** and **the files** as one object card, the tree's node
click landing there at the ref it named. The cards, all from one read —
the vocabulary the substrate itself carries (the reference the
`dsh-layerfs` plugin draws with):

- **the tree** — main's spine with a **band** for every Layer (coloured
  by the branch that published it), one **lane** per branch with work —
  a head *or* a merge the Layers record — sprouting from a **fork node
  at the Commit it forked from** (`forkedFrom` when the page carries
  it, the Layer's `base` as the fallback), a **leaf** per commit the
  page carries, and a **dashed stem** where a publish grafted a leaf
  back to the band it made (`layer.sourceCommit` — on the live store
  every merge lands there, while the commits' own merge fields are
  null). A merged branch whose head is gone has no leaf to stem from —
  `get_history` skips a head-null branch's commits — so its **ring
  stands alone on the band**, named by the Layer, and its lane and Work
  row stay: finished work never reads as "forked, no work". Lane bodies
  follow each commit's own `parent` (a parent outside the page starts a
  new segment), so the drawing never implies a link the read did not
  carry. Lanes pack nearest the spine; time is event slots, not the
  wall clock. A branch nobody worked is ONE idle chip with a count —
  never one column each (the store that motivated this plan had 89
  branches and the old graph drew ~4 lanes before clipping the rest
  off-canvas: the plot's `viewBox` is the content's size now and the
  card scrolls). Commits whose branch the page cut are counted under
  the tree, so the strip's commit figure and the leaves agree or the
  note says why they cannot. Zoom 25–200% + fit, drag or scroll to pan,
  a minimap with the visible window, a hover card per node, sideways by
  default with an upright toggle, and a click reads Files at that ref.
  The lane colours are the `--lane-0…5` palette, defined in all four
  themes; the semantic tones stay semantic.
- **the work** — every branch with work as `C<ordinal> · ↓ L<k>` (its
  head commit's place in the run, and the layer it forked from — with
  `(page)` when the layer list itself was cut), its state (merged /
  open / idle — derived from the layers that name it, never recorded
  twice), its last activity, and the child session and outcome joined
  from the day's delegation rows. A merged branch whose commit is
  outside the page reads `merged · the layer carries its commit` — the
  work happened, and the Layer is where it stays legible.
- **the calls** — the live feed: the substrate's own trace
  (`LAYERFS_TRACE=<home>/traces/<session>.jsonl`, set per child by the
  arm) tailed with per-file offsets, held trailing lines, and the 200
  newest answered with the drop named. Actor, call, ms, and what it
  made (the commit, or `refused`). Traces off is named — never read as
  "no activity".
- **the files** — a ref's tree (branch name, commit id or layer id) at
  a path, and once a file is open its own instruments: **text** (a
  400-line window with its next-window cursor, cut at 256 KB with the
  cut named; a binary is "binary, N bytes"), **blame** (every line with
  the Commit that last changed it, 2,000-line cap), **revisions** (the
  backend's own — the CLI has no such verb), and **diff** (both ends
  resolved through the substrate's `get_comparable`, so a Layer with
  no Commit of its own is the named NotComparable refusal; the changed
  set, or one changed file's hunks colored by op).
- **the store** — the substrate's own accounting: **on disk** (`file`),
  main's latest, every snapshot, unique, compressed, **of which
  metadata** (a subset, never added), and the **SQLite overhead**
  (`file − free − stored`) named separately; the ratios computed from
  the bytes rather than trusted as strings; the Init record with its
  counts (or its absence named); and per-commit costs — average,
  largest, one bar per commit colored by how it was made — from the
  commits the read carried, because `list_commit_storage` refuses main
  by name.

The Delegation panel's rows carry each child's branch and click
straight through, so the tree a child worked is one step from the run
that worked it.

## Registering a folder

The arm resolves the project as override → env knob → the folder's
registration → register-once, so a delegation launched in a registered
folder needs no fact of its own. For a git repository the import
carries what the repo DECLARES (`git ls-files` — the `.gitignore` is
the owner's already-made decision), staged into a scratch dir and
imported from there, so the fleet repo's 35 GB `target/` never counts;
a non-git folder keeps the whole-tree walk, and the bound (2 GiB or
100,000 entries) is the guard. The Workspace is the declared source
tree: `.git` is not carried (a frozen snapshot would be worse than
absent — git history stays the host checkout's job), untracked files
are not carried, and an ignored `.env` can never enter the Store.

## The merge comes home

When a child's merge and publish land, the settle brings the child's
work to the launch folder through the parent's own write path — one
grouped approval in ask mode, the yes modes proceed. The spawn-time
content hashes decide: a path the host moved since the fork is a named
conflict (the file untouched, both versions kept — the branch's
readable in the Files pane), and everything else applies. A refused
merge applies nothing. A non-git folder's delta stays in the Store,
named in the result. And the fork base is the host as it stands: each
spawn syncs the host's newer tracked paths onto the fresh fork (a
git tree diff against the recorded import), so a child never works from
a frozen snapshot of the folder.

## The standing proof

`scripts/layerfs-live.sh` runs the whole seam end to end — a fresh
folder, an isolated delegation, the merge applied home, the spawn sync
— and then reads it back THROUGH the instrument: a warm read under the
250 ms bound, the merged state, the graft data, the full id range, the
nine figures, the feed's route, and the delegation's stamp through the
file route. Run it before a release.

## What is not here

Writes. Every write verb is the delegated child's, behind the arm: the
section reads the tree and browses a branch's files, and nothing on it
commits, merges or publishes. The substrate's own provisioning is
`fleet install`'s standard phase; a machine without Docker, Node 20+ or
a sealed bundle is skipped by name with the provision that lifts it.
