---
status: completed
created: 2026-10-21
updated: 2026-10-21
slug: turn-reports
---

# Plan: every stop carries a report

## Context

Fleet sessions have a bad habit: they run tools and then stop without a
result, a status, or any word about whether a decision is needed. The
compiled prompt's last paragraph speaks about the *run's* final message
("leads with the outcome…"), but it does not state a per-stop contract
and it never names the failure. Two structural facts make a one-line fix
insufficient:

- `SYSTEM_PROMPT` is one long string whose closing sentences are about
  tone as much as structure, so the report is easy to treat as advice.
- A `SOUL.md` **replaces** the compiled base (`AgentOptions.identity`),
  so a contract written only into `SYSTEM_PROMPT` would vanish the day
  the owner writes an identity slot.

## Approach

One compiled-in `REPORTING_CONTRACT` constant, appended by
`system_prompt` to every run's prompt — the compiled voice plus the
contract for a plain run, the owner's identity plus the contract when a
`SOUL.md` is present, so the expectation survives the base being
replaced. `default_prompt()` is the one spelling of the plain
composition.

The contract names three parts (result, status, what is next) and the
three failures: silence after a tool call, a bare "done" or "ok", and a
tool card with no message. It also says the owner must never have to ask
what happened, and that a complete report is short rather than padded —
so it tightens the existing "do not pad" rule instead of fighting it.

## Acceptance criteria

- [x] C1: A plain run's system prompt is `default_prompt()` — the voice followed by the reporting contract — and the contract names result, status, and what is next explicitly.
- [x] C2: A run with an identity override carries the contract exactly once, so the expectation survives `SOUL.md` replacing the base.
- [x] C3: A subagent's prompt carries the contract too, and no frame duplicates it.
- [x] C4: The contract names the failure modes it prevents: silence after a tool call, a bare "done"/"ok", and a tool card alone.

## Files to be touched

- `src/agent.rs` — `REPORTING_CONTRACT`, `default_prompt()`, the assembly, and the prompt tests.
- `tests/integration.rs` — the prompt equality pins compare against the composed prompt.
- `TECHNICAL-DOCUMENTATION.md` — the prompt's composition, in the `agent.rs` entry.

## Out of scope

- A harness-side repair turn (re-asking a model that stopped silently). The ruling asks for the instruction; an enforcement loop is a separate decision with its own cost.
- Editing the owner's `SOUL.md` — there is none on this machine, and the identity slot is the owner's to write.

## Verification

```bash
cargo test --lib system_prompt
cargo test --lib reporting_contract
cargo test --test integration system_prompt
```

Expected: the plain run, the identity override, and the subagent all
carry the contract; no test pins the old bare prompt.

## Linked artifacts

- `TECHNICAL-DOCUMENTATION.md` — `agent.rs` entry.
