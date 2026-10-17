# browser

The render lane. A real Chrome, driven over the DevTools Protocol.

It exists because a deck is HTML, and nothing inside a static Rust
binary rasterizes HTML. A browser does. The lane is a child process, the
same shape as every other lane: `python3 -u service/mcp_server.py`,
speaking stdio JSON-RPC, launched by `lanes.rs`.

## Tools

- `browser_navigate(url, wait_ms)` — open a URL or file, answer the title.
- `browser_screenshot(path, width, height)` — capture the page as a PNG
  to `path`. The image rides disk, never text: its path is what a vision
  model is handed.
- `browser_read(selector)` — the text of the page, or of one element.

## Chrome

`BROWSER_CHROME` names the binary. Unset, the lane probes
`google-chrome-stable`, `google-chrome`, `chromium`,
`chromium-browser` on PATH, then the puppeteer cache under
`~/.cache/puppeteer`. A machine with none of those gets a refusal that
names what was looked for, not a hang.

## No third-party packages

The CDP client in `src/browser/__init__.py` is a plain socket: the
handshake is one HTTP upgrade and the frames after it are a masked
length prefix. No Playwright, no websocket library, no pip install.
