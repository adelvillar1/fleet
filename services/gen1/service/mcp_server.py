#!/usr/bin/env python3
"""
gen1 MCP server: exposes `gen1.imagine` as the `gen1_imagine` tool.

Transport: stdio JSON-RPC (MCP 2024-11-05).

Returned images are base64 data URIs so the harness can render them in the
deliverable preview tab without touching the filesystem.

Dependencies: gen1 package on PYTHONPATH (use `pip install -e` or set
GEN1_PYTHONPATH to the `src` directory).
"""

from __future__ import annotations

import base64
import json
import os
import sys


def _add_gen1_src_to_path() -> None:
    """Ensure `import gen1` resolves from the repo src layout."""
    env_path = os.environ.get("GEN1_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


def main() -> None:
    _add_gen1_src_to_path()
    from gen1.cascade import imagine  # noqa: E402  (import after path patch)

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
                "serverInfo": {"name": "gen1", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "gen1_imagine",
                        "description": (
                            "Generate an image from a prompt via gen1's cascade. "
                            "Returns base64 data URIs for the harness preview."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "prompt": {
                                    "type": "string",
                                    "description": "Text prompt for image generation.",
                                },
                                "aspect_ratio": {
                                    "type": "string",
                                    "description": "Aspect ratio string, e.g. '1:1', '16:9'.",
                                    "default": "1:1",
                                },
                                "n": {
                                    "type": "integer",
                                    "description": "Number of images to generate.",
                                    "default": 1,
                                },
                                "provider": {
                                    "type": "string",
                                    "description": "Optional provider id to force.",
                                },
                            },
                            "required": ["prompt"],
                        },
                    }
                ],
            })
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {}) or {}

            if tool_name != "gen1_imagine":
                error(req_id, -32601, f"Unknown tool: {tool_name}")
                continue

            prompt = arguments.get("prompt", "")
            if not isinstance(prompt, str) or not prompt.strip():
                error(req_id, -32602, "`prompt` must be a non-empty string.")
                continue

            try:
                result = imagine(
                    prompt,
                    aspect_ratio=str(arguments.get("aspect_ratio", "1:1")),
                    n=int(arguments.get("n", 1)),
                    provider=arguments.get("provider"),
                )
            except Exception as exc:  # pragma: no cover — transport boundary
                error(req_id, -32603, str(exc))
                continue

            images = []
            for img in result.images:
                b64 = base64.b64encode(img.data).decode("ascii")
                images.append(f"data:{img.mime};base64,{b64}")

            respond(req_id, {
                "content": [
                    {
                        "type": "text",
                        "text": json.dumps(
                            {
                                "provider": result.provider,
                                "model": result.model,
                                "telemetry": result.telemetry.__dict__,
                                "images": images,
                                "count": len(images),
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
