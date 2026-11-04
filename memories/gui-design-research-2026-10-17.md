---
name: gui-design-research-2026-10-17
description: "2026-10-17 design research for the fleet GUI's visual pass — four parallel sweeps (harness landscape, visual craft, agent in-progress rendering, friction literature) distilled into a ranked action list; the brief lives at docs/architecture/gui-design-research-2026-10-17.md; three conversation-surface directions (ledger / cards / stream) were prototyped and await the owner's pick"
type: reference
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

The design-axis research for the GUI's looks pass (the feature-axis
research is [[harness-landscape-converged-spec-2026-10]]; this one is about
how things look and feel, not what they do). Four parallel sweeps, each
verified against primary sources with named URLs:
**docs/architecture/gui-design-research-2026-10-17.md** (commit 14c3453).

**The highest-leverage finding — one renderer, not seven.** The AI SDK's
part-type state machine is now canonical:
`input-streaming → input-available → output-available | output-error`, plus
`approval-requested / approval-responded / output-denied`. Fleet's route
decision, memory recall, skill load and approval request are all
*tool-shaped* events and should ride ONE card renderer with that state
machine, rather than four card types each inventing a loading appearance.

**The conversation-surface moves, ranked:** collapse identical consecutive
calls into one card with a count (memory recall fires repeatedly); a
per-card elapsed timer (a 40-second operation with no duration "reads as
the agent thinking"); approval as an inline card with Zed's four options
(allow once / deny once / always for tool / always for pattern), never a
modal; a guided empty state — a grayed example command inside the composer
accepted with Tab, not "Ask anything..."; a closing summary stamp per turn
(elapsed + recap). For streaming: TTFT of 300–700ms is the snappy
threshold, render a cue the instant the request leaves the client, buffer
appends to a word or short phrase per frame, remove the cursor on
stream-end rather than the last token, buffer partial markdown so a
half-open fence never breaks layout, and never stream raw JSON to the
renderer.

**The craft values, all pinned by the research:** 13–14px base UI text (16px
reads as a landing page); `font-variant-numeric: tabular-nums` on every
numeric readout; dark-mode borders as translucent white at ~10%
(`oklch(1 0 0 / 10%)`) — the single biggest dark-depth lever; a warmer,
less-saturated neutral ramp because a cool blue-gray reads muddy on dark
(Linear's own rebuild); depth from the background step not the shadow;
motion under 300ms with 180ms ease-out entrances, `scale(0.97)` press, and
0ms for anything repeated dozens of times a session; card radius
inheritance (inner elements take card radius minus padding).

**IA:** ten surfaces group into three or four destinations (work /
intelligence / knowledge / system), one flat level, never a second tier,
with a command palette covering all ten so the grouping is a convenience
rather than a maze. NN/g's eighth heuristic governs the whole pass — every
extra unit of information competes with the relevant units.

**The owner's IA question (2026-10-17): combine the destinations and the
palette, or let the user switch between them?** The answer given, and
prototyped: they are **not alternatives and should not be a switch** —
the rail answers *where am I and what is here* (orientation), the palette
answers *get me to X now* (direct access), so they stack as layers with
the combination as the default. The detail that makes the combination
more than two lists: every palette result carries its destination as a
hint, so the palette teaches the grouping instead of duplicating it.
**Exactly one switch is worth offering — the rail's collapsed state** — and
it must be a *mode*, not a fork of the IA: deep in a run the rail is
chrome, navigating it is the map, and collapsing is a keystroke. What to
refuse: grouped-versus-flat as a setting, because a preference that
exists to satisfy two people doubles the surfaces to test and the
research's "constraints liberate" points the other way — pick the
grouping and make it good. The recommendation over a flat palette is the
**palette grouped by the same four destinations**, so one structure
serves both surfaces and nothing is kept in sync. A rail-mode preference
is cheap because `settings.rs` is already the one writer, so it rides the
same path as every other knob and persists across launches; the cost is
one more state to verify at each width, and 56px icon-only labels are
where ambiguity appears, which is why collapsed-rail icons all carry
accessible names. **The owner chose this third approach and it shipped as
cf10d41** — one `GROUPS` declaration in app.js builds the rail and the
palette searches it, ⌘K opens, typing filters across all ten, empty
groups drop their heading, arrows move a cursor, Enter navigates, Esc
closes and comes home, and unbuilt surfaces say "later" on their row
rather than opening to a surprise.

**Terminal-only, do not port:** leader-key sequences, vim mode, box-drawing
borders, width-adaptive diff stacking, scrollback transcript dumps, "?"
shortcut overlays (a palette replaces them), blur-based notifications,
tab-title counters, synchronized-update escapes, cell-buffer diffing,
double-buffered frames, IME preedit positioning. The web analogues that DO
transfer: re-render only the changed subtree, freeze off-screen cards,
throttle to one frame.

**Anti-slop audit list:** purple-blue gradients on dark; neon glow shadows;
pure black on pure white; 16px landing-page text inside a working surface;
triple separation (border + shadow + own background on one card); springy
hover over 300ms; fade-in on page load; emoji as icons; six-plus saturated
accent hues; color used decoratively rather than informationally.

**The three prototyped directions (2026-10-17 — the owner has NOT picked
one yet).** Built as browser mockups in the session visualization
directory, which is ephemeral and outside the repo, so the names and the
axis live only here. The axis is **density: how much of a run is visible
without a click.** **A · the ledger** — one row per event (state dot,
tool, target, duration) grouped under the turn; the whole run legible at
a glance, the densest of the three, reads as a log rather than a
conversation. **B · the cards** — each event a card collapsed to a header
by default; the most readable, closest to ChatGPT/Amp, and the one that
can slide toward clutter as a run lengthens. **C · the stream** — one
activity line per turn with everything else one click deeper; the calmest
and most codex-like, and the riskiest, because if the expansion is
fiddly the intelligence the owner wanted visible goes hidden again. A
third mockup answers the owner's combine-or-switch question with three IA
variants: both layers live (the recommended default), the rail collapsed
to a 56px icon strip with the palette as the only navigation, and the
palette grouped by destination. All three surface directions carry the
same shared pieces — the route walk, recall hits with scores, per-event
durations, approval as an inline card, a closing summary stamp, and a
composer empty state that is a real suggestion accepted with Tab.

**How to apply:** follow the ranked action list in §6 of the doc. The craft
half landed first (commit 9c27573 — warm neutrals, 10% white hairlines,
tabular numerals, 180ms motion, radius inheritance, and it fixed a latent
`var(--font-sans)` that was undefined so two inputs had been rendering in
the browser default font). The IA half landed next (**cf10d41** — the
grouped rail and the grouped palette built from one `GROUPS`
declaration, the owner's chosen third approach). **What remains is
blocked on the owner's conversation-surface direction** — do not start
the one-renderer refactor until they choose between the ledger, the
cards and the stream, because the direction decides the renderer's
shape. Verify craft changes by measuring the rendered palette (accent
coverage %, contrast ratios), not by asserting the CSS parses — and
remember the GUI assets are `include_str!`'d into the binary at compile
time, so a stylesheet change needs a rebuild AND a server restart before
it can be measured; a pass that renders the old palette is a stale
server, not a failed theme.
