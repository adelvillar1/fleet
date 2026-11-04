# Session recap — 2026-11-04: context health, the measure that replaced the turn count

**Summary.** The turn budget's deferred discussion arrived with a
direction: measure context health and show it on the conversation,
beside the working spinner. The plan
(`docs/plans/2026-11-04-context-health.md`) landed whole — 11 of 11
criteria ticked — and the owner's steer mid-build sharpened it: the
budget is a function of the model's context window and of nothing else,
and the usage reads whatever provider the router actually served, never
one model's contract. The turn count has left the user surface entirely.

## What was built

- **One arithmetic, everywhere.** The agent's fold already derived its
  budget from the serving model's window; the face's gauge divided by a
  hardcoded 100k floor — the divergence the plan existed to close. Now
  every `UiEvent::Usage` carries the run's own `budget_for(serving_window())`
  (a mid-run pin changes the budget with the model), and the state read
  derives it for the next run's model: the pin when it names a model,
  else the conversation's last served model off the log's `End` record,
  else the configured model — with `budget_declared: false` naming the
  floor when nothing declares a window.
- **The health line.** The working line reads the measure the fold
  checks — occupancy against the budget, the folds so far, and the
  trend's turns-to-the-fold once the last three prompt readings climb
  (an approximation, shown only then) — and the turn count is gone from
  it. The head meter shares the arithmetic: calm / high / fold-imminent,
  and the hover names the declaring model (or the named floor), the
  folds, the first-hand share, and the cache-hit ratio.
- **The survival check.** At each fold the agent measures the named set
  — the task, the goal, every `write_file`/`send_file` path, every
  ledger item — against the working set the fold leaves, and the
  compaction record carries the answer. Presence is containment; the
  paraphrase limitation is stated at the definition, the manifest seam
  deferred.
- **The first-hand share.** `verbatim_share` classifies the working set
  by the fold's own markers (summary frames and elision stubs) and rides
  the usage event — how much of the conversation is still word-for-word.
- **The cache canary.** `Usage.cache_read_tokens` reads whatever the
  served provider reports: the Anthropic name as before, DeepSeek's
  `prompt_cache_hit_tokens` by alias, OpenAI's nested
  `prompt_tokens_details.cached_tokens` by a serde capture folded at
  the two chat parse sites — nothing pinned to a model.
- **The resume seed.** The log gains the usage record (bookkeeping,
  replays to no message) beside each answer; the state read takes the
  last of those plus the fold count, so a reopened conversation shows
  its health, never a zero.

## The verification

- Gate: fmt 0, clippy 0, release build 0, `cargo test` 0 (1,838 lib +
  164 integration + the eval corpus and the rest), harness **844 checks
  / 95 cases** exit 0.
- **The live proof** (`/tmp/fleet-health-proof`, the machine's
  credentials in its environment, nothing on disk): seventeen runs
  through the real API. The router served step-5-preview and the budget
  followed it (614,400); the owner's per-model window override moved
  the budget to 100,000 mid-conversation without a restart; the fold
  fired at a two-turn boundary through the owner's own `{"fold": true}`
  control — **198,036 → 42,186 tokens**, one compaction record,
  survival kept 1 of 1; the wire event carried
  `{"budget":100000,"cache_read":3968,"prompt":49724,"share":97,"folds":0}`;
  the state read seeded the reopened conversation (context 49,710,
  folds 1).
- Delivery: `~/.fleet/bin/fleet` replaced through a new inode and the
  service restarted (the served face carries the health code; the
  console's empty session names the floor); the Tauri dmg rebuilt
  (10.39 MiB) and its packaged binary verified to carry the new
  surface; the branch pushed (`b28d04b..d23ea4c`).

## Found on the way

- **The eval corpus's log-shape counts grew by the usage record**
  (re-measured 10/10/11), and the position-based asserts moved one slot
  — the corpus's "the log reconstructs the turn" checks now read the
  assistant's call positionally instead of by fixed index.
- **KI-426 (open)** — the layerfs stub tests are load-sensitive: three
  timed out their 10s `--version` wait under the full parallel suite
  and passed alone. The KI-385/KI-405 class; the cure is the class's own.
- **The one-arithmetic seam the proof caught** — the fold's window
  lookup is per (provider, model) pair, the gauge's state read scans
  all providers for the model name; an override under a provider other
  than the one that served reaches the gauge but not the fold. The
  proof's first override sat under `deepseek` while `stepfun` served,
  and the fold correctly declined to fire. The contract holds — one
  function, the fold's — and the gauge's pre-run by-model scan is the
  honest approximation it names; but an owner override belongs under
  the provider that serves, and the config writer's discovery path
  already puts it there.
- **Models dodge tool calls** — driving a two-turn shape live took the
  owner's forced-fold control (`{"fold": true}` in the session's
  control file) after three models answered one-turn puzzles meant to
  require a read. The control worked exactly as designed, mid-run, and
  is now the recorded way to prove a fold without a 100k-token
  conversation.

## Where it stands, honestly

- The floor (`COMPACTION_TOKEN_BUDGET`, 100k) remains the fallback for
  a model nothing declares — the owner's steer removed it from the
  surface as a *denominator*, not as the fallback it is; its comment
  still cites DeepSeek's 128k as its tuning, a pre-existing note.
- The survival check's containment is honest-but-crude for prose; the
  manifest seam and the sem1 coverage score stay deferred increments.
- The turn-budget *machinery* (the per-run turn cap) stays; the
  2026-11-03 deferred question about it was never this plan's to
  answer.
