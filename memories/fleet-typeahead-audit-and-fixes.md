---
name: fleet-typeahead-audit-and-fixes
description: "2026-10-19 live audit of the composer's two-pass command type-ahead (lexical floor + semantic refinement) — three defects found, all fixed: KI-243 stale keep-alive zeros and KI-244 first-word debounce starvation in 937a28a, and KI-245 closed by owner ruling in 5ca5522 (Enter runs the selected option, with an epoch guard against stale semantic replies); the owner offered Redis and it was refused"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's feature #3 request ("type / and see commands by semantic similarity from the intent;
if not possible, letters") is the shipped two-pass design (`d6b5c7f`): `/` opens all eight
commands; the lexical pass paints in the same frame (exact → prefix → alias → description
substring → fuzzy subsequence); the semantic pass debounced 180 ms POSTs `/api/rank`, which
embeds the query and every candidate through `memory::probed_embedder` and cosine-ranks them;
scores above 0.25 lead, the rest keep their lexical place. **Audited live 2026-10-19 and it was
broken in real use by two defects, both fixed in `937a28a` (gate green 1121):**

- **KI-243 (fixed)** — the embed server closes an idle keep-alive connection after five seconds
  (`Keep-Alive: timeout=5`; llama-server on 127.0.0.1:8901 serving EmbeddingGemma-2 GGUF,
  ~20 ms/embed, `FLEET_EMBEDDING_ENDPOINT` in `~/.fleet/fleet.env`), and `HttpEmbedder::embed`
  rode ureq's pooled connection — one request after an idle gap failed in milliseconds, `None`
  turned the whole ranking into zeros, and the type-ahead silently degraded to lexical. Any
  pause to think longer than 5 s switched the semantic pass off. Fix: one retry in `embed` —
  the failed attempt burns the dead pooled connection, the second rides a fresh one.
- **KI-244 (fixed)** — the rank's debounce armed only inside `openCommands`, which `typeAhead`
  reaches only when the line's FIRST word changes, so `/make the screen darker` ranked the
  mid-typing fragment "make the scr" and never the sentence. Fix: the debounce re-arms on every
  keystroke in `typeAhead`; the first-word guard keeps only the lexical repaint.
- **Verified live after the fixes**: an intent typed after a 6.5-second idle — the case that
  always zeroed — paints the semantic winner at a real score (`/theme` 0.734 for "make the
  screen darker"; `/pane` 0.808 for "show me the files i am working on").

**KI-245 (CLOSED by the owner's ruling, `5ca5522`):** Enter on a partial slash line (`/the` +
Enter) did not run the highlighted command — it sent the literal text to the agent as a task.
Tab and click ran the highlight; Enter matched only exact name or alias, and the handler's own
comment said slash lines never reach the agent, so the code violated its own rule. The owner
ruled: **"the enter should send the option that is selected on the screen."** With the
type-ahead open, Enter (like Tab) now runs the highlighted row; Shift+Enter stays a newline;
the exact-name parse keeps serving the closed case. The verification caught a second wrinkle,
also fixed: an in-flight semantic reply landing after a command ran would REOPEN the box the
run just closed (`rankSemantic` set `commandOpen = true` unconditionally) — the rank now
carries the close's epoch and a stale reply is ignored. Verified live: `/the` + Enter opens
the theme picker and closes the list; a ranked intent + Enter runs its winner; Shift+Enter
newlines.

**The owner offered Redis "to improve responsiveness" — refused, and the refusal is the
record:** latency was never the problem (the local embedder answers in ~20 ms; the failures took
16 ms), and Redis violates the crate contract (one static binary, no external services).

Two small facts worth keeping: the rank scores separate poorly (~0.73 winner vs ~0.70
runner-up), which is why semantics is designed as a refinement of lexical and never a
replacement; and the server accepts the launch token from the `fleet_gui` cookie the page's
fetch already carries, so browser-side API calls need no token in the URL.

**How to apply:** after fixing a type-ahead-class feature, verify with the network capture, not
the painted UI alone (the zeros were invisible in the DOM order). Related:
[[verify-what-renders-not-what-commits]], [[gui-revamp-browser-face-decision]],
[[fleet-deck-visual-pipeline]].
