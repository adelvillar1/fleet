---
name: bot-pwa-b2-b3-landed-2026-10-26
description: "bot-PWA plan state — B2 (the inbox read) and B3 (new conversation admitted on the remote face) LANDED 2026-10-26 in 7d2c81e while the plan file still said "fully pending"; the archived-drift it named is CLOSED (P4) and the whole plan was implemented 2026-10-28 as 2976098 — this memory survives as the lesson that a landed increment does not update its plan file"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28, asked for the bot-PWA plan's status. The plan's own
"Re-audited 2026-10-26" block and the 10-26 recap both say fully
pending; the tree disagrees.

**Landed** — commit `7d2c81e`, 2026-10-26 21:27, "feat(bot-pwa): B2 the
inbox read, B3 the new conversation allowed through" (ungated by D8's
mockup review by the plan's own terms). B2: `Bridge::inbox`
(src/gui/bridge.rs:1405), `inbox_document` (4511), `GET /api/inbox` on
both faces and in the remote allowlist, `session::tail_lines` (16 KiB,
whole JSONL lines, partial fragment dropped), seat-first stable
partition, a row that yields no line named `refused` rather than blank.
Three tests (`the_inbox_maps_the_archive_words_into_the_bots_four`,
`the_inbox_puts_the_seat_first_and_never_renders_a_blank`,
`the_inbox_answers_the_bots_words_over_http`). B3:
`/api/session/new` in the remote `ALLOWED_POST`, and `/api/steer` was
admitted in the same pass — the 10-26 audit had listed both as missing.

**Not started**: B0's mockups (`docs/plans/mockups/2026-10-20-bot-pwa/`
absent — still the owner's gate before any GUI asset), B1 (the remote
face still serves `/app.css`/`/app.js`; `src/gui/assets/` has no bot
set), B4, B5, B6 (`docs/features/bot-app.md` absent).

**B0 SUPERSEDED 2026-10-28** — the mockup folder now exists and is
built from deep research; the owner corrected that B0 was the session's
artifact to produce, not the owner's to owe. D8's approval line is still
empty, so B1 stays blocked. See
[[bot-pwa-b0-mockups-built-2026-10-28]].

**The drift, and it is the plan's own named risk firing.** `9f877f4`
(the next day's archive verb) added `archived` and `handed back` to
`work_state`. `inbox_document`'s mapping was never updated: it reads
the unfiltered `sessions_read` list and `archived` falls through
`_ => "finished"`, so a paired phone's inbox lists archived sessions as
finished, counting against the 50-row bound with no way to tell them
apart — and no test covers it. The plan's own rule ("if
`sessions_read`'s vocabulary grows again, the bot's four states follow
from it in the same commit") was not honoured. `handed back → finished`
is defensible; `archived → finished` probably is not for a bot inbox.

**The general lesson**: a landed increment does not update its plan
file. Read the tree before quoting a plan's audit block — a same-day
audit can be hours stale. See [[draft-plans-audit-2026-10-25]],
[[ledger-rows-lag-their-fixes]],
[[sessions-do-not-end-on-their-own]].
