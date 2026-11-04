---
name: zcode-remote-session-bot-gateway-architecture
description: "ZCode's phone pairing and remote access architecture — Web Remote Control via outbound WebSocket relay + QR code for mobile Safari, and bot gateways."
type: reference
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

ZCode provides two remote access models that both avoid opening inbound router ports or configuring static public IPs.

### 1. Web Remote Control (Mobile Safari H5 Interface)
The primary mobile feature (`webRemoteControl` in ZCode Desktop's `app.asar`):
- **Outbound Relay**: Desktop connects outbound via WebSocket to `relayWsUrl` (e.g. `wss://zcode.z.ai/ws`).
- **QR Pairing**: Desktop generates a session ID (`deviceSid`) and a password hash (`passHash` from `/dev/urandom`), rendering a QR code pointing to:
  `https://zcode.z.ai/remote/v4?sid=<deviceSid>&hash=<passHash>&t=<timestamp>&name=<laptopName>`
- **Mobile Safari Flow**:
  1. User scans the QR code with their iPhone camera; Safari opens the URL.
  2. Safari connects to the same relay WebSocket, presenting `sid` and `hash`. The relay pairs the two connections.
  3. Safari sends `bootstrap-request`. Desktop returns `workspaces` (open projects) and `tasks` (all sessions grouped by project).
  4. Safari renders projects on top with sessions grouped underneath. Tapping a session sends `workspace-bridge-open`.
  5. Desktop attaches the workspace host and RPC bridge, streaming conversation turns, tool calls, and approval seats (`approve`/`deny`).

### 2. Bot Gateways (Messaging Apps)
ZCode also supports messaging bots (`["telegram", "webhook", "feishu", "lark", "weixin", "discord", "wecom"]`):
- Uses outbound long-polling (`getUpdates`) or outbound WebSockets (`WSClient`).
- Paired via OAuth Device Authorization Grant (RFC 8628) or `/bind <code>`, stored in `bot-config.v3.json` (`allowedUsers`).
- Sends tool approvals as interactive cards with buttons.

### Application to Fleet
Fleet's `src/gui/bridge.rs` already implements `sessions()`, `replay()`, `resume()`, `run()`, and `answer()`. To replicate ZCode's mobile Safari experience:
- Fleet can connect outbound to a lightweight WebSocket relay (or use a tunnel like Cloudflare Tunnel / `tailscale serve`).
- Fleet generates an ephemeral QR pairing URL.
- Mobile Safari receives Fleet's project and session list on bootstrap, letting the owner switch projects, inspect sessions, and approve tool calls on the go.

**Why:** Explored during design analysis for adding phone pairing / remote session access to Fleet.
**How to apply:** Model Fleet's remote mobile access after ZCode's `webRemoteControl` bootstrap protocol (projects + sessions grouped) over an outbound relay rather than exposing raw local ports.
