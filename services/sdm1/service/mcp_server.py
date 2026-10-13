#!/usr/bin/env python3
"""
sdm1 MCP server: exposes `sdm1.classify` as the `sdm1_classify` tool and
`sdm1.health_report` as `sdm1_health`.

Transport: stdio JSON-RPC (MCP 2024-11-05).

The default model is `tabpfn-hosted` (Prior Labs REST lane): this server is
the fleet boundary for structured/tabular work, and the hosted lane is the
one that runs on any machine. Authentication is TABPFN_API_KEY in the child
env — credential-shaped names are scrubbed from the inherited environment by
the harness, so the profile row supplies the key explicitly.

Dependencies: sdm1 package on PYTHONPATH (use `pip install -e` or set
SDM1_PYTHONPATH to the `src` directory).
"""

from __future__ import annotations

import json
import os
import sys


def _add_sdm1_src_to_path() -> None:
    """Ensure `import sdm1` resolves from the repo src layout."""
    env_path = os.environ.get("SDM1_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


CLASSIFY_SCHEMA = {
    "type": "object",
    "properties": {
        "table": {
            "type": "array",
            "items": {"type": "object"},
            "description": (
                "Rows as objects. Rows carrying a non-null `target` value are "
                "the labeled context; rows with `target` null or missing are "
                "the query rows to score."
            ),
        },
        "target": {
            "type": "string",
            "description": "Name of the target column.",
        },
        "task_type": {
            "type": "string",
            "enum": ["classification", "regression", "forecast", "anomaly"],
            "description": (
                "Default: classification. Forecast needs a time-ordered table; "
                "anomaly query rows carry the observed value in "
                "`<target>_actual`."
            ),
            "default": "classification",
        },
        "model": {
            "type": "string",
            "description": "Provider id (tabpfn-hosted, nvidia-sdm). Default: tabpfn-hosted.",
            "default": "tabpfn-hosted",
        },
        "num_estimators": {
            "type": "integer",
            "description": "Ensemble size. Default: 8.",
            "default": 8,
        },
        "table_name": {
            "type": "string",
            "description": "Label used in the result. Default: table.",
            "default": "table",
        },
    },
    "required": ["table", "target"],
}


def main() -> None:
    _add_sdm1_src_to_path()
    from sdm1.api import classify, health_report  # noqa: E402  (import after path patch)

    def respond(req_id, result):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}) + "\n")
        sys.stdout.flush()

    def error(req_id, code, msg):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": msg}}) + "\n")
        sys.stdout.flush()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue

        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params", {}) or {}

        if req_id is None:
            # JSON-RPC notification: never answered. Replying emits a frame
            # with no pending requester (id null), which strict clients read
            # as protocol garbage on the shared stdin stream.
            continue
        if method == "ping":
            # MCP requires servers to answer pings with an empty result.
            respond(req_id, {})
            continue

        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "sdm1", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "sdm1_classify",
                        "description": (
                            "Score a table with a tabular foundation model. "
                            "Returns predictions with confidence, verdict, "
                            "summary, and latency. Never raises on backend "
                            "failure — failures come back as verdict=error."
                        ),
                        "inputSchema": CLASSIFY_SCHEMA,
                    },
                    {
                        "name": "sdm1_health",
                        "description": (
                            "Per-provider availability report: whether the "
                            "hosted lane is configured (TABPFN_API_KEY) and "
                            "reachable."
                        ),
                        "inputSchema": {"type": "object", "properties": {}},
                    },
                ],
            })
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {}) or {}

            if tool_name == "sdm1_health":
                try:
                    report = health_report()
                except Exception as exc:  # pragma: no cover — transport boundary
                    error(req_id, -32603, str(exc))
                    continue
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(report)}],
                })
                continue

            if tool_name != "sdm1_classify":
                error(req_id, -32601, f"Unknown tool: {tool_name}")
                continue

            table = arguments.get("table")
            target = arguments.get("target")
            if not isinstance(table, list) or not table or not all(isinstance(row, dict) for row in table):
                error(req_id, -32602, "`table` must be a non-empty array of row objects.")
                continue
            if not isinstance(target, str) or not target.strip():
                error(req_id, -32602, "`target` must be a non-empty string.")
                continue

            try:
                result = classify(
                    table,
                    target=target,
                    task_type=str(arguments.get("task_type") or "classification"),
                    model=str(arguments.get("model") or "tabpfn-hosted"),
                    num_estimators=int(arguments.get("num_estimators", 8)),
                    table_name=str(arguments.get("table_name") or "table"),
                )
            except ValueError as exc:
                error(req_id, -32602, str(exc))
                continue
            except Exception as exc:  # pragma: no cover — transport boundary
                error(req_id, -32603, str(exc))
                continue

            respond(req_id, {
                "content": [
                    {
                        "type": "text",
                        "text": json.dumps(result.as_dict()),
                    }
                ],
            })
        else:
            error(req_id, -32601, f"Method not found: {method}")


if __name__ == "__main__":
    main()
