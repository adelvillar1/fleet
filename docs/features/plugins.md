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

The second brain panel carries **the tool** instrument in its page head,
beside the title: the live state, the state's build time, how many
ledger events have landed since it, the last run, a job picker, and one
refresh button. The instrument is a status line rather than a card,
because a card at the top competed with the landscape below it.

While a job runs the instrument renders the tool's own steps (pending,
running, done, failed, with milliseconds), the log tail, and the failure
line when a step failed; the panel polls once a second and reloads the
derived state when the run finishes. A finished run's steps are history
and the instrument collapses back to its status line, so the page's
title is not pushed down by a list nobody is watching. A tool that is
not answering is a fact with its address and the last state it
published — the atlas below does not disappear.

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


# Face plugins

Fleet's fourth plugin shape. A **channel** carries work from outside; a
**lane** is a model service fleet launches and probes; a **tool
plugin** is a program the owner already runs; a **face plugin** is a
bundle that extends the face's own surfaces — a rail entry, a panel, a
workbench — the thing the DeepSeek Harness demos in creator mode. The
host is the `fleet plugins` lifecycle plus the face's mounting (the
face-plugin-host plan).

## The format

A plugin is a directory in `<FLEET_HOME>/plugins/<name>/`: a
`manifest.json` (the name, the tier, the version, the entry point —
the rail group and label — the untrusted tier's declared read-only
queries, and the provenance) and the static asset. The registry —
`<FLEET_HOME>/plugins.json` — carries each plugin's enable/disable
state and its promotion history, written through the settings writer's
atomic seam. The lifecycle is config, not code: `fleet plugins
list/install/enable/disable/remove`, and the face's plugins surface
(the one-card chain per plugin).

## The two tiers

- **Privileged** — the owner created or vouches for the bundle. Its
  rail entry opens its card as a first-class surface with the face's
  own API reach. Installing one is the elevation act: the install
  names the grant, and the tier rides into the installed manifest.
- **Untrusted** — creator mode's output, never trusted regardless of
  quality. Its card is its bundle in a **sandboxed frame** on the
  network-dead route (`/plugins/<name>/`, the canvas's CSP) — no
  same-origin, no network. Its data arrives only through the
  manifest's declared read-only queries, enforced bridge-side: an
  undeclared query is refused by name, and the bridge's
  `/api/plugins/query` is the sandbox's whole data path.

The ladder: **draft → installed** runs the static pass (a bundle that
does not conform does not install); **installed → privileged** runs
the whole suite and requires the owner's elevation act, which names
the grant *and* the verdict. The registry records each promotion's
event, verdict, and when. A generated draft is untrusted by
construction, and nothing promotes anything by itself — the suite
grades, the owner's act moves.

## The evaluation suite

`fleet plugins eval <name>` runs the deterministic layer: the
manifest's conformance, the credential sweep over the bundle's text
(a bundle naming a key shape is refused — the face's own payload sweep
class), and the asset's sanity. The declared pass drives each of the
manifest's queries once through the bridge's enforcement, so the
sandbox's data path is exercised, not asserted. The suite grades; it
never installs.

## Creator mode

A creator-mode task's product is a draft in
`<FLEET_HOME>/plugins/proposals/<name>/` — the model writes the
manifest and the asset through its own tools, and nothing applies it:
the plugins surface lists the drafts with their install acts, and
installing one is the owner's act (untrusted, sandboxed). The
`tests/fixtures/plugin-demo/` bundle — conversation statistics — is
the acceptance demo's shape: a generated draft's static asset,
rendering the face's real numbers through its declared queries.
