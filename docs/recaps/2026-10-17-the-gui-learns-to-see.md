# The GUI learns to see, and a browser becomes a lane

Status: complete
Date: 2026-10-17

Branch: `master`, two sessions interleaved. This one carried the GUI
revamp's looks pass and the visual-deliverable pipeline to a place where
a run can build a deck, render it, screenshot it, and be shown the
picture. A parallel session landed the theme manager, the credentials
vault, the composer's type-ahead, the web-reading pane and the action
card in the same tree; those are named in the commits table and are not
mine to recount.

The arc: the owner opened G5, called it "good for an mvp, but we need to
work on the looks in a different session", and this became that session
— research first, then craft, then the pieces the research ranked.

## The research came before the pixels

Four parallel sweeps — the harness interface landscape, the visual craft
behind tools considered exceptional, how agent UIs render in-progress
states, and the friction literature — landed as
`docs/architecture/gui-design-research-2026-10-17.md` before any
stylesheet changed. That ordering is the whole reason the craft pass
could be one commit instead of twenty: every value it set was a value
the research had pinned down.

Three findings shaped everything after. The most frictionless harnesses
(opencode, Zed, Codex CLI) share three moves — the conversation is the
entire canvas with detail collapsible, prompts are cards you can
re-submit, and the footer carries a live context percentage. The single
highest-leverage change is one tool-card renderer with the AI SDK state
machine rather than four card types that each invent their own loading
appearance. And the research does not support a numeric cap on
destinations — it supports chunking, one flat level, and never a second
tier.

## The craft pass, in measured values

The stylesheet's three-tier token chain paid off exactly as designed: a
theme is the alias tier re-pointed and nothing else. The craft pass was
39 lines of changed values — warm neutrals where the cool blue-gray ramp
read muddy, 10%-white hairlines (the one dark-mode depth trick shadcn's
docs single out), tabular numerals on every numeric readout so a
climbing value does not shuffle the layout, 13–14px base text at the
pro-tool norm, motion at 180ms ease-out with a `scale(0.97)` press, and
inner radii inheriting the card radius minus its padding.

Measured, not asserted: the accent occupies 0.07% of the page against
the research's sub-10% target for a calm look, and primary contrast
reads 15.96:1 light and 15.66:1 dark off the real stylesheet.

It also surfaced two defects in my own G5 work. Two inputs referenced
`var(--font-sans)`, a token the stylesheet never defined — they had been
silently rendering in the browser default font. And the topbar's
destinations and the stage's right-hand card rail had shared the `.rail`
class name, which would have stacked four destinations vertically inside
a 52px bar.

## The surfaces rail and the palette, from one declaration

The owner chose the third of three prototyped directions: a grouped rail
and a palette whose results are grouped by the same destinations, both
built from one `GROUPS` declaration so there is a single structure and
nothing to keep in sync. Four destinations — work, intelligence,
knowledge, system — one flat level beneath each, and ⌘K reaching all ten
so the grouping is a convenience rather than a maze. The one switch is
the rail's collapsed state, remembered across launches: deep in a run
the rail is chrome, navigating it is the map.

The owner's own ruling then became four themes: `console` (warm dark,
the default), `paper` (warm light), `graphite` (cool dark), `contrast`
(the accessibility theme). The theme manager's guard is the part worth
pointing at — it reads the shipped stylesheet at compile time and fails
the build when any theme's text drops under 4.5:1 on any of its three
surfaces. A runtime check cannot beat a compile-time one.

## The right-hand pane, and the boundary it holds

The pane explores the working directory, reads the actual code with line
numbers, and previews a deliverable as rendered markdown. The boundary
is the point: the root is the launch directory, every path is checked
lexically *before* anything touches the disk so a `..` escape is refused
for that reason whether or not it exists, and again on the canonicalized
path so a symlink out is caught. A credential file is refused by name
before the read.

That ordering was wrong on the first attempt and the tests caught it —
my initial version ran the disk read before the checks, so a
nonexistent path failed with "No such file or directory" instead of
"outside the working directory". A refusal that names the wrong reason
is worse than no refusal.

## The visual pipeline, and what it cost to be honest about it

The owner asked for a three-stage research/structure/render pipeline
matching Kimi and Codex. Their blueprint named Python libraries for the
render stage, which collides with the crate contract — one static Rust
binary, no Node runtime, no checkout dependency — so the mapping had to
change while the architecture held.

Stage 2 is `src/deck.rs`: a rigid interchange schema and the consultant
constraints as *checks that refuse* rather than suggestions in a prompt,
because a constraint a model can ignore is decoration. Pyramid (a
headline must assert), MECE (two points sharing 60% of their content
words are one point), bolding (an unmarked bullet is a point a skim
misses), and per-type bounds — an infographic is one node with many
components, a presentation is paced, a poster is one frame read from
across a room.

Stage 3 renders HTML referencing the face's own alias tokens, so a deck
inherits the theme switch for free, and exports PPTX by hand-writing the
ZIP of OOXML parts — stored rather than deflated, no new crate. Text
rides as real text boxes at real positions rather than a rendered
picture, which is the whole point of an export: the owner opens it and
edits it. Verified with the readers rather than asserted: `unzip -t`
passes every part's CRC, all ten XML parts parse, and a slide carries
four text boxes with the bold marker stripped.

The grader closes the loop, and it is deliberately arithmetic. I first
wrote that "fleet has no browser engine, so there's no screenshot for a
vision model to look at," and the owner corrected me: the app *is* a
server that renders the GUI, the engine is the browser, and the owner is
the bridge. So the grader computes what is computable — overlap is
estimated lines against the lines a box holds, balance is one column's
weight against its neighbour's, and contrast is already guarded at build
time by the theme guard — and the `vision` tool grades the screenshot
the owner takes. A deck now renders full-page in the pane, which is what
makes that screenshot one keystroke.

## The browser lane

"Everything is a plugin" turned out to be the running architecture
rather than an aspiration: five vendored lanes already spawn as child
processes speaking MCP. A browser is the sixth. Three tools — navigate,
screenshot, read — over the DevTools Protocol, with a client that is a
plain socket because the handshake is one HTTP upgrade and the frames
after it are a masked length prefix. No Playwright, no websocket library,
no pip install. Fleet's binary is exactly what it was.

Two bugs that only running it could find. The puppeteer cache holds two
Chrome builds and `os.walk` visits them in directory order, so the lane
picked the old one, which segfaults on flags the new one accepts —
candidates are now collected and sorted by version. And my first version
spawned a fresh Chrome per tool call, so a navigate and the screenshot
that followed it looked at different pages; the screenshot came back a
blank white capture, which is only visible if you check the PNG's byte
count rather than trusting the JSON.

## Also fixed

The skills loader did not speak YAML block-scalar indicators. A
description written with `>` — the ordinary way to write a multi-line
one — was read as the literal `>` plus the block, so the model saw a
description beginning with the indicator. Every skill in the library
written that way was affected, `presentation-layout-patterns` among
them; the ones using a plain scalar read fine, which is why nobody
noticed. The fold flag also had to outlive the first continuation line,
or only line one folded.

And the G4-disclosed test flake is fixed: `bridge()` keyed its temp home
on the process id alone, so every test in the binary shared one home and
one test's `remove_dir_all` could delete the directory out from under
another's settings write. It had been failing roughly every other gate
run; six consecutive clean runs followed the fix.

## The gate

Whole, exit codes read directly at every leg: `cargo fmt --all --check`
exit 0, `cargo clippy --all-targets -- -D warnings` clean, `cargo build
--release` clean, `cargo test` 1006 passing across ten binaries.

Two red legs during the work belonged to a parallel session's in-flight
`src/vui/themes.rs` and `src/vault.rs`, not to this session's files —
every diagnostic pointed at theirs, and I left them alone rather than
editing another session's work mid-flight.

## Commits

| Commit | What |
|---|---|
| `1f3f0b6` | the action card — the interface measures a flaw and offers the fix *(parallel)* |
| `452dd60` | the pane reads the web — fetched server-side, reduced to its prose *(parallel)* |
| `342467e` | the browser lane — a real Chrome as the sixth lane |
| `8a46cbd` | the layout grader, and the deck renders full-page |
| `751e1cb` | a run can be shown a picture, routed through the vision workload |
| `966b0cd` | a message can carry an image, and the text path is unchanged |
| `04a702b` | the PPTX export, and the router's vision workload |
| `6db305e` | a folded description reads as one line, not the indicator |
| `d3fff77` | the credentials vault — AES-256-GCM at rest *(parallel)* |
| `e05f8c3` | the /theme command opens the picker instead of writing a dead name |
| `a882524` | four themes, and a guard that fails the build when one goes unreadable *(parallel)* |
| `6cc0b80` | the render stage and the agent tool — a deck a run can build |
| `d6b5c7f` | the composer's command type-ahead, ranked lexically then semantically *(parallel)* |
| `2842d61` | the structure stage — a schema and constraints that refuse |
| `f4e31d5` | the right-hand pane — the working directory beside the surface |
| `e66740f` | the light and dark themes, and the toggle that switches them |
| `cf10d41` | the surfaces rail and the command palette, from one declaration |
| `9c27573` | the craft pass — warm neutrals, 10% white hairlines, tabular numerals |
| `14c3453` | the GUI design research lands — the converged spec, the craft values |

## Live verification

The browser lane driven over stdio: navigate returned example.com's real
title, screenshot wrote a 62KB PNG with 174 distinct colours, read
returned the paragraph text. The first read came back empty because
example.com has dropped its `<h1>` for a `<p>` — my test's selector was
wrong, not the tool.

`fleet lanes` probes six lanes live with the browser reporting `✓ ok`
and its three tools. The GUI's rail builds four destinations and ten
links with `aria-current` on both; the palette filters, moves a cursor
on the arrow keys, and navigates on Enter; the theme picker boots on
console and persists a choice; and the deck tool refuses by naming the
node and the rule.

## Proposed contract-doc updates

Not applied — the owner is the editor of last resort. Proposed text for
each:

### `docs/features/skills.md` — a new "The frontmatter subset" note

> A description written with YAML's `>` is a folded block: the
> continuation lines ride the key as one value joined by spaces, with the
> block's indentation stripped. `|` opens a literal block, which keeps
> its line breaks — and keeps its lines as written, which is this
> parser's existing nested-block dialect rather than full YAML. A plain
> scalar is unchanged.

### `TECHNICAL-DOCUMENTATION.md` § "Module reference" — two rows

> | `deck` | `src/deck.rs` |
> | `browser` | `services/browser/` (lane) |

> The lane table gains a sixth entry: `browser`, role `render`, whose
> child is stdlib Python driving Chrome over the DevTools Protocol. It
> installs no venv; Chrome is a machine resource named by
> `BROWSER_CHROME`.

### `FUNCTIONAL-SPECIFICATIONS.md` — no update needed

The change adds capabilities rather than altering existing user flows;
the existing flows' descriptions still hold.

## What is left

The GUI revamp's remaining increments — G6 sessions and calibration, G7
the boards, G8 settings and the fleet.env defect, G9 the trust trio, G10
hardening and docs — plus the research brief's own ranked list, whose
top item is still unbuilt: one tool-card renderer with the AI SDK state
machine, collapse-identical-with-a-count, per-card elapsed timers, the
guided empty state in the composer, and the closing summary stamp per
turn.

The deck pipeline's remaining piece is the vision loop's automation: the
lane can screenshot and the `vision` tool can grade, but nothing yet
writes the deck's HTML to a file and calls the lane itself. That is the
piece that turns the owner's ⇧⌘4 into an unattended grade.

Two constraints worth remembering. The browser lane holds a persistent
Chrome on a fixed port, so two concurrent fleet processes collide — the
second gets a named refusal rather than a silent second browser, which is
the right failure but a real constraint. And the vision workload's chain
is the compiled-in default: an owner's `routing.json` override of its
candidates governs the run's own route, but the agent does not carry the
preference table, so the vision route reads the default until it does.
