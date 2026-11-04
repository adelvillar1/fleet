---
name: fleet-deck-two-sessions-collide
description: "2026-10-17 — the dogfooding session is building the SAME deck feature (UiEvent::Deck in agent.rs, Entry::Deck in the TUI) while this session builds the schema; the halves are complementary, their WIP has a red test, and their bridge.rs hunk must be held out of my commits by partial staging"
type: project
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

While shipping the deck structure stage ([[fleet-deck-visual-pipeline]]), the
parallel dogfooding session was found building the **same feature** from the other
end: `UiEvent::Deck { project, nodes, html }` in `src/agent.rs`,
`Entry::Deck(DeckCard { project, nodes })` in `src/tui/app.rs`, and a `Deck` arm in
`describe()` in `src/gui/bridge.rs` — all uncommitted. Their `DeckCard` fields map
exactly onto `deck.rs`'s `ProjectType` and node count, so **the halves are
complementary, not competing**: they own the event and the terminal card, this side
owns the schema and the browser face.

Three consequences that cost real time:

- Their WIP spans agent.rs + tui/app.rs (the variant *definitions*) AND bridge.rs +
  tui/ui.rs (the *uses*). Reverting only the definitions — the obvious move to get a
  clean gate — breaks the build in the other direction, because the uses reference
  variants that no longer exist. Restore all four files or none.
- Their WIP has a **red test**:
  `agent::tests::a_run_with_memory_declares_the_write_path_and_one_without_it_does_not`.
  A full `cargo test` is red until they land; run per-layer (`cargo test --lib deck::`,
  `gui::`, `memory::`) to check your own work.
- `src/gui/bridge.rs` is co-edited, and their `Deck` arm cannot simply be deleted
  (their variant needs it to compile). It was held out of my commit by **partial
  staging**: `git hash-object -w` the file minus their hunk, then
  `git update-index --cacheinfo 100644,<sha>,src/gui/bridge.rs`, leaving the working
  tree intact. See [[shared-file-commits-can-sweep-another-sessions-work]] for the
  failure mode this prevents.

**They landed while I was still working, and it tangled the history.** Their
commit `6cc0b80` ("the render stage and the agent tool") ran `git add` over the
whole working tree, so my *uncommitted* theme-picker changes to `app.js` and
`app.css` rode along inside their commit. Nothing was lost and everything is in
HEAD, but their message does not mention it. The choice made was to **leave the
tangle rather than rebase** — rewriting a commit under an actively-working
session risks worse than a mislabeled one — but the alternative is on the table
if the owner wants clean attribution.

Their commit also added `DECK_TOOL` ("deck", a tenth tool) without updating the
lane integration tests' expected tool counts. That eventually reddened **nine**
tests, not the four first seen, and it blocked the whole gate for everyone — so
it was fixed here as its own commit `b96e5e7` rather than left as "their
increment's to make". The counts and the list literals each needed the deck
entry in the position the roster puts it (after `convention`, before `web`).
Lesson: a co-edited repo's red gate is everyone's problem, and a mechanical
test-only fix is worth landing even when the cause is someone else's change —
but land it as a separate commit that says so.

The traffic ran both ways: their `e05f8c3` fixed a bug **I** left behind when I
renamed the theme attribute from `light` to `paper` — the composer's `/theme`
command still wrote `data-theme="dark"`, a dead value that silently reset the
chosen theme. That is the failure mode in
[[parallel-renames-leave-dead-values]] seen from the other side: when you
rename a vocabulary, grep for the old values in code another session may be
editing too, and expect them to find yours.

**How to apply:** before committing in this repo, check `git status` for co-edited
files and read each staged file's diff for hunks that are not yours. Prefer
partial staging over holding a whole file back. And when a co-edited file has
uncommitted work of yours in it, expect the other session's next commit to sweep
it — land yours promptly, or disclose the tangle rather than silently rewriting
theirs. Tell the other session what `deck.rs` now provides so they consume the
schema rather than re-deriving it. Related: [[fleet-deck-visual-pipeline]],
[[gui-revamp-browser-face-decision]],
[[shared-file-commits-can-sweep-another-sessions-work]].
