---
status: active
created: 2026-10-16
updated: 2026-10-16
slug: harness-voice
---

# Plan: the harness voice — the system prompt and the identity slot

## Context

Fleet's compiled-in `SYSTEM_PROMPT` (`src/agent.rs:42`) is two generic
sentences — the reason a fresh TUI session "knows nothing." The owner
commissioned a comparison of what ZCode and the Hermes agent seed to new
sessions and ruled: take the best of both. The comparison's verdict,
in priority order:

- **From Hermes**: the claim-time verification rule (every claim about
  live state backed by tool output from *this* session, checked when the
  claim is made; "Done" = every criterion tool-verified); the
  execution-discipline guidance (fleet's whole roster — deepseek, glm,
  kimi, mimo — is the exact set the block targets); a hot identity file
  read from the agent's home every build, so a persona change is config
  instead of a recompile; the parallel-tool-call block with its
  round-trip cost rationale; the recap rule.
- **From ZCode**: outcome-first communication; end-turn discipline (a
  turn ending in a promise of future work ended too early); verification
  matched to the change; guidance in tool descriptions rather than a
  bloated prompt.

The prompt stays universal — repo knowledge still arrives through
recall and the orientation doc, not the compiled-in constant.

## Approach

Two seams, both small:

1. **The compiled-in voice.** `SYSTEM_PROMPT` is rewritten in the
   best-of-both shape, sized like Hermes' SOUL.md (every line rides
   every request): claim-time verification, act-don't-describe with
   end-turn discipline, batch independent calls, read back after
   external writes, verification matched to the change, outcome-first
   recaps.
2. **The identity slot.** `<FLEET_HOME>/SOUL.md`, read at the run
   boundary per run: BOM stripped, control characters sanitized, bounded
   at `MAX_SOUL_CHARS` (8192) with a marked cut, empty/absent → the
   compiled-in constant. Present, it **replaces the identity base**
   (Hermes' semantics) while the skills and memory frames still append.
   It rides `AgentOptions.identity`, so the delegation child inherits
   the parent's voice the way it inherits the parent's model — the
   voice follows the model, not the parent's session state.

`SOUL.md` lives in Fleet's own home — the own-home ruling's territory,
like `config.json` and `routing.json`. The owner adapts their Hermes
`SOUL.md` into it when wanted; fleet never reads another harness's file.

## Use cases

- [ ] A: A run with no `SOUL.md` prompts exactly with the compiled-in
      voice, byte for byte — the run this crate has always had, now
      with the full text.
- [ ] B: A run whose home carries a `SOUL.md` prompts with it as the
      base, skills and memory frames appended as before.
- [ ] C: A delegation child's prompt carries the parent's identity.
- [ ] D: An oversized or control-character-laden `SOUL.md` is bounded
      and sanitized at read, never pasted raw into the prompt.

## UX routes

No new surfaces. The prompt's text is model-visible ⟺ logged by the
existing rule: the system prompt is a compiled-in constant or a home
file, reconstructed per request, never logged.

## Acceptance criteria

- [ ] C1: `SYSTEM_PROMPT` carries the best-of-both voice — claim-time
      verification, act-don't-describe + end-turn discipline, parallel
      batching, read-back after external writes, verification matched to
      the change, outcome-first recap — and no repo-specific content.
- [ ] C2: the identity slot reads `<FLEET_HOME>/SOUL.md` per run —
      sanitized, bounded, absent → the constant, present → it replaces
      the base with frames unchanged.
- [ ] C3: a delegation child inherits the parent's identity through
      `Delegation`.
- [ ] C4: `eval`'s runs keep the compiled-in constant (the corpus
      grades behavior against the mock, deterministic).
- [ ] C5: the gate runs whole — `cargo fmt --all --check`,
      `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` — exit codes read directly.
- [ ] C6: the records say it — `CLAUDE.local.md`'s machine-path list
      names `<FLEET_HOME>/SOUL.md`, and a corpus fact records the voice
      ruling.

## Files to be touched

- `src/agent.rs` — `SYSTEM_PROMPT`, `AgentOptions.identity`,
  `system_prompt()`'s base, the `Delegation` construction, tests
- `src/home.rs` — `read_soul`, `MAX_SOUL_CHARS`, tests
- `src/subagents.rs` — the `Delegation` field, the child's options
- `src/main.rs`, `src/tui/mod.rs`, `src/cron.rs` — the boundary reads
- `src/eval.rs` — explicit `None`
- `CLAUDE.local.md`, `memories/the-harness-voice-is-best-of-both.md`

## Out of scope

- Any prompt-injection scan over memory or skill bodies (Hermes' context
  scan is the named follow-up, its own slice).
- Byte-stability/prefix-cache engineering beyond what the existing
  per-run assembly already does.
- Seeding a `SOUL.md` into the owner's fleet home — the slot exists,
  the owner fills it.

## Verification

**C1/C2:** unit tests — the plain run returns the constant; a run with
an identity returns it as the base with the `## Skills` frame still
appended; `read_soul` sanitizes, bounds, and refuses the empty file.

**C3:** the delegation test asserts the child's options carry the
parent's identity.

**C5:**
```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```

## Linked artifacts

- `CLAUDE.local.md` — the machine-path line
- `memories/the-harness-voice-is-best-of-both.md` — the ruling as fact

## Risks

- A longer prompt rides every request. The text is ~300 words against a
  128k context — the same trade Hermes makes on every session, and the
  per-run assembly already resends it only once per request.
- `eval` scenarios asserting on the prompt's exact bytes shift with the
  new text; the corpus's assertions are updated in the same increment.

## Dependencies

- `docs/harness-orientation.md` — stays the repo-knowledge channel; the
  prompt stays universal by design.
- `docs/plans/2026-10-16-canonical-skills-source.md` — the seam this
  prompt's skills frame already rides.
