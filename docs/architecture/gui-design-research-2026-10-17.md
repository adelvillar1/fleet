# GUI design research: what the best harness UIs actually do

Research done 2026-10-17, four parallel sweeps: the harness interface
landscape (Codex, Claude Code, Cursor, Amp, Zed, opencode, Gemini CLI,
Crush, Cline, Aider, Copilot, Jules, Devin), the visual craft behind
tools considered exceptional (Geist, Primer, Radix, shadcn, Linear), the
rendering of agent in-progress states, and the friction/usability
literature (NN/g, Raycast manifesto, calm technology). Sources are named
inline; claims marked **[synth]** are this document's synthesis rather
than a quoted finding.

This is the brief for the GUI's visual pass. It is deliberately not an
inspiration deck — every item below is either a specific value to set or
a specific thing to delete.

## 1. The converged spec, and where fleet stands

Every serious agent UI now ships the same skeleton: the transcript as
the spine, collapsible tool-call cards with diffs, a permission gate
before any mutation, a status line with a context meter, a session list,
git-backed checkpoints, a plan mode, a model switcher, slash commands,
and @-file mentions. Fleet's GUI already has the transcript, the cards,
the inline approval seat, the context meter and spend line in the header,
the router panel, and the delegation tree.

The three surfaces with no peer are the router panel, the delegation
graph, and the memory/skills panels. Those are fleet's differentiators
and they are also the ones most at risk of reading as "a dashboard
bolted onto a chat" — which is exactly the Devin failure mode (IDE +
Browser + Shell + Side Chat + Progress tabs is five surfaces per
session). The design job is not to add; it is to make the deep panels
feel like opencode's calm rather than Devin's sprawl.

The most frictionless harnesses found were opencode, Zed and Codex CLI,
and the moves are specific: the conversation is the entire canvas with
detail collapsible on demand; prompts are clickable cards you can
re-submit with an adjustment; the status footer carries a live context
percentage; diffs are inspectable as they appear rather than after the
fact. Codex's bordered header box plus a starter command list plus a `›`
prompt is the best first-run empty state in the field — better than
"Ask anything...", which by itself "paralyzes most people."

## 2. Craft: the specific values

**Type.** 13–14px is the pro-tool norm for base UI text, not 16px — a
16px body with wide gaps reads as a landing page. Geist separates scales
per role (headings, buttons, labels, copy) with labels at a tighter
line-height than prose. Set `font-variant-numeric: tabular-nums` on every
numeric readout, and reserve monospace for code, ids and numbers only.

**Color, and the one trick that matters most.** In dark mode, borders
should be translucent white at about 10% (`oklch(1 0 0 / 10%)`) rather
than solid gray — shadcn's docs are explicit that this single choice
does more for dark-mode depth than any other. Second: the neutral ramp
should be warmer and less saturated than a cool blue-gray, because a
cool gray ramp on a dark background reads as *muddy*; Linear rebuilt
their palette specifically to escape that. Never pure black on pure
white. Geist's gray scale is semantic by role (background / hover /
border / secondary text / primary text), which is the same shape as our
three-tier chain already is — the alias tier is where the roles live.

**Separation.** Delete roughly half the hairlines. Linear found
separators had "quietly proliferated… sometimes appearing without clear
reason" and fixed it by rounding edges and softening contrast so
structure is "felt not seen." Separate by hierarchy of emphasis —
spacing plus a background step — and use a hairline only where grouping
genuinely changes. In dark mode, depth comes from the background step,
not from shadows.

**Motion.** Everything under 300ms; 180ms reads as responsive and 400ms
reads as slow. Entrances and exits are ease-out. Press feedback is
`scale(0.97)` on `:active`, and entrance scale starts at 0.9 or above,
never 0. Deliberately do not animate anything a user triggers dozens of
times a session — repeat transitions go to 0ms. Tooltips are 125ms.

**Cards.** 12px radius with inner elements inheriting `radius −
padding`; background one gray step above the page; padding 16–24px; no
shadow in dark mode. A card with a border, a shadow *and* its own
background is triple separation and reads as generic.

**Anti-patterns to audit against [synth].** Purple-blue gradients on
dark; neon glow shadows; 16px landing-page text inside a working
surface; every card triple-separated; springy hover over 300ms; fade-in
on page load; emoji as icons; six-plus saturated accent hues; color used
decoratively rather than informationally.

## 3. The conversation surface: one renderer, not seven

The single highest-leverage change. The AI SDK's part-type state machine
is now the canonical shape: `input-streaming → input-available →
output-available | output-error`, plus `approval-requested /
approval-responded / output-denied`. Fleet's route decision, memory
recall, skill load and approval request are all *tool-shaped* events and
should ride one renderer with that state machine, rather than four card
types that each invent their own loading appearance.

Then the specific moves, in order of impact:

- **Collapse identical consecutive calls into one card with a count.**
  Memory recall fires repeatedly; five near-identical recall cards is
  noise, one card reading "5 recalls" is information.
- **A per-card elapsed timer.** A 40-second operation with no duration
  "reads as the agent thinking" — the timer is what makes a slow step
  legible instead of worrying.
- **Approval stays an inline card, never a modal**, and carries the
  four options Zed settled on: allow once, deny once, always for this
  tool, always for this pattern (the last only when a safe pattern is
  derivable). We already have the seat; this is a wording and options
  change.
- **A guided empty state**: a grayed-out example command inside the
  composer itself, accepted with Tab — not a placeholder string.
- **A closing summary stamp** per turn: elapsed time plus a recap of
  what happened. ChatGPT's "Worked for 2m 30s" plus an action recap.
- **Alive versus laggy.** Render a transient cue the instant the request
  leaves the client rather than waiting for the first token; buffer
  appends to a word or short phrase per frame; keep the cursor at the
  trailing edge and remove it on stream-end rather than on the last
  token; buffer partial markdown so a half-open code fence never breaks
  layout; never stream raw JSON to the renderer. Target time-to-first-
  token is 300–700ms, which converts wait time into reading time.
- **Web analogues of the terminal tricks that do transfer**: only
  re-render the changed subtree, freeze off-screen cards, throttle to one
  frame.

## 4. Information architecture: ten surfaces, four destinations

The research does not support a numeric cap on destinations — Miller's
"seven" is not magical and Cowan's reanalysis lands nearer four — but it
does support chunking, one flat level, and never a second tier. So the
ten hash addresses should group into three or four destinations: **work**
(chat, sessions), **intelligence** (router, delegation), **knowledge**
(memory, skills, second brain), and **system** (settings, calibration,
tower). One flat level beneath each, and a command palette that covers
all ten so the grouping is a convenience rather than a maze.

The palette is also the answer to the owner's "nothing to configure
first": every navigation destination plus every action, reachable in one
keystroke, with settings deferred to it rather than shown on first run.
NN/g's eighth heuristic is the governing constraint on the whole pass —
every extra unit of information in an interface competes with the
relevant units — and calm technology's version is to require the
smallest possible amount of attention and to make use of the periphery.

## 5. What not to port

Terminal-only mechanics that do not translate to a browser face:
leader-key sequences, vim mode, box-drawing borders, width-adaptive diff
stacking, scrollback-style transcript dumps, "?" shortcut overlays (a
palette replaces them), blur-based notifications, terminal tab-title
counters, synchronized-update escape sequences, cell-buffer diffing,
double-buffered frames, IME preedit positioning.

Also not to port, on the own-home ruling and the crate contract: any
marketplace, any cross-harness sync, any second store.

## 6. The ranked action list for the looks pass

1. One tool-card renderer with the AI SDK state machine, shared by route
   / memory / skills / approval, with collapse-identical and a per-card
   timer.
2. The guided empty state in the composer, and the closing summary
   stamp per turn.
3. Craft pass on the token values: 13–14px base, tabular numerals on
   every readout, 10%-white dark borders, a warmer neutral ramp, half
   the hairlines deleted, depth from background steps not shadows.
4. Motion budget: 180ms ease-out entrances, `scale(0.97)` press, 0ms for
   repeats.
5. IA: group the ten addresses into four destinations, one flat level,
   and a command palette covering all ten.
6. Anti-slop audit against the list in §2.
