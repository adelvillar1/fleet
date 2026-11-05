---
name: relay-join-secret-rotation-2026-11-01
description: "project 2026-11-01 — the relayed faces restored with ROTATED secrets: OpenShip project fleet-relay (services relay + bot) carries RELAY_JOIN_SECRET via `openship service env set -p fleet-relay --secret` + `env apply`; both FLEET_* proofs now live in ~/.fleet/.env AND the plist env; KI-393 fixed the chain (737dadc); launchctl kickstart rejoined both relays"
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

2026-11-01 late session: the owner's "relaunch it, I don't have a clue" ended
in a full **secret rotation**, because the old proof existed nowhere readable
(not `.env`, not the plist, not shell history — it had lived only in the
environment of the shell that launched the original face, and died with it).

**The rotation (the repeatable recipe):**
1. Generate two fresh secrets (`openssl rand -hex 32` each — the console and
   bot relays are separate deployments with separate values) into temp files.
2. `openship service env set relay -p fleet-relay --secret
   RELAY_JOIN_SECRET=<v>` and the same for `bot`; then `openship service env
   apply <service-id> --project proj_EhUyMW71ICHc5xzG` — applies to the LIVE
   container (the restart verb's own help names env apply), no redeploy.
3. Append `FLEET_RELAY_JOIN_SECRET=` / `FLEET_BOT_RELAY_JOIN_SECRET=` to
   `~/.fleet/.env` — this is the durable home.
4. `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` → both relays joined
   (log: `joined https://fleet.opsh.io` / `joined https://fleetbot.opsh.io`).

**Why both the plist AND .env now carry the values:** the plist's
EnvironmentVariables (which had only PATH) is what launchd actually exports,
so the plist copies made the CURRENT (pre-fix) binary work; KI-393's fix
makes `.env` authoritative for every future build. Belt and braces, both
sides of the fix.

**KI-393 (closed at `737dadc`), the root cause worth remembering:** the
relay check read `std::env::var` directly while its own comment claimed the
own-home chain — but NOTHING loads `.env` into the process env
(`load_fleet_env` reads `fleet.env` and deliberately skips `*_SECRET` names;
every other credential resolves through the dotenv map AT USE TIME). So any
face launched outside an exporting shell (launchd, nohup, cron) refused at
launch even with its proof sitting in `.env` — the exit-1 respawn loop that
looked like a mystery until the service log's `needs FLEET_RELAY_JOIN_SECRET`
line was read. Fix: `relay_join_proof(home, name)` →
`home::resolve_setting` over `read_dotenv_with_process_vault` (env wins,
`.env` falls back, empty = unset) at both console and bot sites, pinned by
`the_join_proof_reads_the_env_then_the_home_dotenv` (a test-unique name so a
process-level value can never reach the assertions).

**The diagnostic chain when a LaunchAgent face exits 1:** read
`~/Library/Logs/fleet-gui.log` (the plist's StandardOut/ErrorPath — the
launch's own words land there), NOT `/tmp/fleet-face.log` (a nohup relaunch's
own file). Secrets must never be echoed — lengths only, values via
`$(cat …)` expansions in commands.

Related: [[fleet-gui-is-a-launchagent]] (kickstart vs bootout+bootstrap after
plist edits), [[both-faces-required-console-and-bot]],
[[remote-relay-openship-plan-2026-10-29]] (the OpenShip deployment runbook).
