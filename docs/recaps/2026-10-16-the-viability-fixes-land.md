# The viability fixes land

Status: complete
Date: 2026-10-16

Branch: `master`, solo. The session implemented
[docs/plans/2026-10-15-dogfooding-viability-fixes.md](../plans/2026-10-15-dogfooding-viability-fixes.md)
end to end — the second dive's five defects plus the toolchain repair
that made the gate whole — with the convergence section grading every
acceptance criterion against the committed code.

## The increments, in order

**Phase 0** filed the second dive's six defects as KI-208..213, each
row pointing at its phase, before any fix landed.

**The toolchain** (KI-007): brew's rustup shim installed keg-only,
rustfmt 1.8.0 and clippy 0.1.89 added to the existing stable
toolchain, CLAUDE.local.md's quirk line updated the same turn.

**The gate became whole** in two increments: the first rustfmt pass
(`3e05dfe`, 49 files, no behavior change) and the first clippy pass at
`-D warnings` (`7643cf7`, 27 files, 64 lints across three waves — each
fixed in the crate's voice or annotated with a named reason; the
boxed `LaneChild::Live` server, the `Pending` and `ProviderModels`
type aliases, and four annotated attributes are the structural ones).
KI-097/KI-104's premises updated: the improve gate's source-apply path
is unblocked on this machine.

**The bash cap bounds the call** (`8a3275e`, KI-211): the drains
stream through channels and a 2s grace collect past the reaped shell —
a background process holding the pipe gets it cut and named
(`truncated: true`) instead of hanging the run forever; the timeout
path survives its own kill the same way; mid-read failures are named.
The rejected alternative (process-group kill via libc/unsafe) is
recorded in the plan.

**The gate judges the real call** (`d7b93dd`, KI-209/KI-210):
`unlabel` strips every `subagent → ` repetition and unwraps the
labelled wrapper, so yes-guarded's destructive class stops a child's
`rm -rf` and the always-ask ruling survives delegation depth 2. Four
new tests; the benign paths pinned unchanged.

**The committed failure names the provider that was dialed**
(`328faf5`, KI-208): the position maps through the walk order, pinned
with a non-identity order against a closed port, and the recorded
attempt feeds `demoted_providers` in the same test.

**The fold stamps only what landed** (`36facd6`, KI-212):
`write_summary` returns its `Result`; a failed fold leaves every
source fact live, unstamped, unrecorded, and unarchived — the forget
half follows the fold, the one shape the plan's text did not spell
out. Pinned with a read-only-corpus test whose retry folds whole.

**The KI-068 seam closed honestly**: the hits fire strictly after the
empty-choices check — and writing that test exposed the plan's
premise as unreachable: the streamed client always assembles one
choice, so a `[DONE]`-only stream is a *completed run with an empty
answer*, and the hits firing there is correct. The degenerate shape is
pinned end to end in `tests/integration.rs` on the cross-process mock
(an in-process loopback server shares the process with the client —
the exact shape the mock-server exists to avoid, which is where the
first attempt's EINVAL came from).

**A torn tail cannot brick a session** (`9b586dd`, KI-213): `replay`
tolerates exactly one shape — an unparseable final line on a log that
does not end with its newline — dropped and named; a malformed line
anywhere else still fails loud; a parseable final line is kept. Three
new tests.

## The gate

Whole, for the first time: `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
`cargo test` — all four exit 0 at every code increment from
`7643cf7` on; final count 1013 passed / 2 ignored. CI stays disarmed
by budget (KI-006, owner decision; the Gitea-on-Railway idea is
recorded in the plan as undecided).

## What is left

Nothing in the plan's scope. The second dive's remaining findings
(the pin dropped on an unmapped workload, the unvalidated
`gates.default_workload`, the improve gate's fail-silent revert
cluster, the consolidator TOCTOU, the role overlay's missing record,
the TUI overlay clobber, the nomenclature trio, the episodic
summaries' recallability) are named in the plan's out-of-scope for a
later plan — none block dogfooding. The experiment the owner named —
work attended, watch `fleet calib`/`judge`/`delegate` — is unblocked.
