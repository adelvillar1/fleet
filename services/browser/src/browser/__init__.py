"""browser: a real browser, driven over the DevTools Protocol.

The lane exists because a deck is HTML, and nothing in a static Rust
binary rasterizes HTML. A browser does. This module is the client half:
it finds Chrome, launches it once with a debugging port, and speaks CDP
over a plain socket — no Playwright, no websocket library, because the
handshake is one HTTP upgrade and the frames after it are a masked
length prefix.

House rule, inherited from the lanes: a capability the machine must
supply is named, not assumed. Chrome's location rides `BROWSER_CHROME`;
when it is unset the usual suspects are probed in order and the refusal
names what it looked for.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import socket
import subprocess
import time
import urllib.request

# The port the lane's Chrome listens on. Fixed rather than picked: a
# second fleet process would fight over a random one, and a fixed port
# that is taken is a named refusal rather than a silent second browser.
CDP_PORT = 9333

# How long to wait for Chrome's debugging endpoint to answer after
# launch. A cold start on a loaded machine is slow; a port that never
# answers is a refusal, not a hang.
LAUNCH_TIMEOUT_S = 15.0

# How long any one CDP call waits for its result.
CALL_TIMEOUT_S = 60.0


class BrowserError(RuntimeError):
    """One named refusal. Every failure path raises this."""


def find_chrome() -> str:
    """The Chrome binary, from `BROWSER_CHROME` or the usual places."""
    override = os.environ.get("BROWSER_CHROME")
    if override:
        if os.path.isfile(override) and os.access(override, os.X_OK):
            return override
        raise BrowserError(
            f"BROWSER_CHROME={override} is not an executable file"
        )
    for name in (
        "google-chrome-stable",
        "google-chrome",
        "chromium",
        "chromium-browser",
    ):
        found = shutil.which(name)
        if found:
            return found
    # The puppeteer cache, where this machine's headless shells live. The
    # cache holds one directory per version and `os.walk` visits them in
    # directory order, which is not version order — an old shell picked
    # over a new one segfaults on flags the new one accepts. So the
    # candidates are collected and the newest version wins.
    cache = os.path.expanduser("~/.cache/puppeteer")
    let_versions: list[tuple[tuple[int, ...], str]] = []
    if os.path.isdir(cache):
        for root, _dirs, files in os.walk(cache):
            for name in files:
                if not name.startswith("chrome-headless-shell"):
                    continue
                candidate = os.path.join(root, name)
                if not os.access(candidate, os.X_OK):
                    continue
                # .../mac_arm-143.0.7499.169/chrome-headless-shell-mac-arm64/<bin>
                version = re.findall(r"\d+", os.path.basename(os.path.dirname(root)))
                key = tuple(int(part) for part in version) if version else (0,)
                let_versions.append((key, candidate))
    if let_versions:
        let_versions.sort(key=lambda entry: entry[0])
        return let_versions[-1][1]
    raise BrowserError(
        "no Chrome found \u2014 set BROWSER_CHROME to the binary; looked for "
        "google-chrome-stable, google-chrome, chromium, chromium-browser on "
        "PATH and under ~/.cache/puppeteer"
    )


class Browser:
    """One Chrome, one tab, and the CDP calls this lane needs."""

    def __init__(self) -> None:
        self.chrome = find_chrome()
        self.process: subprocess.Popen | None = None
        self.sock: socket.socket | None = None
        self.session_id: str | None = None
        self.next_id = 0
        self._buf = b""

    # --- lifecycle ---------------------------------------------------------

    def start(self) -> None:
        if self.sock is not None:
            return
        if not self._endpoint_answers():
            profile = os.path.join(
                os.environ.get("TMPDIR", "/tmp"), f"fleet-browser-{os.getpid()}"
            )
            self.process = subprocess.Popen(
                [
                    self.chrome,
                    f"--remote-debugging-port={CDP_PORT}",
                    "--headless=new",
                    "--no-sandbox",
                    "--disable-gpu",
                    "--disable-dev-shm-usage",
                    "--no-first-run",
                    "--no-default-browser-check",
                    f"--user-data-dir={profile}",
                    "about:blank",
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            deadline = time.time() + LAUNCH_TIMEOUT_S
            while time.time() < deadline:
                if self._endpoint_answers():
                    break
                if self.process.poll() is not None:
                    raise BrowserError(
                        f"Chrome exited during launch with status "
                        f"{self.process.returncode}"
                    )
                time.sleep(0.2)
            else:
                raise BrowserError(
                    f"Chrome did not answer on port {CDP_PORT} within "
                    f"{LAUNCH_TIMEOUT_S:.0f}s"
                )
        self._connect()

    def stop(self) -> None:
        try:
            if self.sock is not None:
                self.sock.close()
        except OSError:
            pass
        self.sock = None
        self.session_id = None
        if self.process is not None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None

    def __enter__(self) -> "Browser":
        self.start()
        return self

    def __exit__(self, *_exc) -> None:
        self.stop()

    def _endpoint_answers(self) -> bool:
        url = f"http://127.0.0.1:{CDP_PORT}/json/version"
        try:
            with urllib.request.urlopen(url, timeout=1) as reply:
                return reply.status == 200
        except Exception:
            return False

    def _connect(self) -> None:
        """Attach to a tab: find its websocket, then speak CDP on it."""
        with urllib.request.urlopen(
            f"http://127.0.0.1:{CDP_PORT}/json/list", timeout=5
        ) as reply:
            targets = json.loads(reply.read().decode("utf-8"))
        page = next(
            (t for t in targets if t.get("type") == "page" and t.get("webSocketDebuggerUrl")),
            None,
        )
        if page is None:
            target = next(
                (t for t in targets if t.get("webSocketDebuggerUrl")), None
            )
            if target is None:
                raise BrowserError("Chrome exposed no debuggable target")
            page = target
        ws = page["webSocketDebuggerUrl"]
        # ws://127.0.0.1:PORT/devtools/page/<id> \u2014 the host and path are
        # all a plain socket needs.
        rest = ws[len("ws://"):]
        host, _, path = rest.partition("/")
        host_name, _, port = host.partition(":")
        self.sock = socket.create_connection((host_name, int(port or CDP_PORT)), timeout=10)
        self.sock.settimeout(CALL_TIMEOUT_S)
        self._handshake(host, f"/{path}")

    def _handshake(self, host: str, path: str) -> None:
        """The one HTTP upgrade, then CDP's attach to the page target."""
        key = "dGhlIHNhbXBsZSBub25jZQ=="  # the RFC's own example key
        request = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: {host}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n"
            "\r\n"
        )
        assert self.sock is not None
        self.sock.sendall(request.encode("ascii"))
        header = b""
        while b"\r\n\r\n" not in header:
            chunk = self.sock.recv(1)
            if not chunk:
                raise BrowserError("Chrome closed the socket during the handshake")
            header += chunk
        if b"101" not in header.split(b"\r\n", 1)[0]:
            raise BrowserError(
                "Chrome refused the websocket upgrade: "
                + header.split(b"\r\n", 1)[0].decode("latin-1")
            )

    # --- the wire ----------------------------------------------------------

    def _send(self, payload: dict) -> None:
        data = json.dumps(payload).encode("utf-8")
        assert self.sock is not None
        header = bytearray()
        header.append(0x81)  # FIN + text frame
        mask_bit = 0x80
        length = len(data)
        if length < 126:
            header.append(mask_bit | length)
        elif length < 65536:
            header.append(mask_bit | 126)
            header.extend(length.to_bytes(2, "big"))
        else:
            header.append(mask_bit | 127)
            header.extend(length.to_bytes(8, "big"))
        # A client's frames are masked, per the RFC, and Chrome requires it.
        mask = os.urandom(4)
        header.extend(mask)
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
        self.sock.sendall(bytes(header) + masked)

    def _recv_exact(self, count: int) -> bytes:
        assert self.sock is not None
        while len(self._buf) < count:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise BrowserError("Chrome closed the socket mid-frame")
            self._buf += chunk
        out, self._buf = self._buf[:count], self._buf[count:]
        return out

    def _recv_frame(self) -> str:
        first = self._recv_exact(2)
        length = first[1] & 0x7F
        if length == 126:
            length = int.from_bytes(self._recv_exact(2), "big")
        elif length == 127:
            length = int.from_bytes(self._recv_exact(8), "big")
        return self._recv_exact(length).decode("utf-8")

    def call(self, method: str, params: dict | None = None) -> dict:
        """One CDP call, answered by its id. Events ride past."""
        if self.sock is None:
            self.start()
        self.next_id += 1
        call_id = self.next_id
        self._send({"id": call_id, "method": method, "params": params or {}})
        while True:
            frame = json.loads(self._recv_frame())
            if frame.get("id") == call_id:
                if "error" in frame:
                    raise BrowserError(f"{method}: {frame['error']}")
                return frame.get("result", {})
            # Anything else is an event this lane does not wait on.

    # --- the actions this lane declares -------------------------------------

    def navigate(self, url: str, wait_ms: int = 400) -> str:
        """Load `url` and answer the page's title once it settles."""
        self.call("Page.enable")
        self.call("Page.navigate", {"url": url})
        time.sleep(max(wait_ms, 0) / 1000.0)
        result = self.call(
            "Runtime.evaluate",
            {
                "expression": "document.title",
                "returnByValue": True,
            },
        )
        title = (
            result.get("result", {}).get("value")
            if isinstance(result.get("result"), dict)
            else None
        )
        return title if isinstance(title, str) else ""

    def read_dom(self, selector: str = "body") -> str:
        """The text of one element, or of the whole page."""
        expression = (
            f"(() => {{ const el = document.querySelector({json.dumps(selector)}); "
            "return el ? el.innerText : ''; })()"
        )
        result = self.call(
            "Runtime.evaluate",
            {"expression": expression, "returnByValue": True},
        )
        value = (
            result.get("result", {}).get("value")
            if isinstance(result.get("result"), dict)
            else None
        )
        return value if isinstance(value, str) else ""

    def screenshot(self, path: str, width: int = 1280, height: int = 1600) -> str:
        """Capture the page to `path` and answer the path and its bytes."""
        self.call("Emulation.setDeviceMetricsOverride", {
            "width": width,
            "height": height,
            "deviceScaleFactor": 1,
            "mobile": False,
        })
        result = self.call("Page.captureScreenshot", {"format": "png"})
        data = result.get("data")
        if not data:
            raise BrowserError("Chrome returned no screenshot data")
        import base64

        raw = base64.b64decode(data)
        os.makedirs(os.path.dirname(os.path.abspath(path)) or ".", exist_ok=True)
        with open(path, "wb") as handle:
            handle.write(raw)
        return path
