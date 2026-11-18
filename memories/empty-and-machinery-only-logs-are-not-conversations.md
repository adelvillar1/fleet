---
name: empty-and-machinery-only-logs-are-not-conversations
description: "Sessions that were never really conversations — 45 logs holding only a start record and 32 whose every message names the machinery — are now their own `empty` and `probe` classes, derived from the log, kept out of the owner's conversation list and counted instead, because they belong to no user and no project."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [they don't belong in the threads or conversation history, they may still be searchable but don't belong to a user, work_class empty probe, machinery_only, SessionSummary.empty, BACKGROUND_WINDOW]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling, second half: "most of these sessions were not user initiated, they seem to be background processes or tasks. they don't belong in the threads or conversation history. they may still be searchable but don't belong to a user and much less to a project."

Measured before the change on ~/.fleet/sessions (918 logs, 133 rows the face called "yours"): 45 held ONLY a start record, and 32 more had no subject in ANY owner message — 77 rows of non-conversation on the owner's list.

Two new classes in `work_class` (src/gui/bridge.rs), both derived from the log so nobody has to hand-mark them:
- `empty` — `SessionSummary.empty`, true when the log holds nothing but its `start` record. A New press that never sent anything.
- `probe` — `SessionSummary.machinery_only`, true when every owner message is a house word, machinery directive or bare continuation.

Both take the BACKGROUND_WINDOW (40) rather than the owner's SEARCH_BOUND (200), so a pile of them can never evict real conversations from the read. Both are counted whole-archive in `counts` as `empty` and `verification`, so the archive stays honest about what it holds; `railRow` in app.js already drops anything whose class is not `yours`, so neither renders. The ruling's "may still be searchable" holds: the rows stay in the document and the `q=` content pass still reaches them.

A DESIGN TRAP worth keeping: `empty` must NOT be inferred from `first_text.is_none()`. A log whose runs wrote answers but no owner message (a seeded or scripted conversation) also has no first_text and IS a conversation. `empty` is read from the record count instead (`text.lines().filter(non-empty).count() <= 1`). The first attempt conflated the two and broke `the_inbox_puts_the_seat_first_and_never_renders_a_blank` and `a_new_conversation_is_a_real_log_and_the_archive_shows_it` — both correct tests catching a wrong predicate.

A SECOND TRAP: `names_no_subject` originally checked `line.contains('?')` FIRST, so "Call the vision tool ... with question='Are they the same shape?'" escaped as a conversation because the question mark lived inside a tool argument. The directive test must run FIRST; a `?` only means "names something" when the opener is not a directive.

Measured effect: the owner's active conversations went 133 rows -> 9 active -> 8 after the `?` fix; 45 counted empty, 86 counted verification. Two behaviour-reversing tests were updated to pin the new truth rather than deleted: an unused log is no longer "yours" but is still counted, and the inbox drops it (a seat is a proposed tool call, and a proposal writes records, so an empty log can hold none) while AC4's never-a-blank rule is now pinned on rows that DO render.
