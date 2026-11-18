---
name: a-cut-is-for-what-is-worth-distributing
description: "A patch bump and a release cut happen when there is enough change to warrant the 15-minute build plus the gate — not for every fix; where no cut is made and no binary is recreated, failures are cheap because rollback is."
type: feedback
date: 2026-11-17
basis: inferred
tier: durable
shape: [deploy, resolved]
phrases: [every fix doesn't bump the patch number, only needed whenever there is enough to warrant a 15 minute build, 20 minutes or more of gates, every minute change goes through the same zero risk pipeline, if the release is not cut and the binary is not recreated we can afford failure because it's cheap to rollback, I want to get back to that]
missing: []
premises: []
recall-count: 2
superseded-by:
valid-until:
source:
---

Owner ruling 2026-11-17, correcting the 2026-11-11 rule (which read "if we fix something and a new build is created we bump the number" — I applied it as "every fix bumps", and cut two full releases for two small fixes in one day).

The ruling in the owner's own words: "no, every fix doesn't bump the patch number, a patch number or cut is only needed whenever there is enough to warrant a 15 minute build, but the 15 minute build still requires 20 minutes or more of gates, so every minute change goes through the same zero risk pipeline. If the release is not cut and the binary is not recreated we can afford failure because it's cheap to rollback. I want to get back to that."

So:
- A CUT is for a batch worth distributing — a release commit moving Cargo.toml, src-tauri/Cargo.toml, src-tauri/tauri.conf.json, Cargo.lock and a CHANGELOG section, then scripts/release.sh tag + assets.
- A FIX without a cut is the normal case: change the tree, verify proportionally, leave it uncommitted or committed without a version move. No release, no reload, no bump.
- The reason the cut exists at all is unchanged and still real: two builds once shipped under one version, so `GET /api/version` and doctor's `binary` line could not tell a stale install from a fresh one (KI-412). That reason argues for never shipping two different binaries under one version — NOT for bumping on every edit.

The cost this corrects, measured 2026-11-17: release.sh tag ~4 min (cold gate lane) + assets ~7 min (release build 4m06 + dmg + sign + upload + feed read-back) = 12-15 min of ritual per shipped change, with the same four legs for a CSS colour as for a concurrency fix. Risk is not uniform and the pipeline must stop pretending it is.
