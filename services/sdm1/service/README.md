# sdm1 service

Stdio MCP server around the [`sdm1`](../) library — the boundary the
DeepSeek-harness fleet profile speaks to for structured/tabular work.

## Tools

| Tool | Wraps | Purpose |
|---|---|---|
| `sdm1_classify` | `sdm1.classify` | Score a table (classification / regression / forecast / anomaly). Backend failures come back as `verdict=error`, never a raise. |
| `sdm1_health` | `sdm1.health_report` | Per-provider availability (configured + reachable). |

## Environment

- `SDM1_PYTHONPATH` — set to the repo `src/` dir so `import sdm1` resolves
  (or `pip install -e`).
- `TABPFN_API_KEY` — authenticates the `tabpfn-hosted` lane (default model).
  Credential-shaped names are scrubbed from inherited environments by the
  harness, so the fleet profile row supplies the key explicitly.
- `SDM1_TABPFN_BASE`, `SDM1_TABPFN_TIMEOUT`, `SDM1_LOG_DIR` — optional; see
  the library docs.

## Wire it up (fleet profile row)

```yaml
- insert:
    - id: mcp-sdm1
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        serverName: sdm1
        transport: stdio
        command: python3
        args: [ '-u', '/Users/alejandrodelvillar/Projects/sdm1/service/mcp_server.py' ]
        env:
          SDM1_PYTHONPATH: /Users/alejandrodelvillar/Projects/sdm1/src
          TABPFN_API_KEY: <machine key>
        cwd: /Users/alejandrodelvillar/Projects/sdm1
        failOnStartupError: false
```

## Test by hand

```bash
SDM1_PYTHONPATH=../src python3 - <<'EOF' | python3 -u ../service/mcp_server.py
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
{"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
EOF
```
