---
name: openship-env-set-apply-pattern
description: "reference — rotating/setting a deployed service's env on OpenShip: `openship service env set <svc> -p <project> --secret KEY=VAL` then `env apply <svc-id> --project <proj>` (live container, rollback-capable) — no redeploy; `service restart`'s own help names env apply as the pending-changes verb"
type: reference
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

OpenShip Cloud env changes (verified 2026-11-01 on the fleet-relay project):

```sh
openship service env set <service> -p <project> --secret KEY=VALUE   # default -e production
openship service env apply <service-id> --project <project-id>       # applies to the running container
```

- `env set` SAVES the value (pending); `env apply` pushes it live through
  the engine's replacement-and-rollback workflow — `service restart`'s own
  help text says "use service env apply for pending environment changes",
  so a redeploy is NOT needed for an env-only change.
- `env get` / `env inspect` show variables MASKED; `env reveal <svc> <keys…>`
  prints actual values (write access required) — never needed for rotation:
  generate a new value instead of reading the old one.
- The CLI is logged into the **cloud** context (`openship status` →
  `api.openship.io`, token auth); `openship project list` finds project
  ids/slugs, `openship service list -p <slug>` lists the stack's services.
- The relay stack: project `fleet-relay` (id `proj_EhUyMW71ICHc5xzG`),
  services `relay` (fleet.opsh.io) and `bot` (fleetbot.opsh.io), each with
  its own `RELAY_JOIN_SECRET`.

Used to rotate both relay join secrets 2026-11-01 — see
[[relay-join-secret-rotation-2026-11-01]] for the full recipe and the
KI-393 root cause behind it.
