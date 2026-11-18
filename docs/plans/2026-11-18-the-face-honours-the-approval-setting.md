---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: the-face-honours-the-approval-setting
---

# The face honours the approval setting

The owner's 2026-11-18 rulings, verbatim, in the order they landed:

1. "vision asked for my approval, which is the wrong behavior. vision should
   always be available and usable."
2. "core tools should never require approval, the only ones that require
   approval are for destructive tool calls."

The second is the general rule the first was an instance of."

## Context

Vision prompted. The cause is NOT vision.

Vision is not in the destructive class and not in `ALWAYS_ASK_TOOLS`
(`approvals.rs:463` — that list is `computer_use` alone), so under
`yes-guarded` it should cross without a prompt. The owner's configured word IS
`yes-guarded` (`~/.fleet/config.json` → `"approvals": "yes-guarded"`).

The prompt came instead from a **disagreement about which mode the run is in**:

- **Headless** (`fleet run`) resolves the mode from the setting:
  `approvals::resolve(yes, config_word(home))` (`main.rs:3556`).
- **The face** does not. `GuiState` seeds `permissions: Mode::Ask`
  (`bridge.rs:459`) and the only writer is the run dispatch
  (`bridge.rs:5596`), which takes the mode the FACE's own permission control
  sent. `config_word` is never consulted on that path.

So with `approvals: yes-guarded` in the config, the browser face runs `Ask`,
where every call prompts — vision included. That is the whole defect: a
setting the owner set is honoured by the CLI and ignored by the surface they
actually use.

## Approach

**PART ONE — implemented.** The ruling is a change to what `ask` MEANS, not a
vision allow-list.

- `Gate::approves_without_asking`: `Mode::Ask` no longer approves nothing. It
  stops at the destructive class and nothing else — the same class
  `YesGuarded` measures against, so the two modes are now the same gate.
  `Yes` remains the unguarded latch that waives the class too. What still asks
  in EVERY mode: the destructive class, `ALWAYS_ASK_TOOLS` (`computer_use`),
  and a server's destructive hint.
- `GuiState::new` seeds `permissions` from
  `approvals::resolve(None, config_word(home))` rather than a hard-coded
  `Mode::Ask`, so the face honours the setting the CLI already honoured.

Measured after: a headless vision run **no longer prompts**. The call proceeded
to the model and failed on its own transport bug (below), which is proof the
gate released it.

## PART TWO — the transport, FIXED

The approval gate was hiding a second, real defect. With the gate released, a
vision call answers:

```
error: request failed: Bad URL: failed to parse URL: RelativeUrlWithoutBase: relative URL without a base
```

The cause is measured, not guessed: `Work::Vision` builds its client with
`Client::routed(router)`, and `Client::routed` sets `base_url: String::new()`
and relies on the router. But the BLOCKING `Client::chat` never consults the
router — it dials `format!("{}/chat/completions", self.base_url)` directly, so
the request URL is `/chat/completions` with no host. Only `chat_stream`
routes. Every other `Client::routed` caller in the crate uses the streaming
path, which is why vision is the only casualty: it is the only blocking caller.

The owner chose **(a)** — give `Router` a blocking chat. Building it revealed
that (a) cannot work where vision lives, and the reason is worth recording
because it is the kind of thing that only shows up at the call site:

- `Work::Vision` is handled inside `async fn run_work`, so the call is already
  **inside a tokio runtime**. A `Router::chat_blocking` would have to drive
  `lane_client::ask` (async) to completion, and `block_on` inside a running
  runtime panics. So the blocking walk was written, then reverted — it would
  have traded one runtime bug for another.
- The honest fix is the one that routes where the call already lives: vision
  **awaits** `client.chat_stream` and drops the events nobody watches. That is
  (b)'s shape, reached from (a)'s diagnosis.

It also buys something the blocking call never had: the walk's own FAILOVER. A
vision ask now fails over across `vision`'s candidates instead of dying on the
first. Verified live — the answer, and no prompt.

## Approach (the original seed bug, for reference)

One resolution, read where the state is built, so every surface starts in the
mode the owner declared.

- `GuiState::new` seeds `permissions` from
  `approvals::resolve(None, approvals::config_word(home).as_deref())` instead
  of the hard-coded `Mode::Ask`.
- The face's own permission control keeps working exactly as it does: a
  per-run choice still overrides for that run (it is the owner's hand on the
  throttle), and it is not persisted back over the config unless the surface
  already does so today.
- `Mode::Ask` remains the fail-closed reading for an absent or unknown word —
  `resolve` already does this, so nothing changes for a fresh home.

Vision needs no special case. The ruling's sentence — "vision should always be
available and usable" — is satisfied by the run being in the mode the owner
chose, because vision is in neither always-ask class. Adding vision to an
allow-list would be a second policy for a tool that has no blast radius: it
reads files the browser pane already serves.

## Acceptance criteria

- [x] C1: a face booted on a home whose `config.json` says
  `"approvals": "yes-guarded"` runs in `Mode::YesGuarded`. Pinned by
  `the_face_starts_in_the_mode_the_owner_declared`, which asserts the seeded
  `GuiState::permissions`.
- [x] C2: the same home with no `approvals` key, `ask`, or an unknown word
  still runs `Mode::Ask`. Pinned by
  `a_home_that_declares_nothing_still_fails_closed` over all three.
- [x] C3: the face's own permission control still overrides for its run — the
  dispatch path (`bridge.rs:5596`) is untouched.
- [x] C4: the class boundary is unchanged and the mode it is measured against
  moved. Pinned by `ask_stops_only_at_the_destructive_class` (a search, a
  suite, a read, a `vision` ask and a `list_dir` all cross; `rm -rf`,
  `write_file` and `computer_use` still ask) and
  `ask_and_yes_guarded_measure_the_same_class`.
- [x] C5: a live run calls `vision` and gets an ANSWER, with no approval
  prompt. Proved twice on 2026-11-18: `tmp/health-mark.png` returned the sheet's
  own title, its six candidates, its tab labels and its colour scheme — and the
  dimensions it reported (1500x900) match the file exactly. No prompt appeared
  in either run.

## Files to be touched

- `src/gui/bridge.rs` — the seed at `GuiState::new` (line 459), and the
  assertion that the dispatch path still overrides.
- `tests/face/harness.mjs` — nothing: this is not a face-paint change.
- `docs/plans/2026-11-18-obscura-is-the-browser-engine.md` — a cross-reference;
  the vision lane's own availability rides the same work.

## Decisions the owner holds

- **D1 — whether the face's permission control should write the config.** Today
  the control is per-run and the config is the standing word. If the owner
  flips the control in the face, should that persist as `approvals` so the next
  run starts there? I would say NO (the control is a throttle, the config is a
  policy, and a slip of the hand should not re-policy the harness) — but it is
  the owner's call and it changes what C3 asserts.

## Out of scope

- Vision's own path rules (absolute paths and `.env` refused) — those are
  deliberate and were measured working.
- The vision provider's routing. Separately: the earlier `vision` failure in
  this session carried `Bad URL: failed to parse URL:
  RelativeUrlWithoutBase`, which is a transport-layer message and is NOT
  explained by this plan. It needs its own diagnosis once the approval gate
  stops blocking the call.

## Verification

`cargo test -p fleet --lib` for C1–C4, then a live face run: press nothing,
ask a question about `tmp/candidates.png`, and get an answer.

## Linked artifacts

- `src/approvals.rs:115` `resolve` — the one resolution the CLI already uses.
- `src/approvals.rs:463` `ALWAYS_ASK_TOOLS` — `computer_use` alone.
- `src/gui/bridge.rs:459` — the hard-coded `Mode::Ask` this plan replaces.
- `src/main.rs:3556` — the headless path, for contrast.
