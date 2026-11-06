---
name: fleet-browser-lane-architecture
description: "owner-directed 2026-10-17 — a browser belongs in fleet as the sixth LANE (a Python MCP child driving Chrome over CDP), not as an embedded engine; SHIPPED as 342467e with three tools, no third-party packages, and one browser per server; \"everything is a plugin\" is the running architecture"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked what it would take to give fleet's rendered deliverables their
own chrome, first as "a native browser as a plugin" then "native rust browser
plugin", then landed on the frame that fits: **"using the everything is a
plugin."** That is not an aspiration in fleet — it is the running architecture.
`lanes.rs` declares lanes, each a **Python child process speaking MCP**,
launched as `python3 -u service/mcp_server.py` with one `<SVC>_PYTHONPATH`
variable and a declared `passthrough` list of ambient names. A browser is one
more entry, and **no embedded engine was ever on the table**: Servo is the only
real native-Rust browser engine, it is not embeddable as a library, it is
roughly five hundred crates, and using it would require the owner to reverse
two standing rulings (the crate contract and zero-new-crates). Recommended
against and skipped.

**SHIPPED as `342467e` (2026-10-17).** Three tools over the DevTools Protocol:
`browser_navigate` (open a URL or file, answer the title),
`browser_screenshot` (capture the page to a path — the image rides **disk,
never text**, because its path is what the `vision` tool is handed), and
`browser_read` (the text of the page or of one CSS selector). The lane lives
at `services/browser/` with `service/mcp_server.py` and
`src/browser/__init__.py`.

**No third-party packages, and that is the load-bearing choice.** The CDP
client is a plain socket: the handshake is one HTTP upgrade and the frames
after it are a masked length prefix. No Playwright, no websocket library, no
pip install — which is what keeps this a lane rather than a dependency.
Chrome's location rides `BROWSER_CHROME`; when unset the usual suspects are
probed and the refusal names what was looked for.

**Two bugs that only driving it could have found:**
1. **Version order in the puppeteer cache.** The cache holds one directory
   *per release* and `os.walk` visits them in directory order, not version
   order — so the lane launched an old shell that segfaults on flags the new
   one accepts (`Chrome exited during launch with status -11`). Candidates are
   now collected and sorted by version. This is the class of bug that is
   invisible in the code and immediate at runtime.
2. **One browser per call was wrong.** The first version spawned a fresh Chrome
   per tool call, so a `navigate` and the `screenshot` that followed it looked
   at *different pages*. Found only because the PNG's byte count was checked
   instead of trusting the JSON, which said success — a blank white capture is
   2875 bytes where the real page is 62KB. **One browser per server** now. The
   lesson: a tool that reports success can still be wrong, so verify the
   artifact, not the ack.

**Operational facts worth carrying:** the lane **installs nothing** (empty
package set in `install.rs`'s table — its child is stdlib Python and Chrome is
a machine resource, the same class as gen1's ONNX weights), and it holds a
persistent Chrome on a **fixed port (9333)**, so two concurrent fleet processes
collide — the second gets a named refusal rather than a silent second browser.
That is the right failure but a real constraint if two faces ever run at once.

**Why it matters:** this is the shape that un-blocks the automated vision loop
without touching the two rulings that keep fleet shippable — and it is the same
seam every other capability already rides, which is the strongest argument that
it is the right one. Combined with the deck grader ([[fleet-deck-visual-pipeline]])
the loop is now complete end to end: a run builds a deck, the deck renders in
the browser, the lane screenshots it, the `vision` tool grades the PNG — no
manual capture by the owner.

**How to apply:** build lanes, never engines. One `Lane` entry plus a Python
child plus a tool declaration is the whole cost; do not reach for an embedded
engine and do not add a crate. When adding a lane, expect the roster-count
assertions to redden — see [[adding-a-tool-breaks-the-declaration-counts]].
Related: [[fleet-deck-visual-pipeline]] (the loop this closes),
[[name-the-component-that-has-the-capability]] (the browser is the rasterizer),
[[gui-revamp-browser-face-decision]] (the pane the deck renders in).
