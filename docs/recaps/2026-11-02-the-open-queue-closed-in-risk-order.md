# Session recap — 2026-11-02: the open queue, closed in risk order

**Summary.** A long session with a single arc: find what had been left
behind, then close it in the order its risk put it. The arc ran from a
plain question — *what deliverables were left behind?* — through the
discovery that the plans themselves could not answer it (KI-398), to a
one-contract closeout (`2026-11-02-open-work-closeout.md`) whose every
residual is now either landed or ruled on. Along the way the settings
surface got the pieces its plan named but never built, the router's
`composed` leg became reachable by judgment, and every completed plan's
acceptance criteria are ticked — the checker passes.

## The arc, in order

1. **The question and the two things it found.** The session's first
   work was a sweep: every plan, every open ledger row, the declared
   gaps. It split into two findings wearing the same clothes —
   *bookkeeping* (eight completed plans carried 61 unticked acceptance
   criteria) and *real work* (four active plans with unbuilt items, a
   draft plan, and a divergence from a ruling no plan owned).

2. **The bookkeeping defect (KI-398).** A completed plan could leave its
   criteria unticked, so the plan answered neither "what shipped" nor
   "what did not". Filed with the counts; corrected an hour later when
   the recount (scoped to each plan's own `## Acceptance criteria`
   heading) came out at 8 plans / 61 criteria, not 17 / 107 — the first
   pass had counted Goal and use-case checkboxes as criteria.

3. **The closeout contract.** One plan for the queue
   (`0cecba0`), ordered by risk rather than by convenience:
   - **E1 — the security divergence (O1, KI-399).** The relay join's
     ruled mechanism named a minted-session store the relay cannot read
     (that file is the face's). The owner's ruling amended it: the
     deployment join secret is the trust boundary, recorded in
     `relay/relay.py`'s header, the open-calls row, and KI-399.
   - **E2 — the measured bound (embed-bounds, KI-249).** The real
     stream gave the tail (60 `embed` rows, 2026-10-26 → 11-02: median
     83 ms, p95 5,921 ms, max 9,107 ms). `calib::embed_bound_ms` — the
     rule, not a guess — gives 27,321 ms for both `PROBE_TIMEOUT` and
     `EMBED_TIMEOUT`, each comment naming the window, count and tail.
     KI-249 closed with the same numbers.
   - **E3 — the settings surface (U1–U10).** The board's own status line
     (saved / refused / unreachable — never the conversation stream),
     a one-press revert through a new `unset` op, the filter's no-match
     card and the three-part item route, blur validation beside the
     field, the three derivations' next steps, the money section
     removed, the One-vocabulary table, and a door that stays up until
     the home step is done — with the read's ladder rendered
     done/now/next. Re-measured on a fresh home.
   - **E4/E5/E6 — the evidence and the small closes.** Argument sources
     on the resolution record, the 30-call fold fixture, the elision
     stub naming tool and artifact, the valid-candidates refusal, the
     byte-identical declaration order, the profile's reproducibility,
     the read-only failover and the write boundary, machine-readable
     schema defaults, the audit as shares, and the plan checker.
   - **E6's tail — C18, C20, C22.** The router's composed leg proved
     live; the beta gate closed by the owner's ruling; the queue's
     closing rule.

4. **The two right-fixes that were not pins.** Three places where the
   first draft was the cheap fix and the second was the right one:
   - **The read scan.** `read_window` scanned to EOF for the total;
     the pin exposed it. The scan now stops one line past the window
     and a continued read's header says *at least N lines* rather than
     a count it never reached.
   - **The MCP validator.** It returned `Ok` for every construct it did
     not implement. The check now returns a three-way verdict —
     `Ok`, `Mismatch(reason)`, or `Unchecked(keywords)` naming every
     keyword the schema declares that the check does not read — so an
     unvalidated answer never reads as a valid one.
   - **The hunk verdicts.** The first `accept_hunk` spliced at a stale
     coordinate; the anchor version refuses when the block is absent or
     appears twice. And the seat's keyboard path: a boot autofocus on
     the composer and a seat that answers on digits cannot both read
     "focused input = the owner typing", so the seat takes the focus
     when it arrives (unless the owner is mid-sentence) and the guard
     reads where the focus is.

5. **The merge discipline.** Master was held by the parallel session's
   worktree throughout; every push was built on a sibling branch from
   master's tip and gated there. The final push (`42b5837`) carries the
   whole arc: 54 commits on the day, the last twelve of them this
   session's closeout.

## The work, by the numbers

- **Gate, final:** fmt, clippy `-D warnings`, **1,776 tests / 0 failed**,
  face harness **669 checks across 61 cases**, `plan_check.py` — every
  completed plan's acceptance criteria are ticked.
- **The plan queue:** 17 plans touched; 16 closed, the one open item
  (C20) closed by ruling, C22 closed on the empty queue.
- **Ledger:** 399 rows / 9 open, checker clean; KI-398 (the unticked
  criteria), KI-399 (the relay divergence), KI-400 (the vault row's
  card) filed, KI-249 closed.
- **The live legs:** the real sys1 lane answered `composed` at 0.892
  for a grounded-composition request and the walk landed on blend1 —
  the router's composed leg, proven rather than compiled.

## Commits (this session, fleet)

| Hash | Message |
|---|---|
| `735f462` | fix(design): the frame bar's acts survive the drag, and the mode keeps its label |
| `572852b` | docs(design): the parity surface in pictures |
| `4448d4e` | chore(gitignore): the runtime dirs and the archify workroom |
| `de2d42b` | docs(readme): the architecture and a turn, drawn |
| `3d246a1` | feat(route): the roster's per-provider headers ride both request paths |
| `16c30e3` | feat(providers): the providers card names its headers and never a value |
| `c554a32` | feat(route): the conversation's session id rides the preparation, opaque and resume-stable |
| `509e0e3` | feat(api): the Responses protocol as a wire dialect, spoken per model |
| `2d1d9ff` | fix(api): the adapter compiles under -D warnings, and the W0 probe lands |
| `c64d3fd` | feat(route): the wire dialect rides the roster, and {trace} joins the placeholders |
| `a17e115` | docs(routing): the session lifecycle contract, and the parity plan completed |
| `d13ca29` | feat(settings): the account card completes the chain (settings-ux U9) |
| `c9c6139` | docs(plans): the headers row is unblocked — settings-ux U9 landed |
| `434e61c` | docs(known-issues): KI-398 — a completed plan can leave its criteria unticked |
| `3bd35ad` | docs(known-issues): KI-398's count corrected — 8 plans, 61 criteria |
| `30d7a8d` | docs(plans): the completed plans read as state again |
| `d816704` | feat(gui): the account card carries the provider's headers (remainders W4) |
| `0cecba0` | docs(plans): one contract for the open queue — everything but session parity |
| `d5f7416` | feat(settings,bounds,relay): the open queue's first three increments |
| `6aa0459` | feat(tools,control,plans): the evidence residuals and the checker |
| `f39c5bf` | feat(mcp): the typed answer is checked, and the hints pick a tier (C13) |
| `ae87bf2` | test(control,queue): the pins C14 asked for (conversation-control C3-C8) |
| `4240622` | feat(checkpoints): a hunk verdict anchors on the review, not a line number |
| `4a1910a` | fix(mcp): a schema the check cannot read says so, never "valid" |
| `6fab5bf` | fix(settings): the read's defaults are the constants the code uses (U2) |
| `2ffcd79` | feat(face): the seat answers from the keys (gui-revamp C12) |
| `72f5efc` | fix(tools): the audit prints shares, not counts (tool-surface C2) |
| `12f00e3` | feat(tools): the audit reads the argument sources (tool-surface C1) |
| `360cf13` | test(tools): the 30-call fold keeps every ledger item (tool-surface C7) |
| `b851fba` | feat(agent): plan mode's prompt states the boundary (tool-surface C0) |
| `9ee9338` | fix(fileops): a windowed read is bounded by the window, not the file (C3) |
| `2594829` | test(fileops): replace_all and the capped diff, pinned (tool-surface C4) |
| `7388f57` | feat(tools): a defaulted argument names its source (tool-surface C13) |
| `7c1f7ec` | feat(tools): the type refusal names its candidates (tool-surface C8) |
| `1b1834d` | test(tools): the declaration-order, profile and dispatch pins (C9-C12) |
| `2d87f7d` | feat(agent): a retrieved resolution names its ranker and score (tool-surface C10) |
| `ebbf868` | feat(face): the tools card shows the audit as shares (tool-surface C16) |
| `26c6a38` | docs(plans): the gui-revamp residuals are closed (closeout C15) |
| `42b5837` | feat(route,sys1): composed is a judged workload — proven live (C18) |

Sibling repos: none this session — the whole arc was fleet.

## What the build proved — and the lessons it taught

- **The plans could not answer their own question.** Eight completed plans
  carried 61 unticked criteria; the first sweep counted 107 because Goal and
  use-case checkboxes were counted as criteria. The recount scoped to each
  plan's own `## Acceptance criteria` heading, and the checker that keeps it
  true is `scripts/plan_check.py`.
- **A harness is not a browser.** The vault row's card returned a node where
  the drawer walks a section list; the live shoot found dead title-bar acts.
  Two lessons in one: the harness drives listeners directly, so anything the
  browser's event path decides (a preventDefault, a default action) is
  invisible to it — and a press that looks pressable must be pressed.
- **The cheap fix is the one that passes.** Three times the first draft was
  the cheap fix and the pin or the live run said so: the read scan, the MCP
  validator, the hunk verdicts. Each now fails loudly instead of passing
  silently.

## Honest notes

- **C18 is proven, not assumed.** `tests/composed_live.rs` drives the real
  sys1 lane over the real route and asserts the verdict and the first
  candidate; the lane's own answer for the probe task was `composed` at
  0.892. It needs the lane up (the keys resolve on this machine) and it is
  the one test that would fail on a keyless machine — which is the honest
  shape of a live proof.
- **C20 is the owner's.** The beta gate's pass three is a manual walk; the
  criterion is closed by ruling, with the substance — the owner's own use is
  clean — recorded as what no test can stand in for.
- **The uncommitted files are the parallel session's.** `src/gui/server.rs`
  and `src/main.rs` carry edits that aren't mine (the heartbeat visibility
  and a rustfmt reflow); `docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md`
  is their draft. Left untouched, as the co-tenant rule requires.
- **The recap is a journal, not a gate.** The gates are the four legs, the
  harness and the checker; this file is the story of why they ran.
