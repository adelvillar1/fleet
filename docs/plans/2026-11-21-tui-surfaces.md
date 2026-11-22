---
status: active
created: 2026-11-21
plan: tui-surfaces
---

# The TUI's intelligence surfaces — delegation graph, kanban, second brain

The face got its intelligence surfaces; the terminal still shows the older
shape of three of them. The owner asked for them in the TUI. This plan adds
them through the terminal's own idioms: the panel pattern (`PanelKind`,
`open_panel`/`apply_panel`, `Mode::Panel` → `knowledge::render`), the
`cards.rs` primitives (`CARD_WIDTH = 72`, escape-aware `fit`, `clip`, `rule`),
and **documents, not rederivations** — a panel that derives its own answer
diverges from the face's, and the divergence is the bug class this repo keeps
paying for. The rule the Bots panel already states: *"It reads the same
document the browser's Bots surface reads, so the two cannot disagree."*

**Measured starting points** (2026-11-21):

- `PanelKind` = `Memory, Skills, Calibration, Delegation, Bots`; panels read
  through `panels.rs::PanelData` (`Delegation => Ok(PanelData::Delegation(
  boards::read_delegation(home, session)))`).
- The delegation panel reads the **live tree only** — zero hits for overlay,
  divergence, `never spawned`, `declared_workload`. The face's
  `delegation_read` additionally joins the day's `OP_DELEGATION` rows into
  `graph_document` with per-node `overlay {spawned_sessions, spawn_count,
  divergence}` + `divergences {never_spawned, ran_twice}` and a
  `declared_workload` why-this-child line. The TUI shows yesterday's
  delegation; that is the defect.
- `board_view.rs` is composed **charts** (router mix, served-rate gauge,
  tower series, armada cards, line composers) "because their width is 42
  columns" — there is **no kanban panel**: the face's work board (the plan
  columns and their counts, `bridge::work_state`'s derivation) has no TUI
  reading.
- The second-brain document the face's `#/second-brain` reads is served by
  `bridge::second_brain_plugin` / the second-brain read the bots door's
  morning read derives from; the TUI's `PanelKind::Memory` panel reads its
  own memory document and does not carry the second-brain sections.

## The surfaces

1. **Delegation, the graph half.** The panel gains the same document the face
   reads: `bridge::delegation`'s `graph` with per-node overlay, the divergence
   words (`never spawned` / `ran twice ×N`) as a column the width budget can
   afford (elide through `cards::clip`, never drop), the `why-this-child` line
   (declared workload · brief shape · provider · outcome) on the card for the
   node a caret/focus rests on, and the graph-level divergences tile. The
   tree-half the panel already draws stays.
2. **The kanban panel.** `PanelKind::Board` — reads the SAME work-state read
   the face's board surface reads, renders the three derived columns
   (queued / in-process / done) as card stacks in the 72-cell card frame
   (`cards::rule('─')` headers, each card its plan's name, count, state word),
   with the honest empty ("no plan carries work right now" — a zero dressed as
   a column is the defect the face already fixed). The count strip is the
   columns' own counts, never a second derivation.
3. **Second brain, a panel named for what it reads.** `PanelKind::SecondBrain`
   — reads the face's second-brain document and renders its sections: the
   morning read's five sentences (each derived, refusals by name), the section
   counts and fact rows (name · tier · recall count), and today's recall
   breakdown if the document carries it. The `Memory` panel keeps its own
   memory read; the two must not merge — the face keeps them apart and so does
   the terminal.

## The rules this plan inherits

- **No rederivation, ever.** A TUI panel takes the bridge document the face
  takes; if a read shape is missing, extend the read — never parse the face's
  DOM, never recompute the derivation from raw stores.
- **The width is the truth on screen.** Cards through `cards.rs` (`fit`,
  `clip`, `rule`), colour escape-aware, every measured value mono. The 42-col
  line-composer discipline holds: a chart that cannot be honest at width is a
  summary row instead.
- **The four sentences.** Empty / loading / refused / selected in the panel's
  own voice, refusals by name — the same law the face obeys (a panel whose
  read failed says what failed, never a zero).
- **Tests are units, not screenshots.** The panel tests follow
  `ui/mod.rs:4950`'s pattern (`a_knowledge_panel_names_itself_and_its_loading_
  state`): open the panel, apply the document, assert the rendered rows. Every
  surface ships with a red-proof: remove the document join and the divergence
  column disappears (named), neuter an sentence and its case fails by name.
- **No build step** (house law).

## Acceptance criteria

- [x] The delegation panel's document is the face's `delegation_read` shape
      (graph + overlay + divergences + declared_workload), and a fixture with
      a twice-ran node shows the word in the TUI; red-proof: revert the join,
      the column vanishes by name.
- [x] The kanban panel exists with a case asserting the three columns and
      their counts from one fixture document, and the empty state names
      itself; red-proof: zero columns drawn where a refusal belongs.
- [x] The second-brain panel reads the face's document, asserts the five
      morning-read sentences and the section rows from one fixture; red-proof:
      a section that did not land prints that section's own refusal, never a
      zero.
- [x] `cargo test` green; no panel reads a store the face's route for the
      same surface does not.
- [x] The TUI screenshots note: `tui-conversation.png` in README gains the
      three surfaces' companions shot from the built binary (the gallery rig's
      discipline, terminal edition: the binary's own output, not a mock).

## Files to be touched

- `src/tui/app.rs` — `PanelKind` grows `Board`, `SecondBrain`; the open/apply
  plumbing follows the Bots panel's shape.
- `src/tui/panels.rs` — the three read joins (the bridge/boards documents the
  face reads).
- `src/tui/ui/knowledge.rs` / `ui/mod.rs` — the renderers through `cards.rs`.
- `src/gui/bridge.rs` — only if a read shape is missing (extend the read;
  document the extension on the read, not in the TUI).
- `docs/features/` — the terminal half of the feature docs the surfaces
  already have.
- tests as above.
