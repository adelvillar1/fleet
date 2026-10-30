---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: dynamic-conversation-names
---

# Plan: dynamic conversation names — the work the conversation is doing now, not the words it started with

## Context

A conversation's name today is its first message line (the owner's
explicit rename aside) — a snapshot of turn one that ages badly: the
session that started "fix the login bug" and spent its afternoon on the
auth refactor still reads as the login bug on the work rail, the
sessions list, and the landing's "pick up where you left off". The
owner's ask: *"the name of a conversation would be dynamic and based on
the last plan or deliverables that were being done"* — cognitive load
drops when the name says what the work **is**.

The deep dive (2026-10-30) found the sources already in the substrate —
no model call, no new store:

- **The owner's title** (`sessions-titles.json`, the pencil) — an
  explicit rename is the truth and must never be overridden.
- **The standing goal** (the goals sidecar, `session_goal`) — what the
  conversation is *for*, set and re-set by the owner through the
  composer's `+goal`; it moves as the work moves. A goal set before the
  first run already migrates to the real session (`move_pending_goal`).
- **The last plan's line** — a plan-mode run appends `Record::Plan {
  text }` to its log: the plan the owner edits and builds. `summarize`
  already walks the whole log once; the last plan's first meaningful
  line rides that pass for free.
- **`first_text`** — today's fallback, demoted to last.

The ladder is dynamic by construction: the goal and the plan rungs
change when the work changes, and every surface — the work rail, the
sessions list, the landing's recent rows, the conversation header —
reads the same derived `title` field, so no surface can disagree about
what a conversation is called.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the ask.** *"if the name of a conversation would be
   dynamic and based on the last plan or deliverables that were being
   done, that would reduce cognitive load when looking at the main page
   and the sessions list."*

## Approach

**One ladder, derived server-side, in the field every surface already
reads.** The sessions read's `title` becomes:

1. the owner's title, when one is set — never overridden;
2. else the standing goal, when the sidecar carries one;
3. else the last plan's first meaningful line, when the log carries a
   plan record;
4. else `first_text`, as today.

The row also carries **`title_from`** — `owner | goal | plan | first`
— so a surface can say where a name came from (the honesty rule: a
derived name is marked as derived, and the conversation header's
tooltip can name the rung). The derivation is pure and cached with the
read: the goal's sidecar is already read per row, the plan rides
`summarize`'s existing pass, and `first_text` is in hand.

**The last-user-message alternative was considered and rejected**: it
changes every turn, so a name would flicker mid-conversation and
punctuation-heavy asks would title sessions badly. The goal and the
plan are the owner's own declarations of what the work is — steadier
and better worded.

**The launch-session fix rides it**: the face's own launch sessions
read as bare ids today (their logs carry no words yet); a session whose
every rung is empty keeps its readable "a conversation · <when>"
fallback from the landing, and a goal set pre-launch names it the
moment the pending goal migrates.

High-level steps:

1. **D0 — the summary's plan rung.** `summarize` carries `last_plan`
   (the last `Record::Plan`'s first meaningful line), one more find_map
   in the existing pass.
2. **D1 — the ladder.** The sessions read's `title` derives through the
   four rungs, `title_from` rides beside it, and the landing's recent
   rows + the conversation header inherit the field they already read.
3. **D2 — the proof.** The fixture pin (a log with plan and goal → the
   right rung), the harness case (the rail row, the landing's recent,
   and the header carrying the dynamic name), the docs.

## Use cases

- [ ] A: A conversation whose work moved reads by its standing goal on the rail, the sessions list, and the landing.
- [ ] B: A plan-mode session names itself by the plan the owner edited.
- [ ] C: The owner's explicit rename always wins, at every rung.
- [ ] D: A surface can say where a conversation's name came from.

## Acceptance criteria

- [x] **D1 — the summary carries the plan rung.** `summarize` returns
      `last_plan` — the last `Record::Plan`'s first meaningful line —
      from a written fixture log; a log with no plan record answers
      `None`; pinned by a unit test.
- [x] **D2 — the ladder orders the rungs.** The sessions read's `title`
      resolves owner → goal → plan → first; a fixture carrying all four
      answers the owner's; removing each in turn walks the ladder down;
      `title_from` names the rung that served. Pinned by a test.
- [x] **D3 — the owner's rename is never overridden.** A session with a
      stored title and a standing goal reads the stored title, and the
      pencil's write still wins after a goal lands.
- [x] **D4 — the surfaces inherit one field.** The work rail's rows,
      the landing's recent rows, and the conversation header render the
      derived title without per-surface re-derivation; a harness case
      over a home/sessions fixture with a goal-named session pins the
      rail row, the recent row, and the absence of the first-words
      fallback.
- [x] **D5 — the plan rung is the plan's own words.** A plan record's
      name is its first meaningful line (heading or sentence), clipped
      at the surface's own bound — not the whole document.
- [x] **D6 — the launch session names itself.** A session whose only
      identity was a bare id picks its goal rung the moment the pending
      goal migrates; the landing's fallback ("a conversation · <when>")
      stays for the empty-everything case.
- [x] **D7 — every button works.** A harness check per touched
      interaction — the rail row's label, the recent row's label, the
      header's name — each asserted against its named element, in the
      same commit as the element.
- [x] **D8 — the gate.** The four commands run and each exit code is
      read directly — `cargo fmt --all --check`, `cargo clippy
      --all-targets -- -D warnings`, `cargo build --release`,
      `cargo test` — plus `node tests/face/harness.mjs`; all green, and
      the harness count quoted in the commit.

## Landed 2026-10-30

D1: `the_summary_carries_the_last_plan` — two plan records answer the
LAST one's first meaningful line (headings stripped), a plan-less log
answers None. D2–D3: `the_title_ladder_orders_the_rungs` and
`the_owners_rename_wins_after_a_goal_lands` over the extracted pure fn
— the read calls `title_ladder(owner, goal, plan, first)` and carries
`title_from` beside the name; the bot inbox's goal field reuses the
once-read value. D4–D7: the face harness, **474 checks across 51
cases** — the NAME case pins the rail's three rungs (goal, plan,
owner) and the header's tooltip ("named by the standing goal"). D6:
the launch session's pending goal names it on migration, and the
landing's "a conversation · <when>" fallback stands for the
empty-everything case. D8: the gate whole — fmt 0, clippy 0, build 0,
the suite green — and the harness count quoted in the commit.

## Files to be touched

**Backend / API:**
- `src/session.rs` — `summarize` carries `last_plan`; the summary struct grows
- `src/gui/bridge.rs` — the title ladder in the sessions read; `title_from` beside it; the unit pins

**Frontend:**
- `src/gui/assets/app.js` — the recent rows' and the header's tooltip reading `title_from` (the rail rows inherit the field)

**Tests:**
- `tests/face/harness.mjs` — the dynamic-name case
- `src/session.rs` tests — the plan rung; `src/gui/bridge.rs` tests — the ladder

**Documentation (see Linked artifacts below):**
- `docs/features/gui.md` — the naming ladder
- `FUNCTIONAL-SPECIFICATIONS.md` — the sessions passage

## Out of scope

- **A model-named session** — the ladder is the owner's declarations,
  not a generated summary; a naming call is a named follow-up if the
  rungs prove too coarse.
- **Renaming by the last user message** — rejected in the Approach;
  too volatile to name by.
- **Retitling past logs** — the ladder derives at read time; nothing
  rewrites history.
- **The titles sidecar's shape** — unchanged; the pencil keeps its
  store.

## Verification

**D1–D3** — the fixture pins:
```bash
cargo test the_summary_carries_the_last_plan
cargo test the_title_ladder_orders_the_rungs
```
**D4–D8** — the face harness and the gate:
```bash
node tests/face/harness.mjs
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: every new check green; the count line quoted in the commit.

## Linked artifacts

- `docs/features/gui.md` — the naming ladder beside the work list's passage
- `FUNCTIONAL-SPECIFICATIONS.md` § "The commands as verbs" — the sessions list's naming

## Risks

- **A goal written for one task lingers into the next.** The goal is
  the owner's own declaration and the `+goal` chip re-sets it in one
  press; `title_from` makes the rung visible so a stale goal is
  diagnosable, not mysterious.
- **Plan texts are long.** The rung clips at the read's own bound
  (first meaningful line), never ships the document.

## Dependencies

- The goals sidecar and the `+goal` chip — live.
- The plan record (`Record::Plan`) — plan-mode runs append it.

## Notes

- The ladder is read-time derivation: no store changes, no migrations,
  no rewrites of the titles sidecar or the logs — the four rungs are
  the sources that already exist, ordered.
- The conversation header reads the title event for renames; the
  ladder's field is the doc's `title`, and the header's tooltip gains
  the rung word so the owner can always see what named a conversation.
