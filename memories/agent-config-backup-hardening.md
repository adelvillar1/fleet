---
name: agent-config-backup-hardening
description: "daily launchd backup of the churn-prone harness configs (zcode provider/setting, kit config, fleet home configs) at ~/.local/share/agent-config-backups/<day>/, 30-day retention, secrets excluded; script ~/.local/bin/agent-config-backup, agent local.agent-config-backup (2026-10-16)"
type: reference
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

The owner's backup hardening (2026-10-16, after the provider-config validation scare): a launchd agent (`local.agent-config-backup`, `~/Library/LaunchAgents/local.agent-config-backup.plist`) runs `~/.local/bin/agent-config-backup` daily at 09:00 (launchd coalesces runs missed while asleep). Each day gets a folder under `~/.local/share/agent-config-backups/<YYYY-MM-DD>/` holding: `zcode-v2__provider_config.json`, `zcode-v2__setting.json`, `zcode-router__config.json`, `fleet__config.json`, `fleet__routing.json` (path-prefixed names, copy-through-temp-and-rename). Retention prunes folders older than 30 days. Run log: `~/Library/Logs/agent-config-backup.log`.

**Why:** zcode desktop migrations and hand-editing churn these files; the `.bak-kit` convention proved its worth in the [[zcode-provider-config-degrades-on-validation-failure]] incident.

**How to apply:** to protect another file, add one `source|name` line to the script's manifest. Secrets are deliberately excluded — never add a `.env` to the manifest. Restore = copy a day's file back, or hand-apply the field that differs. To add files for fleet-side work the owner should mention them; the manifest is the single seam.
