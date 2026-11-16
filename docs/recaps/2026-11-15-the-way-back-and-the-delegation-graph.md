# Session Recap — 2026-11-15: the way back, and the delegation graph

**Summary.** The session opened on the owner's worry — "I ask fleet to do
something and then can't find it; things tend to disappear" — and the
diagnosis was a **window**, not a render: the archive read took the newest
200 session logs of every kind, 175 of them scheduled runs, so **164 of the
owner's 183 conversations were outside the read entirely**. Three fixes
followed: a per-class window, a spine that carries only user-initiated
conversations (with delegation riding under the parent goal), and a
delegation graph that is named, listed and openable. All gate-green,
deployed locally, verified live. No release cut.

## The finding (measured, not guessed)

Fleet's own store held 849 logs; 617–739 were `kind:"scheduled"`. In the
newest 200 by mtime: 175 scheduled, 5 subagent, 1 teaching, ~19 the
owner's. `bridge::sessions_read` truncated the newest `SEARCH_BOUND` (200)
**before** classifying, so the document admitted ~19 owner rows and the
rest were present on disk and absent from every surface — list, archive
toggle, search. The spine had the same shape (`recent_sessions` took the
newest 14 files by mtime, no kind filter), which is why `/api/workspace`'s
`work` was cron; `paintRecent` preferred that over the archive rows; and
`paintEmpty` never loaded `/api/sessions` at all, so a cold landing had no
way back.

## The fixes

- **One window per class** (`ecd3a70`). `sessions_read` keeps the owner's
  own `SEARCH_BOUND` window beside a small `BACKGROUND_WINDOW` (40) for
  scheduled/delegated/channel/probe rows, and reads the background count
  across the whole archive so the number stays honest.
- **The spine is the owner's, and their delegation** (`a2b3494`). Two
  owner rulings landed mid-session: *"the spine should only use user
  initiated sessions, everything else is internal housekeeping"*, then the
  refinement *"a user initiated top goal needs the delegation to be part
  of the spine — the delegation is not user initiated but the processes
  belong to the parent's goal."* So: scheduled runs, probes and peer
  channels are walked past by a first-line classification
  (`session::start_kind`); a delegated child rides as a **thread node
  under its parent** with a `delegates_to` edge, never as a work item, and
  only when its parent is in the neighborhood (`session::start_parent`
  feeds the filter); the archived set is applied during the scan so the
  bound counts what the spine draws.
- **The graph is legible** (`d50942e`). The third ruling — *"the owner
  should be able to see the delegation graph"* — exposed a real defect:
  every child was labelled **`--- TASK ---`**, because the summary keeps
  only the first line of the child's framed opener. `RunFacts.task` now
  takes the first non-frame line of the run's first message (emphasis
  stripped, a path's underscores kept). The goal's work item carries
  `delegates: [{id,title,status}]`; the focus card says "N delegated
  processes" (counted from the tissue's own `delegates_to` edges — no
  second read); the receipts tab lists each process first, with its
  status, and a press opens the child's run.

## Live evidence (127.0.0.1:3080, after `rm`-then-`cp` + `fleet reload`)

- `/api/sessions`: **1 owner row → 110** (109 archived behind the named
  toggle — the owner's own 2026-11-13 bulk archive), background count
  171 → 739 (whole-archive).
- `/api/workspace`: **1 work item** (the groupworks goal), 4 thread nodes,
  **3 `delegates_to` edges** — no cron, no probes, no channel rows.
- The goal card: `status · working · 3 delegated processes`.
- The receipts list: `Research group travel money…` (done), `Research land
  and escorted group travel operations…` (blocked), `Research cruise group
  mechanics…` (blocked) — each openable.
- The landing, cold: "pick up where you left off" renders the owner's
  conversations (it now loads the archive itself).

## The gate

fmt/clippy clean; 2297 lib+integration tests; face harness **1264 checks
across 180 cases** (new: `F1 the landing prefers the owner's
conversations`, `DELEG the delegation graph is named and listed`, plus two
spine tests — the scan keeps a child with its parent and drops an orphan;
the read serves the goal's processes without a work item). Ledger clean:
543 rows, 2 open, no malformed cells.

## Ledger

- **U50** — a window is per class; the owner's work is never crowded out.
- **U51** — the spine is the owner's conversations, and their delegation;
  the graph must be legible, not merely drawn. C3 of the project-centered
  workspace plan is amended, not deleted: the `delegates_to` edge survives
  as the parent goal's own process.

## Contract docs updated (owner-approved)

- `docs/features/gui.md` — the home's recent rows read the archive (and
  the home asks for it), the window is per class; a new **connections
  column** bullet names the spine's rule and the legible delegation graph.
- `FUNCTIONAL-SPECIFICATIONS.md` § "The landing" — the recent list's
  `class == "yours"` now holds end-to-end, because the read bounds the
  owner's conversations separately from the system's runs.
- `TECHNICAL-DOCUMENTATION.md` — no update needed; no section there
  describes the sessions read, and the API table in `gui.md` still names
  it accurately.

## Notes for the next session

- **`fleet reload` restarts the service; it does not install a build.**
  Copy the binary (`rm` then `cp`, never in-place) and *then* reload, or
  the face keeps serving the old one.
- **The remaining findability gap:** the archive's search matches
  id/title/workspace/model/kind/job/class/state — never the words inside a
  conversation. The server has a content pass (`/api/sessions?query=`,
  `session_match`, D9) with **no UI caller**. That is the other half of "I
  remember what I asked for".
- **Delegated children still don't ride the archive rail** (class
  `delegated`), by the 2026-10-29 ruling. They are reachable from the goal
  — the graph and the receipts — which is where the owner now finds them.
- Session-close hygiene: `fleet hygiene` pruned 0 checkpoints, 0 bytes;
  no orphaned fleet processes; corpus needed no consolidation.
- Cache sweep: free space 85.3 → 86.2 GiB, 1020 MiB of superseded
  compilations dropped (release artifacts untouched).
