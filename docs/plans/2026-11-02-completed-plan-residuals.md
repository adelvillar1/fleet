---
status: active
created: 2026-11-02
updated: 2026-11-02
slug: completed-plan-residuals
---

# Plan: the residuals behind the completed plans

## Context

On 2026-11-02 a sweep asked a plain question — *what deliverables were
left behind?* — and found that the plans could not answer it. Eight
plans sat at `status: completed` with 61 acceptance criteria unticked
(KI-398): the work had landed but the boxes were never ticked, so the
plan answered neither "what shipped" nor "what did not".

Each of those plans now carries a **Status pass — 2026-11-02** section:
every criterion walked against the tree at HEAD, the ones with a
verifiable artifact ticked with the `path:line` that proves them, and
the rest left open with what is missing. This plan is the home for
those residuals, because the plans README forbids reopening a completed
plan and because residuals with no owner are the same defect one level
down.

One residual is not a test gap but a divergence from a ruling: **O1** in
`2026-11-01-open-calls.md`. The ruled mechanism (a) was to validate a
relay join's `sid`/`hash` against the relay's own minted-session store;
what shipped is a shared deployment join secret (`relay/relay.py:250`).
A secret-holder can therefore still self-assert a new `sid`, and the
refusal that is tested is "no session proof", not "not a minted
session". Filed as KI-399.

## The residuals

Each row names the plan, the criterion, and the missing piece. Nothing
here is a re-litigation of scope: these are the halves of criteria whose
other halves shipped.

### `2026-10-23-tool-surface-and-lane-resolution.md` (14)

- **C0** — no plan-mode prompt exists (only the refusal string), and
  `docs/features/gui.md` does not state that `bash` stays allowed for
  reading in plan mode; the media-type sniff has no direct test.
- **C1** — the resolution record carries no argument sources; no test
  asserts a denial's resolution record.
- **C2** — truncation and redundancy print as counts, not rates.
- **C3** — no 4,000-line `offset=3800` test, no empty-file test, and the
  total-lines scan reads to EOF, so bytes read are not window-bounded.
- **C4** — no `replace_all` test and no capped-diff test.
- **C7** — no 30-call forced-fold fixture retaining every item and every
  artifact path.
- **C8** — the refusal names the field and the got-value but not the
  `valid: …` candidate list; tools are not one module each.
- **C9** — the bounds are 36 tools / 10,000 tokens (the plan asked 28 /
  8,000); no byte-identical-declaration test.
- **C10** — the resolution reason says only "activated by tool_search";
  no sem1 scores, no lexical-fallback or below-floor test.
- **C11** — `prose` starts read/write/glob rather than web; no
  `deep_context` or low-confidence test.
- **C12** — no failover retry (a provider failure returns the error),
  and the chosen reason is computed then dropped.
- **C13** — no model/catalog enum and no test asserting no model request
  during argument resolution.
- **C14** — the elision stub names neither the tool nor an artifact path.
- **C15** — `structuredContent` is never validated against an
  `outputSchema`; `readOnlyHint` is unread.
- **C16** — the tools card has no deferred count; the gate run is not
  recorded.

### `2026-10-16-gui-revamp.md` (10)

- **C0/C6** — *superseded, not missing*: the per-launch URL token was
  removed by `919416a` in favour of loopback-only plus Host/Origin
  checks. The criteria's wording still describes the token; they should
  be reworded or retired by a later ruling rather than ticked.
- **C1** — no test drives a real run's tokens and tool cards through
  `/events`; `mock_chat` serves `/btw` only.
- **C2** — a server-issued `refused` router document replaces the last
  good snapshot instead of retaining it.
- **C7** — no recorded four-leg gate run at or after the landing.
- **C8** — only the G5 opening is on record; no evidence the owner
  opened every shipped surface.
- **C9** — no boot autofocus, and the onboarding door hides the composer
  on a fresh home.
- **C11** — router decisions, memory recalls and skill loads fold into
  the run-strip line rather than rendering as inline cards.
- **C12** — the active rules live in the trust card, not the
  conversation header.
- **C13** — the diff has per-hunk reject but no per-hunk accept.

### `2026-10-23-conversation-control.md` (5)

- **C3** — drag-reorder and the finished/failed flush are unpinned, and
  the drag is HTML5 DnD rather than the pointer events the plan names.
- **C4** — the forced fold is driven from options, not through
  `Control::Fold` and the loop's own call.
- **C5** — an unusable pin is swallowed without a name; no integration
  test serves a second turn on the pinned provider.
- **C6** — no test drives the loop's pause wait.
- **C8** — the face test on a live-turn row (rewind/stop) is absent.

### `2026-10-17-ki-triage.md` (3)

- **C2** — CI runs `cargo test`, not `--all-targets`, and no green
  all-targets run is recorded at HEAD.
- **C4** — no 80/120-column clipping assertions for the named surfaces.
- **C5** — the `wait_for_port` mitigation landed; no five-consecutive-run
  artifact exists.

### `2026-10-21-docs-speak-the-product-and-the-program.md` (3)

- **C2/C3** — the heading map holds exactly; `arch-gate` and `docs-gate`
  were not run (both write state).
- **C4** — the "exactly three files cite each note" proxy no longer
  holds (18 and 6 files do); the criterion needs a new test or a
  restatement.

### `2026-10-22-skills-the-model-can-reach.md` (3)

- **C2** — the route is `POST /api/skill/retire` (singular), not
  `/api/skills/retire`; retire is a pane button, not a row action.
- **C3** — the retire control is not beside the never-selected mark.
- **C4** — the live-run log evidence the criterion asks for is not
  captured (the behaviour is code-pinned).

### `2026-11-01-open-calls.md` (1)

- **O1** — the mechanism diverges from the ruling (see Context); KI-399
  carries it.

## Approach

Work the residuals in the order their risk puts them, not the order this
file lists them:

1. **O1 (KI-399)** — a security-relevant divergence; decide whether the
   ruling or the implementation moves, then make the two agree.
2. **tool-surface C1/C7/C14/C15** — the resolution record and the fold
   artifacts are the evidence the rest of that plan's claims lean on.
3. **The test-only residuals** (C3, C4, C12, C13, gui-revamp C1/C13,
   conversation-control C3–C6/C8) — pin what already behaves.
4. **The wording residuals** (gui-revamp C0/C6, docs-speak C4,
   skills C2) — restate the criterion to match the ruling that
   superseded it, rather than building to stale wording.

Each residual closes by one of: a pin that proves the behaviour, a code
change that adds the missing half, or a restated criterion with the
superseding ruling cited — never by deleting the row silently.

## Acceptance criteria

- [ ] **R1 — O1 is settled one way or the other.** Either the join
      validates against the minted-session store as ruled, or the ruling
      is amended by the owner and the plan's O1 row cites it. KI-399
      closes with the same answer.
- [ ] **R2 — the evidence residuals are pinned.** tool-surface C1's
      argument sources and denial test, C7's 30-call fold fixture and
      C14's artifact path land; C15 either validates `outputSchema` or
      records that no MCP server in use declares one.
- [ ] **R3 — the test-only residuals are pinned.** The six
      conversation-control rows and gui-revamp C1/C13 each gain the pin
      their criterion names, or a stated reason the pin cannot exist.
- [ ] **R4 — the wording residuals are restated.** gui-revamp C0/C6 and
      docs-speak C4 carry the ruling or measurement that superseded
      them, and skills C2's route name matches the code.
- [ ] **R5 — the completed plans read as state.** Every plan with a
      Status pass either has no open residual left or names the residual
      R1–R4 owns it; the plans README's rule holds again.
- [ ] **R6 — the checker exists.** A read-only check lists every
      completed plan whose acceptance criteria still carry `- [ ]`
      boxes, so the next sweep is a command and not an audit.

## Files to be touched

`docs/plans/*.md` (the status-pass tables and the residuals above), the
pins named in R2/R3 (`tests/`, `src/tools.rs`, `src/agent.rs`,
`src/compaction.rs`, `relay/relay.py`), `docs/known-issues.md` (KI-398,
KI-399), and `scripts/` (R6's check).

## Out of scope

- Re-opening any completed plan's scope. The residuals are the unshipped
  halves of criteria already written.
- Rebuilding the surfaces the shipped criteria describe.
- The settings-ux residuals (U1–U7) — that plan is active and carries
  its own status pass.

## Verification

Each residual's own evidence: a test name in the suite, a `path:line`
that carries the change, or a restated criterion citing the ruling. The
closedown runs the four-leg gate and the face harness by exit code, and
R6's check over `docs/plans/`.

## Linked artifacts

- `docs/plans/2026-11-01-settings-ux.md` — the status pass that named
  the pattern
- `docs/known-issues.md` — KI-398 (the bookkeeping defect), KI-399 (O1's
  divergence)
- `docs/plans/README.md` — the rule this plan serves: never reopen a
  completed plan; start a new one
