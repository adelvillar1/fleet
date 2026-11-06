---
name: fleet-persona-bots-fit-as-bot-profiles
description: "2026-10-20 owner asked whether OpenMausBot-style persona bots (\"talk to a named agent per contact\") make sense in fleet; research verdict — the interaction model fits and is mostly configuration of an existing run, but it needs a new \"bot profile\" object and a naming split from the Bots (access) surface; 2026-10-20 update — the owner then proposed a separate fleet bot GUI reached over the Tailscale they already run, same backend, so the surface has moved toward fleet's own UI (second shell over one bridge, …"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner question 2026-10-20, right after the bot/channel surface landed:
"would it make sense to enable this type of interaction within fleet?",
pointing at github.com/milind-soni/OpenMausBot (research turn, no plan
drafted, nothing built). OMB's pattern: every sidebar entry is a named
agent with persona, model, tool/app access, its own continuous
conversation; approvals arrive as inline Allow/Deny cards; an unaddressed
message auto-routes to the fitting bot (Jev) with a room lead as
fallback; teams are portable as one secrets-stripped Markdown package;
routines/webhooks wake a bot. Under it: a local harness server, CLI
provider drivers, Composio, per-bot cloud/VMs, Electron/React.

**Verdict: the interaction half fits fleet unusually well; the
implementation stack does not.** Fleet already owns every primitive —
`roles.rs` overlays (today only `subagents.rs` loads them), the router's
roster/pin/prepared-route, `role.tools` + `AgentOptions.scope`, MCP tool
lanes as "apps", the `computer_use` tool with its always-ask ruling,
memory/skills/shapes, the seat (raised once, answered once), sys1 +
`route_cache` for auto-routing, `workspace`, cron, `.md` portability, and
pairing/QR/relay (ahead of OMB). The execution surface is already
`AgentOptions`; a bot is a saved configuration of `identity`,
`role_overlay`, `scope`, `memory`, `skills`, `model`.

What is missing is a **bot object**, not a bot engine. Nothing outside
delegation fills those fields in: the face's run sets `role_overlay:
None`, `scope: None`, the global identity; the channel driver creates a
peer's session with the face's model and knows no persona; the
peer→session map is keyed `channel:id`, so one chat is exactly one
conversation (a channel cannot host a roster); there is one global
routing pin (a per-bot preference needs an override into
`route::prepare`); and the GUI agent thread awaits `run` inside its
serial command loop, so two bots at once queue.

**Two collisions to settle before building:** (1) the word "bot" now
means three things in fleet — `#/bots` = who can reach the face
(devices, peers, service state), OMB's bots = agents you talk to, and
the 2026-10-18 ruling's "bot experience" = a surface attached to all
sessions for triage. Recommended split: **Bots = the agents fleet
runs**, paired devices/peers move to **Access**. (2) Persona must not get
a second home: `SOUL.md` is the per-run identity base ([fleet-harness-voice-soul-slot]);
a bot's persona is a role overlay on top, not a second SOUL.

**Recommended shape:** a bot profile at `<FLEET_HOME>/bots/<name>.md`
(role or inline overlay, routing preference, workspace, tool scope,
skills, memory shapes, channel bindings, display name), one place that
builds `AgentOptions` from it, and with no bots configured behaviour is
byte-for-byte today's. Increment order: (1) bot store + run-as-bot for
channel deliveries (map key becomes peer+bot); (2) explicit selection
first (`/name` prefix, DM per bot, GUI picker), automatic Jev-style
routing only after, since it costs a judgment per message; (3) the roster
on the Bots surface reusing the Settings role editor and session rows;
(4) `fleet bots export/import` as one Markdown bundle with no secrets and
**imported bots start `observe`, routines paused** (OMB's safe default
worth copying verbatim).

**Declined, each by a standing rule or product boundaries:** Electron/
React shell (one Rust binary), CLI provider drivers (native port), the
Composio marketplace and per-bot cloud/VMs (tool lanes + `computer_use`
are the equivalents), hosted relay/voice (fleet pairs and relays itself),
and an MCP control plane (low value solo; OMB excludes approvals too).

**Risks named in the recommendation:** policy must stay in the one
resolver per peer *and* bot (a `bash` bot reachable from Telegram is the
blast radius); auto-routing adds a sys1 judgment per message; a roster of
actively used bots needs per-session concurrency sooner than it looks.

## Follow-up 2026-10-20: the channel is still open, deliberately

The owner's next words: "I still don't know if telegram is the right channel
for this. Maybe something like the buzz.xyz app or something else entirely
different. I don't have any rush we just need to find what feels right." So
**no channel decision exists, and none is wanted yet** — the owner is choosing
the surface before anything is built, and explicitly on feel. Do not treat
Telegram as settled in any plan, and do not push a plan while this is open.

**Buzz (buzz.xyz) is a real candidate, not a toy** (read 2026-10-20):
Block, Inc.'s self-hostable, Apache-2.0, Nostr-based *workspace* where humans
and AI agents share channels. Rust relay (`buzz-core`, `buzz-relay`, Axum
WebSocket + REST, NIP-01/NIP-42), Tauri desktop + Flutter mobile, agents with
their own keypairs joining channels like people, `buzz-cli` designed for LLM
tool calls, a `buzz-acp` harness bridging ACP to MCP (Goose/Codex/Claude Code),
YAML workflows, NIP-34 git events, hash-chained audit. Self-hosting costs
Postgres + Redis + S3/MinIO beside the relay. Status: relay/channels/DMs/
canvases/workflows working; mobile and approval gates still being wired. Its
bet is shared rooms with agents as teammates — a different product thesis than
a personal chat channel, and criticism names a single-relay control model and
coarse channel permissions.

**Three axes the choice actually turns on:** home vs endpoint (where the
transcript/seats/run state live vs a window onto them — fleet's endpoint model
already assumes the split, but today Telegram is both); conversation vs
notification (Telegram is a great one-line-and-reply pipe, poor at runs, diffs,
seats, threads — it forces message-editing to fake a stream); solo vs shared
(Buzz is rooms with several people and agents; Telegram is one owner, one bot).

**Recommendation given (assessment only, nothing decided):** fleet's own face
+ installed PWA is the home — it renders seats, diffs, the live run and the
cross-session inbox, which no chat app can — and Telegram at most the pocket
pipe ("finished", "needs an answer", one-line reply). Spiking Buzz should be
a separate one-day experiment driving `buzz-cli` against a local relay as a
tool; if it feels right it becomes a channel plugin later. Buzz only makes
sense as an *endpoint*, never the home: Buzz's event log would compete with
fleet's JSONL as the source of truth (hard rule 5), repeating the duplicate-
archive mistake the console removed once. Slack/Discord declined for the same
reason — the third-party cloud would hold the work. Matrix is operationally
heavy; iMessage/SMS has no real API.

**The question left with the owner** (do not answer it for them): away from
the desk, converse with fleet or be told and approve? First → PWA push, and
Telegram is the wrong home; second → Telegram is already close and just must
stop pretending to be the conversation; wanting several agents in rooms → a
Buzz spike.

## Resolved 2026-10-20: the surface is fleet's own PWA, not a chat app

**Owner ruling: "I think pwa is the way to go."** The surface question the
previous section left open is closed. Telegram is no longer the candidate
home and Buzz is not being pursued (its rooms-with-agents thesis is a
different product; it stays a possible future channel *endpoint*). The
owner reached this through a separate fleet bot GUI over the Tailscale
they already run — then named the delivery shape as a PWA, which is the
same decision made concrete.

**The shape:** one `fleet gui` process, one `Arc<Bridge>`, one agent
thread, one set of session logs, one seat. The **remote listener's root
becomes the bot-first shell**; loopback keeps the console. A second binary
was refused in the plan because a second process means a second agent
thread and a second seat, and the seat's single-answer guarantee stops
holding. Two PWAs coexist because PWA identity includes the origin: the
desk console on `127.0.0.1:3080` and "fleet bot" on the tailnet origin.

**Plan drafted:** `docs/plans/2026-10-20-bot-pwa.md` (status `draft`,
nothing built). Increments B1 shell split + build stamp, B2 `/api/inbox`,
B3 `POST /api/new`, B4 the app, B5 install/Tailscale, B6 docs. The app is
an **inbox**, not the cockpit: sessions grouped by workspace with
seat / failed / running / finished states, the newest assistant line per
row, tap lands on the seat when one waits, and a dispatch box starts a new
conversation. Its own manifest/worker/icon; a deliberately small client,
not a fork of the ~9,800-line `app.js`.

Three gaps the plan had to add, worth reusing as facts about the surface
as it was: `GET /api/sessions` reads the **archive only** (no attention
state, no last line); a run needs an **existing** session (no "new
conversation" route, so an empty session falls into the launch session);
and the remote allowlist served the **console's** `app.css`/`app.js`.

**Consequence to announce:** the phone's existing install from the remote
origin updates **in place** to "fleet bot" (same origin and id), so the
console is no longer reachable from the phone. Deliberate, but must be
said in the release note.

**Still out of scope / next:** bot profiles (the persona/model/tool layer
above — the file's original recommendation stands as the plan after
this one), push notifications (v1 is pull; Telegram remains an optional
pager), and any Buzz spike.

**Why:** the surface question was the owner's and they answered it; a
later session must not re-open Telegram or Buzz as candidates, and must
not propose a second process or a forked client. **How to apply:** build
the surface as a shell over the shared bridge, never a second agent loop;
personas ride the existing run's `AgentOptions` ([plan-brainstorming-before-development]
for the mockup rule). Related: [[fleet-lane-service-target-architecture]]
(endpoints; "bot experience"), [[fleet-channels-bot-surface-gap]] (the
landed surface), [[fleet-phone-pairing-two-reference-architectures]] (OMB's
trust model, already borrowed), [[fleet-pairing-qr-without-tailscale-serve]]
(the tailnet/HTTPS prerequisite the install depends on),
[[fleet-remote-port-onto-master]].
