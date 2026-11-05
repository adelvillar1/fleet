---
name: fleet-harness-voice-soul-slot
description: "fleet's SYSTEM_PROMPT is the best-of-both voice (claim-time verification + execution discipline from hermes, outcome-first + end-turn from zcode), and <FLEET_HOME>/SOUL.md is the hot identity slot that replaces the base per run (aa367e8, 2026-10-16)"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner ruled (2026-10-16) to take the best of both harness prompts for fleet. Implemented in commit `aa367e8`: `SYSTEM_PROMPT` carries claim-time verification ("Done" = every criterion tool-verified), act-don't-describe with end-turn discipline, parallel-call batching, read-back after external writes, verification-matched-to-change, and outcome-first recaps — universal text, no repo content (that rides [[fleet-harness-orientation-doc]] and recall). The identity slot is `<FLEET_HOME>/SOUL.md`: read per run via `home::read_soul` (BOM/control-char sanitized, bounded at 8192 chars), and when present it REPLACES the base while skills/memory frames append; delegation children inherit it with the parent's model. `eval` pins `None` for determinism. The owner can adapt `~/.hermes/SOUL.md` into it; fleet never reads the hermes file. The same commit landed the dogfooding session's streamed finish_reason WIP (found in-tree, completed to compile).

**How to apply:** persona changes are a config edit to `~/.fleet/SOUL.md`, never a recompile; any future prompt work keeps repo knowledge out of the compiled-in text. Plan: `docs/plans/2026-10-16-harness-voice.md`; repo fact: `memories/the-harness-voice-is-best-of-both.md`.

**2026-10-21 — the stop's contract is not part of the voice.** The owner
reported sessions stopping after tool runs with no result, status, or
next-step. `REPORTING_CONTRACT` (in `src/agent.rs`) now rides EVERY run's
prompt after the base: what happened / where it stands / what is next,
and never silence, a bare "done", or a tool card alone. It is appended
by `system_prompt` even when a SOUL.md replaces `SYSTEM_PROMPT` — that
was the load-bearing decision: a contract written only into the base
would vanish the day the owner writes an identity slot.
`default_prompt()` is the plain composition (voice + contract), and the
prompt tests compare against it. Plan: `docs/plans/2026-10-21-turn-reports.md`.

**2026-10-21 — the repair loop is built (23f15f9).** The instruction is
enforced now: `is_silent_stop` (blank or an ack-only message, only after
tools ran) makes the loop ask once, with `REPORT_INSTRUCTION` appended to
that one request's system prompt; the silent turn is not pushed (nothing
said, and two assistant turns would poison a resume). Bounded to one ask;
a second silent stop ends with stop reason `completed without a report`.
A `ReportRepair` event + headless line make the paid extra call visible.
The mock/eval fixtures that ended a tool run with "done"/"All done." were
rewritten to reports — that ending is exactly what the repair catches.
Plan: `docs/plans/2026-10-21-report-repair.md` (completed).
