---
name: bot-pwa-b0-mockups-built-2026-10-28
description: "2026-10-28 — the owner corrected that B0 (the bot-PWA mockups) was session-owned, not owner-owned, and required them to come from deep research; the four screens now exist under docs/plans/mockups/2026-10-20-bot-pwa/ with a research-cited README, verified across 102 headless page loads; still unapproved, so B1 remains blocked"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28, asked what B0 was waiting on. The session had been reporting
B0 as "the owner's gate", as if the owner owed the mockups. The owner's
correction: **"so B0 is in fact session owned, the session should have
come up with best in class bot interfaces for me to review. These
mockups should come from deep research."** A D8-style gate means the
owner *approves*; producing the artifact is the session's job, and the
owner expects it unprompted at the point the plan names it — not to be
asked for it.

**Delivered** — `docs/plans/mockups/2026-10-20-bot-pwa/`:
`inbox.html`, `conversation.html`, `dispatch.html`, `install.html`, one
shared `mockup.css` (tokens copied verbatim from the console's
`app.css`), and a `README.md` carrying the states table, the
430/900-width table, the research provenance per decision, the B4/B5
implementation notes, five open owner questions, and the empty dated
approval line. Each file has its own state switcher, width slider and the
three real themes (paper/console/graphite); state and width ride the URL
(`?state=filter&w=900&theme=console`) so review needs no server.

**Research drove named decisions, not vibe** (two subagents: shipping
agent-app UX, and mobile-web/PWA craft). Cursor's cloud-agent app
launches to an agent inbox; Claude needs three taps to reach a session
→ the inbox is the launch surface, one tap to any conversation. Cursor
and Claude Code surface agent state at *list* level → our four words
live on the row. Manus's working chip has no cancel (its own teardown
flags it) → ours carries Stop beside the clock. Perplexity's approval
teardown names approval fatigue and the un-highlighted safe default →
Allow once first and filled, Always states its scope in words, Deny
quiet rather than red. iOS has no install API → a coach mark pointing
at Share, labelled as browser chrome, not ours.

**Verified, not eyeballed:** 102 headless page loads (17 file-states ×
2 widths × 3 themes) — no horizontal overflow, every control ≥44 px,
every field ≥16 px, every text element at WCAG AA. The pass caught five
real defects: both composer buttons rendering at once (a bare `.when`
hide rule lost to `.dock .primary-act` on specificity), a working chip
flattened out of its flex row, the transcript opening at the top instead
of the newest message, and two theme-cascade bugs.

**Reshaped the same day on a second owner ruling:** the first pass was
work-centric (grouped by state, a workspace path per row, "what should it
do?") and the owner rejected it as rehashing the console. The mockups are
now assistant-first — personas are the axis, the workspace left the list,
dispatch asks who takes it — which exposed that **a session records no
persona** (only `roles.rs`, and only inside a delegation). Full detail:
[[bot-pwa-is-assistant-centric-not-work-centric]].

**Owner asked whether each assistant should get an avatar** (2026-10-28).
Answered: identity yes, portraits no. Every persona now carries a mark —
its serif initial on a tint from the console's own lane palette
(`--lane-0…5`, which already has a per-theme value): tower keeps the
accent as the lead, scout teal, auditor amber, scribe violet. Three
reasons a portrait was refused, all structural: personas are
**user-authored** (any new `roles/<name>.md` would arrive imageless),
a face would turn a readable contract (tools, skills, recall shapes) into
a mascot, and the lane palette costs no new design language or assets.
Recognition gets three channels — shape, colour, name — and the mark
appears in the row, the rail, the conversation header and the persona
page, **never beside a message** (Cue puts avatars in the list; every
mainstream assistant keeps the transcript avatar-free).

**Then the owner raised the bar to readability in every theme**, and the
first tint failed it by measurement: using the lane hue as the glyph's
ink on a **translucent** wash gave scout 4.49 and auditor 4.19 in paper,
and drifted with whatever row sat behind. Fixed with two rules — the
disc is **opaque** (12% of the tint mixed with the theme's own surface,
so the ratio no longer depends on context) and the glyph's ink is its
**own per-theme value**, computed as the smallest step toward black
(paper) or white (dark) that clears 5.2:1, never the hue reused. Final
letter-on-disc ratios, all ≥5.2: tower 5.24/5.27/5.45, scout
5.21/7.79/8.21, auditor 5.21/8.53/8.05, scribe 5.21/5.57/5.43 (paper /
console / graphite). The contrast checker now walks the marks, chips,
role lines and scope block too. The property only holds while the disc
stays opaque — a translucent tint brings the paper failures straight
back, so the rule is written into the README.

The cheapest next
step if more warmth is wanted is an optional `mark:` frontmatter key
naming a glyph from a closed set (the roles editor already preserves
unknown keys, so no schema change) — an open question, not built.

**The roster grew to six on the owner's next request** — three of them
domains: knowledge work, presentations, job hunting. Their skills already
exist in `~/.agents/skills`; the role files and the session binding do
not. Detail: [[bot-pwa-persona-roster-six]].

**The conversation screen gained two states the same day**, on the
owner's inter-persona ruling: "Two personas" (a goal pinned as chrome, a
join row in the log, two bylines, overlapping marks) and "Add a persona"
(a sheet that restates the goal before offering anyone), plus the seat
rebuilt as the destructive case — an unasked append beside a held
overwrite. Matrix now 17 file-states, 102 loads. Detail:
[[one-goal-many-personas-destructive-is-the-only-gate]].

**The owner's verdict on the rebuilt mockups was "looks good"** (still a
review of the artifact, not the D11 approval line), and the follow-up ask
was to fold everything into the plan: **"let's update the entire bot pwa
plan with the new requirements, and implementation before continuing"**.
The plan was rewritten whole around the rulings, with four backend
increments (P1–P4) placed ahead of the GUI ones — see
[[bot-pwa-plan-rewritten-2026-10-28]].

**Plan status stays `draft`** — D11 closes only on the owner's dated
line; `src/gui`, `src/remote`, `build.rs` untouched. Full landed state:
[[bot-pwa-b2-b3-landed-2026-10-26]]. Lesson shape: a gate names who
approves, not who produces — see
[[plan-brainstorming-before-development]].
