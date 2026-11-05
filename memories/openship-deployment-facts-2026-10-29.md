---
name: openship-deployment-facts-2026-10-29
description: "reference 2026-10-29 — measured OpenShip Cloud facts from the live fleet-relay deployment: the edge refuses non-browser user agents (a python-urllib request 403s at the edge, never reaching the service), folder deploys carry no commitSha, the free subdomain is <label>.opsh.io, and the working two-file deploy loop"
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

Measured 2026-10-29 while deploying fleet's relay as project
`fleet-relay` (`proj_EhUyMW71ICHc5xzG`), service `relay`
(`svc_yTCvrwn3nwbhTquS`), live origin `https://fleet.opsh.io`. The
openship-cloud-deployment skill carries the platform mechanics; these
are the facts this deployment added to it (the skill file is not
editable from the memory system, so they live here):

- **The edge filters user agents.** A `Python-urllib/3.x` request gets
  a 403 from the OpenShip edge itself — it never reaches the service.
  A browser UA passes. Any probe written in python must present a
  browser User-Agent or it will misread an edge refusal as a product
  refusal (this cost a false-negative debugging round).
- **Folder deploys carry no commitSha** — `deployment get` shows an
  empty `commitSha` for `deploy --folder`; provenance is the uploaded
  files. Git deploys carry the sha. A folder deploy of a tiny
  directory (exactly the files the build context needs) avoids both
  the 300 MB cap and the GitHub-App link.
- **The free subdomain is `<label>.opsh.io`** — service create with
  `--expose --exposed-port <port> --domain <label>` answers the origin
  in the deployment record's `url`. TLS is provisioned and verified
  (`ssl_verify_result 0`).
- **Working deploy loop** (verified six times): create project
  `--type services`, `openship service create <name> -p <proj> --build
  . --dockerfile Dockerfile --expose --exposed-port <port> --domain
  <label>`, copy the exact files to a scratch dir, `cd` there,
  `openship deploy --folder --project <proj> --service-ids <svc>`,
  poll `openship deployment get <dep>` for `ready` (building →
  deploying → ready, ~1 min). Note `grep -o 'dep_...'` on the deploy
  output matches TWICE (queued line + the follow hint) — take the
  deployment list's head instead.
- **A stack deploys all-or-nothing — name EVERY service id.** Deploying
  one service of a multi-service project (a bot service added beside
  the relay) failed at preflight with `No image is available for bot; no
  service was activated` — the build phase only built the id(s) passed
  to `--service-ids`, and the preflight then demanded the missing
  sibling's image. Pass both: `--service-ids svc_a,svc_b`. The stored
  service config (build/dockerfile) was correct both times; only the
  id list was wrong.
- **The second relay lives here too** (2026-10-29 evening): service
  `bot` (`svc_eYFSu8b7B-s2XQiw`), free subdomain `fleetbot.opsh.io`,
  same two files, same project — the bot face's origin
  ([[both-faces-required-console-and-bot]]).
- **CLI auth**: `~/.openship/config.json` holds the PAT; `openship
  billing state` answers when authed (`whoami` is not a subcommand).
- **Edge route rules beyond user agents (measured 2026-10-30 on
  `fleetbot.opsh.io`):** a **HEAD** request answers **501** at the edge
  (the service never sees it — probes and health checks must use GET),
  and an unknown path answers **404 `text/plain`** from the edge rather
  than the service's JSON. Do not read an edge 404/501 as the service's
  answer; verify with GET and check the content type.
