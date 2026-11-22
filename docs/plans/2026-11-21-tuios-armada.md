---
status: completed — 2026-11-22 (4df5fd0c the protocol layer and the rule; the card, the harness cases and the live tests in its follow-up commit)
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

- [x] A machines card gains a tuios section when `tuios call list-windows`
      answers, with per-session rows named above; red-proof: kill the daemon
      (or point the probe at a dead socket) and the row names the refusal.
      Landed: the section rides the map's "what it carries" cell
      (`tuiosSection` in `app.js`). The refusal is proven twice — on a dead
      socket against the real binary (`--server fleet-dead`, exit 3, the
      daemon's own words: `tests/tuios.rs`) and in the card
      (`TUIO a refusal is named and never an empty grid`), which paints the
      sentence and no session rows.
- [x] The AGENT states pass through verbatim with tuios's own vocabulary.
      `BTreeMap<String, u64>` of the daemon's words per session, printed as
      `working 2 · needs_input 1` — the harness asserts the words and
      asserts Fleet's own (`needs you`, `waiting`, `blocked`) are absent.
- [x] A fleet-tui pane is discoverable (the session this plan created for its
      own proof is the first live specimen). Measured on the live daemon:
      `1 session(s) (["fleet-tui"]); 1 fleet pane(s): [("fleet-tui",
      "Terminal cb1fe908", "fleet")]`. The rule is the live command line —
      `foreground_cmd` or `running_cmdline` by basename — and the red test
      that started this pass (a fake answering one session's windows for
      every session) is settled with it: history and cwd are not evidence.
- [x] `cargo test` green; the verb calls are integration-tested against the
      real daemon on this machine and SKIP named otherwise. `tests/tuios.rs`
      (4 tests) reads the live daemon; with no binary it prints
      `tuios: skipped — tuios is not installed on this machine
      (FLEET_TUIOS_BIN=… is not a file)` and passes. The armada document is
      proven opt-in-gated: `tuios: null` before `<home>/tuios.json` enables
      the sid, the daemon's sessions after.

## Status pass, 2026-11-22 — landed

`4df5fd0c` (the protocol layer: `src/tuios.rs`, the opt-in file, the four read
verbs, the refusal shapes, the fleet-pane rule and its red-proof) and this
commit (the card, the harness cases, the live tests, this record).

The red test the interrupted pass left: `a_fleet_pane_is_matched_by_the_live_
command_line` was right and its fixture was wrong — the fake `tuios` answered
`list-windows` for every session from one body, so the work session "held"
Fleet's windows. The fake now answers per session, as the daemon does
(`$3` is the params JSON), and the work session keeps a shell whose cwd is the
repo and whose history says `fleet tui` — and stays a shell. Inverted and
restored: adding `last_cmdline` as evidence reds both the case and the rule
table by name.

The four sentences are the section's own (`SENTENCES.tuios`): the absence is
named once below the map (`no tuios section on any instance yet — … in
<home>/tuios.json`), the refusal carries the daemon's words, the selected line
names the session carrying Fleet's TUI, and the loading line rides the map's
own reading card. Deleting the absence sentence reds
`TUIO the section's absence is its own named state` and the table case.

Measured while building: a `--server` name is capped at **14 characters** by
the CLI itself (a longer one is a usage error, not a transport error) — worth
knowing before naming a daemon.

The one thing this plan did not add: a `fleet tuios` CLI verb. The opt-in is
the file `<home>/tuios.json` (`OptIn::enable` is the library's own writer), and
the card's sentence names the file rather than a command that does not exist.

## Files

- `src/boards.rs` — the armada read gains the tuios section (per-instance,
  opt-in).
- `src/gui/bridge.rs` + `src/gui/assets/app.js` — the map's card renders the
  section (the four sentences law).
- tests: the verb-call layer against the live daemon, skip-named otherwise.
