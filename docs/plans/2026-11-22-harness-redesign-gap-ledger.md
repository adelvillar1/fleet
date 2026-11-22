# The harness redesign — the plan gap ledger

The goal's rule: a gap in the plan is never closed by an educated guess — it
is researched, the replan is written here, matched to the plan row it amends,
and closed by a commit. One row per gap; a gap without a plan row it amends
is a defect in this ledger, not in the code.

| # | Gap found | Plan row it amends | What the research said | The replan (the decision, recorded) | Closed by |
|---|-----------|--------------------|------------------------|-------------------------------------|-----------|
| G1 | D1 says "OKLCH token chain" | D1 | `themes.rs` parses `rgb(r, g, b)` literals only — "a `color-mix` or a `var()` is not a colour this parser can measure, and guessing would make the guard a lie". oklch() values would be unmeasurable by the AA guard, and the mockup's headless capture cannot paint oklch/var on SVG fill at all. | The tier values ship as `rgb()` literals derived from the OKLCH designs (the conversions recorded in this ledger's palette notes); the OKLCH forms stay the design record in the mockup CSS and here. The design is OKLCH-derived; the ship format is what the guard can measure. | ✅ Pass 1 |
| G2 | D1/A1 say "six tints re-derived, AA-guarded" | D1, A1 | The guard measured each tint against the theme's **page ink** (`themes.rs` test `every_theme_carries_six_readable_persona_tints`), but the approved design (the owner: "the avatars look good") is bright tints with dark labels — light page-ink on a bright tint is ~1.8:1. The disc's actual label token is `--on-tone`, which P7 created for exactly "the label on a tone-filled bead", and the guard never measured it against tints. | The tints go bright in every theme; the disc's label token `--tint-ink` moves to `var(--on-tone)`; the guard's persona test measures `--on-tone` against each tint instead of page ink, and the test's doc says so. `--on-tone` re-points per theme (dark in all four — the tints stay bright). | ✅ Pass 1 |
| G3 | D1 names Space Grotesk / Inter / mono | D1 | Already shipped: `vendor/fonts/` carries all three as variable woff2 (OFL, VENDOR.json), self-contained since an earlier pass. | No work. The type system is already the design-canvas stack. | already true |
| G4 | D1 says "console and contrast retained" | D1 | **Amended by the owner's ruling 2026-11-22: "I like the color design you chose, so don't reuse none of the current themes."** No colour from the current sets survives — all four themes re-derive. | All four theme blocks carry the Indigo Studio palette, pre-verified to zero AA failures by the guard's own math before any CSS lands (the pre-verification script and its four iteration findings are recorded below the palette tables). The parser test's pinned console/contrast ink values update to the new shipped values in the same commit. | Pass 1 |
| G5 | D6 "don't break the app" — unknown whether the face harness pins colors | D6 | Researched before the token change: `app.js` carries **zero** color literals; the harness pins token **names** (`var(--dsw-alias-ok)` in the tone-chip law) and behavior, never values; testcensus's no-hex guard scopes hex only, and the palette ships as rgb literals. The "harbor" strings in the harness are a project-path fixture, not a theme pin. | No color pin existed to update; the harness held at its baseline through the re-point — 1876 checks / 278 cases. | ✅ Pass 1 |
| G6 | A1 "vendor the component" — which artifact, what license text | A1 | BoopFace releases ship a zip (`boopface-x.y.z.zip`) containing the component; lite build ~45 KB, full ~56 KB without skins. MIT + identity carve-out (the name/character is not transferable). | Vendor the release zip's component into `src/gui/assets/vendor/` beside a VENDOR notice with the upstream link, version, license text, and the carve-out; fleet's wrapper is `persona-face`; skins stay off (D8). | Avatar pass |
| G7 | D2 "the drawer is the one reading surface" vs the existing pane | D2 | The face already has `.pane`/`.pane-tabs` (440px, drag 300–720) serving four mixed duties across surfaces. The drawer rework needs a per-surface inventory of pane users before any surgery — the plan's per-surface row for each lens. | Pass 2 opens with the inventory (grep every `openPane|pane-body|pane-tabs` call site), then the drawer grammar lands per surface with its harness pins. No guess. | Pass 2 |
| G8 | D1 "cool graphite night as the default" | D1 | The shipped default IS night (`:root` carries the night values); app.css's header comment claims "the default: warm paper" — a stale doc comment, not a state. | Fix the stale comment in the same commit as the re-point; the default does not move. | ✅ Pass 1 |
| G10 | The reference tier's names looked renamable — "don't reuse none of the current themes" | D1, G4 | A truncated grep hid the truth: 13 reference-tier names are load-bearing (60 component reads — `paper-*`, `ink-*`, `deepseek-*`, `graph-*`, the on-ground signal constants, `contrast-page/ink`). Renaming 60 call sites is refactoring, not re-derivation, and belongs to a later pass. | The NAMES stay (they are provenance labels, not colours); every VALUE re-derives from the new palette (`paper-*` carries the cool-paper day ground it always meant; `deepseek-*` the indigo accent family). A full-declaration audit (`for t in reads; do grep declaration`) proves every read resolves after the surgery. | ✅ Pass 1 |
| G11 | D2 frames the drawer as *replacing* the pane | D2 | The Pass 2 inventory (16 `openPane` call sites, 39 pane references) found the drawer duties ALREADY live in the pane: `openDrawer` delegates into it ("the fixed aside#drawer remains only as the narrow-screen fallback"), and the pane carried surface-dependent tabs (run / code / preview / context) — not the four-tab read/act/connections/made-here grammar. Three call sites auto-open it (goal-started run, set-started run, desk arrival) — flagged as U13 candidates. | **Increment 2a (landed):** the tab strip is the four canonical tabs — read · act · connections · made here — with the five read bodies (context/detail/files/preview/web) consolidated under ONE read tab behind a mode switch (document · code · preview · web); the focus chooses the mode, the owner switches it, legacy persisted values map onto the canonical tab + mode so a reload lands where it meant to. `run`→act, `work`→connections, `receipts`→made here. Covers Conversations AND Board (Board's rows open objects through `openDrawer` → the read tab's document mode). **The U13 audit verdict: the three flagged auto-opens are owner-press consequences, not lens arrivals** — the goal-start and set-start opens answer the owner's own send press ("the owner just asked to watch it work"), and the desk arrival answers the brief the owner just sent; U13 bans ARRIVAL-triggered opens, which the relevance pass already removed. No code change; the verdict is the record. Harness re-pinned the same turn: 45 tab references rewritten to the canonical ids/modes, **1876 checks / 278 cases — the exact baseline held.** **Increment 2b (landed):** the connections list carries the goal edge — the session's standing objective (`face.goal`/`face.goalDone`, state the face already holds, nothing inferred) leads the list, and the tab's availability widens to a goal with no delegations; the narrow `aside#drawer` fallback is retired (its branch gated on element *existence* — always true — so it was unreachable dead code at every width): the aside, its scrim, its close listener and the el entries are gone, and the six harness fallback-reads simplify to the pane. Pinned the same turn: the retirement's structural check (no `id="drawer"`/`id="scrim"` in the served HTML) and a GOAL case (a goal with no delegation shows the connections tab and leads with the goal text) — **1879 checks / 279 cases, the baseline grew by the new pins.** Remaining in G11: none — the drawer grammar is whole; the per-surface re-skins are Pass 3's rows. | ✅ Pass 2a + 2b |
| G12 | Hygiene observation, 2026-11-22: two load-sensitive tests flaked under full-suite parallel runs — `eval::the_corpus_runs_green_and_lands_a_queryable_row` (run 1) and `improve::race_tests::a_scratch_patch_races_to_a_twin_refusal` (run 2) — each passing alone, the third full run green at 2594/0. **The failure output was not captured** (the gate pipeline grepped it away), so the KI ledger's evidence bar is unmet. | D6, hard rule 10 | The class matches KI-574/KI-579 (ambient-state races; the test-env-isolation plan exists). Neither subsystem is touched by the redesign's changes. | If either flakes again: capture the full failure output first, then file the KI row with the assertion evidence and the parallelism context. **Captured on recurrence (the third gate run):** the improve race test failed ALONE with the panic verbatim (`src/improve.rs:2988` — "the corpus does not read the scratch file", left 0.9545…, right 1.0) and again in a solo loop; KI-589 filed with the evidence. | ✅ closed — KI-589 carries it |
| G13 | The Conversations mockup carries evidence chips on assistant replies (⦿ verified · △ asserted) and reply-level recalls naming what was recalled — but the log's memory record carries counts only (`event.facts` + `shape` + `root`), not per-fact bases, and no reply-level verdict exists in any read the face makes. Rendering the chips from what exists would be invention. | D1/D5 (the Conversations re-skin rows), A2's evidence law | Researched in Pass 3: the memory event's shape (the transcript's stripLine), the corpus's basis ladder lives on the Intelligence surface's fact rows (fact.basis from the corpus read), and the desk's claim verdicts are the only verdict data the bridge serves. | Two honest paths, the owner gates the choice: (a) a core-seam row — the log's memory record gains a basis tally (n validated · n stated · n inferred) written at recall time, and the transcript's recall card and reply footers read it; (b) scope evidence labels to where verdicts already exist (the desk's claims) and leave the transcript's replies without them until (a). Either way the chips render only from records that exist. | open — owner gates (a) vs (b) |
| G14 | Pass 3 landed the Conversations re-skin's truthful half: the reply footer carries the turn's own route line (`routed · judged · acme` — the nearest routed record read from the receipts gutter's walk, the decision that served the reply; a turn with no routed record draws nothing). | D1 (the Conversations re-skin), D5 | The mockup's who-line carried the route; the binding rule is unambiguous — routing precedes the model call that produced the reply. The gutter walk already grouped route records per turn, so the line reads that grouping rather than a second walk. | Landed with two pins (the positive: the fixture carries the real log's shape — turn boundary → routed → reply; the absence: a turn with no routed record draws no line). Harness 1881 / 279. | ✅ Pass 3 Conversations |
| G15 | Pass 3's Board re-skin: the board leads with the home's needs-you decisions (the gated mockup's first column) — a needs-you card from the same document the rail reads, ordered as the bridge ordered it (the oldest hold first), each row one press to the thing it names, and the head naming the oldest hold's age from the document's own stamps. The boardLead's change + divergence rows (the relevance pass's own lead) turned out to have shipped with NO CSS rules — they rendered unstyled; the new row grammar styles all three block types. Ages come from a tolerant `ageOf` (epoch or ISO; an unparsable stamp says nothing). | D1 (the Board re-skin) | The needs-you rows carry `at` in two shapes (the holds' ISO strings, the approvals' epoch last_write) — `msAge` handled epoch only. The card shows in both rail scopes: a hold needs an answer whatever plan the owner is reading. | Landed with five pins (the card leads with two rows in the bridge's order; the oldest hold's age is the stamp's own on the row and the head; a press follows the link and the journey returns; the honest absence — no needs_you document paints no card). Harness **1886 / 279**. | ✅ Pass 3 Board |

## The palette record (G1's conversions, amended by G4's ruling — no current colour reused)

Shipped as `rgb()` literals (G1); the OKLCH designs are the record. **Every
value below was pre-verified against the guard's own math** (text tokens vs
page/surface/raised, `--on-tone` vs all six tints, `--on-flat` vs accent,
every gradient stop vs `--on-accent`) — zero failures across all four themes.

Persona tints (identical in every theme — bright heads, dark `--on-tone`
labels; G2's pairing):

| seat | hex | seat | hex |
|---|---|---|---|
| tint-1 | rgb(72, 183, 189) | tint-4 | rgb(210, 133, 203) |
| tint-2 | rgb(148, 185, 109) | tint-5 | rgb(103, 163, 224) |
| tint-3 | rgb(233, 158, 94) | tint-6 | rgb(180, 156, 225) |

**night** (the default — cool graphite):

| token | value | token | value |
|---|---|---|---|
| page | rgb(10, 12, 17) | accent | rgb(135, 139, 245) |
| surface | rgb(17, 20, 26) | accent-deep | rgb(111, 110, 229) |
| raised | rgb(24, 28, 35) | accent-lift | rgb(155, 161, 251) |
| ink | rgb(238, 240, 243) | ok | rgb(71, 196, 150) |
| ink-secondary | rgb(160, 165, 172) | warn | rgb(223, 180, 83) |
| ink-tertiary | rgb(133, 137, 145) | down | rgb(234, 94, 89) |
| on-accent / on-tone / on-flat | rgb(9, 13, 22) | hairlines | white 9% / 16% |

**day** (cool paper — the design-canvas ground, re-derived signals):

| token | value | token | value |
|---|---|---|---|
| page | rgb(240, 244, 247) | accent | rgb(100, 95, 223) |
| surface | rgb(248, 250, 253) | accent-deep | rgb(79, 69, 196) |
| raised | rgb(255, 255, 255) | accent-lift | rgb(72, 60, 149) |
| ink | rgb(25, 30, 40) | ok | rgb(0, 127, 78) |
| ink-secondary = tertiary | rgb(100, 105, 116) | warn | rgb(134, 89, 0) |
| on-tone | rgb(9, 13, 22) | down | rgb(197, 55, 50) |
| **on-flat / on-accent** | **rgb(255, 255, 255)** — white on the mid-bright indigo (the dark label measured 3.93) | hairlines | ink 12% / 20% |

**console** (the graphite collector):

| token | value | token | value |
|---|---|---|---|
| page | rgb(17, 19, 22) | accent | rgb(150, 156, 238) |
| surface | rgb(23, 26, 29) | accent-deep | rgb(119, 122, 216) |
| raised | rgb(31, 34, 38) | accent-lift | rgb(170, 177, 247) |
| ink | rgb(227, 229, 232) | ok / warn / down | as night |
| ink-secondary | rgb(149, 152, 159) | on-accent / on-tone / on-flat | rgb(9, 13, 22) |
| ink-tertiary | rgb(140, 143, 149) | | |

**contrast** (the accessibility theme — pure ground, near-black ink):

| token | value | token | value |
|---|---|---|---|
| page / surface | rgb(255, 255, 255) | accent | rgb(65, 60, 155) |
| raised | rgb(228, 228, 228) | accent-deep | rgb(51, 42, 136) |
| ink | rgb(0, 0, 0) | accent-lift | rgb(60, 54, 149) |
| ink-secondary | rgb(34, 34, 34) | ok | rgb(0, 107, 56) |
| ink-tertiary | rgb(58, 58, 58) | warn | rgb(131, 82, 0) |
| on-tone | rgb(20, 24, 34) | down | rgb(176, 12, 21) |
| **on-flat / on-accent** | **rgb(255, 255, 255)** — white on the dark indigo | hairlines | black 22% / 45% |

**The pre-verification iterations (why the loop exists):** the first pass
produced four guard failures the mockup's eyes never saw — day's dark label
on the mid-bright indigo (3.93), day's warn (4.44), console's tertiary on
raised (4.38), and contrast's dark label on the dark accent (1.98). Each was
fixed by re-deriving the value (white flat/gradient labels on day and
contrast, darker warn, lighter tertiary), then re-verified to zero. The
tint-label decision moved with G2: `--on-tone` is the disc label and the
guard measures it; `--on-flat`/`--on-accent` are the accent-fill labels and
stay per-theme (dark on night/console's bright indigo, white on day/contrast's
deeper one). Tokens split per job — one value never serves two grounds.

Every measured pair is proven by the guard, not by this table — the table
records intent, `themes.rs` measures the artifact.
