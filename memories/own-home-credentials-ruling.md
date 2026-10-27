---
name: own-home-credentials-ruling
description: "fleet resolves credentials from the process environment and <FLEET_HOME>/.env only — no other harness's config file is ever read, the 2026-10-16 ruling that removed the dsh credentials.yaml leg."
type: project
date: 2026-10-16
basis: confirmed-by
tier: durable
shape: [change]
phrases: [the router reads Fleet's own home and nothing else; one stored key serves one harness; the harness store is the fork's, not Fleet's]
missing: []
recall-count: 275
superseded-by:
valid-until:
---

The owner's 2026-10-16 ruling: fleet's router should never be reading a config file from a different harness. Credential resolution is exactly the process environment then <FLEET_HOME>/.env, enforced by home::resolve_setting and by removing the harness_credentials_path/harness_refs helpers that made the dsh ~/.dsh/.credentials.yaml refs read possible. The 2026-10-16 roster migration already put every roster key in <FLEET_HOME>/.env, so nothing was lost; the boards' read-only API seams are a separate ruling and untouched.
