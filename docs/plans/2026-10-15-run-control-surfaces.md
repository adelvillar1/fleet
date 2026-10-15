# Run control: the persistent yes, and compact on demand

Status: active
Date: 2026-10-15

## Why

Two of the three things this plan covers already exist, and the plan
is honest about which parts are gaps rather than rebuilds.

**The yes flag exists.** `fleet run --yes` / `-y` approves every tool
call without prompting (`src/cli.rs:487`), and `fleet cron add
--auto-approve` declares a job unattended. What is missing is not the
capability but its ergonomics and its graduation:

1. **No persistent mode.** Hermes sets YOLO once and it holds; fleet
   makes you retype `--yes` on every invocation, which for an owner
   running a long chain of tasks is friction on the common path.
2. **No graduation.** Hermes's YOLO still guards the genuinely
   destructive class. Fleet's `--yes` is binary: it approves a
   `rm -rf` exactly as readily as a `grep`. The owner's own house rules
   require approval for destructive operations, so a mode that
   silently waives that is a mode that can violate the owner's rule on
   the owner's behalf.

**Compaction exists and is automatic.** `src/compaction.rs` is a full
replay-aware port and `maybe_compact` (`src/agent.rs:1087`) fires it
under the token budget, records the transaction in the session log, and
keeps a recent tail verbatim. What is missing is the owner's hand on
it: no way to compact on demand, and no way to see the budget state
before it fires.

## Acceptance criteria

### The persistent yes

- **C1** A config-level `approvals` setting in `<FLEET_HOME>/config.json`
  — `ask` (the default, today's behavior), `yes` (approve every call),
  `yes-guarded` (approve everything except the destructive class, which
  still prompts). A CLI `--yes` overrides the config for one run; it
  never silently widens a narrower config.
- **C2** `yes-guarded` names its class in one place: the destructive
  shapes a `bash` call can take (recursive delete, history rewrite,
  force push, credential files). The list is a documented constant, not
  a heuristic.
- **C3** `fleet doctor` names the resolved approval mode, so a run that
  stops asking says why in one line rather than leaving the owner to
  infer it.
- **C4** The mode rides the session log's start record, so a replay
  shows what the run was allowed to do.

### Compact on demand

- **C5** `fleet compact [--session <id>]` compacts a finished session's
  stored record on demand, and `fleet run --compact-now` forces the
  compaction the budget would otherwise wait for.
- **C6** `fleet sessions <id>` (from the session-archive plan) shows the
  budget state — measured tokens, the budget, the tail size — so the
  owner can see how close a session is before it fires.
- **C7** Every compaction lands its row in the calibration stream with
  the tokens it saved, so the budget's arithmetic is auditable rather
  than assumed.
- **C8** A forced compaction on a session already under budget is a
  named no-op, not a second summary.

## Design

The approval mode resolves once at startup beside the credential
resolution — env var, then config, then the default — and rides the
`Approver` the loop already takes. `yes-guarded` is the interesting
one and the reason this is not just a config flag: it is the owner's
house rule ("destructive operations require explicit approval in the
current turn") expressed as a machine-checkable class, which means the
rule survives the owner being AFK.

`fleet compact` operates on a finished session's log: it rewrites the
record the way `maybe_compact` rewrites the live one, and the replay
still reproduces every request that followed — the property the
existing port already guarantees. Nothing about the automatic path
changes.

## Out of scope

- Changing the automatic budget. It is tuned and tested; this plan
  adds a hand on the wheel, not a new wheel.
- Per-tool approval policies (approve `bash` but not `write_file`).
  The three modes cover the owner's actual cases; a policy matrix is a
  surface nobody has asked for.
- Interactive approval in the TUI beyond what exists. The TUI already
  renders the prompt.

## The ruling that outranks the modes: computer use always asks

Recorded 2026-10-15, decided before the tool exists.

Computer use through the cua driver drives the real desktop: a click,
a keystroke, a screenshot of whatever is on screen. Its blast radius
is the machine itself rather than a file or a process, so **no
execution mode waives it** — not `yes`, not a config that asked for
`yes`, not an unattended cron job. It requests approval every time.

The ruling is encoded, not just written down: `ALWAYS_ASK_TOOLS` in
`src/approvals.rs` names the tool before it exists, and
`Gate::approves_without_asking` returns false for a call in that class
regardless of mode. A forwarded `subagent → computer_use` call is
caught too, because the arrow prefix is what the prompt reads, not
what the rule keys on. The implementation plan is
`docs/plans/2026-10-15-computer-use-tool.md`.

This is the tier above `yes-guarded`: the destructive class still asks
under `yes-guarded` but runs under `yes`; computer use asks under
every mode including `yes`. The distinction is not severity — it is
whose machine is at risk.
