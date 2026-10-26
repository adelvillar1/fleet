---
name: layerfs-child-report-verification
description: "A LayerFS subagent's reported paths are sandbox-relative (/workspaces/N, ENOENT on the host); verify its work through the store with `node <bundle>/api/agent/cli/src/layerfs.js --project fleet fs read main /<path>`"
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync]
phrases: [subagent created the file but it is not on the host, /workspaces/1 ENOENT, merged to main but git shows nothing, layerfs fs read main, verify delegated child work, fleet-child branch]
missing: []
recall-count: 3
superseded-by: layerfs-child-work-verification
valid-until:
---

A LayerFS-delegated subagent's report is evidence of what happened INSIDE its sandbox, not on the host. Verified 2026-10-26 (subagent 20261026T104335Z-6892fd0, KI-320): the child's bash ran with pwd /workspaces/1 — a path that does not exist on the host (ls → ENOENT, /workspaces absent entirely). Its write_file committed to its LayerFS branch and the harness merged it, so the work is real and readable only through the store. Parent-side recipe to verify a child's claimed file work: (1) the project name is the folder's registration in ~/.fleet/config.json → layerfs.projects["/Users/alejandrodelvillar/Projects/fleet"].name = "fleet"; (2) the CLI is node <machine.layerfs.bundle>/api/agent/cli/src/layerfs.js (bundle recorded in the same config object; today /Users/alejandrodelvillar/.cache/layerfs-worktrees/layerfs-study-c159d7ad3b45/artifacts/build-linux-25b0bed7197842688bed6e1364c9b25e); (3) read the work back: node <cli> --project fleet fs read main /<child-relative-path> — {"kind":"ok","text":…,"size":…} proves it published; NoSuchPath at main means the merge did not land; (4) branch audit: node <cli> --project fleet project list_branches (child branch fleet-child-<sessionid-folded-lowercase> + merge-fleet-child-…, both carrying the child's commits) and branch list_commits <child-branch>. Git-side: the host repo carries NO fleet-child-* branches — git log --all finds nothing, by design (KI-314/315: the child filesystem is the store, not the working tree). Never restate a child-reported path as a host path; read it back from main first.
