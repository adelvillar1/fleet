#!/usr/bin/env python3
"""
sys1 MCP server: exposes `sys1.classify` as the `sys1_classify` tool.

Transport: stdio JSON-RPC (MCP 2024-11-05).

Dependencies: sys1 package on PYTHONPATH (use `pip install -e` or set
SYS1_PYTHONPATH to the `src` directory).
"""

from __future__ import annotations

import json
import os
import sys


def _add_sys1_src_to_path() -> None:
    """Ensure `import sys1` resolves from the repo src layout."""
    env_path = os.environ.get("SYS1_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


def main() -> None:
    _add_sys1_src_to_path()
    from sys1.api import classify
    from sys1.config import load_config

    try:
        # Fleet's carry: the documents session's Perception tool rides the
        # sys1 lane, and sys1.perception is a carried module (see
        # VENDOR.json's fleetVendor note) — the adapter degrades to
        # classify-only when the module is absent upstream.
        from sys1.perception import parse_document  # noqa: E402
    except ImportError:
        parse_document = None

    cfg = load_config()

    def respond(req_id, result):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}) + "\n")
        sys.stdout.flush()

    def error(req_id, code, msg):
        respond(req_id, {"error": {"code": code, "message": msg}})

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

        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "sys1", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "sys1_classify",
                        "description": (
                            "Run sys1 decision-model classification on text. "
                            "Returns verdict, providers used, summary, and latency."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "The payload to classify (diff, PR body, command string, …).",
                                },
                                "task": {
                                    "type": "string",
                                    "description": "Registered task name or 'change' default.",
                                    "default": "change",
                                },
                                "provider": {
                                    "type": "string",
                                    "description": (
                                        "Provider id (decide, glide, jev, …), alias ('both'/'all'), or comma list. "
                                        "Omit or leave empty to route per policy (providers.route); an explicit value bypasses routing."
                                    ),
                                },
                            },
                            "required": ["text"],
                        },
                    },
                    *(
                        []
                        if parse_document is None
                        else [
                            {
                                "name": "sys1_perceive_document",
                                "description": (
                                    "Parse one local document through Nace Perception, using the "
                                    "provider credential sys1 already holds. Returns markdown, pages, "
                                    "units, job id, and any content_truncated disclosure. A scan or a "
                                    "format local extraction cannot read is this tool's case."
                                ),
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "path": {
                                            "type": "string",
                                            "description": "Absolute path to the document on this machine.",
                                        },
                                    },
                                    "required": ["path"],
                                },
                            }
                        ]
                    ),
                ],
            })
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {}) or {}

            if tool_name == "sys1_perceive_document":
                if parse_document is None:
                    error(req_id, -32601, "Unknown tool: sys1_perceive_document")
                    continue
                path = arguments.get("path")
                if not isinstance(path, str) or not path.strip():
                    error(req_id, -32602, "`path` must be a non-empty string.")
                    continue
                try:
                    result = parse_document(path, cfg)
                except Exception as exc:  # pragma: no cover — transport boundary
                    result = {"ok": False, "code": "internal", "reason": str(exc)}
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(result)}],
                })
                continue

            if tool_name != "sys1_classify":
                error(req_id, -32601, f"Unknown tool: {tool_name}")
                continue

            text = arguments.get("text", "")
            task = arguments.get("task", "change")
            # An absent or empty `provider` passes through unset: classify then
            # routes per policy (providers.route), so the MCP surface never
            # coerces a caller into pinning one vendor. An explicit id, alias,
            # or comma list still bypasses routing entirely.
            provider = arguments.get("provider") or None
            if not isinstance(text, str) or not text.strip():
                error(req_id, -32602, "`text` must be a non-empty string.")
                continue

            try:
                result = classify(provider=provider, task=task, text=text, cfg=cfg, log=False)
            except Exception as exc:  # pragma: no cover — transport boundary
                error(req_id, -32603, str(exc))
                continue

            respond(req_id, {
                "content": [
                    {
                        "type": "text",
                        "text": json.dumps(
                            {
                                "task": result.task,
                                "verdict": result.verdict,
                                "escalated": result.escalated,
                                "provider": result.provider,
                                "providers_used": result.providers_used,
                                "answers": result.answers,
                                "summary": result.summary,
                                "latency_ms": result.latency_ms,
                                "reason": result.reason,
                            }
                        ),
                    }
                ],
            })
        elif method == "initialized":
            # Client notification; no response required.
            pass
        else:
            error(req_id, -32601, f"Method not found: {method}")


if __name__ == "__main__":
    main()
