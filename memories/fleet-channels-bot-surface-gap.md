---
name: fleet-channels-bot-surface-gap
description: "2026-10-20 — the bot/channel service and its interface are ON master (port plan completed, B1-B7 delivered, plan 2026-10-20-bot-interface completed after a live Chromium audit found KI-269..274; the PWA shell then added KI-275)"
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

**2026-10-20, RESOLVED — the service and its interface are on master.** The
owner's question "we built bot functionality, we have no interface, how do we
bridge the gap?" is closed twice over: the port plan
([fleet-remote-port-onto-master]) landed the transport and `#/bots`, and the
gap-closing plan `docs/plans/2026-10-20-bot-interface.md` is now **completed** —
B1–B7 delivered by the port under other names (see that plan's Outcome
section), then live-audited in Chromium and repaired.

The earlier record, kept because it names the shape of the problem (branch-only
feature, R6 closed on the half the architecture handed over for free, the
`#/channels` surface that was really an inbox), is below; every hole it lists is
now fixed on master.

**What the audit found in the LANDED work** (owner: "there was some ui work
there already but I don't trust it"). Each verified by rendering, not by reading:

| # | Defect | Cause |
|---|---|---|
| KI-269 | Every panel collapsed on a phone: at ≤900px the stage landed in the 236px rail track beside 664px empty (measured `cols: 236px 664px`) | The narrow media-query block's bare `.face` cannot out-rank the base sheet's `body[data-aside="closed"] .face` / `body[data-rail="icons"] .face`; the stage got track 1. A media query adds NO specificity. Fix: answer each body state at its own specificity with `:is()` |
| KI-270 | `--callback` was decorative — printed, and the CLI warned re-pairing with it was the fix, while nothing was stored | `PairedPeer` had no callback field; `pair()` never took one. Fix: stored default, http(s)-validated, webhook-only, the delivery's own callback wins. Pinned by three tests |
| KI-271 | The `pending` chip emptied the peers list | It filtered a list that can never contain pending peers; the "waiting at the door" tile set the same filter. Fix: chip gone, tile scrolls to the door card and flashes it |
| KI-272 | Activity rows were dead ends; the stamp claimed rows that did not exist ("newest 6 of the last 50" with 6 lines) | A sibling button is not enough — the `data-act` audit is per row, and the parent row still reads as an object. Fix: every row opens session/peer/pending/channel, the feed gained its own filters, the stamp says "all N lines" |
| KI-273 | `fleet channels` printed 3081 while the home configured 3092; `fleet channels` was absent from `fleet help` | It read `DEFAULT_PORT`, not `ServiceConfig::from_settings`. Both fixed, help pinned by a test |
| KI-274 | The port landed with `cargo fmt --check` red (128 hunks / 17 files) and `clippy -D warnings` red (18 lints) — verified in a clean worktree, not this session's edits | The port's "gate green" never ran the gate's first two steps |
| KI-275 | Found later by the **PWA** work's remote assertion: the remote face answered `GET /` with the pairing page before reading the cookie, so the redirect a successful pairing performs (`location.href='/'`) looped back to the pairing form — a paired phone could never reach the shell at the root | The pairing match ran before the device store. Fix: resolve the device first; `/pair` stays the unpaired page. Pinned in the pairing test |

**Verdict given: repair, not rip-and-replace.** The surface already spoke the
editorial console's dialect (`pageHead`, `statStrip`, `filter-bar`, `boardCard`,
the dock inspector, the rail count, the once-only token) — the defects were
correctness, not taste.

**Verification that counted:** Playwright at 1600/1180/900/430 across Bots,
Sessions, Memory; the `data-act` audit; pairing → copy → reload (token absent);
stop/start checked at the socket (3092 closed then open). Gate green at that pass:
1276 lib tests; then 1277 after the PWA increment. All of it **uncommitted** at
session end.

**The design rule that survived the audit**, worth reusing: a surface adds NO
new design vocabulary, only the console's existing one, and tiles that count
their own card navigate to it while chips filter lists they are actually part
of — a chip that empties a list because its items live elsewhere is a dead end
with a label. And counts exist to filter; the tower stays the one cockpit.

**Two things the owner meets even with the surface on master** (2026-10-20):
the face on port 3080 was started the night before the port landed and
therefore served the old embedded assets — no Bots anywhere until the
process was restarted ([[verify-the-binary-is-newer-than-the-source]]);
and after the restart the surface rendered empty states because the
service is **off by default** — `~/.fleet/config.json` had no `channels`
block, so the owner needs `fleet settings set channels.enabled true` (then
restart `fleet gui`) or `fleet gui --channels`. "The interface is missing"
can mean either; name both and give the switch.

**Why:** owner-facing, and the next session must not re-audit from scratch or
assume a plan's completed status means the surface renders.

**How to apply:** never quote a plan closure as evidence a surface exists or
works — render it ([[verify-what-renders-not-what-commits]]) at the widths the
feature exists for, not only desktop. A CSS media query does not beat
attribute-selector state rules. Related:
[[fleet-remote-port-onto-master]] (what landed), [[fleet-pwa-installability-borrow]]
(the install surface built on top), [[fleet-gui-editorial-desk-redesign]],
[[fleet-tui-revamp]], [[fleet-lane-service-target-architecture]].
