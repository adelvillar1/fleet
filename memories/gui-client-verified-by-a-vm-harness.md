---
name: gui-client-verified-by-a-vm-harness
description: "GUI client changes have a COMMITTED face harness since 2026-10-23 (tests/face/harness.mjs, zero-dependency Node vm over the real app.js, run from cargo test by tests/face.rs), plus CDP/Playwright live passes against the served page; includes the stub lessons (per-boot DOM, selector engine, bubbling) and the second-instance mock-service pattern"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The GUI face (`src/gui/assets/app.js`, ~850 lines) has no test runner in the
repo: fleet is one static Rust binary with no Node toolchain, so `cargo test`
can only assert that the asset is *served*, never that it *works*. Every
client-side change in the GUI revamp increments (G2 onward) is verified by a
throwaway Node script that loads the real `app.js` into a `vm` context with a
hand-written DOM stub, scrapes every `id="..."` from `index.html` so the
script cannot ask for an element the page doesn't have, stubs `fetch`,
`EventSource`, `requestAnimationFrame` (synchronous), and `URLSearchParams`,
then feeds it the bridge's own `/api/state` document and asserts on the
elements the face built.

**Why:** without it, "the client parses" gets mistaken for "the client
works". The harness caught real defects a Rust test could not see: a click
handler bound on the log element never receiving a card's inner-button click
(events must bubble through `parentNode`), the composer read via
`textContent` when the script reads `value`, and derivations set with
`innerHTML` being invisible to a `textContent` assertion.

**How to apply:** keep the harness in `tmp/` and never commit it — the repo
has no JS runner and never should. As of 2026-10-20 (d9a07b5) `tmp/` and
`.zcode/` are gitignored, so the convention is enforced by the repo; one
sweep that week did commit them and had to be unwound. Rewrite it per
increment rather than
carrying it forward, and feed it the bridge's own state document rather than
a hand-made one, so the stub's event shapes cannot drift from what the
server emits. Four stub capabilities are load-bearing and easy to forget:
`querySelectorAll` must really walk descendants by class (a stub that
returns `[]` silently disables every keyboard path that reads its rows —
the palette's arrow keys and Enter did nothing until the stub was fixed),
`textContent` must clear children when assigned, or a re-paint appends
instead of replacing and a filter test passes for the wrong reason;
`className` must be a property that reads back what was set, or a
`classList.add` is invisible to an assertion; and `createTextNode` must
exist, or an inline renderer that builds nodes rather than `innerHTML`
has nothing to append to.

**The stub's requirements have grown with the face** (G6, 2026-10-18):
`document.documentElement` and `document.body` must exist as elements —
the theme manager reads `document.documentElement.dataset` at boot and
the rail mode writes `body.dataset.rail`; `querySelectorAll` on an
element must work (the rail's `markRail` calls it directly); and the
`window` stub needs `setInterval`/`clearInterval`/`setTimeout` or boot
dies on a panel's poll timer. Also: the ORIGINAL `tmp/face-harness.js`
is stale — it predates the grouped rail (it asserts the old `addresses`
chips) and the built router panel (it expects `#/router` to show the
later-panel), so do not run it as-is.

**One harness file per increment, named for the increment** (G6–G9,
2026-10-18): `tmp/face-harness-g6.js` (sessions + calibration), `-g7`
(tower + second-brain), `-g8` (settings), `-g9` (trust trio). Rewriting
per increment is deliberate (above), and the G6–G9 stubs additionally
need: `document.createElementNS` returning an element (every chart is
SVG — `svgPlot`/`donut`/`twoSeries` die without it), `setAttribute`
maintaining `className` when the key is `class` (SVG segments are
matched by class), `blur` on elements (`closePalette` calls it), and
`window.confirm` returning true (the rewind and destructive removes
confirm first). **The fetch stub must return a FRESH state document per
call** — `json: () => Promise.resolve(Object.assign({}, state, {events:
stateEvents}))` — because a captured object reference is evaluated once,
so a harness that reseeds `stateEvents` after a resume/rewind watches
the page reload from the OLD array and every assertion fails for a
reason that looks like an app bug. **Drive commands through their real
path**: the palette searches SURFACES (the `GROUPS` declaration) only,
so a slash command like `/review` is typed in the composer and fired
with a synthetic Enter — the palette's ⌘K never finds it. And when the
app's own flow reloads from the server (a rewind's `load(true)`), the
harness must simulate the server's half: seed the reseeded events
BEFORE the click that triggers the reload, because the fetch resolves
inside those ticks.

**Drive the pane and the palette through events, never by reaching
inside.** `face`, `paintMemory` and every other binding live inside the
IIFE closure, so a harness that assigns `face.file = doc` from outside
throws `face is not defined` — the real path is to fire a synthetic
`click` on the tree row and let the app do its own fetch. The same
closure is why a `node()`-built element cannot be asserted by identity;
assert on class and text. And the pane's two routes carry their own
query string, so the test client must join with `&token=`, not the
`?token=` the token-only routes use.

**The fetch stub must set `Content-Length` itself.** `req.write(body)`
followed by `req.end()` makes Node choose chunked transfer encoding, and
this server reads by `Content-Length` — so every `POST` from the harness
arrives with an empty body and the route answers as if nothing was sent.
That is exactly what a browser's `fetch` does *not* do, so the bug is
invisible against a real page. Set the header explicitly
(`h['content-length'] = Buffer.byteLength(body)`) and pass the body to
`req.end(body)`. A `POST` route that mysteriously sees no arguments is
this, not the route.

**Playwright is the live-browser half since 2026-10-19** (the owner's own
direction — "use playwright instead"): npm-install it into scratch (`/tmp/pw`;
the browsers are already cached in `~/Library/Caches/ms-playwright`, so no
download) and drive the actually-served page — console and page errors,
computed layout, screenshots, real clicks. The 2026-10-19 rail restore, the
pane audit, and the deck's rendered fragment were all verified this way. The
vm harnesses remain the per-increment assertion layer; Playwright answers
"what does a real browser actually show". Two gotchas from that session: the
vm DOM stub has no `document.querySelector`, so element lookups in the `el`
map must be `getElementById` even for a descendant span (give it an id); and
`node -e "script" url` puts the url in `argv[1]`, not `argv[2]` — use a
script file when passing arguments.

**Playwright location and traps (2026-10-20):** the module is at
`/Users/alejandrodelvillar/.hermes/hermes-agent/node_modules/playwright-core`
(require it by absolute path; no local install needed) and the executable at
`~/Library/Caches/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-mac-arm64/chrome-headless-shell`
— pass it as `executablePath`, because the package's bundled browser revision
does not match the cache. `page.click` times out on anything inside a `[hidden]`
panel (scope the selector, e.g. `#panel-memory .fact-card`, or use
`.evaluate(() => el.click())`) and on zero-size SVG nodes. The macOS chrome
shell binary is `chrome-headless-shell`, not the Linux `headless_shell`. A live
GUI on :3080 warms skill vectors against the embed server, which also starves
embedding in a concurrent `cargo test` — kill it before the gate.

**Test an action against a mock service on a second instance, never the live
one (2026-10-20).** `fleet gui --no-open --port 0` prints an OS-chosen port, and
`SECOND_BRAIN_PORT=4180` in its environment repoints the plugin at a mock:
python `http.server` that serves `/api/status` and accepts `/api/run` with the
`X-Tower: 1` guard, with a `/seen` route recording what was posted. That proves
the whole loop (select → refresh → `running…` → steps done with ms → state stamp
reloaded) without triggering a real pipeline. Two traps: a fixed `--port N` may
already be bound (the server says so and exits — `--port 0` avoids it), and the
mock must read the request to its `Content-Length` before answering
([[a-test-double-must-read-the-declared-body]]). Kill the test instance by its
listening PID from `lsof -nP -iTCP:<port> -sTCP:LISTEN -t`.

Update 2026-10-21: no Playwright browsers are cached on this machine any more (`~/Library/Caches/ms-playwright` is empty), but `~/Projects/helmsman/node_modules/playwright` (1.64) drives the installed Google Chrome 155 with `chromium.launch({ channel: "chrome", headless: true })` — no download. The folder-picker close used it against the live face at 1280×900 and 390×844 (script shape: /tmp/picker-verify.js in that session; screenshots to the session visualization dir).

**2026-10-23: the harness is COMMITTED, and this file's older advice is stale.**
The "keep it in `tmp/` and never commit it" rule above is retired — `tmp/` is
gitignored and a throwaway script in it is not evidence any more. What exists:
`tests/face/harness.mjs` (zero-dependency Node ESM, no npm) boots the real
`src/gui/assets/app.js` in a `vm` context and asserts on the DOM it builds;
`tests/face.rs` runs it from `cargo test` (node on PATH, skip by name where it
is not). **A plan criterion that says "a face test asserts…" is satisfied only by
a case in this file** — a live CDP pass proves the page works today, not that the
behaviour is pinned, and four completion-verifier refusals came from that
distinction. The current shape: one page = one boot (per-boot `IDS` map; a
module-level map let a previous page paint the next one's DOM); a descendant
`querySelector` with a small selector engine (tag/`.class`/`#id`, chains); no
`process` inside the sandbox (an unhandled rejection from a page shows up at the
gitignore-review level instead); and, because a stub has no layout engine,
geometry criteria are pinned by asserting the stylesheet's rules and left to a
live `getBoundingClientRect` pass for the pixels.

**The stub must honour `stopPropagation` (2026-10-23).** A stub whose `fire()` always
walks every ancestor — with `stopPropagation` defined as a no-op — reads a nested
button's click as its parent card's click too, so a component that deliberately
separates "the card opens its own detail" from "a button inside it opens the
inspector" passes every assertion while the real browser does something else. Make
`stopPropagation` set a flag the bubble loop checks. The same class: the sandbox needs
a `Node` global (the app's `renderSections` tests `section[1] instanceof Node`), and
`lastChild`/`firstChild` getters, or a drawer built from nodes dies.

**Raw CDP is the zero-dependency path (2026-10-23).** Node 24 has a global `WebSocket`, so a ~60-line script speaks the DevTools protocol directly — no npm, no Playwright: launch `"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --remote-debugging-port=9333 --user-data-dir=/tmp/… --no-first-run --disable-background-networking about:blank`, fetch `/json/list` for the page target's `webSocketDebuggerUrl`, then `Runtime.enable` + `Page.enable` + `Emulation.setDeviceMetricsOverride {width,height}` and `Runtime.evaluate` with `returnByValue`. This session verified the work surface at 1440 and 800 this way. **One trap cost several rounds: the previous session's page was cached** — `Page.navigate` to the same URL re-ran the old `app.js` and the new code never executed (no console output, no requests, empty transcript) while curl showed the server serving the new asset. Add a cache-busting query (`?r=Date.now()` before the hash) on every navigate. Trap two: `--dump-dom` with no new-tab handling can hang and write 0 bytes, and `--virtual-time-budget` may not flush — the CDP route is more reliable than the CLI flags.

**The in-app browser is the live pass available in-session (2026-11-03).**
`agent.browsers.getForUrl("http://127.0.0.1:<port>/")` gives a controlled
IAB tab over the actually-served page — `tab.playwright.domSnapshot()`
for the ARIA tree, `getByRole`/`selectOption`/`click` for real actions,
`tab.playwright.evaluate` for page-side reads. This is how the stop bug
was reproduced end to end: instrument `window.fetch` from `evaluate` to
capture the exact bodies the composer posts (`{"task":…,"mode":"auto"}`,
`{}` to `/api/stop`), click the real stop button, and read
`document.getElementById('seat').hidden` / `#seat-actions.children.length`
to see that the seat never painted. **Two traps:** the page is one IIFE
closure, so `face` and every `el` binding are unreachable from
`evaluate` — drive the DOM, never the internals; and a reload re-runs
boot, so re-instrument after every `tab.reload()`.

**Playwright through the owner's Chrome is the cheaper live pass (2026-10-23).** `require('/Users/alejandrodelvillar/Projects/helmsman/node_modules/playwright')` then `chromium.launch({ channel: 'chrome', headless: true })` needs no download and no CDP plumbing; `page.on('pageerror')` + console-error capture is the runtime-error check, and `getBoundingClientRect()` read through `page.evaluate` is the geometry check. This is how the work column's rail-following and its 48px folded strip were proven (col left 236/66 vs rail right 236/66; stage at rail+48; the fold surviving `page.reload()`). **A CSS rule assertion in the harness proves the rule exists; only this proves the pixels**, and for a layout defect the owner reported, the pixels are the claim.

**The stub must read back `style.setProperty` (2026-10-23).** The composer's text size is set as a CSS custom property on the form (`--composer-size`), so the harness's `El.style` — which had been `{ setProperty() {} }` — recorded nothing and the assertion had no way to see the value. Give it a `_props` map with a `getPropertyValue`. Related: the boot's `storage` map now accepts `opts.storage` seeds, so a case can prove a value restored from `localStorage` (a saved size clamped to the scale) and not only one written during the run.

**Restarting the face is part of the pass.** The face runs under the `com.fleet.gui` LaunchAgent (`launchctl kickstart -k gui/$(id -u)/com.fleet.gui`), which must be kicked after `cargo build --release` or it serves the old embedded assets. The first kickstart this session returned HTTP 000 for ~8s (the binary was being replaced) — retry after a short wait rather than concluding it failed, then confirm with `curl /api/sessions` (200) and `curl /app.js | grep -c '<new class>'` before driving a browser at it ([[verify-the-binary-is-newer-than-the-source]]).

**Static markup exists only in the HTML text, not the stub DOM (2026-10-29).**
`PAGE_IDS` is scraped from the real `index.html`, but boot() creates a BARE
div per id — class, `aria-label`, and the inline SVG of a static button are
nowhere in the stub. A case asserting `steer.getAttribute('aria-label')` on a
static composer button reads `undefined` while the real page is fine. Assert
static markup against the `HTML` source string (already a module constant)
the way CSS criteria read `ruleBlock`, and assert BEHAVIOUR through the stub
(its click handler fires and `el.task.value` lands). Related traps hit the
same session: `innerHTML` does NOT parse into children (selector-counting an
icon injected via `innerHTML` reads 0 — assert the `innerHTML` string
contains `'<svg'`), and `focus()` is a no-op with `document.activeElement`
always null, so focus claims are unverifiable in the vm (guard
`setSelectionRange` with try/catch — El lacks it).

**Two harness stub traps from the chat review (2026-10-28).** (1) The
clipboard can't be observed through `writeText` alone — the stub returns a
resolved Promise and the label flip restores after `setTimeout`, which the
stub window runs IMMEDIATELY (`setTimeout: (fn) => { fn(); }`), so a
`copied` label check always reads the restored text; record the clipboard
payload instead (`clipboardWrites.push(text)` exposed on the page) — that is
the real contract anyway. (2) The `.facts` panel wraps each pair in
`div.fact > dt + dd`, so iterating `#facts.children` for dt/dd pairs finds
none — query `page.rows('#facts dt')` and walk to
`dt.parentNode.children[1]`. Also: the harness now carries the `/api/session`
route (the branch-replay doc, `documents.branch`), which is what lets a case
drive `openBranchSession` → `showSessionObject` for drawer summaries; and
the copy/edit buttons and CSS pins (`ruleBlock('.row-rewind')` must hold no
`opacity: 0`) are the 274-check shape in `theChatCountsExchangesAndShowsItsControls`.

**Playwright location, current as of 2026-10-29:** the module that works is
`/Users/alejandrodelvillar/Projects/design-canvas/node_modules/playwright`
(createRequire from that node_modules root) with the same cached
`chromium_headless_shell-1248` executable as `executablePath`. The older
recorded locations (`.hermes/hermes-agent`, `helmsman`) are unverified this
session. A layout probe pattern that works without the server or pairing:
`page.setContent` with the REAL `src/gui/assets/app.css` inlined plus a
DOM stand-in of the component, then `getBoundingClientRect` + `scrollWidth`
down the flex chain — this is how the queue-text overflow was proven fixed
(chip 506 → list 488 → row 488 → text 375 ellipsized; probe at
`tmp/queue-overflow-probe.mjs`). See
[[a-layout-claim-is-measured-not-asserted]]: the CSS-rule pins were green
while the layout was broken.

**Three stub/fidelity traps from the navigation-plan build (2026-10-29,
harness now 323 checks / 35 cases).** (1) **Name collision in the shared
IIFE scope**: declaring a helper whose name already exists anywhere in
app.js's one closure overwrites it — a `function ownerRow(row)` predicate
silently replaced the message renderer's local `ownerRow(event)` and owner
messages vanished from EVERY case (18 failures with one root cause).
Before adding a top-level helper to app.js, grep the file for the name.
(2) **The fake window's `location.hash` setter must fire `hashchange`** —
a real browser runs the router on every programmatic assignment, and the
guard (same-value no-fire) matters because app.js's boot replaceState must
not double-run showRoute. Before the fix, all detach-on-`#/chat` checks
read stale DOM. (3) **The stub DOM is FLAT — all PAGE_IDS elements are
siblings** under documentElement, so `el.rail.querySelectorAll` finds
nothing built into `el.railTree` (they are not real descendants); query
`el.railTree` like the existing bots badge does, and prefer class +
`dataset` filters over attribute selectors. Related face-side lesson from
the same build: the face's own guard for a detached home must let the
`session` ADOPTION event through — a send from the home has to follow its
own conversation.

**Boot parses static `hidden` now; other static attributes still don't
exist (2026-10-29, right-pane build, harness now 344 checks / 37 cases).**
The right-pane case failed "the pane opens" because the stub's bare div
starts `hidden: false` while the real `#pane` boots hidden — boot() now
regexes each id's tag in the HTML and seeds `hidden: true` where the source
carries the attribute. Still true and unfixable at boot: `class`, `role`,
`aria-*` on static markup — a `.pane-tab` class selector finds nothing (the
ids are flat with empty className), so address static buttons by id
(`page.IDS['tab-web'].click()`) and assert static roles/bounds against the
`HTML` source. For values written through `documentElement.style`, assert
via `page.sandbox.document.documentElement.style.getPropertyValue('--pane-
width')` (the queue-probe path). And a component tracking its own scalar
(the grip's width) should keep it in a closure variable — the stub's
`getBoundingClientRect` returns fixed numbers, so reading the layout back
as state makes the keyboard path untestable. Layout probes of the pane:
`tmp/pane-width-probe.mjs` (the grid honors `--pane-width` at 440/720/300
in real Chromium).

**Traps and additions from the preview-formats build (2026-10-29, harness
now 351 checks / 38 cases).** (1) New stub routes the cases needed:
`/api/tree`, `/api/file` (answer by `documents.tree` /
`documents.files[path]`), `/api/web/frame`, `/api/web`, and `/api/memory`
— a case driving a panel whose route isn't stubbed dies in the fetch.
(2) A module constant `RUST` (bridge.rs + server.rs text) lets a case pin
backend FENCES (nosniff, attachment, the credential refusal) alongside
the HTML/CSS pins — the served page's backend is part of the contract.
(3) **Property vs attribute on the fake El**: `img.src = url` sets the
property; `img.getAttribute('src')` reads undefined — assert the
property, and remember the real browser returns the ABSOLUTE url, so
match with `.includes()`. (4) The fake DOM cannot inflate
(DecompressionStream absent, `import()` unavailable), so binary-format
renderers (pdf/xlsx/pptx/docx) can only be checked for their GRACEFUL
failure — the pane's own "would not render: …" card is the assertable
surface, and a real-Chromium probe is the only true render proof
(`tmp/pdf-probe.mjs`: a hand-written minimal PDF, the vendored
pdf.min.mjs imported via blob URL because a setContent page cannot
cross-origin-import from the server, one page rendered, inked-pixel
count as the verdict — 2,345 pixels). (5) A second constant-collision
class: the harness file already had `SRC` (app.js) when the preview case
added another — grep the harness for a constant name too, not just
app.js for helper names.
