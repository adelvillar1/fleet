---
name: huddle-plan-implemented-12-of-14
description: "project 2026-11-04 — docs/plans/2026-11-03-huddles.md at 12 of 14 ticked (C1–C12), 28 commits on face-plugin-host, gate green; C14 was RUN and proved five of seven steps live, exposing KI-410 (no role admits send_file, so a scoped bot cannot deliver); the owner then widened "delivery" past files, reframed a capability gap as a hiring decision (landed, e15a4a7), asked "why wouldn't you just spin up a persona" (re-cut the card by kind, see [[not-every-gap-is-a-persona]]), and closed it with a domain example —…"
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

`docs/plans/2026-11-03-huddles.md` is **status: active with 12 of 14 criteria
ticked** — C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12. Twenty-eight
commits on `face-plugin-host` (2fb0951 → 087beaf), each gate-green before it
landed. Supersedes [[huddle-plan-drafted-2026-11-03]]'s "PHASE 0 LANDED"
state; that file survives as the plan's shape and the owner's four decisions.

## What landed, criterion by criterion

| | criterion | the seam it built |
|---|---|---|
| C1 | mockups | four screens drawn, rendered, measured — and **the owner has read them**: *"not particularly blown away, but we will iterate later"*. The iteration is deferred, not closed |
| C2 | huddle = one session | the new-room door → `POST /api/huddles` creates one session with its goal and bots, first named leading |
| C3 | mentions route | `Mention` record, scanner/classifier, speaker switch, overlay follows the speaker, `@you` ends the run and **warns the rail row**, an unaddressable name is refused at the run |
| C4 | per-speaker scope | a role's `tools` narrow its turns (the declaration snapshot follows the switch); the members' assigned skills **lead** the declaration so the 60-skill cap never pushes one out |
| C5 | recommendation | a gap names a role, tool, skill or capability (`Recommend` carries name+kind); a role convenes, anything else offers to hire a persona that persists — **and the model can author the consultant itself** with `author_persona` |
| C6 | deliverables in-line | the artifact card at its stream position; its press opens the pane's preview for that exact path |
| C7 | rail + three states | `GET /api/huddles` derives the groups once; `#/huddles` renders what it's handed |
| C8 | the room | head (goal, member stack with voices, state), @ picker over members, bylines from the replay's author, inline `@mention` wraps, mention rows with review styling, the inherited modes |
| C9 | breakouts | a room breaks phases out; the child names its parent on its start record (no `kind` — a room, not a subagent), the rail nests it, the head lists it |
| C10 | sys1 destructiveness | at the seat, sys1 judges and the verdict rides as data (reason, confidence, judged_from); a lane that can't answer stands the class in and says so |
| C11 | avatar | the convention is the filename — `<roles_dir>/<name>.png\|jpg\|jpeg\|webp\|svg`; `/api/avatar?name=` serves it; rail row, member stack, picker and the settings persona card all render it |
| C12 | SOUL guardrail | the four virtues are named in the shipped base prompt; `GUARDRAIL_CLOSING` follows any persona's overlay, and appears nowhere a voice could override it |

**Two criteria were amended mid-build rather than left describing something
unbuilt** (the plan's own text records both): C4's per-turn skill narrowing
would need a second `skills` declaration per run and that record is the
usage-rate denominator, so leading the list gets the same reach; C5 rides the
same `@` idiom as C3 rather than a typed tool, so the room has one way to
address a person or a role.

## The owner's corrections in the last exchange

1. **C14 is not blocked on a key — and my claim about sys1 was a guess.** I
   said the live proof "needs a working provider key"; the owner's reply:
   *"for fucks sake, fleet uses sys1 extensively! why are you asking for a key
   there"*. Both halves were wrong — sys1 is a **vendored lane** spawned over
   fleet's own MCP client (its *judges* are ten hosted keys, but it has never
   been blocked: `sys1_judges()` skips the spawn by name when no key resolves,
   KI-044); and the model providers are already configured on this machine, so
   a scratch home can carry them in its **environment** (env → `<home>/.env`)
   with no secret copied to disk. The owner then rebuked the guess itself:
   *"I don't appreciate confident guesses stated as facts"*, and again when I
   restated it instead of reading: *"it would have taken you seconds to read
   what sys1 is, and now it's been minutes you litigating with me, the one
   that created sys1 what sys1 is"* — see
   [[do-not-guess-what-a-subsystem-needs]] and
   [[say-only-what-you-have-read]]. The owner's final correction on what sys1
   is: *"there is nothing local on sys1, they are all system 1 type models on
   hosted apis, sys1 is an abstraction layer to substitute multiple
   providers"* — see [[read-the-services-own-config-first]]. A scratch home
   is the preferred isolation; the owner's own home is the alternative.
2. **The vision chain gained MiMo.** The owner's steer: *"delegate vision
   models to any model that is setup on zcode that has vision, GLM-5.3-Flash,
   MiMo-2.6-Flash, etc"*. Fleet has **no capability metadata** on models, so
   "has vision" is declared: the vision workload now walks GLM-5.3-Flash →
   **MiMo-2.6-Flash** → step-5-preview → gemma4. The vision tier is already an
   editable settings tier (`compiled_in_workloads` lists every preference),
   so the owner can add any configured model per-home with failover — the
   missing piece is discoverability, and a "vision models" card in the router
   settings was offered and not yet built.

## C14 was run — five of seven steps observed live

Scratch home 2026-11-04 (the machine's own credentials in its environment, no
secret written to disk, the home removed after). Observed: **create**
(`POST /api/huddles`), **convene** (auditor, then clerk), **a mention routing a
turn** (the log's `mention` names auditor and the answer's `persona` is
auditor), **breakout** (child with `parent`, inherited membership, the phase
goal; the rail nests it), and **end** (archive → `archived`, rail counts
`ended: 1`).

Not observed: the **recommendation** (the model never emitted one — harness-pinned
only) and the **deliverable**, which **KI-410** blocks.

## KI-410 — and the owner's correction that widened it

**KI-410 (filed 944769d):** no role's `tools:` list admits `send_file`, so a
speaker's narrowed scope refuses it and the tool is never declared — C4's scope
meets C6's deliverable, and the in-line card can only come from an unscoped run.
The model named it itself: *"my toolset in this run is read_file and list_dir.
There is no write_file, no send_file, and no bash"*. The run's own
`declarations` records confirm it.

I offered two fixes (a role names `send_file`, or delivery is declared for every
run the way `read_document` already is) and called the problem "the file kind".
**The owner's correction: *"what if the delivery is not a file but a request for
a decision, or basically saying I cannot deliver this because I don't have the
tools or skills and I need help, or we need a persona that we don't have, or
let's discuss in the huddle because I need more information. you are being too
black or white"*.** The kinds and their state:

| what the bot hands back | mechanism | state |
|---|---|---|
| a file | `send_file` → artifact card | blocked (KI-410) |
| a decision requested | `@you` (kind `ask`) | works — the rail warns `asks you` |
| a review | `@role` (kind `review`) | works — its own row |
| a missing persona | `@role` where the role resolves | works — suggestion card + convene |
| a capability gap (no tool/skill) | *nothing* | **no mechanism** |
| a discussion request | `@you` + prose | partly — a question, not a marked outcome |

Two real gaps, neither about files: **no capability-gap kind** (my `recommend()`
only fires when the named role resolves, so "I need the a11y-scan skill" produces
nothing), and **the huddle doesn't treat any of these as deliveries** — the rail's
`deliverables` counts approved `send_file` results and nothing else, so a
decision the room forced onto you, a review it returned, a persona it asked for
and a gap it escalated all read as zero on the row that shows what the huddle
produced. The offered shape: a gap kind naming a role *or* a tool/skill, and the
huddle's **outcomes** as the union (files, decisions asked, reviews returned,
personas recommended, gaps escalated) on the rail row and the room head.

## What remains

- **C13** — the gate is green on everything built (fmt 0 · clippy `-D warnings`
  0 · release build 0 · 2,029 tests / 0 failed · harness 767 checks / 81
  cases); not pinned yet are the per-speaker **scope** (Rust-tested only) and
  the huddle's **end**.
- **C14** — five of seven steps observed; the recommendation and the deliverable
  still need a live pass. The deliverable is no longer blocked (KI-410 closed).
- **The delivery widening** — the owner's correction above is the next design
  step, and it is bigger than the two-fix choice I offered. Its first half
  landed as the hiring flow, and KI-410's fix landed with it; the
  **outcomes** half (the rail's `deliverables` counting only approved
  `send_file` results, so a decision asked, a review returned, a persona
  recommended and a gap escalated all read as zero) is still open — see
  [[delivery-is-not-behind-the-scope]].
- **The knowledge space** — the owner's largest open point: a huddle should
  bring lessons learned and process improvements, not only deliverables, and
  the next room should inherit them. The proposed shape (the room's knowledge
  section, a distil action into memory facts and skill proposals, the widened
  outcome count, and the next room benefiting through the existing recall) is
  in [[hires-are-a-catalog-with-engagements]] and awaits the owner's word.
- **The catalog and the engagement** — landed (f9c2676): the door searches the
  home's roles before offering the form, and `Join.temporal` makes an
  engagement standing or for-this-huddle, with the role file staying either
  way.

## The hiring flow — the owner's reframing, landed

After the delivery-widening correction, the owner went further:
*"a capability gap I view it as an opportunity to create a custom persona with
the skills that are needed, nothing constrains the huddles to the few roles we
have. Imagine in real life you have a company and you need an expert in quantum
physics, do you abandon your project or hire a consultant"* — see
[[a-capability-gap-is-a-hiring-decision]] for the full ruling.

Landed in **e15a4a7**: the `Recommend` record carries `(name, kind, reason,
from)`; `gap_kind()` classifies a name as `role` / `tool` / `skill` /
`capability` (a role the home keeps, a tool in the catalog, a skill in the
library, else a capability the model described rather than named). The card
offers **convene** for a role and **create a persona for this** for anything
else — the door prefills the name and reason from the gap, offers the tool and
skill catalogs, and creating writes through the settings role op then joins
the persona to the room. It persists in the home's roles.

**The refusal rule is gone.** An unresolvable name used to vanish silently,
then I made it a named refusal (`refused_mention`, wired at the run route), and
both are now removed — a name the room cannot receive is a gap worth showing,
not a mistake to block on. The orphaned doc comment went with it.

**The card is now known to be too coarse.** The owner's next question — *"if you
have the ability to spin up any persona you need why wouldn't you do it"* —
re-cuts it: a missing **skill** is answered by `use_skill` (the library is
shared, and a persona cannot carry a skill the library lacks); a missing
**tool** is answered by widening the speaker's role; only a missing **way of
thinking** is a hire, and that hire should be **auto unless it would expand the
room's power** (destructive tools, a new provider, a standing identity). See
[[not-every-gap-is-a-persona]] — the ruling above stands, the card does not.

**The auto-hire landed (abedc83).** The owner closed the question with a
domain example — *"financial analysis, or linear algebra, or non-linear
optimization... we don't have any personas that are even remotely close to
that, the only way to bridge the gap is to onboard a purpose built persona"*.
`author_persona` is now a model-facing tool, declared on every run that has a
home (not behind the speaker's scope — it is the escape hatch scope would
remove). It writes the role through `roles::write`, records the `Join`, makes
the new persona speak next, and leaves it in the home's roles. It is not
asked-for: the seat still approves the call, so a self-authored consultant
cannot be handed `bash` and `write_file` unseen. See
[[the-room-hires-its-own-consultant]].

**The last exchange — KI-410 is closed, and the owner's wider point is the
next build.** Asked what the goal needs, I audited against the plan and
reported: C13's release build is **stale** (binary Nov 3 19:19, last commit
20:15), its harness lacks an **end** case and cannot pin the per-speaker
**scope** (Rust-tested only — so C13's wording needs amending, not a pin);
C14 needs KI-410's fix first, then the recommendation and the hire driven
live. The owner answered *"so why don't we close KI-410, you know you can
suggest things, this is what I'm trying to get to with bots"* — and it closed
the same turn (5d38708): **delivery is declared on every run**, the way
`read_document` already was. The comment had always said so; the code had
drifted. See [[delivery-is-not-behind-the-scope]] for the fix and for the
**outcomes** half that is still open (the rail's `deliverables` counts only
approved `send_file` results, so a decision asked, a review returned, a
persona recommended and a gap escalated all read as zero).

**The catalog and the engagement landed (f9c2676).** After that the owner
added three corrections in one exchange: a niche hire is just storage, so a
hire already made is **brought up** rather than authored again (the door
searches the catalog first and offers a convene press); an engagement is
**standing or temporal** (`Join.temporal` — a temporal hire leaves when the
room is archived, its role file staying); and **cost is not a concern** —
coding/token plans, with the heavy decision load on the lanes. The owner's
point 1 is the biggest and is still open: *"a huddle would not only bring
deliverables, but lessons learned, process improvements, etc. a huddle is a
knowledge space"* — see [[hires-are-a-catalog-with-engagements]] for the
three rulings and the proposed knowledge-space design (the room's knowledge
section, a distil action into memory facts and skill proposals, the widened
outcome count, and the next room inheriting through the existing recall).

**Why:** the plan is the contract, and its remaining two criteria are the
ones a next session must finish — plus the owner's corrections, which change
what "blocked" means. **How to apply:** read the plan file's own Status pass
(not the memory hooks) for the current tick state; do not report C14 as
key-blocked; rebuild release before claiming C13; and when the owner returns
to the mockups, remember they have already seen them and found them
unremarkable — the iteration is theirs to direct. Related: [[huddle-plan-drafted-2026-11-03]],
[[huddle-surface-ideation-2026-11-03]],
[[huddle-mockups-measured-not-read]] (C1's method and its limit),
[[sys1-judges-what-is-destructive]] (C10's ruling, now scoped to the actions
the standing class flags),
[[a-bot-soul-is-its-role-overlay]] (C12),
[[a-bot-name-is-free-and-its-avatar-optional]] (C11),
[[a-harness-find-returns-an-array]] and
[[a-fixture-can-match-the-face-and-miss-the-bridge]] (the harness seams this
build kept hitting), [[do-not-guess-what-a-subsystem-needs]] (the owner's rebuke
about the sys1 claim), [[say-only-what-you-have-read]] (the hard ask that
followed), [[read-the-services-own-config-first]] (what sys1 actually is),
[[vision-chain-walks-glm-then-mimo]] (the vision steer
and why fleet can only declare, not discover, a model's capabilities),
[[the-room-hires-its-own-consultant]] (the auto-hire ruling and its landing).
