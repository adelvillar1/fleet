---
name: plugins-ux-plan-drafted-2026-11-03
description: "project 2026-11-03 — the owner's "the plugin UI is atrocious" verdict became docs/plans/2026-11-03-plugins-ux.md, gated, then the owner demanded mockups before any build — three HTML mockups in the face's own idiom — then approved them ("implement as designed") and the plan was IMPLEMENTED and COMPLETED whole (5f56b86, gate 2,012/0, harness 703/0 across 67 cases): the built-ins as three grouped status-first sections, every row carrying its press, the install door, the trust disclosure, the rail badge"
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

The owner's verdict (2026-11-03, on the live face after the plugin host's
landing and the built-ins fix): **"the plugin UI is atrocious, we need to
rethink this."** Same register as the settings verdict that produced
[[settings-ux-plan-2026-11-01]] — and the rethink applies that revamp's own
hard-won shape to the page that ignored it.

**What the capture showed** (the live DOM at `#/plugins`, after the
built-ins fix [[creator-mode-plugin-host-exploration-2026-11-02]] landed):
a three-sentence jargon head (manifests, bridge-side enforcement); one
undifferentiated card of sixteen flat prose rows — the second-brain tool
plugin, the seven lanes, and the eight MCP servers interleaved in
construction order, states as filesystem paths (`resolved · judgment · at
/Users/…/venvs/sys1`); an empty-state explaining directory layouts; a raw
path input with a bare "privileged" checkbox.

**Seven grounded gaps → the rethink (all landed):**
- the 16-row wall → **three grouped sections** (tool plugin / lanes / MCP
  servers), each a header with a count, each row a **status dot** (the
  word computed bridge-side in `built_ins_value` — `dot: ok|warn|off` per
  row — the JS maps word→colour, never a second judgment) beside the name
  and a one-line human state;
- pointer prose ("the settings page's lane cards") → **every row carries
  its action** — the tool plugin's row opens the second-brain panel
  (`#/second-brain`), a lane's row opens that lane's card in settings
  (`#/settings/specialists/<encodeURIComponent('lane:'+name)>`), a
  server's row the `server:<name>` card — one press to the thing;
- the machinery leading → the head shrinks to one line with counts
  (`9 built-in · 2 installed · 1 draft awaits review`); `how trust works`
  opens the tiers/sandbox prose as a **disclosure** (the two tier cards —
  amber vouched / blue sandboxed — and the ladder draft → installed ·
  untrusted → privileged with its gates named);
- a developer's install form → **a door**: the press opens the form
  (`face.pluginsDoor`), the refusal renders beside the field, the
  elevation's grant is inline *before* the press, the cancel closes it;
- the bare rail entry → a **count badge** when a draft awaits review (the
  bots entry's pattern; `paintRailCounts` re-run on every state landing).

**The arc:** plan `docs/plans/2026-11-03-plugins-ux.md` gated → the owner
demanded mockups → three mockups built and rendered at
`docs/plans/mockups/2026-11-03-plugins-ux/` (`plugins.html`, `install.html`,
`trust.html` + `mockup.css` + PNGs, in the face's own palette and type with
the live state's real names) → **the owner approved them ("implement as
designed")** → the plan **IMPLEMENTED and COMPLETED whole** (`7813311` the
translation, `5f56b86` the tick): gate fmt 0 / clippy 0 / build 0 /
**2,012 tests / 0**, face harness **703 checks / 0 across 67 cases**, the
Chromium probe quoting the loaded DOM (the pre-data-paint trap recorded in
the plan's Notes — see [[chrome-screenshot-fires-at-load]]).

**Build traps:** the block replacement duplicated `function paintBots() {`
(the old_string ended at the anchor, the new block ended with the same
line — parse error at the file's end is the tell); the surface rewrite
dropped `pluginSections` and the bundle row's drawer press threw
ReferenceError — the rewrite must carry the old block's functions, not
just its markup; the door's elements are STALE after the repaint a check
itself triggered — re-find through the board, never reuse the pre-repaint
reference; a lane press navigates to the settings page, so the case's boot
needs a minimal `documents.settings` or `paintSettings` throws on the
empty reply (an unhandled rejection, not a failed check); the rail badge's
count span needs `paintRailCounts()` re-run after the data lands (it is
not part of the repaint).

**The live verification** (the face rebuilt on the installed path — see
[[fleet-gui-is-a-launchagent]] for the binary-path move): `/api/state`
carries **16 built-ins, every one with its dot**; the served `app.js`
matches the source's byte count. The owner reloads the tab to see it.

Related: [[settings-ux-plan-2026-11-01]] (the shape applied),
[[creator-mode-plugin-host-exploration-2026-11-02]] (the host),
[[a-fixture-can-match-the-face-and-miss-the-bridge]] (the flat-shape seam),
[[a-harness-find-returns-an-array]] (the stub seams),
[[the-surface-keeps-the-machinery-and-hides-the-answer]] (the standing bar).
