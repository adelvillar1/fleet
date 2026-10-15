# Fleet services

The five fleet lane cores — [sys1](sys1/) (judgment), [sdm1](sdm1/) (tabular),
[sem1](sem1/) (embeddings), [gen1](gen1/) (media), and [blend1](blend1/)
(the composed lane: one grounded `ask` over the specialized legs, with
the web leg and the escalation legs beside it) — vendored from the
deepseek-harness fork's `services/` tree (same author, Apache-2.0).
blend1's own upstream is the owner's `~/Projects/blend1` (see its
`VENDOR.json`). Each
carries a `VENDOR.json` recording the upstream pin and the provenance of this
copy; the upstream repos stay the development home.

The cores are stdlib-only Python: no third-party imports at module level, so
each one boots with nothing but a `python3` on PATH. Heavy dependencies —
sentence-transformers, GLiNER2.5, kokoro-onnx, llama-server and its GGUF —
live outside the core (a venv, a downloaded model, a LaunchAgent-owned
server) and are only needed when the corresponding tool actually runs. A
core whose provider lane is absent still boots; only the tool call that
needs the provider fails.

## Launching a lane

Every lane launches the same way — the command the fork's
`@fleet/dsh-components` bundle uses, replicated here:

```sh
<SVC>_PYTHONPATH=<home>/src python3 -u <home>/service/mcp_server.py
# working directory: <home>
```

Concretely, from this repository's root:

| Lane | Command (cwd = `services/<lane>/`) | Tools |
|---|---|---|
| sys1 | `SYS1_PYTHONPATH=src python3 -u service/mcp_server.py` | `sys1_classify` |
| sdm1 | `SDM1_PYTHONPATH=src python3 -u service/mcp_server.py` | `sdm1_classify`, `sdm1_health` |
| sem1 | `SEM1_PYTHONPATH=src python3 -u service/mcp_server.py` | `sem1_embed`, `sem1_probe`, `sem1_index`, `sem1_search` |
| gen1 | `GEN1_PYTHONPATH=src python3 -u service/mcp_server.py` | `gen1_imagine` |
| blend1 | `BLEND1_PYTHONPATH=src python3 -u service/mcp_server.py` | `ask`, `web`, `doctor`, `search`, `refresh`, `seed` |

Each child speaks newline-delimited JSON-RPC 2.0 (MCP protocol version
2024-11-05) on stdin/stdout: `initialize`, `tools/list`, `tools/call`, and
`ping`. Fleet's client (`src/mcp.rs`) sends the `notifications/initialized`
notification after `initialize`; sys1 and gen1 answer that notification with
a `-32601` error frame (id `null`) instead of ignoring it — the client
tolerates the stray frame (known issue KI-002). Closing stdin is a clean
shutdown: each server exits 0 on EOF.

`fleet lanes` runs this launch for every lane, health-checks it, and renders
the result — the command is the reference for "does the launch work from the
vendored copy."

## Home resolution

`src/lanes.rs` resolves each lane's home in the fork's order:

1. `FLEET_LANE_<SVC>_HOME` (e.g. `FLEET_LANE_SEM1_HOME`) — a live checkout or
   an installed copy;
2. the bundled copy under this directory — `FLEET_SERVICES_ROOT` overrides the
   root, otherwise it is discovered by walking up from the running executable;
3. unavailable — the lane reports the reason; nothing else fails.

Both the per-lane override and `FLEET_SERVICES_ROOT` must be **absolute**. A
relative path passes every layout check and still launches from wherever the
caller stood, so a relative one is refused with a named reason instead of
resolving silently.

## The child's environment

A lane child does **not** inherit the caller's environment. `src/mcp.rs`'s
spawn path calls `env_clear()` and hands the child exactly three things:

1. a minimal base — `PATH` (the lanes launch as `python3`, resolved through
   it), `HOME` (the cores resolve their config, cache and store roots through
   `Path.home()`), `LANG`, and `TMPDIR`, each carried only when the caller
   actually has it;
2. the lane's own `<SVC>_PYTHONPATH=<home>/src`;
3. a per-lane passthrough allowlist — the ambient names that lane legitimately
   needs, copied only when present. The list lives in `src/lanes.rs` beside the
   lane that declares it, and the rule is: a credential the lane's providers
   read, and a locator for a machine-local resource the lane cannot discover
   itself (a venv, an endpoint, a store root, a log root, an env file).

`DEEPSEEK_API_KEY` is on no lane's list — Fleet's own credential is none of
the lanes' business. Everything else in the caller's environment, however
credential-shaped, stays behind. The fork's launcher resolves credential refs
explicitly for the same reason; this is the native port of that policy.

## Heavy provider lanes (optional, machine-owner choices)

The cores boot without these; the tools that need them fail at call time
until the lane exists:

- **sem1**: the default `llama-server` provider is a local HTTP endpoint
  (default `http://127.0.0.1:8901/v1/embeddings`). The `st-worker` provider
  (text + images) needs a sentence-transformers venv; point `SEM1_ST_VENV` at
  it. The serving recipe lives in the upstream sem1 `deploy/` directory.
- **sys1**: local GLiNER2.5-Decide runs in its own venv; hosted providers
  need their API keys in the environment (credential-shaped names only — no
  key material lives in this tree).
- **sdm1**: the hosted TabPFN backend reads `TABPFN_API_KEY` from the
  environment.
- **gen1**: offline TTS (kokoro) and the image providers need their venvs
  and weights; `gen1 doctor` in the upstream repo reports what is present.

## Vendoring discipline

`__pycache__/` and egg-info are excluded from the copy (the upstream
`VENDOR.json` `excluded` list records this). To re-vendor after an upstream
change: copy the lane's `src/` and `service/` at the new sha, update
`VENDOR.json`, re-run `fleet lanes` plus the integration test, and record the
move in the ledger if anything about the launch changed.

**blend1 carries two fleet-authored files that a delete-syncing copy would
remove** — the MCP adapter (`service/mcp_server.py`, absent upstream) and
`service/README.md` (fleet's own note on the adapter, also absent upstream).
Re-apply both after the copy; `VENDOR.json`'s `fleetAddition` record names
them.
