# The face's icons, the resume close-out, and the navigation audit

**2026-10-29** · the turn controls became icons and the composer gained
the two beside-verbs (shipped live to :3080) · the resume-record plan
closed out — gate green exit 0, KI-365 filed and closed, the plan
completed · the conversation↔sessions friction audited to a draft plan
awaiting the owner's word · the relaunch ruling saved: a restart of a
verified tree is deployment, not development. Ledger checker green:
**365 rows, 31 open**. No commits — the day's work rides the working
tree, the owner's to commit.

## The morning (the earlier session's commits, one line each)

- `402d947` — turn feedback: copy · like · dislike · fork · hook at the
  end of every reply (the word-chip form this session iconified).
- `945a931` — the memories corpus' counters, archives and new facts.
- `39d33ad` — the loaded-suite flake class: one live-embedder gate and
  the relay's lifecycle signal (KI-340, KI-351).
- `cf25a52` — workspace-history: the worker revived, the absence named,
  the visuals true to the LayerFS model.

## The turn controls are icons; the composer gained the two verbs

The owner's ask: the word actions at the end of every turn become
icons, and the composer carries a steering wheel for steer and a
bifurcation for btw. What landed:

- **Five icons on every reply** (copy · like · dislike · fork · hook),
  drawn from one shared path table (`MSG_ICONS` in
  `src/gui/assets/app.js`) so the reply footer and the owner bubble's
  copy cannot drift apart. The words stay on the button as the tooltip
  and the accessible name; the glyph carries the row.
- **The pressed verdict's trace moved with the words**: accent tint +
  `aria-pressed="true"` + the tooltip's past tense ("liked — this reply
  was right"), replacing the word flip the chips used. Copy's "copied"
  trace is the glyph swapping to a check for a breath.
- **The composer's foot gained two icon buttons** between attach and
  plan: the steering wheel prefills `/steer `, the bifurcation
  prefills `/btw `, caret parked after it — the owner types the
  instruction and the slash parser, the only parser (the shared-parsers
  ruling), dispatches it. No second body shape.
- **Verified**: the face harness green — **297 checks across 31
  cases**, including the rewritten turn-controls case (icons + the
  aria-pressed contract) and the new composer-verbs case (markup,
  labels, both prefills). `docs/features/gui.md` updated in the same
  pass (the controls passage and a composer-verbs sentence).

## The resume-record close-out (the 2026-10-28 tree work)

The uncommitted resume work (a conversation continued by the owner is a
resumed session, not a terminated one) got its close-out:

- **The gate green, exit codes read directly**: fmt 0, clippy 0,
  build 0, test 0 (the integration target alone: 149 passed, 3
  ignored). The suite ran once, in the background, after the owner's
  ruling that a relaunch of a verified tree owes no new test run.
- **KI-365 filed and closed** — the resume defect ("a continued
  conversation reads as terminated"): the three seams (summarize read
  only the last line, work_state mapped ended to the terminated words,
  Session::open recorded nothing), the fix (the `resume` record, the
  whole-file fold, the `resumed` word, the replay payload), pinned by
  `a_continued_session_is_resumed_not_ended` and
  `a_resumed_session_is_not_terminated`. The plan's KI-354 reference
  was corrected in the plan itself: KI-354 was taken by the sem1
  llama-server instance between the plan's draft and this close-out.
- **Plan `2026-10-28-resume-record.md` → completed**, all three
  criteria checked with their proofs.

## The navigation audit (draft, awaiting the owner)

The owner's friction report — conversation and sessions disconnected,
no jumping between active sessions, the main conversation page
one-and-done, and the work rail polluted by non-user-initiated
sessions — audited to code and archive evidence:

- `#/chat` and `#/sessions` render one shared stage; the home exists
  only as the no-conversation state and nothing returns to it (New
  mints a shell log as a side effect — 34 in the archive).
- The rail's count badges are dead structure: the spans exist, nothing
  fills them.
- The archive census: 538 logs — 362 scheduled + 41 subagent + 135
  kind-less (34 shells, 101 with text); the rail's newest-200 slice is
  85% system-initiated; the needs/working/archived groups filter by
  state, not class, so system runs leak onto the user screen. All 362
  scheduled logs carry the kind stamp — the classification is
  complete; the render is not.
- **Draft plan `2026-10-29-conversation-sessions-home.md`**: N0 the
  rail renders the owner's work only (class-yours, the background line
  and its toggle gone); N1 the home always reachable; N2 New mints
  nothing until words exist; N3 the badges come alive. Four harness
  checks designed. Named follow-up: the ~59 harness/probe sessions in
  the archive need their own stamping decision (a `channel` mark on
  verification traffic), deliberately out of scope.

## The relaunch ruling (the owner's, saved to memory)

"if you finish the work and complete the tests but forgot to relaunch
the app, relaunching it shouldn't require a new test run" —
verification attaches to the code state, not the deploy act. Sequenced:
build → test → relaunch, each once; re-verify only when the tree
changed since the verification; never bundle another slice's
verification debt into a deploy step.

## Files changed (uncommitted)

- Face: `src/gui/assets/app.js` (the icon table and builders, the
  footer rewrite, `paintFeedback`'s state contract, the composer
  prefill wiring), `src/gui/assets/index.html` (the two composer
  buttons), `src/gui/assets/app.css` (the `.msg-icon` rules).
- Harness: `tests/face/harness.mjs` (the two updated verdict checks,
  the composer-verbs case, 297 green).
- Docs: `docs/features/gui.md` (the icons, the composer verbs),
  `docs/known-issues.md` (KI-365),
  `docs/plans/2026-10-28-resume-record.md` (completed),
  `docs/plans/2026-10-29-conversation-sessions-home.md` (draft),
  `docs/recaps/` (this file).
- The resume work itself (from the 2026-10-28 session, closed out
  here): `src/session.rs`, `src/gui/bridge.rs`,
  `src/gui/assets/app.js`, `src/session_recall.rs`, `src/tui/app.rs`,
  `src/tui/ui/mod.rs`.
- Outside the repo: the memory `relaunch-is-not-a-code-event`.

## Doc updates — applied

- `docs/features/gui.md` — the turn controls read as icons with the
  pressed verdict in accent + `aria-pressed` + the past-tense tooltip;
  a sentence names the composer's two verb icons and the prefill
  contract.
- **`FUNCTIONAL-SPECIFICATIONS.md` — the `resumed` word, applied on the
  owner's word.** The original proposal named the stop-reason table;
  on the write, the word turned out to belong to the state vocabulary,
  not the end-reason table (`resumed` is a derived state, never a stop
  reason). Two passages: the work-list state words gained *resumed*,
  and the run's-end bullet gained the resume sentence — a conversation
  continued after its last end carries a `resume` record and reads
  `resumed`: open, never terminated.
- `TECHNICAL-DOCUMENTATION.md` — no update needed: the face-level
  additions are covered by the gui feature doc, and the routing summary
  still holds.

## Open questions / next steps

- **The navigation plan awaits the word** — `2026-10-29-conversation-
  sessions-home.md` is draft; the owner's four frictions are designed,
  not yet built.
- **The FS `resumed` row** — apply on approval.
- **Commits** — the day's work is uncommitted; the owner's. The
  resume slice, the icon slice, and the plan/ledger/docs could land as
  separate commits with explicit paths.
- **The archive's verification traffic** (~59 harness/probe sessions)
  needs its own stamping decision before the navigation plan's N0
  hides nothing it shouldn't.
- **The working tree's strays** (named, not touched, per the daily
  maintenance's rule): `home.txt`, `rr.json`, the `.lock` sidecars,
  `src/tui/ui/mod.rs.bak-resume`, `docs/architecture/system.drawio` +
  `.svg`, `services/browser/generated/`. The owner decides keep,
  commit, or retire.

## Notes

- The gui was rebuilt and restarted (PID 96252, same launch shape:
  `fleet gui --no-open --remote`) — the live face on :3080 serves the
  icons and the resume words. No run was in flight at the restart
  (last log write 09:36).
- Cache sweep at close: `fleet: debug 10.4 GiB → 10.4 GiB; pruned 0
  files older than 7d; free space 80.6 GiB (release targets
  untouched)` — nothing stale qualified: the debug tree is all
  today's builds, the release target is never touched, and the daily
  LaunchAgent keeps the bound.
- No process registry on this machine — fleet replaced it (the
  session's processes: the gui restart, intentional and running).
- The harness's fake DOM asserts static markup against the page's own
  text (the ids-only page build) — the composer-verbs check reads the
  HTML source, the way the CSS checks read their rules.
- The full cargo suite takes minutes cold; run it as a background task
  when a turn continues past it — the owner cancelled the first
  foreground attempt watching it sit.

## The guided onboarding (added the same day, after the icons)

The plan `2026-10-29-guided-onboarding.md` is **completed**. The first-run
surface is two states of the home, decided by one derived read
(`/api/onboard`):

- **The door** — when no candidate resolves, the welcome becomes its
  first-run state: `fleet · first run`, "One account, and fleet can
  think.", the benefit sentence, and one decision. It looks before it
  asks (an account already on the machine is a single press), and the
  press proves itself.
- **The cards** — the capabilities the machine can still gain, each with
  its benefit, a **visible proof** (a derived reading — "6 of 6
  specialists are installed", "30 repositories under …", "8 choices in
  force · quick … · hard …"), one action, and a real "not now".
- **The strategy preview** — the roster step ends in a screen, not a
  jump: the chain as built with each candidate's state and billing, the
  last resort named, what a quick task and a hard one do, the ceilings in
  force, and the window fallback. Then one confirmation.
- **The sequence** — `fleet onboard` walks the same steps, resuming at
  the first incomplete one; decisions ride the calibration stream's
  existing `lifecycle` rows, so nothing new is stored.

### Proven by hand (the plan's deferred check)

The door's press, run against the live face on this machine:

```
POST /api/onboard {"op":"prove"}
→ proved: true, answer: "Hello there, how are you?"
```

One bounded call through the router, the model's own words back — the
criterion the plan deferred to a hand proof, recorded here.

### The audit's finding

Two criteria were marked met before the behaviour existed: the cards had
no visible proof element, and the strategy step's action still jumped to
settings. Both landed the same day, with harness checks of their own
(`OB every card carries a visible proof`, `PV the strategy step shows
its preview`), and the plan's record carries the correction rather than
a quiet green.

### State

Harness 375 checks across 41 cases; the gate green (fmt 0, clippy 0,
build 0, test 0). The live document on :3080 derives every figure from
this machine: 6/6 specialists installed, 30 repositories, quick
`token-plan/qwen3.8-flash`, hard `zai-coding-plan/GLM-5.3-Flash`, the
`deepseek` last resort, three ceilings, 35 declared windows.
