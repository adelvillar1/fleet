---
name: fleet-gate-exit-code-trap
description: "In the fleet project cargo is not on PATH and piping the gate through tail masked a total failure as exit 0 — run the gate with the PATH export and read cargo's own exit code; the same trap recurred 2026-10-20 and also fires when only the cargo binary is called by absolute path (rustc is still not resolvable)"
type: feedback
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

During the 2026-10-15 deep dive I ran `cargo test 2>&1 | tail -25` in the background and read "exit 0" as a green gate — but cargo wasn't even on this shell's PATH (`command not found: cargo`) and the pipe made the exit code tail's, not cargo's. The repo's own KI-014 rule ("never through a status-losing pipe") exists precisely for this and I hit it anyway on the first try.

**It recurred verbatim on 2026-10-20**, building the rail-mark change: `cargo build --release 2>&1 | tail -20` returned a "completed, exit code 0" background notification whose only content was `command not found: cargo`. I announced the build before reading the log; the next `fmt` run failed loudly (`no such command: fmt`) and the rebuild under the absolute path then died too (`could not execute process rustc -vV (never executed)` — cargo resolves `rustc` off PATH like any other tool). The working invocation is **both** on PATH:

`export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"` — in the fleet project, `~/.cargo/bin` does not exist at all (only `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`, where `cargo`, `rustc`, `rustfmt`, `cargo-fmt` live). A wrapper in `~/.local/bin` or another session's shell profile may put cargo elsewhere, so verify with `command -v cargo` before trusting a background build.

**Why:** a compound command's exit code is the last command's; a failed gate behind a pipe reads as success, and a red suite reaching the owner is the exact incident KI-014 records. And a "build finished, exit 0" notification from a background task is exactly as meaningless as a `tail` pipe unless the log has a `Finished` line.

**How to apply:** run the gate as `... cargo test > log 2>&1; echo "EXIT=$?"`, then read the log's `Finished` / `test result:` line — never a piped tail, never the background notification alone.

**It fired a third time on 2026-10-23, in a form the other two didn't cover: it truncated the RESULT, not the exit code.** A four-leg gate written as `{ ...; cargo clippy ... 2>&1 | tail -5; echo "clippy exit=${PIPESTATUS[0]}"; ...; cargo test 2>&1 | tail -25; echo "test exit=${PIPESTATUS[0]}"; } > log 2>&1` returned exit 0 and read clean — but `${PIPESTATUS[0]}` is not a zsh parameter (it is empty there, and bash's array only ever holds the *last* pipeline's status), and the `tail -25` kept only integration + doc-tests: the 1360-test lib result was discarded. The totals were 110 passed, not 1498. Recovery cheap and worth doing: `grep "test result:" log` and sum — a suite whose binaries don't all appear was truncated, not green. The safe shape is one command per leg, each with its own exit captured (`cargo test > log 2>&1; echo $? > log.exit`), then read the log.

**Fourth form, same day (2026-10-23): the reporter itself lied.** A background leg file ended with `test exit=` — the `$?` captured after a `}` group had already run, and the backgrounded command's own "exit code 0" notification reported the wrapper, not the leg. The tell was a labelled line with no number after it; the fix was to re-run `<cmd> > log 2>&1; echo $? > log.exit` as its own single command and read `log.exit`, plus `grep "test result:"` on the log to confirm all binaries appeared (idle 1360 lib + integration ran; the earlier truncated pass had shown only 110). Interim progress reads (`tail` of a file still being written) are fine, but never read a leg's verdict from a file whose reporter is still writing.

**Fifth form (2026-10-23), the plainest of all: a backgrounded leg file whose four exit lines all read 127 and whose log began `zsh:3: command not found: cargo`.** The PATH was on my interactive shell but not on the background subshell, and I had announced the gate as running before reading the log. The recovery is mechanical and took one step once I looked: `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"` inside the same compound command, then re-run the four legs. **The tell is the `=` line with a non-zero number, or a log whose first line is `command not found`** — if you are about to say "the gate is running", read the log's head first.

**Sixth form (2026-10-23), the quiet one: every leg reads 0 and the tree is still reddenable.** A full gate returned `FMT=0 CLIPPY=0 BUILD=0 TEST=0` and the log's first `test result:` line was `1371 passed; 1 failed` — I read the four `=0` lines and moved on, then read the totals only on a second pass. **`TEST=0` is the *last* leg's status, not the suite's**: the four-leg compound's exit lines are honest about each command, but a person (or a summary) reads `TEST=0` as "the tests passed". The reliable habit is `grep -c "^test .* ok$"` plus `grep "^test result:"` and reading the FAILED line before touching the exit codes — and when one red shows up three times in a session, stop re-running it and fix the mechanism (the KI-287 recurrence this exposed).

**A variant that is NOT this trap, seen 2026-10-23 and worth telling apart: a leg printed `TEST=101`.** 101 is the `--lib` death-signal base (a test binary that panicked or aborted reports 101, not 1), so the reporter was honest and the red was real: `no_payload_carries_a_credential` matched the CSS class `task-wrap` (the scan forbids the literal `sk-` in any served asset). One `cargo test --lib <name>` confirmed the mechanism in 0.04s and the suite went green after the rename. **So read the exit code's value, not just its non-zeroness** — 127 is the missing-cargo form above, 101 is a red test, and only the second needs a cause hunt.

**First form of the day again, 2026-10-23: the baseline itself was a false green.** The very first `cargo test --lib --quiet 2>&1 | tail -20` run in the background "completed, exit code 0" and its log's only line was `zsh:3: command not found: cargo` — I had read the notification and started planning on a suite that never ran. `~/.cargo/bin` and `/opt/homebrew/bin` carry no cargo; the only toolchain is `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` (cargo 1.89.0). From then on every leg was `export PATH=...; cargo … > log 2>&1; echo "EXIT=$?"` and read on the number. **A baseline is a leg too** — do not announce "baseline is green" until the log has a `test result:` line.

The same trap bites the GUI: since G5 (dfdaefb), a skills approve from `fleet gui` rides `improve::apply`, whose gate spawns `cargo` on the process's own PATH — a `fleet gui` launched from an environment without cargo refuses every approve with "could not run `cargo`" (named, not silent), so launch it from a shell that has the rustup PATH. Related: [[deep-dive-2026-10-15-second-pass-findings]], [[a-verification-with-no-output-is-not-a-pass]] (the content half of the same trap).
