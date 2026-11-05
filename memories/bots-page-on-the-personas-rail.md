---
name: bots-page-on-the-personas-rail
description: "owner direction 2026-10-29 — the bots page takes the same rail as settings, grouping SESSIONS BY PERSONA with what needs the owner (needs-approval) and what ended its turn (handed back); the pane opens the conversation with details + decisions + full history — all from the EXISTING replay read; plan `draft`, unbuilt; the channels read carries NO conversation while the sessions read already carries personas"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner direction 2026-10-29, verbatim: *"we can probably use the same pattern on
the bots page, use the rail to group sessions by persona with pending action
from the users or messages related to an end of turn."* Plan:
`docs/plans/2026-10-29-bots-rail.md` (`draft`, uncommitted, NOT built).

## The finding that shaped it: the page's data does not carry conversations

The bots surface today is **channel plumbing** — service card (start/stop/
heartbeat), pairing card, Telegram token, peers, pending peers, an activity
line. Read live: `/api/channels` returns only `activity`, `activity_bound`,
`channels[{name, peers, pending}]`, `read_at`, `service`, `telegram`,
`waiting_seats`. **No session, no persona, no message.**

But the axis already exists one read over: `/api/sessions` rows carry
`personas` (lead first) and `channel`, and the two states the owner named are
already derived by `work_state` — **`needs-approval`** ("pending action from
the user") and **`handed back`** ("a message related to an end of turn"). On
this machine at planning time: 5 persona-bearing sessions (tower ×5, auditor
×1), **0 channel sessions**, 11 `handed back` rows in the archive. So the
shape is real and the population is not yet — a plan built on "the channels
feed already lists conversations" would have been wrong in its first line.

Personas are a REGISTRY, not a list to copy: `roles/` in the repo and
`~/.fleet/roles/` (six — `tower`, `clerk`, `auditor`, `scholar`, `presenter`,
`advocate`), read by `fleet roles` and the delegation path. A seventh role
must appear in the rail with no code change.

## The design

**rail of personas → that persona's conversations → the conversation**:
persona rows (derived, each with its own mark, badge = its needs-you +
turned-back count, no badge at zero), then `unassigned` (no-persona sessions,
named not hidden), then `channels` (the plumbing as ONE section, not the whole
page). A persona's section orders **needs you** (inline approve/deny — only
destructive acts hold, per [[one-goal-many-personas-destructive-is-the-only-gate]])
→ **turned back** (the clipped last message + age, the end-of-turn signal) →
**recent** (row budget when long). A row's press opens the conversation in the
right pane (tabs stood down), `#/bots/<persona>/<session>`.

Bridge: `/api/channels` grows `personas`/`unassigned` derived from
`roles::load_all` + the same session summaries the archive uses, the last line
via the EXISTING bounded tail read (16 KiB), capped at 50 with the bound on
the document — so the page stops reading 200 session rows to show 20.

**This is the bot-PWA ruling applied to the console surface**
([[bot-pwa-is-assistant-centric-not-work-centric]]: personas are the axis,
assistant-centric not work-centric) — which is where the idiom came from, not
a new design.

## The pane carries the whole conversation: details, decisions, history

The owner's next amendment (2026-10-29), verbatim: *"the right side pane
could carry the entire conversation details, history and action items or
decisions."* Checked against a REAL persona session (24 cards) before writing
the plan, and **all three parts were already in reads the face makes** — so
nothing had to be invented, which is the finding worth keeping:

- **Details** — the replay's own head (`/api/session?id=`): personas and join
  order, serving model, created/ended, turns, workspace, state word.
- **Decisions** — the run's OWN records, filterable from the same cards: the
  pending **seat** (from the state doc, approve/deny), the standing **`goal`**
  cards, the **`memory`** cards (facts it decided to remember — literally
  decisions, each opening in the memory panel), and the ending ask when
  `stop_reason` is a handback.
- **History** — the entire `cards` list, newest first, **50 cards a page with
  "load earlier"**, tool rows collapsed to one line (the transcript's idiom),
  and the replay's `omitted` count named when the read cut. A 200-card session
  reads as a column, not a wall — the same height discipline the settings
  revamp applies.

The card became **B2-C1…C4** (open/close/route, details, decisions, history).
**Deliberately out of scope and named: model-extracted to-dos** — pulling
action items out of prose is the extraction leg's job
([[extraction-leg-repointed-to-sys1]] / sys1-extraction E3), and putting an
unverifiable model summary beside verified rows is exactly what this project
refuses; if the owner wants it later it arrives as its own increment carrying
its evidence.

## The cross-surface constraint

It **consumes** `sectionRail()` from the settings plan rather than copying the
rail; the settings plan's R0 was amended to extract it for exactly this. Build
order: settings R0 first, then bots B0 — and at the pause the bots side had
only its `<nav class="section-rail" id="bots-rail">` container written, nothing
else.

**The 2026-10-30 pause is the design-language lesson for this surface too:** the
owner said *"remember to keep consistency on the design language of the app for
both surfaces"*, and measuring the new rail's row against the app's own
`.surfaces-link` found it drifted on every metric (height, gap, padding, radius,
type weight, icon box, motion) and had swapped the accent-bar current mark for a
wash. A second surface is exactly where a copied rail becomes a second visual
language — which is why the helper and the SHARED CSS declaration are both the
deliverable, not either one alone
([[fleet-settings-abstraction-search]], [[a-relocated-child-loses-the-ancestor-that-styled-it]]). Out of scope and worth keeping: no
conversation state machine (the archive's states are the truth), no
`unassigned` backfill, and **no unread counts** — the archive has no
read/unread notion, so "needs you" and "turned back" are the only two honest
groups and inventing a third would be a state the log cannot support.
