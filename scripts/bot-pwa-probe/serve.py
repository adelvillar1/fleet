#!/usr/bin/env python3
"""A scratch mirror of the fleet server, for measuring the bot PWA.

It serves BOTH faces on ONE origin — the shape that lets the console's
worker and the bot's be registered side by side — reading the assets live
from the repo so an edit is measured without a build:

  /                     the console shell (index.html, registers /sw.js at scope /)
  /sw.js /app.css /app.js /manifest.webmanifest /icon-*.png
  /bot/                 the bot shell (the installed app's start_url)
  /bot/bot.css /bot/bot.js   the bot's two versioned assets, under the scope
  /bot/bot-sw.js        the bot worker
  /bot.css /bot.js /bot-sw.js /bot.html   the pre-fix paths, kept so the
                        same rig can measure the "before" state
  /api/...              fixture documents shaped like the bridge's
  /bump?to=<hash>       change the served build hash in place (the update cell)

__FLEET_ASSET_HASH__ is substituted exactly as the Rust server does at
serve time, and the headers mirror it too: no-cache on the worker,
immutable on the build's asset URLs, Service-Worker-Allowed per route.

    python3 scripts/bot-pwa-probe/serve.py --port 3196 --hash h1
"""
import argparse
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

ASSETS = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "src", "gui", "assets")
)
STATE = {"hash": "h1"}

INBOX = {
    "bound": 50,
    "sessions": [
        {
            "id": "s1",
            "personas": ["atlas"],
            "state": "active",
            "goal": "ship the bot PWA",
            "last": "the composer is on screen now",
            "at": 42,
        },
        {
            "id": "s2",
            "personas": ["juno"],
            "state": "finished",
            "goal": "read the release notes",
            "last": "done — nothing to change",
            "at": 900,
        },
    ],
}

# Long enough that the transcript outgrows a phone: the clip only appears
# when .scroll has something to grow with.
CARDS = [
    {"type": "message", "role": "owner", "text": "the composer is off the bottom of my phone"},
    {"type": "message", "text": "measured at 390x844: .scroll grows past the viewport and .dock lands below the fold. One flex rule on .conv fixes it.", "persona": "atlas"},
    {"type": "tool_call", "tool": "measure", "summary": "390x844 -> dock below the fold"},
] + [
    {"type": "message", "text": f"turn {n}: the transcript is the conversation, not the audit, and the composer sits in the dock below it, which is the whole reason the layout rule matters on a phone — a paragraph long enough to be a real turn in a real session.", "persona": "atlas"}
    for n in range(4, 26)
]


def read(name):
    with open(os.path.join(ASSETS, name), "r", encoding="utf-8") as handle:
        return handle.read()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, format, *args):
        pass

    def send(self, status, ctype, body, extra=()):
        if isinstance(body, str):
            body = body.encode()
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for name, value in extra:
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(body)

    def versioned(self, name, ctype, query):
        body = read(name)
        if query.get("v", [None])[0] == STATE["hash"]:
            self.send(200, ctype, body, [("Cache-Control", "public, max-age=31536000, immutable")])
        else:
            self.send(200, ctype, body, [("Cache-Control", "no-cache")])

    def worker(self, name, allowed):
        body = read(name).replace("__FLEET_ASSET_HASH__", STATE["hash"])
        self.send(200, "text/javascript; charset=utf-8", body, [
            ("Cache-Control", "no-cache"),
            ("Service-Worker-Allowed", allowed),
            ("X-Content-Type-Options", "nosniff"),
        ])

    def do_GET(self):
        url = urlparse(self.path)
        path = url.path
        query = parse_qs(url.query)
        try:
            if path in ("/", "/console"):
                body = read("index.html").replace("__FLEET_ASSET_HASH__", STATE["hash"])
                return self.send(200, "text/html; charset=utf-8", body, [("Cache-Control", "no-cache")])
            if path == "/sw.js":
                return self.worker("sw.js", "/")
            if path == "/manifest.webmanifest":
                return self.send(200, "application/manifest+json", read("manifest.webmanifest"), [("Cache-Control", "no-cache")])
            if path in ("/app.css", "/app.js"):
                return self.versioned(path[1:], "text/css; charset=utf-8" if path.endswith("css") else "text/javascript; charset=utf-8", query)
            if path in ("/bot.html", "/bot/", "/bot"):
                body = read("bot.html").replace("__FLEET_ASSET_HASH__", STATE["hash"])
                return self.send(200, "text/html; charset=utf-8", body, [("Cache-Control", "no-cache")])
            if path in ("/bot.css", "/bot/bot.css"):
                return self.versioned("bot.css", "text/css; charset=utf-8", query)
            if path in ("/bot.js", "/bot/bot.js"):
                return self.versioned("bot.js", "text/javascript; charset=utf-8", query)
            if path == "/bot-sw.js":
                return self.worker("bot-sw.js", "/")
            if path == "/bot/bot-sw.js":
                return self.worker("bot-sw.js", "/bot/")
            if path == "/bot.webmanifest":
                return self.send(200, "application/manifest+json", read("bot.webmanifest"), [("Cache-Control", "no-cache")])
            if path.startswith("/bot-icon-") or path.startswith("/icon-"):
                with open(os.path.join(ASSETS, os.path.basename(path)), "rb") as handle:
                    return self.send(200, "image/png", handle.read())
            if path == "/api/inbox":
                return self.send(200, "application/json", json.dumps(INBOX))
            if path == "/api/session":
                return self.send(200, "application/json", json.dumps({"id": "s1", "cards": CARDS}))
            if path == "/api/state":
                return self.send(200, "application/json", json.dumps({"running": False, "personas": ["atlas"], "goal": "ship the bot PWA"}))
            if path == "/api/sessions":
                return self.send(200, "application/json", json.dumps({"sessions": INBOX["sessions"]}))
            if path == "/events":
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Cache-Control", "no-cache")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            if path == "/health":
                return self.send(200, "text/plain", STATE["hash"])
            if path == "/bump":
                STATE["hash"] = query.get("to", ["h2"])[0]
                return self.send(200, "text/plain", STATE["hash"])
            return self.send(404, "text/plain", "not found: " + path)
        except BrokenPipeError:
            pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        self.rfile.read(length)
        self.send(200, "application/json", json.dumps({"ok": True}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=3196)
    parser.add_argument("--hash", default="h1")
    args = parser.parse_args()
    STATE["hash"] = args.hash
    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print(f"serving {ASSETS} on 127.0.0.1:{args.port} hash={args.hash}", flush=True)
    server.serve_forever()
