---
name: fleet-gui-reload-dead-end
description: "KI-246, fixed 2026-10-19 (9b9ce53) — the boot refused to run without a token in the URL while the app strips that token from the address bar at boot, so the first reload rendered the full shell with every listener unattached (inert UI); found by the theme-manager audit, which verified all four themes, both access paths, persistence, and the contrast guard as fully working. SUPERSEDED the same day by KI-247 (919416a, the owner's ruling): the per-launch credential is GONE entirely — browsing the address of a …"
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

The owner's feature #5 question ("was the theme manager … is it accessible through the
ui?") audited clean — **four themes (console default, paper, graphite, contrast) all
switch with visually distinct grounds (rgb 16,14,13 / 247,244,241 / 13,15,18 /
255,255,255), the topbar toggle opens the picker with descriptions + measured ratios,
`/theme` + Enter opens the same picker (works since the KI-245 fix), the choice persists
across reload via localStorage with a pre-paint head script, and `gui::themes` fails the
build when any theme's text drops under 4.5:1 or a theme is missing from the picker (3
tests green in every gate).**

**But the audit's reload step found KI-246 — the biggest defect of the whole 2026-10-19
audit pass, and plausibly a contributor to the original "the gui is nowhere to be found /
barebones" reports.** The boot had a guard: no token in the URL → show the refusal and
`return` out of the IIFE before a single listener attaches. The app *strips the token from
its own address bar the moment it boots* (the documented no-credential-in-screenshots
rule, `history.replaceState`). So the first reload re-navigated the token-less URL: the
shell rendered in full — rail, composer, toggle, everything — and nothing on it responded.
F5, a browser restoring the tab, reopening a cached tab: all dead shells, with a refusal
pointing at a URL no longer in the bar.

**The fix (client + server, `9b9ce53`, gate green 1121):**
- `app.js`: the boot's `if (!token) { …; return; }` guard is gone. Every fetch already
  rode the launch cookie (same-origin, automatic — the fetch helpers never appended the
  token), and `load()`'s 401 → "This page is not the one fleet printed." remains the
  refusal for a browser that genuinely holds no seat. `listen()` builds the EventSource
  URL with the query token only when the bar had one to give (a literal `?token=null`
  would shadow the valid cookie — see the server half).
- `server.rs`: the credential check is **any-of** (`presents(token)`: header ∨ query ∨
  cookie) instead of first-presented-wins, so a stale query token from a bookmarked URL
  cannot shadow the fresh cookie a relaunch set. Unauthenticated requests still get the
  bare 401 (the port-scan invariant: "a port scan sees 401s and learns nothing else").

**Verified live:** fresh boot → face; **reload (token-less bar) → the face boots and the
theme picker, all four themes, and the type-ahead all respond on it**; cleared cookies +
token-less → the bare 401; stale query token + fresh cookie → boots (any-of). KI-245's
Enter ruling verified on the reloaded page too.

**Why:** the boot guard read the address bar for a credential the address bar was
designed to lose — two correct decisions (strip-the-token, refuse-without-token)
composed into a dead UI on the owner's most ordinary gesture.

**How to apply:** when a boot guard reads something the app itself rewrites (URL, referrer,
a persisted flag), trace the reload/second-visit path before shipping — and when an owner
reports a surface as broken or missing, check whether a reload reproduces it before
auditing the feature. **Operational note (same evening):** the owner browsed
`http://127.0.0.1:54665/` and reported "it doesn't seem to be working" — 54665 was my
ephemeral verification instance (`--port 0`), killed the moment its test finished. The
owner's stable address is **`http://127.0.0.1:3080`** (the default port; `fleet gui`
always serves there until stopped), and a live instance was left running detached on 3080
that evening for exactly this reason (stop it with `pkill -f "fleet gui"`). Verification
launches use `--port 0` so they never collide with it — but check what a
`pkill -f "fleet gui"` will kill before firing it: the owner may be browsing the instance
you are about to stop. **Supersession (same day, KI-247, `919416a`):** the owner ruled the
whole credential model wrong — "if the app is running and I browse the server and port it
should open fleet without a refusal." The token mint, the field, the gate, and the cookie
are gone; the boundary is the loopback-only bind plus Host/Origin checks (a foreign Host
is the rebinding attack, a foreign Origin on POST/PUT/PATCH/DELETE is the cross-site form;
both refused 403 with the rule named); the printed URL is the bare address, and a fresh
browser with no cookies browsing it boots the full face. The owner's note that
authentication returns when fleet becomes a multi-user app is recorded in KI-247. This
file's cookie/any-of fix was the intermediate step, not the landing. Related:
[[verify-what-renders-not-what-commits]],
[[gui-revamp-browser-face-decision]], [[fleet-typeahead-audit-and-fixes]].
