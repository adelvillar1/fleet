# The chat and composer, made to excel — and the MCP registry, shipped whole

Status: completed
Date: 2026-10-23

Two plans implemented end to end in one session, plus the corrections
the owner's own dogfooding forced along the way. The through-line: the
owner kept using the face while it was being rebuilt, and every use
found something real — a control that landed on the wrong surface, a
footer that could not take one more button, a composer that lost its
draft when the conversation changed.

## What changed

**The MCP server registry, complete** (`a063f66`…`4b7a9ec`, plan
`2026-10-22-mcp-server-registry.md`, 10/10 criteria): `mcp.json` beside
`lanes.json` with env NAMES only and no field a value could occupy; the
scan over the machine's four foreign harness configs (16 entries found:
13 stdio, 3 HTTP refused by shape) with advisorial badges read off the
entry's own launch shape; import with the source badge and the source's
own enabled flag; run-start pre-join for enabled servers with the named
env refusal before anything spawns; `fleet mcp health`; and the
settings card. The build itself caught two things worth keeping: the
desktop configs carry credentials INSIDE the args (`docker run -e
NAME=VALUE`), so the scan extracts those pairs at the read — and the
first live scan printed a GitHub PAT before that fix landed, which is
why the extraction is at the scan, not the import. The owner then said
"import them": 13 servers in the live registry, 6 probe ok (49 tools
that now pre-join every owner-facing run), 7 named failures (4 env
names to seal, 1 relative-path cwd, 1 broken Python install, 1 slow
docker warm-up).

**The chat and composer, made to excel** (`c204791`, `ee5b305`,
`f813b63`, plan `2026-10-23-chat-composer-excellence.md`, 12/12
criteria): the goal always in the run bar with a set/replace/clear
popover; per-conversation drafts (the one global key migrated and
consumed); edit-and-resend riding the append-only rewind; paste-to-attach
through the picker's own bounds; the `@` typeahead over files, sessions,
skills and memory; the since-you-were-here divider, back-to-live with
its unread count, and hidden-tab notifications; the receipt's artifacts
with diff peeks and clickable paths in replies; turn folding with the
one-press expand; literal content search over the 200 newest logs with
the matched line on the row; "what the model saw" — the request
assembly factored into one composer the loop and `/api/context` share,
pinned byte-equal against the captured provider bodies; and the
single-tool re-run through the executor, answer in the drawer, never in
the log, `write_file` refused by name.

**Small, owner-driven**: the work list folds to a 48px strip and
follows the folded rail (KI-305/306); the conversation's text size is a
multiplier over the whole transcript after the composer-only first
draft was corrected (KI-307); the run's telemetry rows joined the
scale.

## Commits this session

Owner-owned; the session's spans (oldest first): `2d88d70` (activity
pins) → `ae70ca5` (work list folds) → `25a93f8`/`c96d729`/`10b8bd6`
(text size, corrected to the transcript) → `dea0d1a` (MCP plan
corrected) → `a063f66`/`82b7679`/`32a1099`/`14e3bfe`/`4b7a9ec` (MCP
increments 1–5) → `93e31b1`/`1859aaf` (chat plan drafted, audit layer
joined) → `c204791`/`ee5b305`/`f813b63` (chat increments 1, 2–6, 7–9)
→ `464acc0` (MCP close-out docs).

## Verification

Every increment gated: fmt, clippy, release, cargo test — final state
1436 lib tests + 1591 total passed / 1 ignored, face harness 215 checks
across 26 cases. The byte-equality pin (`what_the_model_saw_is_the
_request_itself`) and the re-run pin (`a_recorded_call_re_runs_without
_touching_the_log`) are the two tests this session is proudest of. Two
refactor regressions (the folder arms' texts, the skills cap) were
caught by the suite and fixed before the gate ran — the gate catching
its own refactor is the system working.

## Ledger

KI-285 (the lock), KI-287 (the flake, mechanism finally named and
closed), KI-302–308 filed and closed this stretch; 308 rows, 33 open.
The twelve named open rows sorted for the owner: five mechanical, two
flakes, two daily-biters (KI-284's write guard above all), three
design rows. The owner's ruling: the ledger burn-down waits until the
harness is reliably usable, then it gets taken care of.

## Doc updates

Applied in-session as the work landed (not deferred):
`docs/features/mcp.md` (new), FUNCTIONAL-SPECIFICATIONS, TECHNICAL-
DOCUMENTATION, CLAUDE.md's pointer index, `docs/features/gui.md`,
`docs/architecture/surfaces.md`, and both plans flipped to `completed`
with their state paragraphs recording the honest history — including
the MCP drafting session that went sideways and the chat plan's
mid-build corrections. No deferred doc debt.

## Uncommitted work, not this session's

The tree carries ~970 lines of uncommitted work (src/lifecycle.rs,
src/tool_coverage.rs, tool_advisor changes, two draft plans: the
tool-availability gaps and judgment-beats-one-lane/swarm). That is the
tool-use improvement session's WIP — the swarm router plan the owner
named is drafted, not built. Left untouched on purpose.

## Notes for the next session

- The face runs the new build (restarted after the final commit); the
  owner's live registry holds 13 imported servers, 6 ok.
- The swarm/execution-topology plan is the next build; its two named
  seams (spawn-per-row, the discarded second head) are verified in the
  tree.
- The `sk-` credential-scan constraint (KI-308) remains live: no served
  asset may contain the literal string.
- Build cache: several release builds this session; `scripts/cache
  -sweep.sh` is available if space tightens.
