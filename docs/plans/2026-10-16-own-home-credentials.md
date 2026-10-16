# Fleet reads its own home only — the harness credential leg removed

Status: active — the owner's ruling, 2026-10-16: "fleet's router should
never be reading a config file from a different harness."
Date: 2026-10-16

## Why

Fleet's credential resolution has a third leg carried from slice 1:
after the process environment and `<FLEET_HOME>/.env`, both the
router's key resolution (`src/route.rs` `resolve_key`) and the main
credential helper (`src/main.rs` `setting`) fall through to reading
`~/.dsh/.credentials.yaml` — the DeepSeek Harness's credential store —
for a `refs.<NAME>` value. It was documented as deliberate ("one stored
key serves both harnesses", hard rule 4). The owner has ruled it out:
Fleet's router reads Fleet's own home, never another harness's config.
The leg is also redundant on this machine since the 2026-10-16 roster
migration put every roster key, `DEEPSEEK_API_KEY` included, in
`<FLEET_HOME>/.env`.

Not in scope, ruled separately: the boards' read-only seams (the
router kit's operator API, the second-brain state files) read live
state over an API/files the owner designed as dashboards — they are
not config files, and this plan does not touch them. The 2026-10-16
kit-key migration was a one-time copy into `<FLEET_HOME>/.env`; no
runtime path references the kit's files.

## Acceptance criteria

- **C1** The resolution chain everywhere is exactly: process
  environment → `<FLEET_HOME>/.env`. The `or_else` harness legs in
  `route.rs::resolve_key` and `main.rs::setting` are gone, with their
  doc comments telling the truth.
- **C2** `home.rs`'s `harness_credentials_path` and `harness_refs` and
  their unit tests are removed — the read cannot come back through a
  dead helper.
- **C3** The surfaces that name the chain tell the truth: doctor's
  not-set detail names only the environment and `~/.fleet/.env`;
  `fleet --help`'s environment block likewise.
- **C4** The docs that codified the old chain are corrected in the
  same turn: CLAUDE.md hard rule 4 and the `home.rs` map line, the
  README's configuration table and its harness-store paragraph, and
  CLAUDE.local.md's env table (`DSH_HOME`'s row goes).
- **C5** No behavior change for any key present in the environment or
  `<FLEET_HOME>/​.env` (the existing suite green unchanged; no test
  pinned the harness leg).
- **C6** The gate runs whole and green: fmt, clippy `-D warnings`,
  release build, test.

## Files to be touched

`src/route.rs`, `src/main.rs`, `src/home.rs`, `src/doctor.rs`,
`src/cli.rs` (help text), `CLAUDE.md`, `README.md`,
`CLAUDE.local.md` (gitignored). New: none.

## Verification

The gate, locally, exit code read directly; plus a grep of `src/` for
`harness_credentials_path|harness_refs|credentials\.yaml` returning
nothing outside history, and a live `fleet doctor` (credentials ok
from `<FLEET_HOME>/.env`).

## Linked artifacts

- The owner's ruling, this session, 2026-10-16
- CLAUDE.md hard rule 4; README "Configuration";
  [docs/known-issues.md](../known-issues.md) carries no row — this was
  documented design, now re-ruled, not an encountered defect
