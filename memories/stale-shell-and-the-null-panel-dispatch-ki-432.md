---
name: stale-shell-and-the-null-panel-dispatch-ki-432
description: "KI-432 (2026-11-05) — the owner's \"new conversation stays on the latest\" was a stale shell tab, but it exposed a real class: one null PANEL_EL entry aborts the whole route dispatch"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner reported (2026-11-05): pressing new conversation "just stays
on the latest one and never creates a new one." Diagnosis, driven live
over CDP against the served face at 127.0.0.1:3080: the plain
create-then-send path WORKED (session/new fired once, the run carried
the new session's id, the face landed in it), the server's
/api/session/new answers 200, and both served assets carry
`Cache-Control: no-cache` (the SW caches only app.css/app.js at the
`?v=HASH` address, never the document). The surviving failure mode for
their symptom: **the browser tab predated the desk delivery and ran the
shell it had loaded** — the cure is a hard reload (cmd-shift-R).

But the report exposed a real defect in the desk face (KI-432, closed,
pushed 4e269ff): a panel element the served page lacks binds **null**
in a real browser (only the face harness's stub refuses it by name),
and ONE null entry in showRoute's `Object.keys(PANEL_EL).forEach`
hidden-loop threw mid-loop — aborting the whole dispatch, so nothing
detached, nothing opened, the surface froze on whatever it showed.
**Both PANEL loops now skip a null entry, with the failure mode named
at the guard.** The lesson: the harness's refused-missing-id rule
protects build time, but the real browser tolerates null until USED —
any loop over id-bound elements needs its own null skip.

Also learned driving it: the face's real element ids are `#task` and
`#composer` (NOT `composer-task`/`composer-form` — wrong-id probes
read as "no form"); a synthetic form submit via dispatched Event works
for driving send; `face` is IIFE-scoped (unreachable from CDP eval —
read DOM state instead); the composer textarea is NOT the form's
named submit target, so a raw `submitter.click()` can navigate.

Operational state at diagnosis: the relays flapped overnight (241 poll
failures — TLS timeouts/resets to both opsh.io relays), recovered by
06:37; `/Applications/fleet.app` is from Nov 4 13:06, two deliveries
old (it runs its own bundled server and never got the desk or the
transplant — installing the fresh dmg is the named follow-up). Probe
debris: two sessions archived by name with the owner's approval
(b6dd5793, bbff378e — the second needed /api/stop first; archive
refuses a run in flight). A third debris: `fleet remember` with
FLEET_HOME=scratch still wrote memories/proof-note.md into the REPO's
corpus — the memory root resolves from the WORKING DIRECTORY
(resolve_root walks up from cwd), not from FLEET_HOME; removed, and
scratch-home probes that call remember must also set cwd.

**Resolution and the false alarm (owner asked "was it fixed or you need
my approval?"):** fixed and deployed, nothing needed approval — the
guard is in the served shell, verified by grepping for
`abort the dispatch`. The first verification declared "the deploy
didn't take" because it grepped the single-line phrase
`must not abort the dispatch`, which the comment's own line wrap
splits — a multi-line comment defeats a single-line grep; verify with
a fragment that survives wrapping or grep the code, not the comment.
Two stranded pieces landed in the cleanup: the 3.9-safe coverage score
head in services/sys1/src/sys1/tasks.py had sat UNCOMMITTED since the
coverage beat (the commit's explicit-paths list missed the vendored
file — a fix spanning Rust plus a vendored file must carry both paths)
landed at 75063ec; and the recall-proof's recall-count bump committed
as its chore.

**Why:** "it stays on the old one" from this owner means check the
served flow live first — but the surviving mismatch class is real and
was a one-line land.

**How to apply:** when a face behavior is reported broken, drive the
served page over CDP with a fresh profile before theorizing; verify the
server log for the era; and guard every id-bound loop against null
bindings. See also [[verify-the-binary-is-newer-than-the-source]]
(the stale-SPA-tab lesson this extends), [[fleet-gui-is-a-launchagent]],
[[a-relay-join-secret-lives-outside-the-dotenv]].
