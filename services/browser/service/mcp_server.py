#!/usr/bin/env python3
"""
browser MCP server: a real Chrome, driven over the DevTools Protocol.

Exposes `browser_navigate`, `browser_screenshot`, `browser_read` as tools.

Transport: stdio JSON-RPC (MCP 2024-11-05).

The lane exists because a deck is HTML, and nothing in a static Rust
binary rasterizes HTML. A browser does. Chrome's location rides
`BROWSER_CHROME`; when it is unset the usual suspects are probed in
order and the refusal names what was looked for.

Dependencies: browser package on PYTHONPATH (set BROWSER_PYTHONPATH to the
`src` directory). No third-party packages: the CDP client is a plain
socket, because the handshake is one HTTP upgrade and the frames after
it are a masked length prefix.
"""

from __future__ import annotations

import json
import os
import sys


def _add_browser_src_to_path() -> None:
    """Ensure `import browser` resolves from the repo src layout."""
    env_path = os.environ.get("BROWSER_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


def main() -> None:
    _add_browser_src_to_path()
    from browser import Browser, BrowserError  # noqa: E402

    # One browser for the server's lifetime, not one per call. A lane
    # that spawned a fresh Chrome per tool call would leave a navigate
    # and the screenshot that follows it looking at different pages,
    # which is the one thing this lane exists to prevent.
    browser = Browser()

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
            respond(req_id, {})
            continue

        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "browser", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "browser_navigate",
                        "description": (
                            "Open a URL or a file path in a real Chrome and answer "
                            "the page's title. Use it to render something a "
                            "screenshot or a read needs. The browser persists "
                            "across calls, so a navigate and the screenshot that "
                            "follows it see the same page."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": {"type": "string"},
                                "wait_ms": {"type": "integer", "default": 400},
                            },
                            "required": ["url"],
                        },
                    },
                    {
                        "name": "browser_screenshot",
                        "description": (
                            "Capture the current page as a PNG and answer the path "
                            "and its size in bytes. The image is written to disk, "
                            "never returned as text \u2014 its path is what the "
                            "caller hands to a vision model."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"},
                                "width": {"type": "integer", "default": 1280},
                                "height": {"type": "integer", "default": 1600},
                            },
                            "required": ["path"],
                        },
                    },
                    {
                        "name": "browser_read",
                        "description": (
                            "The text of the current page, or of one CSS selector "
                            "within it. Use it to read what a page actually says "
                            "rather than what it was expected to say."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "selector": {"type": "string", "default": "body"},
                            },
                        },
                    },
                ]
            })
        elif method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments", {}) or {}
            try:
                if name == "browser_navigate":
                    url = arguments.get("url", "")
                    if not url:
                        raise BrowserError("`url` is required")
                    if not url.startswith(("http://", "https://", "file://")):
                        url = "file://" + os.path.abspath(url)
                    title = browser.navigate(url, int(arguments.get("wait_ms", 400)))
                    text = json.dumps({"navigated": url, "title": title})
                elif name == "browser_screenshot":
                    path = arguments.get("path", "")
                    if not path:
                        raise BrowserError("`path` is required")
                    written = browser.screenshot(
                        path,
                        int(arguments.get("width", 1280)),
                        int(arguments.get("height", 1600)),
                    )
                    text = json.dumps({"path": written, "bytes": os.path.getsize(written)})
                elif name == "browser_read":
                    text = json.dumps({
                        "text": browser.read_dom(arguments.get("selector", "body")),
                    })
                else:
                    raise BrowserError(f"no tool named `{name}`")
                respond(req_id, {
                    "content": [{"type": "text", "text": text}],
                    "isError": False,
                })
            except BrowserError as exc:
                respond(req_id, {
                    "content": [{"type": "text", "text": f"error: {exc}"}],
                    "isError": True,
                })
            except Exception as exc:  # noqa: BLE001  (a lane never crashes the run)
                respond(req_id, {
                    "content": [{"type": "text", "text": f"error: {type(exc).__name__}: {exc}"}],
                    "isError": True,
                })
        else:
            error(req_id, -32601, f"no method named `{method}`")


if __name__ == "__main__":
    main()
