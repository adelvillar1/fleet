# The rail map and drawer bodies — the plan gap ledger

The goal's rule: a gap in the plan is never closed by an educated guess — it is
researched, the replan is written here, matched to the plan row it amends, and
closed by a commit. One row per gap; a gap without a plan row it amends is a
defect in this ledger, not in the code.

Plan: `docs/plans/2026-11-23-rail-map-and-drawer-bodies.md`.

| # | Gap found | Plan row it amends | What the research said | The replan (the decision, recorded) | Closed by |
|---|-----------|--------------------|------------------------|-------------------------------------|-----------|
| G1 | R5 says "each object kind's read body opens with a relevance line and one suggested press". Read against the code, the drawer's object model only carries the FILE object (the tree and the receipt chips open it); a conversation's state already rides its head and its act tab, and facts/deliverables never enter the drawer — their surfaces carry their state. | R5 | `openFile`/`showReadMode` hold the file object and nothing else; the mock's conversation line would repeat the head's state word, and a fact line has no drawer read to sit in. | The line and its press ship for the file object (kind · size · the sibling body one press away). Conversations keep their head + act tab; facts and deliverables keep their own surfaces. No line is invented for an object the drawer does not hold — the study's mock remains the target if those objects later gain a drawer read. | this commit (R4+R5) |
