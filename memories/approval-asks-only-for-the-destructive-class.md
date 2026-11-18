---
name: approval-asks-only-for-the-destructive-class
description: "Approval now stops only at the destructive class — every mode, `ask` included — and the browser face honours the configured approval word instead of hard-coding `ask`; vision no longer prompts, but its transport is separately broken."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [deps, state-sync, resolved]
phrases: [core tools should never require approval, "vision asked for my approval, which is the wrong behavior", approves_without_asking, config_word, RelativeUrlWithoutBase, Client::routed]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling: "core tools should never require approval, the only ones that require approval are for destructive tool calls." (The lead-in was vision prompting: "vision asked for my approval, which is the wrong behavior. vision should always be available and usable.")

WHAT CHANGED. `Gate::approves_without_asking` (src/approvals.rs): `Mode::Ask` no longer approves nothing. It now stops at the destructive class and nothing else — `Mode::Ask | Mode::YesGuarded => !Gate::is_destructive(call)`. `Mode::Yes` remains the unguarded latch. What still asks in EVERY mode, unchanged: the destructive class (`rm -rf`, forced overwrite, `write_file`, `move`/`delete` file changes), `ALWAYS_ASK_TOOLS` (`computer_use` alone), and a server's destructive hint. The `Mode::Ask` doc comment and the module header were rewritten, because they claimed `Ask` "prompts for every call" and "approves nothing".

SECOND HALF, a real defect the ruling exposed: `GuiState::new` hard-coded `permissions: Mode::Ask` and the only writer was the run dispatch, so `approvals: yes-guarded` in config.json was honoured by `fleet run` (which resolves via `approvals::resolve(yes, config_word(home))`) and IGNORED by the browser face. The face now seeds from `resolve(None, config_word(home))`; absent/unknown still reads `Ask` (fail-closed).

TESTS: `ask_approves_nothing_without_asking` pinned the OLD rule and was rewritten to `ask_stops_only_at_the_destructive_class`; added `ask_and_yes_guarded_measure_the_same_class`, `the_face_starts_in_the_mode_the_owner_declared`, `a_home_that_declares_nothing_still_fails_closed`. Note: `Bridge::detached` needs `#[tokio::test(flavor = "multi_thread")]`, not plain `#[test]` — a plain one panics "no reactor running".

PART TWO STILL BROKEN, and the approval gate was hiding it: with the gate released, vision reaches the model and fails `request failed: Bad URL: failed to parse URL: RelativeUrlWithoutBase`. Cause measured: `Work::Vision` builds `Client::routed(router)`, which sets `base_url: String::new()`, but the BLOCKING `Client::chat` never consults the router — it dials `format!("{}/chat/completions", self.base_url)` directly. Only `chat_stream` routes. Vision is the only blocking `Client::routed` caller in the crate, which is why it is the only casualty. Two fixes recorded in the plan: (a) give `Router` a blocking chat so `Client::chat` walks it, or (b) make vision stream. Not yet chosen or built.

Gate: 2376 Rust tests, 1457 face checks, fmt and clippy clean. Plan `docs/plans/2026-11-18-the-face-honours-the-approval-setting.md` is `active` with C1-C4 ticked and C5 half-proved.
