# The web tool, available to every agent

Status: active
Date: 2026-10-15

## Why

TinyFish is already in fleet — `services/blend1/src/blend1/lanes.py:332`,
the `web` leg: search plus fetch, live sources as givens, bounded at
three URLs and 2000 characters each, url provenance riding every
excerpt, and the fetched text framed as data-only in the S2 prompt as
a prompt-injection defense. It is reachable today only through
blend1's MCP `ask` tool with the web leg forced (`live: true`) or left
to auto — which means paying for the whole composed answer
(synthesizer, critics, validation gate) to look something up, and only
when blend1 happens to be in the run's lane specs. A plain
`fleet run` passes an empty specs map (`src/main.rs:127`), so the
capability is unreachable from an ordinary run at all.

The owner's ruling: surface it so it is usable by all agents, root and
delegates alike.

## Acceptance criteria

- **C1** A `web` tool declared on every run — the same
  declared-everywhere rule `convention` runs on, because a live source
  is as much the tree's own as its vocabulary.
- **C2** Admitted to a delegate's scope by default: it joins
  `ADMISSIBLE_TOOLS` (`src/subagents.rs:207`) so a role that narrows
  to the default set still carries it, and a role that narrows harder
  drops it deliberately rather than by accident.
- **C3** The execution resolves blend1's launch spec itself — the
  pattern `judge()` already runs for sys1 (`src/route.rs:1146`) — so
  the tool works whether or not the run carried lane specs, and works
  identically for a root and a depth-2 grandchild.
- **C4** The bounds and the injection defense are the web leg's own,
  not a second implementation: the Rust side declares and dispatches,
  the lane decides what a fetched excerpt may contain and how it is
  framed. A tool result that arrived through the lane says so, with
  its url provenance.
- **C5** A missing `TINYFISH_API_KEY` is a named refusal the model can
  read and react to — the lane's own fail-open naming the extra — not
  a silent empty answer. The key is referenced by NAME only, never a
  value (standing rule).
- **C6** `fleet doctor` names whether the web leg is live, the way it
  already names the vector path.
- **C7** Tests: the declaration rides every run, the delegate scope
  admits it, the spec resolution works with an empty specs map, and
  the unkeyed refusal is named.

## Design

The tool declares `search` and `fetch` shapes over one lane call. The
Rust side is thin on purpose: ~30 lines reusing `McpServer::spawn` and
`call_tool`, the machinery the judge path already proves.

One addition to the vendored service is required: blend1's MCP server
exposes `ask`, `doctor`, `search` (its corpus search), and `refresh` —
there is no tool that calls the web leg without the compose. Adding
`web` to `services/blend1/service/mcp_server.py` is the small, honest
change; the alternative — a fresh Rust client to TinyFish's REST API —
would duplicate the bounds and the injection defense in a second place,
which is the reliability loss the owner's ruling exists to prevent.

The tool's model-visible name is `web`, not `blend1_web`: the agent
asks for a live source, and where the code lives is the lane's
business. The lane namespace (`mcp__blend1__web`) remains for a run
that carries blend1 explicitly.

## Out of scope

- A sixth vendored lane owning the web leg. Extracting it from blend1
  is a real refactor with a real cost, and nothing in the current ask
  needs it.
- Caching fetched pages. A live source is live; the route-decision
  cache's discipline does not transfer.
- The delivery seam. Separate session, separate architecture decision
  (what leaves the machine at all).
