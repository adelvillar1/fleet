# Session recap — 2026-11-04: huddles, the room is a knowledge space

**Summary.** The huddle feature was implemented whole — 17 of 17 criteria
ticked, plan completed. A huddle is a room with members (you + bots), a
goal, mentions that route, per-speaker scope, a capability-gap card that
can convene a role or hire a purpose-built persona, breakouts, and a
knowledge section. The live proof caught two execution-gate defects that
the declarations alone had hidden.

## What was built

- **The room** (`#/huddles`): a rail with the three states (active,
  inactive, ended), member marks, goals, outcome counts; a head with the
  goal, the member stack and each bot's voice, the phases, the knowledge
  line, and a break-out action.
- **Mentions** (C3): `@name` writes a `Mention` record (member + kind:
  directive | ask | review); the next turn is authored by that member —
  the `Assistant.persona` and the role overlay both follow. `@you` asks
  and ends the run. A mention of a name the room cannot receive is a
  **gap**, not a refusal.
- **Scope** (C4): a role's `tools:` narrows its speaker's work tools; its
  assigned skills lead the declaration under the 60-skill cap.
- **Capability gaps** (C5): a bot can name a capability the room lacks —
  a role, a tool, a skill, or a need it describes. A role the home
  keeps convenes at once; anything else offers to hire, and the hire is
  a catalog search first (bring up, don't re-author).
- **Hiring** (C5): `author_persona` is declared on every run that has a
  home; it writes the role through the loader's grammar, joins it, and
  it speaks next. Engagements are chosen: standing, or temporal — a
  temporal hire leaves when the room is archived, its role file staying
  for the next engagement.
- **Breakouts** (C9): a room breaks a phase out; the child names its
  parent on its start record, inherits the membership, and carries its
  own phase brief or the parent's standing goal.
- **Knowledge** (C15–C17): the room shows what it **recalled** (the facts
  that rode its runs) and what it **wrote** (the facts its members
  committed), derived from the log; a "distil what we learned" action
  starts a run whose brief turns the room's lessons into facts and skill
  proposals; the rail's outcome count widens to files plus knowledge.
- **Delivery** (C6): `read_document` and `send_file` are declared on
  every run, so a scoped bot can write a file and hand it over.

## The live proof found two defects

Driven on a scratch home (the machine's credentials in its environment,
no secret on disk, the home removed after), the seven steps were
observed: create, convene, a mention routing a turn, a recommendation,
a deliverable, a hire, a breakout, end.

- **KI-410, first half** — no role admitted `send_file`, so a scoped bot
  could write a file and had no way to hand it over. Fixed: delivery is
  declared on every run.
- **KI-410, second half** — after the declaration fix, the live run
  showed the call **refused at execution**: "send_file is outside this
  subagent's admitted scope". Fixed: the delivery tools and the room's
  escape hatch are exempt from `scope_refusal`.
- **The hire, same pass** — `author_persona` was declared but refused as
  an unknown tool, because `prepare_call` resolves only the base-tool
  table. Fixed: it rides the serial lane.

**Lesson:** a declaration is not an execution. The first fix made the
tool visible and the live proof showed it unusable; the harness cannot
see a scope refusal, so only the live path proves it.

## Verification

- **Gate:** fmt 0 · clippy `-D warnings` 0 · release build 0 ·
  `cargo test` 0 (**2,032 passed / 0 failed**, 16 targets) · harness
  **775 checks / 82 cases**.
- **Plan:** 17 of 17 criteria ticked; plan_check clean; the ledger
  **409 rows / 15 open**, KI-410 closed with its evidence.

## Notes

- The owner's three corrections shaped the design: **cost is not a
  concern** (token plans; the heavy judgment rides the lanes) — the
  cost-per-room suggestion was dropped; **huddles are knowledge spaces**,
  not just deliverable producers; **hires are a catalog**, and temporal
  engagements are legitimate because a hire is only storage.
- The sys1 destructiveness verdict was observed live as the **named
  fallback** ("standing class") — the lane could not answer in the
  scratch home, which is the degradation path working as designed.
