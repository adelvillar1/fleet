#!/usr/bin/env python3
"""The reference relay: a dumb switchboard between a phone and a Fleet.

Fleet dials out and holds a long-poll; a phone's HTTP requests arrive as
frames on that poll; Fleet's answers come back as frames and are written
to the phone's held-open request. The relay never parses a request or an
answer — it is a map of pending request ids and two queues, which is why
this is a hundred lines of anything.

Three verbs the device speaks:

    POST /join   {sid, hash}            register a device session
    GET  /poll?sid=<sid>                long-held; returns the phone's frames
    POST /frame?sid=<sid>  {id, ...}    one answer, against a frame's id

And the phone speaks one:

    ANY  /s/<sid>/<path>                forwarded to the device; the reply
                                        is held until the device answers

The dedicated-relay shape — the one this deployment is — also forwards
the origin root to the single joined device:

    ANY  /<path>                        forwarded to the device, so the
                                        face's shell, manifest, and worker
                                        resolve at the origin root and the
                                        PWA installs

The root is gated per path. A browser navigation cannot present headers,
so the shell's own fetches — the page, its assets, the manifest, the
worker, the icons, the pairing page, and the pairing redemption — pass
without the secret. Everything else carries `x-fleet-relay: <secret>`;
a request without it is refused before it becomes a frame.

The device session's secret gates the phone's side too: a phone that
cannot name it gets a 403 and learns nothing else.

Threat model, stated plainly: the relay sees metadata — paths, frame
ids, sizes — and ciphertext for everything content-bearing. Frames the
phone marked `x-fleet-enc: 1` carry bodies sealed end to end with a key
from an X25519 exchange the relay only watched: it holds no scalar, so
it cannot derive the key, and its capture is opaque. The shell's own
plaintext frames are public code. The secret gates the session, so an
unpaired relay cannot stitch a phone to a device; what a relay can do
is refuse to carry frames, which is availability, not confidentiality.

    python3 relay.py --port 8421
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import queue
import threading
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse

# How long a poll is held before it answers empty. The client's own
# timeout is 40 seconds, so a hold under it means every poll is answered
# by the relay rather than cut off by the client — no error path, no
# rejoin. It also has to sit under the ceilings public edges put on held
# connections, which is the same bound doing double duty.
POLL_SECONDS = 25

# How long a phone's request is held waiting for the device's answer. A
# device that has gone away must not leave a phone hanging forever.
REQUEST_SECONDS = 65

# How long a device session lives without a poll. A face that stopped —
# killed, redeployed, asleep — held its relay session forever otherwise,
# and a root that sees two sessions cannot tell which one is real. Three
# poll holds and a margin: a live face polls every POLL_SECONDS.
DEVICE_TTL_SECONDS = 90

# The headers a root-mode phone presents. `x-fleet-relay` is the gate:
# the session secret, compared against the joined device's. `x-fleet-enc`
# marks a body sealed end to end, which the relay carries verbatim and
# echoes back on the answer. Both are consumed here — they are relay
# transport, and the device has no use for either.
RELAY_HEADER = "x-fleet-relay"
ENC_HEADER = "x-fleet-enc"
STRIPPED_HEADERS = ("host", "content-length", RELAY_HEADER, ENC_HEADER)

# The root paths a browser fetches on its own — no headers, no seal.
# Everything here is public code or the one-time pairing form.
ROOT_OPEN_GET = frozenset({
    "/",
    "/app.css",
    "/app.js",
    "/manifest.webmanifest",
    "/sw.js",
    "/icon-192.png",
    "/icon-512.png",
    "/apple-touch-icon.png",
    "/bot.css",
    "/bot.js",
    "/bot.webmanifest",
    "/bot-sw.js",
    "/bot-icon-192.png",
    "/bot-icon-512.png",
    "/pair",
})
ROOT_OPEN_POST = frozenset({"/api/pair"})


class Device:
    """One dialed-out device: its secret, its frame queue, and the
    requests its phone is waiting on."""

    def __init__(self, sid: str, secret: str) -> None:
        self.sid = sid
        self.secret = secret
        self.frames: queue.Queue[dict] = queue.Queue()
        self.answers: dict[str, queue.Queue] = {}
        self.lock = threading.Lock()
        self.joined_at = time.time()
        self.last_poll = time.time()


DEVICES: dict[str, Device] = {}
DEVICES_LOCK = threading.Lock()


def device_for(sid: str) -> Device | None:
    with DEVICES_LOCK:
        return DEVICES.get(sid)


def prune_stale_devices() -> None:
    """Drop the sessions that stopped polling. A face that died held
    its session forever otherwise, and the root's choices rotted under
    the corpses."""
    cutoff = time.time() - DEVICE_TTL_SECONDS
    with DEVICES_LOCK:
        for sid in [sid for sid, device in DEVICES.items()
                    if device.last_poll < cutoff]:
            del DEVICES[sid]


def live_devices() -> list[Device]:
    with DEVICES_LOCK:
        return sorted(DEVICES.values(), key=lambda device: -device.last_poll)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt: str, *args) -> None:  # noqa: A002 - stdlib shape
        # Quiet by default: a relay's log is a relay's traffic, and the
        # owner running one does not want a scroll of it.
        pass

    # ------------------------------------------------------------ reading

    def _body(self) -> bytes:
        length = int(self.headers.get("content-length") or 0)
        return self.rfile.read(length) if length else b""

    def _json(self) -> dict:
        try:
            return json.loads(self._body() or b"{}")
        except json.JSONDecodeError:
            return {}

    def _reply(self, status: int, payload: dict, headers: dict | None = None) -> None:
        body = (json.dumps(payload) + "\n").encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        for name, value in (headers or {}).items():
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(body)

    def _secret_ok(self, device: Device, presented: str) -> bool:
        # Constant-time enough for a secret compared over HTTP: the
        # comparison is not the interesting side channel here.
        return bool(presented) and presented == device.secret

    # ------------------------------------------------------------- routes

    def do_POST(self) -> None:  # noqa: N802 - stdlib shape
        parsed = urlparse(self.path)
        if parsed.path == "/join":
            self._join()
        elif parsed.path == "/frame":
            self._frame(parsed)
        elif parsed.path.startswith("/s/"):
            # The phone's verb is ANY, as the module says — and a run, a
            # plan, and a seat's answer are all POSTs. Routing only GET
            # here left a phone able to read the face through the relay
            # and unable to say anything through it.
            self._phone(parsed)
        else:
            self._root(parsed)

    def do_GET(self) -> None:  # noqa: N802 - stdlib shape
        parsed = urlparse(self.path)
        if parsed.path == "/poll":
            self._poll(parsed)
        elif parsed.path == "/healthz":
            prune_stale_devices()
            self._reply(200, {"ok": True, "devices": len(DEVICES)})
        elif parsed.path.startswith("/s/"):
            self._phone(parsed)
        else:
            self._root(parsed)

    # The phone's verbs beyond GET: a run and a plan are POSTs and a
    # seat's answer is a POST with a smaller body. Only the phone's path
    # is forwarded; the device's own frames still have their two verbs.
    do_PUT = do_POST
    do_DELETE = do_POST

    # ------------------------------------------------- the device's verbs

    def _join(self) -> None:
        prune_stale_devices()
        body = self._json()
        sid = (body.get("sid") or "").strip()
        secret = (body.get("hash") or "").strip()
        if not sid or not secret:
            self._reply(400, {"error": "a join names a sid and a hash"})
            return
        with DEVICES_LOCK:
            existing = DEVICES.get(sid)
            if existing is not None and existing.secret != secret:
                self._reply(403, {"error": "that session is taken"})
                return
            device = existing or Device(sid, secret)
            DEVICES[sid] = device
        self._reply(200, {"joined": sid})

    def _poll(self, parsed) -> None:
        from urllib.parse import parse_qs

        sid = (parse_qs(parsed.query).get("sid") or [""])[0]
        device = device_for(sid)
        if device is None:
            self._reply(404, {"error": "no such session"})
            return
        device.last_poll = time.time()
        # Hold the poll: the frames arrive from the phone's side, which is
        # a different thread, so the queue is where they meet.
        frames = []
        try:
            first = device.frames.get(timeout=POLL_SECONDS)
            frames.append(first)
            while True:
                frames.append(device.frames.get_nowait())
        except queue.Empty:
            pass
        # A bare array: the client reads the body as the frame list
        # itself, so a wrapper object would be a shape mismatch.
        self._reply(200, frames)

    def _frame(self, parsed) -> None:
        from urllib.parse import parse_qs

        sid = (parse_qs(parsed.query).get("sid") or [""])[0]
        device = device_for(sid)
        if device is None:
            self._reply(404, {"error": "no such session"})
            return
        answer = self._json()
        request_id = answer.get("id")
        with device.lock:
            waiting = device.answers.get(request_id)
        if waiting is not None:
            waiting.put(answer)
        self._reply(200, {"delivered": bool(waiting is not None)})

    # ---------------------------------------------------- the phone's verb

    def _phone(self, parsed) -> None:
        from urllib.parse import parse_qs

        parts = parsed.path.split("/", 3)
        if len(parts) < 3 or not parts[2]:
            self._reply(404, {"error": "no route"})
            return
        sid = parts[2]
        # A bare entry URL (the one the QR carries) is the root: the
        # phone that opens it is asking for the face's page.
        path = "/" + (parts[3] if len(parts) > 3 else "")
        device = device_for(sid)
        if device is None:
            self._reply(404, {"error": "that session is not connected"})
            return
        secret = (parse_qs(parsed.query).get("hash") or [""])[0]
        if not self._secret_ok(device, secret):
            self._reply(403, {"error": "that session needs its secret"})
            return
        self._forward(device, parsed, path, self._body())

    # ---------------------------------------------------- the root's verb

    def _root(self, parsed) -> None:
        """The origin root is the device: the shell, its assets, the
        manifest, the worker, and every API call, forwarded to a joined
        device session. Headered paths route by the secret itself — the
        device whose secret matches is the device the phone means — so
        one stale session never stands between a phone and its fleet.
        The browser-forced fetches cannot present a secret; they go to
        the most recently polling device, which is the live one."""
        prune_stale_devices()
        bare = parsed.path.split("?")[0] or "/"
        method = self.command
        open_path = (method == "GET" and bare in ROOT_OPEN_GET) or (
            method == "POST" and bare in ROOT_OPEN_POST
        )
        presented = self.headers.get(RELAY_HEADER) or ""
        devices = live_devices()
        if presented:
            matches = [device for device in devices
                       if self._secret_ok(device, presented)]
            if not matches:
                self._reply(403, {"error": "that origin needs its relay secret"})
                return
            device = matches[0]
        elif open_path:
            if not devices:
                self._reply(
                    503,
                    {"error": "no device is joined; fleet dials this relay"},
                )
                return
            device = devices[0]
        else:
            self._reply(403, {"error": "that origin needs its relay secret"})
            return
        # A device that has stopped polling is a face that restarted or
        # died. Holding the phone's request the full answer timeout made
        # every restart a visible hang (KI-371's third lesson); a named,
        # instant answer is something a reload can act on.
        if time.time() - device.last_poll > 40:
            self._reply(
                503,
                {"error": "fleet is restarting — the device is not polling; try again in a moment"},
            )
            return
        enc = 1 if self.headers.get(ENC_HEADER) == "1" else None
        self._forward(device, parsed, parsed.path, self._body(), enc)

    # --------------------------------------------------------- the frame

    def _forward(
        self,
        device: Device,
        parsed,
        path: str,
        body: bytes,
        enc: int | None = None,
    ) -> None:
        """One phone request, whole, becomes a frame and waits for the
        device's answer. Shared by the per-session path and the root:
        the gate already happened, and the frame shape is the same."""
        request_id = uuid.uuid4().hex
        forwarded = [
            [name, value]
            for name, value in self.headers.items()
            if name.lower() not in STRIPPED_HEADERS
        ]
        # One fact the device cannot derive and needs: this request
        # arrived through a relay. A pairing redemption uses it to hand
        # the phone its relay secret (the relay already holds this
        # secret at join, so nothing crosses that did not already
        # cross); a socket redemption must never carry one.
        forwarded.append(["x-fleet-via", "relay"])
        frame = {
            "id": request_id,
            "method": self.command,
            "path": path + (f"?{parsed.query}" if parsed.query else ""),
            "headers": forwarded,
            "body": body.decode("utf-8", "replace"),
        }
        if enc is not None:
            frame["enc"] = enc
        waiting: queue.Queue = queue.Queue()
        with device.lock:
            device.answers[request_id] = waiting
        device.frames.put(frame)
        try:
            answer = waiting.get(timeout=REQUEST_SECONDS)
        except queue.Empty:
            with device.lock:
                device.answers.pop(request_id, None)
            self._reply(504, {"error": "the device did not answer in time"})
            return
        with device.lock:
            device.answers.pop(request_id, None)

        # The device's answer, whole, is the phone's reply. The headers it
        # chose are the ones the phone gets, which is what makes the page
        # the device serves the page the phone sees — plus the two facts
        # the phone needs to open a sealed body: that it is sealed, and
        # which frame it answered. An answer marked `x-fleet-b64` carries
        # bytes (an icon) the text frame cannot: decode here, strip the
        # marker, and the phone's browser sees the bytes it asked for.
        payload = answer.get("body", "")
        reply_headers = [
            (name, value) for name, value in answer.get("headers") or []
        ]
        if any(name.lower() == "x-fleet-b64" for name, _ in reply_headers):
            reply_headers = [
                (name, value) for name, value in reply_headers
                if name.lower() != "x-fleet-b64"
            ]
            raw = base64.b64decode(payload) if payload else b""
        else:
            raw = payload.encode() if isinstance(payload, str) else b""
        self.send_response(int(answer.get("status") or 200))
        sent_length = False
        for name, value in reply_headers:
            if name.lower() == "content-length":
                sent_length = True
            self.send_header(name, value)
        if answer.get("enc") == 1:
            self.send_header(ENC_HEADER, "1")
        self.send_header("x-fleet-frame", str(answer.get("id") or request_id))
        if not sent_length:
            self.send_header("content-length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)


def main() -> None:
    parser = argparse.ArgumentParser(description="The reference Fleet relay.")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument(
        "--port",
        type=int,
        # A container is told its port from the outside; the environment
        # is where a platform says so.
        default=int(os.environ.get("PORT") or 8421),
    )
    args = parser.parse_args()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    # What actually bound: port 0 means the operating system picked, and
    # the caller (a test, a launcher) reads the banner for the truth.
    bound_port = server.server_address[1]
    print(f"relay listening on http://{args.host}:{bound_port}")
    print("fleet joins it with `fleet gui --remote --relay http://{host}:{port}`".format(
        host=args.host, port=args.port
    ))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
