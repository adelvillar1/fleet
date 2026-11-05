---
name: app-leftovers-identification-2026-10-27
description: "2026-10-27 — the leftover-audit pattern (bundle-id → extension-base → vendor/team → case-folded name against installed apps) identified ~3.1 GB of verified orphans, but the FIRST pass's 14.5 GB was mostly false positives: extension ids, embedded-helper circularity, Apple/team-id daemons, and live dev caches"
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

**Owner, 2026-10-27:** "it would be really great if we could use that
same pattern [the Claude Desktop leftovers sweep] to identify
applications that are no longer installed on this machine that have
leftover garbage." One-off audit, not committed as a script (offered).

**The pattern that worked** (two passes over ~403 artifacts in
`~/Library/{Preferences,Caches,HTTPStorages,WebKit,Containers,Saved
Application State,Group Containers,Application Support,Logs}`):
1. Build the installed set from THREE registries — location globs
   (/Applications, /System/Applications, ~/Applications), Spotlight
   (`kMDItemContentType == 'com.apple.application-bundle'`), and one
   `lsregister -dump` pass — collecting bundle ids + display names +
   TeamIdentifiers.
2. Classify each artifact: exact bundle-id hit → **extension-base
   chain** (walk the dotted id down to a 3-segment prefix; `com.microsoft.Word.widgetextension`
   and `org.blenderfoundation.blender.thumbnailer` are installed apps'
   extensions, not orphans) → vendor/team token match →
   **case-folded** name match → otherwise orphan candidate.
3. **Ground-truth every big-ticket item individually** (`mdfind
   kMDItemDisplayName` + an `/Applications` glob) before reporting —
   heuristics produce a wrong top-line number.

**The four false-positive classes the first pass (254 items / 14.5 GB)
was mostly made of:** app-extension ids (Blender's thumbnailer, Word's
widgets); **embedded-helper circularity** — Steam's own
`Steam Helper.app` lives INSIDE `App Support/Steam/Steam.AppBundle/`,
so the leftover itself matched Spotlight and "proved" itself live
(Steam.app is absent; 1,508 MB is a real orphan); Apple/system daemons
wearing non-`com.apple` names (`GeoServices`, `PassKit`,
`SharedImageCache`, `askpermissiond`, team id `243LU875E5`); and live
dev tooling (`ms-playwright` 780 MB, `pnpm` 372 MB, `bun`, and
`coreMLCache` 3.3 GB of framework-compiled models) that automation
depends on. Also case-sensitivity (VS Code's `Code` folder never
matched "visual studio code") and a last-segment rule that kept
Otter's `…desktop` id because Docker Desktop's name contains "desktop"
— both fixed in pass two.

**Result:** verified orphans ≈3.1 GB — Steam 1,508 MB, Comet 625 MB,
FluidAudio 443 MB (predecessor of the installed FluidVoice,
`com.FluidApp.app`, no reference to the old name), M365 Copilot
container 438 MB, Otter 337 MB (hand-corrected from "kept"), Luminar
Neo 104 MB, plus rtk/Qwen/Presenton/Topaz/Animoji/LM Studio/MiniMax/
SharePoint ≈135 MB and a tail of dead pref plists. Left alone: 69.7 GB
of live-app data — and two levers named for the owner's actual
resource concern: **Docker's data dir still occupies 32.7 GB** even
after their config-change reset zeroed its store (disk image + logs),
and `coreMLCache` is the biggest reclaim-with-rebuild item. Offered,
not run: trashing the verified table, and keeping the audit as
`scripts/app-leftovers.py`.

**EXECUTED the same day — owner: "move them."** 29 items (~2.3 GB) moved
to `~/.Trash/orphans-20261027/` (restorable until emptied; space only
frees on Trash-empty). **Four paths could NOT move from the shell:**
`~/Library/Containers` is folder-wide TCC-blocked for the terminal
(even a dead unregistered container, even `touch` — proven, so it's the
folder, not the container registry): `com.microsoft.m365copilot`
(438 MB), `com.skylum.luminarneo.photoextension`,
`com.topazlabs.TopazPhotoAIplugin`; and `App Support/Topaz Labs LLC`
(12.4 MB) is **root-owned** (an installer made it). Finish = drag in
Finder (prompts for the admin password on the root one) or grant the
terminal Full Disk Access and retry. Two sweep-mechanics gotchas: two
paths shared the basename `com.microsoft.SharePoint-mac` across
different roots (App Support moved first and took the Trash name, so
HTTPStorages needed a suffixed target), and FluidAudio (443 MB) was
left in place per its own "your call" caveat — still pending the owner's
word.

Related: [[claude-desktop-leftovers-2026-10-27]] (the first sweep),
[[mcp-vizro-removed-2026-10-27]] (the pattern's origin).
