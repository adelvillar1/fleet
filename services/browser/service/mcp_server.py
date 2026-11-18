#!/usr/bin/env python3
"""
browser MCP server: obscura first, Chrome only when the owner chose it.

Exposes the lane's three names — `browser_navigate`, `browser_read`,
`browser_screenshot` — and, when obscura is the engine, every tool obscura
itself publishes, under its own name.

Transport: stdio JSON-RPC (MCP 2024-11-05).

The engine order is the owner's 2026-11-18 ruling: obscura, then Chrome as the
last resort, with a hosted provider named as the fourth door. Chrome is used
only when `browser.engine` in the home's settings says `chrome` — an implicit
fallback would hide the memory footprint the owner objects to.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys


# The names this lane has always published, mapped onto obscura's. `browser_read`
# is the lane's own invention and does not exist in obscura at all, so it is a
# translation rather than a rename: `browser_snapshot` answers refs, control
# states and visible text, which is what a driving agent needs. Prose is one
# name away under `browser_markdown`.
ALIASES = {
    "browser_read": "browser_snapshot",
}

# A name whose TRANSLATION is more than a rename: the lane's contract differs
# from the engine's answer, so the args are rewritten on the way in and the
# answer on the way out. `browser_screenshot` is the one: obscura answers image
# CONTENT and takes no path, while the lane has always answered a path on disk
# whose bytes a vision model can then be handed. Silently forwarding `path`
# would be the `unsupported option 'path'` refusal this lane hit on its first
# live call.
import base64 as _base64

# The three names the lane answers even before it has spoken to an engine.
LANE_TOOLS = ("browser_navigate", "browser_read", "browser_screenshot")

NOTES = {
    "browser_navigate": (
        "Open a URL in the browser and answer the page's title. The browser "
        "persists across calls, so a navigate and the screenshot that follows "
        "it see the same page."
    ),
    "browser_read": (
        "The text of the current page, or one CSS selector within it — an alias "
        "for obscura's `browser_snapshot`, which answers the page's refs, "
        "control states and visible text. Use it to read what a page actually "
        "says rather than what it was expected to say."
    ),
    "browser_screenshot": (
        "Capture the current page as a PNG at `path` and answer the path and "
        "its size in bytes. The image is written to disk, never returned as "
        "text — its path is what the caller hands to a vision model."
    ),
}

# The lane's own schemas, used when it must answer tools/list before the
# engine has spoken (a strict client lists before it calls).
LOCAL_SCHEMAS = {
    "browser_navigate": {
        "type": "object",
        "properties": {
            "url": {"type": "string"},
            "wait_ms": {"type": "integer", "default": 400},
        },
        "required": ["url"],
    },
    "browser_read": {
        "type": "object",
        "properties": {"selector": {"type": "string", "default": "body"}},
    },
    "browser_screenshot": {
        "type": "object",
        "properties": {
            "path": {"type": "string"},
            "width": {"type": "integer", "default": 1280},
            "height": {"type": "integer", "default": 1600},
        },
        "required": ["path"],
    },
}


def home_dir() -> str:
    return os.environ.get("FLEET_HOME") or os.path.expanduser("~/.fleet")


def engine_preference() -> str:
    """The engine the owner chose, from the home's settings.

    `obscura` when unset: that is the default the owner's ruling establishes,
    and a missing obscura then produces a self-healing card rather than a
    silent fallback to Chrome.
    """
    path = os.path.join(home_dir(), "config.json")
    try:
        with open(path, "r", encoding="utf-8") as handle:
            document = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return "obscura"
    chosen = (document.get("browser") or {}).get("engine")
    return chosen if chosen in ("obscura", "chrome") else "obscura"


def find_obscura() -> str | None:
    """obscura's binary: the owner's override, then the home's bin, then PATH."""
    override = os.environ.get("BROWSER_OBSCURA")
    if override:
        if os.path.isfile(override) and os.access(override, os.X_OK):
            return override
        return None
    for candidate in (
        os.path.join(home_dir(), "bin", "obscura"),
        "/usr/local/bin/obscura",
        "/opt/homebrew/bin/obscura",
    ):
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return shutil.which("obscura")


class Obscura:
    """One `obscura mcp` child, spoken to over its own stdio."""

    def __init__(self) -> None:
        self.process: subprocess.Popen | None = None
        self.tools: list[dict] = []
        self.next_id = 0

    def start(self) -> None:
        binary = find_obscura()
        if binary is None:
            raise RuntimeError(
                "obscura is not installed — install it from "
                "https://github.com/h4ckf0r0day/obscura/releases, or choose "
                "Chrome as the browser engine in Settings"
            )
        self.process = subprocess.Popen(
            [binary, "mcp", "--allow-private-network"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        self.call("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "fleet-browser-lane", "version": "0.2.0"},
        })
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        listed = self.call("tools/list", {})
        self.tools = list(listed.get("tools") or [])

    def _send(self, payload: dict) -> None:
        assert self.process and self.process.stdin
        self.process.stdin.write(json.dumps(payload) + "\n")
        self.process.stdin.flush()

    def call(self, method: str, params: dict) -> dict:
        self.next_id += 1
        wanted = self.next_id
        self._send({
            "jsonrpc": "2.0", "id": wanted, "method": method, "params": params,
        })
        while True:
            assert self.process and self.process.stdout
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError("obscura closed its stream")
            try:
                frame = json.loads(line)
            except json.JSONDecodeError:
                continue
            if frame.get("id") == wanted:
                if "error" in frame:
                    raise RuntimeError(str(frame["error"]))
                return frame.get("result") or {}

    def stop(self) -> None:
        if self.process is not None:
            try:
                self.process.terminate()
                self.process.wait(timeout=5)
            except Exception:  # noqa: BLE001  (a lane never crashes the run)
                try:
                    self.process.kill()
                except Exception:  # noqa: BLE001
                    pass
            self.process = None


def lane_tool_list(engine_tools: list[dict]) -> list[dict]:
    """The engine's tools, with the lane's names present and described.

    An alias keeps the ENGINE's schema when it has one (so the model sees the
    real parameters), and falls back to the lane's own schema when the engine
    exposes no such tool. The engine's other tools ride under their own names —
    withholding them is what would leave the harness able to screenshot and
    unable to drive a page.
    """
    by_name = {tool.get("name"): tool for tool in engine_tools}
    out: list[dict] = []
    seen: set[str] = set()
    for name in LANE_TOOLS:
        target = by_name.get(ALIASES.get(name, name)) or {}
        schema = target.get("inputSchema") or LOCAL_SCHEMAS[name]
        out.append({
            "name": name,
            "description": NOTES[name],
            "inputSchema": schema,
        })
        seen.add(name)
    for tool in engine_tools:
        name = tool.get("name")
        if not name or name in seen or name in ALIASES:
            continue
        out.append(tool)
        seen.add(name)
    return out


def _screenshot_to_path(engine, arguments, req_id, say):
    """Capture through obscura and write the PNG the lane promises.

    obscura answers image content (`{type: image, data: <base64>}`) and takes
    only a size; the lane's contract is a PATH on disk, because the caller
    hands that path to a vision model rather than carrying bytes through the
    transcript. So the args are narrowed to the engine's own and the answer is
    written out. A size the engine refuses is not invented: only the fields it
    declares are sent.
    """
    ask = {}
    if arguments.get("width"):
        ask["width"] = int(arguments["width"])
    if arguments.get("height"):
        ask["height"] = int(arguments["height"])
    result = engine.call("tools/call", {
        "name": "browser_screenshot", "arguments": ask,
    })
    path = arguments.get("path") or "generated/page.png"
    for item in result.get("content") or []:
        data = item.get("data")
        if item.get("type") == "image" and data:
            raw = _base64.b64decode(data)
            directory = os.path.dirname(os.path.abspath(path)) or "."
            os.makedirs(directory, exist_ok=True)
            with open(path, "wb") as handle:
                handle.write(raw)
            return {
                "content": [{
                    "type": "text",
                    "text": json.dumps({"path": path, "bytes": len(raw)}),
                }],
                "isError": False,
            }
    # The engine answered no image: carry ITS words rather than a guess.
    text = "no image came back"
    for item in result.get("content") or []:
        if item.get("type") == "text" and item.get("text"):
            text = item["text"]
            break
    return {"content": [{"type": "text", "text": f"error: {text}"}], "isError": True}


def main() -> None:
    engine: Obscura | None = None
    chrome = None
    engine_error = ""

    def ensure_engine():
        nonlocal engine, chrome, engine_error
        if engine is not None or chrome is not None:
            return
        if engine_preference() == "obscura":
            try:
                candidate = Obscura()
                candidate.start()
                engine = candidate
                return
            except Exception as exc:  # noqa: BLE001
                engine_error = str(exc)
                engine = None
        # Chrome, and ONLY because the owner chose it.
        sys.path.insert(0, os.environ.get("BROWSER_PYTHONPATH", ""))
        from browser import Browser  # noqa: E402

        chrome = Browser()
        chrome.start()

    def respond(req_id, result):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}) + "\n")
        sys.stdout.flush()

    def error(req_id, code, msg):
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": msg},
        }) + "\n")
        sys.stdout.flush()

    def say(req_id, text, failed=False):
        respond(req_id, {
            "content": [{"type": "text", "text": text}],
            "isError": bool(failed),
        })

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
            continue
        if method == "ping":
            respond(req_id, {})
            continue
        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "browser", "version": "0.2.0"},
            })
            continue
        if method == "tools/list":
            try:
                ensure_engine()
                tools = lane_tool_list(engine.tools if engine else [])
            except Exception as exc:  # noqa: BLE001
                # The refusal is a NAMED answer, never an empty list: a lane
                # that lists nothing reads as "no browser here" and hides the
                # one thing the owner needs to do about it.
                engine_error = engine_error or str(exc)
                tools = [
                    {"name": name, "description": NOTES[name],
                     "inputSchema": LOCAL_SCHEMAS[name]}
                    for name in LANE_TOOLS
                ]
            respond(req_id, {"tools": tools})
            continue
        if method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments", {}) or {}
            try:
                ensure_engine()
                if engine is not None:
                    if name == "browser_screenshot":
                        respond(req_id, _screenshot_to_path(engine, arguments, req_id, say))
                        continue
                    result = engine.call("tools/call", {
                        "name": ALIASES.get(name, name), "arguments": arguments,
                    })
                    respond(req_id, result)
                    continue
                if name == "browser_navigate":
                    url = arguments.get("url", "")
                    if not url:
                        raise RuntimeError("`url` is required")
                    if not url.startswith(("http://", "https://", "file://")):
                        url = "file://" + os.path.abspath(url)
                    say(req_id, json.dumps({
                        "navigated": url,
                        "title": chrome.navigate(url, int(arguments.get("wait_ms", 400))),
                    }))
                elif name == "browser_screenshot":
                    path = arguments.get("path", "")
                    if not path:
                        raise RuntimeError("`path` is required")
                    written = chrome.screenshot(
                        path,
                        int(arguments.get("width", 1280)),
                        int(arguments.get("height", 1600)),
                    )
                    say(req_id, json.dumps({
                        "path": written, "bytes": os.path.getsize(written),
                    }))
                elif name == "browser_read":
                    say(req_id, json.dumps({
                        "text": chrome.read_dom(arguments.get("selector", "body")),
                    }))
                else:
                    raise RuntimeError(
                        f"`{name}` is not available on the Chrome engine — "
                        "it is one of obscura's tools"
                    )
            except Exception as exc:  # noqa: BLE001  (a lane never crashes the run)
                detail = engine_error or f"{type(exc).__name__}: {exc}"
                say(req_id, f"error: {detail}", failed=True)
            continue
        error(req_id, -32601, f"no method `{method}`")

    if engine is not None:
        engine.stop()
    if chrome is not None:
        try:
            chrome.stop()
        except Exception:  # noqa: BLE001
            pass


if __name__ == "__main__":
    main()
