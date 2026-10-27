---
name: sem1-notes-followup-ki284-and-missing-sources
description: "Two agreed work items from the sem1 corpus-refresh run — fix KI-284's write-guard refusal of mandated absolute paths, and author sem1's missing CLAUDE.md + FUNCTIONAL-SPECIFICATIONS.md then re-verify the corpus — blocked this run only by the missing working folder; full plan recorded for the next run."
type: project
date: 2026-10-27
basis: confirmed-by
tier: working
shape: [change, incomplete]
phrases: [corpus-refresh-sem1, KI-284, write_guard, missing_sources, blend1_refresh_sem1.py, action: rebuilt]
missing: [working folder set in the run bar (~/Projects/fleet) so filesystem tools are admitted]
recall-count: 0
superseded-by: consolidation-2026-10-27
valid-until: 2026-10-25
---

Owner asked (2026-10-23, after the corpus-refresh-sem1 run) to address both notes from that run. Work plan, in order:

LEG A — KI-284 write-guard fix (fleet repo): the guard refuses content-constrained files; two write_file refusals cited `Projects` (src/boards.rs:101) when writing tmp/blend1_refresh_sem1.py despite the mandated absolute paths. KI-284 is at docs/known-issues.md:347 (open); the prescribed action is "exempt what the caller cannot rename: absolute paths under the owner's home...". The template is KI-219's closed fix: the guard must take tokens through the same channel rules the vocabulary derives from — path-like runs (e.g. /Users/alejandrodelvillar/Projects/...) should parse as one path token, not split into `Projects`/`Users` prose-ish identifiers. Entry points: write_guard (src/agent.rs, search `fn write_guard`), channel_tokens (src/nomenclature.rs:853), split_identifier. Pin with tests in the style of KI-219's `prose_in_a_markdown_write_is_not_an_identifier` — one asserting a mandated absolute home path in content writes clean. Gate: export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" && cargo build --all-targets && cargo test (KI-007: no rustfmt/clippy). Then close KI-284 in known-issues.md with evidence.

LEG B — missing sem1 sources: author ~/Projects/sem1/CLAUDE.md and ~/Projects/sem1/FUNCTIONAL-SPECIFICATIONS.md from the actual sem1 codebase (sem1 = semantic embeddings lane; see services/sem1 + ~/Projects/sem1/docs/plans/2026-10-07-semantic-embeddings-lane.md and the 2026-10-07 recap). Match the house convention: study a sibling that HAS both files (e.g. ~/Projects/blend1 or ~/Projects/fleet) for structure/tone first. Content must be factual to the codebase — no invented APIs.

THEN: re-run python3 ~/Projects/fleet/tmp/blend1_refresh_sem1.py (driver already on disk, works). Expect action=rebuilt, source_files 3→5, items >24. Verify on disk (new mtimes + changed shas under ~/.local/share/blend1/vectors/sem1/embeddinggemma-2/). Then supersede sem1-corpus-24-items-unchanged-through-2026-10-23 with the new corpus state. The 2026-10-24T00:25Z scheduled job will also hit this — if Leg B lands before that window, that job's report will show rebuilt.

BLOCKER: this run has no working folder — bash/read_file/write_file/todo all refused by name. Owner must set the folder in the run bar (~/Projects/fleet — fleet's tools reach sem1's tree from there, as this conversation's earlier calls proved). Nothing was changed on disk for either leg this turn.
