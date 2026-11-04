---
name: fleet-scheduler-lives-inside-the-gui
description: "project 2026-10-26 — fleet's cron scheduler is spawned inside the `fleet gui` process, so jobs fire only while the face runs and a long-lived process keeps running old code until restarted"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

`cron::Scheduler::spawn` is called from the GUI bridge, not from a launchd
plist of its own — `com.fleet.gui` is fleet's only LaunchAgent, and its
`fleet gui --no-open --remote` process hosts both the face and the
scheduler. Three consequences, all paid for on 2026-10-25/26:

- **Jobs fire only while the GUI runs.** Fifteen jobs (the corpus
  refreshes, staggered from 00:10Z). No GUI process → no fires, silently.
- **A landed writer does not take effect until the GUI restarts.** The
  `embed_timing` writer landed 10-24 17:31; the GUI process had started
  10-23 3PM, so its scheduler ran pre-writer code and fired jobs that
  recorded nothing — the stream held zero rows for a day while the cron
  card read "last run 2026-10-24 · completed". A running process is a
  frozen binary.
- **A restart runs the catch-up.**
  `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` (check
  `curl -s 127.0.0.1:3080/api/state` first — nothing in flight) → new
  PID, and the catch-up immediately ran the missed fires, each recording.

**How to apply:** when a measurement or scheduled behavior "hasn't
happened yet", check the recorder is live code on a RUNNING process
before concluding anything about the data; the restart is the deploy step
for anything the scheduler runs. The face is on port 3080 (loopback plus
the routable `--remote` listener). Related:
[[fleet-run-control-and-timeout-gaps]], [[fleet-pairing-qr-without-tailscale-serve]].
