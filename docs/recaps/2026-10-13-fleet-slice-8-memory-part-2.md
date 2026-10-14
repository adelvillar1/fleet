# Slice 8 part 2 — the write path, the consolidation cycle, provenance, the turn wiring (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 8, part 2 of 2).
**Feature:** `docs/features/memory.md`. **Ledger:** KI-060 (closed), KI-061..KI-069.

## What landed

`src/memory.rs` — the semantic layer's write path, its timer, and its permission
model; `src/agent.rs` — the turn wiring; `src/tools.rs`, `src/session.rs`,
`src/cli.rs`, `src/main.rs`, `src/tui/*`, `src/calib.rs` — the surfaces and rows.

- **The write path — `remember`.** A built-in tool declared beside `use_skill`,
  `brain_remember`-shaped: structured parameters (`name`, `description`, `type`,
  `shape`, `basis`, `tier`, `valid-until`, `body`), mechanism over prose.
  `basis` (`confirmed-by | inferred`) is required on every write; `valid-until`
  is required for the session-scoped tier and defaulted for durable; the name
  is validated as a file stem (no traversal, no dotfile); a taken name is
  refused and named toward `supersede`. The fork's field order, temp+rename,
  round-tripped by part 1's loader. `fleet remember` is the CLI form over the
  *same* validator — one implementation.
- **Three speeds.** A `tier` field: working (session-scoped, expires on
  `valid-until`, enters the assembly after the protected tier and ranked
  recall, excluded from the ranking), durable (the default; the corpus's
  pre-tier facts load unchanged), episodic (the cycle's alone — a hand write
  is refused).
- **The consolidation cycle — scheduled, not manual.** `CONSOLIDATE_INTERVAL`
  is one hour; `spawn_consolidator` runs one pass immediately, then hourly,
  in `chat` and the TUI; `fleet memories --consolidate` runs one on demand.
  Per pass: compress working memory past its `valid-until` or older than
  `WORKING_MAX_AGE_DAYS` (7) into one episodic summary (stamped
  `superseded-by` it), supersede durable inferred facts never recalled and
  older than `STALE_AFTER_DAYS` (180), forget expired facts. A confirmed-by
  stale fact is left alone and named. Every action is a `consolidate`
  calibration row; a `today` parameter seam makes a pass deterministic under
  test. Expired facts **archive** to `memories/.archive/` — reversible,
  invisible to the loader, claimed with `create_new` and a serial suffix
  (`<name>.md`, `<name>.2.md`) so a re-used name cannot clobber a prior
  version.
- **Provenance as the permission model.** `supersede` is a separate tool with
  a required `justification` on both paths. A confirmed-by fact is refused
  without approval at the module seam — and in the agent, a confirmed-by
  target gets a second Approver prompt whose synthetic call is named
  `supersede (confirmed-by)`, per-call scope, no replay across facts. An
  inferred fact supersedes on its recorded justification alone; modify writes
  an always-inferred replacement and stamps the old fact; forget ends
  validity the day before and appends the retirement note.
- **The turn wiring.** `AgentOptions.memory`; per run the assembled block
  enters the system prompt in a `## Memory` frame, both tools are declared,
  one `Record::Memory { shape, facts }` is logged (hard rule 5, the `skills`
  precedent; replays to no message), and the recall hits that entered are
  logged at their entry scores. Headless prints `— memory: shape deploy ·
  3 facts`; the TUI's status bar carries it. A memory-less run is
  byte-for-byte unchanged; an empty corpus declares the write path, adds no
  block, writes no record.
- **KI-060 closed.** The writer bounds and sanitizes what the model sees:
  control characters stripped (removed, not replaced), the description capped
  at 512 with a marked cut, name/type/justification likewise, the body
  refused past 64 KiB rather than cut. The body stays unsanitized by design —
  it never enters context; only descriptions do.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **305 unit + 65 integration
  passed, 1 ignored** in the final state (baseline 264 + 61; nothing
  deleted). rustfmt/clippy still absent (KI-007).
- Real `fleet run` (scripted mock): progress line `— memory: shape deploy ·
  2 facts`, log record `{"type":"memory","shape":"deploy","facts":[...]}`.
- Real `fleet memories --consolidate` on a scratch corpus wrote `2 compress ·
  1 supersede · 2 forget` rows into the live devdec stream, each naming its
  fact and reason; the repo's own 9-fact corpus passes clean ("nothing to
  consolidate", `git status memories/` unchanged).
- Negative probes: the stamp-failure test fails at `unwrap_err() on an Ok
  value` when the propagation is reverted (the exact bug); the archive test
  fails at `reused.2.md` missing when the collision guard is reverted; the
  budget pin fails at the old value.

## Review cycle

- **Stage 1 (spec, step-5-preview): CHANGES-REQUIRED** — one must-fix: the
  KI-060 mechanism was complete but only the description's bound was pinned.
  Parent-fixed directly in `14bbc3d` (two tests: the body's 64 KiB refusal at
  `Remember::parse`, and the type/justification bounds) — and caught its own
  wrong-seam assertion on the way (the body refuses at parse, not write).
- **Stage 2 (quality, step-5-preview): APPROVED** — no criticals; two
  Importants, both fixed in `feb107c`:
  1. the supersede stamps' failures were swallowed (`rewrite_field` /
     `append_note` returned `bool`, ignored at all three call sites) — a
     failed retirement still reported success and recorded a calib row, so
     the audit could claim an action that didn't land. Now a typed
     `WriteError::Stamp`, propagated, recorded only after the writes land;
  2. the archive silently overwrote on a re-used name — now claimed with
     `create_new` plus a serial suffix.
  Plus the minors: the compress→forget ordering pinned by a test, the
  feature doc's summary-recall claim corrected (recall surfaces the
  summary's description; the folded bullets are on disk for audit, not model
  context), and KI-067 (the approver doesn't see the target fact's current
  content), KI-068 (recall bumps fire before the model call, so a failed run
  still counts hits), KI-069 (`fleet remember`'s composed path has no
  automated coverage) logged for later.
- **Operational note — KI-066:** an unattended smoke ran against a live model
  when the implementer's mock-server failed to write its port file; ~19
  read-only turns on real quota before it was killed. Logged with the
  evidence and the hardening (assert the mock's readiness artifact before an
  unattended `--yes` run; a non-loopback base URL in a test is a hard stop).

## Not done (slice 9+)

- Subagents (slice 9), compaction (10), scheduled runs (11).
- The `src/memory.rs` split into a `memory/` submodule — owed at the next
  increment; the write path and the cycle are cleanly severable and the file
  reads sectioned at 4,800 lines.
- The approval prompt's content (KI-067), the pre-call bump (KI-068), the
  CLI write's automated coverage (KI-069).
