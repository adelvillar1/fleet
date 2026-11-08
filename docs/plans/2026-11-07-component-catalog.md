---
status: completed
created: 2026-11-07
updated: 2026-11-07
slug: component-catalog
---

# Plan: the component catalog — a checked vocabulary for UI deliverables

## Context

Fleet builds visual deliverables two ways today: the design canvas's `frame`
tool takes opaque HTML from the model, and the `deck` tool takes a rigid serde
schema that `deck::check` refuses before `deck::render` draws it. The deck is
the proof that a checked, structured deliverable works — the model builds to
the checker because the tool's description names the constraints. What fleet
has no equivalent of json-render is a general component vocabulary: a closed
set of typed components with descriptions, a spec shape, and a refusing
validator, so the agent can build UI components for other projects without
emitting an invalid one. This plan is the first piece: the catalog, the spec,
the validator, one render, and the tool that carries them. The render targets
(image, terminal, code) and the canvas inspector are later plans.

## Approach

The deck is the precedent, not the thing to generalize. `src/deck.rs` proves
the shape: a serde schema (`Deck`, `Node`, `Component`, `Layout`), a
`check(&Deck) -> Vec<Refusal>` whose refusals name the node and the rule, a
`render(&Deck) -> String`, and a `deck` tool whose description names the
constraints as the contract. The catalog is a new sibling deliverable family
beside it — the same four parts over a general component vocabulary, where the
deck is presentations and the canvas's frames are opaque pages. The pattern is
shared; the family is new, and the tech doc's module reference records it as
one.

A new `src/catalog.rs` holds the vocabulary as one data-driven table: a
`Component` entry per kind with its typed props and its description. The spec
is json-render's flat shape — `UiSpec { root, elements }`, children referenced
by id — chosen because it is diffable and streamable. `catalog::check` walks
the spec and refuses: an unknown kind, a missing required prop, a wrong prop
type, an unknown prop, a dangling child id, a missing root, a cycle, and a
depth/size bound. `catalog::render` draws a known-good spec to a complete,
self-contained HTML document in fleet's own tokens (a `:root` block plus the
component classes, the same contract a frame carries). The catalog's
descriptions are injected into the system prompt so the model knows the
vocabulary, and a `ui` tool (`src/tools.rs`) accepts a spec, runs `check`, and
refuses or renders.

The starter vocabulary is bounded: layout (container), text (heading, text,
quote, callout, badge), data (metric, table, list, chart, timeline,
comparison), media (image), action (link, button), and form (form_field). The
exact prop set is implementation detail; the table's shape and the checks are
the contract.

Decision-model check: no sys1 fit in this piece — the vocabulary is closed and
every check is deterministic. The later suggest mode (ranking catalog
components by semantic match against an intent, the deck's `suggest_layout`
pattern) is the named future sys1/embedding job.

High-level steps:

0. **The mockup first.** `docs/plans/mockups/2026-11-07-component-catalog/` holds the catalog as static HTML in the app's own tokens: every component kind with its props, drawn the way it will render, plus one sample spec rendered end to end so the whole loop is visible before any code. The sheet is measured rendered in every theme — no overflow, no clipped text, ink ≥ 4.5:1 — the PNGs are read in a light and a dark theme, and the owner reads it before any code of C1+ lands (C0).
1. `src/catalog.rs`: the component table, the `UiSpec` type, and the serde round-trip (C1, C2).
2. `catalog::check` with the refusal set and its unit tests (C3, C4).
3. `catalog::render` to HTML and its unit test (C5).
4. The prompt injection and its test (C6).
5. The `ui` tool in `src/tools.rs`, its handler in `src/agent.rs`, and the refusal test (C7).
6. The four-leg gate, once, at the end (C8).
7. The feature doc and the tech doc's module reference (C9).

Structure guards this edit meets (`docs/architecture/structure-guards.md`): the
tool-surface bounds (a new tool spec joins `src/agent.rs`'s list, so
`MAX_VISIBLE_TOOLS` and `MAX_DEFINITION_TOKENS` are re-measured by the existing
pin test) and the face harness's fixture counts (no face fixture changes; the
tool is model-facing, not a face surface). Run `scripts/edit-preflight.sh`
before the first compile.

## Acceptance criteria

- [x] **C0 — the mockup is drawn, measured, read, and owner-read.** `docs/plans/mockups/2026-11-07-component-catalog/` holds the catalog as static HTML in fleet's own tokens — every kind with its props, plus one sample spec rendered end to end — measured rendered in every theme (no overflow, no clipped text, ink ≥ 4.5:1), the PNGs read in a light and a dark theme, and the owner reads it before any code of C1+ lands.
- [x] **C1 — the catalog table is data-driven.** `src/catalog.rs` ships a component table in which every entry has a unique kind, a non-empty description, and typed props, and a unit test asserts all three.
- [x] **C2 — the spec round-trips.** `src/catalog.rs` defines `UiSpec` (`root` plus `elements`), and a unit test parses a fixture spec from JSON and re-serializes it to an equal value.
- [x] **C3 — the validator refuses.** `catalog::check` returns a named refusal for a spec with an unknown component kind, a missing required prop, and a dangling child id, and a unit test asserts each refusal names the element and the fault.
- [x] **C4 — a known-good spec passes.** `catalog::check` returns zero refusals for a known-good fixture spec, and a unit test pins that.
- [x] **C5 — the render is complete.** `catalog::render` returns a complete HTML document for the known-good fixture, and a unit test asserts the output carries one marker per element and stays under the frame's size cap.
- [x] **C6 — the catalog feeds the prompt.** The system prompt names every catalog kind with its description, and a unit test asserts each kind appears in the assembled prompt.
- [x] **C7 — the `ui` tool refuses a bad spec.** The `ui` tool in `src/tools.rs` runs `catalog::check` before rendering and refuses a bad spec by name, and a unit test drives the handler with a bad spec and asserts the refusal.
- [x] **C8 — the gate is green.** The four-leg gate runs clean: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, and `cargo test` each exit 0 on the merged tree.
- [x] **C9 — the docs land.** `docs/features/catalog.md` documents the catalog, the spec shape, the check rules, and the `ui` tool, and `TECHNICAL-DOCUMENTATION.md`'s module reference names `catalog.rs`.

## Files to be touched

**Mockups:**
- `docs/plans/mockups/2026-11-07-component-catalog/` — the component sheet and one sample spec as static HTML in the app's tokens, plus the measured PNGs.

**Backend:**
- `src/catalog.rs` — new: the component table, `UiSpec`, `check`, `render`, the prompt text.
- `src/tools.rs` — the `ui` tool spec.
- `src/agent.rs` — the tool's handler, the spec list, the prompt injection.

**Tests:**
- `src/catalog.rs` (inline `tests`) — the round-trip, the refusals, the render, the prompt.
- `tests/` — a scoped integration case for the tool handler if the inline tests cannot reach it.

**Documentation:**
- `docs/features/catalog.md` — new.
- `TECHNICAL-DOCUMENTATION.md` — the module reference.
- `FUNCTIONAL-SPECIFICATIONS.md` — the deliverable surface.

## Out of scope

- Render targets beyond HTML (image, terminal, code, PDF) — the next plan.
- The canvas's spec inspector / review card.
- The fleet-authored skill for the workflow.
- Any change to the `frame` tool or the design canvas's frame model.
- The suggest mode (ranking components by semantic match).

## Verification

**C0:** the mockup sheet rendered at 1440×900 in all four themes, measured for overflow and ink contrast, the PNGs read in a light and a dark theme, and the owner's read recorded.
**C1–C6:** `cargo test -p fleet --lib catalog` — the inline unit tests.
**C7:** the handler test in the same run; drive the `ui` tool with a spec naming an unknown kind and assert the refusal text names the kind.
**C8:** `cargo fmt --all --check`; `cargo clippy --all-targets -- -D warnings`; `cargo build --release`; `cargo test`. Read the exit codes, never through a status-losing pipe (KI-014).
**C9:** read the two docs; the feature doc names the catalog, the spec, the checks, and the tool.

## Linked artifacts

- `docs/features/catalog.md` — the catalog's vocabulary, the spec shape, the check rules, the `ui` tool, and the render contract.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — `catalog.rs` as a new sibling deliverable family beside `deck`.
- `TECHNICAL-DOCUMENTATION.md` section "`tools.rs`" — the `ui` tool joins the closed built-in set.
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the `ui` deliverable.

## Risks

- A second page-making tool beside `frame` could confuse the model; the descriptions must stay distinct (`frame` is a canvas page, `ui` is a checked component for another project).
- The starter vocabulary could be too small to be useful or too large to be learnable; the descriptions carry the weight, and the set is a table, so it grows cheaply.
- The render's tokens must match the design system's alias tier or the deliverable drifts from the brand; reuse the token table rather than a new one.

## Dependencies

- `docs/plans/2026-10-24-deck-vector-moves-2-3.md` — the deck's schema/check/render/tool pattern this generalizes.

## Notes

- The deck's tool description is the precedent for naming the constraints in the tool text: "a model that knows the checker refuses will build to it."
- json-render's spec shape (`{root, elements}`) is borrowed; its runtime, state store, and expression language are deliberately not.
- The tool is the minimal consumer; the render targets are the next piece.
