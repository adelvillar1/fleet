# The component catalog

A closed vocabulary for building UI components for other projects. The
catalog is the model's vocabulary: a declared set of component kinds, each
with typed props and a description, a spec shape, and a validator that
refuses a spec outside the vocabulary by name. A run builds a component page
with the `ui` tool; the page renders to one self-contained HTML file in the
face's own tokens, saved beside the session and handed over as a file card.

The catalog is a **sibling deliverable family** beside the deck
(presentations, infographics, posters) and the canvas's frames (opaque
designed pages). The pattern is the deck's — a schema, a refusing check, and
a render — over a general component vocabulary. The design record is
[docs/architecture/deep-dive-2026-11-07-ui-component-creation-and-json-render.md](../architecture/deep-dive-2026-11-07-ui-component-creation-and-json-render.md).

## The vocabulary

Sixteen kinds in six groups. Every entry carries a description the model
reads, and the props it accepts — the required ones first.

| Group | Kinds |
|---|---|
| layout | `container` |
| text | `heading`, `text`, `quote`, `callout`, `badge` |
| data | `metric`, `table`, `list`, `chart`, `timeline`, `comparison` |
| media | `image` |
| action | `link`, `button` |
| form | `form_field` |

The descriptions ride the system prompt, so the model builds inside the set
rather than inventing components. The validator's refusals name the vocabulary
when a kind is unknown.

## The spec

A flat tree — json-render's shape, chosen because it is diffable and
streamable:

```json
{
  "root": "page",
  "elements": {
    "page": { "type": "container", "props": { "dir": "column" },
              "children": ["head", "metric", "cta"] },
    "head": { "type": "heading", "props": { "text": "Q3 revenue review" }, "children": [] },
    "metric": { "type": "metric", "props": { "label": "Revenue", "value": "$12M" }, "children": [] },
    "cta": { "type": "button", "props": { "label": "Export", "action": "export_report" }, "children": [] }
  }
}
```

`root` names the page element; `elements` maps an id to its `type`, `props`
and children's ids. A spec is a document, not a nested tree — two versions
differ element by element.

## The checks

`catalog::check` refuses before anything renders, and every refusal names the
element and the fault:

- an unknown `type` — the vocabulary is closed
- a missing required prop, a wrong prop type, an unknown prop
- a child id that is not an element, a root that is not an element
- a cycle, an element nested deeper than the bound, an element no one reaches
- a spec past the element bound

The bounds are the frame's own: 256 elements, 12 levels of nesting, and a
512 KiB rendered page. A refusal is a fixable answer, not a failure — the
tool's description names the validation as its contract, so the model builds
to it.

## The render

A known-good spec renders to a complete HTML document carrying the face's
paper tokens and the catalog's `cat-*` classes. One file, network-dead: the
brand travels inside it, the same contract a canvas frame meets. Every
element carries a `data-cat` marker with its id, so a render is auditable
against its spec.

## The tool

`ui` accepts a spec and an optional name. It validates, renders, saves the
page beside the session (`<name>.html`), and returns the page. A refusal
returns a model-visible `error:` naming every element and fault. A rendered
page rides an artifact card, the same receipt a `send_file` gets.

## The mockup

The render contract was drawn and measured before the code:
`docs/plans/mockups/2026-11-07-component-catalog/` holds the sheet — every kind
with its props, plus one sample spec rendered end to end — measured in all
four themes (no overflow, no clipped text, ink ≥ 4.5:1). The primary button
carries an `--on-accent` token: white on the light themes, dark on the bright
accents of the dark themes, the face's own `.dk-btn.primary` pattern.
