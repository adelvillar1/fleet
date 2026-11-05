---
name: fleet-second-brain-is-a-tool-plugin
description: "owner ruling 2026-10-20 — second brain is a separate tool that must be a fleet plugin whose plugin offers refresh actions; built as src/plugins/second_brain.rs forwarding the tool's own POST /api/run (nine PIPE_DEFS jobs, X-Tower header, 4175, SECOND_BRAIN_PORT), verified live"
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

**The owner's ruling, verbatim shape:** "the way I view it is second brain is a
separate tool that needs to be treated as a plugin in fleet, but the plugin
should offer actions within it that allow the user to refresh the data. Second
Brain is my attempt at making sense of an ever growing project landscape as a
solo developer." This SUPERSEDES the earlier framing that the fix is a widened
entry format — see [[fleet-second-brain-is-seven-fields-in-the-gui]], whose
parse-gap findings still hold but are no longer the seam. Fleet does not parse
the tool as its own model; it adapts to it.

**The tool.** One program: the repo `~/Projects/second-brain`, the state home
`~/.second-brain` (a symlink to it), and `control_tower.py serve 4175`. Its
`POST /api/run` takes `{"job": …}`, needs `X-Tower: 1`, answers at once and
runs the job on a thread; `GET /api/status` names the running job, `steps`
(label/status/ms), `current`, a log tail, `started`/`finished`/`error`,
`last_ok`, `built_at`. Jobs = `PIPE_DEFS` keys: all, chats, knowledge, gates,
harness, reports, maturity, loops, tower, mirror. Write-backs also exist for
conflicts (`/api/conflict`, needs Postgres) and lifecycle (`/api/lifecycle`).

**What was built (2026-10-20, uncommitted — a parallel session held the tree).**
`src/plugins/mod.rs` (the seam: "channels carry work in, lanes are model
services, a tool plugin is a program the owner already runs"; deliberately no
trait yet) and `src/plugins/second_brain.rs`: discovery (`control_tower.py`
presence distinguishes "not installed" from "not live"), a 2s status probe, and
`refresh(job)` validating the job against the copied vocabulary then POSTing.
Wiring: `GET /api/second-brain/plugin`, `POST /api/second-brain/act`
(`{"action":"refresh","job":…}`) in bridge.rs/server.rs. Face: a "the tool"
card above the tiles in the second-brain panel (address, build stamp, last
run, job picker, refresh, the tool's own steps with ms, log tail), polling 1s
while a job runs and reloading the derived state when it finishes. Docs:
`docs/features/plugins.md`. Home resolves exactly as the boards do
(`SECOND_BRAIN_HOME` → `~/.second-brain` → `~/Projects/second-brain`), port
from `SECOND_BRAIN_PORT` else 4175.

**Verified.** 7 unit tests against a one-request mock (status parse, idle read,
vocabulary refusal, posted job + guard header, busy 409, dead address, entry
point); clippy clean on the new code; full lib suite 1305 green; the real tool
answers `live: true`, built_at `2026-10-09 01:52`; Chromium against a mock tool
showed select → refresh → `running…` → steps done with ms → build stamp
reloaded, at 1440 and 390, no console errors. One defect found and fixed in
that pass: the job picker reset to `all` on every repaint, so the choice now
lives on the face (`face.pluginJob`).

**Delivered 2026-10-20.** The parallel session committed its own work as
`adfd353` (settings surface) and explicitly excluded the plugin WIP from it, so
the split held without hunk surgery; the plugin then committed alone as
`9bd8b0d` (9 files, 899 lines), pushed, the release binary rebuilt, and the
face restarted through its LaunchAgent — `launchctl kickstart -k
gui/$(id -u)/com.fleet.gui` (the process is `com.fleet.gui`, PPID 1; find it
with `launchctl list | grep fleet`), after confirming `/api/state` showed
`running = False`. Gate 1306 at the commit.

**Transient start-up probe, worth knowing.** The first `/api/second-brain/plugin`
call after the restart timed out at the 2s bound (`live: false`, the address
named) and the card correctly showed "not live … the last state it published";
the next poll answered in ~1ms. The tool's server (launchd
`com.alejandrodelvillar.second-brain-tower`) was rewarming. So a single red
probe right after a restart is not a defect — but the designed fallback is
indistinguishable from a real outage ([[a-fallback-path-hides-the-defect]]),
and if it repeats in ordinary use the 2s probe budget is the first thing to
raise.

**Deliberately not built:** starting the tool's own server (its lifetime is
supervised elsewhere — there IS a launchd job for it, which settles the
argument), conflict dispositions and lifecycle writes (no UI yet), and the
attention queue / project drill-down, which still have a natural home on this
panel.

**Next, and it has since landed (2026-10-20, same session):** the plugin's
face was called "rather underwhelming" the moment it shipped. The owner picked
the Atlas of two mockups and it shipped as `a85472e` — the instrument moved
into the page head, and the panel is now landscape-first with the attention
queue as a rail. See [[fleet-second-brain-visual-redesign-awaiting-pick]].
The owner then held memory and skills to the same bar:
[[fleet-memory-and-skills-knowledge-tables]].

**How to apply:** a tool plugin reads what the tool publishes and forwards the
tool's own actions with the tool's own vocabulary; it never writes the tool's
state or invents an action. A tool that is down is a fact with its address and
the last state it published — the sections below must not disappear. Related:
[[fleet-second-brain-is-seven-fields-in-the-gui]],
[[gui-revamp-browser-face-decision]], [[fleet-channels-bot-surface-gap]] (the
sibling plugin shape), [[verify-the-binary-is-newer-than-the-source]].
