---
name: corpus-blend1-2026-11-19
description: "blend1's corpus refresh at the 2026-11-19T00:35Z window repeated the no-op steady state: 11 source files, 94 items, unchanged, 0 dropped duplicates, no source-set delta, and the store byte-identical (manifest md5 f23109d7…, vectors.bin sha256 4567b48e…) to its 2026-10-28 build — a third consecutive window with the digest wildcard empty because no design board is bound to blend1."
type: project
date: 2026-11-19
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [fleet mcp call blend1 refresh, corpus-blend1-2026-11-19, "\"action: unchanged\"", empty_wildcards canvas/digests/blend1, 94 items over 11 source files, EXCLUDED_FILENAMES CLAUDE.local.md]
missing: []
premises: []
recall-count: 0
superseded-by: corpus-blend1-2026-11-19
valid-until:
source:
---

Corpus refresh snapshot for project `blend1`, run by the `corpus-refresh-blend1` cron job at 2026-11-19T00:35Z.

Route: `fleet mcp call blend1 refresh '<args>'` (KI-369 spawn route — one command spawns the vendored lane through its launch spec, initializes with the cold window a first run needs, calls, prints, tears down). Run from /Users/alejandrodelvillar/Projects (the parent of the fleet repo, which the job document calls repo home). No `fleet` on PATH, so the binary was invoked directly at /Users/alejandrodelvillar/.fleet/bin/fleet, reporting **fleet 0.8.15** (mtime 2026-11-17 15:39). FLEET_HOME unset, so the default /Users/alejandrodelvillar/.fleet applied. Exit 0.

Sources declared:
- /Users/alejandrodelvillar/Projects/blend1/docs/**/*.md
- /Users/alejandrodelvillar/Projects/blend1/*.md
- /Users/alejandrodelvillar/.fleet/canvas/digests/blend1/*.md

Reply verbatim:
{"project": "blend1", "source_files": 11, "items": 94, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "store": "/Users/alejandrodelvillar/.local/share/blend1/vectors/blend1", "missing_sources": [], "empty_wildcards": ["/Users/alejandrodelvillar/.fleet/canvas/digests/blend1/*.md"], "sources_added": [], "sources_removed": []}

Verification beyond the reply:
- 94 items over 11 files reconciles exactly from the stored manifest's per-key counts (manifest.json `entries`, grouped by key before `#`): CLAUDE.md 5, FUNCTIONAL-SPECIFICATIONS.md 7, README.md 10, TECHNICAL-DOCUMENTATION.md 9, docs/plans/2026-10-13-blend1-foundation.md 7, docs/plans/2026-10-13-blend1-validation-gate.md 7, docs/plans/2026-10-14-blend1-escalation-and-score.md 5, docs/plans/2026-10-14-blend1-project-corpora.md 9, docs/plans/2026-10-14-blend1-web-lane.md 9, docs/recaps/SESSION-RECAP-2026-10-13.md 11, docs/recaps/SESSION-RECAP-2026-10-14.md 15 — sum 94 over 11 distinct keys. The manifest also carries count 94, dims 768, model embeddinggemma-2, provider llama-server. Store path resolves to the model subdirectory `vectors/blend1/embeddinggemma-2/`.
- 12 .md files exist on disk (5 root + 7 under docs) but only 11 are corpus files: `CLAUDE.local.md` is excluded deliberately by the lane's own `EXCLUDED_FILENAMES = frozenset({"CLAUDE.local.md"})` (services/blend1/src/blend1/corpus.py:57, applied at :226-227 against both the given and the resolved name). The off-by-one is the rule working, not a missed file — independently confirmed as `git check-ignore -v CLAUDE.local.md` → blend1/.gitignore:32. So the job string's `blend1/*.md` wildcard legitimately yields 4 sources, not 5.
- No write was spent: manifest.json 108,737 bytes and vectors.bin 289,768 bytes are both still mtime 2026-10-28 12:37, under `vectors/blend1/embeddinggemma-2/`. manifest sha256 73131f0ce0a3176171b6529a582af7af4136322f51b9178c2161c138150e083b, md5 f23109d76f30ff291a88b09b7dbcfc77; vectors.bin sha256 4567b48e349bb3a18bd45635f801021b16d0ba56cbfa71b655a025e494856a60, md5 3f0b9c3b79f0ac4ea335a5d659f89c7c. Both hashes are identical to the ones recorded by the 2026-11-08, 2026-11-17 and 2026-11-18 snapshots, so `changed: false` describes the store, not a stale report.
- The store postdates every indexed file: the newest indexed .md mtime is 2026-10-14 17:54 (CLAUDE.md), so no source text has moved since the 2026-10-28 build — an independent reason for `unchanged` beside the hash. blend1 repo HEAD is 45c6933 (2026-10-24, "fix(tests): the missing-variable test enforces its own precondition"); docs untouched since Oct 14.
- `unchanged` is a real comparison, never a fail-open: the embedding endpoint answered live — curl 127.0.0.1:8901/health → {"status":"ok"}, and /v1/models names the embeddinggemma-2 BF16 GGUF in the HF cache snapshots dir.
- Source-set delta is a true zero in both directions: `sources_added: []`, `sources_removed: []`, and the manifest's 11 distinct keys are exactly the live glob set (12 on disk minus the excluded CLAUDE.local.md).
- `missing_sources` is empty: every declared literal path resolved.
- `empty_wildcards` names only the digest glob: `~/.fleet/canvas/digests/` does not exist at all (not just blend1's subdir), ~/.fleet/canvas/ holds only board-1 and sample-board, and no project.json under it contains the string "blend1" — so no design board is bound to this project. Satisfiable, not dark; it fills the moment a board is bound to blend1 and saved.
- This window is the fifth identical no-op in the run of them (2026-11-08, 11-12, 11-17, 11-18, now 11-19 share the same 11 files, same 94 items, same manifest md5), which is what a steady state looks like across many days rather than one.

Naming: the canonical snapshot spelling is `corpus-blend1-<YYYY-MM-DD>.md` — corpus at the head, dashed date at the tail — and the substrate writes it into /Users/alejandrodelvillar/Projects/fleet/memories/ (the fleet home's `memories/` is an empty scaffold, so memory.rs's resolve_root walks up from the cwd to the repo's own corpus). That FILE spelling is what the tree's ignore rule finds: `git check-ignore -v memories/corpus-blend1-2026-11-19.md` resolves to .gitignore:37 `/memories/*corpus*20[0-9][0-9]*.md`, so a snapshot can never be committed by accident (KI-386, KI-466). The fact NAME takes no dot, since the remember grammar is lowercase letters, digits and hyphens.
