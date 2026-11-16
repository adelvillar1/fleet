---
title: the login shell's PATH — the app sees the machine it runs on
date: 2026-11-15
status: completed
---

# The login shell's PATH

Three complaints arrived together from the desktop app on 2026-11-15:

- **the runtime card**: "the kiri daemon is not answering — the LayerFS
  worker cannot run · repair: start the container runtime (kiri or Docker)"
- **the MCP lanes**: cloudflare, dappier and filesystem "could not start:
  could not spawn the MCP server: No such file or directory (os error 2)"
- (silent, same cause) blend1 refused: "python 3.10+ is required … this
  interpreter is 3.9.6"

## Measured, not guessed

| probe | terminal-launched face (3080) | dock-launched app (64207) |
|---|---|---|
| `fleet doctor` / layerfs | `ok — worker answering · image present` | card says the daemon is not answering |
| `/api/lanes` blend1 | `ok` | `error — this interpreter is 3.9.6` |
| process env | the shell's PATH | `PATH=/usr/bin:/bin:/usr/sbin:/sbin` |

`ps eww -p <fleet-desktop>` shows the app's whole PATH is launchd's
four directories. Everything the owner installed lives elsewhere —
`docker` at `/usr/local/bin`, `npx` at `~/.local/bin`, `python3` at
`/opt/homebrew/bin`, `kiri` at `/opt/homebrew/bin`. So:

- `health::docker_answering()` runs `docker info`, the spawn fails with
  ENOENT, and the card reports the **daemon** as down while it answers
  (it answered `0.2.0`, and the worker is up on `127.0.0.1:56511`).
- `mcp.rs` spawns `npx …` and gets the same ENOENT — the exact string
  the app shows for cloudflare, dappier and filesystem.
- lanes spawn `python3` and get macOS's 3.9.6 instead of 3.13.

One root cause, three symptoms: the app never had the environment the
owner's terminal has.

## The fix

Adopt the login shell's PATH once, in the desktop shell's `run()`, before
the first run, the lane build, any probe, or the face starts. A pure
module (`fleet::shellpath`) does the read and the merge so both halves are
testable without a machine:

- the read runs `$SHELL -lic 'printf <sentinel>$PATH<sentinel>'`, bounded
  at 3s, stdin null, stderr null, falling back to `-lc` when the
  interactive shell says nothing. The sentinel survives profile chatter;
  a timeout kills the child and yields nothing.
- the merge puts the login entries first (the owner's own ordering is
  authoritative) and appends inherited entries the login PATH lacks,
  dropping empties and duplicates.
- a read that fails, hangs or says nothing leaves PATH untouched — a
  narrow PATH is a degradation; an app that will not start is not.

And the card stops asserting a cause the check never verified: the
message names the probe (`docker info` did not answer), not the daemon.

## Acceptance criteria

- [x] C1: `fleet::shellpath` exists with `login_path(shell)`,
  `parse_login_path(output)`, `merge(current, login)` and
  `adopt_login_path()`; tests pin sentinel extraction from noisy output,
  merge order and de-duplication, an empty read, and a real `/bin/sh`
  read.
- [x] C2: the desktop shell adopts the PATH at the top of `run()` —
  before `first_run`, `lane_environment`, `build_lanes` and `start_face` —
  and prints one line naming what it adopted (or that it left PATH alone).
- [x] C3: the runtime card's down-message and its repair line name the
  check that failed, not an unverified daemon state.
- [x] C4: the gate is green (`cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
  `cargo test`), each exit read directly.
- [x] C5: deployed and verified live — the app rebuilt and installed: the
  runtime card is gone, `blend1` answers `ok` under the app's own process,
  and the three MCP servers probe `ok` from the app.
- [x] C6: KI-544 records the defect, the measurement, and the fix.
