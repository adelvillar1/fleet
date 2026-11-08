---
name: the-turn-count-leaves-the-face
description: "The 2026-11-05 owner ruling narrows the turn ask to one thing: the turn number must be invisible on main sessions, caps are out of scope this session, and internal/delegation turn counts may stay."
type: user
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: [rendering, resolved]
phrases: [make invisible the turn count on the main sessions, turn 3 · 4 tools · thought 2s, the context indicator is already implemented, turn count is meaningless to users, paintWorking, summarizeRun]
missing: []
recall-count: 5
superseded-by:
valid-until:
---

Owner ruling, 2026-11-05, stated in the face (narrowing the earlier "turn cues and caps" ask to one item):

"the context indicator is already implemented, what I need is to remove or make invisible the turn count on the main sessions, they are meaningless to the users. if they need to remain in place for internal delegation or any other justifiable reason that's okay. That's all I need from this session"

Decoded scope:
- **IN:** make the turn NUMBER invisible wherever a main (interactive/owner-facing) session renders it. The context gauge already shipped and is not part of this.
- **OUT (this session):** turn caps. The cap the loop applies is a separate, still-deferred question (2026-10-31 plan decision 3); do not fold it into this work.
- **EXPLICITLY PERMITTED to stay:** any turn count with an internal or delegation justification — the child's own occupancy bound (KI-076), checkpoint paths `checkpoints/<session>/<turn>/`, the rewind cut, the log's records, resume, exit code 3 / stop_reason `max-turns`, the archive's `turns` field.

Visible turn-number surfaces found in the docs at master (each quoting its own string):
1. the completed turn's fold mark: `turn 3 · 4 tools · thought 2s` (docs/features/gui.md, FUNCTIONAL-SPECIFICATIONS.md "Run behavior")
2. the working line on the conversation's tail: "a spinner, 'working', the turn, and the elapsed time"
3. the bots page's conversation card details: "persona, model, turns, created, ended, workspace"
Not a turn count and left alone: the header's `exchanges` (owner messages, per gui.md's own definition), and the work-list rows, which carry state/age/project/job and no turn number.
