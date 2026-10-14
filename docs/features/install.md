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
available and moves to the gate. `--lanes` streams each tool's own
output under the venvs header as it arrives — a multi-minute
`uv pip install` of the torch stack cannot look frozen — and then
names each lane's outcome and a summary line:

```
fleet install — the home layout, the config scaffold, and the doctor gate

  home            /Users/x/.fleet
  + sessions      created /Users/x/.fleet/sessions
  = memories      present /Users/x/.fleet/memories
  + skills        created /Users/x/.fleet/skills
  = cron          present /Users/x/.fleet/cron
  + proposals     created /Users/x/.fleet/proposals
  + config.json   written /Users/x/.fleet/config.json — the roster's override seam: {"pin": null, "providers": []}
  venvs           /Users/x/.fleet/venvs — one venv per core, idempotent, degrade-without
  sys1 | Resolved 12 packages in 1m 04s
  sys1 | Installed 12 packages in 1m 04s
  + sys1          installed /Users/x/.fleet/venvs/sys1 — point ~/.config/sys1/config.toml at it: [providers] local_venv = "/Users/x/.fleet/venvs/sys1"
  ! sem1          skipped /Users/x/.fleet/venvs/sem1 — uv did not finish within 900s and was killed
  = lanes         3 of 4 lanes installed, 0 present, 1 skipped
  ! credentials   DEEPSEEK_API_KEY: process env → /Users/x/.fleet/.env → the harness refs (~/.dsh/.credentials.yaml, read-only)

── doctor ── the install's exit code is the doctor's ──
credentials ok   set (…abcd)
home        ok   writable (/Users/x/.fleet)
…
```

The summary line is where a degraded `--lanes` run shows: a skipped
lane is counted in the report itself, not only in its own line. It is
a report line and not a gate — a skipped lane leaves the install
green, because the lane is declared and dead, not broken.

**What touches the network.** The install's own steps are
filesystem-only — the layout, the config scaffold, and the lane venvs
are local writes, and the bare install performs no network I/O at
all. The network the command does perform is the gate's: it ends on
`fleet doctor`'s own checks, which include the API-reachability
probe (a TCP connect to the configured `DEEPSEEK_BASE_URL`) and the
recall probe (a real embeddings round trip against the local
endpoint). `--lanes` adds one network step of its own, the package
installs, and degrades without it.

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

The absence test is `symlink_metadata`, which does not follow the
final component. A **symlink** at the config path — dangling or not —
is named as such and left exactly as it is: `exists()` follows the
link, so a dangling one reads absent, and a plain write would send the
scaffold *through* the link into whatever it points at. Nothing is
ever written through a symlink, and a file where a directory belongs
(`<home>/sessions` as a regular file, say) is a named failure with a
nonzero exit, never a silent overwrite.

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
| `sys1` (judgment) | `gliner2[local]` | a config-file key: `[providers] local_venv` in `~/.config/sys1/config.toml` |
| `sdm1` (tabular) | `structured-data-models` | `export SDM1_MODELS_VENV=<venv>` |
| `sem1` (embeddings) | `sentence-transformers` | `export SEM1_ST_VENV=<venv>` |
| `gen1` (media) | `kokoro-onnx` | `export GEN1_KOKORO_PYTHON=<venv>/bin/python` |

The package sets are the fork's with one fix at the vendoring boundary:
`gliner2` declares its local-model stack — torch, transformers, numpy,
peft, safetensors — under the `local` extra, and
`gliner2/models/base.py` imports torch at module load, so the bare
`gliner2` the fork's bootstrap (and sys1's own setup note in
`providers.py`) names produces a venv that cannot serve. Verified: with
`gliner2[local]` the lane's python imports `gliner2.classification`
(the sys1 local provider's own import); with bare `gliner2` it fails
with `ModuleNotFoundError: No module named 'torch'`. The lane installs
`gliner2[local]`.

The three behaviors, each named in the output:

- **Present** — the venv's `bin/python` exists **and** the
  installer's receipt (`venvs/<lane>/.fleet-lane`, naming the lane,
  its package list, and the UTC minute it landed) sits beside it: the
  lane is detected and named, not rebuilt. The receipt is what makes
  the check honest. `uv venv` creates `bin/python` *before* the
  packages are installed, so the python alone is not evidence of a
  finished lane — a Ctrl-C or an OOM-kill mid-install leaves it in
  place, and a presence check on the python alone would read Present
  over a venv whose packages never landed. A venv without the receipt
  — an interrupted prior run, a hand-made directory, a venv built by
  a pre-receipt version of the installer — reads not-built: rebuilt or
  skipped, never misread as Present. (A pre-receipt venv therefore
  rebuilds once; that is the honest migration.)
- **Installed** — the venv was created and its packages installed;
  the receipt is written once they land, and the line names the wiring
  the core reads (the export, or sys1's config key with the real
  path).
- **Skipped** — the venv could not be built: no `uv`/`python3` on
  PATH, a failing create, a failing package install (no network), or a
  tool killed at the bound. The reason is named, the install still
  succeeds, and the lane stays declared and dead — slice 4's existing
  dead-lane behavior, never a hard failure. A half-built venv this run
  created is removed best-effort, so the lane's state stays honestly
  absent and a later run retries instead of reading "present" over a
  dead venv; a directory that was already there is retried over or
  skipped, never removed, and the receipt keeps it reading not-built
  either way.

Every lane subprocess runs under `LANE_INSTALL_TIMEOUT_SECS` (900
seconds — the torch stack legitimately takes minutes, but forever is
not a bound), and its lines stream to the operator as they arrive.
Two reader threads drain the child's pipes so a chatty child cannot
deadlock the wait, the parent waits with a deadline, and a tool that
has not finished within the bound is killed: the lane is skipped
naming the bound and the half-built venv is removed. A wedged
download, a tool waiting on a terminal, or a child that never exits
is therefore a named skip rather than a hung install.

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

A pre-existing defect rides along with that gate and is deliberately
untouched here: the doctor's **roster** check prints `FAIL` for an
invalid config without setting the gate's failure flag, so such a
config prints FAIL and still exits 0. Slice 15's refactor preserved
the behavior and its test locks the green exit. It is recorded in the
ledger — see `docs/known-issues.md` — and is not a slice-15 defect.

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
  is not valid JSON, and a **dangling symlink** at the config path is
  named and left dangling — nothing is written through it;
- a **file where a directory belongs** is a named failure with exit 1,
  never a silent overwrite;
- the exit-code contract — a green doctor is a green install, a red
  doctor (dead API port) is a red install with the failing check named;
- the quick start's deterministic core — install, then `fleet doctor`
  green on the home the install left;
- the lane bootstrap's three behaviors: a fake `uv` on a controlled
  PATH installs all four lanes with the fork's command shapes and a
  second run rebuilds none (the argv log is unchanged); a PATH with no
  venv tool skips every lane by name and the install stays green; a
  failing package step skips the lane by name and leaves no half-built
  venv behind;
- the **receipt** — a lane installed this run carries
  `venvs/<lane>/.fleet-lane` naming its packages and its UTC minute,
  and a second run reads Present off it; a partial venv with
  `bin/python` and no receipt is rebuilt rather than read present;
- the **skip summary** — a degraded `--lanes` run names the skips in
  the report's own summary line (`! lanes  0 of 4 lanes installed, 0
  present, 4 skipped`) and still exits 0.

The subprocess **bound** and the **streaming** are unit tests in
`src/install.rs`, because the production bound is fifteen minutes —
not a test's wait. They drive the bound-injecting seam
(`bootstrap_lane_bounded`) with a fake `uv` on a controlled PATH:
`a_hung_tool_is_killed_at_the_bound_and_the_lane_is_skipped` proves a
tool that sleeps is killed at 300ms, the lane is skipped naming the
bound, and the half-built venv is removed;
`the_progress_sink_receives_the_tools_lines_as_they_arrive` proves a
tool's lines reach the sink as they arrive (a `Vec` in the test,
stdout for the operator).

The release-binary and `fleet chat` halves of the quick start are
proven by hand against the live roster (the chat half is not
deterministic and does not belong in a suite); the executed output is
in the slice's session recap. The real `fleet install --lanes` was also
run for real on this machine: the machine's own `uv` built all four
venvs (Python 3.12, the fork's pin) and installed every package set
(1.6 GB), a second run named all four present and rebuilt none, the
sem1 venv's python imports `sentence-transformers` 6.1.0, and the
sys1 venv's python imports `gliner2.classification` once the `local`
extra's torch stack is in — the executed output is in the recap too.
