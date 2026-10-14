# Install and launch (slice 15)

`fleet install`: the idempotent home layout and config scaffold, the
per-lane heavy-lane venvs ported natively from the fork's
`services/bootstrap.mjs`, `fleet doctor` as the install's gate, the
release-binary seam named, and the README's quick start made literally
true — on a clean machine, install → doctor green → `fleet chat`
completes a real task, and no step requires the fork or Node.

Spec: the fork's `services/bootstrap.mjs` (read-only reference) and the
plan's slice 15 line in `docs/plans/2026-10-13-fleet-everyday-harness.md`.

## The surface

`fleet install [--lanes]` is one sequence, not a toolkit — a stray flag
or word is a usage error. It prints one line per step in the house
style (the fork's `+`/`=`/`!` vocabulary: created, present, named), then
ends on the doctor's own checks:

```
fleet install — the home layout, the config scaffold, and the doctor gate

  home            /Users/x/.fleet
  + sessions      created /Users/x/.fleet/sessions
  = memories      present /Users/x/.fleet/memories
  + skills        created /Users/x/.fleet/skills
  = cron          present /Users/x/.fleet/cron
  + proposals     created /Users/x/.fleet/proposals
  + config.json   written /Users/x/.fleet/config.json — the roster's override seam: {"pin": null, "providers": []}
  ! lanes         optional heavy lanes — `fleet install --lanes` builds /Users/x/.fleet/venvs/<lane>/; a lane that cannot build is skipped by name
  ! credentials   DEEPSEEK_API_KEY: process env → /Users/x/.fleet/.env → the harness refs (~/.dsh/.credentials.yaml, read-only)

── doctor ── the install's exit code is the doctor's ──
credentials ok   set (…abcd)
home        ok   writable (/Users/x/.fleet)
…
```

The bare install never blocks on a network: it names the lane step as
available and moves to the gate. `--lanes` is the only step that
touches the network (the package installs), and it degrades without.

## The home layout

Five directories under the home (`FLEET_HOME`, else `~/.fleet`), one
per consumer in the crate:

| Directory | Consumer |
|---|---|
| `sessions/` | the append-only JSONL session logs (`src/session.rs`) |
| `memories/` | the durable-fact corpus (`src/memory.rs`) |
| `skills/` | the SKILL.md tree and its proposals (`src/skills.rs`) |
| `cron/` | the job store (`src/cron.rs`) |
| `proposals/` | the self-maintenance gate's pending changes (`src/improve.rs`) |

Every step is idempotent: a directory that exists is named present and
untouched, and the install never destroys anything.

## The config scaffold

`<home>/config.json` is the roster's override document — the
compiled-in defaults are the roster, and this file is the seam an owner
edits (`route::load_roster` merges it per entry, never replaces). The
scaffold is the minimal valid document:

```json
{
  "pin": null,
  "providers": []
}
```

It is written only when absent. An existing file — whatever it holds,
including a hand-edited one that is not valid JSON — is named and left
byte-for-byte untouched: the install is not the place an owner's config
gets clobbered.

Credentials are not configured by the install and no key material is
written anywhere. The resolution chain is named in the install's output
and here — the environment variable `DEEPSEEK_API_KEY`, then
`KEY=VALUE` lines in `<home>/.env`, then the DeepSeek Harness
credential store's `refs` block (`$DSH_HOME/.credentials.yaml`, default
`~/.dsh/.credentials.yaml`, read-only) — with names only, never values.
The doctor's credentials check below the install's report is the place
that says whether one resolved.

## The heavy-lane venvs

A native port of the fork's `services/bootstrap.mjs`: per-lane,
idempotent, degrade-without. The venvs live at
`<home>/venvs/<lane>/`, one directory per vendored core, built by the
machine's own `uv` (the fork's tool, asked for Python 3.12) or
`python3 -m venv` when `uv` is absent — never Node, never a checkout
dependency, and the fork stays the read-only spec.

| Lane | Packages | Wiring the core reads |
|---|---|---|
| `sys1` (judgment) | `gliner2` | a config-file key: `[providers] local_venv` in `~/.config/sys1/config.toml` |
| `sdm1` (tabular) | `structured-data-models` | `export SDM1_MODELS_VENV=<venv>` |
| `sem1` (embeddings) | `sentence-transformers` | `export SEM1_ST_VENV=<venv>` |
| `gen1` (media) | `kokoro-onnx` | `export GEN1_KOKORO_PYTHON=<venv>/bin/python` |

The three behaviors, each named in the output:

- **Present** — the venv's `bin/python` exists: the lane is detected
  and named, not rebuilt. This is the whole presence check, the fork's
  own rule.
- **Installed** — the venv was created and its packages installed; the
  line names the wiring the core reads (the export, or sys1's config
  key with the real path).
- **Skipped** — the venv could not be built: no `uv`/`python3` on
  PATH, a failing create, or a failing package install (no network).
  The reason is named, the install still succeeds, and the lane stays
  declared and dead — slice 4's existing dead-lane behavior, never a
  hard failure. A half-built venv from a failed install is removed
  best-effort, so the lane's state stays honestly absent and a later
  run retries instead of reading "present" over a dead venv.

The exports are printed, not written: the lanes read the ambient
environment through slice 4's passthrough, so the owner puts the named
lines in the shell that boots Fleet (the fork's own posture). gen1's
ONNX weights (`~/.cache/gen1/tts/kokoro-v1.0.onnx`,
`voices-v1.0.bin`) are machine-owner downloads named in the lane's
line, not fetched by the install — the same check-and-document posture
the fork's `llama` lane takes for llama.cpp and its GGUF. The
embedding server itself is the memory slice's probed vector path;
`doctor`'s recall line names whether it is live.

## The doctor gate

The install adds no new check. It ends on `doctor`'s own checks —
credentials, home, API reachability, shell, the routing roster, the
calibration ledger, one named line per MCP core, plus the
informational calib and recall lines — and its exit code is the
doctor's: green 0, a named failure nonzero. `fleet doctor` and the
install's gate are one function (`doctor_checks` in `src/main.rs`), so
the two surfaces cannot drift.

## Release binaries

The release path is `cargo build --release` — one static binary at
`target/release/fleet`, which is what the README's quick start runs.
Per-OS cross-built artifacts are a release-process concern and are
**not built here**: CI is disarmed by owner decision (no Actions
funds), so nothing dispatches a cross-build. The seam is this section
and the commands are recorded for when the owner re-arms it:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin \
  x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-msvc
cargo build --release --target aarch64-apple-darwin    # this machine
cargo build --release --target x86_64-unknown-linux-gnu
# …one artifact per target under target/<triple>/release/fleet
```

The binary is self-contained: no Node runtime, no harness checkout,
and the lanes resolve from the `services/` tree beside the executable
(or `FLEET_SERVICES_ROOT`), with a missing tree degrading to named
dead lanes rather than a failed boot.

## Tests

`tests/install.rs` drives the real binary against a scratch
`FLEET_HOME`, with the doctor's API probe pointed at a loopback
listener (no external host) and a placeholder credential (presence
only — no key material):

- the layout and the scaffold's exact bytes, created once;
- idempotence — a second run names every step present, recreates
  nothing, and leaves the scaffold's bytes untouched;
- an existing `config.json` is named and never rewritten, even when it
  is not valid JSON;
- the exit-code contract — a green doctor is a green install, a red
  doctor (dead API port) is a red install with the failing check named;
- the quick start's deterministic core — install, then `fleet doctor`
  green on the home the install left;
- the lane bootstrap's three behaviors: a fake `uv` on a controlled
  PATH installs all four lanes with the fork's command shapes and a
  second run rebuilds none (the argv log is unchanged); a PATH with no
  venv tool skips every lane by name and the install stays green; a
  failing package step skips the lane by name and leaves no half-built
  venv behind.

The release-binary and `fleet chat` halves of the quick start are
proven by hand against the live roster (the chat half is not
deterministic and does not belong in a suite); the executed output is
in the slice's session recap.
