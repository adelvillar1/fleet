# Fleet reads its own home only — the harness credential leg removed

Status: complete — landed 2026-10-16
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

## Convergence

- **C1** met. `route.rs::resolve_key` and `main.rs::setting` are now
  one-liners over `home::resolve_setting`; their doc comments say
  "Fleet's own home, and nothing else." `home.rs`'s `resolve_setting`
  doc carries the ruling too.
- **C2** met. `harness_credentials_path` and `harness_refs` and both
  of their unit tests are gone. The one remaining `src/` grep hit for
  `credentials.yaml` is the new negative regression test
  (`resolve_setting_reads_fleets_home_and_no_other_harnesss_store`),
  which writes a decoy harness store, names its home in `DSH_HOME`,
  and asserts the key still does not resolve — the ruling pinned, not
  a leftover read. It serializes on an env lock, matching the pattern
  in `lanes.rs`/`judge/tests.rs`.
- **C3** met. doctor's not-set detail: "export it, or add it to
  ~/.fleet/.env". `fleet --help`'s environment block names only
  `~/.fleet/.env`. The install report's credentials line reads
  `process env → <home>/.env`.
- **C4** met. CLAUDE.md hard rule 4 and the `home.rs` map line, the
  README configuration table and its paragraph, CLAUDE.local.md's env
  table (`DSH_HOME`'s row gone; the machine-paths line now says Fleet
  never reads the store). The same turn also corrected the surfaces
  the plan's file list did not name but which described the chain:
  `docs/features/install.md` (two sample output blocks and the
  credentials paragraph), `docs/architecture/overview.md` (request
  flow step 1), `docs/architecture/ecosystem.md` (cross-repo facts),
  `docs/architecture/deep-dive-2026-10-15.md` (run-path diagram), the
  `CREDENTIAL_ENV` doc in `src/install.rs`, and `Candidate::key_env`'s
  doc in `src/route.rs`. Left as dated history: KI-010's evidence
  row and `docs/plans/2026-10-12-fleet-native-app.md`.
- **C5** met. No test pinned the harness leg, and the suite is
  unchanged for every key that resolves from the environment or
  `<FLEET_HOME>/.env`.
- **C6** met. Gate run whole with the exit code read directly:
  `cargo fmt --all --check` 0 (one line re-wrapped in `doctor.rs`),
  `cargo clippy --all-targets -- -D warnings` 0, `cargo build --release`
  0, `cargo test` 0 — 1012 passed / 2 ignored (net −1 against the
  previous 1013: two harness tests removed, one ruling test added).
- Live `fleet doctor` green, credentials `set (…12f9)` resolved from
  `/Users/alejandrodelvillar/.fleet/.env` with no `DEEPSEEK_API_KEY`
  in the shell environment — the first leg alone is no longer the
  only path to a key.

## Linked artifacts

- The owner's ruling, this session, 2026-10-16
- CLAUDE.md hard rule 4; README "Configuration";
  [docs/known-issues.md](../known-issues.md) carries no row — this was
  documented design, now re-ruled, not an encountered defect
