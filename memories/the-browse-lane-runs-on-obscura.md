---
name: the-browse-lane-runs-on-obscura
description: "The browse lane runs on obscura now — a stdio proxy publishing 39 tools (its 3 historical names aliased onto obscura's, the rest exposed), proven live with navigate, read and a real 800x600 screenshot."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [deps, rendering, resolved]
phrases: [a harness that cannot properly use a browser is not usable, create the aliases that are needed, obscura mcp, browser_read → browser_snapshot, unsupported option 'path', engine_preference]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The browse lane now runs on obscura. `services/browser/service/mcp_server.py` was rewritten from a hand-rolled CDP wrapper into a stdio proxy to `obscura mcp`, per the owner's 2026-11-18 ruling ("a harness that cannot properly use a browser is not usable in my view").

THE ALIASES (owner: "create the aliases that are needed"): `browser_navigate` → obscura's `browser_navigate` (pass-through); `browser_read` → `browser_snapshot` (the lane's own invented name, translated — obscura has no `browser_read` at all); `browser_screenshot` → `browser_screenshot`, but NOT a pass-through. Beyond the aliases, all of obscura's other tools ride under their own names, so the lane publishes **39 tools** (3 aliases + 36), verified live.

THE NON-OBVIOUS TRANSLATION: obscura's `browser_screenshot` schema is `{width, height}` with `additionalProperties: false` and it answers IMAGE CONTENT (`{type: image, data: base64}`), while the lane's contract has always been a PATH on disk whose bytes a vision model is then handed. Forwarding `path` verbatim earned `Error: unsupported option 'path'` on the first live call. The lane now narrows the args to the engine's own, base64-decodes the answer and writes the file, answering `{path, bytes}`. Verified: a real 800x600 PNG at 49,993 bytes.

ENGINE ORDER: `engine_preference()` reads `browser.engine` from `$FLEET_HOME/config.json`, defaulting to `obscura`; `chrome` only when the owner chose it, and the hand-rolled CDP client is now reached ONLY from that branch (untouched otherwise). `find_obscura()` probes `BROWSER_OBSCURA`, then `<home>/bin/obscura`, then /usr/local/bin, /opt/homebrew/bin, then PATH. A missing engine answers a NAMED error and a tools/list that still carries the three names — never an empty list, because an empty list reads as "no browser here" and hides what the owner must do.

LIVE EVIDENCE, and why the test page mattered: verified against a page this session served itself (h1 → "Lane Check H1", body → the heading plus a marker paragraph). Against `https://example.com` obscura's own rendering is odd — its body is a STYLE, an svg, six paragraphs and a link with NO h1, so `querySelector('h1')` is null while the page is plainly loaded (`bodyLen: 917`, title `Example Domain`). That is obscura's treatment of that page, not a lane fault; testing against a page under my own control is what let me tell the two apart.

Gate at this point: 2360 Rust tests, 1457 face checks across 204 cases, lane server parses. STILL OPEN in `docs/plans/2026-11-18-obscura-is-the-browser-engine.md` (status draft, 8 criteria): the `browser.engine` setting declaration, the doctor check with the install press, the release check (C7/C8), and C9 — the lane's 39 tool specs exceed `MAX_VISIBLE_TOOLS` (36), so they WOULD be withheld behind `tool_search` by `tier_declarations` unless the group is made un-withholdable.
