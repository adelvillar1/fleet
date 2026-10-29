---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: probe-stamp
---

# Plan: the probe stamp — verification traffic is born marked, and the legacy backlog is marked beside the log

## Context

The navigation plan's named follow-up (2026-10-29): the archive holds
~59 kind-less verification runs — eval-scenario replays the driver ran
against the real home, face checks, manual probes ("Say the single
word: ok", "Run this bash command exactly: echo checkpoint-me") — and
they render as the owner's conversations on the rail. The owner's
ruling puts system-initiated sessions in the log, never on the user
screen; these are traffic in the same class, born without the words to
say so. The log is append-only, so the legacy rows cannot be re-stamped
in place — and should not be.

One word at every layer: **probe**. A probe is a run that exists to
check the harness, not to be a conversation.

## Approach

1. **The birth stamp** — `fleet run --probe` creates the session
   through a `create_probe` seam (the same one `create_scheduled`
   uses), so the start record carries `kind: probe` from birth. The
   stamp rides the log the way scheduled and subagent kinds do.
   `--probe` with `--session` is a usage error: the kind is the birth
   circumstance, and a resumed log keeps the kind it was born with.
2. **The classification** — `work_class` maps the kind to the `probe`
   class; the rail's predicate (class-yours) already excludes it, the
   owner-scoped counts already exclude it, and `fleet sessions` still
   lists it — the log for analysis stays whole.
3. **The legacy sidecar** — `sessions-probes.json` (id → stamp), the
   titles/archived pattern: entries whose log is gone are dropped, the
   write is atomic, and the row builder consults it before `work_class`
   — a kind-less log named there classifies `probe` without one byte of
   history moving. The marking surface is `fleet sessions --probe <id>`
   (repeatable), the archive's own command marking its own traffic.
4. **The backlog curation is honest** — only unambiguous rows are
   marked: first-user-text exactly matching an eval-corpus scenario, or
   the named verification shapes. Borderline rows (the owner's own
   warmups, dogfoods) stay visible and named, the owner's call.

## Acceptance criteria — met 2026-10-29

- **[x] C1** `fleet run --probe` births a session whose log carries
  `kind: probe` (`a_probe_log_carries_its_kind`); `--probe` with
  `--session` refuses by name (`run_takes_the_probe_flag_and_refuses_the_pair`);
  the run and sessions help carry the flag.
- **[x] C2** A probe-kind row and a sidecar-named kind-less row both
  classify `probe`, the owner-scoped counts exclude both, and the
  document keeps both (`a_probe_is_classed_and_the_owners_counts_let_it_go`);
  the rail renders neither (the harness fixture carries a probe row —
  323 checks across 35 cases, green).
- **[x] C3** The gate ran whole with exit codes read directly (fmt 0,
  clippy 0, build 0, test 0). gui.md and the functional spec say what a
  probe is. The legacy backlog: **74 logs marked**
  (`sessions-probes.json`) — the unambiguous matches (eval-scenario
  texts, the named verification shapes) plus the four `bot dogfood`
  scripted-append checks. The borderline remainder is named, not
  marked: "session warmup" (the owner's own ritual) and three genuine
  owner questions ("what is the status of the bot pwa plan", "help me
  find out why the session…", "why does the router show so many
  failures?").

## Landed

- `src/session.rs` — `PROBE_KIND` and `Session::create_probe`, the
  same seam the other kinds use.
- `src/cli.rs` / `src/main.rs` — `fleet run --probe` (refused beside
  `--session`: the kind is the birth circumstance) and `fleet sessions
  --probe <id>` (repeatable), the archive marking its own traffic.
- `src/gui/bridge.rs` — `sessions-probes.json` (load/mark/save, the
  archived set's rules), the `probe` class in `work_class`, and the row
  builder consulting the sidecar before the kind. The rail's predicate
  and the owner-scoped counts needed no change: anything not
  class-yours was already off the screen.
- The marking ran against the live archive: 70 + 4 ids through the new
  verb, the gui restarted onto the build.

## Files to be touched

`src/session.rs`, `src/cli.rs`, `src/main.rs`, `src/gui/bridge.rs`,
`tests/face/harness.mjs`, `docs/features/gui.md`,
`FUNCTIONAL-SPECIFICATIONS.md`, `docs/known-issues.md` (the follow-up's
row), this plan.

## Out of scope

- The GUI's /api/run gaining a probe flag — face runs are the owner's
  conversations; verification traffic drives through the CLI.
- Re-stamping or deleting any existing log — the sidecar is the whole
  mechanism, append-only holds.
- The borderline rows' disposition — named, not decided.
