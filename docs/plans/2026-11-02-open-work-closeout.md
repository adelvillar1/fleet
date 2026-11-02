---
status: active
created: 2026-11-02
updated: 2026-11-02
slug: open-work-closeout
---

# Plan: the open work, closed in risk order

## Context

The 2026-11-02 sweep asked what had been left behind and found two
different things wearing the same clothes. Eight completed plans carried
61 unticked acceptance criteria (KI-398) — most of that was bookkeeping,
and each plan now carries a **Status pass — 2026-11-02** table that
ticks the proven criteria with a `path:line` and names what is missing
in the rest. Behind the bookkeeping sat the real queue: four active
plans with unbuilt work, one plan that needs a status pass before anyone
can say what is left, and a divergence from a ruling that no plan owned.

This plan is the single contract for that queue — **every open item
except `2026-11-02-session-parity-remainders.md`**, whose C4 headers row
another session is building. It folds in
`2026-11-02-completed-plan-residuals.md` (created an hour earlier for
the same residuals; that file is superseded and points here), so there
is one home and not two.

## What is in scope

| Source | Open item | Why it is first or last |
|---|---|---|
| `2026-11-01-open-calls.md` O1 + KI-399 | the relay join validates a shared secret, not the minted session the ruling named | security: a secret-holder can assert a new `sid` |
| `2026-10-24-embed-bounds-from-measured-tail.md` C0–C4 + KI-249 | the probe/embed timeouts were never measured against the real tail; a call outliving its bound is dropped and reported as a failure | a live defect, and the measurement is cheap |
| `2026-11-01-settings-ux.md` U1–U7 (+U10 re-measure) | on-surface confirmation, revert, filter completion, field validation, named refusals, the money section, the vocabulary passage | the surface the owner called atrocious |
| `2026-10-23-tool-surface-and-lane-resolution.md` (15) | the evidence half: argument sources, the fold fixture, artifact paths, `outputSchema` | the rest of that plan's claims lean on these |
| `2026-10-23-conversation-control.md` (5) | pins for the queue drag, control-driven fold, unusable pin, pause, live-turn rewind | behaviour exists; the pin is the gap |
| `2026-10-16-gui-revamp.md` (10) | real-run SSE test, refused snapshot, gate evidence, autofocus, inline cards, header rules, per-hunk accept, two superseded wordings | mixed: tests, one wording pass, two small builds |
| `2026-10-17-ki-triage.md` (3) | `--all-targets` green run, 80/120-col clipping, five-run flake artifact | evidence-shaped |
| `2026-10-21-docs-speak-the-product-and-the-program.md` (3) | `arch-gate`/`docs-gate` runs, the exact-three-files proxy | one run, one restatement |
| `2026-10-22-skills-the-model-can-reach.md` (3) | route name, retire placement, live-run log evidence | small |
| `2026-10-15-router-enforcement.md` | the one leg: a live run where the real sys1 judges a task `composed` | external behaviour, needs a calibrated probe |
| `2026-10-30-beta-shakeout.md` B5 | the beta gate: pass three after the owner's own use | owner-dependent |
| `2026-10-19-sys1-extraction-leg.md` | no usable checklist — needs a status pass, then what survives it | unknown until read |
| KI-398 R6 | the checker that lists completed plans with unticked criteria | keeps this from recurring |

## Approach

Six increments, each closing whole. Nothing here re-opens a completed
plan's scope; a residual is the unshipped half of a criterion already
written, and it closes by a pin, a code change, or a restated criterion
with the superseding ruling cited — never by deleting the row.

### E1 — the security divergence (O1, KI-399)

Decide with the owner which way it goes: (a) validate the join against
the relay's minted-session store as ruled, or (b) amend the ruling to
name the shared secret and the trust it implies. Then make the code, the
test, and the open-calls O1 row agree. This comes first because the
current state trusts anyone who holds the secret to assert any session.

**Closes:** C1.

### E2 — the measured bound (embed-bounds, KI-249)

Run the window gate on the real stream (≥20 `embed_timing` rows and ≥24
hours of span, or say the shortfall by name), add `calib::embed_tail`,
derive the two constants by the rule, and record the provenance on each
constant. A call dropped at the wrong bound is a wrong answer, and this
is the cheapest fix in the queue.

**Closes:** C2, C3.

### E3 — the settings surface (U1–U6, U10, U7)

In the order the plan already sets: the confirmation on the board, the
revert on a written knob, the filter's no-results card and the
three-part item route, blur validation, refusals that name their door
(the env, ceiling and vault derivations each carrying their next step,
C8), and the money section's removal (seven sections, no cap row on the
face, `fleet settings` still writing the three). Then re-measure U10's
onboarding ladder and land U7's vocabulary passage. The face harness
pins each; a Chromium probe quotes the confirmation, the no-results card
and the filter's keystroke path.

**Closes:** C4, C5, C6, C7, C8, C9, C10.

### E4 — the evidence residuals (tool-surface, conversation-control, gui-revamp C1/C13)

The resolution record gains argument sources and a denial pin; a
30-call fold fixture keeps every item and artifact path; the elision stub
names the tool and the artifact; `structuredContent` is validated
against an `outputSchema` or the absence of one is recorded. The five
conversation-control pins and gui-revamp's real-run SSE test and
per-hunk accept land with them.

**Closes:** C11, C12, C13, C14.

### E5 — the small closes (ki-triage, docs-speak, skills, gui-revamp C0/C6, gui-revamp C9/C11/C12)

Runs and restatements: a recorded `cargo test --all-targets`; 80/120
column clipping assertions; the five-run flake note; `arch-gate` and
`docs-gate`; the exact-three-files proxy restated against the real
counts; the skills route name aligned and the retire control placed;
gui-revamp's superseded token wording retired with `919416a` cited; boot
autofocus; the inline-card question settled; the rules surface decided.

**Closes:** C15, C16, C17.

### E6 — the close-out tail (router leg, sys1 leg, beta B5, the checker)

The router's composed leg (C18) needs a live run where the real sys1
returns a usable `composed` verdict — that is calibration work, not a
test; if it cannot be reached, the leg is ruled unreachable and the
plan's header says so. The sys1 extraction leg (C19) gets its status
pass first, then whatever survives. The beta gate (C20) closes on the
owner's own pass three. The checker (C21) lands with them so the next
sweep is a command — and the queue's own closing rule (C22) is the last
step: every residual row ticked in its owner's status pass or dropped
with its reason and ruling cited.

**Closes:** C18, C19, C20, C21, C22.

## The residuals, row by row

Folded from the superseded residuals plan; each row is the plan that
owns its status-pass table.

### tool-surface-and-lane-resolution (15)

- **C0** — no plan-mode prompt (only the refusal string); gui.md does
  not state that `bash` stays allowed for reading in plan mode; the
  media-type sniff has no direct test.
- **C1** — no argument sources on the resolution record; no denial pin.
- **C2** — truncation and redundancy print as counts, not rates.
- **C3** — no 4,000-line offset test, no empty-file test, and the
  total-lines scan reads to EOF.
- **C4** — no `replace_all` test, no capped-diff test.
- **C7** — no 30-call forced-fold fixture.
- **C8** — no `valid: …` candidate list in the refusal; tools are not
  one module each.
- **C9** — bounds are 36/10,000 against the plan's 28/8,000; no
  byte-identical-declaration test.
- **C10** — the reason says only "activated by tool_search"; no sem1
  scores, no fallback test.
- **C11** — `prose` starts read/write/glob not web; no deep-context or
  low-confidence test.
- **C12** — no failover retry; the chosen reason is dropped.
- **C13** — no model/catalog enum; no no-model-request test.
- **C14** — the elision stub names neither tool nor artifact.
- **C15** — no `outputSchema` validation; `readOnlyHint` unread.
- **C16** — no deferred count on the tools card; no recorded gate.

### conversation-control (5)

- **C3** — drag-reorder and the finished/failed flush unpinned; HTML5
  DnD rather than pointer events.
- **C4** — the forced fold is driven from options, not `Control::Fold`.
- **C5** — an unusable pin is swallowed silently; no integration pin.
- **C6** — no test drives the loop's pause wait.
- **C8** — no face test on a live-turn row.

### gui-revamp (10)

- **C0/C6** — superseded by `919416a` (token removed); restate.
- **C1** — no real-run SSE test.
- **C2** — a refused router document replaces the last good snapshot.
- **C7** — no recorded four-leg gate at the landing.
- **C8** — only the G5 opening is on record.
- **C9** — no boot autofocus; the onboarding door hides the composer.
- **C11** — decisions/recalls/loads fold into one line, not inline cards.
- **C12** — the active rules live in the trust card, not the header.
- **C13** — per-hunk reject only, no accept.

### ki-triage (3)

- **C2** — no recorded `--all-targets` green run.
- **C4** — no 80/120-column clipping assertions.
- **C5** — no five-consecutive-run flake artifact.

### docs-speak (3)

- **C2/C3** — `arch-gate` and `docs-gate` not run.
- **C4** — the exact-three-files proxy no longer holds.

### skills (3)

- **C2** — route is `POST /api/skill/retire`, not `/api/skills/retire`.
- **C3** — retire is not beside the never-selected mark.
- **C4** — no live-run log evidence.

## Acceptance criteria

- [x] **C1 — *(2026-11-02: the owner's amendment recorded in `relay/relay.py`, the open-calls row and KI-399 (closed))*  O1 is settled and closed.** The join validates against the
      minted-session store as ruled, or the owner amends the ruling and
      the code, the test, the open-calls O1 row and KI-399 all carry the
      same answer.
- [x] **C2 — *(2026-11-02: `calib::embed_tail` + the window gate (c1fa009) and its fixture pin)*  the embed window gate runs and says what it found.**
      `calib::embed_tail` returns count, median, p95 and max per kind
      over real day files, pinned by a fixture test, and the pass states
      the window's shortfall by name when it is not met.
- [x] **C3 — *(2026-11-02: `calib::embed_bound_ms` + both constants at 27,321 ms, each comment naming window/rows/tail; KI-249 closed with the same numbers)*  the timeouts carry their measurement.** PROBE_TIMEOUT and
      EMBED_TIMEOUT equal the rule's output over the measured tail (or
      are unchanged with a measurement-confirmed comment), each constant
      names the window, row count and tail, and KI-249 closes with the
      same numbers.
- [x] **C4 — *(2026-11-02: the board status line; SET/U1 pins the three stubbed posts and the unchanged stream)*  the settings save answers on the board.** A save shows
      `saved <label>` in the surface within the paint cycle, a refused
      write shows its reason there, a dead fetch shows the unreachable
      line, and nothing lands in the conversation stream; harness pins
      the three stubbed posts.
- [x] **C5 — *(2026-11-02: `unset` + `compiled_default`; the card's revert; U2 pin asserts the `unset` body)*  a written knob reverts in one press.** The read carries
      each scalar's default, the card's revert flips the row's mark, and
      a destructive list act names its blast radius and survival.
- [x] **C6 — *(2026-11-02: the no-match card, the three-part route, arrows/Enter; U3 pins the card, the press marks the row, the deep link)*  the filter finishes.** No match shows the card with
      suggestions; a match opens the item's card through the three-part
      route; a bare deep link does the same; arrows and Enter drive the
      list.
- [x] **C7 — *(2026-11-02: blur validation beside the field with the save held; U4 pins both refusals and the missing POST)*  the fields validate before the round trip.** A numeric
      knob refuses "abc" and a value past the ceiling at blur, beside
      the field, and the server's refusal still lands.
- [x] **C8 — *(2026-11-02: the env/ceiling/vault next steps; U5 pins the three lines)*  every refusal names its door.** The env, ceiling and vault
      derivations each carry their next step, pinned by harness
      assertions on the three lines.
- [x] **C9 — *(2026-11-02: the section removed; the harness pins seven rails and no cap row)*  the money section is gone.** Seven sections on the rail,
      no cap row anywhere on the face, `fleet settings` still writing
      the three, and a sweep for spend text on the face.
- [x] **C10 — *(2026-11-02: the door stays up until the `home` step is done and the ladder renders done/now/next; re-measured on a fresh home)*  U10 and U7 are closed.** A fresh home shows the
      three-step ladder with its unlock path (re-measured on a scratch
      home), and the vocabulary mapping passage exists with both
      surfaces' doc sections citing it.
- [x] **C11 — *(2026-11-02: `argument_sources` on the record; the three-call and denied-call pins)*  the resolution record is complete.** Every tool call
      carries requested, resolved, method, reason, catalog, tier and
      argument sources, and a denial's record is pinned.
- [x] **C12 — *(2026-11-02: the elision stub names the tool and the artifact; the 30-call fixture reads the artifact back)*  a fold keeps its artifacts.** A 30-call forced fold
      retains every todo item and every artifact path, and an elided
      result names its tool and the artifact that holds it.
- [x] **C13 — MCP results are typed.** *(2026-11-02: `outputSchema` parsed on `tools/list`, `structuredContent` validated (root type, `required`, property types) with a named note on mismatch; `readOnlyHint` read and selecting the read tier, `destructiveHint` the destructive one, the stricter winning; the tier recorded on the `tool_screen` row. Four pins; `docs/features/mcp.md` carries the passage.)*  `structuredContent` is validated
      against an `outputSchema` where one is declared, `readOnlyHint` is
      read, and the absence of a schema is recorded rather than assumed.
- [x] **C14 — the conversation-control pins land.** *(2026-11-02: the drag reorder and the failed→drain, the control-driven fold, the pause wait with a stop inside it, and the live turn's rewind returning at the run's end — two harness cases and two Rust tests.)*  Drag-reorder (or a
      stated reason it cannot be pinned), the control-driven fold, the
      unusable pin named, the pause wait driven by a test, and the
      live-turn rewind/stop face test.
- [ ] **C15 — the gui-revamp residuals are closed or restated.**
      Real-run SSE test, refused-snapshot retention, recorded gate,
      boot autofocus, per-hunk accept, the inline-card and header-rule
      questions settled, and C0/C6 restated with `919416a` cited.
- [x] **C16 — *(2026-11-02: `--all-targets` 1,751/0, five integration runs 163/0, the 80/120 column pin, `arch-gate` PASS and `docs-gate` exit 0)*  the evidence runs are recorded.** `cargo test
      --all-targets` green, 80/120-column clipping assertions,
      `arch-gate` and `docs-gate` exit 0, and the flake note's five-run
      artifact — each named in the plan it closes.
- [x] **C17 — *(2026-11-02: skills C2-C4 restated to the code's route and placement; docs C4 restated)*  the skills and docs restatements land.** The skills route
      name matches the code and the retire control sits beside the
      never-selected mark; docs-speak's exact-three-files proxy is
      restated against the real counts.
- [ ] **C18 — the router's composed leg is settled.** Either a live run
      shows the real sys1 returning a usable `composed` verdict (the
      route record quoted), or the leg is ruled unreachable and the
      plan's header says so.
- [x] **C19 — *(2026-11-02: the sys1 leg's status pass written; the plan is cancelled with its one live item named here)*  the sys1 extraction leg reads as state.** Its status pass
      exists, and everything it leaves open is folded here or closed.
- [ ] **C20 — the beta gate closes on evidence.** The shakeout matrix is
      whole, no crasher is open, and pass three's summary is in a recap.
- [x] **C21 — *(2026-11-02: `scripts/plan_check.py` lists completed plans with unticked criteria; run on this tree)*  the checker exists.** A read-only command lists every
      completed plan whose acceptance criteria still carry `- [ ]`
      boxes, and KI-398 closes with it cited.
- [ ] **C22 — the queue is empty by its own rule.** Every row of the
      residuals table above is ticked in its owner's status pass, or
      carries a one-line reason it was dropped, with the ruling cited.

## Files to be touched

`relay/relay.py` + `tests/integration.rs` (O1), `src/calib.rs` +
`src/net.rs`/`src/agent.rs` (the bounds and their comments),
`src/gui/assets/{app.js,index.html,app.css}` + `src/gui/bridge.rs` +
`src/settings.rs` (the settings increments), `src/agent.rs`,
`src/tools.rs`, `src/compaction.rs`, `src/mcp.rs` (the evidence
residuals), `tests/face/harness.mjs` (every face pin),
`docs/plans/*.md` (each owner's status pass), `docs/known-issues.md`
(KI-249, KI-398, KI-399), `scripts/` (the checker), and the feature docs
each increment's third gate names.

## Out of scope

- **`2026-11-02-session-parity-remainders.md` C4** — the headers row in
  the account card; another session owns it, and the card it rides
  landed 2026-11-02 (settings-ux U9).
- **The two drafts** — `2026-10-23-capability-pitch.md` and
  `2026-11-02-usage-in-calibration-and-stream-anomalies.md` are
  unstarted proposals, not residuals of shipped work.
- **Re-opening any completed plan's scope.** The residuals are the
  unshipped halves of criteria already written.
- **The design canvas review loop's end-to-end ask** — named as a
  known gap outside the plans; it needs a model run, not a code change.

## Sequencing and why

1. **E1 then E2** — a trust divergence and a dropped-call defect both
   outrank polish, and both are small.
2. **E3** — the surface the owner names most often; it also unblocks the
   headers row's home.
3. **E4 then E5** — evidence before restatement: pin what behaves, then
   reword what no longer matches.
4. **E6 last** — the router leg and beta B5 wait on live runs and on the
   owner; the checker lands with them so this sweep is the last audit.

## Verification

The four-leg gate by exit code (fmt, clippy, build, test) on every push
that carries code, the face harness green, and — for the surface
increments — a Chromium probe at 1440×900 quoting the states the
criteria name. Each residual's own evidence is a test name, a
`path:line`, or a restated criterion citing the ruling. The close-out
runs the new checker over `docs/plans/` and expects an empty list.

## Linked artifacts

- `docs/plans/2026-11-01-settings-ux.md` — the status pass that named
  the unticked-box defect
- `docs/plans/2026-11-02-completed-plan-residuals.md` — superseded by
  this plan (created and folded the same day)
- `docs/known-issues.md` — KI-249 (the bound), KI-398 (the bookkeeping
  defect), KI-399 (O1's divergence)
- `docs/plans/README.md` — the rule this plan serves: never reopen a
  completed plan; start a new one
