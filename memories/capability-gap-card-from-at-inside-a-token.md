---
name: capability-gap-card-from-at-inside-a-token
description: "A capability-gap card can be false: the @name scanner read `@loader_path/../lib` in inline code as an address, so the owner was offered \"create a persona for this\" for a tool that does not exist — check the record's reason against its source prose before treating a gap card as a real request."
type: feedback
date: 2026-11-07
basis: inferred
tier: durable
shape: [fail-open, resolved]
phrases: [last_mention, src/session.rs, Record::Recommend, capability gap, suggestRow, "overscroll-behavior: contain"]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Found while diagnosing the owner's "the modal had no scrolling and I could not approve that tool request" (fleet 0.4.0-beta.1, 2026-11-06/07).

TWO defects, both real, both now closed:

1. KI-468 — the persona door could not be submitted. `.door` is `position: fixed; inset: 0; display: grid; place-items: center`, and `.door .form` was `width: 560px; max-width: calc(100vw - 48px)` with NO `max-height` and NO `overflow`. A form taller than the window overflows both edges equally and its acts row — where every door is confirmed — sits below the fold with nothing to scroll. Measured in Chromium with the shipped sheet + shipped markup: before, form 1108px in a 613px viewport, overflow-y visible, acts bottom at 1085px after scrolling as far as it goes (unreachable); after `max-height: calc(100dvh - 48px); overflow-y: auto; overscroll-behavior: contain` the form is 565px, scrollable, acts reachable at 566px. Same before/after at 900px. Fixed commit 2026-11-07.

2. KI-469 — THE "TOOL REQUEST" WAS NEVER REAL. `last_mention` in src/session.rs required only that the byte before `@` not be a name byte; it never required the name to END the word. So an rpath written in inline code — `` `@loader_path/../lib` `` — parsed as address `loader_path` with following text `/../lib`, and the agent posted a `Record::Recommend { name: "loader_path", kind: "capability" }` that the face rendered as a suggest card with a "create a persona for this" button. THE PROOF: the record's `reason` is byte-equal to the prose slice after the parsed name (session 20261106T033200Z-14f5a714.jsonl:7723 vs the assistant text at :7719). Two such records exist in the whole corpus, both from that one session.

THE CHECK TO RUN before acting on a capability-gap/suggest card: read the record's `reason` and find it in the session's own assistant text. If the reason is a mid-sentence fragment and the `name` is path-shaped, token-shaped, or an at-rule, the card is a parse artifact, not a request — there is nothing to approve, and the right action is to fix the scanner rather than hire a persona for a name nobody needs.

THE FIX in the scanner: (a) the byte after the name must not continue a token — `/ . : @ \ %` all continue it, a backtick and whitespace and punctuation close it; (b) a `const DIRECTIVES` list of CSS at-rule names (media, supports, import, keyframes, font-face, charset, namespace, page) never addresses anyone whatever whitespace follows, which is the one shape where a space is not evidence of an address. Pinned by five assertions in `a_mention_is_scanned_and_its_kind_classified`; with the guards removed the test fails on `Some(("loader_path", "/../lib`"))`.
