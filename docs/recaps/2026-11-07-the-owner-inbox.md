# Session recap — 2026-11-07: the owner's inbox

**Summary.** The owner's ask — an inbox for the user's pending actions
and approvals, a queue of what the owner needs to do, widened twice by
steer (huddle questions; cron failures and doctor findings; the rail
placing it directly below the conversation) — landed whole:
`docs/plans/2026-11-07-owner-inbox.md`, **11 of 11 criteria ticked**, on
its own branch (`owner-inbox`, worktree
`~/Projects/fleet-owner-inbox`), the KI burn-down's worktrees untouched.

## What was built

- **The mockup first** (C0): `docs/plans/mockups/2026-11-07-owner-inbox/`
  — 16 cells (2 states × 2 widths × 4 themes), every token hand-copied,
  the `ib-` family prefixed. Measured: the eye passed what the
  measurement caught — the **primary row buttons failed ink in the dark
  themes** (white on the accent, 3.06/2.93) — fixed with the house's own
  wash treatment (accent-lift on the wash, the count pill's proven pair),
  then 16/16 clean, worst pair 5.39:1.
- **The read** (C1): `GET /api/owner-inbox` — one bounded document, five
  classes: the pending seats (read from the attached sessions' own seat
  state, the verdict riding the row), the onboarding ladder's incomplete
  steps, the huddle rooms' unanswered questions, the cron jobs whose
  `last_outcome` is `Failed` or `SkippedNeedsApproval` (the store's own
  record, read), and the doctor's failing checks (the home and the API;
  the credentials ride the onboarding ladder — one absence, one row).
- **The discovery that shaped it**: an `inbox(bound)` read **already
  existed** — the bot face's own session-list read (KI-372's class-yours
  filter, the phone's queue). The owner's queue is a different thing for
  a different face: renamed to `owner_inbox`, its own route
  (`/api/owner-inbox`), the bot's read untouched.
- **The asks read the huddles derivation, never a second state machine**
  (C4): the plan's first draft scanned the rooms' logs for
  `Mention { member: MENTION_OWNER }` — the build found the huddles read
  already derives `awaiting_you` per room ("the last owner message and
  the last `@you`, in log order — a reply after the ask clears it"), and
  its `preview` after an unanswered ask IS the ask's words. The duplicate
  scan was deleted before it shipped; the bridge test pins the huddles
  derivation's own output.
- **The surface** (C6): `#/inbox` on the rail directly below the
  conversation, the badge riding the row through the rail counts' own
  economy (`paintRailCounts`' inbox arm) — one fill, so the badge and
  the page cannot disagree. The rows act in place: the seat's
  once/always/deny through the existing cross-session answerer, the
  step's own control, the ask's jump into the room, the job's settings
  door.
- **The wire** (C7): a seat arriving on the wire re-reads the queue —
  the seat case fires the read for the showing conversation, and the
  cross-session arm (seat/seat_cleared/answered) throttles one read for
  every other session's. The pin caught the gap: the first draft only
  listened on the own-session path.

## The verification

- **Gate on the branch**: fmt 0, clippy 0 (two lints in my own test
  fixed — a useless `format!`, a length-to-zero comparison), release
  build 0, `cargo test` **2,143 passed / 0 failed**.
- **The harness:** **981 checks / 116 cases** — the five IB cases pin
  the five classes rendered, the seat answered in place on its own
  session, the honest empty state, the ask's jump to its room, and the
  wire re-read.
- **The bridge pin:** `the_owner_inbox_collects_the_rooms_asks_and_the_cron_failures`
  — the ask's room/asker/words, the failed job as the row (the completed
  job never), the fresh home's onboarding ladder, and the total being
  the classes' own sum.
- **The live proof** (C8, the owner's real face at 3080): the real
  inbox read **total 2** — the vault step and the github step, both true
  findings on the owner's home. A scratch session's `write_file` seat
  (ask mode, the verdict "the judgment lane could not answer — the
  standing destructive class decides") appeared as a row and **answered
  in place** — once — the run proceeded, the file landed, the row left
  (3 → 2). A scratch room's clerk asked `@you` through its own run — the
  ask row carried the room, the asker, and the words — and **answered by
  the owner's words in the room**, the row leaving (3 → 2). The reload
  leg: `#/inbox` re-opened with the onboarding rows and the badge "2"
  without a click.

## Found on the way

- **The drive hit 100% again mid-gate** — `~/.fleet/checkpoints` at
  **69 G**; the owner-authorized prune re-ran (821 turn dirs to the last
  20 per session, **55 GiB freed**). The retention leg in
  daily-maintenance remains the real fix, third occurrence recorded.
- **The two-checkout hazard bit twice**: the shared checkout and the
  worktree share relative paths, and two edits landed in the main
  checkout's `app.js` instead of the worktree's (the ROUTES entry, the
  BUILT list) — one silently reverting another. The discipline that
  caught it: grep the worktree's file after every edit, revert the
  stray, and the harness's failures pointed straight at the drift.
- **The palette's `later` and the icons' count** are declaration-driven
  (ROUTES/BUILT) — the inbox's mount was three list entries plus one
  glyph, and the tests that count destinations re-pointed to seventeen.

## Where it stands, honestly

- The work is on **`owner-inbox`** (origin, pushed through the docs
  commit) — `master` and the burn-down's refs untouched; the merge is
  the owner's call.
- The face at 3080 runs the branch binary — the inbox is live, badge
  and all.
- The taxonomy's edges, named: a vault-only key reads in the doctor's
  credentials as absent (the bridge's check resolves env and the home's
  .env; the vault's unlock is the CLI's session) — the onboarding
  ladder's own step covers it; a cron job skipped for needing an
  approval rows beside a failed one, because both are the owner's
  decision.
