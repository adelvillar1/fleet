# blend1 service

Stdio MCP server around the [`blend1`](../) library — the blended lane of
the sys1 house pattern, and the boundary the fleet profile speaks to for
grounded asks. One `ask()` runs the d1 router (sys1) solo first, the
semantic (sem1) and tabular (sdm1) fact lanes concurrently, then
synthesizes with Liquid LFM2.5-24B-A2B only when the router clears
`ROUTE_FLOOR` (0.70); a validation gate (GLiDE → d1) rates the answer
before it ships.

`service/mcp_server.py` and this README are NEW in this vendored copy
(the upstream ships the library plus the FastAPI `main.py`, and has no
service-level README); see `VENDOR.json`'s `fleetAddition`. Both are
fleet carries: a re-vendor that copies `service/` over this tree removes
them, so re-apply both after the copy (see `services/README.md`'s
vendoring discipline).

## Tools

| Tool | Wraps | Purpose |
|---|---|---|
| `ask` | `blend1.ask` | One blended ask. Returns the BlendResult's fields plus `escalated` and `failed_lanes` — a lane that fails open records its reason; nothing degrades silently. |
| `doctor` | `blend1.doctor` | Per-lane availability snapshot (route, semantic, tabular, generate, validate, critics, voice, web), each with available + reason + provider. |
| `search` | `blend1.search` | Semantic recall over a project's corpus — scored entries, best first; an unseeded store returns []. |
| `refresh` | `blend1.refresh_corpus` | Docs-derived corpus refresh, hash-gated: the steady state is file reads, no embedding. |
| `seed` | `blend1.seed_corpus` | The corpus's write path; idempotent by key. |

## Environment

- `BLEND1_PYTHONPATH` — set to the repo `src/` dir so `import blend1`
  resolves (mandatory; an absent value is a named error on the wire).
  It addresses `services/blend1/src`, **not** a venv root: a hand-run
  adapter given `~/.fleet/venvs/blend1` cannot import the core. Fleet's
  own launch spec sets the whole wiring at spawn, so nothing needs
  exporting in the shell that boots fleet — the only case that needs
  these names is a hand run.
- `BLEND1_SYS1_PYTHONPATH` / `BLEND1_SEM1_PYTHONPATH` /
  `BLEND1_SDM1_PYTHONPATH` / `BLEND1_GEN1_PYTHONPATH` — the sibling lane
  cores, same mechanism the other adapters use. An absent sibling is not
  fatal: its lane fails open with the reason named.
- Keys resolve through blend1's own env loading
  (`~/.config/blend1/env` → sdm1 → dev-decisions → gen1 → ~/.hermes/.env,
  real environment wins). Names are reported by `doctor`, never values;
  the adapter writes JSON-RPC frames to stdout and nothing else.

## Project resolution over MCP

The library's rule is explicit parameter > `BLEND1_PROJECT` > cwd
git-repo name > `None` (the legacy store). **Over MCP the cwd is the
lane's home** — the launch spec sets it (fleet's `src/lanes.rs` and the
`cwd:` in the row above) — so an omitted `project` resolves to the repo
hosting `services/`, never the legacy store, and `refresh`'s default
globs resolve against that same cwd, where this lane holds no docs: an
omitted `sources` derives nothing and reports `action: "no-source"`.
Pass both explicitly.

## Wire it up (fleet lane row)

```yaml
- insert:
    - id: mcp-blend1
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        serverName: blend1
        transport: stdio
        command: python3
        args: [ '-u', '<repo>/services/blend1/service/mcp_server.py' ]
        env:
          BLEND1_PYTHONPATH: <repo>/services/blend1/src
          BLEND1_SYS1_PYTHONPATH: <repo>/services/sys1/src
          BLEND1_SEM1_PYTHONPATH: <repo>/services/sem1/src
          BLEND1_SDM1_PYTHONPATH: <repo>/services/sdm1/src
          BLEND1_GEN1_PYTHONPATH: <repo>/services/gen1/src
        cwd: <repo>/services/blend1
        failOnStartupError: false
```

## Test by hand

```bash
BLEND1_PYTHONPATH=../src python3 - <<'EOF' | python3 -u ../service/mcp_server.py
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
{"jsonrpc": "2.0", "method": "notifications/initialized"}
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "doctor", "arguments": {}}}
EOF
```
