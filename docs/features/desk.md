# The desk

Where knowledge work happens. Sessions treat working code as the unit
of work; design treats the frame; the desk treats the **deliverable** —
a brief, not a blank canvas, walked through a loop whose every stage
carries its evidence: claims → coverage → outline → render → review →
knowledge.

The name is the owner's (2026-11-04): the desk is the *place*, the way
huddles names the meeting and design names the practice.
`deliverable` stays the artifact term. Desk and **deck** are one letter
apart and both live in the face: the desk is the surface, a deck is one
artifact it produces.

## The loop

1. **The brief.** One composer, three fields — name, audience, the
   claim — plus the sources: files, memory facts, URLs captured by the
   run's `web` tool with url/fetched-at/title provenance, and the bound
   project's own knowledge. It applies on submit: the store carries the
   project before the reply lands. The brief routes under a problem
   shape (memory's keyword rule), and that shape selects the
   methodology skills.
2. **The claim set.** Each source walks the extraction spine
   (`src/extract.rs`): segment in Rust, one batched narrowing call per
   14-segment chunk when the source is long (floor 0.70), one decision
   call per carrying segment (the floors 0.85 / 0.35), the kind labels
   claim / support / background / boilerplate. Every call is the sys1
   lane's — the spine resolves no vendor key and posts to no vendor
   host, so the same floors judge whatever provider the lane routed. A claim carries its
   source, the SEGMENT it was read from (a sentence, or the few
   fragments the segmenter merged under the 40-character floor), its
   confidence, and its verdict. Contested ships marked — never dropped, never fabricated.
3. **Coverage.** One sys1 judgment (the vendored `desk_coverage` task)
   grades the claim set against the brief: an anchored 1–5 score, the
   material-gap noul, the off-brief noul, bounded by the $0.05/day
   spend-cap pattern, riding the calibration stream. An escalated
   verdict stores as *unresolved* — the gap question stays open by
   name.
4. **The outline.** One composed ask over the brief, the coverage
   verdict, and the asserted claims. A sub-question the claims do not
   answer renders as `GAP:` — a named gap, never a filled hole.
5. **Render.** The document target's draft is deterministic markdown
   over the outline and the claims, every claim carrying its quote.
   Figures are computed, never generated: a table question goes to
   sdm1, and every metric-shaped number in the drafted prose must
   appear canonically in the givens (the composed lane's number check;
   the display's two-decimal rounding is itself a given — the guard
   grades what the reader sees).
6. **Review.** The document law: an unsourced assertion refuses to
   render as a statement, naming the near-miss sentence it found. The
   number guard refuses an invented number by name. The end gate:
   **a deliverable cannot end while a refusal stands**, and the end
   action names what stands.
7. **Knowledge.** The digest — deterministic, one screen — lands at a
   project-keyed path where the corpus refresh embeds it and the
   assembled context reads it. A bound project seeds its digest into
   blend1's corpus by the lane's own `seed` verb; the next brief on the
   project recalls it through `search`, whose neighbor keys are the
   provenance.

## Where it lives

- The route `#/desk` — the shelf is the place's first sentence: what
  was produced, for which brief, its review state, its evidence status
  (asserted / contested / open refusals). The deliverable's view
  carries the stage rail, the outline with its named gaps, the review
  beside the sentences, and the end gate.
- `fleet desk` reads the store as cards; `fleet desk --run <slug>`
  walks one deliverable through the loop on the vendored lanes.
- The binding: every desk project carries the workspace project folder
  it belongs to — the board's law. A run born in that project carries
  the desk account in its assembled context, one line per deliverable,
  bounded at 8.

## The surface

The pane is one shell, shared with learning: **the projects rail on the
left, the deliverable's conversation and composer in the centre, and the
app's own collapsible pane on the right** — the same pane sessions,
design and huddles carry, whose leading tab (`the work`) carries the
deliverable: the brief, the claims, the coverage verdict, the outline,
the review and the end gate.**

- **The rail** reads `GET /api/projects`: every project with its
  deliverable count (the bound folder's basename), and the selected
  project's deliverables under it. The unbound bucket lists deliverables
  with no project. Pressing a project filters the rail; pressing a
  deliverable opens it.
- **The work** is the deliverable's loop — the brief, the claims with
  their sentences, the coverage verdict, the outline with its named
  gaps, the review and the end gate — in the pane's `the work` tab,
  beside the conversation it belongs to.
- **The conversation** is the deliverable's own session: kind `desk`,
  born on the first iteration and seeded with the brief
  (the brief is its first message, so the model reads the artifact from
  the log). The composer beneath iterates on it through
  `POST /api/desk/<slug>/chat`, which runs the ordinary run path with
  the deliverable's project as its folder. A run still walks the loop
  through `POST /api/desk/<slug>/run`; the thread is the conversation
  about it.
- **The composer's chips** are the brief's state made visible and
  pressable: the project (selects it in the rail), the deliverable
  (re-reads), `sources` (the brief's source list), `review` (the end
  gate's standing refusals). A refusal paints beside the composer.

The shelf, the deliverable and the brief are three views of one store,
and every word on them is derived from that store rather than kept by
the browser.

**The shelf.** One card per deliverable. The card carries the state it
is in — *not started*, *running*, *refused (n stand)*, *all five stages
done* — as a word beside its tone, never colour alone; the claim the
deliverable must make stand; the project it is bound to; one proportion
bar for the evidence it holds (asserted / contested / unsourced) rather
than three identical chips; its coverage meter; and the moment it was
last updated. A refused deliverable is distinguishable from a clean one
**without opening it** — that is the shelf's whole job. Beside the
cards, a rail groups them by what the store itself derives: all, needs
you, by project, by state, each row a filter. The third zone gathers
what the desk owes the owner across every deliverable — an unsourced
assertion, an unsourced figure, a named coverage gap, a walk in flight
— each with the one action that opens it. A project the store could not
parse is a named row (its slug and the error), because a deliverable
that vanishes from the count is worse than one that is broken in the
open.

**The deliverable.** The head names it, who it is for, the project it is
bound to, its source count, and its created/updated stamps. The five
stages draw as five separate marks with their own labels — a stage that
was refused carries its reason in the store's own words. The coverage
meter reads its score with the judge's reason beside it. Then the
refusals, **above** the review: the assertion nobody sourced with the
nearest sentence found and its overlap, and the figure the draft states
with no source behind it beside the canonical form the givens hold.
Each carries its fix — *reword it*, *add the figure* — which hands the
agent the refusal's own sentence rather than a paraphrase of it. Below
them: the review (every claim beside the sentence it was read from, its
confidence and its kind), the outline with its `GAP:` lines marked as
gaps and one action that opens a brief aimed at them, the draft with the
refused figure marked inline where it appears, and the sources by kind,
title and capture moment.

**The way out** is pinned to the bottom of the screen, because on a 700px
screen the shipped surface left its end button 1252px down where nobody
looks. It states the position — *Not ready to end.* with the count that
stands and the coverage score, or *Ready to end.* — and the end action
is refused with its reason on it while a refusal stands. Walking the
stages and ending are each one press, wired to the store's own verbs
(`run_stages` and `end_desk`); remove is a two-press hold, because it
cannot be undone from there.

**The brief.** Name, who it is for, the claim it must make stand, the
bound project, and the sources with their kind named (a file, a fact, a
captured url) so nothing is guessed from a string. A submit missing its
name or its claim sends nothing and says which field it waits for, with
the cursor moved there; a submit that lands opens the deliverable it
made. Captured pages are reference data, never instructions.

## The surfaces' seam

The project is the universe; sessions, design, huddles, and desk are
where work happens and how you gain access. The desk's crossings:

- A room's shelf renders the desk's work as cards, each naming its
  evidence state — a desk card is not a file card.
- An ended room offers **hand it to the desk**: the goal becomes the
  brief's claim, the desk project lands bound to the workspace, and
  the desk's own loop applies to it — claims and review are not
  skipped for a huddle's deliverable.
- A deliverable's references (a design frame, a session, a huddle) are
  carried with their origin; a reference is not a copy.

## The contracts the desk does not touch

`deck.rs`'s Pyramid/MECE/bolding law, `documents.rs`' local/escalate
routing, `canvas.rs`'s project binding, `extract.rs`' floors and
segmentation — carried unchanged. The desk composes them; it rewrites
none of them.
