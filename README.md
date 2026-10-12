# Fleet

Fleet is the native, cross-platform launcher for the [DeepSeek Harness](../README.md) — a single Rust binary for macOS, Linux, and Windows that boots the harness as-is, so every `dsh` surface stays exactly as the harness ships it.

**Fleet is based on DeepSeek Harness.** It is not a port or a reimplementation of the agent runtime: the interface must remain intact, and the only way to guarantee that is to launch the harness itself. Fleet owns discovery, launch, and health checks; the harness owns the product.

## Status

Alpha. The launcher is standard-library-only Rust (no third-party crates), so `cargo build` works offline on every supported platform.

## Requirements

- Node.js `^22.19 || >=24`, the harness's own engine floor
- A DeepSeek Harness checkout with dependencies installed (`pnpm install` at the checkout root)

## Build

```sh
cd fleet
cargo build --release   # binary at target/release/fleet
cargo test              # behavior tests, loopback only
```

## Commands

### fleet [run] [profile] [options] [-- <harness arguments>...]

Launches the harness exactly as `dsh <profile>` does:

```sh
fleet                              # same as: dsh web
fleet run fleet                    # same as: dsh fleet
fleet run fleet --patch ./overlay.yml
fleet run web -- --no-open --port 4080   # words after -- go to the harness verbatim
```

Options: `--home <path>` (harness checkout), `--node <path>` (Node binary), `--profile <name>` (profile under `$DSH_HOME/profiles`), and repeatable `--patch <path>` overlays applied after the profile layer. Fleet's own flags are recognized anywhere before `--`; unknown flags pass through to the harness, mirroring the launcher. Ctrl+C stops the harness through the terminal process group, exactly as for a directly launched `dsh`, and the harness's exit code becomes Fleet's.

### fleet status [--host <host>] [--port <port>]

Probes the harness web UI's TCP port (default `127.0.0.1:3080`) and exits 0 when something is listening.

### fleet doctor [--home <path>]

Prints one line per check: node presence against the version floor, the harness checkout, installed dependencies, and the profiles under the dsh home. The web UI probe at the end is informational and never affects the exit code.

## Where Fleet looks

- Harness checkout: `--home`, then `FLEET_HOME`, then `~/.fleet/harness`, then the current directory — the checkout is identified by the harness CLI entry `apps/cli/src/bin.ts`
- Node binary: `--node`, then `FLEET_NODE`, then `node` on `PATH`
- User home: `HOME` on Unix, `USERPROFILE` on Windows; the dsh data home is `DSH_HOME` or `<user home>/.dsh`

## Platform support

macOS (arm64, x86_64), Linux (x86_64, arm64), and Windows (x86_64) are first-class: the crate is standard-library-only, uses no platform-specific APIs, and cross-compiles with the usual Rust targets.

## Based on DeepSeek Harness

DeepSeek Harness (https://github.com/deepseek-ai/deepseek-harness) is the agent runtime Fleet launches: the profiles, plugin bundles, web and desktop clients, and the `dsh` CLI are all the harness's own, and Fleet adds only distribution. Both projects are MIT-licensed; see [LICENSE](../LICENSE).
