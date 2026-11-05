---
name: settings-ux-plan-2026-11-01
description: "project 2026-11-01/02 — the "settings UX is atrocious" plan (G1–G11, U1–U11) is COMPLETE: its own Status pass — 2026-11-02 table audited all eleven against the code and found every one built (U1–U10 landed across 36c5754/b03520b/5a65a6a/d13ca29/d816704/d5f7416, U11 the gate green for that pass); the plan file still carries status: active, so read its status-pass table and not the frontmatter or the boxes"
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

2026-11-01: the owner's verdict — *"settings user experience is
atrocious"* — landed on the layer ABOVE the 10-29 revamp (which shipped
complete, S1–S10 green: rail, one-section-at-a-time, one-line rows,
pane editor, filter, budgets, addressability). Research: the local UX
corpus (silent success / submit feedback / inline validation /
no-results / error-recovery rules) + choice architecture (default
effect, reversibility, feedback, translating attributes, choice
overload) + the 10-29 plan's own unshipped promises read against the
code.

**Plan: `docs/plans/2026-11-01-settings-ux.md`** — drafted `410eb4b`,
then four same-day owner steers revised it: `f7681c7` (the model is
the section), `9b246a3` (the caps leave the face too), `fa71ffd` (the
flow rule + U9/U10; the gate renumbered U11). All on master via the
fleet-master-wt worktree.

The gaps, each with code evidence:

- **G1** a save's confirmation lands OFF-surface — `notice()` appends
  to `face.stream`, invisible at `#/settings`.
- **G2** a lost connection is swallowed (`writeSettings`' empty
  `.catch`). **G3** no revert to a compiled default. **G4** destructive
  acts unlabeled/unrecoverable. **G5** the filter dead-ends silently.
- **G6** the filter finds then DROPS the item — **S9's three-part item
  route never shipped** (`parts[2]` handler absent).
- **G7** validation only after the round trip. **G8** refusals name no
  door. **G9** raw numbers without consequence. **G10** two
  vocabularies (GUI marks vs the CLI's `unset · default`).
- **G11** the models section leads with machinery (tiers, gates,
  `billing · kind`).

**Increments**: U1 on-surface confirmation; U2 revert + blast radius;
U3 the filter that finishes (suggestions, the ITEM route, keyboard);
U4 blur-time validation; U5 every refusal names the door; **U6
WITHDRAWN** (money removed); U7 one vocabulary; U8 the model is the
section; U9 a chained setting completes on one surface; U10 the fresh
home's ladder; U11 the gate.

**U6 withdrawn — money REMOVED (owner, 2026-11-01).** *"budget is not a
concern… I use coding/token plans"*, then *"again money I don't care"*,
then, after I relocated the three caps to environment's catch-all,
*"if anyone cares about that, they can fork the harness and add it"* —
so the caps leave the GUI entirely (`9b246a3`). `fleet settings` and the
document still write them. **The seven-section rail is still open**: the
real face still renders `money` as a section in `app.js`.

**BUILT 2026-11-01 (partly):**

- `36c5754` **the models section opens on the MODEL**: every catalog
  model as one `provider/model` row (kind `model`, `auto` first), the
  pin marked current, a press opening the model's card whose act is
  `writeSettings('set', {key:'pin', value:pair})`; the accounts list
  carried **connect a provider** (endpoint → key → name); the walk
  (tiers + gates) demoted to a card below. Six `MODEL …` harness checks.
- `2858e11` the design session's master merged (14 commits), no
  conflicts.
- `23eb430` **the live probe's catch**: a provider written without
  `key_env` is refused by the roster load, which then emptied EVERY
  list — only Playwright on the real binary saw it. The write stamps
  the derived `<ID>_API_KEY` server-side; the model list ranks current
  first, then keyed, then the roster's order.
- `f6e2d15` **the connect card answers on itself** — a `role="alert"`
  derivation line carries the discovery's sentence (down tone) and
  keeps the values for the retry.
- `0bfddfe` **the provider catalogs are the providers' own, and the
  actions are not buried.** Why the surface had shown 1 model where
  zai lists 11: the reload (`provider-discover`) refused every
  COMPILED provider (`names no configured provider`) and so never ran.
  `reload_entry` now falls back to the roster when the config has no
  entry, inherits a missing `base_url`/`key_env` from it (the real
  `ollama-cloud` entry carried neither → derived an impossible
  `OLLAMA-CLOUD_API_KEY`), and tolerates a roster the file made
  unloadable (a test caught the first cut refusing a valid reload).
  The model list is bounded (6 rows, "68 more models") so the accounts
  card — connect FIRST, then one row per provider — sits directly
  under it.
- `e448b0c` the merge with origin's `737dadc` (KI-393).
- `b03520b` **reload every provider** — the accounts card's second
  action walks each enabled non-lane provider through
  `provider-discover` in turn and answers `reloaded N of M` naming the
  ones that did not (liquid always among them: its listing 404s).
  Three `MODEL …` harness checks; harness 618/56.

**The catalog, measured live on the real home (2026-11-01):** roster
said 36 models, the providers list **72** — zai 11, token-plan 20,
deepseek 2, xiaomi 6, stepfun 6, ollama-cloud 25, liquid 1 (its listing
404s — `.../openai/v1` has no `/models`), blend1 1 (a lane). Only
**deepseek declares settings** (`context_length` 1,048,576 /
`max_output_tokens` 393,216) → rows read `1M window`; the other OpenAI
listings carry no window fields at all, so "no settings shown" is the
truth, not a bug. The reload writes each catalog as a config override
(seven now), each removable from its card.

**+ `b03520b`** (pushed `e448b0c..b03520b`) **reload every provider** as an
app action; **`5a65a6a`** (pushed `b03520b..5a65a6a`) **the onboarding's
vault step** — the owner's *"the onboarding or lack of it is what
matters"*: `STEP_VAULT` between the first account and the lanes, proved by
real state (`10 sealed · 5 still plaintext` on their home, whose step
therefore reads done=false), landing on the vault's own form. See
[[a-setting-nobody-asked-for-is-onboarding-debt]].

Gate at `5a65a6a`: fmt 0, clippy 0, build 0, **test 1,919/0**, face
harness **618/56**. Pushed `6012b76..f6e2d15`, `737dadc..e448b0c`,
`e448b0c..b03520b`, `b03520b..5a65a6a`. The tree also carries 22
uncommitted `memories/*.md` files that are the runtime's own
`recall-count` churn — never stage them with a settings commit.

**Still open**: U1 (confirmation off-surface for every other write), U2,
U3, U4, U5, U10 (fresh-home ladder), the chained cards for the
provider/lane/job/scan (U9 — the mockup has them; the app has connect,
the pin, and the provider card's key/endpoint/reload/remove), and the
seven-section rail.

**U9 verified open (2026-11-02, on the owner's challenge "are you sure
that is not already completed? check git history"):** three legs — the
plan file's last commit is still `fa71ffd` (the 2026-11-01 revisions;
no commit has ever ticked it), no landing commit exists anywhere
(searched all branches for U9/account-card/chained), and the last
`app.js` settings commit before today's design work is `b03520b` (the
U8-era catalog reload). The remainders plan's W4 (the face's headers
row) waits on this card — see
[[session-parity-remainders-plan-drafted-2026-11-02]].

**U9 is HALF-built, and the plan's checkboxes cannot be trusted
(2026-11-02, the owner's "did you leave something called U9 open?"):**
read against `app.js`, the provider card (`providerSections`,
~line 15387) DOES carry endpoint, key, the catalog with on/off
toggles + windows, removal with blast radius, and the lane/tools
facts — but the **walks half is absent**: no up/down/stop ordering
and no add-to-a-walk picker exists anywhere. Meanwhile **all eleven
boxes are still `[ ]`**, including things that shipped (the model
first, connect, catalog discovery), and U6's `money` section still
renders as a section (`id: 'money'`, ~line 14957). So the plan file
is stale in BOTH directions — audit each criterion against the code
before trusting either a tick or a blank. The next piece is U9's
walks ordering + add-to-a-walk, because
[[session-parity-remainders-plan-drafted-2026-11-02]]'s W4 (the face's
headers row) is sequenced behind that card.

**U9 LANDED 2026-11-02 (`d13ca29`, plus docs `02186c2` and the unblock
`c9c6139`; pushed `5c0c5ab..c9c6139`).** The provider card
(`providerSections`) gained **the walks it serves**: one row per
workload whose chain carries one of that provider's models, with
**raise / lower** swapping its candidate one step, **stop** removing
every one of its models (refusing a write that would empty the walk),
and an **add-to-a-walk picker**; every act writes the whole order
through the same `workload` op the tier card uses and repaints the row.
The job card gained **the next fire**, computed in the settings read by
`Schedule::next_after(Minute::now())` at read time (disabled → null,
unparseable → null rather than an invented time).
- Proofs: nine `SET/U9` harness checks (624 checks / 56 cases; the El
  stub's `select()` now takes a value so a picker can be chosen), and a
  live scratch-home round trip (`tmp/u9-probe.js` against :3099 with a
  seeded `routing.json` walk) where stop removed `ollama-cloud/glm-5.3-flash`
  and add wrote it back; gate 1,747/0.
- **The first cut clipped the workload name to `stand…` and stacked
  `stop` into `st p`** — seen only in the screenshot; the row is now
  `.cfg-walk` (name on its own line at full width, the order beneath,
  the three acts in their own column). Row order in the card matters:
  walk rows carry `data-role="walk"` and the add row `data-role="walk-add"`
  because a harness selector counts `.cfg-fallback` rows.
- The remainders plan's W4 (the face's headers row) is now **UNBLOCKED**
  in its own file (`c9c6139`), with the cut named.

**The remainder, after U9 (the census, 2026-11-02).** What is left of
this plan is U1–U7 and U11; U3/U4 also owe the S9 item route. **U6
(money) is now the cheapest real win** — the section still renders and
the rail/harness still read eight, so deleting it touches the section
list and one harness assertion. The full open-work inventory across all
plans and the ledger lives in
[[completed-plan-unticked-criteria-census-2026-11-02]] (which also files
KI-398 for the unticked-completion shape this plan exhibited).

**The plan's status pass (2026-11-02, appended to the plan file).** The
eleven boxes had never been ticked as the work landed, so they could not
be read as state — the audit walked each against the code: **U8, U10
built; U9 built; U1–U4 open** (no settings-level confirmation card, no
revert on a written knob, no no-results card, no client-side numeric
check); **U6 open** (`money` still renders, the rail still reads eight,
the harness still pins eight); **U5/U7 unmeasured**. Lesson:
[[a-plan-never-ticked-is-not-a-status]] — a plan's ticks are state only
if they were written at the moment the work landed.

**Remaining, verified against git and the tree (2026-11-02, after the
owner's "compare against git history first").** U1–U6 are genuinely
unbuilt, and the check was by absence in the code AND no commit: no
settings-level `saved <label>` on-surface line (U1), no scalar defaults
in the read and no revert act on a written knob (U2), no no-results card
and no `#/settings/<section>/<item>` item route (U3), no blur-time
validation (U4 — the only two `blur` handlers are the design rename
inputs), `money` still builds a section so the rail reads eight (U6).
U5 and U7 are unmeasured/unverified rather than confirmed missing. U6
remains the cheapest real win: deleting the section touches the section
list and one harness assertion that pins eight.

**The other plans waiting on U9 are cleared (2026-11-02, `30d7a8d`).**
The batch status pass over the eight completed plans
([[completed-plan-unticked-criteria-census-2026-11-02]]) found no plan
blocked on this one except the remainders plan's headers row (C4), which
stays open but is no longer waiting — it has the card. So this plan's
U1–U7 is now the ONLY settings-shaped queue, and it is the top of the
real (as opposed to bookkeeping) work list, ahead of the residuals
plan's R1 (O1's relay-join divergence).

**The headers row LANDED on this card the same day (`d816704`).** The
remainders plan's W4 — the face's headers editor — was built into
`providerSections` the evening U9 landed (the read carries the CONFIG's
headers via `RosterEntry.headers` from the raw candidate, the card edits
them with add/remove and the live trap warning, the save rides the
provider write); the remainders plan is now completed 6/6. This plan's
own queue is unchanged: U1–U7 (U6 the cheapest real win).

**Driving the live face**: `FLEET_HOME=~/.fleet
./target/release/fleet gui --port 8898 --no-open` from fleet-master-wt
is the real home on the new build. A shell-launched background face
DIES when the shell exits (the owner hit ERR_CONNECTION_REFUSED twice);
relaunch as a tracked background task, and the settings read takes ~2 s
on a real home (the "reading" card until it lands — don't shoot the
page at 1.5 s and call it broken). The scratch home needs the fake
provider (`node /tmp/fake-provider.js`, 127.0.0.1:8977). The owner's
own face is the LaunchAgent `com.fleet.gui` on **:3080** (`--remote`,
relays fleet.opsh.io / fleetbot.opsh.io), pointed at
`/Projects/fleet-master-wt/target/release/fleet` since the owner's
"let's make it part of the app" (PlistBuddy `Set :ProgramArguments:0`,
plist backed up `com.fleet.gui.plist.bak-20261102`, then `bootout` +
`bootstrap` — `kickstart -k` re-spawned the old binary), so :3080
serves the settings build: `curl 3080/app.js | grep -c "reload every
provider"` → 1, and a Playwright pass found 6 visible model rows, both
actions, 8 providers, no page errors. Copying a real home for a probe: copy
`config.json routing.json .env vault.json fleet.env ui.json` only — a
full `rsync` of ~/.fleet hangs on the huge indexes.

**U1–U7 and U10 BUILT 2026-11-02 (`d5f7416`, the closeout plan's E3 —
[[open-work-closeout-2026-11-02]]).** U1 a board-level status line
(`settingsSay`/`paintSettingsStatus`: `saved <label>`, the refusal's
words, the unreachable sentence; all 22 settings notices route there and
NOTHING lands in the conversation stream); U2 `settings::unset` +
`compiled_default` with a `revert to the default` act on a file-sourced
knob (the read carries `default` per scalar); U3 the no-match card naming
the sections it could mean plus `openSettingsItem` walking
`#/settings/<section>/<item>` (arrows/Enter on the list, a bare deep link
opens and marks the row); U4 blur validation beside the field holding
the save; U5 the env/ceiling/vault derivations each carry their next
step; **U6 `money` is GONE** (the landing map's `money` step with it,
the harness now pins seven rails and no `BUDGET_USD_DAY` row); U7 the
One vocabulary table in `docs/features/gui.md` cited from
`FUNCTIONAL-SPECIFICATIONS.md`; U10 the door stays up while the `home`
step is incomplete (not only while `ready` is false — a vendored lane
made a fresh home answer) and renders the read's ladder done/now/next,
re-measured live on `/tmp/fleet-fresh-home` (door shown, ladder shown,
counts/composer/chip/pane down). Harness 656 checks / 57 cases
(`theSettingsBoardAnswersForItself` pins U1–U5). Also fixed in passing:
the vault row's card builder returned a bare node (KI-400,
[[a-card-builder-returns-sections-not-a-node]]).

**COMPLETE — every criterion built (the plan's Status pass — 2026-11-02,
read 2026-11-02 on the owner's "some of this work is already done").**
The status-pass table in the plan file is the state, and it audits
**U1–U11 all built**, each with its evidence: U1 `settingsSay` paints
the board's own line and all 22 settings notices route there; U2
`settings::unset` + `compiled_default` (ten settings tied to the
constant the code falls back on); U3 the no-match card + the
`#/settings/<section>/<item>` route; U4 blur validation beside the
field; U5 the env/ceiling/vault derivations each naming their next
step; U6 `money` gone from the face (harness pins seven rails); U7 the
One vocabulary table; U8 the model-first section; U9 the account card's
walks + the job card's next fire; U10 the door up while the `home` step
is incomplete, re-measured live on `/tmp/fleet-fresh-home`; U11 the gate
green for that pass. **The plan's frontmatter still reads
`status: active` and its boxes were never ticked** — so the frontmatter
and the boxes both read as "open" while the work is done. This is the
plan the warmup summary got wrong; see
[[a-memory-index-line-is-not-the-plans-state]].

Mockup in the session visualizations dir
(`settings-surface-mockup.html`) with a mock-only control bar.
Related: [[a-chained-setting-completes-on-one-surface]],
[[the-surface-keeps-the-machinery-and-hides-the-answer]],
[[a-list-that-came-from-the-roster-is-not-the-providers-catalog]],
[[a-green-pin-can-hide-a-broken-first-run]],
[[two-sessions-one-repo-master-worktree]],
[[fleet-settings-abstraction-search]].
