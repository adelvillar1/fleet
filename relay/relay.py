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

The device session's secret gates the phone's side too: a phone that
cannot name it gets a 403 and learns nothing else.

Threat model, stated plainly: the relay sees frame plaintext — a task's
text, a seat's summary, a transcript. The secret gates the session, so an
unpaired relay cannot stitch a phone to a device, but a compromised relay
reads what crosses it. End-to-end encryption is a later increment; this
server does not pretend to be it.

    python3 relay.py --port 8421
"""

from __future__ import annotations

import argparse
import json
import queue
import threading
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse

# How long a poll is held before it answers empty. The client's own
# timeout is 40 seconds, so a hold under it means every poll is answered
# by the relay rather than cut off by the client — no error path, no
# rejoin.
POLL_SECONDS = 25

# How long a phone's request is held waiting for the device's answer. A
# device that has gone away must not leave a phone hanging forever.
REQUEST_SECONDS = 65


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


DEVICES: dict[str, Device] = {}
DEVICES_LOCK = threading.Lock()


def device_for(sid: str) -> Device | None:
    with DEVICES_LOCK:
        return DEVICES.get(sid)


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
        else:
            self._reply(404, {"error": "no route"})

    def do_GET(self) -> None:  # noqa: N802 - stdlib shape
        parsed = urlparse(self.path)
        if parsed.path == "/poll":
            self._poll(parsed)
        elif parsed.path.startswith("/s/"):
            self._phone(parsed)
        else:
            self._reply(404, {"error": "no route"})

    # ------------------------------------------------- the device's verbs

    def _join(self) -> None:
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

        # The phone's request, whole, becomes a frame. The body is read
        # before the frame is queued: a POST's body is part of the request
        # the device is being asked to answer.
        body = self._body()
        request_id = uuid.uuid4().hex
        frame = {
            "id": request_id,
            "method": self.command,
            "path": path + (f"?{parsed.query}" if parsed.query else ""),
            "headers": [
                [name, value] for name, value in self.headers.items()
                if name.lower() not in ("host", "content-length")
            ],
            "body": body.decode("utf-8", "replace"),
        }
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
        # the device serves the page the phone sees.
        payload = answer.get("body", "")
        raw = payload.encode() if isinstance(payload, str) else b""
        self.send_response(int(answer.get("status") or 200))
        sent_length = False
        for name, value in answer.get("headers") or []:
            if name.lower() == "content-length":
                sent_length = True
            self.send_header(name, value)
        if not sent_length:
            self.send_header("content-length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)


def main() -> None:
    parser = argparse.ArgumentParser(description="The reference Fleet relay.")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8421)
    args = parser.parse_args()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"relay listening on http://{args.host}:{args.port}")
    print("fleet joins it with `fleet gui --remote --relay http://{host}:{port}`".format(
        host=args.host, port=args.port))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
