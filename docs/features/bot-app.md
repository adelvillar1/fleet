# The bot app

The phone surface: fleet's **assistant-first** PWA, installed from its
own relay origin (`https://fleetbot.opsh.io` — a separate relay service
from the console's, the same seal; see `remote-access.md`). It
is a second shell over the same bridge as the console —
one process, one agent thread, one log — never a second fleet. Plan:
`docs/plans/2026-10-20-bot-pwa.md` (status: completed); the mockups it was
built against live in `docs/plans/mockups/2026-10-20-bot-pwa/`.

## Pairing

The bot pairs exactly as the console does: from `#/bots` on the
loopback face, press the pair action, scan the QR, paired — the same
card, the same one-time window, only the origin behind it differs (the
bot relay origin with its entry fragment; the LAN address when no bot
relay dials). See `remote-access.md` for the origins and the seal.

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
   themes). An **optional, owner-supplied image** may replace the initial
   — `<roles_dir>/<name>.png|jpg|jpeg|webp|svg`, rendered in rows and
   member stacks and never beside a message; absent one, the derived
   initial stands. Renaming a bot is renaming its role file.
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
| Install | the bot relay origin (`https://fleetbot.opsh.io`) installs **fleet bot** (its own manifest, worker, icons); the app lives at `/bot/`, its worker's scope is `/bot/`, and the worker's offline retry page says what is true |

## Two windows, one bridge

The owner's 2026-10-28 ruling: this is a **second window into fleet**,
not a replacement for the work face. Both faces serve from the same
process, sharing one bridge, one agent thread, and one seat:

| window | where | shell |
|---|---|---|
| The work experience | the console face — loopback `:3080`, and the paired-device face on the machine's address `:3080` | `fleet` — the console, the origin the owner's phone PWA was installed from |
| The persona experience | the bot face, its own port (`--bot-port`, default **3081**) | `fleet bot` — the assistant-first shell |

The two ports are two origins, which is what keeps them two installs —
the PWA identity includes the origin (the plan's D3), so the console's
home-screen app and `fleet bot`'s coexist without touching each other.
Each face carries its own shell and assets and refuses the other's by
name (403 by route); the API is shared, because the phone and the desk
are two clients of one conversation. Pairing is home-level: one device
token works on either face.

## The console's huddle

The session machinery this face built — `Start.personas` in join order, a
standing `goal`, `Join`/`Leave` records, `Assistant.persona` bylines — is
what the console's **huddle** is made of. The huddle is the work face's
room over the same bridge: the owner and one or more bots exchanging
messages, addressing each other with `@`, posting deliverables in-line,
breaking a phase out into a child room, and keeping what the room learned
beside what it produced. It does not move here. This face stays
assistant-first — the unit is still the persona, the inbox still groups
conversations by the persona working them, and no huddle rail appears —
while the console's room and this window are two clients of one
conversation. The rulings both answer to are above; the room's own what
and why is [`huddles.md`](huddles.md).

## The routes

The bot face's allowlist admits exactly: the bot shell and its assets
(`/`, `/bot.css`, `/bot.js`, `/bot.webmanifest`, `/bot-sw.js`,
`/bot-icon-*.png`), the installed app's scoped set (`/bot/`,
`/bot/bot.css`, `/bot/bot.js`, `/bot/bot-sw.js` — the same four files by
another path, which is what gives the worker a scope of its own), the
stream, and the API the app uses — including `/api/inbox`,
`/api/session/new` (with an optional `{persona, goal}` body),
`/api/session/join`, `/api/session/leave`, and the four run controls. The
console's assets are no longer served remotely: the phone gets the bot
app, the desk keeps the console.

## The install

The installed app is a real PWA, not an installable page. `bot.html`
registers `/bot/bot-sw.js?v=<build>` at scope `/bot/` (skipped on a
relayed `/s/<sid>/` page, where the origin is shared and the asset paths
carry that prefix). The worker's `VERSION` is the server-substituted
asset hash, so its cache is `fleet-bot-<build>` — never the registration
URL's query, which used to yield the literal `fleet-bot-` when a page
registered without one. `install` pre-caches the two versioned assets, so
the cache exists and is non-empty the moment the worker activates;
`activate` deletes the previous `fleet-bot-*` and nothing else (the
console's `fleet-shell-*` is untouched).

The scope is the point: a client under `/bot/` is matched by the longest
registered scope, so the console's `/sw.js` (scope `/`) can never control
the bot app, and a phone with both apps installed sees the bot's own
retry page — *"the run keeps going without you"* — offline, never the
console's. The face's root (`/`) stays what it was: the pairing entry,
deliberately outside the worker's scope — the installed app's `start_url`
is `/bot/`, and that is the page the worker owns. Measured 2026-11-21 at 390x844 with both workers registered on
one origin: bot page controlled by `/bot/bot-sw.js?v=<hash>`, caches
`fleet-bot-<hash>` (2 entries at install); offline, the bot's page is the
bot's, the console's is the console's; a hash bump reloads clean
(`fleet-bot-h1` → `fleet-bot-h2`, the console's cache intact).

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
its versioned assets under `/bot/`; the manifest's `id`, `start_url` and
`scope` are `/bot/`; the worker's `Service-Worker-Allowed` is `/bot/`, its
cache name and precache URLs are the build's, and its cache boundary names
exactly `/bot/bot.css` and `/bot/bot.js` and bypasses `/api/`, `/events`,
and `/pair`; the allowlist answers 403 by name for settings, vault,
channels, pairing, and the bot's scoped set on the console face; a persona
join lands as a log record and folds into the summary's set; the seat names
its destructive class from the gate. The conversation's layout is one rule
(`.conv` is the flex column its children were already written for) and is
measured at 390x844: `.scroll` scrolls, `.dock` ends at the viewport edge,
and `elementFromPoint` on the composer answers the composer — removing the
rule puts the dock at y=4631 in an 844px `#app`.
