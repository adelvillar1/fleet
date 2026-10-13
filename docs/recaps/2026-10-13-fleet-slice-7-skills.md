# Slice 7 — skills, with self-authoring (2026-10-13)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 7).
**Feature:** `docs/features/skills.md`.
**Ledger:** KI-052 (the drafter is deterministic, not model-written),
KI-053 (a drafted lesson is never re-drafted), KI-054 (proposal names
collide across kinds), KI-055 (the card family wraps two ways),
KI-056 (the usage scan double-parses).

## What landed

`src/skills.rs` — the skills module, plus the seams that carry it.

- **Loading.** One directory per skill under `<FLEET_HOME>/skills/`,
  each a `SKILL.md` with `name` + `description` frontmatter and the body
  after the fence. The frontmatter subset is hand-parsed: `key: value`
  lines between `---` fences, quotes stripped on both halves, unknown
  keys kept for forward compatibility, a leading UTF-8 BOM stripped.
  Rejected with the reason named: no opening or closing fence, a nested
  block or list entry or bare scalar inside the fence, a duplicated
  key, an empty key or value. A malformed skill is **skipped by name**
  with one logged line and the others still load; a live skill carrying
  `proposed: true` is refused (a proposal that reached the live tree is
  not promoted silently); the frontmatter's `name` must equal the
  directory's.
- **Progressive disclosure.** Every run's system prompt carries the
  skills as a name+description list (past a soft cap of 50, one logged
  line warns that the list is growing the prompt — nothing is hidden).
  A body enters context only through `use_skill(name)` — declared beside
  the closed four when a run has skills — and rides the ordinary
  approval-gated `tool_result` path, bounded by the same
  `MAX_TOOL_OUTPUT` cap and marker. An unknown name is a typed error
  result. Model-visible ⟺ logged falls out of the existing seam with no
  new record type.
- **Reconstructability.** One `skills` session-log record per run at run
  start (the `lane_tools` precedent), replaying to no message. A run
  with no skills records nothing and its prompt is the compiled-in
  constant, so the skill-less run is byte-for-byte unchanged.
- **The self-authoring half.** After a session — `run` after its task,
  `chat` after its loop, the TUI when its agent thread ends — a
  deterministic drafter reads the session log plus the day's calibration
  rows and files create/update proposals into `skills/proposals/`: a
  tool call that failed and was then corrected by an APPROVED rerun
  (records and both commands cited), a repeated multi-step sequence of
  approved calls, or a `turn` row graded failed with a named class
  joined to the session by the task's `input_hash`. At most three
  lessons per session. Nothing applies a proposal.
- **Curation.** The usage scan: the last N session logs (default 20,
  `skills.retire_after_sessions`), `use_skill` calls whose results were
  approved, counted per skill. Unused across the whole window → retire
  proposal; two descriptions at 0.8 token Jaccard → merge proposal, the
  absorbed twin getting no separate retire draft. The short window
  proposes nothing — "unused across the last 20" is not knowable from
  three sessions.
- **`fleet skills`.** Cards in the lanes/providers/calib style: name,
  description, usage count, mtime. `--proposed` lists the drafts with
  kind and evidence; `--curate` runs the scan; `--show <name>` prints a
  draft verbatim (a name carrying path separators or `.`/`..` is
  refused). No new TUI surface — a skill's use is the ordinary tool
  card.

## The gate boundary

No code path in this slice moves a proposal into the live tree. The only
writer in the drafter and the curation is `skills/proposals/`; the
proposals loader refuses a live skill carrying `proposed: true`; the
surface names slice 12's gate on every proposal card. Applying is slice
12's: gate green, non-destructive or approved, recorded in the ledger.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` → **232 unit + 56 integration passed,
  1 ignored** in the final state. rustfmt and clippy are still absent on
  this machine (KI-007) and were not run.
- The AC's literal checks, exercised: a malformed skill is skipped by
  name while the others load (unit test, plus a real-binary probe on a
  scratch home printing `fleet: skills: skipping \`broken\` — the file
  does not open with a \`---\` frontmatter fence` beside the loaded
  cards); a selected skill's body enters context as the tool result and
  the next request reads exactly those bytes; an unknown name is a typed
  error result; `fleet skills` renders cards. The parent re-ran the
  scratch-home probe on the final tree, including a BOM-prefixed skill
  (loads) and `--show ../known-issues` (refused: a name carries no path
  separator).
- The self-authoring half end to end: a scripted session whose `bash`
  call failed and was corrected drafts `bash-recovery.md` with
  `proposed: true`, `kind: create`, the session id, both record indices,
  and both commands cited — and the live tree holds exactly the one
  skill it held before.
- Curation end to end: four skills, a two-session window from
  `config.json`, one used pair and one unused — a retire draft for the
  unused skill and a merge draft for the near-identical pair, with the
  absorbed twin getting no retire draft and the live tree unchanged.
- UX exercised by hand against the real binary on a scratch
  `FLEET_HOME`: `fleet skills` cards, `--curate` filing proposals,
  `--proposed` listing them with kind and evidence, `--show` printing a
  draft verbatim.

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all 12 checks;
  the compiled-in system prompt stays constant with the dynamic list
  appended per request and recorded once per run (hard rule 5 holds),
  and the gate boundary is proven from the write paths.
- **Stage 2 (quality, step-5-preview): APPROVED** — no criticals; one
  Important and seven minors, all fixed in `34e5922`:
  - the `failure_correction` detector's early return silently dropped
    every correctable failure after the first uncorrected one — it now
    skips to the next failure (the doc's "first such pair" is true
    again), with a test whose first failure is uncorrected and second
    corrected;
  - denied calls no longer count as corrections or as repeated-sequence
    evidence (`Executed` carries `approved`; a half-denied turn drafts
    nothing);
  - a BOM-prefixed SKILL.md loads (and a still-failing BOM'd file names
    the BOM in its reason);
  - quoted frontmatter keys unquote (one rule for both halves of a
    pair), with a named rejection for an unbalanced quote;
  - the drafter's evidence carries both commands (compacted, bounded),
    so the owner can judge a draft at the gate without replaying;
  - `--show` refuses separator-bearing names with a typed error;
  - the skills list warns past a soft cap of 50;
  - KI-056 records the usage scan's double-parse (the fix belongs with
    `session::list`).

## Not done

- Model-written proposal prose (KI-052): the bodies are evidence-shaped
  templates, reviewed deterministically.
- Evidence accumulation onto an existing draft (KI-053).
- Pair-named merge drafts (KI-054).
- Slice 12's gate itself: nothing here applies, and the proposals wait.
- The TUI's drafter wiring is compile-verified and mirrors headless; it
  has not been driven through a live TUI session (the standing accepted
  gap).
