# Functional specifications

This file is the project's user-facing behavior reference. It aggregates the
existing sources: `docs/features/*.md`, `docs/architecture/*.md`,
`src/cli.rs`, `src/tui/`, and the eval corpus in `src/eval.rs`.

If a command or surface has no entry here, it is either covered in a feature
doc above or missing. The "Source" column points to the file whose claims were
aggregated here; when behavior changes, follow that file.

## Command reference

| Command | Source |
|---|---|
| `fleet run "<task>"` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md`; `docs/features/subagents.md` |
| `fleet chat` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `fleet sessions` | `docs/features/memory.md`; `docs/features/subagents.md` |
| `fleet sessions --recall <query>` | `docs/features/memory.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `fleet sessions --consolidate` | `docs/features/memory.md` |
| `fleet sessions --archive` | `docs/features/memory.md` |
| `fleet lanes` | `docs/architecture/overview.md`; `docs/features/install.md` |
| `fleet providers` | `docs/features/routing.md`; `docs/architecture/ecosystem.md` |
| `fleet calib` | `docs/features/calibration.md`; `docs/features/calibration-loop.md` |
| `fleet judge` | `docs/features/calibration-loop.md` |
| `fleet delegate` | `docs/features/calibration-loop.md` |
| `fleet explain` | `docs/features/explain.md` |
| `fleet eval` | `docs/features/eval.md` |
| `fleet eval --last` | `docs/features/eval.md` |
| `fleet settings set <key> <value>` | `src/cli.rs`; `docs/architecture/deep-dive-2026-10-15.md` |
| `fleet skills` | `docs/features/skills.md` |
| `fleet skills --proposed` | `docs/features/skills.md` |
| `fleet skills --curate` | `docs/features/skills.md` |
| `fleet skills --show <name>` | `docs/features/skills.md` |
| `fleet improve` | `docs/features/self-maintenance.md` |
| `fleet improve apply <name> [--approve]` | `docs/features/self-maintenance.md` |
| `fleet improve undo <name>` | `docs/features/self-maintenance.md` |
| `fleet remember ...` | `docs/features/memory.md` |
| `fleet doctor` | `docs/architecture/overview.md`; `docs/architecture/deep-dive-2026-10-15.md` |
| `fleet doctor --diagnose <session>` | `docs/features/eval.md` |
| `fleet install` | `docs/features/install.md` |
| `fleet install --lanes` | `docs/features/install.md` |
| `fleet compact` | `docs/features/compaction.md` |
| `fleet cron` | `docs/features/cron.md` |
| `fleet graph` | `docs/architecture/deep-dive-2026-10-15.md` |
| `fleet roles` | `docs/features/subagents.md` |
| `fleet convention <concept>` | `docs/features/skills.md`; `docs/features/memory.md` |

## The browser face

`fleet gui [--port N] [--no-open] [--host H]` serves the app's own
interface on loopback: the conversation first, the intelligence
surfaces one flat level beneath four destinations, and the tower as the
one console. A per-launch token gates every route and prints with the
URL; a non-loopback host is refused by name. The surfaces and the trust
trio are documented in `docs/features/gui.md`.

## Run behavior

- A pinned provider routes every shape through that provider's own models.
- Without a pin, sys1_classify judges the workload and the preference table
  walks the declared chain.
- The default workload can be promoted as `Fallback` when confidence is below
  the gate threshold.
- DeepSeek is the only pay-as-you-go candidate and is declared in no workload's
  chain; it serves only when pinned.
- A mid-stream failure is fatal rather than a failover.
- Every tool call goes through the approval gate one call at a time; a denial
  is a model-visible result, not a panic.
- Lane children are spawned lazily and closed on every exit path; a dead lane
  fails fast for the rest of the run without killing it.

## TUI modes

| Mode | Behavior | Source |
|---|---|---|
| Chat | Streamed tokens, approval prompt, tool-call progress | `docs/architecture/deep-dive-2026-10-15.md` |
| Sessions | Session picker with compacted marker, recall launch | `docs/features/subagents.md`; `docs/features/memory.md` |
| Approval | Keyboard-driven approve/deny | `docs/architecture/overview.md` |
| Lanes | Health cards for each vendored lane core | `docs/features/install.md` |
| Settings | Scalar knobs as cards; list surfaces are CLI-only today | `docs/architecture/deep-dive-2026-10-15.md` |
| Board | Three read-only boards cycle with Ctrl+B | `docs/features/boards.md` |

## Background jobs

- Compaction runs between turns when the assembled prompt exceeds the budget.
- Consolidation runs on an hourly timer in chat and the TUI, and on demand.
- Cron jobs fire on schedule, catch up exactly one missed window, and run
  through the same agent loop with their own session log.

## Eval corpus families

- Session-corpus replay
- Agent-continuation
- Conversation-fold
- Two-turn tool loop
- Subagent delegation
- Memory recall
- Delegation-loop closes

The corpus is fixed and deterministic; `fleet eval` prints house cards and
lands one `eval_corpus` row.

## Error vocabulary

| Class | Meaning |
|---|---|
| `credentials` | Key or base URL missing or invalid |
| `balance` | 402 from the provider |
| `rate limit` | 429 from the provider |
| `network` | TCP/HTTP failure before a response |
| `decode` | Response could not be parsed |
| `tool` | Built-in or lane tool returned an `error:` result |
| `approval` | The user denied a tool call |
| `max-turns` | The turn cap was reached |
| `failed` | Unclassified terminal failure |

## Known functional gaps

| Area | Status | Likely source |
|---|---|---|
| TUI settings list surfaces | Partially implemented | `src/settings.rs`, `src/tui/app.rs`, `docs/architecture/deep-dive-2026-10-15.md` |
| Structured provider editor | Follow-up | `src/settings.rs` |
| Human TUI open | Unclosed | `docs/architecture/deep-dive-2026-10-15.md` |

These gaps are noted here as missing coverage, not as defects in code. When
this file falls out of sync with the sources above, prefer fixing the source
feature doc or command implementation and then re-aggregating this file.
