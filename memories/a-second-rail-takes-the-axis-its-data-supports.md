---
name: a-second-rail-takes-the-axis-its-data-supports
description: "Health, inbox and board now wear the second rail, each scoped by what its own data supports — checks, classes, and projects respectively — with the corpus and second brain moved to Intelligence because knowledge is not a health check."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [section-rail stage-split second rail, health by checks not projects, "inbox by its five classes, no project scope", board scoped by project with all tasks kept, second brain is a category of intelligence, var used above its declaration hoists undefined]
missing: []
premises: []
recall-count: 55
superseded-by:
valid-until:
source:
---

The second rail (`section-rail` + `stage-split`) is every split page's spine. On 2026-11-17 it existed on seven surfaces and was missing on three: health, inbox and board — all declared as bare `stage-board`, so each read as one scroll rather than a place.

**The axis is decided by the data, not by symmetry.** Each of the three takes the scope its own read supports:
- **health** — its checks (lanes, doctor, build). *Not* projects: those are the machine's, not anyone's work.
- **inbox** — the five classes it already computed (`INBOX_CLASSES`: seats/onboarding/asks/cron/doctor), plus waiting and archived. Those questions lived as a chip strip *inside* the board; the rail is the tab and the filter now, one state with two doors. **No project scope** — inbox rows carry no project (`bridge.rs::inbox_rows` gives class and key only), and inventing one would be a lie the surface tells.
- **board** — the work's project, with an `all tasks` row kept alongside so nothing is reachable only through a filter. Tasks have carried a `project` since `boards.rs`, and no surface offered it as a scope.

**Where content belongs outranks where it sat.** The corpus and the second-brain atlas were stacked on health with the lane probe — "is the machine well" sorted together with "what does it know". The owner ruled the second brain is *a category of intelligence*, so both moved to Intelligence as their own sections, taking the icons `corpus` (three stacked layers) and `second-brain` (the project grid).

**Three traps, each one a real defect:**
1. `var` used above its declaration hoists as `undefined` — the board head rendered `The board · undefined` for one commit. Declare the scope before the head that names it.
2. A repaint guard keyed on the *lens* (`face.panel === 'second-brain'`) paints the wrong surface once a section moves: the health board and the Intelligence board both mounted a `section-body`. The guard must ask which surface currently holds the section.
3. An arrival that always re-reads wipes the document a press just set (`inboxDecide` sets `face.inbox = reply.body`, then the hash change re-fetched the stale copy). A section press changes only the address's second segment — gate the read on the *lens* changing (`face.lastLens`), and derive the tab from the section rather than keeping a parallel `face.inboxTab`.

The icons discipline governs new rails: no two rows share a drawing, none falls through to `dot`. Inventing a key is the failure (`machine` does not exist) — route it through the renderer at 17×17 and read it.
