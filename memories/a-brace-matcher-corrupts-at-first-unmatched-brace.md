---
name: a-brace-matcher-corrupts-at-first-unmatched-brace
description: "feedback 2026-10-31 — a hand-rolled brace-matching patcher silently merged struct literals with following code across six files (15 literals); one matched `}` inside a doc comment or string, and every damage site showed as a parse error at the NEXT token — patch literals by anchored text insertion, never by brace counting"
type: feedback
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

**2026-10-31 (provider-models landing):** I added a field to `ChatRequest` and patched 15 struct literals with a `python` brace-counter: scan from `ChatRequest {`, count `{`/`}`, insert `max_tokens: None` before the closing brace. It corrupted **six files** — because the counter treated a `}` inside a doc comment, string, or macro as a real close, stopping one brace too early and swallowing the code that followed: `}row, not a method: ...` (a doc comment's prose fused to the brace), `}atch client.chat_stream`, `}n::to_string(&StreamRequest`. Each damage surfaced only as a parse error at the *next* token, and two of the six (api.rs, bridge.rs) held my OWN uncommitted edits, so `git checkout` would have destroyed work — hand repair only.

**Why:** Rust literal bodies carry `}` in comments/strings (`/// ... } ...`), and my matcher saw raw text. The damage pattern — `}<first-word-of-swallowed-code>` at column 0 — is the tell.

**How to apply:**
- Patch struct literals by ANCHORED TEXT INSERTION: find the literal's exact `tools: Vec::new(),\n        };` tail with a unique string replace, or edit each site by hand with `Read` + `Edit`. Never count braces over whole files.
- Before any regex/python patch over Rust source: `git diff` the file first — if it carries YOUR uncommitted work, the damage is un-recoverable by checkout; restore the original from `git diff` context hunks by hand.
- Repair symptom → cause map: `}word` at column 0 means a swallowed close; find the literal whose insert ran early and restore the deleted tail from `git show HEAD:file` (the committed original is the ground truth for what the matcher ate).
- The final gate caught all six — but only because `cargo build` ran on EVERY edit; a batch patch that doesn't build between files hides it longer.

Related: [[a-repeated-marker-slice-clobbers-whatever-follows]] (the same class: index-slicing a file with a marker destroys a later region — the fix there was rewriting the file whole).
