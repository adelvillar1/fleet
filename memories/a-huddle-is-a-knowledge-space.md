---
name: a-huddle-is-a-knowledge-space
description: "owner ruling 2026-11-04 — a huddle brings lessons learned and process improvements, not only deliverables; it is a knowledge space, and the next room should inherit what it taught. Landed as the room's knowledge section and a distil action (C15–C17)"
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

> *"1) I would expect that a huddle would not only bring deliverables, but
> lessons learned, process improvements, etc. a huddle is a knowledge
> space."*

The owner said this after I had built the hiring flow and was treating a
huddle's value as its **outputs** — files, and the count of them. A room is
also where knowledge is *produced*, and the next room should inherit it.

**The design that landed (e407c75), and why it needs no new store.** Fleet
already has exactly two knowledge substrates, and they map cleanly onto the
two kinds of lesson:

| kind of lesson | Fleet's home | how it is recalled |
|---|---|---|
| a fact about the world ("the launch page's hero needs a decision") | a memory fact (`memories/<name>.md`) | ranked by meaning into the next relevant run |
| a better way of working ("when reviewing contrast, do X") | a skill / skill proposal | declared to the model, loaded on demand |

So the room is the **occasion** for distilling into Fleet's existing memory,
and the **context** that gets the right knowledge recalled — not a new
artifact type.

**What landed:**
- **C15** — the room shows what it **recalled** (the facts that rode its
  runs, off the log's `memory` records) and what it **wrote** (the approved
  `remember` results). A refused write is not a lesson — the derivation
  checks `approved`, pinned by `a_rooms_knowledge_comes_off_its_own_log`.
- **C16** — a **"distil what we learned"** action starts a run whose brief
  turns the room's lessons into facts and skill proposals through the
  existing writers; the owner reviews the proposals as now.
- **C17** — the rail's outcome count widened to files **plus** the knowledge
  the room wrote. The other outcome kinds (decisions asked, reviews returned,
  personas recommended, gaps escalated) are derivable from the log and follow
  the same seam — that half is still open.

**Why:** the owner is describing the room as an occasion for distilling into
Fleet's existing memory, and the context that gets the right knowledge
recalled. I had been counting deliverables, which is one kind of output and
not the point. **How to apply:** when the owner says a surface is a knowledge
space, look for the existing substrate that already stores that kind of
knowledge before proposing a new one — here memory facts and skills, both
with writers and recall already built. Related:
[[hires-are-a-catalog-with-engagements]] (the catalog and the engagement that
preceded this), [[delivery-is-not-behind-the-scope]] (the outcomes half still
open), [[huddle-plan-implemented-15-of-17]] (the plan's state).
