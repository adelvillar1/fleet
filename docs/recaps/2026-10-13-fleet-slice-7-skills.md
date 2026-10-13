# Slice 7 — skills, with self-authoring (2026-10-13)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 7).
**Feature:** `docs/features/skills.md`.
**Ledger:** KI-052 (the drafter is deterministic, not model-written),
KI-053 (a drafted lesson is never re-drafted), KI-054 (proposal names
collide across kinds), KI-055 (the card family wraps two ways).

## What landed

`src/skills.rs` — the skills module, plus the seams that carry it.

- **Loading.** One directory per skill under `<FLEET_HOME>/skills/`,
  each a `SKILL.md` with `name` + `description` frontmatter and the body
  after the fence. The frontmatter subset is hand-parsed: `key: value`
  lines between `---` fences, quotes stripped, unknown keys kept for
  forward compatibility. Rejected with the reason named: no opening or
  closing fence, a nested block or list entry or bare scalar inside the
  fence, a duplicated key, an empty key or value. A malformed skill is
  **skipped by name** with one logged line and the others still load; a
  live skill carrying `proposed: true` is refused (a proposal that
  reached the live tree is not promoted silently); the frontmatter's
  `name` must equal the directory's.
- **Progressive disclosure.** Every run's system prompt carries the
  skills as a name+description list. A body enters context only through
  `use_skill(name)` — declared beside the closed four when a run has
  skills — and rides the ordinary approval-gated `tool_result` path,
  bounded by the same `MAX_TOOL_OUTPUT` cap and marker. An unknown name
  is a typed error result. Model-visible ⟺ logged falls out of the
  existing seam with no new record type.
- **Reconstructability.** One `skills` session-log record per run at run
  start (the `lane_tools` precedent), replaying to no message. A run
  with no skills records nothing and its prompt is the compiled-in
  constant, so the skill-less run is byte-for-byte unchanged.
- **The self-authoring half.** After a session — `run` after its task,
  `chat` after its loop, the TUI when its agent thread ends — a
  deterministic drafter reads the session log plus the day's calibration
  rows and files create/update proposals into `skills/proposals/`: a
  tool call that failed and was then corrected (records cited), a
  repeated multi-step tool sequence, or a `turn` row graded failed with
  a named class joined to the session by the task's `input_hash`. At
  most three lessons per session. Nothing applies a proposal.
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
  draft verbatim. Exit 0 (a read surface); a `--show` miss is the one
  failure. No new TUI surface — a skill's use is the ordinary tool card.

## The gate boundary

No code path in this slice moves a proposal into the live tree. The only
writer in the drafter and the curation is `skills/proposals/`; the
proposals loader refuses a live skill carrying `proposed: true`; the
surface names slice 12's gate on every proposal card. Applying is slice
12's: gate green, non-destructive or approved, recorded in the ledger.

## Evidence

- Gate: `cargo build --all-targets` exit 0, **zero warnings**; `cargo
  test` → **223 unit + 56 integration passed, 1 ignored** (was 198 + 51;
  25 unit and 5 integration tests added, none deleted). rustfmt and
  clippy are still absent on this machine (KI-007) and were not run.
- The AC's literal checks, exercised: a malformed skill is skipped by
  name while the others load (unit test `a_malformed_skill_is_skipped_by_name_and_the_others_load`,
  plus a real-binary probe printing `fleet: skills: skipping \`broken-one\`
  — the file does not open with a \`---\` frontmatter fence` beside three
  loaded cards); a selected skill's body enters context as the tool
  result and the next request reads exactly those bytes (integration
  `a_selected_skills_body_rides_the_tool_result_path`, which also
  asserts the system prompt carries name+description and none of the
  body, the five declarations, and the single `skills` record replaying
  to no message); an unknown name is a typed error result
  (`an_unknown_skill_is_a_typed_error_result`); `fleet skills` renders
  cards (`the_skills_command_lists_cards_proposals_and_a_draft`).
- The self-authoring half end to end: a scripted session whose `bash`
  call failed and was corrected drafts `bash-recovery.md` with
  `proposed: true`, `kind: create`, the session id and both record
  indices cited — and the live tree holds exactly the one skill it held
  before (`a_sessions_failure_then_correction_drafts_a_proposal`).
- Curation end to end: four skills, a two-session window from
  `config.json`, one used pair and one unused — a retire draft for the
  unused skill and a merge draft for the near-identical pair (0.86),
  with the absorbed twin getting no retire draft and the live tree
  unchanged (`the_usage_scan_curation_proposes_retirement_and_merge`).
- UX exercised by hand against the real binary on a scratch
  `FLEET_HOME`: `fleet skills` cards, `--curate` filing three proposals,
  `--proposed` listing them with kind and evidence, `--show` printing a
  draft verbatim, and the skip line for a malformed skill.

## Review notes

- The usage scan counts **approved** results: a denied `use_skill` call
  loaded nothing, so it is not a use. The join is the assistant call's
  id against the `tool_result`'s `approved` flag.
- A `chat` session writes one `skills` record per run (per message), by
  the spec's "one record per run" — a resumed log therefore carries one
  per run it made, which is what each run's prompt actually was.
- The drafter's evidence for the failed-turn lesson is the join on
  `input_hash`; that required two additive `#[serde(default)]` fields on
  `calib::ReadRow` (`input_hash`, `reason`), which no existing reader
  breaks on.

## Not done

- Model-written proposal prose (KI-052): the bodies are evidence-shaped
  templates, reviewed deterministically.
- Evidence accumulation onto an existing draft (KI-053).
- Pair-named merge drafts (KI-054).
- Slice 12's gate itself: nothing here applies, and the proposals wait.
