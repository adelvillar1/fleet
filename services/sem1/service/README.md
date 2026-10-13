# sem1 service

Stdio MCP server around the [`sem1`](../) library — the boundary the
DeepSeek-harness fleet profile speaks to for semantic/embedding work.
Vectors in, vectors out; verdicts stay with the decision layers.

## Tools

| Tool | Wraps | Purpose |
|---|---|---|
| `sem1_embed` | `sem1.embed` | Embed text (or images via `st-worker`) into vectors. Refuses unsupported modalities instead of answering. |
| `sem1_probe` | `sem1.probe` | Capability + reachability report: declared modalities, dims, model, latency. |

## Environment

- `SEM1_PYTHONPATH` — set to the repo `src/` dir so `import sem1` resolves
  (or `pip install -e`).
- No secrets: the default `llama-server` provider is the local LaunchAgent
  at `http://127.0.0.1:8901` (`deploy/`). The `st-worker` provider needs the
  `.venv-st` venv (`SEM1_ST_VENV` overrides).

## Wire it up (fleet profile row)

```yaml
- insert:
    - id: mcp-sem1
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        serverName: sem1
        transport: stdio
        command: python3
        args: [ '-u', '/Users/alejandrodelvillar/Projects/sem1/service/mcp_server.py' ]
        env:
          SEM1_PYTHONPATH: /Users/alejandrodelvillar/Projects/sem1/src
        cwd: /Users/alejandrodelvillar/Projects/sem1
        failOnStartupError: false
```

## Test by hand

```bash
SEM1_PYTHONPATH=../src python3 - <<'EOF' | python3 -u ../service/mcp_server.py
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "sem1_probe", "arguments": {}}}
EOF
```
