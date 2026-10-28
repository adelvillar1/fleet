# fleet bot — the B0 mockups

The four static screens the bot-PWA plan gates its GUI work on
(`docs/plans/2026-10-20-bot-pwa.md`, decision **D8**). Nothing in
`src/gui/assets/` changes until the owner's dated approval line at the
bottom of this file exists.

Open any file directly in a browser — no server, no data, no build:

| File | Screen | States it carries |
|---|---|---|
| `inbox.html` | the launch surface | everyone · one persona opened · empty · one row · filter open · a long last line and a refused tail |
| `conversation.html` | the stream, the goal, the personas | live run · destructive seat · two personas · add a persona · failed run · stopped run |
| `dispatch.html` | new conversation | empty · typed |
| `install.html` | install and unreachable | in the browser (the iOS prompt) · installed standalone · offline retry — plus the icon pair |

Each file has its own state switcher, a width control (430 → 900, slider
or presets), and a theme control (paper / console / graphite), so one
file covers the whole review matrix. The state and width also ride the
URL — `inbox.html?state=filter&w=900&theme=console` opens straight to a
particular frame, which is how the screenshots for this review were
taken.

## Widths

| Width | What it stands for | What must hold |
|---|---|---|
| **430 px** | iPhone 15 Pro Max logical width — the design target | one column, 18 px gutters, 44 px minimum tap targets, the composer above the keyboard |
| **900 px** | tablet / landscape container | the layout reflows, it does not stretch: the inbox becomes list + detail, sheets become centred dialogs, the conversation centres at 680 px |

The reflow is CSS container queries on the phone frame, not media
queries — the mockup *is* the app shell, so the app's own width decides
the layout. B1 should keep that property.

## The design language

Every token is copied verbatim from `src/gui/assets/app.css`: page
`rgb(246,244,239)`, surface `rgb(252,251,248)`, ink `rgb(24,26,29)`,
accent `rgb(47,95,208)`, ok/warn/down, the serif display face, the mono
face, radii 7 / 10 / 14 / 20, `--ease: cubic-bezier(.2,0,0,1)`, 120 ms
and 180 ms. The three themes are the console's own (paper, console,
graphite) with their real dark values.

No second design language: the seat card, the stamp badge, the hairline,
the mood of the thing are the console's. What is new is only the
information architecture a phone needs.

## The design inversion: the assistant is the axis

The first pass of these mockups organized the inbox by **work** — sessions
grouped by state, a workspace path in every row — and the owner rejected
it in one line: *"this is just rehashing what fleet does, which is work
centric; the whole bot idea is assistant centric."* That was right, and it
is the failure the plan itself warns about: a surface that ships the
console's frame with a phone's chrome.

The mockups now invert it:

- **The unit is a persona, not a session.** Six of them, and three are
  domains rather than jobs: `scholar` (knowledge work), `presenter`
  (presentations), `advocate` (job hunting) — beside `tower` (runs the
  work), `auditor` (checks what changed) and `clerk` (keeps the ledger).
  Every conversation belongs to one, the list is grouped by them, and the
  rail filters to one in a tap.
- **The row leads with the persona's mark, then what it said.** The
  workspace has left the list entirely; it appears once in the
  conversation's own meta line (`the gate on the fix · in ~/Projects/fleet`),
  because where the work happens is not who you are talking to.
- **The conversation header is the persona** — its mark, its name, its
  role — and the conversation's title is stated once above the stream.
- **Dispatch asks "who takes it" first**, then the brief, then the folder.
- **Urgency is an entry point, not the ordering.** What needs you sits in
  a compact strip above the personas: three one-line rows, tappable, never
  the screen's whole structure. One of them is a CV waiting for a read —
  the same mechanism serving work that is not code.

The persona is not a mascot: opening one shows what it actually is — the
role, the tools it admits, the skills it loads, the memory shapes it
recalls — because fleet already carries that primitive (see below).

### Identity without portfolios

Each persona carries a mark: its serif initial on a tint drawn from the
console's own lane palette (`--lane-0…5`, which already has a value per
theme). `tower` keeps the accent as the lead; `scholar` is teal,
`presenter` magenta, `advocate` lime, `auditor` amber, `clerk` violet.

That is deliberately *not* a portrait, for three reasons that are
structural rather than aesthetic:

1. **Personas are user-authored.** A role is `<home>/roles/<name>.md`,
   created at will, with any name. A design that needs an image per
   persona stops working the moment a new role file appears; a derived
   mark works for one role or forty.
2. **A face would misrepresent what it names.** The persona card says
   what a persona is — tools, skills, recall shapes, prompt overlay. A
   portrait turns that contract into a mascot, and generated faces are
   the most reliable tell of a designed-by-template product.
3. **It costs no new design language.** The lane palette is the house's
   own, with dark-theme values already chosen; the mark is two CSS lines
   and no asset.

**Readable in every theme, by measurement.** Two rules make the mark legible
rather than merely tinted: the disc is **opaque** (`12% of the tint mixed
with the theme's surface`), so the ratio never depends on which row the
mark lands in; and the glyph's ink is **its own value per theme**, not the
tint itself — a 12 px serif letter needs more than a hue. Measured
letter-on-disc contrast:

| persona | paper | console | graphite |
|---|---|---|---|
| tower | 5.24 | 5.27 | 5.45 |
| scholar | 5.21 | 5.04 | 5.45 |
| presenter | 5.24 | 5.70 | 5.90 |
| advocate | 5.29 | 9.27 | 8.32 |
| auditor | 5.21 | 8.53 | 8.05 |
| clerk | 5.21 | 5.57 | 5.43 |

Every cell clears 5.2:1, which is AA with margin for a glyph this size.
The original tint (the hue used as ink on a translucent wash) failed in
paper — the teal 4.49, the amber 4.19 — and drifted with whatever sat
behind it.

Recognition gets three channels — shape, colour, name — so colour is
never the only signal. The research agrees on *where* an avatar pays:
Cue puts agent avatars in the list; every mainstream assistant keeps the
transcript avatar-free. So the mark appears in the row, the rail, the
conversation header and the persona page, and never beside a message.

If more warmth is wanted, the cheapest honest step is an optional
`mark:` key in the role's frontmatter naming a glyph from a closed set —
the roles editor already preserves unknown keys, so it needs no schema
change and no art. Absent, the initial stands.

## Personas are roles, and roles name their skills

A persona is not a drawing: it is the skill list it loads. The canonical
folder `~/.agents/skills` holds **453 skills**, and the three domains the
owner asked for are all covered there today:

| domain | skills that exist in `~/.agents/skills` |
|---|---|
| knowledge work | `llm-wiki`, `arxiv`, `grounded-citations`, `documentation-consolidation`, `documentation-refresh`, `meeting-action-items`, `weekly-review-planning`, `data-triage`, `obsidian`, `notion` |
| presentations | `slide-deck`, `presentation-layout-patterns`, `branded-presentations`, `powerpoint`, `theme-factory`, `data-visualization`, `kimi-slides`, `slideshow` |
| job hunting | `jobhunt-deliverables`, `cv-tailoring`, `freelance-profile-building`, `cold-email-outreach-deliverability`, `email-outreach-campaigns` |

So no new skills are needed for the roster — the mapping is. fleet already
has it: a role is a markdown file whose frontmatter names the skills it
loads (`src/roles.rs`, `Role::skills`), and the roles editor writes the
same frontmatter and preserves unknown keys. The three domain personas,
as they would be authored:

```markdown
---
name: scholar
description: Knowledge work — reads, cites, and says what it did not read.
tools: bash, read_file, write_file
skills: llm-wiki, arxiv, grounded-citations, documentation-consolidation
shapes: resolution, convention
---
Read before you write. Every claim that leaves this role carries its
source or says it has none.
```

`presenter` names `slide-deck`, `presentation-layout-patterns`,
`branded-presentations`, `data-visualization`; `advocate` names
`cv-tailoring`, `jobhunt-deliverables`, `cold-email-outreach-deliverability`,
`freelance-profile-building`.

**What is missing is not the skills — it is the files.** `~/.fleet/roles/`
does not exist on this machine yet, so none of these personas can be
loaded; and a session still cannot name one (see below). Authoring the
three role files is a ten-minute job once the roster is ruled; making the
inbox group by persona needs the session binding.

## One goal, several personas

A conversation can hold more than one persona, so it needs something that
is not a persona to be *about*: the **goal**, taken from the owner's own
words, pinned between the header and the stream where it cannot scroll
away. Everything else follows from it.

- **Adding a persona never changes the goal.** The sheet restates the goal
  above the people it offers, and says so in a line. A new goal is a new
  conversation — not a side effect of a join.
- **A join is a row in the log**, not a silent change of voice: *auditor
  joined · checks what changed*, with a time. The transcript then carries
  two bylines, and the header shows overlapping marks plus a `+`.
- **The policy is binary, and it is the owner's**: **non-destructive work
  runs without asking** — reads, searches, appends, edits with a diff. The
  transcript shows the append that wrote six ledger lines and never asked.
  **Destructive work is held every time** and flagged: an overwrite is the
  same file and the same goal, and it still stops, because it discards
  what is there.

A destructive seat never looks like the calm one: red hairline, red
`⚠ destructive` badge, the lines that will go, a filled red action, and
**no always-allow** — a standing permission for an irreversible act is a
footgun, and the note says the append did not ask, so the difference is
visible. Sending outward counts as destructive too: the CV in the inbox
strip is held because a sent CV cannot be unsent.

What this needs from the backend, none of it built:

| fact | why it is needed |
|---|---|
| `goal` on the session | written from the owner's first instruction, never rewritten by a join; shown as chrome |
| `personas` on the session (a set; the first is the lead) | the inbox groups by persona and shows a stack for a shared one |
| a join record in the log | model-visible ⟺ logged; a persona's participation must be reconstructable |
| a destructive class on the approvals gate | fleet already has the always-ask class; the surface maps "destructive" to it and lets everything else through |

## The persona is `roles` — and it has no session binding yet

This is the one real dependency, and it should be settled before B1:

- **The primitive exists.** `src/roles.rs` defines `Role { name,
  description, tools, skills, shapes, overlay }`, read from
  `<home>/roles/<name>.md` and rendered by `fleet roles`. A role narrows
  what a delegation may do and prefixes its prompt — a persona in all but
  name.
- **The binding does not.** A session log records its `kind` (scheduled,
  delegated, owner) and its channel; it does **not** record a role. Today
  a persona only exists *inside* a delegation, as an argument to the
  `subagent` tool.
- **The plan deferred it.** D7 says bot profiles — "a named identity with
  a role, routing preference, and tool scope" — are the next plan, and
  "this one makes the surface they will live on". The owner's ruling above
  pulls that premise into the surface: an inbox grouped by persona cannot
  be built without a persona on the session.

So the smallest honest reading: this surface needs a **`persona` field on
the session** (the role's name, written at creation, and named in the
handback record if it changes), plus the inbox's grouping. That is a new
increment's work — not B0's, and not something these mockups may decide.

## What the research changed

The brief was "best in class, from deep research", so the decisions below
are tied to what shipping agent interfaces actually do — and, where they
fail, to the failure.

**The inbox is the launch surface, and it leads with who, not what.**
Cursor's cloud-agent app opens on an agent inbox and is cache-first, so the
list paints before sync; reaching a session in a drawer-based assistant
(Claude: hamburger → scroll Pinned or Recents → tap) costs three taps. The
one shipping app that is genuinely bot-first — Cue by Manus — puts a
horizontal row of agent avatars above a flat list, so you pick the one you
are talking to before you read the work. That row is the persona rail here.
*(sources: https://cursor.com/docs/cloud-agent/mobile ;
https://apps.apple.com/us/app/cue-by-manus/id6808613358 ;
https://github.com/bryanrg22/claude-ios-ui)*

**State lives on the row, not in the chat — but it is not the row's
subject.** Both Cursor and Claude Code on iOS surface agent status at
session-list level (`Working`, `Needs input`, `Passed`, `Merged`). We keep
the four words — seat, failed, running, finished — as a small dot beside
the age, because on an assistant-first screen the persona and what it said
are the row's subject and the state is an attribute. Cue, which leads with
agents, under-signals state for exactly this reason; the dot is the
correction. *(sources: https://apps.apple.com/us/app/cursor/id6767085653 ;
the Claude iOS recreation above)*

**Status is a dot plus a word, never colour alone.** The seat row carries
a dot, the accent rail, the "needs you" tag and the header count; a
finished row is a hollow ring. Colour-blind review and a greyscale
screenshot both still read.

**The working chip carries its own Stop.** Manus keeps a floating "is
working 0:08" chip above the composer and explicitly keeps the composer
open for mid-run steering — but its chip has no cancel, which its own
teardown flags as a weakness. Ours has Stop beside the clock.
*(source: https://aiuxplayground.com/teardowns/manus/output)*

**Approval: once / always / deny, with the safe choice obvious.** The
failure mode documented across agent products is approval fatigue —
per-action modals that train people to blanket-Allow — and Perplexity's
teardown names the specific defect: the default-safe choice is not
highlighted, so "deny" and "allow" look equally weighted. Here Allow once
is first and filled, Always states its scope in words, and Deny is quiet
rather than red. Research also recommends batching more than three
same-tier approvals; that is named as a follow-up, not built in v1.
*(sources: https://setproduct.com/blog/ai-agent-ui-design-patterns ;
https://aiuxplayground.com/teardowns/perplexity/human-loop ;
https://code.claude.com/docs/en/permissions)*

**The seat is answered once — and the card says so.** A real user report
against Claude's mobile app: a permission dialog for work already approved
on the desktop. fleet's bridge already guarantees one answer; the card
states it ("whoever answers first wins, and the second is refused") so the
contract is visible rather than discovered.

**No avatars in the transcript.** All five major assistants carry identity
by alignment and bubble fill; the assistant is full-width text. Ours adds
only a small serif *f* byline with a relative time, which is what makes a
long stream scannable on a phone.

**17 px body, 44 px targets, 18 px gutters, 68 px rows.** Apple's body size
is 17 pt and its minimum control is 44×44 pt; the row ladder and gutter
come from the same place. The composer's field is 16 px, never less —
anything smaller triggers Safari's focus zoom.
*(sources: https://developer.apple.com/design/human-interface-guidelines/typography ;
https://developer.apple.com/design/human-interface-guidelines/accessibility ;
https://css-tricks.com/16px-or-larger-text-prevents-ios-form-zoom/)*

**Sheets, not modals.** Dispatch and the filter are bottom sheets — a
scoped task, one at a time — which is the guidance for both iOS and
Material, and it leaves the inbox visible behind the task. At 900 px they
become centred dialogs, which is the same idea at tablet scale.
*(source: https://developer.apple.com/design/human-interface-guidelines/sheets)*

**iOS install is manual, so the app asks once, in its own voice.** There is
no install API on iOS; the flow is Share → Add to Home Screen, and since
Safari 26 every home-screen site opens as a web app with no installability
requirements. The browser-mode screen shows that coach mark and labels the
browser's own chrome as not ours.
*(sources: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable ;
https://webkit.org/blog/17333/webkit-in-safari-26-0/)*

**The unreachable page is the worker's, and it says what is true.** The
offline fallback is served only for a failed navigation, and only the two
versioned assets are ever cached — so the retry page can honestly say the
run continues on the machine and no session data is on the phone.
*(source: https://web.dev/articles/offline-fallback-page)*

## Carried into B4 / B5 as implementation notes

These are findings that do not change a mockup but must not be lost when
the app is written:

- **Use `100dvh`, not `100vh`** for the shell, and keep the composer clear
  of the keyboard with `env(safe-area-inset-bottom)` plus the visual
  viewport — `dvh` does not shrink for the keyboard, and `position: fixed`
  follows the *layout* viewport. *(https://web.dev/blog/viewport-units)*
- **Pin the stream geometrically, not with a keyboard handler.** The
  working reference: detach when the user scrolls up, re-attach inside a
  near-bottom zone, reserve the streaming turn's height, and batch token
  paints per frame — per-token DOM replacement makes Safari clamp the
  scroller repeatedly. *(https://github.com/huggingface/chat-ui/pull/2525)*
- **`overscroll-behavior: contain` on the stream scroller** so the phone's
  pull-to-refresh cannot swallow a scroll gesture.
- **Gate `:hover` behind `@media (hover: hover)`** — iOS keeps a hover state
  stuck after a tap.
- **`prefers-reduced-motion: reduce` removes the pulses and every
  transition**, which the mockups already honour.
- **The worker caches exactly `/bot.css?v=` and `/bot.js?v=`** and bypasses
  `/api/`, `/events`, `/pair` by path — the console's worker's boundary,
  restated because streaming responses must never pass through a cache.
- **The icon pair** is the serif *f* on the accent gradient; keep the mark
  inside the maskable 40% safe circle (the third tile in `install.html`
  shows the guide).

## Verified, not just drawn

Every state was rendered headless at both widths in all three themes — 90
page loads (15 file-states × 2 widths × 3 themes) — and measured:

- no horizontal overflow at 430 or 900, and the frame is exactly the width
  asked for;
- every control inside the phone is at least 44 × 44 px;
- every editable field is at least 16 px (the iOS focus-zoom floor);
- every text element reaches WCAG AA (4.5:1, or 3:1 at large sizes) against
  the surface actually behind it, in paper, console and graphite — including
  the persona marks, chips, role lines and scope lines.

Two cascade traps surfaced in that pass, both worth carrying into B1:

- **Chromium paints its native button face** (`#efefef`) behind any
  `<button>` whose background is transparent. Without `appearance: none`
  every chevron, overflow dot and list row wears a grey the design never
  asked for — and a computed-style check reports that grey as real.
- **A custom property that is a `color-mix()` of theme tokens, declared at
  `:root`, freezes against the light palette** and inherits the resolved
  value into every theme. The derived tints (`--seat-bg`, `--bubble-bg`)
  have to be declared where the theme's tokens live, or dark themes get
  light-lavender cards with the dark theme's light text on them.

## Open questions for the owner

1. **The persona binding** (the big one). A session has no persona today —
   see "The persona is `roles`" above. Is the smallest step a `persona`
   field written at session creation, defaulting to the leading role? And
   when a persona's role file is edited, do existing conversations keep the
   role they ran under, or follow the file?
2. **The names.** The mockups say `tower`, `scholar`, `presenter`,
   `advocate`, `auditor`, `clerk` — `auditor` is lifted from the role
   fixture in `src/roles.rs`, the domains are the owner's, the rest are
   invented for the brief. fleet's own vocabulary has *role*, the plan
   says *bot profile*, and this README says *persona*; one of those should
   win before B1.
3. **A glyph per persona?** See "Identity without portfolios": an optional
   `mark:` frontmatter key (closed glyph set, initial when absent) is the
   only way to go further without shipping images. Say whether it is worth
   the schema note.
4. **Who owns a shared conversation in the list?** The mockups file it
   under the persona who started it, with the second mark shown. The
   alternative is filing it under each participant (the count then stops
   being a count of conversations). Say which.
5. **A goal change.** If you redirect mid-run — "forget the ledger, fix the
   flaky test" — is that a new conversation, or an explicit goal-change
   event in the log? The mockups assume the former, because a goal you can
   edit silently is not an invariant.
6. **Does the persona own a model?** A role narrows tools, skills, shapes
   and its prompt today; routing is the router's. If `scholar` should run
   on a reading-shaped model and `tower` on the `hard` chain, that is the
   next plan's "routing preference" — the surface should know whether to
   show it.
7. **Archived sessions in the inbox.** The archive verb landed
   (`9f877f4`, 2026-10-27) after this plan was drafted; `work_state` now
   returns `archived` and `handed back`. The mockup proposes an **Archived
   filter** and keeps archived rows out of the default list. The current
   B2 mapping has no `archived` arm (it falls into `finished`), so this is
   a real decision, not a cosmetic one.
8. **Search.** The header carries a search control, which the plan does not
   ask for. With a 50-row bound the list can outgrow a thumb; say whether
   it belongs in v1.
9. **Two-pane at 900 px.** The inbox shows list + detail at tablet width.
   Accept, or keep single-column everywhere and let the frame letterbox?
10. **Background runs.** `sessions_read` distinguishes scheduled and
   delegated runs from the owner's own conversations. A cron job has no
   persona — should it get one (the job's role), or appear under a quiet
   "background" tag so a scheduled run never reads as someone waiting on
   you?
11. **Row density.** 68 px rows fit about nine rows per screen. Denser
   (56 px, preview on one line) trades reading comfort for overview.

## Approval

The gate closes with a dated line here, written by the owner. B1 does not
start before it.

- [ ] Mockups approved — __________________________ (date)
