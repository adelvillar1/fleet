# Session Recap — 2026-11-08: the learning loop

**Summary.** The learning loop landed whole as a first-class work
surface, in its own worktree off the trunk: the lesson session kind,
the prober and the probe loop, the minimal tree, the teaching loop, the
knowledge graph and its gated extraction, the practice, the face's
Learning rail section and pane, the feeds into the demand map and the
skill drafter, and the evidence discipline. The plan is completed and
every criterion is ticked; the app is rebuilt, installed, and live on
the new binary. The owner read the mockup and will run the first lesson
later — that live proof is the feature's remaining gate.

## Plans worked on

### `docs/plans/2026-11-08-learning-loop.md` — the learning loop

Status after this session: **completed** (every box ticked; the
pre-commit plan check passes).

| Acceptance criterion | Status | Evidence |
|---|---|---|
| C0 — the mockup | ✅ met | `docs/plans/mockups/2026-11-08-learning/` measured in four themes (contrast 13.4–18.1, no overflow), read in a light and a dark theme, and read by the owner 2026-11-08 |
| C1 — the lesson session kind | ✅ met | `Start.kind` carries `teaching`/`practice`, `Session::create_lesson`, `Session::list_kind`, `fleet sessions --kind`, the face's sessions read `?kind=`; `a_lesson_log_carries_its_kind_and_the_list_filters_by_it` |
| C2 — the prober and the probe loop | ✅ met | `seed_prober` (never overwriting the owner's file), the `ProbeConversation` seam, `record_probe` landing the answers and the bracket as `probe` facts plus one `learn_probe` row; the lesson tests pin the loop and the records |
| C3 — the plan | ✅ met | `graph::LessonNode`/`LessonPlan`/`build_lesson_plan`, the critical path as the teaching order (foundation first, the spine leading), the Mermaid coarse view, the `lesson-plan` fact; seven graph tests |
| C4 — the teaching loop | ✅ met | the teacher role, `teach` (foundations never re-taught, the repairs bounded, an unrepaired confusion stops the lesson, the learner's own "obviously true" the only terminal condition), `record_lesson`'s `check` facts |
| C5 — the knowledge graph | ✅ met | the `premises` field through `Remember::parse` (a premise that is not a proposition name is refused), the loader, the edit, the supersede's inherit-or-clear; `record_proposition`; the extraction pipeline's 80% uncovered-token gate and the deterministic merge; the memory and lesson round-trip tests |
| C6 — the practice | ✅ met | the practitioner role, `practice` (one problem per turn, a setting that covers the claim refused by name), `record_practice`'s `practice` facts |
| C7 — the face surface | ✅ met | the Learning rail group and the pane's four sections, the bridge's one learning document, the learning start's seam; the harness case pins the rail, the nodes' states, the position, the holds, the chains, the outcomes, the history, and both entries' posts |
| C8 — the feeds | ✅ met | the `learn_check` row and the demand map's `learn` family; the clustering beat files the repeating confusion into the improve store; the drafter files a skill draft over a lesson's repeated procedure |
| C9 — the evidence discipline | ✅ met | the graph's record carries no confidence, mastery, score, or held line (pinned on the file's own text); the session's records carry the held and the answer; the probe's carry the thinking and the confidence |
| C10 — the gate | ✅ met | fmt, clippy, release build, test — exit 0 each; 2,024 lib tests, the integration suites (the lanes tests green under `BROWSER_CHROME`), the face harness 998 checks / 117 cases, `plan_check.py` exit 0 |
| C11 — the docs | ✅ met | `docs/features/learning.md`, the surfaces doc's rail inventory and section, the memory doc's `premises` field and proposition type, the README's pointer |

## The live proofs

- **The CLI's kind filter.** A scratch home with a teaching session and
  an ordinary one: `fleet sessions --kind teaching` lists the teaching
  session, `--kind practice` none, the default both.
- **The learning start's seam.** The bridge test pins that a learning
  start seeds the prober into a home that lacks it, leaves an
  owner-edited file alone, and sends the command carrying the lesson
  kind and the role.
- **The face's read.** The delivered face answers `/api/learning` with
  the document — the honest empty state (no lessons yet) the owner's
  first lesson will fill.

## Delivery

- The branch `learning-loop` was merged to master by a fast-forward
  (`29daca3..27a737e`), carrying master's own hybrid-retrieval work
  with it; the definition bound moved to 10,800 for both crossings
  (the `hybrid_search` tool and the learning record's `premises`), and
  the tier test's synthetic core was made clearly over the bound rather
  than 12 tokens above it.
- The Tauri dmg rebuilt (`fleet_0.4.0-beta.1_aarch64.dmg`, 10.39 MiB)
  and its packaged binary verified to carry the learning surface; the
  old app moved aside to `fleet-previous.app`, not deleted.
- `~/.fleet/bin/fleet` replaced through a new inode (19,702,896 bytes)
  and the service restarted — pid 66125, port 3080 answering 200, the
  served `app.js` carrying the learning surface.

## What remains

The owner's first lesson, run end to end: the probe brackets, the plan
draws, the teaching moves one checked step at a time, the graph
accumulates, and the friction's proposals land in the stores. The
machinery is pinned; the lesson is the owner's.
