# The MCP server registry

Fleet's MCP story has two registries. The **vendored lanes** — sys1,
sdm1, sem1, gen1, blend1, browser — are compiled in, resolved lazily,
and configured through `lanes.json`. The **server registry** — this
document — is the owner's: the MCP servers they configured beside the
lanes, carried by the fleet home's own `mcp.json`, discovered from the
machine's other harnesses' configs or written by hand.

The model-facing namespace is shared: a tool crosses as
`mcp__<name>__<tool>`, whichever registry owns the name. The six lane
names and the four built-in tools are reserved, so a server can never
be indistinguishable from a lane.

## The file

`<FLEET_HOME>/mcp.json`, one object of `servers`, one entry per server:

```json
{
  "servers": {
    "github": {
      "program": "docker",
      "args": ["run", "-i", "--rm", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN", "ghcr.io/github/github-mcp-server"],
      "env_names": ["GITHUB_PERSONAL_ACCESS_TOKEN"],
      "enabled": true,
      "source": { "harness": "claude-desktop", "imported": "2026-10-23" },
      "last_probe": { "at": "…", "ok": true, "server_name": "github-mcp",
                      "server_version": "0.9.0", "tools": 12, "reason": "" }
    }
  }
}
```

The one rule the shape enforces: **env NAMES only, never values** —
there is no field a value could be written into, and an entry carrying
`env` is refused at write and at read. Each name resolves at spawn
through the chain the lanes use — the process env, then the fleet
home's `.env`/vault — and an unresolved name is a named refusal, never
a silent empty value. A credential that rides inside the args (the
desktop configs' `docker run -e NAME=VALUE`) is extracted at the scan:
the stored args become `-e NAME`, the NAME joins `env_names`, and no
field carries the value. Named residual: a credential inside a server's
own address (`https://host/mcp?apiKey=…`) cannot be extracted — the URL
is the launch — so the registry stores it verbatim and no card ever
prints it.

## The verbs

`fleet mcp` renders the registry and the scan as cards, in the lanes'
visual grammar — never a plain-text table.

- `fleet mcp` — the configured servers: status dot (probed ok, never
  probed, last probe failed), enabled state, tool count, the launch
  line, each env name marked as resolving at spawn, and the source
  badge for an imported server. An empty registry is an empty-state
  card that offers the scan.
- `fleet mcp scan` — the machine's four foreign configs (Claude
  Desktop, Claude Code, Cursor, Codex), read-only, never written. Each
  entry is reported **configured** / **available** / **unsupported**
  with its source file named; absent files are skipped silently, an
  unparseable file is a named skip, and an HTTP/SSE entry reports
  unsupported by shape. Each row carries an advisorial badge —
  **containerized**, **pinned package**, **bare interpreter**,
  **remote endpoint**, **unclassified** — read off the entry's own
  launch shape; it orders the review list (containerized and pinned
  first, bare interpreters flagged) and decides nothing.
- `fleet mcp import <name>` — one scanned entry into the registry,
  with the source badge and the source's own enabled flag (an entry the
  other harness had disabled imports disabled). An unsupported shape is
  refused by its reason. A name discovered under more than one harness
  is refused until the ask says `fleet mcp import <harness>:<name>`.
- `fleet mcp enable|disable <name>` — in place, through the one
  settings writer; a disabled server stays in the registry and out of
  every run's declarations.
- `fleet mcp remove <name>` — out of the registry, naming what remains.
- `fleet mcp health` — probe every configured server: spawn,
  initialize, `tools/list`, report the server's self-reported name and
  version, tear down. Each outcome rides `last_probe`, so the cards
  carry it; the exit code is success only when every probe came back
  ok. A failure names its step with the child's stderr tail — the lane
  probe's way.

## The run's view

Every owner-facing run — the headless `fleet run`/`chat` and the face's
composer — seeds the registry's enabled servers into its lane specs and
**pre-joins** them before the first request: their tools are declared
from the first turn, because a lazily-learned server would never
declare its tools at all. Each join is visible twice over, one
`server joined` line and one `lane_tools` record naming both ends of
the mapping. A server that fails to join is dead for the run and its
reason rides the progress log; a server whose env name resolves to
nothing refuses the run by name before anything spawns. A subagent's
run and a scheduled job do not pre-join: the child sees nothing of the
owner's servers, and an unattended job neither needs them nor pays
their spawn cost.

## The card

The settings board carries the servers card beneath the lanes card: the
same rows the CLI renders, with enable/probe/remove actions, and the
last scan's discovery half beneath — one import action per available
entry, an unsupported row's import disabled with its reason, configured
entries collapsed into a note. Management is loopback-only:
`/api/settings` is absent from both of the remote face's allowlists.
