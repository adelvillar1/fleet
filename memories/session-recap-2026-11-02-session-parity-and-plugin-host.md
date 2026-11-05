---
name: session-recap-2026-11-02-session-parity-and-plugin-host
description: "project 2026-11-02/03 — the recap at docs/recaps/2026-11-02-session-parity-and-the-plugin-host.md: FIVE plans implemented whole (provider-session-headers, session-parity-remainders, usage-in-calibration, the face plugin host, and the plugins-ux rethink), the 1.99.1 alignment verified, the repo gone public (5d4ee6e), the recap extended through the plugins-ux plan (808cba2, pushed ea70a53), gate 2,012/0, harness 703/0"
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

The session's close-of-record: **four plans implemented whole and landed**,
journaled at `docs/recaps/2026-11-02-session-parity-and-the-plugin-host.md`
(committed `6aa368f` with the built-ins fix that followed the owner's
review).

1. **`docs/plans/2026-11-02-provider-session-headers.md`** — completed
   (`3d246a1`, `16c30e3`, `3c1d32b`): the roster `headers` field,
   `{session}`/`{env:NAME}` placeholders, the two-gate refusals, the
   names-only card.
2. **`docs/plans/2026-11-02-session-header-parity.md`** — completed
   (`c554a32`, `a17e115`): the opaque resume-stable UUID session ids
   seeded per conversation (idempotent for re-routes), the
   token/control-char rules, the request-id capture, the writer's trap
   warning.
3. **`docs/plans/2026-11-02-session-parity-remainders.md`** — completed
   (`c64d3fd`, `509e0e3`, `d816704`, `5c0c5ab`): the Responses adapter
   as a **wire dialect only** (`store: false`, no `previous_response_id`,
   the full history every turn — hard rule 5), `{trace}`, the W0 probe.
4. **`docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md`**
   — completed (`0253e89`, `7aa0544`, `43b37e5`, `0259bb4`, `140c739`):
   the usage columns + spend card, the anomaly pass, the trajectory shape.
5. **`docs/plans/2026-11-02-face-plugin-host.md`** — completed
   (`dea8393`, `a1b7f5c`, `ca9a5cd`, `eabe031`, `07bb991`, `c8dde93`):
   the format + registry + CLI, the mounting + management, the two
   tiers' frames, the eval suite, the creator drafts, the promotion
   ladder, the demo bundle.

**The 1.99.1 alignment** (the owner's *"we had to upgrade rust to 1.99.1
and that generated some conflicts, the session cleaned all that up"*): the
cleanup commit `eabe031` carried the 41 lint fixes AND the co-tenant's
parked plugin work on `face-plugin-host`; the follow-up session verified
the **committed** state (fmt 0, clippy 0, build 0, test 1,996/0, harness
678/0) and landed the finish at `c8dde93`. The gate re-run on the
committed tree — not the commit message's claim — is the verification
(see [[a-parallel-session-can-reintroduce-your-fixed-lints]]).

**Named pending halves** (the plans' own verification steps, recorded as
pending, not claimed): the responses probe's live run, the plugin demo's
scratch-home creator run, the eval suite's rendered browser-lane pass, the
usage plan's live provider leg.

**The close-out delivery pass (same evening).** The owner's *"is there
anything pending to merge, the app rebuilt, and relaunched?"* — merges
none; the face was **2.5 hours stale** (binary 18:16 from `3ebf1fe`, served
`app.js` with zero `plugins` occurrences and 950,592 bytes vs the source's
961,837). The reload: `git -C ~/Projects/fleet-master-wt merge --ff-only
origin/master` → `cargo build --release` there → `launchctl kickstart -k
gui/$(id -u)/com.fleet.gui` → the served bytes re-proven (`grep -c plugins`
= 36, byte count exactly the source's), `/` 200, both relays rejoined. The
delivery state after: committed, pushed (tip `c8dde93`), rebuilt,
restarted — all four legs measured, not asserted.

**The session's reusable lessons**, each its own memory:
[[a-harness-find-returns-an-array]], [[a-python-literal-sweep-eats-non-literal-sites]],
[[a-parallel-session-can-reintroduce-your-fixed-lints]],
[[a-mid-build-park-can-be-committed-by-the-cleanup]], [[a-1991-lint-fix-can-be-wrong]],
[[a-hedged-cross-module-seam-row-drifts-the-arch-gate]],
[[the-credential-sweep-reads-the-faces-copy]].

**Extended 2026-11-03 — the session's second half is in the same recap:**
the plugins-ux plan (`docs/plans/2026-11-03-plugins-ux.md`, completed at
`5f56b86` — the owner's "atrocious" verdict, three approved mockups, the
translation: three grouped status-first sections, the presses, the install
door, the trust disclosure, the rail badge), the built-ins fix (`6aa368f`
— the plugins surface now lists the 16 built-in plugins: the second-brain
tool plugin, the seven lanes, the eight MCP servers), and the flat-shape
seam fix (`2a8d0f6` — the nested wrapper the live face exposed). The face
rebuilt on the installed path (`~/.fleet/bin/fleet`) and verified live —
see [[plugins-ux-plan-drafted-2026-11-03]] and
[[fleet-gui-is-a-launchagent]] for the binary-path move.

**The recap itself was extended** (`808cba2`, pushed `ea70a53` after
merging the co-tenant's `5d4ee6e` — **the repo went public**, the stale
private claims and the version corrected): the summary now reads five
plans, the plugins-ux plan has its own block with the criteria status, the
second-half section carries the full arc (the mockups-first ruling, the
two live-face findings, the binary-path move, the Tauri rebuild, the
repo-public merge), the commits table gained the four plugins-ux rows, the
open questions name each plan's pending live leg, and the notes recorded
three transferable lessons (the mockups-first workflow, the Chrome
`--screenshot` capture trap, the `El.find`-returns-an-array lesson that
bit twice). The closing numbers: gate 2,012/0, face harness 703/0 across
67 cases.

Related: [[usage-plan-amended-trajectory-shape-2026-11-02]],
[[creator-mode-plugin-host-exploration-2026-11-02]],
[[session-parity-remainders-plan-drafted-2026-11-02]],
[[session-header-parity-plan-drafted-2026-11-02]],
[[plugins-ux-plan-drafted-2026-11-03]],
[[rust-1991-toolchain-move]], [[session-recap-2026-11-02-two-plans-landed]]
(the earlier recap, superseded by this one's wider arc).
