//! The component catalog — a checked vocabulary for UI deliverables.
//!
//! A deliverable built from the catalog is a [`UiSpec`]: a flat tree of typed
//! components. [`check`] refuses a spec that breaks the vocabulary — an
//! unknown kind, a missing required prop, a wrong prop type, an unknown prop,
//! a dangling child, a cycle, a depth or size bound. [`render`] draws a
//! known-good spec to a complete, self-contained HTML document in the face's
//! own tokens. The catalog's descriptions are the model's vocabulary:
//! [`prompt_text`] feeds them to the system prompt, so the model builds inside
//! the closed set instead of inventing components.
//!
//! The deck is the precedent (`src/deck.rs`): a serde schema, a refusing
//! check, and a render. The catalog is the same shape over a general
//! component vocabulary, and it is a sibling deliverable family beside the
//! deck (presentations) and the canvas's opaque frames (pages).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// The most elements a spec may carry. A deliverable is one page, not a
/// document; a spec past this bound is a document wearing a component's
/// clothes.
pub const MAX_ELEMENTS: usize = 256;
/// The deepest container nesting. Past this the page is a tunnel.
pub const MAX_DEPTH: usize = 12;
/// The rendered document's size cap — the frame's own bound
/// (`canvas::MAX_FRAME_BYTES`), so a catalog deliverable and a canvas frame
/// cost the same.
pub const MAX_HTML_BYTES: usize = 512 * 1024;

/// The type a prop accepts. The validator checks the spec's value against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    /// Any string.
    Text,
    /// A JSON number.
    Number,
    /// A JSON boolean.
    Bool,
    /// One of the listed strings.
    Enum(&'static [&'static str]),
    /// A JSON array (of strings or of objects, by the component's own rule).
    Items,
}

/// One prop of a component: its name, whether it is required, and its type.
#[derive(Debug, Clone, Copy)]
pub struct PropDef {
    pub name: &'static str,
    pub required: bool,
    pub kind: PropKind,
}

/// One catalog kind: its name, its group, the description the model reads, and
/// its props.
#[derive(Debug, Clone, Copy)]
pub struct ComponentDef {
    pub kind: &'static str,
    pub group: &'static str,
    pub description: &'static str,
    pub props: &'static [PropDef],
}

const fn prop(name: &'static str, required: bool, kind: PropKind) -> PropDef {
    PropDef {
        name,
        required,
        kind,
    }
}

/// The catalog. One entry per kind; the descriptions are the model's
/// vocabulary, so they are written for the model, not for a reference page.
pub static CATALOG: &[ComponentDef] = &[
    ComponentDef {
        kind: "container",
        group: "layout",
        description: "A vertical or horizontal stack. Every other component sits inside one.",
        props: &[
            prop("dir", false, PropKind::Enum(&["column", "row"])),
            prop("gap", false, PropKind::Number),
            prop(
                "align",
                false,
                PropKind::Enum(&["start", "center", "end", "stretch"]),
            ),
        ],
    },
    ComponentDef {
        kind: "heading",
        group: "text",
        description: "A display line. Level 1 is the page title, 2 a section, 3 a subsection.",
        props: &[
            prop("text", true, PropKind::Text),
            prop("level", false, PropKind::Enum(&["1", "2", "3"])),
        ],
    },
    ComponentDef {
        kind: "text",
        group: "text",
        description: "A paragraph. The workhorse; carries the evidence the heading asserts.",
        props: &[
            prop("text", true, PropKind::Text),
            prop("tone", false, PropKind::Enum(&["primary", "secondary"])),
        ],
    },
    ComponentDef {
        kind: "quote",
        group: "text",
        description: "A pulled quotation with its source.",
        props: &[
            prop("text", true, PropKind::Text),
            prop("attribution", false, PropKind::Text),
        ],
    },
    ComponentDef {
        kind: "callout",
        group: "text",
        description: "A titled note that stands apart from the flow. One tone at a time.",
        props: &[
            prop("title", true, PropKind::Text),
            prop("body", true, PropKind::Text),
            prop(
                "tone",
                false,
                PropKind::Enum(&["neutral", "ok", "warn", "down"]),
            ),
        ],
    },
    ComponentDef {
        kind: "badge",
        group: "text",
        description: "A short status mark. Solid, never translucent.",
        props: &[
            prop("label", true, PropKind::Text),
            prop(
                "tone",
                false,
                PropKind::Enum(&["neutral", "accent", "ok", "warn", "down"]),
            ),
        ],
    },
    ComponentDef {
        kind: "metric",
        group: "data",
        description: "One number, large, with its label and an optional change.",
        props: &[
            prop("label", true, PropKind::Text),
            prop("value", true, PropKind::Text),
            prop("delta", false, PropKind::Text),
            prop("tone", false, PropKind::Enum(&["up", "down", "flat"])),
        ],
    },
    ComponentDef {
        kind: "table",
        group: "data",
        description: "Rows and columns with one neutral rule colour and hairline dividers.",
        props: &[
            prop("columns", true, PropKind::Items),
            prop("rows", true, PropKind::Items),
        ],
    },
    ComponentDef {
        kind: "list",
        group: "data",
        description: "A short list. Each item one line, one point.",
        props: &[
            prop("items", true, PropKind::Items),
            prop("ordered", false, PropKind::Bool),
        ],
    },
    ComponentDef {
        kind: "chart",
        group: "data",
        description: "A generated chart from a data spec, never hand-drawn geometry.",
        props: &[
            prop("kind", true, PropKind::Enum(&["bar", "line"])),
            prop("series", true, PropKind::Items),
            prop("labels", true, PropKind::Items),
        ],
    },
    ComponentDef {
        kind: "timeline",
        group: "data",
        description: "A sequence with a dot and a connector; one label and one note each.",
        props: &[prop("events", true, PropKind::Items)],
    },
    ComponentDef {
        kind: "comparison",
        group: "data",
        description: "Two sides contrasted. One side may be marked better.",
        props: &[
            prop("left", true, PropKind::Text),
            prop("right", true, PropKind::Text),
            prop("better", false, PropKind::Enum(&["left", "right"])),
        ],
    },
    ComponentDef {
        kind: "image",
        group: "media",
        description: "A figure with its alt text. The slot is decided before the crop.",
        props: &[
            prop("src", true, PropKind::Text),
            prop("alt", true, PropKind::Text),
            prop("aspect", false, PropKind::Enum(&["16:9", "4:3", "1:1"])),
        ],
    },
    ComponentDef {
        kind: "link",
        group: "action",
        description: "An inline jump. Understated, never a button.",
        props: &[
            prop("label", true, PropKind::Text),
            prop("href", true, PropKind::Text),
        ],
    },
    ComponentDef {
        kind: "button",
        group: "action",
        description: "A filled action. One primary per surface.",
        props: &[
            prop("label", true, PropKind::Text),
            prop("action", true, PropKind::Text),
            prop(
                "tone",
                false,
                PropKind::Enum(&["primary", "secondary", "danger"]),
            ),
        ],
    },
    ComponentDef {
        kind: "form_field",
        group: "form",
        description: "One labelled input. The label is always visible.",
        props: &[
            prop("label", true, PropKind::Text),
            prop(
                "kind",
                true,
                PropKind::Enum(&["text", "email", "number", "textarea"]),
            ),
            prop("placeholder", false, PropKind::Text),
            prop("required", false, PropKind::Bool),
        ],
    },
];

/// The catalog kinds in declaration order, for the prompt and the tests.
pub fn kinds() -> Vec<&'static str> {
    CATALOG.iter().map(|c| c.kind).collect()
}

/// The catalog grouped by family, for the prompt.
pub fn groups() -> Vec<&'static str> {
    let mut seen: Vec<&'static str> = Vec::new();
    for c in CATALOG {
        if !seen.contains(&c.group) {
            seen.push(c.group);
        }
    }
    seen
}

/// Look up a kind.
pub fn find(kind: &str) -> Option<&'static ComponentDef> {
    CATALOG.iter().find(|c| c.kind == kind)
}

/// A spec: a flat element map with a named root. json-render's shape, chosen
/// because it is diffable and streamable — a spec is a document, not a nested
/// tree, so two versions differ element by element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSpec {
    pub root: String,
    #[serde(default)]
    pub elements: BTreeMap<String, Element>,
}

/// One element: a catalog kind, its props, and the ids of its children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub props: BTreeMap<String, Value>,
    #[serde(default)]
    pub children: Vec<String>,
}

/// A refusal: the element's id and the fault, in the crate's model-visible
/// line style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub element: String,
    pub fault: String,
}

impl Refusal {
    pub fn line(&self) -> String {
        format!("`{}`: {}", self.element, self.fault)
    }
}

fn kinds_joined() -> String {
    kinds().join("`, `")
}

/// Check a spec against the catalog. Every refusal names the element and the
/// fault, so a repair is one fix, not a search.
pub fn check(spec: &UiSpec) -> Vec<Refusal> {
    let mut out = Vec::new();
    if spec.elements.is_empty() {
        out.push(Refusal {
            element: "(spec)".into(),
            fault: "the spec carries no elements".into(),
        });
        return out;
    }
    if !spec.elements.contains_key(&spec.root) {
        out.push(Refusal {
            element: "(root)".into(),
            fault: format!("`{}` is not an element of the spec", spec.root),
        });
    }
    if spec.elements.len() > MAX_ELEMENTS {
        out.push(Refusal {
            element: "(spec)".into(),
            fault: format!(
                "the spec carries {} elements; the bound is {}",
                spec.elements.len(),
                MAX_ELEMENTS
            ),
        });
    }

    for (id, el) in &spec.elements {
        let Some(def) = find(&el.kind) else {
            out.push(Refusal {
                element: id.clone(),
                fault: format!(
                    "`{}` is not a catalog kind — the vocabulary is `{}`",
                    el.kind,
                    kinds_joined()
                ),
            });
            continue;
        };
        for name in el.props.keys() {
            if !def.props.iter().any(|p| p.name == name) {
                out.push(Refusal {
                    element: id.clone(),
                    fault: format!("`{}` is not a prop of `{}`", name, el.kind),
                });
            }
        }
        for p in def.props {
            match el.props.get(p.name) {
                None => {
                    if p.required {
                        out.push(Refusal {
                            element: id.clone(),
                            fault: format!(
                                "`{}` is required on `{}` and is missing",
                                p.name, el.kind
                            ),
                        });
                    }
                }
                Some(v) => {
                    if let Some(reason) = check_prop(p, v) {
                        out.push(Refusal {
                            element: id.clone(),
                            fault: format!("`{}` {}", p.name, reason),
                        });
                    }
                }
            }
        }
        for child in &el.children {
            if !spec.elements.contains_key(child) {
                out.push(Refusal {
                    element: id.clone(),
                    fault: format!("the child `{}` is not an element of the spec", child),
                });
            }
        }
    }

    check_reachability(spec, &mut out);
    out
}

fn check_prop(p: &PropDef, v: &Value) -> Option<String> {
    match p.kind {
        PropKind::Text => {
            if v.is_string() {
                None
            } else {
                Some("must be a string".into())
            }
        }
        PropKind::Number => {
            if v.is_number() {
                None
            } else {
                Some("must be a number".into())
            }
        }
        PropKind::Bool => {
            if v.is_boolean() {
                None
            } else {
                Some("must be a boolean".into())
            }
        }
        PropKind::Items => {
            if v.is_array() {
                None
            } else {
                Some("must be an array".into())
            }
        }
        PropKind::Enum(opts) => match v.as_str() {
            Some(s) if opts.contains(&s) => None,
            Some(s) => Some(format!(
                "must be one of `{}` — `{}` is not",
                opts.join("`, `"),
                s
            )),
            None => Some("must be a string".into()),
        },
    }
}

/// Depth, cycle and share check, walking from the root. A cycle is a refusal
/// (the render would recurse forever); a child two parents share is a refusal
/// (the spec is a tree, so the render would draw it twice); an unreachable
/// element is named too, because a spec with a page no one links to is a
/// draft the model forgot.
fn check_reachability(spec: &UiSpec, out: &mut Vec<Refusal>) {
    let mut visited: HashSet<String> = HashSet::new();
    let mut ancestry: HashSet<String> = HashSet::new();
    walk(spec, &spec.root, 0, &mut ancestry, &mut visited, out);
    for id in spec.elements.keys() {
        if !visited.contains(id) {
            out.push(Refusal {
                element: id.clone(),
                fault: "is not reachable from the root".into(),
            });
        }
    }
}

fn walk(
    spec: &UiSpec,
    id: &str,
    depth: usize,
    ancestry: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    out: &mut Vec<Refusal>,
) {
    if depth > MAX_DEPTH {
        out.push(Refusal {
            element: id.to_string(),
            fault: format!("is nested deeper than {} levels", MAX_DEPTH),
        });
        return;
    }
    if !ancestry.insert(id.to_string()) {
        out.push(Refusal {
            element: id.to_string(),
            fault: "is reached from its own child — the tree has a cycle".into(),
        });
        return;
    }
    if !visited.insert(id.to_string()) {
        out.push(Refusal {
            element: id.to_string(),
            fault: "is reached more than once — a spec is a tree, so a shared child draws twice"
                .into(),
        });
        ancestry.remove(id);
        return;
    }
    if let Some(el) = spec.elements.get(id) {
        for child in &el.children {
            walk(spec, child, depth + 1, ancestry, visited, out);
        }
    }
    ancestry.remove(id);
}

/// The catalog as the model's vocabulary, for the system prompt. One line per
/// kind, the description first — the model reads this to build inside the set.
pub fn prompt_text() -> String {
    let mut out = String::from(
        "The component catalog — the closed vocabulary for `ui` specs. Every element's `type` \
         must be one of these kinds; a spec outside the vocabulary is refused by name.\n",
    );
    for group in groups() {
        out.push_str(&format!("\n{}:\n", group));
        for c in CATALOG.iter().filter(|c| c.group == group) {
            let req: Vec<&str> = c
                .props
                .iter()
                .filter(|p| p.required)
                .map(|p| p.name)
                .collect();
            let opt: Vec<&str> = c
                .props
                .iter()
                .filter(|p| !p.required)
                .map(|p| p.name)
                .collect();
            out.push_str(&format!("- `{}` — {}. ", c.kind, c.description));
            if !req.is_empty() {
                out.push_str(&format!("required: `{}`. ", req.join("`, `")));
            }
            if !opt.is_empty() {
                out.push_str(&format!("optional: `{}`. ", opt.join("`, `")));
            }
            out.push('\n');
        }
    }
    out
}

fn esc(v: &Value) -> String {
    let s = match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn attr(v: &Value) -> String {
    esc(v).replace('\'', "&#39;")
}

fn prop_str(el: &Element, name: &str) -> Option<String> {
    el.props
        .get(name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Render a known-good spec to a complete, self-contained HTML document. The
/// document carries the face's paper tokens and the catalog's `cat-*` classes,
/// so it is the same artifact a frame is: network-dead, one file, the brand
/// inside it.
pub fn render(spec: &UiSpec) -> String {
    let title = spec
        .elements
        .get(&spec.root)
        .and_then(|el| prop_str(el, "text").or_else(|| prop_str(el, "label")))
        .unwrap_or_else(|| "component".to_string());
    let mut out = String::new();
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str(&format!("<title>{}</title>\n", esc(&Value::String(title))));
    out.push_str("<style>\n");
    out.push_str(STYLES);
    out.push_str("\n</style>\n</head>\n<body>\n");
    if let Some(root) = spec.elements.get(&spec.root) {
        render_element(spec, &spec.root, root, 0, &mut out);
    }
    out.push_str("\n</body>\n</html>\n");
    out
}

fn render_element(spec: &UiSpec, id: &str, el: &Element, depth: usize, out: &mut String) {
    if depth > MAX_DEPTH {
        return;
    }
    let pad = "  ".repeat(depth);
    match el.kind.as_str() {
        "container" => {
            let dir = prop_str(el, "dir").unwrap_or_else(|| "column".into());
            out.push_str(&format!(
                "{}<div class=\"cat-container\" data-cat=\"{}\" data-dir=\"{}\">\n",
                pad, id, dir
            ));
            render_children(spec, el, depth + 1, out);
            out.push_str(&format!("{}</div>\n", pad));
        }
        "heading" => {
            let level = prop_str(el, "level").unwrap_or_else(|| "1".into());
            out.push_str(&format!(
                "{}<h1 class=\"cat-heading\" data-cat=\"{}\" data-level=\"{}\">{}</h1>\n",
                pad,
                id,
                level,
                esc(el.props.get("text").unwrap_or(&Value::Null))
            ));
        }
        "text" => {
            let tone = prop_str(el, "tone").unwrap_or_else(|| "secondary".into());
            out.push_str(&format!(
                "{}<p class=\"cat-text\" data-cat=\"{}\" data-tone=\"{}\">{}</p>\n",
                pad,
                id,
                tone,
                esc(el.props.get("text").unwrap_or(&Value::Null))
            ));
        }
        "quote" => {
            out.push_str(&format!(
                "{}<blockquote class=\"cat-quote\" data-cat=\"{}\">\n",
                pad, id
            ));
            out.push_str(&format!(
                "{}  {}\n",
                pad,
                esc(el.props.get("text").unwrap_or(&Value::Null))
            ));
            if let Some(attribution) = prop_str(el, "attribution") {
                out.push_str(&format!(
                    "{}  <cite>{}</cite>\n",
                    pad,
                    esc(&Value::String(attribution))
                ));
            }
            out.push_str(&format!("{}</blockquote>\n", pad));
        }
        "callout" => {
            let tone = prop_str(el, "tone").unwrap_or_else(|| "neutral".into());
            out.push_str(&format!(
                "{}<div class=\"cat-callout\" data-cat=\"{}\" data-tone=\"{}\">\n",
                pad, id, tone
            ));
            out.push_str(&format!(
                "{}  <div class=\"cat-callout-title\">{}</div>\n",
                pad,
                esc(el.props.get("title").unwrap_or(&Value::Null))
            ));
            out.push_str(&format!(
                "{}  <div class=\"cat-callout-body\">{}</div>\n",
                pad,
                esc(el.props.get("body").unwrap_or(&Value::Null))
            ));
            out.push_str(&format!("{}</div>\n", pad));
        }
        "badge" => {
            let tone = prop_str(el, "tone").unwrap_or_else(|| "neutral".into());
            out.push_str(&format!(
                "{}<span class=\"cat-badge\" data-cat=\"{}\" data-tone=\"{}\">{}</span>\n",
                pad,
                id,
                tone,
                esc(el.props.get("label").unwrap_or(&Value::Null))
            ));
        }
        "metric" => {
            let tone = prop_str(el, "tone").unwrap_or_else(|| "flat".into());
            out.push_str(&format!(
                "{}<div class=\"cat-metric\" data-cat=\"{}\">\n",
                pad, id
            ));
            out.push_str(&format!(
                "{}  <span class=\"cat-metric-value\">{}</span>\n",
                pad,
                esc(el.props.get("value").unwrap_or(&Value::Null))
            ));
            out.push_str(&format!(
                "{}  <span class=\"cat-metric-label\">{}</span>\n",
                pad,
                esc(el.props.get("label").unwrap_or(&Value::Null))
            ));
            if let Some(delta) = prop_str(el, "delta") {
                out.push_str(&format!(
                    "{}  <span class=\"cat-metric-delta\" data-tone=\"{}\">{}</span>\n",
                    pad,
                    tone,
                    esc(&Value::String(delta))
                ));
            }
            out.push_str(&format!("{}</div>\n", pad));
        }
        "table" => {
            out.push_str(&format!(
                "{}<table class=\"cat-table\" data-cat=\"{}\">\n",
                pad, id
            ));
            out.push_str(&format!("{}  <thead><tr>\n", pad));
            if let Some(cols) = el.props.get("columns").and_then(|v| v.as_array()) {
                for c in cols {
                    out.push_str(&format!("{}    <th>{}</th>\n", pad, esc(c)));
                }
            }
            out.push_str(&format!("{}  </tr></thead>\n", pad));
            out.push_str(&format!("{}  <tbody>\n", pad));
            if let Some(rows) = el.props.get("rows").and_then(|v| v.as_array()) {
                for row in rows {
                    out.push_str(&format!("{}    <tr>\n", pad));
                    if let Some(cells) = row.as_array() {
                        for cell in cells {
                            out.push_str(&format!(
                                "{}      <td>{}</td>\n",
                                pad,
                                esc(&Value::String(cell.to_string()))
                            ));
                        }
                    }
                    out.push_str(&format!("{}    </tr>\n", pad));
                }
            }
            out.push_str(&format!("{}  </tbody>\n", pad));
            out.push_str(&format!("{}</table>\n", pad));
        }
        "list" => {
            let ordered = el
                .props
                .get("ordered")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let tag = if ordered { "ol" } else { "ul" };
            out.push_str(&format!(
                "{}<{} class=\"cat-list\" data-cat=\"{}\">\n",
                pad, tag, id
            ));
            if let Some(items) = el.props.get("items").and_then(|v| v.as_array()) {
                for item in items {
                    out.push_str(&format!("{}  <li>{}</li>\n", pad, esc(item)));
                }
            }
            out.push_str(&format!("{}</{}>\n", pad, tag));
        }
        "chart" => {
            let kind = prop_str(el, "kind").unwrap_or_else(|| "bar".into());
            out.push_str(&format!(
                "{}<div class=\"cat-chart\" data-cat=\"{}\" data-kind=\"{}\">\n",
                pad, id, kind
            ));
            let series = el
                .props
                .get("series")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let labels = el
                .props
                .get("labels")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let max = series
                .iter()
                .filter_map(|v| v.as_f64())
                .fold(1.0f64, |a, b| a.max(b));
            for (i, value) in series.iter().enumerate() {
                let h = (value.as_f64().unwrap_or(0.0) / max * 100.0).clamp(0.0, 100.0);
                let label = labels.get(i).map(esc).unwrap_or_default();
                out.push_str(&format!(
                    "{}  <div class=\"cat-chart-col\"><div class=\"cat-chart-bar\" style=\"height:{:.0}%\"></div><span class=\"cat-chart-label\">{}</span></div>\n",
                    pad, h, label
                ));
            }
            out.push_str(&format!("{}</div>\n", pad));
        }
        "timeline" => {
            out.push_str(&format!(
                "{}<div class=\"cat-timeline\" data-cat=\"{}\">\n",
                pad, id
            ));
            if let Some(events) = el.props.get("events").and_then(|v| v.as_array()) {
                for event in events {
                    let label = event.get("label").map(esc).unwrap_or_default();
                    let note = event.get("note").map(esc).unwrap_or_default();
                    out.push_str(&format!(
                        "{}  <div class=\"cat-timeline-item\"><span class=\"cat-timeline-dot\"></span><div class=\"cat-timeline-body\"><span class=\"cat-timeline-label\">{}</span><span class=\"cat-timeline-note\">{}</span></div></div>\n",
                        pad, label, note
                    ));
                }
            }
            out.push_str(&format!("{}</div>\n", pad));
        }
        "comparison" => {
            let better = prop_str(el, "better").unwrap_or_default();
            out.push_str(&format!(
                "{}<div class=\"cat-comparison\" data-cat=\"{}\">\n",
                pad, id
            ));
            out.push_str(&format!(
                "{}  <div class=\"cat-comparison-side\" data-tone=\"{}\">{}</div>\n",
                pad,
                if better == "left" { "better" } else { "" },
                esc(el.props.get("left").unwrap_or(&Value::Null))
            ));
            out.push_str(&format!(
                "{}  <div class=\"cat-comparison-side\" data-tone=\"{}\">{}</div>\n",
                pad,
                if better == "right" { "better" } else { "" },
                esc(el.props.get("right").unwrap_or(&Value::Null))
            ));
            out.push_str(&format!("{}</div>\n", pad));
        }
        "image" => {
            let aspect = prop_str(el, "aspect").unwrap_or_else(|| "16:9".into());
            let alt = prop_str(el, "alt").unwrap_or_default();
            let src = prop_str(el, "src").unwrap_or_default();
            if src.is_empty() {
                out.push_str(&format!(
                    "{}<div class=\"cat-image\" data-cat=\"{}\" role=\"img\" aria-label=\"{}\">{}</div>\n",
                    pad, id, attr(&Value::String(alt)), aspect
                ));
            } else {
                out.push_str(&format!(
                    "{}<img class=\"cat-image\" data-cat=\"{}\" src=\"{}\" alt=\"{}\" style=\"aspect-ratio:{}\">\n",
                    pad,
                    id,
                    attr(&Value::String(src)),
                    attr(&Value::String(alt)),
                    attr(&Value::String(aspect.replace(':', " / ")))
                ));
            }
        }
        "link" => {
            let href = prop_str(el, "href").unwrap_or_default();
            out.push_str(&format!(
                "{}<a class=\"cat-link\" data-cat=\"{}\" href=\"{}\">{}</a>\n",
                pad,
                id,
                attr(&Value::String(href)),
                esc(el.props.get("label").unwrap_or(&Value::Null))
            ));
        }
        "button" => {
            let tone = prop_str(el, "tone").unwrap_or_else(|| "primary".into());
            let action = prop_str(el, "action").unwrap_or_default();
            out.push_str(&format!(
                "{}<button class=\"cat-button\" data-cat=\"{}\" data-tone=\"{}\" data-action=\"{}\">{}</button>\n",
                pad,
                id,
                tone,
                attr(&Value::String(action)),
                esc(el.props.get("label").unwrap_or(&Value::Null))
            ));
        }
        "form_field" => {
            let kind = prop_str(el, "kind").unwrap_or_else(|| "text".into());
            let label = prop_str(el, "label").unwrap_or_default();
            let placeholder = prop_str(el, "placeholder").unwrap_or_default();
            let required = el
                .props
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            out.push_str(&format!(
                "{}<div class=\"cat-form-field\" data-cat=\"{}\">\n",
                pad, id
            ));
            out.push_str(&format!(
                "{}  <label class=\"cat-form-label\">{}{}</label>\n",
                pad,
                esc(&Value::String(label)),
                if required { " *" } else { "" }
            ));
            if kind == "textarea" {
                out.push_str(&format!(
                    "{}  <textarea class=\"cat-form-input\" placeholder=\"{}\"{}></textarea>\n",
                    pad,
                    attr(&Value::String(placeholder)),
                    if required { " required" } else { "" }
                ));
            } else {
                out.push_str(&format!(
                    "{}  <input class=\"cat-form-input\" type=\"{}\" placeholder=\"{}\"{}>\n",
                    pad,
                    attr(&Value::String(kind)),
                    attr(&Value::String(placeholder)),
                    if required { " required" } else { "" }
                ));
            }
            out.push_str(&format!("{}</div>\n", pad));
        }
        other => {
            out.push_str(&format!(
                "{}<!-- unknown catalog kind `{}` on `{}` -->\n",
                pad, other, id
            ));
        }
    }
}

fn render_children(spec: &UiSpec, el: &Element, depth: usize, out: &mut String) {
    for child in &el.children {
        if let Some(el) = spec.elements.get(child) {
            render_element(spec, child, el, depth, out);
        }
    }
}

/// The document's own stylesheet: the face's paper tokens and the catalog's
/// `cat-*` classes. A render is one file — the brand travels inside it.
const STYLES: &str = r#"
:root {
  --page: rgb(246, 244, 239);
  --surface: rgb(252, 251, 248);
  --raised: rgb(239, 236, 230);
  --hairline: rgb(24 26 29 / 10%);
  --hairline-strong: rgb(24 26 29 / 17%);
  --ink: rgb(24, 26, 29);
  --ink-secondary: rgb(82, 88, 96);
  --ink-tertiary: rgb(94, 102, 112);
  --accent: rgb(47, 95, 208);
  --on-accent: #fff;
  --ok: rgb(22, 110, 46);
  --warn: rgb(122, 82, 8);
  --down: rgb(176, 36, 32);
  --shadow-1: 0 1px 2px rgb(24 26 29 / 6%), 0 1px 1px rgb(24 26 29 / 4%);
  --display: "New York", ui-serif, Georgia, "Iowan Old Style", "Times New Roman", serif;
  --text: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  --mono: ui-monospace, "SF Mono", SFMono-Regular, "JetBrains Mono", Menlo, Consolas, monospace;
  --radius-sm: 7px; --radius-md: 10px; --radius-lg: 14px;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--page); color: var(--ink);
  font: 13.5px/1.55 var(--text); -webkit-font-smoothing: antialiased; }
.cat-container { display: flex; flex-direction: column; gap: 8px; }
.cat-container[data-dir="row"] { flex-direction: row; align-items: center; }
.cat-heading { font: 500 20px/1.2 var(--display); letter-spacing: -.02em; margin: 0; }
.cat-heading[data-level="2"] { font-size: 17px; }
.cat-heading[data-level="3"] { font-size: 15px; }
.cat-text { font: 400 13.5px/1.55 var(--text); color: var(--ink-secondary); margin: 0; }
.cat-quote { border-left: 2px solid var(--accent); padding: 2px 0 2px 12px;
  font: 400 15px/1.5 var(--display); color: var(--ink); margin: 0; }
.cat-quote cite { display: block; margin-top: 6px; font: 400 11px/1.4 var(--text);
  color: var(--ink-tertiary); font-style: normal; }
.cat-callout { border: 1px solid var(--hairline); border-radius: var(--radius-md);
  background: var(--surface); padding: 10px 12px; display: grid; gap: 4px; box-shadow: var(--shadow-1); }
.cat-callout[data-tone="warn"] { border-color: color-mix(in srgb, var(--warn) 40%, transparent); }
.cat-callout[data-tone="ok"] { border-color: color-mix(in srgb, var(--ok) 40%, transparent); }
.cat-callout[data-tone="down"] { border-color: color-mix(in srgb, var(--down) 40%, transparent); }
.cat-callout-title { font: 600 12.5px/1.3 var(--text); }
.cat-callout-body { font: 400 12.5px/1.5 var(--text); color: var(--ink-secondary); }
.cat-badge { display: inline-flex; align-items: center; padding: 2px 8px; border-radius: 999px;
  font: 600 10px/1.5 var(--text); letter-spacing: .06em; text-transform: uppercase;
  background: var(--raised); color: var(--ink-secondary); }
.cat-badge[data-tone="accent"] { background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }
.cat-badge[data-tone="ok"] { background: color-mix(in srgb, var(--ok) 12%, transparent); color: var(--ok); }
.cat-badge[data-tone="warn"] { background: color-mix(in srgb, var(--warn) 14%, transparent); color: var(--warn); }
.cat-badge[data-tone="down"] { background: color-mix(in srgb, var(--down) 12%, transparent); color: var(--down); }
.cat-metric { display: grid; gap: 2px; }
.cat-metric-value { font: 500 26px/1.1 var(--display); letter-spacing: -.03em; font-variant-numeric: tabular-nums; }
.cat-metric-label { font: 400 11px/1.4 var(--text); color: var(--ink-tertiary); }
.cat-metric-delta { font: 600 11px/1.4 var(--text); color: var(--ok); }
.cat-metric-delta[data-tone="down"] { color: var(--down); }
.cat-table { width: 100%; border-collapse: collapse; font: 400 12px/1.45 var(--text); }
.cat-table th { text-align: left; font: 600 10px/1.4 var(--text); letter-spacing: .08em;
  text-transform: uppercase; color: var(--ink-tertiary); padding: 6px 8px;
  border-bottom: 1px solid var(--hairline-strong); }
.cat-table td { padding: 7px 8px; border-bottom: 1px solid var(--hairline); color: var(--ink-secondary); }
.cat-table td:first-child { color: var(--ink); }
.cat-list { margin: 0; padding-left: 18px; display: grid; gap: 4px;
  font: 400 12.5px/1.5 var(--text); color: var(--ink-secondary); }
.cat-list li::marker { color: var(--ink-tertiary); }
.cat-chart { display: flex; align-items: flex-end; gap: 6px; height: 76px; padding: 8px 10px;
  border: 1px solid var(--hairline); border-radius: var(--radius-sm); background: var(--surface); }
.cat-chart-col { flex: 1; display: grid; justify-items: center; gap: 4px; height: 100%; align-content: end; }
.cat-chart-bar { width: 100%; border-radius: 3px 3px 0 0;
  background: color-mix(in srgb, var(--accent) 72%, var(--surface)); }
.cat-chart-label { font: 500 9px/1 var(--mono); color: var(--ink-tertiary); }
.cat-timeline { display: grid; }
.cat-timeline-item { display: grid; grid-template-columns: 12px 1fr; gap: 10px;
  position: relative; padding-bottom: 12px; }
.cat-timeline-item::before { content: ""; position: absolute; left: 5px; top: 12px; bottom: 0;
  width: 1px; background: var(--hairline-strong); }
.cat-timeline-item:last-child::before { display: none; }
.cat-timeline-dot { width: 11px; height: 11px; border-radius: 50%; background: var(--accent); margin-top: 3px; }
.cat-timeline-label { font: 600 12.5px/1.3 var(--text); }
.cat-timeline-note { font: 400 11.5px/1.45 var(--text); color: var(--ink-tertiary); }
.cat-comparison { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.cat-comparison-side { border: 1px solid var(--hairline); border-radius: var(--radius-sm);
  padding: 10px 12px; display: grid; gap: 4px; align-content: start; background: var(--surface); }
.cat-comparison-side[data-tone="better"] { border-color: color-mix(in srgb, var(--ok) 40%, transparent); }
.cat-image { width: 100%; border-radius: var(--radius-sm); border: 1px solid var(--hairline);
  background: var(--raised); display: block; object-fit: cover; }
.cat-link { font: 500 13px/1.4 var(--text); color: var(--accent); text-decoration: none;
  border-bottom: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); }
.cat-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  padding: 7px 14px; border-radius: var(--radius-sm); background: var(--accent);
  color: var(--on-accent); font: 600 12.5px/1 var(--text); border: 0; cursor: pointer; width: max-content; }
.cat-button[data-tone="secondary"] { background: var(--raised); color: var(--ink); }
.cat-button[data-tone="danger"] { background: var(--down); color: var(--on-accent); }
.cat-form-field { display: grid; gap: 4px; }
.cat-form-label { font: 600 11px/1.4 var(--text); color: var(--ink-secondary); }
.cat-form-input { padding: 7px 10px; border-radius: var(--radius-sm);
  border: 1px solid var(--hairline-strong); background: var(--surface);
  color: var(--ink); font: 400 13px/1.4 var(--text); }
.cat-form-input::placeholder { color: var(--ink-tertiary); }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(json: &str) -> UiSpec {
        serde_json::from_str(json).expect("the fixture parses")
    }

    /// The known-good fixture every criterion shares: a container with a
    /// heading, a metric, a text and a button.
    fn good() -> UiSpec {
        spec(
            r#"{
              "root": "page",
              "elements": {
                "page": { "type": "container", "props": { "dir": "column" },
                          "children": ["head", "metric", "body", "cta"] },
                "head": { "type": "heading", "props": { "text": "Q3 revenue review", "level": "1" }, "children": [] },
                "metric": { "type": "metric", "props": { "label": "Revenue", "value": "$12M", "delta": "+34% YoY" }, "children": [] },
                "body": { "type": "text", "props": { "text": "Revenue grew 34% year over year." }, "children": [] },
                "cta": { "type": "button", "props": { "label": "Export", "action": "export_report" }, "children": [] }
              }
            }"#,
        )
    }

    #[test]
    fn the_catalog_table_is_data_driven() {
        let mut seen = HashSet::new();
        for c in CATALOG {
            assert!(
                seen.insert(c.kind),
                "the kind `{}` is declared twice",
                c.kind
            );
            assert!(
                !c.description.is_empty(),
                "`{}` carries no description",
                c.kind
            );
            assert!(!c.props.is_empty(), "`{}` carries no props", c.kind);
            let mut prop_names = HashSet::new();
            for p in c.props {
                assert!(
                    prop_names.insert(p.name),
                    "`{}` declares `{}` twice",
                    c.kind,
                    p.name
                );
            }
        }
    }

    #[test]
    fn the_spec_round_trips() {
        let original = good();
        let json = serde_json::to_string(&original).expect("it serializes");
        let back: UiSpec = serde_json::from_str(&json).expect("it parses");
        assert_eq!(back.root, original.root);
        assert_eq!(back.elements.len(), original.elements.len());
        for (id, el) in &original.elements {
            let other = back.elements.get(id).expect("the element survives");
            assert_eq!(other.kind, el.kind);
            assert_eq!(other.props, el.props);
            assert_eq!(other.children, el.children);
        }
    }

    #[test]
    fn the_validator_refuses() {
        let unknown = spec(
            r#"{ "root": "a", "elements": { "a": { "type": "hero", "props": {}, "children": [] } } }"#,
        );
        let faults = check(&unknown);
        assert!(
            faults
                .iter()
                .any(|f| f.fault.contains("is not a catalog kind") && f.element == "a"),
            "an unknown kind is refused by name: {faults:?}"
        );

        let missing = spec(
            r#"{ "root": "a", "elements": { "a": { "type": "heading", "props": {}, "children": [] } } }"#,
        );
        let faults = check(&missing);
        assert!(
            faults
                .iter()
                .any(|f| f.fault.contains("`text` is required") && f.element == "a"),
            "a missing required prop is refused by name: {faults:?}"
        );

        let wrong = spec(
            r#"{ "root": "a", "elements": { "a": { "type": "heading", "props": { "text": 7 }, "children": [] } } }"#,
        );
        let faults = check(&wrong);
        assert!(
            faults
                .iter()
                .any(|f| f.fault.contains("`text` must be a string")),
            "a wrong prop type is refused by name: {faults:?}"
        );

        let dangling = spec(
            r#"{ "root": "a", "elements": { "a": { "type": "container", "props": {}, "children": ["ghost"] } } }"#,
        );
        let faults = check(&dangling);
        assert!(
            faults
                .iter()
                .any(|f| f.fault.contains("the child `ghost` is not an element")),
            "a dangling child is refused by name: {faults:?}"
        );
    }

    #[test]
    fn a_known_good_spec_passes() {
        assert!(check(&good()).is_empty(), "the fixture carries no fault");
    }

    #[test]
    fn the_render_is_a_complete_document_under_the_cap() {
        let html = render(&good());
        assert!(html.starts_with("<!doctype html>"), "it is a document");
        assert!(html.contains("</html>"), "it closes");
        for id in ["page", "head", "metric", "body", "cta"] {
            assert!(
                html.contains(&format!("data-cat=\"{id}\"")),
                "the element `{id}` is drawn"
            );
        }
        assert!(html.len() <= MAX_HTML_BYTES, "the render is under the cap");
    }

    #[test]
    fn the_prompt_names_every_kind_with_its_description() {
        let prompt = prompt_text();
        for c in CATALOG {
            assert!(prompt.contains(c.kind), "the prompt names `{}`", c.kind);
            assert!(
                prompt.contains(c.description),
                "the prompt carries `{}`'s description",
                c.kind
            );
        }
    }

    #[test]
    fn a_cycle_is_refused() {
        let cyclic = spec(
            r#"{
              "root": "a",
              "elements": {
                "a": { "type": "container", "props": {}, "children": ["b"] },
                "b": { "type": "container", "props": {}, "children": ["a"] }
              }
            }"#,
        );
        let faults = check(&cyclic);
        assert!(
            faults
                .iter()
                .any(|f| f.fault.contains("the tree has a cycle")),
            "a cycle is refused: {faults:?}"
        );
    }

    #[test]
    fn a_child_two_parents_share_is_refused_and_named_apart_from_a_cycle() {
        let shared = spec(
            r#"{
              "root": "a",
              "elements": {
                "a": { "type": "container", "props": {}, "children": ["b", "c"] },
                "b": { "type": "container", "props": {}, "children": ["d"] },
                "c": { "type": "container", "props": {}, "children": ["d"] },
                "d": { "type": "text", "props": { "text": "drawn twice" }, "children": [] }
              }
            }"#,
        );
        let faults = check(&shared);
        assert!(
            faults
                .iter()
                .any(|f| f.element == "d" && f.fault.contains("a shared child draws twice")),
            "a shared child is named: {faults:?}"
        );
        assert!(
            !faults
                .iter()
                .any(|f| f.fault.contains("the tree has a cycle")),
            "a shared child is not a cycle: {faults:?}"
        );
    }

    #[test]
    fn an_unreachable_element_is_refused() {
        let orphan = spec(
            r#"{
              "root": "a",
              "elements": {
                "a": { "type": "heading", "props": { "text": "hi" }, "children": [] },
                "ghost": { "type": "text", "props": { "text": "no one links me" }, "children": [] }
              }
            }"#,
        );
        let faults = check(&orphan);
        assert!(
            faults
                .iter()
                .any(|f| f.element == "ghost" && f.fault.contains("not reachable")),
            "an unreachable element is named: {faults:?}"
        );
    }
}
