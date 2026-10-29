---
status: draft
created: 2026-10-29
updated: 2026-10-29
slug: conversation-sessions-home
---

# Plan: the conversation home — the work rail is the owner's, and the way back exists

## Context

The owner's report (2026-10-29), verbatim:

> conversation and sessions, they feel somewhat disconnected, also there is
> now easy way to jump through active working sessions, landing on the
> main conversation page is a one and done deal, there is no way of getting
> back to it … also there is a lot of noise on the Work rail with non-user
> initiated sessions showing and polluting the view, system initiated
> sessions should be only available via a log for analysis, never on the
> user screen

What the code and the live archive say:

- **The two work destinations are one stage.** `#/chat` and `#/sessions`
  both render `panel-chat`; the only difference is the work column
  (`showRoute`, `src/gui/assets/app.js:14025` — `work = address ===
  'sessions'`). The rail presents them as two destinations in one group
  (`GROUPS`, app.js:45). The home — greeting, counts, "pick up where you
  left off" — exists only as the no-conversation state (`el.empty`
  hidden whenever the log "talked", app.js:6485), so neither destination
  is whole: "conversation" is a transcript without the list, "sessions"
  is a transcript with the list, and the home belongs to neither.
- **The main conversation page is one-and-done, exactly as the owner
  says.** Nothing returns to the no-conversation state: the rail's
  "conversation" item and Escape both walk to `#/chat`, which with a
  loaded session shows the bare transcript (app.js:15222,
  14092); boot restores the last panel from `fleet-panel`
  (app.js:15227), so after a day in the archive every boot lands there.
  The only door back is **New** — and New mints a log immediately
  (`newConversation`, app.js:8219 → `/api/session/new`), so "getting
  back" costs an empty session in the archive. The archive holds **34**
  of these start-only shells.
- **Active sessions are invisible until you open the surface.** The
  rail's count badge — "a surface that has something waiting says so on
  the rail" (app.js:14781) — is dead structure: the `surfaces-count`
  spans are created with `dataset.count` and no code ever fills them.
  The work list hides at ≤1180 px and folds to a strip; there is no
  other jump verb.
- **The rail carries system work.** Archive census (2026-10-29, 538
  logs): 362 `scheduled`, 41 `subagent`, 135 kind-less — 34 shells, 101
  with text (owner conversations, plus face-harness and manual probes).
  The rail's newest-200 slice: 141 scheduled + 28 subagent + 30
  kind-less. The current design hides scheduled/subagent/channel behind
  a collapsed "background · 169 runs · hidden" line (paintWork,
  app.js:7904) — but the line itself is permanent noise, and the
  needs/working/archived groups filter by state, not class
  (app.js:7880), so a system run in one of those states leaks onto the
  user screen. All 362 scheduled logs carry the kind stamp (verified —
  no backfill needed); the classification is complete, the render is
  not.

The owner's ruling settles the design: system-initiated sessions are
available via a log for analysis — `fleet cron` lists every job and run,
`fleet sessions` ranks the archive, the calibration stream carries the
rows — and never on the user screen.

## Approach

Four increments. N0 and N1 are the owner's words made structure; N2 and
N3 close the seams the diagnosis named.

### N0 — the work rail renders the owner's work only

The rail renders rows of class `yours` and nothing else. The
background/delegated/channel groups and their toggle leave `paintWork`
entirely; the needs/working/archived groups gain the class filter so no
state can leak a system row. `/api/sessions` keeps deriving `class` for
every row (the document is the archive's truth; the CLI surfaces read
it) — the change is the face's render, not the data. The head count
speaks the owner's work in the same words (`counts.yours`, already
derived — bridge.rs:5094).

### N1 — the home, always reachable

The welcome becomes a state of the conversation surface, not the
absence of one. The rail's "conversation" item and Escape land on the
home: greeting, counts, recent — with the current conversation one
click away in the recent list ("this face · id", already marked).
Detaching from a conversation clears the face's pointer
(`face.session = null`, welcome shows, log hidden) and touches nothing
server-side: the bridge's launch session continues, rejoining is one
click. The conv head's word stays "work" — the list is the way between
conversations, the rail is the way home.

### N2 — New mints nothing until words exist

The home composer composes into a pending conversation; the session is
created at the first send (create-then-send in one turn), so an
abandoned compose leaves no shell. The pending draft lives under its
own storage key until it has a session to own it. The rail skips rows
with no user text and no kind — they are not conversations; the CLI
archive still lists them. The 34 existing shells stop being rendered
and stop being created.

### N3 — the rail says where the work is

The count badges come alive: "sessions" carries the needs + working
counts when either is non-zero (the spans and `dataset.count` exist;
the painter is the missing half), and the badge is the click-through.
A run in flight elsewhere is visible from every surface, which is the
"jump through active working sessions" verb at the rail's own level:
see it waiting, click it, land in it.

## Acceptance criteria

- **N0.** With the real archive, the work rail renders only class-yours
  rows in every group; no toggle, no background line, no scheduled or
  subagent or channel row reachable on the user screen; the head count
  says the owner's count. Pinned by a face-harness check against a home
  seeded with scheduled, subagent and channel rows.
- **N1.** From any loaded conversation, one press — the rail's
  "conversation" item or Escape — shows the home (greeting, counts,
  recent); the recent list re-enters the conversation that was open;
  the bridge's launch session is unchanged by the round trip. Pinned by
  harness checks: home → conversation → home → same conversation.
- **N2.** Composing on the home and sending creates exactly one
  session; abandoning a compose creates none; no shell rows render.
  The draft survives a reload while pending. Pinned by harness checks
  and by the archive's own counts across the run.
- **N3.** A session needing approval or working shows a count on the
  rail's sessions badge from every other surface; clicking it opens the
  work surface. Pinned by a harness check.

## Files to be touched

- `src/gui/assets/app.js` — paintWork's groups and filters, the home
  state and the detach verb, the pending-composer send path, the badge
  painter
- `src/gui/bridge.rs` — nothing structural: `/api/sessions` already
  derives class and counts; only if N2's create-then-send needs a
  combined route does the bridge grow one
- `tests/face/harness.mjs` — the four checks above
- `docs/features/gui.md` — the work-list section and the surface map
- `docs/known-issues.md` — the defects this plan files (the dead
  badges, the one-and-done home, the unfiltered groups, the shells)

## Out of scope

- **The harness/probe sessions in the owner's archive** (the ~59
  kind-less verification runs): they are owner-initiated as far as the
  log can say, and hiding them is a stamping decision (a `channel` mark
  on verification traffic) that belongs to its own plan with the
  harness's read paths re-verified. Named follow-up, not silent scope.
- **A system-run analysis surface in the face.** The ruling names the
  log (`fleet cron`, `fleet sessions`); the face stays clean until the
  owner asks for a door.
- **Deleting the 34 existing shells.** The rail stops rendering them;
  the archive keeps them, the daily maintenance's naming rule governs
  their retirement.
- **The bot face.** It renders the server's words verbatim and shares
  the parsers; a rail change there is its own slice if the owner wants
  one.

## Verification

The gate runs whole and exit codes are read directly. The four harness
checks are the proof; the live face gets one manual pass (home → work →
home; the rail with today's archive; a compose abandoned) before the
plan's status flips.
