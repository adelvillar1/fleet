---
name: external-repo-ki-closures-2026-10-24
description: "2026-10-24 — the seven external-repo KI rows closed across six repos (router-kit CI green after four layers, sys1 landed and re-vendored, blend1's adapter filed upstream, cua driver 0.37.0 live-verified); the owner-input audit closed four more; by evening the owner answered the last three asks — docker verified KI-136, KI-192 ruled out, the blend1 merges landed and everything pushed"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The external-repo session (plan `docs/plans/2026-10-24-external-repo-ki-rows.md`,
9/9; recap `docs/recaps/2026-10-24-external-repo-ki-rows.md`) closed all seven
rows the burn-down had deferred as "wanting their own session":

- **KI-001** — agnostic-router-kit CI GREEN (run 40107568736) after four
  layers, each hidden under the last: Node 24 for the probes' `.mts` imports;
  `npm install` for the `file: lib/workflow` dependency; `npm install --prefix
  router` because `kit apply` renders the scratch runtime by copying
  `router/node_modules/@typesafe-ai` from the checkout and that directory is
  gitignored; and a real kit bug — the named-absence law for unrunnable
  binaries was macOS-only (ENOEXEC arrives as a sync throw there, EACCES async
  everywhere, and glibc's execvp `/bin/sh` fallback made the old no-shebang
  fixture untestable on Linux — the garbage file RAN). Fixture is now
  exists-not-executable; EACCES joined the law. Commits 632c589→7fd0aac,
  pushed to master.
- **KI-003/KI-029** — sys1's plan-surface work had landed (owner commits + the
  `provider or []` guard d144912); branch pushed even with origin; fleet
  re-vendored at `f0faebc` — see [[vendor-sync-deletes-carried-files]] for the
  hang the first pass caused.
- **KI-004** — the owner had already added gen1's remote and pushed; closed on
  evidence.
- **KI-005** — sem1's CLAUDE.md landed (8bdc26f, pushed to main); the last
  ecosystem repo without one.
- **KI-143** — blend1's 566-line MCP adapter + its test filed upstream on
  branch `feat/mcp-stdio-adapter` (92c49ac, pushed; merging is the owner's).
- **KI-207** — owner ruled update: cua-driver 0.37.0 installed, doctor
  all-green, and a live MCP round trip answering 62 tools where every call
  previously died on the daemon-contract mismatch.
- **KI-311 opened**: blend1's corpus derive-dedupe (KI-297's fix) is a
  fleet-side carry exactly like the adapter was — file it upstream by the same
  move.

**The same-day owner-input audit (`8f613c4`) verified the remaining
owner-decision rows against commit history and closed four more:** KI-117 (the
ruling was in its own row — no preserve_order, verified), KI-135 (superseded
by the capability surface: `measure`→sdm1/`sdm1_classify`,
`imagine`→gen1/`gen1_imagine`, pinned by
`every_capability_declares_a_spec_and_maps_to_a_lane`), KI-309 (the
superseding 10-24 fact corrects the mis-recorded shas with verified hashes and
the embedder-keyed path), KI-024 (overtaken by the driver update). KI-268
re-pointed (the rank test lives at `src/gui/server.rs:3151`, the /api/rank
route, not route.rs); KI-190 unblocked (the awaited eval.rs round landed).

**Evening — the owner answered the three asks, and nothing waits on them
anymore.** KI-136: verified in a docker container (`rust:1`, Linux x86_64) —
the quick start's literal sequence (build → install → doctor → chat) ran
against the repo; all six lanes resolved, the only doctor FAIL was the unset
`DEEPSEEK_API_KEY` (the quick start's own step 2, an account matter), chat
launched and exited 0. KI-192: ruled **not needed** — the append-only
disposition design stands, no read-before-write guard. The merges: blend1 got
a second filing (`feat/corpus-carries` — the KI-297 derive dedupe, the
KI-222 600 s embed budget, the KI-223 absolute-key and KI-225 tuple
contracts, with main's tests re-pointed and the env-file symlink excluded by
resolved name), then both branches merged `--no-ff` to main and pushed
(`c76b2eb..4db80e1`, suite green on the merge) — KI-311 closed. Fleet's
pending changes committed and pushed (`6be8096..a74eb6a`): the ledger
rulings (df5493b), the judgment-beats plan draft (63e8d0d), the composer
recap (9d1fb62), the 17-fact corpus catch-up (a74eb6a). The ledger now reads
~282 closed / 11 open / 17 noted — every open row mechanical (KI-066, 239,
240, 241, 249, 252 moves 2–3, 268, 310). Fleet commits that day: 6334a80,
33a4fa3, 7a3f1d7, 1543905, f63479c, 8f613c4, 44b7ea4, df5493b, 63e8d0d,
9d1fb62, a74eb6a. (That census was superseded the same evening — the
unlocked-items session closed the mechanical remainder; see
[[unlocked-ki-items-shipped]] for the final 294/2/15.) Related:
[[fleet-ki-triage-pass-one]],
[[owner-is-a-team-of-one]], [[vendor-sync-deletes-carried-files]].
