---
status: active
created: 2026-10-24
updated: 2026-10-24
slug: known-issues-burn-down
---

# Plan: the known-issues burn-down — the ruling's sorted twelve

## Context

The ledger burn-down was deferred by ruling (2026-10-23) until the harness
was reliably usable; the owner lifted it this session ("let's work on the
known issues list"). The ledger stands at 310 rows — 257 closed, 35 open,
17 noted — and all twelve rows of the ruling's sorted list (mechanical
256/277/294/282/308, flakes 288/293, daily 284/289, design 278/296/297)
are still open. Every row's code site was re-verified against the current
tree before this plan was written; three rows outside the sorted twelve
turned out to be already fixed by shipped work and close with evidence
only (KI-282, KI-248, KI-242), and two are one-line quick wins (KI-238,
KI-237).

One design fork the row deferred to the owner (KI-256's approval shape)
was put to the owner and went unanswered; this plan records the
recommended ruling — **the per-job allowlist** — and builds it. Overturning
it later is a plan amendment, not a rebuild.

Also corrected at session start: HEAD `de6878d` does not compile — the
composer session's two-arg `Bridge::sessions` landed without the remote
listener's call site (the gate ran on the working tree, which carried the
fix uncommitted). The hunk commits first, alone.

Working discipline for every increment: one gate, one commit each,
explicit paths; every ledger row touched in the same commit as its fix;
the plan's `status` updated in the same turn the work lands.

## Approach

The increments and the criteria they carry:
increment 0 lands the missed hunk (C0); increment 1 closes three rows
with evidence and no code (C1); increment 2 resolves the skills directory
at the unattended sites (C2); increment 3 reshapes the credential scan
(C3); increment 4 bounds the two flakes (C4); increment 5 reconciles the
cron task strings (C5); increment 6 exempts the write guard's unrenamable
content (C6); increment 7 lands the two quick wins (C7); increment 8
gives the face its file address (C8); increment 9 wires the per-job tool
allowlist (C9); increment 10 separates the route home from the improve
home (C10); increment 11 settles the vendored lanes' steady state (C11).

### Increment 0 — the missed hunk (C0)

`src/remote/listener.rs:507` calls `bridge.sessions(&session)`; the
committed signature at `src/gui/bridge.rs:1233` is
`sessions(&self, current: &str, query: &str)`. The change is the working
tree's own one-liner — pass the decoded `q` query parameter through, the
way the gui server's own `/api/sessions` route does. Commit it alone
before any other work so every later diff is honest; `cargo check
--all-targets` before and after, exit codes read directly.

### Increment 1 — evidence closes (C1)

No code. Three rows describe work that already shipped:

- KI-282: the face serves `/favicon.ico` (`src/gui/server.rs:244` answers
  with the icon; the asset table at `:1648` maps the path), and the live
  probe this session returned 200 — the PWA install work landed it.
- KI-248: the control file carries the three surfaces the row asked for —
  `Control::Steer` folds an owner message into the running turn
  (`src/agent.rs:2110`), `Control::Stop` returns through
  `stop_by_owner` (`:2111`), and `/btw` is the side-channel — all landed
  2026-10-23 in the conversation-control work.
- KI-242: asset replies now carry validators — `Cache-Control: no-cache`
  on documents and `public, max-age=31536000, immutable` on stamped
  assets, pinned by the tests at `src/gui/server.rs:2412/2420`.

Each row's Status becomes `closed` naming this evidence; the Next action
column records the probe or the commit.

### Increment 2 — the skills directory at the unattended sites (C2, KI-294)

`src/cron.rs:1312` and `src/subagents.rs:1642` call
`crate::skills::load(home)` with FLEET_HOME, where the loader expects the
library directory itself — so both declare nothing. Both sites resolve
through the same precedence the interactive paths use
(`skills::resolve_skills_dir`, `<home>/skills` when present), matching
`load_declared`'s behavior in `src/main.rs:958`. Steps: add a
`skills::load_resolved(home)` helper that applies the precedence once;
call it from both sites; pin with a test that builds a home carrying
`skills/<name>/SKILL.md` and asserts the scheduled path and the delegated
path each declare it.

### Increment 3 — the credential scan reads key shape (C3, KI-308)

`no_payload_carries_a_credential` (`src/gui/server.rs:3408`) fails any
payload containing the literal `sk-` — a CSS class named `task-wrap` tripped
it. The scan matches key shape instead: `sk-` followed by a run of at
least 20 key characters (`[A-Za-z0-9_-]`), and its failure message names
the matched word with its byte offset instead of dumping the payload.
Steps: extract the scan into a helper returning `Option<(usize, String)>`;
assert a `task-wrap` payload passes and an `sk-`-shaped token fails with
word and offset named.

### Increment 4 — the two flakes bound on observable state (C4, KI-288 + KI-293)

`the_serve_loop_holds_one_poll_at_a_time` (`src/remote/relay.rs:391`)
replaces its fixed wait with a bounded poll on the loop's own counter;
`the_related_route_answers_one_document_and_refuses_a_foreign_kind`
(`src/gui/server.rs:2639`) reads until the head terminator under a
deadline instead of sleeping — the same shape the control-file and seat
tests already use. A real overlap still fails; a loaded machine is given
the time the state needs.

### Increment 5 — the task-string reconciliation (C5, KI-277)

`seed_missing` (`src/cron.rs:832`) skips any job whose name exists, so
the KI-221/KI-224 wording fix never reached the fourteen live
`corpus-refresh-*` jobs. Steps: in the seed pass, when a stored job's task
string diverges from `seed_task(project)`, rewrite the task string only —
schedule, enabled, and any owner edits stay as stored. Pin: a test whose
store carries a pre-fix task string comes out of `seed_missing` carrying
the current wording; the live store heals on the next `fleet cron` seed.

### Increment 6 — the write guard exempts the unrenamable (C6, KI-284)

`guard_content` (`src/agent.rs:5808`) refuses content whose backtick-span
words case-match an established term, so mandated content — absolute
paths under the owner's home, protocol tokens, established doc names —
cannot be written at all, and callers route around the guard through
bash. Steps: the exemptions applied after the vocabulary match —
(a) the occurrence sits inside an absolute path (`/`-rooted); (b) the
token is protocol-shaped (the `mcp__`-style double-underscore
convention); (c) the token is an established file name (`CLAUDE.md`,
`TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — another
file's own established usage); (d) a Python file's docstrings are prose
and leave the guard's own candidate pass. C6 is this increment's work:
the recorded unrenamable refusals pass after the exemptions, a novel
separator-variant the author invents is still refused, and the guard's
existing negative tests stay red — KI-220's pin included, so a
backticked case variant in a doc (`RunOptions` in a plan) remains
refused and keeps the documented bare-prose shape.

### Increment 7 — quick wins (C7, KI-238 + KI-237)

The pane's credential refusal (`src/gui/bridge.rs:3545`) widens from
`name == ".env" || name.starts_with(".env.")` to
`name.starts_with(".env")` — one character, covers `.envrc` — pinned
beside the existing refusal test. The pane Esc handler's comment
(`src/gui/assets/app.js`, the `closePane` handler) states the code's real
behavior: close-when-empty, home-when-file-open.

### Increment 8 — the file address (C8, KI-289)

The face's route table carries eleven panels and no file address, so a
deliverable cannot be linked. Steps: `showRoute` parses `#/file/<path>`
beside the sessions focus-id handling, calls `openPath(focusId)`, and the
pane opens with the preview tab showing the file (rendered for markdown).
Pinned by a face check driving the route and asserting the pane and
preview state.

### Increment 9 — the per-job tool allowlist (C9, KI-256, the recorded ruling)

A scheduled run gets `lane_specs: None` (`src/cron.rs:1329`), so a job
whose task names a lane tool cannot call it. The recorded ruling: the
per-job allowlist — `Job` gains an optional `tools: Vec<String>`; the
seed writes `mcp__blend1__refresh` on `corpus-refresh-*` jobs; the cron
runner resolves those names against the enabled lanes and passes the
specs; a job with no list gets none, and auto-approve semantics are
unchanged because enabling the job is the approval. The job card shows
the tools line. Pin: a cron test asserting the allowlisted job's specs
carry the named tool and an unlisted job's carry none.

### Increment 10 — the route home (C10, KI-278)

A scheduled run sets `improve_home: None` on purpose, and the same field
is the home a delegated child routes from (`src/subagents.rs:1697` reads
it for `route::prepare_child`) — so a cron run's child runs on the
parent's committed candidate. Steps: `AgentOptions` gains a route home
distinct from the improve home; the subagent dispatch reads it;
`scheduled run` passes the sessions directory's parent (the layout's own
convention); `improve` stays gated on `improve_home` exactly as today.
Pin: a test asserting the child's `own_route` resolves from the scheduled
run's home.

### Increment 11 — the vendored lanes' steady state (C11, KI-297 + KI-296)

blend1's `_derive` (`services/blend1/src/blend1/corpus.py:257`) builds a
list while the change check compares sets, so a repeated `##` heading
leaves the derived set permanently larger than the store and every
refresh re-embeds a byte-identical corpus (four recorded instances, one
a 2,186-chunk re-embed). Steps: dedupe by key at derive time, first
occurrence kept, and report the dropped-duplicate count; pin with a
repeated-heading fixture whose steady-state refresh reports
`changed: false`. The embedder probe half: the probe retries within its
budget the way `embed` does, so one hung answer inside a flap is not read
as "no backend" (pinned by a first-slow/second-fast double whose probe
succeeds). The environmental half records today's evidence in the row —
the endpoint healthy at probe (embed 200 in 327 ms), no restart performed
on a healthy service.

## Acceptance criteria

The build order is the ruling's order: the hunk and the evidence-only
closes first, then the mechanical rows, the flakes, the daily rows, the
design rows.

- [ ] **C0 — the missed hunk.** `src/remote/listener.rs`'s `/api/sessions`
  route forwards the search query to the two-arg `Bridge::sessions`, in a
  commit of its own; `cargo check --all-targets` exits 0 before and after.
- [ ] **C1 — evidence closes KI-282, KI-248, KI-242.** Each row's Status
  reads closed with its evidence named (the 200 probe; the shipped steer/
  stop/btw verbs; the shipped cache validators and their tests), and no
  code changes in that commit.
- [ ] **C2 — KI-294.** Both unattended `skills::load` sites resolve the
  canonical skills directory via the shared precedence; a test with a
  home carrying `skills/<name>/SKILL.md` asserts both the scheduled and
  the delegated path declare it.
- [ ] **C3 — KI-308.** The credential scan matches `sk-` plus a run of at
  least 20 key characters and names the matched word with its offset on
  failure; a `task-wrap` payload passes and an `sk-`-shaped token fails
  with both named, by test.
- [ ] **C4 — KI-288 + KI-293.** Both flaky tests bound on observable state
  (the loop's counter under a deadline; read-until-head-terminator), no
  fixed sleeps, by test.
- [ ] **C5 — KI-277.** `seed_missing` reconciles a diverged task string to
  `seed_task(project)` leaving schedule/enabled untouched; a test carries
  a pre-fix string in and the current wording out.
- [ ] **C6 — KI-284.** The write guard exempts the unrenamable: docstring
  prose stripped from its own Python pass, absolute-path-only
  occurrences, protocol-shaped tokens, and established file names — the
  row's recorded unrenamable refusals (`Scheduled`, `MCP`, `Implements`,
  `Path`, `Projects`, the protocol token, `CLAUDE.md` and its siblings)
  pass after the change, and the guard's existing negative tests stay
  red, including KI-220's: a backticked case variant in a doc is still a
  doc naming a thing, so the plan-drafting three (`RunOptions`,
  `LaneJoined`, `LaneChild`) keep the documented bare-prose shape. By
  test.
- [ ] **C7 — KI-238 + KI-237.** `.envrc` is refused by the pane's
  credential read (test beside the existing refusal), and the Esc
  handler's comment matches the code's behavior.
- [ ] **C8 — KI-289.** `#/file/<path>` opens the pane with the preview tab
  showing the file, rendered for markdown; a face check drives the route.
- [ ] **C9 — KI-256.** A job's optional `tools` list reaches the run's
  lane specs; the seed writes `mcp__blend1__refresh` on corpus jobs; a
  job without the list gets none; the card shows the line; by test.
- [ ] **C10 — KI-278.** The child dispatch reads a route home distinct
  from `improve_home`; a scheduled run passes the sessions parent; a test
  asserts the child's `own_route` resolves from it.
- [ ] **C11 — KI-297 + KI-296.** blend1's derive dedupes by key and
  reports dropped duplicates (repeated-heading fixture converges to
  `changed: false`, by test); the embedder probe retries within its
  budget (first-slow/second-fast double succeeds, by test); the row
  records today's healthy-endpoint evidence with no restart.

## Files to be touched

`src/remote/listener.rs`; `src/cron.rs`; `src/subagents.rs`; `src/agent.rs`;
`src/skills.rs`; `src/gui/server.rs`; `src/gui/assets/app.js`;
`src/gui/bridge.rs`; `services/blend1/src/blend1/corpus.py`;
`docs/known-issues.md`; the tests beside each site.

## Out of scope

- The 23 open rows outside the ruling's twelve and the quick-wins pair —
  external-repo rows (KI-001, KI-003, KI-004, KI-005, KI-029, KI-143,
  KI-207), owner-decision rows (KI-066, KI-117, KI-135, KI-136, KI-249,
  KI-252, KI-268, KI-309, KI-310), and the deck/frontier rows (KI-239,
  KI-240, KI-241) each want their own session or their owner's decision;
  they stay open, recorded, not silently dismissed.
- No `Cargo.toml` changes (KI-117's tradeoff stands as ruled).
- The sem1-llama service is not restarted while it answers (KI-296's
  environmental half is evidence-and-decide, and today's evidence is
  healthy).

## Verification

Per increment: the gate's four commands (`fmt --all --check`, `clippy
--all-targets -- -D warnings`, `build --release`, `test`), exit codes
read directly, never through a status-losing pipe. Close-out: every row
worked this session closed with evidence or left open with the named
remainder; the plan's `status` and the ledger agree; commits carry
explicit paths, one increment each.

## Linked artifacts

- `docs/known-issues.md` — the rows above
- `docs/recaps/2026-10-24-tool-availability-shipped.md` — the prior
  session whose hunk C0 lands
- the memory `owner-is-a-team-of-one` — the ruling's sorted twelve
