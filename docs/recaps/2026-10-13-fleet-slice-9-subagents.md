# Slice 9 — subagents (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 9).
**Feature:** `docs/features/subagents.md`. **Ledger:** KI-070..KI-078.

## What landed

`src/subagents.rs` (new, 887 lines) — the scope, the request parse, the
child route, the child run, the result, the inherited approver — plus
wiring in `src/tools.rs` (the declaration), `src/agent.rs` (the scope in
`AgentOptions`, scoped declarations, typed refusal by name, the dispatch,
`UiEvent::Subagent`, one `Box::pin` for the delegation recursion),
`src/session.rs` (`kind`/`parent` on `start`, `create_subagent`, the
summary kind), the TUI card, and `tests/integration.rs` (9 new tests).

- **The tool.** `subagent(goal, tools?, max-turns?)`, declared beside
  `use_skill`/`remember`/`supersede`. The goal is required and must be
  self-contained — the child sees nothing of the parent's conversation.
  The admitted scope defaults to the four built-ins plus `use_skill`;
  `remember`, `supersede`, lane tools, and `subagent` itself are not in
  the schema's enum, and the loop refuses an out-of-scope call by name
  anyway. `max-turns` defaults to 20, bounded at 50.
- **The child's run.** The same loop, never a fork: a new session log
  whose `start` record carries `kind: subagent` and the parent's session
  id (both optional — an ordinary log's first line is byte-unchanged),
  the same compiled-in system prompt, the goal as the only user message,
  the scoped declarations, the turn cap.
- **Same model, structurally.** The parent passes its resolved client
  AND its active provider decision into the child's run. No second
  resolution, no separate provider config, no per-call model parameter —
  the never-downgrade rule is a property of the code, not an
  instruction. The smoke and the integration tests assert it on the
  wire: every recorded request, parent's and child's, carries the same
  model.
- **The same gate.** The child's calls go through the parent's Approver,
  labelled `subagent → <tool>` with the goal in the prompt, so the owner
  sees whose call it is. Per-call scope; `--yes`/always applies
  tree-wide.
- **The result.** `{summary, session, log, turns, outcome, reason}` —
  `completed | max-turns | failed`, a failure returning the partial
  result plus the typed reason plus the log path, never silence. It
  rides the parent's ordinary bounded `tool_result` path and lands in
  the parent's log.
- **Depth one.** `SUBAGENT_MAX_DEPTH = 1`, enforced twice: the scope
  never admits the tool, and the loop refuses it by name with the cap
  cited.
- **The verification sentence.** Both reviewers found the same gap — the
  plan's AC "the parent verifies external side effects itself" had no
  home where the parent model would see it. The tool's description now
  says the summary is the subagent's own report, not evidence, and to
  verify any external side effect it claims before reporting it as fact;
  the feature doc carries the matching "Not evidence" section. The
  verification is the parent's by design — a mechanism that "verified"
  would be the child grading its own homework.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **326 unit + 74 integration
  passed, 1 ignored** (baseline 305 + 65; nothing deleted — 8
  declaration-set assertions updated for the tool the slice adds).
  rustfmt/clippy still absent (KI-007).
- A real smoke against the scripted mock, KI-066-safe: the port file
  asserted before the run, every other roster candidate disabled in a
  scratch config, the embedding endpoint pointed at a dead port so the
  shared llama-server was never touched. A delegated `list_dir` task ran
  in its own session, called the tool, and answered; exit 0.
- The parent's visual pass (tmux + ANSI + vision), driving a real
  delegation: the parent's approval modal (`subagent` + the goal +
  `[y]/[n]/[a]`); the child's inherited gate labelled `subagent →
  list_dir` with the goal; the live card showing the child's session id,
  its call count, and last tool; the completed card (`✓ subagent … done`
  / `└ subagent completed · 2 turns · session … · log …`); the parent's
  final answer carrying the child's report. Two distinct session ids
  throughout.
- Negative probes: the turn cap ends a always-calling child at the cap
  with the partial log; an API error mid-child yields `failed` with the
  reason; a scoped-out call is refused by name; a malformed request is a
  typed refusal with no child created.

## Review cycle

- **Stage 1 (spec, step-5-preview): CHANGES-REQUIRED** — one must-fix,
  the verification sentence above. Non-blocking: the declaration test
  hardcoded the five names instead of asserting against
  `ADMISSIBLE_TOOLS` (fixed — the comment's coupling claim is now true);
  the smoke's safety is attested but not committed as a script.
- **Stage 2 (quality, step-5-preview): APPROVED** — no criticals;
  panic-free production paths; the recursion depth-bounded three ways;
  the child's lifecycle honest under interruption (inline, dropped with
  the parent, its log's `start` still names its lineage); four scope
  layers with distinct failure modes; no shared-state hazard (the calib
  stream serializes through one writer, the sessions dir claims with
  `create_new`, the recall index is untouched). Importants: the
  verification gap (fixed) and the TUI card's owed terminal capture
  (done, above). Minors logged: KI-075 (a resumed session's card loses
  the goal), KI-076 (no wall-clock cap on a delegation as a whole).
- **Both reviews dispatched in ONE parallel call** — the first use of
  the new pattern; the two verdicts arrived together, ~8 minutes of
  wall-clock instead of two serial rounds.

## Incidents logged during the slice

- **KI-066 recurrence (the parent's rig):** pinning the mock candidate
  is not enough — with the pinned port wrong, the roster's failover
  advanced onto the live chain and a full delegation was answered by a
  live provider. The mock's requests file staying empty was the alarm.
  The fail-closed rule now disables every other candidate and asserts
  the port file's port equals the config's before launch.
- **KI-074, sharper root cause:** the corpus root resolves from the cwd
  walk only and never consults FLEET_HOME (while the sessions dir and
  the calib root both use the home) — running fleet from under
  `~/.hermes` silently adopts Hermes' own memory store as the corpus.
- **KI-077/078 (the visual pass):** the status bar's memory segment
  clips mid-word at 120 columns; long goals truncate unwrapped in the
  approval modal and push the card's state token off the line.

## Not done (slice 10+)

- Compaction (10), scheduled runs (11), self-maintenance (12).
- The minors above; a committed smoke script for the KI-066 pattern.
