# Every configurable surface, reachable from the TUI

Status: active
Date: 2026-10-15

## Why

Fleet's knobs live in four places and none of them is the TUI:
`<FLEET_HOME>/config.json` (roster, pin, approvals, calib root, skills
knobs), `<FLEET_HOME>/routing.json` (preferences, gates), environment
variables (`FLEET_HOME`, `FLEET_EMBEDDING_ENDPOINT`, `FLEET_CUA_DRIVER`,
and every provider key), and CLI flags that exist only per invocation
(`--model`, `--max-turns`, `--compact-now`, `--yes`).

The owner's directive: **every configurable surface needs a settings
configuration counterpart in the TUI, and hooks or calls to make them
configurable from the GUI.** The desktop app UI itself is a separate
session — this plan makes the TUI the complete, honest settings
surface, so the GUI session builds on a finished contract rather than
inventing one.

## The inventory, verified against the code

| Surface | Where it lives today | TUI today |
|---|---|---|
| Roster candidates (id, base_url, key env, billing, kind, enabled) | `config.json` `providers[]` | none |
| Provider pin | `config.json` `pin` | none |
| Approval mode (ask / yes / yes-guarded) | `config.json` `approvals` | none |
| Calibration log root | `config.json` `calib.log_root` | none |
| Skills knobs (`retire_after_sessions`, `merge_similarity`) | `config.json` `skills{}` | none |
| Routing preferences (workload → provider/model) | `routing.json` | none |
| Gates (`min_confidence`, `default_workload`) | `routing.json` | none |
| `calibration.informs_failover` | `routing.json` | none |
| Embedding endpoint | `FLEET_EMBEDDING_ENDPOINT` | none |
| cua driver path | `FLEET_CUA_DRIVER` | none |
| FLEET_HOME itself | `FLEET_HOME` | none |
| Run defaults (`--model`, `--max-turns`, `--compact-now`, `--yes`) | CLI flags | none |
| Cron jobs (schedule, task, auto_approve, enabled) | `jobs.json` | none |

The TUI has no settings pane at all: its keys are navigation, pick,
close-overlay, quit, and the approval triad.

## Acceptance criteria

- **C1** A settings pane in the TUI, reached by one key from any state,
  rendering every surface above as cards — never a plain-text table,
  the house UX rule.
- **C2** Each setting is editable in place, and an edit writes through
  to the same file the CLI reads, so `fleet doctor` and a subsequent
  `fleet run` see the change without a restart or a second source of
  truth.
- **C3** Every edit validates against the same rule the file's loader
  enforces (`deny_unknown_fields` on provider entries, `0..1` clamping
  on similarity, a confidence in `0..1`, a schedule the cron parser
  accepts). A refused edit names its reason on one line and changes
  nothing — the GUI must not be a way around the file's own validation.
- **C4** A destructive edit (removing a provider the roster walks,
  changing the pin off a keyed provider, disabling calibration) is
  confirmed in the pane before it lands.
- **C5** The write is atomic — unique temp file plus rename — so a
  crash mid-edit cannot leave a half-written config that fails the next
  load.
- **C6** Env-var surfaces are read and shown, and writable through a
  `<FLEET_HOME>/fleet.env` counterpart the credential resolution
  already prefers over nothing. Key NAMES are shown; values are never
  printed, per the standing credential rule — a key field renders as
  set/unset with its env var name.
- **C7** A `settings` command on the headless path with the same
  surface: `fleet settings [--json]` lists, and `fleet settings set
  <key> <value>` writes. The TUI and the CLI are two views over one
  writer, so a scripted change and a GUI change land identically.
- **C8** The pane names what is read-only and why (FLEET_HOME is a
  launch-time fact; a compiled-in default is not a setting).
- **C9** Tests: each surface round-trips through the writer; a refused
  edit changes nothing; the atomic write leaves no temp file on
  success; the env-var counterpart never prints a value; the TUI pane
  renders every surface as cards.

## Design

**One writer, many readers.** `src/settings.rs` owns the read-modify-
write over `config.json` and `routing.json`, with the same tolerance
the loaders have (a missing file is the defaults) and strictly more
strictness on write (validation before the rename, never after). The
CLI's `fleet settings set` and the TUI's pane both call it; neither
writes a file directly.

**The TUI pane follows the picker pattern the crate already has** —
`Action::Move` / `Action::Pick` / `Action::CloseOverlay` already exist
and the session picker already renders cards. A settings pane is a
picker over a typed list of settings, each with its own editor shape
(text, toggle, choice, list). No new input paradigm.

**Env vars get a file counterpart rather than becoming settings.**
`FLEET_HOME` is launch-time and stays read-only. `FLEET_EMBEDDING_ENDPOINT`
and `FLEET_CUA_DRIVER` are read from the env when set and otherwise from
`<FLEET_HOME>/fleet.env`, which the credential resolution already walks
— so the GUI can set them without touching the process environment,
and a machine that prefers the env keeps it. Provider keys are the same
file, and the pane shows set/unset only.

## Out of scope

- The desktop app UI. Separate session; this plan hands it a finished
  `settings` contract to build on.
- Per-project settings. Everything here is per-home, which is what the
  loaders read today.
- Migrating the existing files. The writer reads the current shape and
  preserves unknown fields it does not manage, so an owner's hand-edits
  survive a GUI edit.
