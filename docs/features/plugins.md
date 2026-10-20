# Tool plugins

Fleet's third plugin shape. A **channel** carries work from outside; a
**lane** is a model service fleet launches and probes; a **tool plugin**
is a program the owner already runs — with its own repository, its own
state, and its own write vocabulary — which fleet surfaces without
reimplementing. Second brain is the first.

The rule: fleet reads what the tool publishes and forwards the actions
the tool already exposes. It never writes the tool's state, and it gives
an action no meaning the tool does not.

```
src/plugins/mod.rs            the seam (one module, no trait yet)
src/plugins/second_brain.rs   discovery, status, the refresh action, tests
src/gui/bridge.rs             /api/second-brain/plugin and /api/second-brain/act
src/gui/server.rs             the two routes
src/gui/assets/app.js         the tool card: facts, job picker, refresh, pipeline steps
```

## The second-brain plugin

Second brain lives at `~/Projects/second-brain` (also `~/.second-brain`,
a symlink) and serves its own dashboard on 4175. Its `POST /api/run`
starts one of nine pipeline jobs on a thread and returns; `GET
/api/status` names the running job, its steps, its log tail, and the
last build. Those are the actions and the progress the plugin carries.

| | |
|---|---|
| Home | `SECOND_BRAIN_HOME`, then `~/.second-brain`, then `~/Projects/second-brain` — the boards' own resolution, so the plugin and the state read cannot disagree about which tool this machine has |
| Address | `http://127.0.0.1:<SECOND_BRAIN_PORT or 4175>` |
| Jobs | `all`, `chats`, `knowledge`, `gates`, `harness`, `reports`, `maturity`, `loops`, `tower`, `mirror` |
| Guard | the tool's `X-Tower: 1` header, set server-side; the browser never talks to the tool |

The job names are the tool's `PIPE_DEFS` keys, copied once in
`src/plugins/second_brain.rs`. An unknown name is refused with the
vocabulary sentence before it reaches the tool; the tool validates
again, which is the authority.

## The face

The second brain panel opens with **the tool** card: the home, the
address, the state's build time, the last run, a job picker, and one
refresh button. While a job runs the card renders the tool's own steps
(pending, running, done, failed, with milliseconds), the log tail, and
the failure line when a step failed; the panel polls once a second and
reloads the derived state when the run finishes. A tool that is not
answering is a fact with its address and the last state it published —
the sections below do not disappear.

The chosen job lives on the face (`face.pluginJob`), not on the select
element: the card repaints on every poll, and a selection that reset
mid-choice would send a different job than the one the owner read.

## Failure rules

- **Not live**: the probe costs two seconds; the state read is separate
  and still lands. The card says the address it tried.
- **Not installed**: a resolved home with no `control_tower.py` is named
  as a misconfiguration, not an outage — the two have different fixes.
- **Busy**: the tool's 409 (`pipeline busy: <job>`) is shown as the tool
  said it.
- **Refused**: an unknown job or action is a sentence, not a bare 400
  from a program the owner is not looking at.

Starting the tool's server is deliberately not an action yet: the tool
may already be supervised elsewhere, and attaching to a running
instance is the rule. A start action needs that lifetime question
settled first.

## Tests

`cargo test --lib plugins::second_brain` covers the status parse, the
idle read, the vocabulary refusal, the posted job and guard header
against a one-request mock, the busy 409, the dead address, and the
entry-point check.

The face was verified in Chromium against a mock tool: the refresh
button posts the chosen job with the guard header, the button enters
`running…`, the steps move to done with their milliseconds, and the
state stamp reloads when the run finishes.
