---
name: one-goal-many-personas-destructive-is-the-only-gate
description: "owner rulings 2026-10-28 — a conversation may hold more than one persona, so it carries a GOAL (from the owner's words, pinned in the chrome, never changed by a join); non-destructive work runs without asking, only DESTRUCTIVE acts are held and flagged with a visual cue and no always-allow"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner rulings 2026-10-28, mid-review of the B0 mockups (see
[[bot-pwa-is-assistant-centric-not-work-centric]]), in three messages:
**"we also need to allow inter-persona conversations, non-destructive
actions should be allowed as long as the original goal of the
conversation is preserved, so that would probably mean a conversation
where more than one persona is added"**, then **"destructive are held for
approval by the user and flagged as destructive with a visual cue"**, then
**"non-destructive should be allowed."**

**The rule is binary, and everything else follows from it.**

- **Non-destructive work runs without asking** — reads, searches,
  appends, edits with a diff. The mockup shows the proof, not the claim: a
  `write_file … +6 · appended` completes in the transcript with no seat.
- **Destructive work is held every time and flagged** — an overwrite is
  the same file and the same goal and still stops, because it discards
  what is there. The flag is visual before it is textual: red hairline,
  red `⚠ destructive` badge, the exact lines that will go, a filled red
  action, and **no always-allow** (a standing permission for an
  irreversible act is a footgun). The card's note says the append above
  did not ask, so the contrast is visible rather than explained.
- **Sending outward counts as destructive** — the triage strip holds a CV
  about to be sent, because a sent CV cannot be unsent. The strip holds
  only destructive or failed things; non-destructive work never reaches
  the owner's attention.

**A multi-persona conversation needs something that is not a persona to
be about: the goal.** Taken from the owner's own first instruction,
pinned between the header and the stream where it cannot scroll away
(chrome, not content — a sticky band inside a padded scroller never
covers cleanly). Adding a persona restates the goal and never edits it; a
new goal is a new conversation. **A join is a row in the log** (*auditor
joined · checks what changed*, with a time) — not a silent change of
voice, because anything the model saw must be reconstructable. The
transcript then carries two bylines, the header shows overlapping marks
plus a `+`, and the working chip names who is working.

**What the backend needed — built 2026-10-28 as `2976098`.** **P2** is
`goal` + `personas` on the session (a set; first = lead) plus the join
record in the log — landed as `create_with_persona`, `Join`/`Leave`
records, and the summary's folded set. **P3** is landed too, with a
decision worth keeping: `write_file` joined the gate's destructive class
(its ordinary work is an overwrite), superseding the old "only bash has
reach" pin — see
[[a-dated-ruling-supersedes-a-pinned-tests-reason]] — and the seat
document carries `destructive` as data on both delivery paths, so the
face's red cue is the gate's decision, never the client's guess. The bot
runs `mode: guarded`, which IS the binary rule: non-destructive approves,
destructive falls to the seat. Two of the open questions were answered 2026-10-28 in `9b4ef7b`
([[bot-pwa-followups-9b4ef7b]]): the goal change is the owner's explicit
recorded verb — `/api/goal` admitted on the remote face, the bot's goal
line tappable, a join still unable to touch it — and a shared
conversation files under its lead while pre-P2 sessions read
*unassigned*. Per-turn authorship also landed there: the agent stamps
every Assistant record with the lead, so the transcript bylines from the
log. `bash` destructiveness remains per tool and argument shape with a
conservative default — a command's reversibility cannot be answered
statically. Plan shape:
[[bot-pwa-plan-rewritten-2026-10-28]]; the landing:
[[bot-pwa-implemented-2976098]].

**Why:** the owner's autonomy contract is per-action reversibility, not
per-action certainty — asking about everything trains blanket approval,
and asking about nothing loses work. **How to apply:** classify every
agent action as destructive or not before deciding it needs a seat; show
destruction with a distinct visual language rather than a sentence; and
when a surface can hold several actors, give the session a goal field,
because a conversation that can change shape without a fixed subject has
no invariant left to preserve. Related:
[[bot-pwa-persona-roster-six]], [[bot-pwa-b0-mockups-built-2026-10-28]],
[[mockup-review-matrix-is-a-tool]].
