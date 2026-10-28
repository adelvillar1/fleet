# The bot app

The phone surface: fleet's **assistant-first** PWA, installed from the
tailnet origin. It is a second shell over the same bridge as the console —
one process, one agent thread, one log — never a second fleet. Plan:
`docs/plans/2026-10-20-bot-pwa.md` (status: completed); the mockups it was
built against live in `docs/plans/mockups/2026-10-20-bot-pwa/`.

## The rulings it implements

1. **Assistant-centric, not work-centric.** The unit is a persona, not a
   session. The inbox groups conversations by the persona working them;
   the workspace appears once inside a conversation, never in a list row.
2. **The roster** — six personas, three of them domains:
   `scholar` (knowledge work), `presenter` (presentations), `advocate`
   (job hunting), `tower` (runs the work), `auditor` (checks what
   changed), `clerk` (keeps the ledger). Each is a role file under
   `<home>/roles/`, and each names the skills it loads from the canonical
   `~/.agents/skills` folder — a test enforces the resolution.
3. **A persona is a contract, not a mascot.** Its mark is its serif
   initial on an opaque disc tinted from the console's lane palette, with
   a per-theme ink measured to clear AA (5.2:1 or better in all three
   themes). No portraits.
4. **Inter-persona conversations.** A conversation carries a **goal** —
   the owner's own words, shown as chrome above the stream — and a set of
   **personas** in join order, the first being the lead. A persona joins
   through the `+` in the header; the join is a row in the log
   (`join`), because anything the model saw must be reconstructable.
   Adding a persona never changes the goal.
5. **Only destructive acts take the seat.** The rule is binary:
   non-destructive work (reads, searches, appends, edits with a diff)
   runs under the bot's `guarded` mode without asking; destructive work
   — `write_file` (an overwrite discards what is there), `bash` commands
   in the destructive list, `file_change` move/delete — is held every
   time and flagged: a red hairline, a `⚠ destructive` badge, and no
   always-allow. The flag is data (`destructive` on the seat document),
   the gate's own decision, not the client's guess.
6. **The archive is its own word.** An archived conversation reads
   `archived` in the bot inbox, not `finished` — a put-away is not a
   done.

## The surfaces

| surface | what it does |
|---|---|
| Inbox | persona rail with counts; conversations grouped under each persona, rows led by the persona's mark; a compact strip above holding only what needs the owner (destructive seats, failures); the dispatch box |
| Conversation | the participant stack and a `+` in the header; the goal as chrome; per-persona bylines; tool rows; the seat, dressed by its class; the working chip with Stop |
| Dispatch | who takes it, then the goal, then send — the persona is the first choice because it decides tools, skills, and recall |
| Install | the tailnet origin installs **fleet bot** (its own manifest, worker, icons); the worker's offline retry page says what is true |

## The routes

The remote face's allowlist admits exactly: the bot shell and its assets
(`/`, `/bot.css`, `/bot.js`, `/bot.webmanifest`, `/bot-sw.js`,
`/bot-icon-*.png`), the stream, and the API the app uses — including
`/api/inbox`, `/api/session/new` (with an optional `{persona, goal}`
body), `/api/session/join`, `/api/session/leave`, and the four run
controls. The console's assets are no longer served remotely: the phone
gets the bot app, the desk keeps the console.

## The session facts

A session carries `personas` (the start record's set, folded with every
`join`/`leave` record) and a standing `goal` (the goals sidecar, the
console's own seam). Both appear in `sessions_read` rows and in
`/api/inbox`'s rows. Sessions created before these fields existed read as
*unassigned* — no persona is invented for them.

## Verification

The mockups were verified at 430 and 900 px in all three themes (102 page
loads: no overflow, controls ≥44 px, fields ≥16 px, every text element at
WCAG AA). The built face is pinned by tests: the shell names the app and
its versioned assets; the manifest is the manifest type; the worker's
cache boundary names exactly `/bot.css` and `/bot.js` and bypasses
`/api/`, `/events`, and `/pair`; the allowlist answers 403 by name for
settings, vault, channels, and pairing; a persona join lands as a log
record and folds into the summary's set; the seat names its destructive
class from the gate.
