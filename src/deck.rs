//! Visual deliverables: the structure that separates content from design.
//!
//! A three-stage pipeline — research, structure, render — and this module
//! is the middle one, the stage that decides whether the output looks
//! considered or generated. The research stage gathers ground truth (the
//! agent already searches, recalls and fact-checks); the render stage
//! draws (HTML first, because the browser face renders HTML with a
//! three-tier token chain already; PPTX and image engines later). What
//! lives here is the interchange between them: a rigid schema that
//! forces content into atomic visual components *before* any layout is
//! chosen, and the consultant constraints that make the result read like
//! it was structured by someone who had a point.
//!
//! The constraints are the whole reason this module exists rather than a
//! template string. The Pyramid Principle — every block opens with its
//! conclusion — MECE, so sub-points do not overlap — and a bolding rule,
//! so a skim reads the argument. Each is a check that refuses, not a
//! suggestion in a prompt, because a constraint a model can ignore is
//! decoration.
//!
//! Fleet is one static Rust binary with no checkout dependency, so the
//! schema is plain serde and the checks are plain functions. No Python,
//! no template engine, no second store: the deck is a value the agent
//! builds and a surface renders.

use serde::{Deserialize, Serialize};

/// What kind of visual deliverable this is. The research stage routes on
/// it — an infographic is one high-density flow, a presentation is a
/// linear narrative, a poster is a single frame — and the render stage
/// picks its page geometry from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// A linear narrative: one idea per page, paced.
    Presentation,
    /// A single high-density flow: everything at once, structured.
    Infographic,
    /// One frame, read from across a room.
    Poster,
}

impl ProjectType {
    /// The word the schema and the surfaces carry.
    pub fn label(self) -> &'static str {
        match self {
            ProjectType::Presentation => "presentation",
            ProjectType::Infographic => "infographic",
            ProjectType::Poster => "poster",
        }
    }
}

/// The page geometry a node is drawn in. A layout is chosen from the
/// content's shape rather than the other way round, which is what keeps a
/// metric callout from being squeezed into a bullet list because the
/// template said so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// A conclusion beside its evidence.
    SplitTwoColumn,
    /// One number, large, with its label.
    MetricCallout,
    /// Three or four figures side by side.
    MetricRow,
    /// A sequence, left to right.
    Timeline,
    /// Two states, contrasted.
    Comparison,
    /// A claim, set apart.
    Quote,
    /// A chart with its caption.
    Chart,
    /// The whole frame, for an infographic's spine.
    FullBleed,
}

impl Layout {
    /// The word the schema carries.
    pub fn label(self) -> &'static str {
        match self {
            Layout::SplitTwoColumn => "split_two_column",
            Layout::MetricCallout => "metric_callout",
            Layout::MetricRow => "metric_row",
            Layout::Timeline => "timeline",
            Layout::Comparison => "comparison",
            Layout::Quote => "quote",
            Layout::Chart => "chart",
            Layout::FullBleed => "full_bleed",
        }
    }
}

/// One atomic visual component. The set is deliberately small: a
/// component the renderer cannot draw is a component the planner should
/// not have been allowed to ask for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    /// A list of points, each carrying its own emphasis.
    BulletList { items: Vec<String> },
    /// One figure and what it measures.
    Metric { value: String, label: String },
    /// Several figures, peer to peer.
    Metrics { items: Vec<(String, String)> },
    /// A sequence of labelled steps.
    Steps { items: Vec<String> },
    /// Two contrasted lists.
    Contrast { left: String, right: String },
    /// A claim and who made it.
    Quote { text: String, attribution: String },
    /// A chart's caption and the series it draws. The data itself rides
    /// beside it, because a caption without its numbers is decoration.
    Chart {
        caption: String,
        series: Vec<(String, f64)>,
    },
    /// Prose, for the one paragraph that has to be a paragraph.
    Prose { text: String },
}

/// One page or one visual block. A node is self-contained: it carries
/// everything the render needs, so a renderer never walks backwards for
/// context and a node can be reordered or dropped without breaking its
/// neighbours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub layout: Layout,
    /// The small label above the headline: where in the argument this
    /// block sits.
    #[serde(default)]
    pub eyebrow: String,
    /// The conclusion this block opens with. Required, and required to be
    /// a conclusion — see [`check_pyramid`].
    pub headline: String,
    #[serde(default)]
    pub components: Vec<Component>,
}

/// A whole deliverable: what kind it is, the theme it renders in, and
/// its nodes in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deck {
    pub project_type: ProjectType,
    /// The theme name the render stage resolves. The tokens are the
    /// renderer's; the deck names which set it wants.
    #[serde(default = "default_theme")]
    pub theme: String,
    pub nodes: Vec<Node>,
}

/// The default theme: the face's own dark ground, so a deck rendered
/// without a theme still looks like it belongs to the app it was made in.
fn default_theme() -> String {
    "console".to_string()
}

/// The bounds the planner enforces. A deck that breaks one is refused
/// with the bound named, rather than rendered and hoped for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The most nodes one deck may carry. An infographic is one frame; a
    /// presentation is paced. Both have a ceiling.
    pub max_nodes: usize,
    /// The fewest. A deck with nothing in it is not a deck.
    pub min_nodes: usize,
    /// The most words one headline may carry — a conclusion, not a
    /// paragraph.
    pub max_headline_words: usize,
    /// The most components one node may carry.
    pub max_components: usize,
}

impl Bounds {
    /// The bounds for one project type.
    pub fn for_type(project_type: ProjectType) -> Bounds {
        match project_type {
            ProjectType::Presentation => Bounds {
                max_nodes: 40,
                min_nodes: 2,
                max_headline_words: 18,
                max_components: 4,
            },
            ProjectType::Infographic => Bounds {
                max_nodes: 1,
                min_nodes: 1,
                max_headline_words: 14,
                max_components: 8,
            },
            ProjectType::Poster => Bounds {
                max_nodes: 1,
                min_nodes: 1,
                max_headline_words: 10,
                max_components: 3,
            },
        }
    }
}

/// Why a deck was refused. Every refusal names the node and the rule, so
/// the agent that built it can fix the one thing rather than start again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A node's headline is not a conclusion.
    Pyramid { node: String, reason: String },
    /// Two sub-points overlap enough to be the same point.
    Overlap {
        node: String,
        left: String,
        right: String,
    },
    /// A bullet carries no emphasis, so a skim would miss it.
    Unemphasised { node: String, item: String },
    /// A bound was broken.
    Bound { node: String, reason: String },
    /// A node id is missing or duplicated.
    Identity { reason: String },
}

impl Refusal {
    /// The refusal as one line, the way every other model-visible
    /// failure in this crate reads.
    pub fn line(&self) -> String {
        match self {
            Refusal::Pyramid { node, reason } => {
                format!("`{node}`: the headline is not a conclusion — {reason}")
            }
            Refusal::Overlap { node, left, right } => format!(
                "`{node}`: `{left}` and `{right}` overlap — sub-points must be mutually exclusive"
            ),
            Refusal::Unemphasised { node, item } => {
                format!("`{node}`: `{item}` carries no **bold** — a skim would miss the point")
            }
            Refusal::Bound { node, reason } => format!("`{node}`: {reason}"),
            Refusal::Identity { reason } => format!("the deck's nodes: {reason}"),
        }
    }
}

/// The words that carry no content, so two points sharing only them are
/// not the same point. Small on purpose: an aggressive stop list would
/// refuse legitimate overlap.
const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "are", "was", "were", "has", "have",
    "its", "their", "than", "then", "into", "over", "under", "about", "when", "what", "which",
    "while",
];

/// One string's content words: lowercased, punctuation stripped, stop
/// words dropped, deduplicated. Two points are compared on these, so
/// "the 54% increase in APAC" and "APAC sales velocity rose 54%" share
/// the words that make them the same claim.
fn content_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in text
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '%')
        .filter(|word| !word.is_empty())
    {
        if STOP_WORDS.contains(&word) {
            continue;
        }
        if !words.iter().any(|seen| seen == word) {
            words.push(word.to_string());
        }
    }
    words
}

/// How much two points overlap, as the shared content words over the
/// smaller set. Zero when either side has no content words at all — two
/// empty points are not the same point, they are absent.
fn overlap(left: &str, right: &str) -> f64 {
    let left = content_words(left);
    let right = content_words(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let shared = left
        .iter()
        .filter(|word| right.iter().any(|other| other == *word))
        .count();
    shared as f64 / left.len().min(right.len()) as f64
}

/// The overlap at which two sub-points read as the same point.
const MECE_OVERLAP: f64 = 0.6;

/// Whether one bullet carries emphasis. The rule is that a skim reads
/// the argument, so the words that matter are marked.
fn emphasised(item: &str) -> bool {
    item.contains("**")
}

/// Whether one headline is a conclusion rather than a topic or a
/// question. The Pyramid Principle's test, reduced to what a string can
/// show: it asserts something, and it does not ask.
fn is_conclusion(headline: &str) -> bool {
    let trimmed = headline.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with('?') {
        return false;
    }
    // A bare topic is a label, not a conclusion: it has no verb. The
    // cheap proxy is that a conclusion is more than a few words and is
    // not only capitalised nouns — so require some length and at least
    // one lower-case word, which a noun phrase like "Market Analysis
    // 2026" does not have.
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 3 {
        return false;
    }
    words.iter().any(|word| {
        word.chars()
            .next()
            .is_some_and(|first| first.is_lowercase())
    })
}

/// The words one string carries.
fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Check one deck against the consultant constraints and the bounds for
/// its type. An empty result means it is ready to render.
pub fn check(deck: &Deck) -> Vec<Refusal> {
    let mut refusals: Vec<Refusal> = Vec::new();
    let bounds = Bounds::for_type(deck.project_type);

    // Identity first: an id that is missing or duplicated makes every
    // other refusal ambiguous.
    let mut seen: Vec<&str> = Vec::new();
    for node in &deck.nodes {
        if node.id.trim().is_empty() {
            refusals.push(Refusal::Identity {
                reason: "a node carries no id".to_string(),
            });
        } else if seen.contains(&node.id.as_str()) {
            refusals.push(Refusal::Identity {
                reason: format!("`{}` appears twice", node.id),
            });
        } else {
            seen.push(&node.id);
        }
    }

    if deck.nodes.len() > bounds.max_nodes {
        refusals.push(Refusal::Bound {
            node: "the deck".to_string(),
            reason: format!(
                "a {} carries at most {} nodes, and this one carries {}",
                deck.project_type.label(),
                bounds.max_nodes,
                deck.nodes.len()
            ),
        });
    }
    if deck.nodes.len() < bounds.min_nodes {
        refusals.push(Refusal::Bound {
            node: "the deck".to_string(),
            reason: format!(
                "a {} carries at least {} nodes, and this one carries {}",
                deck.project_type.label(),
                bounds.min_nodes,
                deck.nodes.len()
            ),
        });
    }

    for node in &deck.nodes {
        // The pyramid: the headline opens with the conclusion.
        if !is_conclusion(&node.headline) {
            refusals.push(Refusal::Pyramid {
                node: node.id.clone(),
                reason: "state the conclusion as a sentence, not a topic or a question".to_string(),
            });
        }
        if words(&node.headline) > bounds.max_headline_words {
            refusals.push(Refusal::Bound {
                node: node.id.clone(),
                reason: format!(
                    "the headline carries {} words, past the {}-word bound",
                    words(&node.headline),
                    bounds.max_headline_words
                ),
            });
        }
        if node.components.len() > bounds.max_components {
            refusals.push(Refusal::Bound {
                node: node.id.clone(),
                reason: format!(
                    "{} components, past the {}-component bound",
                    node.components.len(),
                    bounds.max_components
                ),
            });
        }

        // MECE, over every pair of sub-points the node carries: a bullet
        // against every other bullet, and a bullet against a contrast's
        // sides. Two points that say the same thing are one point.
        let mut points: Vec<String> = Vec::new();
        for component in &node.components {
            match component {
                Component::BulletList { items } => points.extend(items.iter().cloned()),
                Component::Steps { items } => points.extend(items.iter().cloned()),
                Component::Contrast { left, right } => {
                    points.push(left.clone());
                    points.push(right.clone());
                }
                _ => {}
            }
        }
        for (index, left) in points.iter().enumerate() {
            for right in &points[index + 1..] {
                if overlap(left, right) >= MECE_OVERLAP {
                    refusals.push(Refusal::Overlap {
                        node: node.id.clone(),
                        left: clip(left, 40),
                        right: clip(right, 40),
                    });
                }
            }
        }

        // The bolding rule, on bullets: a skim reads the marked words.
        for component in &node.components {
            if let Component::BulletList { items } = component {
                for item in items {
                    if !emphasised(item) {
                        refusals.push(Refusal::Unemphasised {
                            node: node.id.clone(),
                            item: clip(item, 40),
                        });
                    }
                }
            }
        }
    }

    refusals
}

/// One string clipped to `most` characters with a marked cut, the way
/// every other bounded string in this crate reads.
fn clip(text: &str, most: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= most {
        return text.to_string();
    }
    let mut out: String = text.chars().take(most.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// The deck as one line the agent can read: how many nodes, what kind,
/// and the refusals if there are any. This is the tool's own answer, in
/// the shape every other tool result in this crate takes.
pub fn summary(deck: &Deck) -> String {
    let refusals = check(deck);
    if refusals.is_empty() {
        return format!(
            "a {} of {} node(s), ready to render in the `{}` theme",
            deck.project_type.label(),
            deck.nodes.len(),
            deck.theme
        );
    }
    let named: Vec<String> = refusals.iter().map(Refusal::line).collect();
    format!("refused: {} — {}", refusals.len(), named.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, headline: &str, components: Vec<Component>) -> Node {
        Node {
            id: id.to_string(),
            layout: Layout::SplitTwoColumn,
            eyebrow: String::new(),
            headline: headline.to_string(),
            components,
        }
    }

    fn deck(nodes: Vec<Node>) -> Deck {
        Deck {
            project_type: ProjectType::Presentation,
            theme: "console".to_string(),
            nodes,
        }
    }

    #[test]
    fn a_well_formed_deck_passes_every_constraint() {
        let good = deck(vec![
            node(
                "slide_01",
                "Q3 growth came from regional expansion, not pricing",
                vec![
                    Component::BulletList {
                        items: vec![
                            "**54% increase** in APAC sales velocity".to_string(),
                            "**Cloud adoption** outpaced local deployments".to_string(),
                        ],
                    },
                    Component::Metric {
                        value: "$4.2M".to_string(),
                        label: "Net new ARR".to_string(),
                    },
                ],
            ),
            node(
                "slide_02",
                "The reserve covers two quarters at current burn",
                vec![Component::BulletList {
                    items: vec!["**18 months** of runway at plan spend".to_string()],
                }],
            ),
        ]);
        assert!(check(&good).is_empty(), "{:?}", check(&good));
        assert!(
            summary(&good).contains("ready to render"),
            "{}",
            summary(&good)
        );
    }

    #[test]
    fn the_pyramid_refuses_a_topic_and_a_question() {
        // A bare label is not a conclusion.
        let topic = deck(vec![node(
            "slide_01",
            "Market Analysis 2026",
            vec![Component::BulletList {
                items: vec!["**one** point".to_string()],
            }],
        )]);
        assert!(
            check(&topic)
                .iter()
                .any(|r| matches!(r, Refusal::Pyramid { .. })),
            "{:?}",
            check(&topic)
        );
        // Neither is a question.
        let question = deck(vec![node(
            "slide_01",
            "What drove Q3 growth?",
            vec![Component::BulletList {
                items: vec!["**one** point".to_string()],
            }],
        )]);
        assert!(
            check(&question)
                .iter()
                .any(|r| matches!(r, Refusal::Pyramid { .. })),
            "{:?}",
            check(&question)
        );
    }

    #[test]
    fn mece_refuses_two_points_that_say_the_same_thing() {
        let same = deck(vec![node(
            "slide_01",
            "Growth came from two regions, not one",
            vec![Component::BulletList {
                items: vec![
                    "**54% increase** in APAC sales velocity".to_string(),
                    "**APAC sales** velocity rose **54%** in the quarter".to_string(),
                ],
            }],
        )]);
        let refusals = check(&same);
        assert!(
            refusals
                .iter()
                .any(|r| matches!(r, Refusal::Overlap { .. })),
            "{:?}",
            refusals
        );
        // And genuinely distinct points pass.
        let distinct = deck(vec![node(
            "slide_01",
            "Growth came from two regions, not one",
            vec![Component::BulletList {
                items: vec![
                    "**54% increase** in APAC sales velocity".to_string(),
                    "**Cloud adoption** outpaced local deployments".to_string(),
                ],
            }],
        )]);
        assert!(
            !check(&distinct)
                .iter()
                .any(|r| matches!(r, Refusal::Overlap { .. })),
            "{:?}",
            check(&distinct)
        );
    }

    #[test]
    fn the_bolding_rule_refuses_an_unmarked_bullet() {
        let flat = deck(vec![node(
            "slide_01",
            "Two things moved, and one of them mattered",
            vec![Component::BulletList {
                items: vec!["APAC sales velocity rose 54%".to_string()],
            }],
        )]);
        assert!(
            check(&flat)
                .iter()
                .any(|r| matches!(r, Refusal::Unemphasised { .. })),
            "{:?}",
            check(&flat)
        );
    }

    #[test]
    fn the_bounds_differ_by_project_type() {
        // An infographic is one frame.
        let many = Deck {
            project_type: ProjectType::Infographic,
            theme: "console".to_string(),
            nodes: vec![
                node("one", "Everything on one frame, read at a glance", vec![]),
                node(
                    "two",
                    "A second frame is not an infographic any more",
                    vec![],
                ),
            ],
        };
        assert!(
            check(&many)
                .iter()
                .any(|r| matches!(r, Refusal::Bound { .. })),
            "{:?}",
            check(&many)
        );
        // A presentation needs more than one.
        let one = Deck {
            project_type: ProjectType::Presentation,
            theme: "console".to_string(),
            nodes: vec![node(
                "only",
                "A one-slide deck is a poster, not a deck",
                vec![],
            )],
        };
        assert!(
            check(&one)
                .iter()
                .any(|r| matches!(r, Refusal::Bound { .. })),
            "{:?}",
            check(&one)
        );
    }

    #[test]
    fn a_duplicate_or_missing_id_is_refused_by_name() {
        let dup = deck(vec![
            node("slide_01", "The first conclusion, stated plainly", vec![]),
            node("slide_01", "The second conclusion, also stated", vec![]),
        ]);
        assert!(
            check(&dup)
                .iter()
                .any(|r| matches!(r, Refusal::Identity { .. })),
            "{:?}",
            check(&dup)
        );
        let blank = deck(vec![node(
            "  ",
            "A conclusion with no id behind it",
            vec![],
        )]);
        assert!(
            check(&blank)
                .iter()
                .any(|r| matches!(r, Refusal::Identity { .. })),
            "{:?}",
            check(&blank)
        );
    }

    #[test]
    fn the_schema_round_trips_through_json() {
        // The interchange is the point: content leaves the planner as
        // JSON and the renderer reads it back, so the two never share a
        // Rust type.
        let original = deck(vec![node(
            "slide_01",
            "Q3 growth came from regional expansion",
            vec![
                Component::BulletList {
                    items: vec!["**54% increase** in APAC".to_string()],
                },
                Component::Chart {
                    caption: "ARR by quarter".to_string(),
                    series: vec![("Q1".to_string(), 1.2), ("Q2".to_string(), 2.8)],
                },
            ],
        )]);
        let text = serde_json::to_string(&original).expect("the deck serialises");
        let back: Deck = serde_json::from_str(&text).expect("the deck reads back");
        assert_eq!(original, back);
        // And the owner's own schema shape is what rides the wire.
        let value: serde_json::Value = serde_json::from_str(&text).expect("as json");
        assert_eq!(value["project_type"], "presentation");
        assert_eq!(value["nodes"][0]["layout"], "split_two_column");
        assert_eq!(value["nodes"][0]["components"][1]["type"], "chart");
    }

    #[test]
    fn overlap_ignores_stop_words_and_rewards_shared_content() {
        // Two points that share only stop words do not overlap.
        assert_eq!(overlap("the and for with", "that this from are"), 0.0);
        // Two empty points are absent, not identical.
        assert_eq!(overlap("", "anything"), 0.0);
        // The same claim in two vocabularies overlaps heavily.
        let a = overlap(
            "54% increase in APAC sales velocity",
            "APAC sales velocity rose 54%",
        );
        assert!(a >= MECE_OVERLAP, "{a}");
    }
}
