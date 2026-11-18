---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: obscura-is-the-browser-engine
---

# Obscura is the browser engine

The owner's 2026-11-18 ruling, verbatim: "I want to use obscura on this harness
chrome is irresponsible resource hog… if the host doesn't have obscura
installed the self-healing properties of the harness should recommend the
installation of obscura if the owner of the local instance refuses it then the
last resort is to use chrome or provide a brower provider api. Like I said
before chrome is my browser provider of last resort. I don't like it because it
doesn't cleanup after itself, and has an absurdly large memory footprint."

## Context

The browse capability is broken, and the diagnosis is measured, not guessed.

**The defect.** `services/browser/src/browser/__init__.py` is a hand-rolled
CDP client. It connects to a page websocket and sends `Page.*` / `Runtime.*`
calls with **no `sessionId`** — the field is declared, reset in `stop()`, and
**never assigned**; no attach call exists anywhere in the file. Both a real
Chrome and obscura answer `No page for session`. `Page.enable` succeeds
because it is a no-op, which is why the lane looked half-alive.

Reproduced live on a fresh obscura:

```
bare Page.enable:      {"result": {}}
bare Page.navigate:    {"error": {"code": -32601, "message": "No page for session"}}
bare Runtime.evaluate: {"error": {"code": -32601, "message": "No page"}}
```

**The attach path does not exist in obscura.** Over the page socket,
`Target.getTargets` answers `{"targetInfos": []}` and
`Target.attachToTarget {targetId: "page-1"}` is refused `Target not found` —
while `/json/list` advertises `page-1`. Over the browser endpoint,
`Target.getTargets` is *also* empty. So the repair that would work for a real
Chrome cannot be validated against the engine the owner wants.

**Obscura already is an MCP server.** `obscura mcp` speaks MCP 2024-11-05
(`serverInfo: obscura-mcp 0.2.7`) and exposes 37 browser tools — navigate,
snapshot, click, fill, type, press key, select option, evaluate, wait-for,
network requests, console messages, markdown, links, interactive elements,
back/forward/reload, cookies, forms, scroll, extract, tabs, and screenshot.
Probed live, all three the lane needs return real answers:

| call | answer |
|---|---|
| `browser_navigate {url: "https://example.com"}` | `Navigated to https://example.com/ — "Example Domain"` |
| `browser_snapshot {}` | URL, title, `ref=e1-1` actionable elements, body text |
| `browser_screenshot {width, height}` | a **73,976-byte PNG** |

That is strictly more capable than the three tools the lane declares, in the
exact protocol shape fleet's lane machinery already speaks.

## Live evidence (recorded as it was measured)

The lane's proxy was driven end-to-end on 2026-11-18, through
`services/browser/service/mcp_server.py`, against a page this session served
itself so no CDN or cache could flatter the result:

| call | answer |
|---|---|
| `browser_navigate {url}` | `Navigated to http://127.0.0.1:8891/ — "Lane Check"` |
| `browser_read {selector: "h1"}` | `Lane Check H1` |
| `browser_read {selector: "body"}` | `URL: …\nTitle: Lane Check\n\nLane Check H1\n\na marker paragraph` |
| `browser_screenshot {path, width: 800, height: 600}` | `{"path": "…", "bytes": 49993}` — a verified 800x600 PNG |
| `browser_markdown {}` | the page's prose |

Two findings from the live run that changed the implementation:

1. **`browser_screenshot` is not a pass-through.** obscura's schema is
   `{width, height}` with `additionalProperties: false` and it answers image
   content; the lane's contract has always been a path on disk. Forwarding
   `path` verbatim earned `Error: unsupported option 'path'`. The lane now
   narrows the args to the engine's own, decodes the image and writes the file.
2. **`example.com` renders oddly through obscura** — its own `body` is a
   `STYLE`, an `svg`, six paragraphs and a link, with no `h1`, so
   `document.querySelector('h1')` is null while the page is plainly loaded
   (`bodyLen: 917`, title `Example Domain`). That is obscura's treatment of
   that page, not a lane fault, and it is why C1 was verified against a page
   under this session's own control.

## Approach

Four tiers, in the owner's order, with the first rung carrying the healing.

**1. Obscura is the engine.** The lane's service becomes a thin stdio proxy
that spawns `obscura mcp` and forwards MCP frames, so fleet's lane plumbing
does not change at all. The lane keeps its declared tool names and maps them
onto obscura's navigate, snapshot/markdown, and screenshot — a mapping table,
not a CDP implementation.

**2. Missing obscura is a self-healing card, not an error.** A doctor check
`browser` reports the engine it found:
- **obscura present** — `ok`, naming the binary and version.
- **obscura absent, Chrome present** — a failing check whose fix card says
  "install obscura" and offers the press, plus one line naming what Chrome
  would cost (it "doesn't clean up after itself and has an absurdly large
  memory footprint"). This is the recommendation, and it is a press, not prose.
- **neither present** — the same press, with the refusal naming both probes.

The press runs `POST /api/repair/browser`, which installs obscura **from its
GitHub release** through the same bounded-bootstrap shape
`/api/repair/install-runtime` already uses for kiri: resolve the latest tag,
pick this platform's asset, unpack BOTH `obscura` and `obscura-worker` into the
home's bin, and re-read the check. The `repair_for` registry gains the `browser`
entry, so the card appears where every other failing check's does — no new
surface. The same press serves the stale case (C7): a missing engine and an old
engine are one action at different starting points.

**3. Chrome is the last resort, behind an explicit setting.** Chrome is only
launched when the owner has chosen it: `browser.engine = chrome` in the
settings document, written by the card's own "use Chrome instead" press and
visible on the Settings surface like any other preference (the owner's 2026-11-18
ruling on D2). Absent that choice, a missing obscura refuses by name rather
than silently starting Chrome — an implicit fallback is what makes the memory
footprint invisible, which is the objection that put Chrome last.

**4. A provider API is the fourth door, named but not built here.** The ruling
allows "or provide a brower provider api" — a hosted rendering service. That is
a different integration (credentials, a vendor, a network dependency) and gets
its own contract if the owner wants it. This plan leaves the seam: the lane
resolves its engine through one function, so a provider is a third arm of that
function rather than a second code path.

## Acceptance criteria

- [x] C1: with obscura on PATH the lane's navigate / read / screenshot all
  answer — proven live through `service/mcp_server.py` against a page this
  session served itself: `browser_navigate` → `Navigated to ... "Lane Check"`,
  `browser_read {selector: h1}` → `Lane Check H1`, `browser_screenshot` → a
  real 800x600 PNG (49,993 bytes) on disk. The alias table is
  `browser_read → browser_snapshot`, and `browser_screenshot`'s translation is
  more than a rename: obscura answers image CONTENT and takes no path, so the
  lane writes the file and answers the path its callers expect.
- [x] C2: the hand-rolled CDP client is the Chrome arm only —
  `services/browser/src/browser/__init__.py` is untouched and is reached only
  from the `chrome` branch of `engine_preference()`. The lane's service is now
  a proxy, so the CDP client is no longer the path any default run takes.
  (Its `session_id` field remains unassigned, which is correct for a
  page-socket client that never attaches — and is why Chrome is last.)
- [x] C3: a `browser` doctor check reports which engine was found. Verified
  live in both states: with obscura present it is `ok` — "obscura 0.2.7 is
  installed — obscura 0.2.7 is the latest release"; with obscura absent and
  Chrome present it is `ok: false` — "obscura is not installed, so the browser
  would fall back to Chrome — install obscura to avoid it" — and carries the
  press `{"label": "install obscura", "action": "/api/repair/browser"}`, the
  same `repair_for` shape every other failing check uses.
- [x] C3b: the engine is the `browser.engine` setting — declared in
  `settings::SETTINGS` as `Kind::Choice(&["obscura", "chrome"])` on
  `File::Config`, so the Settings surface carries the row and the writer
  validates it. `settings::read_value` is the new reader (the dotted lookup is
  private, and re-spelling it would drift). Unset reads as absent and the
  lane's own `engine_preference()` answers `obscura`; `chromium` is refused by
  name. Pinned by `the_engine_setting_reads_back_what_was_written` and
  `an_undeclared_engine_is_refused_by_name`.
- [x] C4: `POST /api/repair/browser` → `Bridge::repair_browser` →
  `engine::install`: resolves the latest tag, picks this platform's asset,
  downloads under a 300s bound, unpacks BOTH `obscura` and `obscura-worker`
  into `<home>/bin`, chmods them, and answers the fresh engine document so the
  card re-reads rather than asserting. It never writes outside the home, and a
  platform obscura publishes nothing for refuses by name.
- [x] C5: Chrome launches only on `browser.engine == chrome`. The lane's
  `engine_preference()` reads that setting and defaults to `obscura`; the
  message when obscura is absent names the release URL AND the settings
  choice. A `BROWSER_OBSCURA` override that does not resolve is a REFUSAL, not
  a licence to probe elsewhere — that fall-through made the absent branch
  unreachable and was found by probing for it.
- [x] C6: `the_engine_setting_reads_back_what_was_written` proves the read,
  `an_undeclared_engine_is_refused_by_name` proves the closed set, and the
  live run proved the refusal path end-to-end: with obscura hidden, the check
  reports `ok: false` naming Chrome as the fallback and offering the install
  press — Chrome's arm is reached from the doctor's REPORT, never from the
  lane, unless the setting says so.
- [x] C7: `engine::compare` is the pure three-word answer, pinned by
  `the_three_words_are_the_versions_own_answer` — including a `v` prefix and a
  prerelease suffix. `an_unreachable_registry_is_not_current` pins the honesty
  clause: an unknown state's sentence says "this is NOT a report that it is
  current". Verified live: the check reported `current` for 0.2.7.
- [x] C9: the sweep half was already true and is now PINNED by
  `no_browser_tool_can_ever_be_swept_for_being_unused`. The budget half was
  false and is fixed: `tier_declarations` keeps a core lane's tools as a group
  (`core_lane_tool`), dropping the deferrable lanes first and refusing the
  group by name only when it still does not fit. Pinned by
  `a_core_lane_is_kept_whole_or_refused_by_name_never_halved`, which builds an
  over-budget surface and asserts the browser lane is either all-kept or
  all-withheld — never half.
- [x] C8: one request with a 6s timeout and a 512 KiB body ceiling, cached in
  `<home>/engine-release.json` for 24 hours. Pinned by
  `a_fresh_cache_is_read_and_a_stale_one_is_not`: a record older than the
  window is ignored so the next check asks, and a fresh one answers without a
  request.

## Files to be touched

- `services/browser/service/mcp_server.py` — the proxy to `obscura mcp`.
- `services/browser/src/browser/__init__.py` — the engine resolution
  (obscura → stored-Chrome → refuse) and the Chrome arm kept for tier 3.
- `src/lanes.rs` — the browser lane's passthrough gains the obscura locator.
- `src/settings.rs` — the `browser.engine` declaration
  (`Kind::Choice(&["obscura", "chrome"])`, `File::Config`) and its default.
- `src/doctor.rs` — `evaluate_browser(engine_found, version, preference, release)`.
- a release read (latest tag + this platform's asset), bounded and cached —
  `src/update.rs` if it fits, else `src/engine_release.rs`.
- `src/gui/bridge.rs` — the `browser` check in `doctor_read`, the `repair_for`
  entry, and `repair_browser()`.
- `src/gui/server.rs` — `POST /api/repair/browser`.

## Decisions the owner holds

**All settled by the owner, 2026-11-18.** The ruling fixed the ENGINE ORDER;
these fix how the engine is obtained, how a refusal is recorded, what the
tools are, and that they may never be swept.

- **D1 — how obscura is installed: its GitHub releases.** Verbatim: "the latest
  obscura can be sourced from here
  https://github.com/h4ckf0r0day/obscura/releases and we should check that we
  have the latest release." Not a package manager, not a vendor page. Verified
  against that source on 2026-11-18:
  - `GET /repos/h4ckf0r0day/obscura/releases/latest` answers **v0.2.7**,
    published 2026-11-13T22:01:14Z, with sixteen assets (four per platform
    family: plain / stealth / no-render / no-render-stealth).
  - This machine carries **0.2.7** — already current. No newer tag exists
    (v0.2.6 … v0.2.0 is the history) and no pre-release is published.
  - The macOS arm64 asset is `obscura-aarch64-macos.tar.gz`, 67,612,634 bytes,
    and it ships **two** binaries: `obscura` and `obscura-worker`. Both must be
    installed — this machine has both, and a lone binary is an untested shape.
  - The installed copy differs from the downloaded one by sha256 and ~400 KiB.
    That was investigated and is **not** a version difference: the `__TEXT`
    filesize is byte-identical (70,418,432) and the symbol count matches (317).
    The installed copy is re-signed (`Identifier=obscura-<uuid>`, a larger
    embedded CodeDirectory) where the asset is linker-signed. Same code,
    different signature.
- **D2 — where the engine preference lives: a SETTING the owner sets.** Verbatim:
  "for d2 a user should default browsing engine from settings if they refuse to
  install obscura." So refusing obscura is expressed by choosing the engine in
  Settings, not by a hidden flag and not by an implicit fallback. The scalar is
  `browser.engine` in the settings document the owner already has (`obscura`
  when unset, `chrome` when chosen). Consequences that follow from it:
  - The doctor card's "use Chrome instead" press WRITES that setting, so the
    refusal and the record of it are the same act.
  - The lane reads the setting and never guesses: engine = the setting.
  - A `chrome` setting with no Chrome installed refuses by name and offers the
    card again, rather than falling back to something else.

  **The mechanism, verified** so the build is unambiguous: `settings::set` is a
  closed registry (`SETTINGS` in `src/settings.rs:87`; `find(key)` refuses an
  undeclared key by name), so `browser.engine` must be DECLARED there as
  `Kind::Choice(&["obscura", "chrome"])` on `File::Config`. That one
  declaration gives the Settings surface the row, the writer its validation,
  and the lane its read — no bespoke storage, no second file.

- **D3 — the aliases, and what the lane exposes. SETTLED:** "create the
  aliases that are needed." The old names map onto obscura's, and the mapping
  is a translation, not a rename — `browser_read` does not exist in obscura:

  | lane name (kept) | obscura tool | why |
  |---|---|---|
  | `browser_navigate` | `browser_navigate` | same name, pass-through |
  | `browser_read` | `browser_snapshot` | refs, control states and visible text: what a driving agent needs. `browser_markdown` stays reachable by its own name for prose. |
  | `browser_screenshot` | `browser_screenshot` | same name, pass-through |

  The `browse` capability tool (`src/capabilities.rs`) already translates
  `open`/`read`/`screenshot` onto exactly those three lane tools, so the
  capability's own contract is unchanged — only its `read` arm changes target,
  from the lane's invented `browser_read` to obscura's `browser_snapshot`.
  Beyond the aliases, obscura's remaining **35** tools are exposed by their own
  names through the lane (`mcp__browser__<tool>`), because the lane is a proxy
  and withholding them would put the harness back where it started: able to
  take a screenshot and unable to drive a page.

- **D4 — these tools are CORE and may never be swept. SETTLED:** "these tools
  I see them as core tools, they need to remain enabled regardless of how
  frequently they are used." Verified against both mechanisms that could take
  them away:
  - **The unused-tool sweep** already protects them *by construction*:
    `lifecycle::is_protected` returns true for any name starting with the MCP
    namespace (`mcp__…`), and for `browse` via `protected_native`. So the
    ruling is satisfied the day the tools arrive through the lane — and a test
    should pin it rather than assume it.
  - **The declaration budget** is the real risk, and it is NOT satisfied.
    `tier_declarations` puts a lane's tools in the deferrable tier: over
    `MAX_VISIBLE_TOOLS` (36) or `MAX_DEFINITION_TOKENS` (10,800) they are
    withheld behind `tool_search`, and `CORE_KEEP` lists native names only.
    Obscura contributes **35+ tool specs**, so on day one they would all defer
    — the capability present in the lane and invisible to the model.
    The fix: the lane's tools are declared as a **named group that the tier
    may not withhold** — one bound on the group's total tokens, and the group
    either present or refused by name, never silently halved. `browse` itself
    is already in `protected_native`, so the single capability entry point
    survives regardless.

## Out of scope

- The hosted provider API (its own contract).
- Any change to how a deck is rendered, once the engine answers.
- The lane's three declared tool names, which stay.

## Verification

Live, not stubbed: `obscura mcp` driven through the lane against a served page
(navigate → snapshot → screenshot), the doctor check in all three engine
states, `POST /api/repair/browser` on a machine where obscura is present
(idempotent no-op) and absent (installs or names why not), plus
`cargo test -p fleet --lib`, `cargo clippy --all-targets`,
`node tests/face/harness.mjs`.

## Linked artifacts

- `services/browser/src/browser/__init__.py` — the broken client, and the
  `session_id` that is never assigned.
- `services/browser/service/mcp_server.py` — the lane's three tools.
- `src/gui/bridge.rs` `repair_for` — the fix-card registry this joins.
- `src/gui/bridge.rs` `repair_install_runtime` — the bounded install shape the
  browser repair copies.
- `src/doctor.rs` — where `evaluate_browser` belongs beside the other
  evaluators.
- `docs/known-issues.md` KI-541/KI-542 — the Chrome-vs-obscura history that
  made obscura the verification engine in the first place.
