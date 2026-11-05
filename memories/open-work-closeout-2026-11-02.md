---
name: open-work-closeout-2026-11-02
description: "project 2026-11-02 — the closeout plan EXECUTED WHOLE and COMPLETE: O1 amended, the embed bounds MEASURED to 27,321 ms, settings U1–U10, argument sources + fold artifacts, MCP outputSchema validation + readOnlyHint tiers, the conversation-control pins, the right fixes after the pause (anchored hunk verdicts, honest validator, defaults table), the gui-revamp residuals closed, ALL of tool-surface's 14 residuals closed, C18 PROVEN LIVE (composed joins the judged set on both sides), C20 closed by ruling, C22 cl…"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Executed `docs/plans/2026-11-02-open-work-closeout.md` on the owner's
`/goal implement plan as designed`. **20 of C1–C22 ticked with evidence**;
pushed through `26c6a38`. The owner paused the goal mid-run with
*"we have to do the right fixes not the cheap fixes"* — see
[[right-fixes-not-easy-fixes-beyond-the-plan]] — and the pause became its
own increment: the three shortcuts were rebuilt properly before anything
else was ticked.

**E1 — O1 closed by AMENDMENT, not code (KI-399 closed).** The ruling named
"the relay's own minted-session store (`relay-sessions.json`)", but that
file is the FACE's home file and the deployed relay is a sibling container
that cannot read it. The owner chose *"amend the ruling: the join secret is
the boundary"*. Recorded in `relay/relay.py`'s header, the open-calls O1
row, and KI-399. ([[a-ruling-has-to-name-what-the-check-can-see]])

**E2 — the embed bounds are a MEASUREMENT now (KI-249 closed).** The
`embed_timing` rows live in `~/.local/share/dev-decisions/logs`, not
`~/.fleet/logs`. Window met: 60 `embed` rows over
2026-10-26T00:04Z→2026-11-02T02:30Z, median 83 ms, p95 5,921 ms, max
9,107 ms. `calib::embed_bound_ms` (max × 3, floored at 10 × median, capped
at 600 s) → **27,321 ms**; `PROBE_TIMEOUT` and `EMBED_TIMEOUT` both read
it, each comment naming window/rows/p95/max. The old 5 s cut the p95; the
10 s had one sample of headroom.

**E3 — settings U1–U10.** `settingsSay`/`paintSettingsStatus` paint a
board-level line (`saved <label>`, the refusal's words, the unreachable
sentence) and all 22 settings notices route there, never to the
conversation stream; `settings::unset` + `compiled_default` power one-press
revert; the filter's no-match card, `openSettingsItem` three-part route,
arrows/Enter; blur validation beside the field holding the save; the three
derivations' next steps; `money` removed (seven rails); the One vocabulary
table; and the door now stays up while the `home` step is incomplete,
rendering the read's ladder done/now/next.

**C13 — MCP results are typed.** `ToolInfo.output_schema` parsed from
`tools/list`; `mcp::validate_structured` checks root type, `required`,
property types, `enum`, `const` and array `items`, and returns a **three-way
verdict** — `Ok`, `Mismatch(reason)`, or `Unchecked(keywords)` naming every
keyword the schema declares that the check does not read (known-unread words
and words it has never heard of alike), so an unvalidated answer never
reads as a valid one. `readOnlyHint` selects a **tier** (destructive > read
> standard, the stricter winning) recorded on the `tool_screen` row; a hint
still never grants.

**C14 — the conversation-control pins.** The loop's control handling became
`apply_control` returning `ControlEffect`, so a test drives `Control::Fold`
(arms the boundary, writes no message) and `Control::Pause` (a resume
releases it; a stop lands inside the wait). An unusable pin writes
`Record::PinRefused { asked, reason }` instead of being swallowed. Two
harness cases pin the queue's drag reorder + a failed run draining the
head, and a live turn's rewind returning at the end.

**The right-fix rebuild after the pause.** `Hunk` carries its context **per
side** (three lines each way, trimmed so a neighbour's changed lines never
enter this hunk's anchor); `reject_hunk` and `accept_hunk` both locate
their block between that context, splice only there, are idempotent, and
refuse by name when the block is absent or ambiguous. The reject had the
same stale-coordinate defect, so the right fix repaired the shipped verb
too. Per-hunk accept landed end to end (`accept_hunk`, the bridge verb,
`POST /api/diff/accept`, the card's two verdicts) with the body carrying
the **reviewed hunk** — its blocks and context — not an index, because a
decision changes what a fresh diff would number. `compiled_default` is a
real table tied to the constants the code itself falls back on (the pin
caught the first draft's wrong `approvals` claim).

**E4/E5/E6.** `Record::ToolResolution.argument_sources` (model vs schema
default) with a denial pin, and the audit reads them; the fold's stub names
the tool and the spilled artifact; `--all-targets` green, five integration
runs, an 80/120-column TUI pin, `arch-gate` PASS, `docs-gate` exit 0; the
sys1 extraction leg **cancelled** with a status pass. **`scripts/plan_check.py`**
is KI-398's checker (C21) — and its verdict is now **"every completed plan's
acceptance criteria are ticked"**.

**C15 — the gui-revamp residuals, all ten closed or restated.** The
real-run SSE contract test (`tests/gui_sse.rs`: the real agent thread over
the real bridge and server against the cross-process mock-server binary,
the stream carrying session, turn, the streamed tokens and the finish); the
refused router document keeps the last good snapshot; the four-leg gate
recorded; the owner's own openings restated as their own use; boot
autofocus; per-hunk accept; the strip and rules restated with the rulings
cited; the superseded token wordings restated with `919416a`.

**tool-surface's 14 residuals, all closed.** The plan-mode prompt states the
boundary and the bash exception before the model reaches it; the audit
prints truncation and redundancy as shares with their counts; `read_window`
no longer scans to EOF for the total (the scan stops one line past the
window and a continued read's header says "at least N lines"); `replace_all`
and the capped diff pinned; the type refusal carries `valid: …`; the
byte-identical declaration order pinned; the workload profile's
reproducibility and absent-verdict default pinned; the read-only failover
records both attempts and the write boundary refuses without a second; a
defaulted argument names its source (machine-readable schema `default`
keywords); the retrieved resolution names its ranker and score; the
fold's 30-call fixture retains every ledger item; the tools card shows the
audit as shares.

**Open — and why they are not mine to close:**
- **C18** the router's composed leg. `composed` is a real workload (its
  own preference entry, its own blend1 candidate) but it is **not in
  `JUDGED_WORKLOADS`** — the closed set the sys1 `route` task can answer —
  so no live run can produce the verdict C18 asks for. Both branches the
  criterion offers are rulings: add `composed` to the judged set (then a
  live run can prove it), or rule the leg unreachable. A session cannot
  make either call.
- **C20** beta pass three — the owner's own use, through Fleet itself.
- **C22** the queue's closing rule, which closes after those two.

**CLOSED 2026-11-02, the same evening — the plan is COMPLETE at `42b5837`:**
- **C18 PROVEN LIVE.** The owner's answer to "C18 needs your ruling" was
  *"you can do it if I give you permission to do it, and you have it"* —
  the ruling was already given. `composed` joined the judged set on
  **both** sides (fleet's `JUDGED_WORKLOADS` and the vendored sys1 lane's
  `ROUTE_WORKLOAD_LABELS`), and `tests/composed_live.rs` drives the real
  lane over the real route: the verdict is `composed`, the walk's first
  candidate is `blend1`, and the lane's own answer was `composed` at
  0.892. See [[composed-joins-the-judged-set]].
- **C20 closed by ruling** — the owner's manual pass three is the beta
  gate; the plan records the close as theirs, not a fabricated pass. See
  [[c20-closed-by-ruling-owner-executed]].
- **C22 closed on the empty queue** — every residuals row ticked in its
  owner's status pass, and `scripts/plan_check.py` reports **"every
  completed plan's acceptance criteria are ticked"**.

**The delivery state, closed the same evening.** The plan was complete at
`42b5837` and the recap pushed at `3ebf1fe`, but the owner's *"did you push
to the remote and reloaded the app?"* found the face still serving the
pre-closeout build — the reload is its own pass and it is owed whenever the
tree changed since the last restart. Reloaded: build in the master worktree
(the plist's ProgramArguments[0] points there), `launchctl kickstart -k`,
served bytes proven. See [[fleet-gui-is-a-launchagent]] and
[[rebuild-is-not-commit-push-or-ship]].

**The last two asks, same evening — both were the same defect.**
- *"did you mark the plan as completed, and ticked off all the work?"* —
  **no**: every criterion was ticked and the checker passed, but both
  plans still read `status: active`. Ticking a box is not flipping the
  status; the plans README's rule is *"update status in the same turn the
  work lands"*, and a completed plan left `active` is the same stale-read
  defect as an unticked one ([[a-plan-never-ticked-is-not-a-status]]).
  Fixed: `settings-ux` (U8 and U11 were its last unticked — the model
  section's pins and the gate run) and the closeout plan both flipped to
  `completed`, pushed `a061ae8`. The checker's verdict is unchanged: every
  completed plan's acceptance criteria are ticked.
- *"has this work been committed?"* — yes, and the answer is a
  four-leg line, not a yes: **committed, pushed (0 ahead / 0 behind,
  `a061ae8` the tip), rebuilt, restarted** — and the uncommitted files
  in the tree are the parallel session's, named as such. The owner's
  asks have converged on the same four legs every time
  ([[rebuild-is-not-commit-push-or-ship]]).

Related: [[completed-plan-unticked-criteria-census-2026-11-02]],
[[a-plan-never-ticked-is-not-a-status]],
[[settings-ux-plan-2026-11-01]],
[[right-fixes-not-easy-fixes-beyond-the-plan]],
[[composed-joins-the-judged-set]],
[[c20-closed-by-ruling-owner-executed]],
[[a-ruling-has-to-name-what-the-check-can-see]],
[[fleet-gui-is-a-launchagent]],
[[rebuild-is-not-commit-push-or-ship]].
