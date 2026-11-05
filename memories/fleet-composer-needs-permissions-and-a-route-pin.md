---
name: fleet-composer-needs-permissions-and-a-route-pin
description: "owner ask 2026-10-20, SHIPPED 48cd01e — the composer's run bar carries permissions (Ask before / Autonomous / Full access → approvals::Mode) and a route picker (Auto + a per-chat provider pin threaded through route::prepare_with_pin); attachments shipped f51906c; the grant-audit card and Full-access stripe stay unbuilt (the session bar is an open question)"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The owner's ask (2026-10-20):** the conversations page and the chat input
card must carry the permissions granted — "ask before, autonomous, and full
access like zcode has" — must show **auto as provider**, and must "allow to pin
a provider to the chat". The same turn asked for a recommended revamp. The
recommendation was delivered first (design gate) and the owner said "I like
your proposal, let's do it".

**The finding that shaped it: both mechanisms already existed in the crate;
the browser face was the only surface ignoring them.**

- `approvals::Mode` is exactly the three states: `Ask` (default, every call
  prompts), `YesGuarded` (everything but the destructive class runs; destructive
  still asks), `Yes` (everything but the always-ask class). The predicate is
  `Gate::approves_without_asking`.
- `Roster.pin` was global-only (Settings page). There was no per-run pin.

**Shipped, `48cd01e`** (pushed, release rebuilt, face restarted by
`launchctl kickstart -k gui/$(id -u)/com.fleet.gui`):

- **UI → mode mapping:** `Ask before`→`Ask`, `Autonomous`→`YesGuarded`,
  `Full access`→`Yes`. The gate is consulted **after** the owner's remembered
  rules, so a standing "always" still wins. `latched()` reports `true` under
  `Yes`, so the face matches the console/TUI.
- **`GuiApprover` now takes a `Mode`** instead of always prompting — the one-line
  wiring gap that had made the whole ladder invisible in the browser.
- **Wire shape:** `/api/run` accepts `mode` (`ask` | `auto` | `full`, also
  `yes-guarded`/`yes`) and `pin`; a body naming neither reads as the safe pair,
  and an empty pin is auto. `GuiState` holds `permissions` + `provider_pin`, and
  `Bridge::run_with` sets them before dispatching (the command loop is serial,
  so one pair is the whole state).
- **Pin is per run, not config:** `route::prepare` became a thin wrapper over a
  new `prepare_with_pin(home, dotenv, task, specs, pin)` that lays the chat's pin
  over the roster's. The agent thread keeps a per-session `route_pins` map and
  **invalidates the cached prepared route when the pin changed**, so a re-pin
  re-routes instead of serving the stale decision.
- **The picker's values come from `/api/settings` → `catalog.providers`**
  (enabled only), not the top-level document — the top level has no
  `providers` key; that misfire cost one round trip to find.
  [[no-form-asks-for-a-value-fleet-already-has]]
- **Full access colours its selected label and its note red** — the one place
  the design spends visual weight.

**Attachments shipped, `f51906c`** (the owner's next ask on the same card: "we
need to be able to attach files to chat and set a working folder"). The composer
gained an Attach control; text files only, read in the browser, shown as
removable chips, framed ahead of the task, bounded and path-refused server-side.
The working-folder half did **not** ship — it is planned as four phases in
[[fleet-chat-working-folder-plan]], now a committed plan-as-contract at
`docs/plans/2026-10-21-working-folder.md` (`4077099`, gated 12/12) after the
owner corrected that a plan belongs in the repo, not in chat
([[plan-is-a-file-not-prose]]); the tools ignore the builtin root today.

**Still unbuilt from the recommendation (say so rather than implying parity):**
the **grant-audit card in the transcript** whenever the mode or pin changes, and
a removable warning stripe while Full access is on. The persistent session bar
(mode, pin, plan, spend/context moved down, replacing the "no route yet" chip)
is the one the owner pushed back on: "those are already in the conversation
card" — treat the composer run bar plus the existing header figures as the
delivered form rather than re-proposing the bar.

**Test:** one test pins the request parsing (three labels, empty pin as auto, a
body naming neither as the safe pair). Suite 1311.

Related: [[fleet-config-surface-is-the-settings-page]],
[[fleet-gui-editorial-desk-redesign]], [[fleet-sessions-archive-groups-by-project]],
[[rebuild-is-not-commit-push-or-ship]].
