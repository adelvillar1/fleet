# Every stop carries a report

Status: completed
Date: 2026-10-21

The owner reported the session habit directly: fleet runs tools and then
stops without giving a result, a status, or a word about whether a
decision is needed. The prompt had a closing paragraph about the run's
final message, but it was voiced as guidance about tone rather than a
contract for every stop, and it named no failure.

## What changed

- `REPORTING_CONTRACT` is a new compiled-in constant: a turn that ran
  tools ends with one message giving **what happened** (with the
  concrete evidence), **where it stands** (done / partly / blocked), and
  **what is next** (the action being taken, the one owner decision, or
  "nothing needed from you"). It names the three failures observed —
  silence after a tool call, a bare "done"/"ok", and a tool card alone —
  and says the owner must never have to ask what happened. A complete
  report is short, not padded, so it tightens the existing "do not pad"
  rule rather than fighting it.
- `system_prompt` appends the contract to every run. A plain run's
  prompt is `default_prompt()` — the voice followed by the contract —
  and an identity override (`SOUL.md`) replaces the voice but still
  gets the contract, because the expectation of a report is the
  harness's, not the identity's. Subagents inherit it the same way.
- The voice's own closing sentences about the final message moved into
  the contract; the voice keeps the tone rule ("plain claims over
  adjectives…") so the structure has one home.

Verification: `cargo test --lib system_prompt`,
`cargo test --lib reporting_contract`, and the integration suite's
prompt pins, which now compare against `default_prompt()`. The contract
is a compiled constant, so the tests are the verification; no live run
is needed to prove a string.

## What is left

Not built, and deliberately so: a harness-side repair turn that re-asks
a model which stopped silently. The ruling asked for the instruction;
an enforcement loop is a separate decision with its own request cost.
The owner's `SOUL.md` was not created or edited — there is none on this
machine, and the contract no longer depends on one.
