# Skills the model can actually reach

Status: completed
Date: 2026-10-22

The owner asked how skills are invoked; the answer measured was
"model-driven only, and it has never fired" — 0 used, 442 never selected
across 20 sessions. The owner chose the fix that makes the model-driven
path work. Built against
`docs/plans/2026-10-22-skills-the-model-can-reach.md`, whose voice was
approved in mockups before a line was written.

## What changed

- **The line is a trigger.** Each declared skill rides the prompt as
  `` - `name` — use when <trigger> ``, where the trigger is the
  description with its leading `Use when`/`Use this`/`Use for` stripped,
  its first word lowered, and the whole clipped near 160 characters — a
  match signal, not the catalogue entry, because the body arrives on the
  call. The derivation is a pure function (`skills::trigger`), so a
  library edit shows up in the next run with no migration, and the four
  description shapes plus the clip are unit-pinned.
- **The list is capped.** `SKILLS_PROMPT_CAP` (60) with the library's
  newest by the skill's own mtime floating up; one line per run names
  how many are held back and that `fleet skills` lists them all. The
  prompt and the `skills` record read one selection, so the log pins
  what the model saw. A capped run costs ~2k tokens where 442 entries
  cost ~16k.
- **The owner retires.** A `retire` action on the skills page writes
  `<home>/skills-retired.json` through the one writer; the declared
  list is read **per run**, so a retirement reaches the next run with
  no restart; the library is byte-identical before and after (pinned by
  a test that hashes the SKILL.md), and `unretire` returns it. The
  suggestion is the scan's own number — the `never selected` count
  beside the action on those very rows. KI-215's ruling holds: the
  library is cross-harness, so fleet suggests and the owner retires.
- **Fidelity to the current UI** — the owner's standing constraint:
  the action is a `rowAction` after the row's `copy` with the decisions
  queue's approve as its in-flight and refusal pattern, one more
  `surfaceStrip` entry, a `boardCard` in the existing `surfaceRail`,
  the decisions queue's `rail-row` shape — and no new CSS class.

## The criteria, walked

- **C0** — `skills::trigger` over four shapes (three frames, one with
  none) and the clip; the prompt's section sentence says the trigger is
  the match; the two integration prompt assertions pin the new line.
- **C1** — `the_declared_list_is_the_newest_under_the_cap_and_the_line_names_the_rest`:
  70 skills declare the newest 60, the oldest is held back, the prompt
  carries the trigger line, and the growth line names 70/10.
- **C2** — `a_retirement_leaves_the_library_byte_identical`: the file
  hashed before and after, the declared list empties, the return
  restores it. The routes are `POST /api/skill/retire` and
  `/api/skill/unretire` through `settings::retire_skill`.
- **C3** — `docs/features/skills.md`'s progressive-disclosure section,
  CLAUDE.md's pointer line, and the functional spec's skills surface.
- **C4** — live: the strip shows `1 RETIRED`, the retired section
  renders, and the next run's `skills` record declares 60 skills
  without the retired name — read from the running face, not the unit
  tests alone. The record still carries name+description as it always
  did; the trigger line is the prompt's own derivation, pinned by C0's
  tests.

Gate: 1493 passed, 1 ignored. One mid-build correction: the declared
list was first read once at thread start (the load's own comment), so a
retire only reached a run after a restart — the load moved to per-run,
which is what the comment always claimed.

## What is left

**KI-278**, found while wiring this: a scheduled run and a role's skill
resolution declare no skills at all — `cron.rs` and `subagents.rs` call
`skills::load(home)` with the FLEET_HOME where the loader expects the
canonical library directory. The interactive paths are correct; those
two sites are not, and the fix is named in the row.

The measure that matters is the usage scan: after the trigger lines and
the capped list, the scan's `used` count should leave zero — the strip
already reads `4 USED` from the runs this session's probes drove, but
that is the probe's own asking, not the model reaching for a skill on
its own. The next real sessions will say.

Cache sweep at recap: free space on the volume: 70.1 GiB → 70.1 GiB (release targets untouched)
