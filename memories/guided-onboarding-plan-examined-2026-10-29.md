---
name: guided-onboarding-plan-examined-2026-10-29
description: "project 2026-10-29 — the owner's draft plan docs/plans/2026-10-29-guided-onboarding.md (`fleet onboard`) examined claim-by-claim against the code, AMENDED with four findings, then RE-DESIGNED around a code find (NO first-run today: a keyless home shows the full power-user welcome and the first send writes `routing config:` into the transcript), then given a DESIGN LANGUAGE mapping every element onto an existing CSS primitive; draft, 14 criteria, uncommitted"
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

2026-10-29: the owner asked me to examine
**docs/plans/2026-10-29-guided-onboarding.md**, a plan they drafted in a
separate session — guided onboarding for first-time users, five
dependency-ordered steps ("speak, measure, remember, prove, choose":
first LLM key → the five lanes in lane order → the projects folder →
the GitHub PAT → the model roster), with a derived-content rule (every
fact narrated is read from LANES/route/settings/mcp registries, never
restated), writes only through existing seams, a two-rung proof ladder
(presence free / opt-in one-call live verify), skip-with-named-
consequence, resumability recomputed from doctor's checks.

**What verified clean:** `set_secret` refuses non-credential-shaped
names; the GUI's settings `"secret"` op already calls it; LANES carries
the per-lane passthrough key env names; CONTEXT_WINDOWS + the
undeclared-model floor back step five; KI-074/KI-087 are closed rows
whose fix (the memory line names the adopted root) is what the projects
step extends; `TOWER_LOG_PROJECTS_DIR` with `~/Projects` default matches
the step; `vault_onboard` exists; the live mcp.json's github card
carries GITHUB_PERSONAL_ACCESS_TOKEN as an env NAME.

**The two real gaps found:** (1) `fleet install` was missing — the
lanes' live verify spawns venvs only install builds; (2) the GUI cards
were promised in Files and never specified, no criterion covering the
GUI (against the house's UX-co-equal rule). Two smaller: the completion
probe's "one recalled fact" is unmeetable on an empty corpus, and step
1's "flash tier by default" misnames the chain (a one-key home WALKS TO
its key via the failover tail).

**The GUI deep pass and the re-design** (owner: "I really want this to
be frictionless and to drive the user through the process explaining
first what the steps are for and the benefits"). Code find: **there is
no first-run at all** — a fresh keyless home renders the full
power-user welcome (method strip, lanes row, four offers, a Perception
footnote, dash counts) and the first send fails IN THE TRANSCRIPT with
`routing config: <error>` (bridge.rs); the CLI is worse
(`route::preflight` prints every acceptable `*_API_KEY` name). The
replacement design (see [[onboarding-is-a-door-not-a-wizard]] for the
rules): **two states of the home** — the **door** (only when no
routing candidate resolves a key: three sentences, one field, one
button, benefit language) and the **work home** with ONE quiet "set up
more" entry of benefit-first cards. Look-before-asking (scan env +
`.env`, offer a found key as one press); the proof is the product (one
bounded chat call, the answer shown); a keyless send raises the door
inline keeping the typed words. **Nothing else blocks**: no passphrase
and no install on the critical path — a chat turn needs one model key
and no lanes (judgment fails open) and `set_secret` with NO passphrase
writes the plaintext `.env` line (settings.rs:2988), so the door is
zero-ceremony; "Seal your keys" (`vault_onboard` + `vault_seal_env`) is
an offered upgrade. (This CORRECTS the first amendment, which had put
install and the passphrase on the blocking path.)

**The design-language pass** (owner: "make sure that the design language
is consistent with what we have in the current gui"). Read from
app.css: an editorial operations console — warm paper, display serif
for announcements, interface sans for reading, mono for measurement,
three-tier colour chain, `.card` 14px radius + hairline, prose 54ch,
lowercase eyebrows letterspaced to uppercase, declarative titles with
no exclamation. Every onboarding element now maps onto an existing
primitive: **`.empty`** (the door's frame: `min(760px, 100% − 56px)`,
centred, no card border), **`.empty-eyebrow`** (`fleet · first run`,
10px/600/0.13em accent-lift + the 22px `::before` rule),
**`.empty h1`** (display serif, clamp 36–52px, `-0.025em`, 16ch),
**`.empty p`** (14.5px, secondary ink, 54ch), **`.offer`** rows for the
discovered-key press AND the capability cards (26px raised icon square,
12.5px/600 title, 11.5px tertiary `said`), **the product's own
`.composer`** with `.foot-btn.send` as the door's field (the first act
must be the act they keep doing), **`.method`** for the mechanism
disclosure, **`.notice`** + the real artifact for a proof. An explicit
**may-not-introduce list** is a criterion: no colour outside
`--dsw-alias-*`/`--lane-*`, no new radius/font/spacing unit/icon set,
no component class beyond `.empty`/`.offer`/`.card`/`.method`/
`.notice`/`.foot-btn`/`.ghost`/`.row-rewind` — a door-only class is
layout only.

The same pass caught a self-contradiction: the GUI section said "not a
wizard" while the rest of the plan used the word six times. **"Wizard"
retired everywhere** — it names a modal metaphor the design rejects;
the thing is "the onboarding" (a sequence in the CLI, two states of the
home in the GUI).

Plan state: **draft, 14 criteria, uncommitted** (left alone; the owner
is shaping it in the other session). Six findings recorded in the plan's
own `Examined 2026-10-29` section, with finding 2 marked superseded by
finding 5 rather than quietly rewritten. Related:
[[onboarding-is-a-door-not-a-wizard]],
[[fleet-onboarding-composer-proposal]],
[[fleet-settings-abstraction-search]],
[[fleet-anti-hermes-constitution]],
[[no-form-asks-for-a-value-fleet-already-has]],
[[fleet-gui-editorial-desk-redesign]].
