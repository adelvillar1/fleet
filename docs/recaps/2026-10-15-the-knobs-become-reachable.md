# The recall loop closes, and every knob becomes reachable

Status: complete
Date: 2026-10-15

## What the session did

`docs/recaps/2026-10-15-the-surfaces-land.md` covers the delegation
surfaces; `docs/recaps/2026-10-15-the-recall-loop-closes.md` covers the
recall loop's first closing. This one covers the arc since: the
continuous-self-improvement layer made real — the direction channel made
writable and then self-populating, the session archive made workable,
the desktop tool added with a ruling that outranks the execution modes,
compaction given an owner's hand, and every configurable surface made
reachable from both the CLI and the TUI through one writer.

## The increments, in order

**The direction channel became writable** (`e574db5`). `fleet remember`
gained `--phrases` and `--missing`; the CLI had been speaking a subset
of the writer's shape, so every fact it wrote scored trajectory 0.00 —
the ranking's largest term, dark on the whole surface-written corpus.
Six durable facts written with their resolution vocabulary.

**KI-058 re-measured on the current model.** Eight probe pairs through
the live embedder: unrelated 0.58–0.66, related 0.77–0.93. The
unrelated floor reproduces KI-058's 0.6049 within 0.03, which confirms
the slice-19 reweighting on embeddinggemma-2. No weight change
warranted; the ruling is now evidenced rather than assumed.

**The phrase-writing piece** (`dc16f07`). When a `remember` call
supplies no phrases, the harness derives the direction from the current
task's own successful tool calls. Two scopes keep the channel's meaning:
the window is the calls after the last user message, and only succeeded
calls count. Live proof: a real run called `remember` with no phrases
and the fact landed carrying `[sed -n '2010,2075p' src/agent.rs,
src/agent.rs, grep -n "trajectory_phrases" src/agent.rs]`.

**The session archive became workable** (`df95b06`). `fleet sessions
--recall` points the six-channel ranker at the archive, where a session
IS a trajectory. An instruction phrased in a session's own vocabulary
ranks it first at 0.64 with **trajectory 0.86**. A second
embedder-keyed index keeps the archive's cache independent of the facts
corpus's. `fleet sessions` renders cards instead of the plain-text
table that violated the house rule; `fleet sessions <id>` opens a log as
cards.

**The web tool reaches every agent** (`df95b06`). `web` is declared on
every run and admitted to a delegate's scope by default, resolving
blend1's spec itself so it works with the empty specs map a plain
`fleet run` passes. blend1's MCP server grew a `web` tool calling the
web leg without paying for the compose.

**The route-decision cache** (`cd2fce4`). `select` had judged every
route from scratch — spawning sys1, initializing, classifying, closing.
Live proof: the second run of one task served from the record at
latency 0 ms against the first's 1344 ms, same route and verdict.

**Compaction gained an owner's hand** (`5aabaa6`). `fleet compact
[--force]` folds a session's stored context on demand: live proof
folded 2 turns into 1 summary, ~34038 → ~219 tokens, record appended,
calib row landed. `fleet run --compact-now` forces the fold at the next
turn boundary — and its unit test caught a real bug, the flag not being
consumed on the no-op path.

**The computer-use tool** (`db217b3`). The cua driver turned out to be
a real MCP server with 56 tools, so the tool is the same lane-client
path the judge uses, with a small named action set over it. The ruling —
computer use asks in every mode including `yes` — was encoded before the
tool existed, and **the live run proved that was not enough twice over**:
`Mode::Yes` mapped to `ApproveAll`, an approver with no always-ask
check, so `--yes` dispatched a desktop action with no prompt at all;
and once fixed, the denial lied, reporting "the user did not approve"
when there was no user and the mode said yes. Both were invisible to
the unit tests and only appeared by running it.

**Every configurable surface, reachable** (`99ac009`, `e41fed2`,
`74036d0`). `src/settings.rs` is one writer over the home's three
files; `fleet settings` and the TUI's `ctrl+,` pane are two views over
it. The loader's tolerance on read, strictly more strictness on write;
an owner's hand-written extras survive an edit; atomic writes; and a
malformed file is preserved as evidence rather than silently
overwritten. The list-shaped surfaces carry a destructive rule: removing
an override of a compiled-in provider reverts it, removing one the file
added removes the provider from every walk — refused without `--force`.

## Errors worth keeping

- **The port probe.** Checked llama-server's default 8080 instead of
  fleet's own 8901, and reported "no loopback embedder" from it. The
  endpoint under test is the one the code declares, not the one the
  server binary defaults to.
- **The fixture collision, third instance.** A new test in an unrelated
  module named `zorbnicator` in a string literal and killed the
  honest-miss proof — the vocabulary derives from the whole tree. The
  preventive rule is in the `fleet-repo-conventions` skill.
- **The doc-channel collision.** The recap describing that collision
  named the word in backticks, and the doc channel re-established it,
  killing the proof a second time from the doc that reported the
  source-channel instance. The skill now covers docs that REPORT a
  collision too.
- **The truncated status.** `git status --short | head -12` cut off the
  line naming a fact written that turn, and the commit claimed six
  facts while seven existed. The skill records: read the status in
  full, filter noise with `grep -v`, never `head`.
- **A subagent's "pre-existing red tests" were artifacts of its scratch
  copy.** Re-ran them on the real tree: 33 passed, 0 failed. Re-verify
  what a child reports; do not relay it.
- **`fleet sessions` claimed "newest first" while `session::list` hands
  them over oldest first**, so a bare `fleet compact` was folding the
  OLDEST session on the machine. Found by the live run, fixed.
- **My own wrong prediction, caught by the code being right.** I
  expected blend1 was not compiled in and that removing its override
  would need `--force`; it is compiled in, the rule worked, and my grep
  of the roster had missed it.

## Disclosed gaps

- **KI-207**: the cua driver on disk (v0.21.0) is incompatible with the
  running daemon (contract 0.8.0 vs SDK 0.7.0). Everything fleet
  controls is verified live; the driver's own answer is unreachable.
  Not updated unilaterally — replacing a system binary is the owner's
  call.
- **The web tool's `fetch`/`max_urls` are validated but not forwarded**:
  the leg takes the query alone, so the lane's own bounds are the real
  clamp and the schema says so.
- **Session-recall's `recall_hit` row** needs its own count store;
  routing a synthetic `Memory` through `recall_hits` would bump the
  `.jsonl` log.
- **The TUI settings pane has never been opened by a human.** Behavior
  is pinned by unit tests and render by a TestBackend assertion; the
  interactive experience closes by opening it.
- **C4's in-pane confirmation is the unbuilt half** of the destructive
  rule — the pane refuses where the CLI confirms with `--force`, which
  is the safe direction and is disclosed in the plan.

## The gate

977 passed / 2 ignored at the final commit; release exit 0,
warning-free at each increment. rustfmt and clippy remain uninstalled
on this machine, so the style was matched by hand and the gate was run
locally throughout — CI is disarmed by budget (owner decision).
