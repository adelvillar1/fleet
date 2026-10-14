# Subagents (slice 9)

One delegated task, run in its own child session on the parent's own
route, with a scoped tool set, the parent's approver, a bounded turn cap,
and a result that returns partial results with a typed reason instead of
silence.

```
src/subagents.rs    the module: the scope, the request, the child route, the run, the result
src/tools.rs        the `subagent` declaration (the tool vocabulary's home)
src/agent.rs        the scope in AgentOptions, the scoped declarations, the dispatch, the UiEvent
src/session.rs      the `kind`/`parent` fields on `start`, create_subagent, the summary's kind
src/tui/app.rs      the Subagent event onto the tool card
src/tui/ui.rs       the picker's `subagent` marker
src/main.rs         `fleet sessions` names the kind
```

## The tool

`subagent` is declared beside `use_skill`, `remember`, and `supersede` on
every run that is not itself a subagent — delegation is part of the tool
set this crate ships, not a capability a run opts into. Three parameters:

```json
{
  "type": "function",
  "function": {
    "name": "subagent",
    "description": "Spawn a subagent that runs one delegated task in its own session, with its own context and no knowledge of this conversation. …",
    "parameters": {
      "type": "object",
      "properties": {
        "goal": {
          "type": "string",
          "description": "The delegated task, stated in full. The subagent sees nothing of this conversation, so every path, constraint, and definition of done belongs here."
        },
        "tools": {
          "type": "array",
          "description": "The subagent's admitted tool names, narrowing the default (bash, read_file, write_file, list_dir, use_skill). An empty list admits no tool at all.",
          "items": {
            "type": "string",
            "enum": ["bash", "read_file", "write_file", "list_dir", "use_skill"]
          }
        },
        "max-turns": {
          "type": "integer",
          "minimum": 1,
          "maximum": 50,
          "description": "The subagent's turn cap. Defaults to 20 and is bounded at 50."
        }
      },
      "required": ["goal"]
    }
  }
}
```

`goal` is required because the child sees **nothing** of the parent's
conversation: a goal that is not self-contained is a task the child
cannot do. `tools` narrows the admitted scope, and its enum is the whole
admissible set — `remember`/`supersede` (a subagent writing durable facts
is slice 12's provenance question), the lane namespace (a subagent
spawning lanes is scope creep), and `subagent` itself (depth one) are not
in it, so a conforming caller cannot name them. `max-turns` is bounded in
the schema **and** at parse, because a bound only the schema carries is a
bound a non-conforming caller ignores.

## The child's run

The loop is reused, never forked. A child is an ordinary `Agent` over an
ordinary `Session`:

- a **new session log** under the parent log's own directory, whose
  `start` record carries `"kind":"subagent"` and the parent's session id;
- the **same compiled-in `SYSTEM_PROMPT`** (plus the skills list the
  parent declared, because `use_skill` is in the admitted scope and a
  tool whose every call fails is not a tool);
- the **goal as its first and only user message**;
- the **scoped declarations**;
- the **child's turn cap** (`SUBAGENT_TURN_CAP` = 20 by default, 1..=50
  when asked).

The child's `start` record, verbatim from a real run:

```json
{"type":"start","v":1,"model":"deepseek-chat","created":"2026-10-14T01:46:02Z","kind":"subagent","parent":"20261014T014602Z-b3be8297"}
```

An ordinary session's `start` record carries neither field, so an
ordinary log's first line is byte-for-byte what it has always been. Both
fields are `Option` with `skip_serializing_if`, so a pre-slice-9 log
still deserializes: the vocabulary grew mid-format-version, the way
`lane_tools`, `skills`, and `memory` did.

## Same model, structurally

The child runs on the **same routed client** the parent is using and
takes its model from the parent's own options, so "never downgrades"
(directive 2, hard rule 9) is a property of the construction rather than
a rule the model is asked to follow. There is no `model` parameter in the
schema and none in the request; the only place a child could resolve a
different model is `subagents::child_route`, and it does not — it returns
the parent's model and carries the parent's active provider decision
beside it, to be named in the progress line.

This is the in-product form of the fork's model-selection preflight: the
fork checks a child's requested route against an allowed list before
creating it (`assertAllowedModelSelection`, `preflightChildLlmRoute` in
`packages/subagent/tool-subagent/src/model-selection.ts`), and Fleet's
equivalent is that there is nothing to check — the route is inherited,
not chosen.

The smoke's recorded requests prove it on the wire: all four (the
parent's two and the child's two) carried the roster candidate's model,
not the nominal one the options named.

## The same approver

One gate for the whole tree. The child's calls reach the owner through
the parent's `Approver`, and the forwarded call is renamed
`subagent → <tool>` and carries the goal beside the child's own
arguments, so a console prompt reads:

```
fleet: allow subagent → bash `count the files · ls -la`? [y/N/a]
```

The labelling happens on the way to the gate and never on the way back:
the child's own log and its screening rows name the real tool.

## The scope

`Scope` decides two things, and both matter: which declarations the
child's requests carry, and which calls the loop refuses. A tool outside
the scope is not merely uncallable — it is absent from what the model
saw. The refusal is what meets a call that arrives anyway:

```
error: `write_file` is outside this subagent's admitted scope (bash); ask for it in the task, or do it yourself
error: `subagent` is outside this subagent's admitted scope: a subagent cannot spawn subagents (depth cap 1)
```

The scope check sits **after** the approval gate, the order every other
call takes: the owner sees a subagent reach for something it was not
given, and the harness refuses it. That also keeps the screening row the
calibration stream carries as the gate's own verdict, rather than a
second kind of verdict invented for the scope.

## Depth

`SUBAGENT_MAX_DEPTH = 1`, a named constant. Enforcement is structural —
the tool is outside a child's admitted scope, so the loop refuses the
call by name — and the constant names the cap the refusal cites and the
assertion in `subagents::run` checks, so a later slice that opens a
second level meets a guard rather than inventing one.

## The result

```rust
pub struct SubagentResult {
    pub summary: String,      // the child's final message, or its last words
    pub session: String,      // the child's session id
    pub log: PathBuf,         // the child's log path
    pub turns: u64,           // the turns the child ran
    pub outcome: Outcome,     // Completed | MaxTurns | Failed
    pub reason: Option<String>, // the typed failure reason, on Failed
}
```

rendered as one header line plus the child's own words, and riding the
same bounded `tool_result` path every other tool's output rides:

```
subagent completed · 2 turns · session 20261014T014602Z-39b19298 · log /…/sessions/20261014T014602Z-39b19298.jsonl
The working directory holds three entries: config.json, home, and sessions.

subagent failed (API error 500: the child's model went away) · 1 turn · session … · log …
checked one of five
```

A failure returns the partial result (the last words in the child's own
log), the typed reason (the agent error's own display, so the parent's
log and the child's turn row name the same cause), and the log path —
never silence, never an empty `Ok`. A child that produced no words says
so explicitly. The `turns` count on a failure is read back from the
child's log rather than the loop's counter, because the counter would
name the turn the child died on.

## The live surface

The parent's turn waits; progress rides the existing `UiEvent` seam as
one new variant, `Subagent { goal, session, stage }` with
`SubagentStage::{Spawned, Working { calls, tool }, Done { outcome, turns }}`.
The child's own turn, token, and finish events deliberately do **not**
ride there — they belong to the child's conversation, and landing them in
the parent's would move its turn counter and clear its working state
mid-run.

The TUI renders the delegation as one tool card in the existing card
style: the ordinary `ToolCall` event opens it, the subagent events fill
its preview (`child <id> · 2 call(s) · last bash`), and the ordinary
`ToolResult` event closes it with the outcome. Headless prints progress
lines to the same sink the child's own turns write to:

```
— subagent: List the entries … · child 20261014T014602Z-39b19298 · 20-turn cap on router-kit
— turn 1
→ list_dir: .
← subagent completed · 2 turns
```

`fleet sessions` and the TUI's picker name the kind beside the row —
`ended subagent` against `ended session` — because a child is an ordinary
session with a parent, not a second class of thing.

## Calibration

No new op. The child's `turn` and `tool_screen` rows land in the same
stream the parent's do, naturally: they are real turns on the real
roster, recorded by the same `Agent` code the parent's run uses. The
`route` row stays one per session — the child shares the parent's router,
so its requests are already inside the parent's decision row's
`providers_used`.

## What a subagent is not

- **Not a second agent.** It is this loop, this prompt, this client, this
  gate, this log format — with a different conversation and a narrower
  tool set.
- **Not a memory writer.** `remember`/`supersede` are outside the scope;
  a subagent's findings reach the corpus only if the parent writes them,
  which is slice 12's provenance question.
- **Not a lane caller.** The lane namespace is outside the scope; a child
  that needs a lane says so in its answer and the parent calls the lane.
- **Not a spawner.** Depth one, enforced twice.
- **Not concurrent.** The parent's turn waits. A child that takes twenty
  turns takes twenty turns of the parent's turn.
- **Not evidence.** The summary is the child's own report. The result
  returns the means to check it — the session id, the log path, the turn
  count, the outcome — and the tool's description tells the parent to
  verify any external side effect the child claims (a file it wrote, a
  command it ran) before reporting that claim as fact. The verification
  is the parent's, by design: a mechanism that "verified" would be the
  child grading its own homework.
