---
name: fleet-lane-service-target-architecture
description: "2026-10-18 owner-proposed target architecture for fleet remote access — a channel service (channels as plugins) + the websocket/browser lane + a bot experience in the GUI; ruling: agent core untouched, but the lane service (not presentation) owns peer identity/policy, externally-initiated intake, session multiplexing, and seat arbitration; owner ruling: both the chat and bot experiences must coexist — the core has NO modes, surfaces ATTACH (one session = chat, all sessions = bot), and today's chat path stay…"
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

Owner proposal 2026-10-18, in the phone-pairing exploration
([[fleet-phone-pairing-two-reference-architectures]]): a **channel service where
individual channels are plugins**, plus the **websockets/browser lane**, plus a
**bot-like experience inside the fleet GUI** that interacts with both lanes —
with the hypothesis "if the fleet CLI is built to support both communication
lanes the bot experience is just a presentation layer, correct?"

**My ruling: the hypothesis is half right — the split is agent core / lane
service / presentation, and the lane service is real work, not presentation.**
The agent core stays untouched (the same G1 seam that made the GUI possible: a
new surface is a bridge, not a new agent). But four things sit below the
presentation line:

1. **Peer identity + authorization policy** — a lane endpoint is a named peer
   ("Alejandro's iPhone", "#fleet-alerts"), and the may-this-peer-run /
   approve / see-which-workspaces decision must live in exactly ONE place, the
   discipline `approvals::resolve` already holds (ZCode's per-bot
   allowedUsers/allowedCommands/allowedWorkspaces and OMB's per-device
   capabilities are the same rule twice). Surfaces render policy; they never
   decide it.
2. **Externally-initiated intake** — every current fleet surface is
   owner-initiated (`fleet run`, the composer, the TUI picker); a Telegram DM
   arrives with no owner action. This is the security boundary that matters
   most: an internet message can cause a shell command to run, gated only by
   the seat. Pairing exists to make the message's author provably the owner.
3. **Session multiplexing** — the GUI bridge is ONE anchored session per launch
   (`Command::Resume` re-anchors, it does not add; the TUI picker switches the
   same way). N lanes × M peers needs N concurrent sessions, each with its own
   anchor, transcript, and seat. ZCode does exactly this: one bridge per remote
   session, opened per tap (`workspace-bridge-open`).
4. **Seat arbitration** — with TUI + GUI + phone + a lane peer all able to
   answer, "first answer wins" needs a rule and the losers need an
   "answered elsewhere" state. Core logic, not rendering.

**The unifying abstraction that makes the presentation claim true: a session has
ENDPOINTS, and an endpoint is a (transport, peer identity) pair.** The TUI, the
GUI browser tab, the phone web app over the relay, and a Telegram DM are all
endpoints of the same session. The GUI bot panel is then the view over ALL
endpoints of ALL sessions — an inbox with lanes — and ZCode's "bots panel" and
OMB's "channels + companion devices" are the same idea presented twice.

**One correction to "channels are plugins":** the existing `src/lanes.rs`
Python MCP cores are TOOL lanes — a spawned `python3` child speaking stdio
JSON-RPC, contributing tools to a run, under the KI-031 environment discipline
with a health probe behind `fleet lanes`. A channel plugin is a different
animal: a long-lived CONNECTION with reconnect, pairing state, and inbound
event delivery. Same "everything is a plugin" principle, different lifecycle —
so it is a sibling registry beside lanes.rs, NOT a new entry in the existing
one. Conflating them would drag pairing secrets and reconnect loops into a
model built for stateless tool servers.

**The symmetry to build toward:** the GUI bot panel and the phone web app are
the same kind of thing — two clients of ONE lane service, differing only in
trust (loopback owner token vs paired device token with default-deny routes,
per OMB's companion model). Neither holds agent logic.

**Owner ruling, immediately after: BOTH experiences must exist — the normal
chat experience as it is today AND the bot experience — so the core must have
NO MODES.** The distinction cannot live in the core or in a switch the owner
flips; it lives in what each surface ATTACHES to:

- **Chat** = a surface attached to exactly ONE session, owning its seat and
  driving it. Every invariant of today's bridge survives (one anchor per
  surface, the prepared-route lifecycle on rewind, the seat's single owner),
  because from that surface's point of view nothing changed.
- **Bot** = a surface attached to ALL sessions at once — triage, raise seats,
  dispatch. It never OWNS a seat; it only raises one for an attached surface or
  lane peer to answer.

**The non-negotiable that protects the working product:** with no channels
configured and no paired devices, the lane service and relay are simply absent
and `fleet gui`/`fleet chat` are byte-for-byte what they are today. The bot
experience is opt-in subsystems bolted BESIDE the chat path, never under it —
which also keeps the crate contract (pairing, device tokens, and the mobile UI
add zero crates).

**Where each experience lives, grounded in the GUI's real structure:** the rail
is built from ONE `GROUPS` declaration (four groups, one level beneath each),
so the bot experience is one entry in the `work` group beside `chat` and
`sessions` — and tapping a lane-originated session there ATTACHES THE EXISTING
CHAT SURFACE to it (same composer, re-attached; not a second app). The TUI's
chat is unchanged and its existing session picker just gains a lane/peer
column. The phone web app is bot-first (sessions grouped by project, tap into
chat). The seat is the proof that "presentation" is the right word for the
experiences: it is raised once and answered once at the core, renders as the
in-conversation card in chat and as a badge + push in bot, and neither
experience knows the other exists. A lane-originated session also gets its own
prompt line through the existing `GUI_PROMPT` per-face seam — one more value,
no new machinery.

**Addendum 2026-10-20 — "bot" has acquired a third sense.** The landed `#/bots`
surface ([[fleet-channels-bot-surface-gap]]) uses "bot" for *who can reach the
face*, and the owner's OpenMausBot question raised a third: a *named persona
agent you talk to* ([[fleet-persona-bots-fit-as-bot-profiles]]). This ruling's
"bot = a surface attached to all sessions" is a second sense still. Recommended
split recorded in the persona memory: Bots = the agents fleet runs; Access =
paired devices and peers; this ruling's sense is a triage view, not a roster.

**Why:** the owner named this shape deliberately after seeing both reference
architectures; it is the target the eventual plan should implement, and it
supersedes picking either reference alone.

**How to apply:** the plan now exists and its increment order supersedes the
order sketched here — see [[fleet-remote-access-plan-drafted]] (R0 workspace
field on the log → R1 pairing + remote listener → R2 session registry → R3
phone UI + inbox → R4 channel service → R5 seat arbitration → R6 GUI as lane
client → R7 relay → R8 second plugin). Two rules from this ruling that the
plan keeps: never put peer policy in a surface — it belongs beside
`approvals::resolve` — and the channel registry is a *sibling* to lanes.rs,
never an entry in it. Related:
[[gui-revamp-browser-face-decision]] (the face this extends),
[[fleet-phone-pairing-two-reference-architectures]] (the references and the
gaps), [[fleet-browser-lane-architecture]] (the existing lane precedent).
