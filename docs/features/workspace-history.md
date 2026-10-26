# Workspace history (C4–C6 of the seamless plan)

The LayerFS substrate, read as fleet's own knowledge surface: one
content-addressed Store, main's trunk, every branch a delegated child
worked, the merge edges back, and the figures behind them. This is the
HOW for `docs/architecture/surfaces.md`'s *Workspace history — knowledge*
entry.

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
                        layerfs-history.mjs (the backend's trunk and
                        figures), layerfs-probe.sh (the diagnostic)
```

## How the reads are composed

Nothing in fleet talks to LayerFS through a second implementation. The
agent CLI answers the tree (`project list_branches`, `branch
list_commits`, `fs list`), the sealed bundle's BACKEND package answers
main's Layers and the Store's figures, and fleet's own delegation rows
answer the join from a branch back to the child that worked it. Three
facts the live run taught are structural now:

**main's commits are the backend's.** `list_commits main` answers
`NoSuchBranch` on the real wire, so the trunk the face is built around
is the one the CLI cannot see. `scripts/layerfs-history.mjs` imports
the bundle's packages by absolute path — nothing resolves from fleet's
own tree — and its answer carries main's commits beside the Layers and
the figures. The stub test's fixture is the captured bytes of a real
Store (18,777,528 logical, 11,229,684 stored, dedupe 1.25553), not an
invention.

**The store path is the project's own.** Every read composes
`LAYERFS_STORE_PATH` from the folder's registration before it spawns
anything, the same composition the arm's spawns use. The seven manual
exports the recipe used to need are gone: `fleet install` records the
machine's facts, and the reads and the runs compose from them.

**One branch's failure is a note, never an empty view.** A dead backend
script, a branch whose commits cannot be listed, a worker that does not
answer — each is a named reason on the document, and everything that
did answer still serves. An empty list that reads as "nothing here" is
the one shape the surface never draws.

## The section

Four panes, all from one document:

- **History** — the DAG. main's trunk is the spine; each child branch
  is a lane forking from it, with the merge edge drawn back when the
  parent settled it. A commit's message, its `via`, its time and its
  parent count ride its tooltip; a second parent is the merge the edge
  is drawn for.
- **Work** — one lane per child branch: the session that worked it, the
  outcome the arm recorded, the state the tree carries (merged, open,
  idle — derived from main's own commits, never recorded twice), and
  the commits it stands on.
- **Files** — a branch's tree at a path, one page at a time through the
  CLI's own listing. Directories navigate; the reads are text-only, the
  way a delegated child's are.
- **Storage** — the Store's figures as bars against the logical size,
  and the layers the content-addressed tree is made of.

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
named in the result; and the Store's view of untouched files ages until
a delta re-import at spawn — the plan's own named residue.

## What is not here

Writes. Every write verb is the delegated child's, behind the arm: the
section reads the tree and browses a branch's files, and nothing on it
commits, merges or publishes. The substrate's own provisioning is
`fleet install`'s standard phase; a machine without Docker, Node 20+ or
a sealed bundle is skipped by name with the provision that lifts it.
