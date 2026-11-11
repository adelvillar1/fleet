---
name: owner-inbox-plan-drafted
description: "project 2026-11-07 — the owner's inbox IMPLEMENTED 11/11 on branch `owner-inbox` (own worktree, 39d083e) and MERGED to master by the owner-approved fast-forward (d7a5ea9..39d083e, pushed); one rail queue directly below the conversation, five classes — seats answerable IN PLACE, the onboarding ladder, huddle asks (read from the huddles derivation's own awaiting_you/preview, never a second scan), cron last_outcome, doctor findings (home + api only; credentials ride the ladder) — badge via paintRailCounts' inb…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked 2026-11-07 for "an inbox for user's pending actions and
approvals, sort of a queue of what the user needs to do", naming onboarding
actions, then steered twice mid-draft: **"also questions to the user coming
from the different huddles"** and **"lets include the cron failures and
doctor findings as well"**, plus the placement ruling **"on the rail right
below conversation"**. Plan at `docs/plans/2026-11-07-owner-inbox.md` —
**IMPLEMENTED 11/11, status completed**, branch `owner-inbox` (own worktree
`~/Projects/fleet-owner-inbox`, based on face-plugin-host 16fc679), pushed
`39d083e` to origin as a NEW branch — master and the burn-down's refs
untouched, the merge the owner's call. Delivered to 3080 (new inode,
restart verified, the live inbox reading the owner's real state).

**The merge (owner's "merge and build"):** another pure fast-forward —
origin/master sat exactly at the branch's base (`d7a5ea9`, the burn-down's
release-machinery restore), zero diverging commits; `d7a5ea9..39d083e
owner-inbox -> master` pushed, and the dmg rebuilt at 9.75 MiB with the
inbox verified inside the bundle (`owner-inbox`/`ib-row`/`paintInbox`, 17
hits). The version string is `0.4.0-beta.1` from the burn-down's restore.

**The five classes ride machinery that already exists:**
- **Seats** — read from the attached sessions' own seat state (the bridge
  drives one session at a time; `ApprovalRequest` carries
  call/summary/verdict, NOT a map — the first draft's `seat.get(...)` was
  E0599); answered via the existing cross-session `bridge.answer`.
- **Onboarding** — `/api/onboard`'s ladder, incomplete steps only; the
  real face showed `vault` + `github` as the owner's true pending setup.
- **Huddle asks** — **the build found the huddles read already derives
  `awaiting_you` per room** ("the last owner message and the last `@you`,
  in log order — a reply after the ask clears it") and its `preview` after
  an unanswered ask IS the ask's words. My first draft's `room_ask` log
  scan was a SECOND state machine — deleted before it shipped (D5's own
  law, cited in the plan). The asks collect across the huddles doc's
  active/inactive/ended arrays.
- **Cron** — the store's own `last_outcome` (`Failed` /
  `SkippedNeedsApproval`) beside schedule/task/last_run; a skipped job is
  the owner's allowlist decision and rows beside a failure.
- **Doctor** — home + api only (the cheap evaluates); **the credentials
  ride the onboarding ladder** — one absence, one row (a vault-only key
  would false-positive the bridge's env+dotenv resolution; named in the
  recap).

**The name collision that mattered:** an `inbox(&self, bound: usize)` read
ALREADY EXISTED — the **bot face's** own session-queue read
(`inbox_document`, the KI-372 class-yours filter, the phone's queue, served
at `/api/inbox` bound 50). The owner's browser-face queue is a different
thing for a different face: renamed `owner_inbox`, route
`/api/owner-inbox`, the bot's read untouched. Grep for an existing name
before claiming it.

**The two-checkout hazard (twice in one increment):** the shell's cwd
RESETS to the main checkout between Bash commands, so a scripted edit with
a relative path (`p = 'src/gui/assets/app.js'`) silently lands in
**face-plugin-host's** copy while the work builds in
`~/Projects/fleet-owner-inbox`. Two edits landed wrong (the ROUTES entry
and the BUILT list) — one partially reverting the other — and the symptom
was precise: the served shell lacked ROUTES.inbox, so `#/inbox` rendered
the palette's "later" note while the API returned a full document. See
[[two-checkouts-edits-land-in-the-wrong-checkout]].

**Face details that bit:**
- `paintRailCounts`' new inbox arm declared `var waiting`, **shadowing the
  function-scope `waiting`** the sessions span reads — the sessions badge
  went empty whenever the inbox arm ran first. Rename (`ibWaiting`); the
  N3 harness case caught it.
- The badge rides the rail counts' OWN economy (`paintRailCounts`' inbox
  arm over `.surfaces-count[data-count="inbox"]`), not a bespoke element —
  the empty counter is decoration, per the existing rule.
- Three declaration lists carry a surface: ROUTES (the palette/route),
  BUILT (the palette's later/open label), GROUPS (the rail row) — missing
  ROUTES/BUILT reads the surface as "later" even with the panel built, and
  the ICONS destination-count tests (16 → 17) re-point with the mount.
- The mockup's primary buttons failed ink in the dark themes (white on
  the accent, 3.06/2.93) — the wash treatment (accent-lift on
  accent-wash, the count pill's proven pair) fixed all 16 cells to ≥4.5.
- The wire pin exposed a real gap: a seat event for ANOTHER session
  returns before the reducer's seat case, so the cross-session arm
  (seat/seat_cleared/answered) throttles its own inbox read.

**Verification:** gate — fmt 0, clippy 0 (two lints in my own bridge test
fixed: a useless `format!`, a len>0 comparison), release build 0,
`cargo test` **2,143/0**; harness **981 checks / 116 cases** (five IB
cases: the five classes rendered, the seat answered in place on its own
session, the honest empty state, the ask's jump, the wire re-read). Bridge
pin `the_owner_inbox_collects_the_rooms_asks_and_the_cron_failures`.

**The live proof (C8, the owner's real face at 3080):** the real inbox
read total 2 (the vault + github steps — true findings); a scratch
session's `write_file` seat (ask mode, folder set first — a folder-less
chat refuses writes BEFORE the gate and never seats) appeared as a row
with the fail-closed verdict riding it, was **answered in place**
(`once`), the run proceeded, the file landed, the row left (3 → 2); a
scratch room's clerk asked `@you` through its own run — the ask row
carried the room, the asker, the words — and was **answered by the
owner's words in the room**, the row leaving; `#/inbox` re-opened with
the onboarding rows and the badge "2" after a reload. The scratch
sessions were left unarchived (the archive route wants its session
param's own name — left as named artifacts).

Related: [[new-mints-at-the-press-ki-453]] (the detach semantics),
[[huddle-plan-completed-2026-11-04]] (the mention vocabulary's origin),
[[rebuild-is-not-commit-push-or-ship]] (the delivery state named in the
closing line), [[two-sessions-one-repo-master-worktree]] (the branch
discipline this plan ran under).
