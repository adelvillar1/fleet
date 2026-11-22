---
status: active
created: 2026-11-21
plan: tuios-armada
---

# tuios in the armada map — what the owner's multiplexer carries

The owner runs **tuios 0.13.3** (installed 2026-11-21, `gaurav-gosain/tap`)
as their desktop terminal shell, and Fleet's TUI already runs inside it as a
detached, daemon-kept pane (`tuios attach fleet-tui`). Tier 1 (the owner's
shell) is done and proven by capture: Fleet's own frame, rendered in tuios,
visible through `tuios capture-pane`.

**Tier 2 is this plan**: the armada deployment map gains tuios as one of the
things an instance carries — measured through tuios's own JSON control
protocol, never guessed.

## The measured protocol facts (0.13.3, live daemon on this machine)

- `tuios call <verb> [json]` is the control surface (`list-verbs` names them;
  control protocol version 1). Errors carry `{error: {code, message, hint}}`
  and exit 1; unreachable daemon exits 3.
- Verbs measured and useful to the map: `list-windows` (per session: id, name,
  workspace, size, **AGENT** column), `capture-pane` (what a pane shows),
  `agent-activity` (the daemon's per-agent ring: prompts, tool calls, finished
  turns, **marked untrusted by tuios itself**), `add-task` (task cards with
  stages `todo→plan→doing→verify→review→shipped→done`, claimable **across
  linked machines**), `list-agents`, `list-attention`.
- The daemon starts on demand and keeps sessions across restarts; sessions can
  be created **on other machines** (`--host build`).

## What the armada map gains

One row per tuios session on a machine that runs the daemon, carrying: session
name · windows (count by AGENT state) · the agents' own stage words · the
fleet-related panes it sees (Fleet's TUI is one). The card is **derived from
the protocol's answers** — a machine without the daemon gets the named
refusal ("tuios is not running on ⟨instance⟩"), never an empty grid; an
unreadable verb is a named probe failure.

The untrusted marking is honoured: the map shows tuios's own stage words, it
does not judge them (that is the calibration's job, and the beats stay where
they are).

## Rules

- Same-document law: the map calls the daemon the owner runs; it does not
  cache or recompute tuios state.
- The map is read-only over the protocol — no `add-task`, no send-keys, no
  move. Fleet watches the multiplexer; Fleet drives sessions through its own
  paths.
- Optional per instance: the tuios integration is a card the owner enables
  per machine (the armada map's existing per-instance rows), never global.

## Acceptance criteria

- [ ] A machines card gains a tuios section when `tuios call list-windows`
      answers, with per-session rows named above; red-proof: kill the daemon
      (or point the probe at a dead socket) and the row names the refusal.
- [ ] The AGENT states pass through verbatim with tuios's own vocabulary.
- [ ] A fleet-tui pane is discoverable (the session this plan created for its
      own proof is the first live specimen).
- [ ] `cargo test` green; the verb calls are integration-tested against the
      real daemon on this machine and SKIP named otherwise.

## Files

- `src/boards.rs` — the armada read gains the tuios section (per-instance,
  opt-in).
- `src/gui/bridge.rs` + `src/gui/assets/app.js` — the map's card renders the
  section (the four sentences law).
- tests: the verb-call layer against the live daemon, skip-named otherwise.
