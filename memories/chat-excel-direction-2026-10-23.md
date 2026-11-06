---
name: chat-excel-direction-2026-10-23
description: "2026-10-23 the owner asked \"what would make the chat and composer excel\"; the thesis, friction inventory, and the plan (nine increments, twelve criteria, audit layer folded in) — then IMPLEMENTED TO COMPLETION the same day: c204791 (goal chip) + ee5b305 (composer/stream packs) + f813b63 (audit layer), plan completed, gate 1436 lib / 1591 total, 215 face checks"
type: project
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

Owner question 2026-10-23, after the MCP registry shipped: *"what would
make this harness a top tier chat experience"* — low friction, no context
switching, depth on demand.

**The thesis:** a top-tier agent chat is a *work surface with a memory*.
One rule covers both halves: everything you need to DECIDE is one glance
away; everything you'd want to AUDIT is one press deeper — never a panel
away. The depth ladder already goes *down*; what was missing is *across*
(file/session/artifact a reply names) and *back* (where you were, what
changed while gone).

**Anti-goals the owner endorsed by silence across two builds:** no
chat-app furniture, no telemetry in modals, no auto-expanding anything,
no floating bubbles over the reading surface ("persistent" means always
present in the chrome — the run bar, per the owner's no-wasted-space
rulings).

**The plan** `docs/plans/2026-10-23-chat-composer-excellence.md` was
drafted (gate PASS), amended once (the owner: "why leave the audit layer
out? I have all the time in the world today and the 500M free tokens" —
an explicit don't-trim-for-size signal), then **implemented to
completion in one session**:

- `c204791` — the goal chip: always present in the run bar (`+ goal`
  when unset — teaches the affordance; one ellipsized line when set),
  popover with Set/Replace/Clear riding `/api/goal`, the same route
  `/goal` rides so command and chip cannot disagree. Rationale that
  carried it: the STALE-GOAL problem — the objective rides every run's
  prompt, and an objective you cannot see is one nobody drops.
- `ee5b305` — increments 2–6: per-conversation drafts
  (`fleet-draft-<id>`, legacy single key migrates to the front door and
  is consumed); edit-and-resend on owner bubbles riding the append-only
  `Bridge::rewind` (rewind to the turn before the run the message
  launched; a first-ever turn edits without rewind; beside a driving
  run the edit queues); paste-to-attach through the picker's own
  client-side pipeline and bounds (the plan first named `/api/attach` —
  corrected mid-build; a pasted image names the document surface rather
  than vanishing); `@` typeahead over the reads the face already makes
  (tree/sessions/skills/memory; Enter inserts per KI-245, a file
  reference opens the pane preview, a session reference leaves a link
  that opens that conversation); the return pack (since-you-were-here
  divider seeded ONCE per conversation per visit — the stream's onopen
  re-replay after the boot's scroll-settle otherwise erases it;
  back-to-live counter; Notification on seat/finish only while
  `document.hidden`); receipt artifacts from the checkpoint window with
  diff peeks + clickable paths (KI-289's transcript half); turn
  navigation (fold per mark, next/prev, expand-run).
- `f813b63` — the audit layer: content search as a LITERAL server-side
  match over the newest 200 logs (`?q=` on /api/sessions, matched line
  riding the row, bound named on the document; semantic refused by the
  1/81 top-1 measurement); **what the model saw** — the turn loop's
  request assembly factored into `composed_prompt` + `base_declarations`
  shared by the loop and `GET /api/context`, pinned by a byte-equality
  test against the CAPTURED mock request bodies (messages and tools
  arrays; the reconstruction trims trailing assistant messages past the
  last user/tool message because the reply is the log's next record,
  not part of what the model saw); **single-tool re-run** through the
  executor and bound, lane calls over the registry/roster one-call seam,
  result in the drawer and NEVER in the session log (pinned), write_file
  refused by name, ask-class calls stand a real seat.

Final: plan `completed`, all twelve criteria ticked, plan-gate PASS
12/12, gate fmt+clippy+release green with 1436 lib / 1591 total passed,
face harness 215 checks across 26 cases. Two refactor regressions the
suite caught in the factored composer are recorded in
[[a-factored-composition-must-keep-the-exact-texts]]. One named
hold-back in the plan: `improve`'s declaration carries the lesson
store's live records — a fact the log cannot hold — so the
reconstruction names its absence.

Related: [[fleet-work-and-the-conversation-revamp]],
[[fleet-conversation-control-shipped]],
[[fleet-text-size-belongs-to-the-reading-surface]],
[[gui-client-verified-by-a-vm-harness]].
