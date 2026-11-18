---
name: user-facing-copy-speaks-plain-words-not-harness-jargon
description: "The harness now writes to the person, not the codebase — a voice rule in the compiled system prompt bans lane/face/seat/surface/rail from the chat, and 60+ UI strings across every page were rewritten in plain words, with services named by what they do (sys1 judges, sem1 indexes, sdm1 calculates, gen1 generates, blend1 composes)."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, docs, resolved]
phrases: ["things like face, seat, etc bear no meaning to a real user", the harness itself should avoid that jargon on user facing messages in the chat, lanes should be replaced with services, A huddle is a working session with a bot or bots, read has not landed, the face said no without a reason]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 copy rulings, in two halves.

FIRST — the UI: "some of them like huddles are showing a message that is irrelevant to the user... those messages that try to explain every page need to be meaningful to the user and not contain any internal app jargon, things like face, seat, etc bear no meaning to a real user." A huddle is a working session with one or more bots (multi-user/multi-bot in the armada context).

SECOND — the harness's own chat voice: "the harness itself should avoid that jargon on user facing messages in the chat." Lane/lanes mean nothing to a user; the real vocabulary is: sys1 = validation/judgment model, sem1 = indexing service, sdm1 = calculation engine, gen1 = generation service, blend1 = composing service. "If anything lanes should be replaced with services."

WHAT LANDED. The compiled head of the system prompt (SYSTEM_PROMPT in src/agent.rs) gained a voice rule between "Plain claims over adjectives" and the standing virtues: "Write to the person, never to the codebase" — say "service" not "lane", "the app" not "the face", "approval"/"prompt" not "seat", "screen"/"page" not "surface"/"rail". A service's own name is allowed when it IS the subject (sys1 is the judgment service, sem1 indexes, sdm1 calculates, gen1 generates media, blend1 composes), but pipeline plumbing (lanes, folds, compactions, calibration rows, digests, sidecars) stays out. Named explicitly as "not a ban on precision: it is a ban on making the person learn our vocabulary to understand their own work." Pinned by `the_prompt_forbids_internal_jargon_in_the_conversation`, which asserts the rule rides head_prompt(None) so every run carries it.

UI copy changes in src/gui/assets/: the largest family was "The X read has not landed yet. It reads ..." — 22 strings, all now "Still loading. ..." in plain words. "the face said no without a reason" and "the surface said no..." (28 occurrences) became "the app refused without saying why". Huddles page: "A room for the work" → "Start a huddle" with the note "A working session you share with one or more bots"; goal/bots labels became questions ("what do you want out of it?", "who do you want in it?"); the `<home>/roles/` hint became "No bots are set up yet. Add one from Settings › Bots". The services card "the lanes" → "the services"; "Rebuild the lanes" → "Rebuild the services"; "use the sys1 lane to..." → "use the judgment service to...". Rust user-facing error strings: "the browser lane did not answer" → "the browser service did not answer", and lane_phrase now renders "{role} service".

MEASURED: the audit ran over 241 user-facing strings. The words the owner named (face, seat, ledger, rail, lane, sys1, read has not landed) went to zero in UI copy. What remains deliberately: "LayerFS store" and "substrate" on the technical isolation surface, where those are the actual subject matter.

TRAP WORTH KEEPING: a `?` inside a tool argument defeated a naive "a question names something" guard — "Call the vision tool ... with question='Are they the same shape?'" escaped classification as a conversation. The directive test must run before any question-mark test. (Same class of bug, found on both the naming and the classification passes.)
