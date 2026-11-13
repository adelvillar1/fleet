# The vocabulary — what the face may say

**U45's source file.** Every user-facing string in the face resolves through
this list. A label that is not here does not ship. The **expert** column is
what the same thing is called when Settings → Advanced → Expert view is on;
the plain column is the default, and the only register a user who never read
this ledger should ever meet.

Rules:

1. **Plain by default.** The plain word is the product's word. It appears in
   the rail, the page titles, buttons, empty states, toasts, and errors.
2. **Expert is a translation, never a second UI.** Turning expert on swaps
   nouns and reveals numeric detail; it does not reveal different surfaces.
3. **Forbidden means forbidden.** A forbidden term may not appear in any
   user-facing string, including errors, tooltips, aria-labels, and empty
   states. It may appear in logs, code, this repo's docs, and the expert
   register.
4. **One noun per thing.** No synonym sets: if two words name the same
   object, one of them is forbidden.

## The shell

| Plain | Expert | Meaning | Forbidden |
|---|---|---|---|
| workspace | — | everything on this machine, as one thing | dashboard, cockpit, console (for the whole app) |
| lens | — | one view of the workspace (the centre column) | panel, tab, board (as a general term) |
| view | — | a saved set of lens + filters | saved view is fine; "perspective" is not |
| connections | graph, tissue | the column showing what the focused thing touches | tissue, receipts, graph column |
| focused | — | the one thing the workspace is currently about | selection (for focus), pivot target |
| activity | — | what has happened recently | feed, event stream, journal |
| run | session, execution | one time fleet did work | session (plain), turn |
| step | turn | one exchange inside a run | turn (plain) |

## Work

| Plain | Expert | Meaning | Forbidden |
|---|---|---|---|
| work | work item, task, unit | one piece of work and everything it produced | task AND deliverable as separate nouns for the same thing |
| title | — | the one-line name of a work item | label, subject, slug |
| status | — | queued · working · blocked · done | state, phase |
| transcript | thread, session log | the conversation a work item was done in | thread (plain), log |
| artifact | deliverable, output | what a work item produced | deliverable (plain), output doc |
| stage | pipeline stage | where an artifact is in its process | loop stage, bead |
| review | calibration, grade | a decision graded against what happened | calibration (plain), judgment |
| lesson | — | something learned, with its evidence | learning, takeaway |
| brief | — | the starting statement of a work item | claim (plain), prompt |

## Who and what

| Plain | Expert | Meaning | Forbidden |
|---|---|---|---|
| assistant | bot, persona | a named helper work is given to | bot (plain), agent (plain) |
| machine | instance, node | a machine running fleet | instance (plain), node |
| this machine | install, flagship | the machine you are on | local console, flagship (plain) |
| your models | providers, roster | the model services fleet can call | router (plain), lanes |
| what fleet knows | memory, corpus, facts | the knowledge it has and where each came from | corpus (plain), substrate, memory (plain) |
| what fleet can do | capabilities, tools, skills, workflows | the actions it can take | capability registry, tools surface |
| secret | credential, vault entry | a stored key | credential (plain), token, key (as a noun for the stored thing) |
| room | huddle | a shared conversation with several participants | huddle (plain), breakout |

## Actions

| Plain | Expert | Meaning | Forbidden |
|---|---|---|---|
| new work | new brief, create task | start a work item | compose (as the only door) |
| send | dispatch, run | start the work | submit, execute |
| hand to desk | hand off | move a run's result into artifacts | settle (plain) |
| advance | — | move an artifact one stage | promote, progress |
| settle | — | close an artifact, graded | finalize, complete |
| connect | grant, pair | give access to something | authorize (plain) |
| keep | — | save a view | pin (for views) |

## Things the face must not say (plain register)

`sys1` · `blend1` · `lane` · `gate` (as a noun) · `corpus` · `substrate` ·
`principal` · `armada` · `calibration` · `router` · `delegation` · `huddle` ·
`breakout` · `distil` · `seat` · `basis` · `anti-vector` · `provenance` ·
`receipts` · `tissue` · `spine` · `MCP` · `judgment beat` · `workload` ·
`SOUL` · `LayerFS` · `graph log`

Each of these has a plain translation in the tables above, or it belongs
only in Settings → Advanced → Expert view.

## Where the words come from

The seven lenses are named for the question they answer, not the subsystem
they read:

| Lens | The question |
|---|---|
| Threads | What is being worked on right now? |
| Board | What is waiting, moving, or stuck? |
| Desk | What landed, and is it any good? |
| Intelligence | What does it know, and what can it do? |
| Learning | What has it learned the hard way? |
| Reviews | Was it right? |
| Inbox | Who is talking to it? |
| Fleet | What machines are in this fleet? (armada only) |

## Expert view

Settings → Advanced → **Expert view** turns on the expert column: subsystem
names, numeric identifiers, raw model names, route decisions, and the
storage paths of everything. It is for the owner and for debugging. It never
hides the plain UI; it annotates it.

## Appendix — the current surface strings, and their fate

The 21 routes and 4 rail groups the 0.7.5 face declares, mapped to the
rebuild. `keep` = the string survives as-is; `rename` = the same surface
under a plain name; `fold` = the surface becomes a lens segment; `advanced` =
moves to Settings → Advanced; `delete` = removed.

| Current route | Current label | Fate |
|---|---|---|
| `chat` | the conversation | **rename → Threads** |
| `sessions` | sessions | **fold → Threads** (the archive is the thread list) |
| `huddles` | huddles | **fold → Threads** (a room is a thread with participants; the registry goes `advanced`) |
| `board` | the board | keep → **Board** |
| `desk` | desk | keep → **Desk** |
| `memory` | intelligence | keep → **Intelligence** |
| `skills` | intelligence · skills | **delete** (already a segment; the redirect dies) |
| `learning` | learning | keep → **Learning** |
| `calibration` | calibration | **rename → Reviews** |
| `inbox` | the inbox | keep → **Inbox** |
| `armada` | armada | **rename → Fleet** (armada on only) |
| `router` | router | **advanced → Routing** |
| `delegation` | delegation | **advanced → Helpers** |
| `vault` | vault | **advanced → Secrets** |
| `workspace-history` | workspace history | **advanced → Versions** |
| `second-brain` | second brain | **advanced → Health** |
| `plugins` | plugins | **advanced → Plugins** |
| `bots` | bots | **advanced → Assistants** |
| `design` | design | **advanced → Design** |
| `settings` | settings | keep → **Settings** (which is now also the Advanced home) |
| `about` | about | **advanced → About** |
| *(rail extra)* `telemetry` | telemetry | **advanced → Health** |

Rail groups `work / intelligence / knowledge / system` are **deleted** — the
rail is the seven lenses and nothing else (U44).

Section headings inside the surviving surfaces (`what needs you`, `made
here`, `connections`, `saved views`, `this machine`, `filter this view`) are
**kept** — they are already plain. Every heading carried by a deleted or
folded surface dies with it; every heading carried by a renamed one becomes
its new plain name.

Body copy is rewritten under the rules above in the phase that rebuilds its
surface; until then it is **marked for translation**, and the P6 lint refuses
a literal that is in the forbidden list.

