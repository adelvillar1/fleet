---
status: draft
created: 2026-10-26
updated: 2026-10-26
slug: layerfs-history-instrument
---

# The history instrument: one read, one tree, three instruments

> **Status: DRAFT** — awaiting the owner's word to start at C0.

The owner's read of the shipped workspace-history face: **"the graph
and visual pieces of layerfs as they were implemented on our end are
really underwhelming."** This plan agrees, and names why. We shipped a
*summary* of the substrate — four cards that restate what the Store
holds — while the reference implementation (the `dsh-layerfs` plugin,
the only visual LayerFS has) ships an *instrument*: a real DAG with a
vocabulary, lanes that show live work, and an accounting panel that
does the maths. On top of that, our own version is measurably broken on
our own data: it delivers 50 of 89 branches without saying so, draws
about 46 of those 50 off-canvas, and takes 4.8 seconds to do it.

## Context

Measured 2026-10-26 against the live substrate and the reference's own
source. Everything below was run, not read off a docstring.

**The reference is not upstream.** `github.com/Ephemeral-AI-Lab/layerfs@ad78914`
(the sealed bundle we drive) ships verbs and JSON — `project` / `branch`
/ `fs` / `workspace` / `sandbox` — plus `@layerfs/backend` for
observation. It has no graph, no UI, no ASCII tree. The visual
experience is the separate repo `Ephemeral-AI-Lab/dsh-layerfs`: a DSH
plugin whose **Work** and **History** tabs show every agent's branch
live. Our four panes are named after those tabs; we copied the nouns,
not the instrument.

**What the reference actually draws.** `packages/view/src/tree.js`
(283 lines) + `treezoom.js` (133) + `history.js` (346) + `files.js` (289),
fed by `packages/model/src/derive.js`, `attribution.js`, `events.js`.
Its vocabulary, from its own legend:

| mark | meaning |
| --- | --- |
| band on main | a Layer |
| leaf | a Commit |
| bough | a Branch, sprouting from a fork node at its **actual** base |
| ring | a merge, grafting back |
| dashed stem | a publish, into a band |
| bare twig | a Branch nobody is on |

Time is event slots, not wall clock. Lanes are packed nearest-parent
and keep their lane as history grows; each branch has its own colour;
agents get a head at the lane end. It renders upright or sideways, has
zoom 25–64% + fit + pan, a **minimap overview** with a position window,
hover cards, and `F` to follow. The Work tab is branch chips with
counts, a sandboxes board (`944 calls · 133 cmds`), a branches board
where each row reads `C913 · ↓ L8`, a **store.stats** panel
(`8.90 GB on disk for 4.93 TB of history`, dedupe/compression, the Init
record, per-commit average/largest with a histogram) and a **live call
feed** (`$ branch publish report-final2 · 2 ms`). Screenshots:
`demo/quarry/assets/history-tree.png`, `history-sideways.png`,
`work-store-stats.png`, `docs/tutorial/14-history-blame.jpg`.

**What ours does now.** `historyDag` is 76 lines (`app.js:9523`): a
spine from the layers, one fixed 250px column per branch, every fork
edge starting at one fixed Y (`app.js:9571`), grey dots. No zoom, no
minimap, no author, no fork/merge/publish vocabulary. `historyWork` is
a five-column table; `historyFiles` lists and then refuses to read
(`app.js:9726` — "reads are text-only" and no read at all);
`historyStorage` draws three bars (`app.js:9767`).

**The three defects that make it underwhelming rather than merely
plain:**

1. **Silent truncation.** `list_branches` is called with no page size
   (`layerfs_tools.rs:1686`), so the CLI's default 50 applies. The live
   store holds 89+ branches (`hasMore: true`). The endpoint delivers
   exactly 50 with `refused: []` — nothing says 39+ were dropped.
2. **Drawn off-canvas.** Content width is `130 + 50 × 250 ≈ 12.4k px`;
   `svgPlot` builds a viewBox of the *card* width (`app.js:4060`), so
   about 46 lanes sit past the right edge. The plot is 104 px tall.
3. **Forks placed wrong.** `forked_from` arrives in the payload and is
   parsed into `BranchView` — and `grep` finds zero uses in `app.js`.
   Every fork curve starts at main's newest row.

**The read is the expensive part.** `/api/workspace-history` answers in
**4.8 s** for 7 KB, because it fans out: one `list_branches`, then one
`branch list_commits` **subprocess per branch** (48–74 ms each, measured
over ten), then one node script. The script that would replace the
fan-out — both packages imported, `get_history` + `get_storage` — runs
end-to-end in **40–90 ms**.

**And the data we need is already in one answer we don't read.**
`backend.store.project.get_history()` — **8 ms**, one call — returns
`{kind, version, store, project, branches, commits, moreBranches,
layers, moreLayers, workspaces}` where:

- `store.stats` carries **all nine figures plus `init`** — so the
  separate `get_storage` call is redundant;
- `branches[]` = `{name, base, forkedFrom, head}` — fork placement,
  exactly what we drop;
- `layers[]` = `{id, parent, root, logical, sourceBranch, sourceCommit,
  overwroteCount}`;
- `commits[]` = 19 fields including `branch`, `parent`, `secondParent`,
  `base`, `mergedFrom`, `displaced`, `lww`, `logical`, `newCanonical`,
  `newStored`, `newMetadata`. Our `parse_commit` (`layerfs_view.rs:289`)
  keeps **five**;
- `version` + `get_history(since)` = **1 ms** `{kind: "unchanged"}` —
  incremental refresh we never use.

**Merge evidence lives in the layer, not the commit.** Measured on the
`fresh-src` store (8 layers, 9 commits, 27 branches): every published
layer carries `sourceBranch: "merge-fleet-child-…"` and
`sourceCommit: <the child's commit>`, while the commits' `mergedFrom`
and `secondParent` are **null**. The graft edge the reference draws is
therefore `layer → sourceCommit`; a graph built from commit parents
alone would draw no merges at all.

**`list_commit_storage("main")` refuses `NoSuchBranch`** — main is a
Layer chain. Main's per-commit storage comes from the history's own
commits, not that call.

**The other reads, timed:** `get_comparable(layer)` 3 ms (`{commit,
root, exact}` — the Layer→diffable-Commit bridge the reference needs
for click-through); `list_overwritten` 7 ms; `list_revisions` 47 ms;
`fs.get_storage` 7 ms; `fs blame` 24 ms returning every line with its
commit and message; `list_sandboxes` 6 ms and empty — fleet's children
work host-side, so no Workspace is ever mounted and
`get_history.workspaces` stays `[]`.

**The live feed's source exists and we never turn it on.** The trace:
`LAYERFS_TRACE=<file>` per process, one JSON line per call (`method`,
`args`, `start`, `ms`, `ok`, `kind`, `commit`, `sizes`), read when the
client is made; `@layerfs/runtime`'s `events.subscribe({project, since})`
tails `<home>/traces/*.jsonl` every 400 ms. Our arm sets no trace, so
`<home>/traces` is empty and there is nothing live to show.

**What the face already gives us to build with:** `boardCard`,
`svgPlot`, `label`, `dot`, `chip`, `board-table`, `figure-bars`; and
tokens only (`--dsw-alias-*`) across four theme blocks — light,
`console`, `graphite`, `contrast`.

The owner's standing rules bind. **Anti-Hermes:** no new knob — the
face reads what the substrate reports, and a machine without the
substrate still names that. **Lanes as providers:** nothing here needs
a lane; these are deterministic reads and a renderer. **Integral, not
bolt-on:** the read replaces the fan-out it supersedes, the graph
replaces the DAG function it supersedes, and the panes keep their
existing cards.

## Approach

**One read, one tree, three instruments.**

The read becomes a single backend call. `get_history(0)` returns the
branches, their commits, the layers, the store figures and a `version`
in 8 ms; `history()` parses all of it and drops nothing the graph
draws. `since` makes a second read free when nothing moved. The CLI
survives only where the backend cannot answer: `fs read`, `fs blame`,
`fs list_revisions`, `fs diff_files`/`branch diff_commits` for a
node's diff, and the sandbox listing.

The tree is drawn from nodes and edges the answer already carries —
never from a re-derived fan-out:

- **layer** at its slot on main's spine, joined to its parent layer;
- **branch lane** from `forkedFrom` (a Layer id) through each commit's
  `parent` chain, ending at `head` (or a bare twig when `head` is null);
- **commit → fork** when its `parent` belongs to another branch (the
  reference's synthetic `fork:` node);
- **graft** from each layer to its `sourceCommit`, since that is where
  the merge evidence lives;
- **idle** branches (no `head`) collapse into one chip — the store that
  motivated this plan has 88 of them and drawing 88 columns is what
  breaks the face today.

The three panes become instruments: Work carries the lanes plus the
live call feed (which requires the arm to set `LAYERFS_TRACE`), Files
reads text, blame and revisions, Storage does the accounting with the
substrate's own terms.

The ordering rule that keeps it honest: **the substrate is the only
writer** — this plan adds no Store write, no branch bookkeeping, no
publish change. The view reads; the panes render; a failed read is a
named absence, never an empty lie.

## Acceptance criteria

- [ ] **C0 — the one read; the cut named.**
      `history()` makes ONE backend call (`get_history(0)`) and ZERO
      CLI calls; the store figures come from that answer's
      `store.stats` (the separate `get_storage` call goes).
      `GET /api/workspace-history` on the fleet project answers
      **< 250 ms** (measured 4,808 ms today) and carries `version`.
      A read whose last `version` is still current answers
      `{kind: "unchanged"}` (1 ms) and leaves the view untouched.
      `moreBranches` / `moreLayers` set `truncated` on the view and the
      face prints the cut in words — "65 of 89+ branches shown, the
      newest first" — where today `refused` is `[]` and nothing is
      said. Every field the graph draws is parsed:
      `branches[].{name,base,forkedFrom,head}`,
      `layers[].{id,parent,root,logical,sourceBranch,sourceCommit,overwroteCount}`,
      `commits[].{id,branch,parent,secondParent,base,mergedFrom,displaced,lww,logical,newCanonical,newStored,newMetadata,via,message,time}`.
      **Pinned:** a fixture answer with `moreBranches: true` sets
      `truncated` and names the cut; an unchanged `since` answer changes
      nothing; the live fleet answer parses 65 branches and 2 layers
      with no field left null-by-omission.
- [ ] **C1 — the graph is a tree.**
      Nodes: layer (band), commit (leaf), fork (synthetic, at
      `forkedFrom`/`base`), merge graft (from `layer.sourceCommit`),
      bare twig (a branch with no `head`). Edges: the layer parent
      chain; each commit's `parent` within its branch; commit → fork
      when the parent is on another branch; layer → `sourceCommit` for
      every published layer. Lanes packed nearest-parent, each branch
      one tone from a new categorical palette `--lane-0…5` defined in
      **all four theme blocks**; semantic tones (accent/ok/down/warn)
      stay semantic and are never used as lane colours. Idle branches
      collapse into one "N idle" chip that opens their list — never one
      column each. Geometry never clips: the plot's `viewBox` is the
      **content's** width and the card scrolls; zoom 25–200% + fit,
      pan, and a minimap strip with the visible window. Hover card per
      node; click a node → Files at that ref (a Layer resolves through
      `get_comparable`, refused `NotComparable` by name); `f` follows
      the newest node; orientation toggle, sideways by default.
      **Pinned:** the live fleet answer (2 layers, 1 commit, 65
      branches — main, one worked, 63 idle) draws 2 bands, 1 leaf and
      one idle chip of 63; the
      `fresh-src` answer (8 layers, 9 commits, 27 branches) draws 8
      bands, 9 leaves and 7 grafts; a fixture with 89 branches puts no
      node outside the plot's bounds.
- [ ] **C2 — the work lanes and the live calls.**
      The Work pane shows every branch as `name · C<n> · ↓ L<k>`
      (commit count; the layer it forked from, by logical index) with
      its state (merged / open / idle), its last activity from its
      newest commit's `time`, and — where fleet's own delegation rows
      join — its child session and outcome. Sorted by activity; idle
      branches live in C1's chip list.
      The arm sets `LAYERFS_TRACE=<home>/traces/<session>.jsonl` for
      every child it spawns, so traces exist at all. A Rust tail over
      `<home>/traces/*.jsonl` — per-file offset, truncation detected by
      `size < offset` and reset, a partial trailing line held until its
      newline, `project` and `since` filters — serves
      `GET /api/workspace-history/calls?since=` with
      `{actor, method, args, start, ms, ok, kind, commit, sizes}`,
      newest last, capped at 200 lines with `more: true` when older
      lines were dropped. The face shows the feed beside the lanes:
      actor, method, ms, ok/refused, and the commit it made. No traces
      on this machine → the pane says traces are off, never "no
      activity".
      **Pinned:** a fake trace folder with two files, one truncated
      mid-line, returns the complete lines once in order and never a
      partial line; a second call with `since` returns only newer
      lines; an empty folder answers the named absence; a refused call
      (`ok: false`) renders as refused, not as work.
- [ ] **C3 — the files instrument.**
      One route, four modes: text (`fs read` with line windows), blame
      (`fs blame` — line → commit, 24 ms), revisions
      (`fs list_revisions` — 47 ms), and a node's diff
      (`branch diff_commits` for a commit's changed set,
      `fs diff_files` for one changed file). Clicking a commit or layer
      on the graph opens Files at that ref and the title names it.
      Caps, named when they bite: text 400 lines / 256 KB, blame 2,000
      lines, revisions 50 per page; a binary is "binary, N bytes", not
      a mojibake render.
      **Pinned:** a fixture ref/path answers its text, blame and
      revisions; a path not in the tree is refused by name; each cap
      reports that it cut.
- [ ] **C4 — the storage accounting.**
      The pane prints the substrate's own terms, and the arithmetic is
      the substrate doc's, not an invention:
      **on disk = `file`**; **main's latest, raw = the newest layer's
      `logical`**; **every snapshot, raw = `logical`**; **unique =
      `canonical`**; **compressed = `stored`**; **of which metadata =
      `metadata`** (a subset of `stored`, labelled as such — never
      added to it); **SQLite overhead = `file - free - stored`**, named
      separately. `dedupe = logical / canonical` and
      `compression = canonical / stored`, both rendered from the
      numbers rather than trusted as strings. The Init record
      (`entries`, `files`, `bytes`, `millis`) is shown, and a store
      with `init: null` says so. Per-commit costs (average and largest
      `newStored`) come from the history's own commits with a
      oldest→newest histogram and a `via` legend; `list_commit_storage`
      is NOT called for `main` (it refuses `NoSuchBranch`) and the pane
      says why when per-commit figures for main are absent.
      **Pinned:** the live figures render exactly — logical 46,549,294;
      canonical 15,562,812; stored 7,388,596; metadata 500,678;
      file 7,860,224; free 0; dedupe 2.99×; compression 2.11× — and the
      row arithmetic equals the substrate's (`file - free - stored` =
      471,628); a store with `init: null` names the absence.
- [ ] **C5 — close-out.**
      `scripts/layerfs-live.sh` grows the instrument check: the
      endpoint answers under the bound on the `fresh-src` store (8
      layers, 27 branches), the graph's node count equals layers +
      commits, and the truncation line names the cut when
      `moreBranches` is true. Docs match the shipped behavior in the
      same increments: `docs/features/workspace-history.md`,
      `docs/architecture/surfaces.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
      `README.md`. The gate is green by exit code (fmt, clippy
      `-D warnings`, build, test), the staged tree compiles in a
      detached worktree before the commit, and the status flips in the
      commit that lands the work.

## Files to be touched

`src/layerfs_view.rs` (the view model rebuilt over `get_history`:
nodes and edges, truncation, the nine storage figures, per-commit
costs, the file/blame/revisions reads) · `scripts/layerfs-history.mjs`
(one call, the whole answer passed through, `version` included; the
separate `get_storage` call removed) · `src/layerfs_tools.rs`
(`Gate::get_history` — the CLI-free read; `fs read` / `blame` /
`list_revisions` / `diff_files` wrappers; the trace file in
`apply_env`) · `src/subagents.rs` (the per-child trace file) ·
`src/gui/bridge.rs`, `src/gui/server.rs` (the calls tail and the file
route) · `src/gui/assets/app.js`, `app.css` (the graph, the lanes, the
files instrument, the storage accounting, the `--lane-*` palette in
four theme blocks) · `src/gui/assets/index.html` (the pane's
controls) · `tests/face/harness.mjs` and the integration tests the pins
name · `docs/features/workspace-history.md`,
`docs/architecture/surfaces.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
`README.md` · this plan.

## Out of scope

- **History replay / playback** (the reference's play button, stepping
  commits on a timer). The orientation toggle and zoom are in; a
  scripted replay is not.
- **A sandboxes board built on `daemon.workspace.*`.** Fleet's children
  work host-side, so no Workspace is ever mounted and
  `get_history.workspaces` is `[]`; the Work pane shows sandboxes only
  when `project.list_sandboxes` reports running ones.
- **Avatars and skins** (the reference's hero images). The lane head
  carries the agent's name as text; the face has no such asset
  vocabulary and inventing one is a different plan.
- **The substrate's own bookkeeping** — branch `head: None` after
  merges land on layers, `NoCommit` publish races, any Store write.
  This plan is a reader.
- **The trace's daemon phase timing** (`daemon[].timing`) — the feed
  carries `ms` and `sizes`; the nested phase breakdown stays in the
  substrate's own observe skill.

## Verification

The gate whole and local, read by exit code — `cargo fmt --all
--check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, `cargo test` — with the staged tree compiled
in a detached worktree before every commit (the co-edited-tree
discipline). C0's two numbers are the plan's own claim and are
measured before and after (4,808 ms → the bound) and land in this file.
C1's node counts are checked against both live stores (fleet: 2
layers + 1 commit + 64 idle; `fresh-src`: 8 + 9 + 7 grafts). C2's tail
semantics are pinned by fixtures, not by a live race. C4's arithmetic
is pinned against the live figures above. `scripts/layerfs-live.sh`
runs green with the grown check. Face cases cover the graph's shapes,
the truncation line, the lanes, the files modes and the storage rows.

## Linked artifacts

Predecessors: `docs/plans/2026-10-26-layerfs-closing-the-seams.md`,
`docs/plans/2026-10-26-layerfs-live-fork-base.md`,
`docs/plans/2026-10-25-layerfs-seamless.md` (whose C4 built the face
this plan replaces). Docs: `docs/features/workspace-history.md`,
`docs/architecture/surfaces.md`. Ledger: KI-321 (closed; the publish
retry). The reference implementation:
`Ephemeral-AI-Lab/dsh-layerfs`, `packages/view/src/{tree,treezoom,history,files}.js`,
`packages/model/src/derive.js`, screenshots
`demo/quarry/assets/history-tree.png`, `history-sideways.png`,
`work-store-stats.png`, `docs/tutorial/14-history-blame.jpg`. The
substrate's own observation contract:
`layerfs@ad78914`, `.agents/skills/layerfs-sdk-observe/references/{backend,storage-metrics,tracing}.md`.
