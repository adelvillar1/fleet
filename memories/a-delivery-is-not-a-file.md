---
name: a-delivery-is-not-a-file
description: "owner correction 2026-11-04 — a huddle's delivery is whatever the room hands back (a file, a decision requested, a review, a missing persona, a capability gap, a request to discuss), not only a file; I narrowed it to files and called that the whole problem"
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

When the C14 live proof exposed that no role admits `send_file`, I filed KI-410
and offered two fixes — a role names `send_file`, or delivery is declared for
every run — and framed the defect as "the file kind". The owner's reply:

> *"what if the delivery is not a file but a request for a decision, or
> basically saying I cannot deliver this because I don't have the tools or
> skills and I need help, or we need a persona that we don't have, or let's
> discuss in the huddle because I need more information. you are being too
> black or white"*

**Why:** "delivery" is the huddle handing something back to the owner — a file is
one kind. Treating it as the only kind made me miss that four other kinds already
have mechanisms (a decision requested is `@you`; a review is `@role` with kind
`review`; a missing persona is a `@role` that resolves; a discussion request is
`@you` plus prose) and that two have none (a capability gap naming a tool or
skill, and any of them counted as an outcome). It also made the rail's
`deliverables` — which counts approved `send_file` results and nothing else —
look correct when it is really counting one kind.

**How to apply:** when a surface is supposed to show "what the room produced",
enumerate the kinds before naming a counter, and check each kind has a mechanism.
Here the honest shape is a **capability-gap kind** (naming a role *or* a
tool/skill) plus the huddle's **outcomes** as the union — files, decisions asked,
reviews returned, personas recommended, gaps escalated — on the rail row and the
room head. A two-option fix offered for a one-kind reading of a multi-kind
problem is the tell that the reading was too narrow.

Related: [[huddle-plan-implemented-12-of-14]] (C14's run and KI-410),
[[a-card-builder-returns-sections-not-a-node]] (a row that renders fine can still
be unpressable — pin the press).
