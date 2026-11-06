---
name: not-every-gap-is-a-persona
description: "owner question 2026-11-04 — \"if you have the ability to spin up any persona you need why wouldn't you do it\"; I had collapsed three different gaps into one hiring card, and the owner's question exposes the collapse"
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

After the hiring flow landed (e15a4a7), the owner asked:

> *"if you have the ability to spin up any persona you need why wouldn't you do it"*

The question is right, and it exposes that I had built one card for three
different things. The separation:

1. **A missing skill is answered, not hired.** A persona's `skills:` are *names
   from the library*, and the room's library is shared — every skill in it is
   declared to every speaker, and any speaker can `use_skill` it. So if the
   library has the accessibility skill, the auditor loads it directly and no
   persona is involved. If the library lacks it, **no persona can carry it
   either** — a persona's skills resolve from that same library. My "hire a
   persona with this skill" card was wrong on both branches.
2. **A missing tool is answered by widening the speaker's scope.** Tools come
   from the harness; a role's `tools:` list only *narrows* them. When the
   auditor said "I have no write_file", the fix is the auditor's own tool list
   (or the room's standing) — not a new actor with a different list.
3. **A missing *way of thinking* is the only real hire.** "Quantum physicist" is
   a framing, a voice, an approach the room doesn't have. That is a persona — and
   even then it is a *standing actor the owner will work with*, with tools and a
   voice the owner may want to see before it exists.

**Why:** the owner's question is the test — "why wouldn't you do it?" — and the
honest answer is that for two of the three kinds, doing it is *cheaper and
correcter* without a persona. Auto-hiring for a skill the library already has
would be a new agent to do what `use_skill` does in one call.

**How to apply:**
- **Skill gap → answer in place.** If the library has it, the bot uses it (no
  card). If the library lacks it, the gap is *"the home needs this skill"* — a
  library question, not a persona.
- **Tool gap → widen the speaker's role.** The bot asks for the tool in its own
  role, not for a new actor.
- **Voice gap → hire, and the default is auto.** Auto-create the persona when it
  is non-destructive and the room can afford it; ask the owner when it would
  expand the room's power (destructive tools, a new provider, a standing
  identity the owner should see). That is the answer to "why wouldn't you do
  it": **auto-hire by default, gated by scope** — not an owner-pressed card,
  which is what I built.
- The reasons an owner-pressed card is still sometimes right: a new persona is a
  new agent with a new model call per turn, and a bot that can author its own
  teammates can author ones with `bash` and `write_file` — expanding its own
  scope, the same class as a destructive act Fleet already seats.

**What this supersedes:** the hiring card as built in
[[a-capability-gap-is-a-hiring-decision]] (owner-pressed, one card, any
non-role gap) — the *ruling* there stands (nothing constrains a huddle to the
six roles; a gap is a hiring opportunity), but the card must be re-cut by kind.

**And the auto-hire half is now answered** — the owner's next ruling,
[[the-room-hires-its-own-consultant]]: a *domain* (financial analysis, linear
algebra, non-linear optimization) is a hire the room makes itself, landed as
the model-facing `author_persona` tool (declared on every run that has a home,
not behind the speaker's scope; the seat still approves it). So the three-way
separation above holds, with the third branch no longer owner-pressed: skill
gaps are answered in place, tool gaps by widening the speaker's role, and
domain gaps by the model authoring the consultant itself.

Related: [[huddle-plan-implemented-12-of-14]] (C5's state, C14's run),
[[a-delivery-is-not-a-file]] (the delivery kinds — the other half of the same
collapse), [[the-room-hires-its-own-consultant]] (the auto-hire landing).
