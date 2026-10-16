# The findings land

Status: complete
Date: 2026-10-15

## What the session did

The warmup corrected itself — the DeepSeek balance was never a blocker
(KI-008 held the truth; the CLAUDE.md bullet was stale) — the two
delivered plans closed with their deferred legs named, and the deep
dive the owner asked for became a plan and then an implementation: six
phases across two delegation waves with a two-stage review, every
increment gated locally and committed as it went green.

## The increments, in order

**The two stale plans closed.** Each verified criterion by criterion
against the code before its status flipped: web-tool-for-every-agent
(C6 deferred — no fleet surface named the web leg) and
session-archive-recall (C7 deferred by the module's own record).

**The DeepSeek correction.** `CLAUDE.local.md` fixed, KI-008
re-confirmed with the sharper last-resort semantics, and the CLAUDE.md
bullet corrected under the owner's instruction — which then grew into a
full refresh of the file's state, module map, and command list once the
deep dive had informed it.

**The deep dive.** Forty-five modules surveyed: the routing core, the
agent loop, and the recall ranker read line-deep by the parent; the
recall core, the calibration cluster, and the surfaces read by three
isolated subagents and spot-verified at `path:line` before anything was
written down. The survey lives at
[docs/architecture/deep-dive-2026-10-15.md](../architecture/deep-dive-2026-10-15.md);
`overview.md` carries a staleness banner pointing at it.

**The findings plan, implemented.** Six phases, five implementer
subagents across two waves (file-disjoint inside each wave), the parent
owning every `main.rs` wiring per the orchestrator-wiring rule, two
read-only reviewers (spec, then quality) over the first wave, the
review's must-fixes taken directly by the parent, and a converge pass
grading every AC against the committed code. What landed: the settings
pane reaches the list surfaces with the in-pane destructive
confirmation; `fleet doctor` names the web leg and the lanes card keeps
blend1's own availability; the recall-hit feedback moved to after the
model answers (KI-068 closed-fixed); the doc drift's five instances
corrected; the delegation cells earned their consumer with the fit
arithmetic aligned; and the session archive got its own count store and
hit rows.

## Errors worth keeping

- **The ownership-seam drift — the review's critical.** Phase 2 chained
  the web entry into a Vec the TUI's lanes pane also renders; the CLI
  card was updated to give the web leg its own accented line, the TUI
  card was not — and the suite stayed green because the TUI fixture
  carried five entries while the live probe produces six. The plan's
  own shared-file note predicted the seam; the fix filters at the
  render and makes `WEB_LEG` pub so both surfaces read one constant.
  The lesson: a change to a shared shape owes a grep of every consumer,
  not just the one in the brief.
- **The parent's own wiring error.** The implementer's recipe named
  `Candidate::id()`, which does not exist — the field is `pub id`. The
  compiler caught it because the parent ran the gate on the real tree;
  a recipe from a child is a hypothesis about the API, and the build is
  what verifies it.
- **The doc-channel collision, third instance.** The nomenclature
  honest-miss proof was red at HEAD: a recap reporting the fixture
  collision named the nonsense word in a backtick span, and the docs'
  backticks are vocabulary. The recap now writes it in plain prose
  (the doc channel does not read prose), and the proof's comment
  records the retirement mechanism — the third one, one more backtick
  away.
- **The pre-existing red both children classified correctly.** Each
  wave-1 implementer proved the nomenclature failure pre-existing with
  a stash or worktree reproduction rather than asserting it — the
  discipline held exactly where it was designed to.
- **The transient shared-tree red.** Wave 2's phase-6 child hit a
  failing delegate test that was the phase-4 sibling mid-rename; it
  waited and retried rather than touching a file it did not own. The
  expected shape of a shared tree, handled by the rule.

## Disclosed gaps

- The TUI settings pane has never been opened by a human. Behavior is
  pinned by the state-machine tests and render by TestBackend
  assertions; the interactive open remains the experience's proof.
- The pane's `provider.add` is an upsert — editing a seeded entry's id
  creates a provider from the pane. The card re-read disambiguates; the
  writer's return does not yet say add versus update, and the
  structured two-field editor is the named follow-up.
- C7's misspelled-field refusal is asserted at the writer's level, not
  re-asserted through the pane (the same code path, one assertion
  short of symmetric).
- The doctor/lanes split is deliberate: doctor names the key state,
  lanes carries the lane's own report. On this machine the two honestly
  disagree — the key is set in the process env, the lane's venv lacks
  the tinyfish SDK — and each surface names its own truth.
- The ten `memories/*.md` recall-count bumps predate this session (the
  previous session's live runs) and remain uncommitted runtime state.
- rustfmt and clippy are absent on this machine; every increment
  matched style by hand and disclosed it. CI is disarmed by budget; the
  local gate is the only gate.

## The gate

Final state: `cargo build --all-targets` exit 0, warning-free;
`cargo test --no-fail-fast` exit 0 — 998 passed (876 lib, 6 blend1,
2 + 7 + 11 integration, 96 the main suite), 0 failed, 2 ignored (the
live-API e2e and its sibling). Live surfaces exercised: `doctor`,
`lanes`, `providers`, `settings` (a scratch-home round-trip),
`delegate --dry-run`, `sessions --recall` (22 hits, 22 rows, the store
bumped, the logs untouched), `calib`, and `eval` (20 scenarios, pass).
