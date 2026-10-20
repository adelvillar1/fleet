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
    /// Whether this refusal names the node with this id. Deck-level
    /// refusals (the type bounds) name no node, and a node-scope repair
    /// is not answerable for them.
    pub fn names_node(&self, id: &str) -> bool {
        match self {
            Refusal::Pyramid { node, .. }
            | Refusal::Overlap { node, .. }
            | Refusal::Unemphasised { node, .. }
            | Refusal::Bound { node, .. } => node == id,
            Refusal::Identity { .. } => false,
        }
    }

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

/// Render one deck as an HTML fragment.
///
/// The fragment references the face's own alias tokens rather than its
/// own palette, so a deck rendered inside the browser face inherits the
/// theme switch for free — the same three-tier chain the rest of the
/// surface uses, and the reason a deck never needs a second palette to
/// keep in step. The classes are `deck-*` and the stylesheet owns them,
/// so the renderer carries no CSS and the design lives in one place.
///
/// A deck is escaped on the way out. Everything the planner produced
/// came from a model, and a deck is rendered into a page that also runs
/// a conversation: the content is data, never markup.
pub fn render(deck: &Deck) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<section class=\"deck\" data-type=\"{}\" data-theme-name=\"{}\">",
        deck.project_type.label(),
        escape(&deck.theme)
    ));
    for node in &deck.nodes {
        out.push_str(&render_node(node));
    }
    out.push_str("</section>");
    out
}

/// One node: its eyebrow, its conclusion, and its components in the
/// layout the planner chose.
fn render_node(node: &Node) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<article class=\"deck-node\" data-layout=\"{}\" id=\"{}\">",
        node.layout.label(),
        escape(&node.id)
    ));
    if !node.eyebrow.trim().is_empty() {
        out.push_str(&format!(
            "<p class=\"deck-eyebrow\">{}</p>",
            escape(&node.eyebrow)
        ));
    }
    out.push_str(&format!(
        "<h3 class=\"deck-headline\">{}</h3>",
        inline(&node.headline)
    ));
    if !node.components.is_empty() {
        out.push_str("<div class=\"deck-components\">");
        for component in &node.components {
            out.push_str(&render_component(component));
        }
        out.push_str("</div>");
    }
    out.push_str("</article>");
    out
}

/// One atomic component.
fn render_component(component: &Component) -> String {
    match component {
        Component::BulletList { items } => {
            let mut out = String::from("<ul class=\"deck-bullets\">");
            for item in items {
                out.push_str(&format!("<li>{}</li>", inline(item)));
            }
            out.push_str("</ul>");
            out
        }
        Component::Metric { value, label } => format!(
            "<div class=\"deck-metric\"><b class=\"deck-value\">{}</b><span class=\"deck-label\">{}</span></div>",
            escape(value),
            escape(label)
        ),
        Component::Metrics { items } => {
            let mut out = String::from("<div class=\"deck-metrics\">");
            for (value, label) in items {
                out.push_str(&format!(
                    "<div class=\"deck-metric\"><b class=\"deck-value\">{}</b><span class=\"deck-label\">{}</span></div>",
                    escape(value),
                    escape(label)
                ));
            }
            out.push_str("</div>");
            out
        }
        Component::Steps { items } => {
            let mut out = String::from("<ol class=\"deck-steps\">");
            for (index, item) in items.iter().enumerate() {
                out.push_str(&format!(
                    "<li><span class=\"deck-step-no\">{}</span>{}</li>",
                    index + 1,
                    inline(item)
                ));
            }
            out.push_str("</ol>");
            out
        }
        Component::Contrast { left, right } => format!(
            "<div class=\"deck-contrast\"><div class=\"deck-side\">{}</div><div class=\"deck-side\">{}</div></div>",
            inline(left),
            inline(right)
        ),
        Component::Quote { text, attribution } => format!(
            "<blockquote class=\"deck-quote\"><p>{}</p><cite>{}</cite></blockquote>",
            inline(text),
            escape(attribution)
        ),
        Component::Chart { caption, series } => {
            let peak = series
                .iter()
                .map(|(_, value)| *value)
                .fold(0.0f64, |peak, value| peak.max(value));
            let mut out = String::from("<figure class=\"deck-chart\">");
            out.push_str(&format!(
                "<figcaption class=\"deck-caption\">{}</figcaption>",
                escape(caption)
            ));
            out.push_str("<div class=\"deck-bars\">");
            for (label, value) in series {
                // A bar's height is its share of the peak, so the tallest
                // series fills the track and the rest read against it. A
                // zero peak draws nothing rather than a full bar.
                let share = if peak > 0.0 { value / peak } else { 0.0 };
                out.push_str(&format!(
                    "<div class=\"deck-bar\"><span class=\"deck-bar-value\">{}</span><span class=\"deck-bar-track\"><span class=\"deck-bar-fill\" style=\"height:{:.0}%\"></span></span><span class=\"deck-bar-label\">{}</span></div>",
                    escape(&format_number(*value)),
                    share * 100.0,
                    escape(label)
                ));
            }
            out.push_str("</div></figure>");
            out
        }
        Component::Prose { text } => {
            format!("<p class=\"deck-prose\">{}</p>", inline(text))
        }
    }
}

/// One number as the deck carries it: whole when it is whole, one
/// decimal when it is not, and never scientific notation on a slide.
fn format_number(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

/// Inline emphasis: `**bold**` becomes `<strong>`, and everything else
/// is escaped text. The same rule the constraint checker enforces, so a
/// bullet the checker accepted is a bullet the renderer marks.
pub fn inline(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("**") {
        if at > 0 {
            out.push_str(&escape(&rest[..at]));
        }
        rest = &rest[at + 2..];
        match rest.find("**") {
            Some(end) => {
                out.push_str(&format!("<strong>{}</strong>", escape(&rest[..end])));
                rest = &rest[end + 2..];
            }
            // An unclosed marker is literal text, not a broken tag.
            None => {
                out.push_str("**");
                break;
            }
        }
    }
    out.push_str(&escape(rest));
    out
}

/// The five XML escapes. A deck is rendered into a page that also runs a
/// conversation, so content that arrived from a model is data here.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(character),
        }
    }
    out
}

// --- the layout grader -----------------------------------------------------

/// The line height a component's text sets at, in pixels. Every estimate
/// below is against this and the box height, because an estimate against
/// an unknown width is a guess.
const LINE_PX: f64 = 19.0;

/// The height a component's box has, in pixels: the slide's body below
/// the headline.
const BOX_PX: f64 = 264.0;

/// How many characters one line holds at the component size. The mono
/// face is narrower than the sans, but a deck's prose is the sans, and an
/// estimate that is one character pessimistic is an estimate that refuses
/// nothing it should have caught.
/// How many characters one line holds at the component size — public
/// because the tool's refusal carries the numbers an owner repairs
/// against, and a constant the caller has to guess is a bound that
/// reads as arbitrary.
pub const CHARS_PER_LINE: usize = 52;

/// The lines one component's box holds: the body height at the line
/// height, floored. The other half of the repair budget.
pub fn layout_holds() -> usize {
    (BOX_PX / LINE_PX).floor() as usize
}

/// One layout fault the grader found: the deck is structurally valid —
/// [`check`] passed — and it will still render badly.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutFault {
    /// A component's text will not fit the box it renders in.
    Overfull {
        node: String,
        /// Which component, by its position in the node.
        component: usize,
        /// The estimated lines against the lines the box holds.
        lines: usize,
        holds: usize,
    },
    /// One column of a multi-column node carries far more than the other,
    /// so the node reads lopsided.
    Unbalanced {
        node: String,
        /// The estimated line counts, heaviest first.
        columns: Vec<usize>,
    },
    /// The headline sets to more lines than its own box holds, so it
    /// collides with the components beneath it.
    HeadlineOverflow {
        node: String,
        lines: usize,
        holds: usize,
    },
}

impl LayoutFault {
    /// The fault as one line, the way every other refusal in this crate
    /// reads.
    pub fn line(&self) -> String {
        match self {
            LayoutFault::Overfull {
                node,
                component,
                lines,
                holds,
            } => format!(
                "`{node}`: component {component} sets to about {lines} lines in a box that holds {holds}"
            ),
            LayoutFault::Unbalanced { node, columns } => format!(
                "`{node}`: the columns set to {columns:?} lines \u{2014} one side is carrying the node"
            ),
            LayoutFault::HeadlineOverflow { node, lines, holds } => format!(
                "`{node}`: the headline sets to about {lines} lines in a box that holds {holds}"
            ),
        }
    }
}

/// How many lines one component's text sets to at the column width. An
/// empty component is one line, not zero: an empty box is a box.
fn component_lines(component: &Component) -> usize {
    let chars = match component {
        Component::BulletList { items } => items
            .iter()
            .map(|item| item.chars().count() + 2)
            .sum::<usize>(),
        Component::Metric { value, label } => value.chars().count() + label.chars().count(),
        Component::Metrics { items } => items
            .iter()
            .map(|(value, label)| value.chars().count() + label.chars().count() + 2)
            .sum::<usize>(),
        Component::Steps { items } => items
            .iter()
            .map(|item| item.chars().count() + 3)
            .sum::<usize>(),
        Component::Contrast { left, right } => left.chars().count().max(right.chars().count()),
        Component::Quote { text, attribution } => {
            text.chars().count() + attribution.chars().count() + 2
        }
        Component::Chart { caption, series } => {
            caption.chars().count()
                + series
                    .iter()
                    .map(|(label, _)| label.chars().count() + 8)
                    .sum::<usize>()
        }
        Component::Prose { text } => text.chars().count(),
    };
    chars.div_ceil(CHARS_PER_LINE).max(1)
}

/// How many lines one node's headline sets to at the full node width.
/// A headline is the widest text on the node, so it holds more per line
/// than a component's column does.
fn headline_lines(node: &Node) -> usize {
    let chars = node.headline.chars().count();
    chars.div_ceil(CHARS_PER_LINE * 2).max(1)
}

/// How many lines the headline's own box holds: two, at the size a
/// headline sets at.
const HEADLINE_HOLDS: usize = 2;

/// The ratio at which two columns of one node read as lopsided. The
/// heavier column sets to this many times the lighter one's lines.
const UNBALANCED_RATIO: usize = 3;

/// One fault as the card's JSON: the trigger named, the node and the
/// numbers beside it — the shape a live surface renders without
/// re-deriving anything.
pub fn fault_value(fault: &LayoutFault) -> serde_json::Value {
    match fault {
        LayoutFault::Overfull {
            node,
            component,
            lines,
            holds,
        } => serde_json::json!({
            "trigger": "overfull", "node": node, "component": component,
            "lines": lines, "holds": holds,
        }),
        LayoutFault::Unbalanced { node, columns } => serde_json::json!({
            "trigger": "unbalanced", "node": node, "columns": columns,
        }),
        LayoutFault::HeadlineOverflow { node, lines, holds } => serde_json::json!({
            "trigger": "headline_overflow", "node": node,
            "lines": lines, "holds": holds,
        }),
    }
}

/// Grade one deck's layout: whether its text fits its boxes and its
/// columns carry comparable weight.
///
/// This is the arithmetic half of the review, and it is deliberately not
/// a model call. Fleet renders a deck as HTML, and nothing in a static
/// Rust binary rasterizes HTML — so there is no screenshot for a vision
/// model to look at, and a grader that needed one would be a grader that
/// never runs. The three failure classes the blueprint names are all
/// computable from the structure: overlap is lines-against-box, contrast
/// is already guarded at build time by the theme guard, and balance is
/// one column's weight against its neighbour's. What a vision model adds
/// — and what the `vision` tool is for — is grading a screenshot the
/// owner already has, which is the honest division.
pub fn review(deck: &Deck) -> Vec<LayoutFault> {
    let mut faults: Vec<LayoutFault> = Vec::new();
    let holds = (BOX_PX / LINE_PX).floor() as usize;
    for node in &deck.nodes {
        if headline_lines(node) > HEADLINE_HOLDS {
            faults.push(LayoutFault::HeadlineOverflow {
                node: node.id.clone(),
                lines: headline_lines(node),
                holds: HEADLINE_HOLDS,
            });
        }
        let lines: Vec<usize> = node.components.iter().map(component_lines).collect();
        for (index, set) in lines.iter().enumerate() {
            if *set > holds {
                faults.push(LayoutFault::Overfull {
                    node: node.id.clone(),
                    component: index,
                    lines: *set,
                    holds,
                });
            }
        }
        // Balance, only where there is something to balance: a
        // multi-column node whose heaviest column is several times its
        // lightest reads as one column with a stub beside it.
        if lines.len() > 1 {
            let heaviest = lines.iter().copied().max().unwrap_or(0);
            let lightest = lines.iter().copied().min().unwrap_or(0);
            if lightest > 0 && heaviest >= lightest * UNBALANCED_RATIO {
                let mut ranked = lines.clone();
                ranked.sort_unstable_by(|left, right| right.cmp(left));
                faults.push(LayoutFault::Unbalanced {
                    node: node.id.clone(),
                    columns: ranked,
                });
            }
        }
    }
    faults
}

/// The review as one line the agent can read: what it found, or that the
/// layout holds. The same shape [`summary`] takes.
pub fn review_line(deck: &Deck) -> String {
    let faults = review(deck);
    if faults.is_empty() {
        return format!(
            "the layout holds \u{2014} {} node(s), every component fits its box",
            deck.nodes.len()
        );
    }
    let named: Vec<String> = faults.iter().map(LayoutFault::line).collect();
    format!("refused: {} \u{2014} {}", faults.len(), named.join("; "))
}

// --- the auto-fit ----------------------------------------------------------

impl LayoutFault {
    /// The node the fault belongs to, so the auto-fit can find and repair
    /// it without matching on every variant twice.
    fn node_id(&self) -> &str {
        match self {
            LayoutFault::Overfull { node, .. }
            | LayoutFault::Unbalanced { node, .. }
            | LayoutFault::HeadlineOverflow { node, .. } => node,
        }
    }
}

impl Layout {
    /// The roomier layout a node swaps to when compression would have to
    /// cut away more of the text than the node can lose: the wide frames
    /// give every column more width, and `full_bleed` gives the whole
    /// slide to one node. A layout that is already the roomiest has
    /// nowhere to go, and the compression is what remains.
    pub fn roomier(self) -> Option<Layout> {
        match self {
            Layout::SplitTwoColumn
            | Layout::Comparison
            | Layout::Timeline
            | Layout::MetricRow
            | Layout::Chart => Some(Layout::FullBleed),
            Layout::MetricCallout | Layout::Quote | Layout::FullBleed => None,
        }
    }
}

/// One text clipped to a character budget, keeping what a skim reads.
/// A bolded span — the takeaway the bolding rule requires — survives the
/// clip whole when it fits, and survives as a clipped bold span when it
/// does not, because a truncated bullet that lost its emphasis would
/// also lose the rule that made it scannable.
fn compress_text(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let open = text.find("**");
    let span = open.and_then(|start| {
        text[start + 2..]
            .find("**")
            .map(|end| (start, start + 2 + end + 2))
    });
    if let Some((start, end)) = span {
        if end - start <= budget.saturating_sub(1) {
            let mut out = String::new();
            out.push_str(&text[start..end]);
            out.push('\u{2026}');
            return out;
        }
        let inner = &text[start + 2..end - 2];
        let inner_budget = budget.saturating_sub(5).max(4);
        return format!("**{}\u{2026}**", clip(inner, inner_budget));
    }
    clip(text, budget)
}

/// Compress one component's texts to the line budget the grader
/// measured against — the deterministic half of the self-correction
/// loop, and the half that never needs a model: the same arithmetic the
/// grader used to refuse is the arithmetic the repair uses to fit.
/// Returns whether anything changed.
fn fit_component(component: &mut Component, holds: usize) -> bool {
    let before = crate::deck::component_lines(component);
    let total_budget = holds.saturating_mul(CHARS_PER_LINE);
    let pad = if matches!(component, Component::Steps { .. }) {
        3
    } else {
        2
    };
    match component {
        Component::BulletList { items } | Component::Steps { items } => {
            let per_item = (total_budget / items.len().max(1))
                .saturating_sub(pad)
                .max(12);
            for item in items.iter_mut() {
                *item = compress_text(item, per_item);
            }
        }
        Component::Contrast { left, right } => {
            let per_side = total_budget.max(16);
            *left = compress_text(left, per_side);
            *right = compress_text(right, per_side);
        }
        Component::Metrics { items } => {
            let per_label = (total_budget / items.len().max(1)).saturating_sub(6).max(8);
            for (value, label) in items.iter_mut() {
                let value_len = value.chars().count();
                *label = compress_text(label, per_label.max(value_len + 4));
            }
        }
        Component::Metric { value, label } => {
            let value_len = value.chars().count();
            *label = compress_text(label, total_budget.saturating_sub(value_len + 2).max(8));
        }
        Component::Quote { text, attribution } => {
            let text_budget = total_budget
                .saturating_sub(attribution.chars().count() + 4)
                .max(24);
            *text = compress_text(text, text_budget);
        }
        Component::Chart { caption, series } => {
            let series_chars: usize = series
                .iter()
                .map(|(label, _)| label.chars().count() + 8)
                .sum();
            *caption = compress_text(caption, total_budget.saturating_sub(series_chars).max(16));
        }
        Component::Prose { text } => {
            *text = compress_text(text, total_budget);
        }
    }
    crate::deck::component_lines(component) != before
}

/// The deterministic half of the self-correction loop: a deck the
/// consultant checks passed and the layout grader refused comes back
/// repaired — text compressed to the measured budgets, headlines trimmed
/// to their box, heaviest columns balanced toward their lightest — and
/// whatever the repair could not clear comes back named. The consultant
/// constraints outrank the fit: a pass whose trim would break the
/// pyramid, the MECE split, or the bolding reverts wholesale, because a
/// deck that fits and lies is worse than a deck that overflows and says
/// so.
pub struct AutoFit {
    /// The deck after the repair.
    pub deck: Deck,
    /// What changed, in the order it changed — the render's answer names
    /// these beside the verdict.
    pub actions: Vec<String>,
    /// The faults the repair could not clear. Empty means the returned
    /// deck renders clean.
    pub residual: Vec<LayoutFault>,
}

/// The number of repair passes before whatever remains is named as
/// residual. Compression converges in one pass by arithmetic; the extra
/// passes absorb the knock-on effects of a balance trim.
const AUTO_FIT_PASSES: usize = 3;

pub fn auto_fit(deck: &Deck) -> AutoFit {
    let mut deck = deck.clone();
    let mut actions: Vec<String> = Vec::new();
    // The consultant verdict the deck arrived with is the baseline: the
    // repair may not change it. A deck that arrived with a deck-level
    // refusal (a bounds bound, say) keeps it — the auto-fit repairs
    // layout, and reverting on a refusal the input already had would
    // make every repair of an imperfect deck impossible.
    let baseline = check(&deck);
    for _pass in 0..AUTO_FIT_PASSES {
        let faults = review(&deck);
        if faults.is_empty() {
            return AutoFit {
                deck,
                actions,
                residual: Vec::new(),
            };
        }
        let snapshot = deck.clone();
        let faults = faults.clone();
        let mut changed = false;
        for fault in &faults {
            let Some(node) = deck
                .nodes
                .iter_mut()
                .find(|node| node.id == fault.node_id())
            else {
                continue;
            };
            match fault {
                LayoutFault::Overfull {
                    component, holds, ..
                } => {
                    let index = *component;
                    let holds = *holds;
                    if let Some(component) = node.components.get_mut(index) {
                        if fit_component(component, holds) {
                            changed = true;
                            actions.push(format!(
                                "`{}`: component {} compressed to fit {} lines",
                                node.id, index, holds
                            ));
                        }
                    }
                }
                LayoutFault::HeadlineOverflow { .. } => {
                    let budget = HEADLINE_HOLDS * CHARS_PER_LINE * 2;
                    let trimmed = compress_text(&node.headline, budget);
                    if trimmed != node.headline {
                        let candidate = Node {
                            headline: trimmed,
                            ..node.clone()
                        };
                        if headline_lines(&candidate) <= HEADLINE_HOLDS
                            && is_conclusion(&candidate.headline)
                        {
                            actions.push(format!("`{}`: headline trimmed to its box", node.id));
                            *node = candidate;
                            changed = true;
                        }
                    }
                }
                LayoutFault::Unbalanced { columns, .. } => {
                    // The heaviest column compresses toward the lightest
                    // one's lines, so the node reads as peers again.
                    let heaviest = columns.first().copied().unwrap_or(0);
                    let lightest = columns.iter().copied().min().unwrap_or(1).max(1);
                    let heaviest_at = node
                        .components
                        .iter()
                        .position(|component| crate::deck::component_lines(component) == heaviest);
                    if let Some(index) = heaviest_at {
                        if let Some(component) = node.components.get_mut(index) {
                            if fit_component(component, lightest) {
                                changed = true;
                                actions.push(format!(
                                    "`{}`: the heaviest column balanced toward {} lines",
                                    node.id, lightest
                                ));
                            }
                        }
                    }
                }
            }
        }
        // The consultant constraints outrank the fit: a pass that added
        // a refusal the input did not have reverts wholesale.
        let now = check(&deck);
        if now.iter().any(|refusal| !baseline.contains(refusal)) {
            deck = snapshot;
            break;
        }
        if !changed {
            break;
        }
    }
    // Whatever still overflows gets the roomier layout, when there is
    // one — the upgrade the blueprint calls for before any text is cut
    // deeper than half.
    for node in &mut deck.nodes {
        let faults = review_of_node(node);
        let overfull = faults
            .iter()
            .any(|fault| matches!(fault, LayoutFault::Overfull { .. }));
        if !overfull {
            continue;
        }
        if let Some(roomier) = node.layout.roomier() {
            let swapped = Node {
                layout: roomier,
                ..node.clone()
            };
            if review_of_node(&swapped).is_empty() && check_of_node(&swapped) {
                actions.push(format!(
                    "`{}`: the layout upgraded to {} — the text was not mine to cut",
                    node.id,
                    roomier.label()
                ));
                *node = swapped;
            }
        }
    }
    let residual = review(&deck);
    AutoFit {
        deck,
        actions,
        residual,
    }
}

/// The grader, over one node: the same arithmetic `review` runs, at the
/// node scope the auto-fit repairs at.
fn review_of_node(node: &Node) -> Vec<LayoutFault> {
    let single = Deck {
        project_type: crate::deck::ProjectType::Infographic,
        theme: String::new(),
        nodes: vec![node.clone()],
    };
    review(&single)
        .into_iter()
        .filter(|fault| fault.node_id() == node.id)
        .collect()
}

/// The consultant checks, over one node — the auto-fit's roomier gate.
/// Deck-level refusals (the type bounds) name no node and are not this
/// repair's to answer.
fn check_of_node(node: &Node) -> bool {
    let single = Deck {
        project_type: crate::deck::ProjectType::Infographic,
        theme: String::new(),
        nodes: vec![node.clone()],
    };
    check(&single)
        .iter()
        .all(|refusal| !refusal.names_node(node.id.as_str()))
}

// --- the layout template index ---------------------------------------------

/// The index's entries: one embedded description per layout, written the
/// way an owner describes what they want rather than the schema's own
/// words. This is the vector half of "choose the layout from the
/// content" — the intent skill's table, matched by cosine instead of
/// read by eye.
const LAYOUT_EXAMPLES: &[(Layout, &str)] = &[
    (Layout::SplitTwoColumn, "a conclusion beside its supporting evidence: the point on one side, the proof and its numbers on the other"),
    (Layout::MetricCallout, "one number carries the whole point: a single figure, large, with what it measures beneath it"),
    (Layout::MetricRow, "three or four peer figures side by side, compared at a glance"),
    (Layout::Timeline, "a sequence in time, step after step, left to right"),
    (Layout::Comparison, "two states set against each other: before and after, with and without, this versus that"),
    (Layout::Quote, "a claim worth setting apart, in someone's words, with who said it"),
    (Layout::Chart, "a trend across periods, drawn, with a caption that says what moved"),
    (Layout::FullBleed, "an infographic's whole frame: everything visible at once, dense, read by scanning"),
];

/// The nearest layout for one intent string, by cosine against the
/// index, with its score. `None` when this machine has no embedding
/// backend — the honest degradation, and the reason the intent skill's
/// own table still exists: the match refines the classification, it
/// does not replace it.
pub fn suggest_layout(intent: &str) -> Option<(Layout, f64)> {
    suggest_layouts_with(&*crate::memory::probed_embedder(), intent)
        .into_iter()
        .next()
}

/// The whole index ranked against one intent, best first — the tool's
/// answer carries the top of this, so the planner sees the runner-ups
/// and can disagree with the match on content grounds.
pub fn suggest_layouts_with(
    embedder: &dyn crate::memory::Embedder,
    intent: &str,
) -> Vec<(Layout, f64)> {
    let query = match embedder.embed(intent) {
        Some(query) => query,
        None => return Vec::new(),
    };
    let mut ranked: Vec<(Layout, f64)> = LAYOUT_EXAMPLES
        .iter()
        .filter_map(|(layout, example)| {
            embedder
                .embed(example)
                .map(|vector| (*layout, crate::memory::cosine(&query, &vector)))
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked
}

/// The matcher over an explicit embedder — the seam the tests use.
pub fn suggest_layout_with(
    embedder: &dyn crate::memory::Embedder,
    intent: &str,
) -> Option<(Layout, f64)> {
    let query = embedder.embed(intent)?;
    LAYOUT_EXAMPLES
        .iter()
        .filter_map(|(layout, example)| {
            embedder
                .embed(example)
                .map(|vector| (*layout, crate::memory::cosine(&query, &vector)))
        })
        .max_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

// --- the PPTX export -------------------------------------------------------

/// The export: one deck as a `.pptx` file's bytes.
///
/// A PPTX is a ZIP of XML parts, and the crate contract is one static
/// binary with no new dependency — so the ZIP is written here, stored
/// rather than deflated, which every reader accepts. The parts are the
/// minimum a PowerPoint-compatible reader needs: the content types, the
/// relationships, a presentation, one slide per node, and the theme and
/// master a slide cannot render without.
///
/// Text rides as real text boxes with real positions rather than a
/// rendered picture, which is the whole point of the export: the owner
/// can open it and edit it, and a locked image of a slide is the thing
/// the blueprint explicitly refuses.
pub fn to_pptx(deck: &Deck) -> Vec<u8> {
    let mut parts: Vec<(String, Vec<u8>)> = vec![
        (
            "[Content_Types].xml".to_string(),
            content_types(deck.nodes.len()).into_bytes(),
        ),
        (
            "_rels/.rels".to_string(),
            ROOT_RELS.to_string().into_bytes(),
        ),
        (
            "ppt/presentation.xml".to_string(),
            presentation(deck.nodes.len()).into_bytes(),
        ),
        (
            "ppt/_rels/presentation.xml.rels".to_string(),
            presentation_rels(deck.nodes.len()).into_bytes(),
        ),
        (
            "ppt/slideMasters/slideMaster1.xml".to_string(),
            SLIDE_MASTER.to_string().into_bytes(),
        ),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels".to_string(),
            MASTER_RELS.to_string().into_bytes(),
        ),
        (
            "ppt/slideLayouts/slideLayout1.xml".to_string(),
            SLIDE_LAYOUT.to_string().into_bytes(),
        ),
        (
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels".to_string(),
            LAYOUT_RELS.to_string().into_bytes(),
        ),
        (
            "ppt/theme/theme1.xml".to_string(),
            THEME.to_string().into_bytes(),
        ),
    ];
    for (index, node) in deck.nodes.iter().enumerate() {
        let number = index + 1;
        parts.push((
            format!("ppt/slides/slide{number}.xml"),
            slide(node).into_bytes(),
        ));
        parts.push((
            format!("ppt/slides/_rels/slide{number}.xml.rels"),
            SLIDE_RELS.to_string().into_bytes(),
        ));
    }
    zip(&parts)
}

/// The theme's colours, so a slide inherits the face's own palette rather
/// than Office's defaults.
const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="fleet"><a:themeElements><a:clrScheme name="fleet"><a:dk1><a:srgbClr val="100E0D"/></a:dk1><a:lt1><a:srgbClr val="EBE7E4"/></a:lt1><a:dk2><a:srgbClr val="16130F"/></a:dk2><a:lt2><a:srgbClr val="1D1918"/></a:lt2><a:accent1><a:srgbClr val="4176E6"/></a:accent1><a:accent2><a:srgbClr val="7AAAFF"/></a:accent2><a:accent3><a:srgbClr val="3FB950"/></a:accent3><a:accent4><a:srgbClr val="D29922"/></a:accent4><a:accent5><a:srgbClr val="F85149"/></a:accent5><a:accent6><a:srgbClr val="948C87"/></a:accent6><a:hlink><a:srgbClr val="7AAAFF"/></a:hlink><a:folHlink><a:srgbClr val="948C87"/></a:folHlink></a:clrScheme><a:fontScheme name="fleet"><a:majorFont><a:latin typeface="SF Pro Text"/></a:majorFont><a:minorFont><a:latin typeface="SF Pro Text"/></a:minorFont></a:fontScheme><a:fmtScheme name="fleet"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#;

const PRESENTATION_RELS_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#;

const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/></Relationships>"#;

/// One slide's relationships: it renders against the one layout, and every
/// slide names the same one — so this is a constant rather than a `format!`
/// with nothing to format.
const SLIDE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#;

const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;

const SLIDE_MASTER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#;

const SLIDE_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank"><p:cSld name="blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sldLayout>"#;

/// The content types, one slide override per node.
fn content_types(nodes: usize) -> String {
    let mut slides = String::new();
    for number in 1..=nodes {
        slides.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{number}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/><Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/><Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/><Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>{slides}</Types>"
    )
}

fn presentation(nodes: usize) -> String {
    let mut ids = String::new();
    for number in 1..=nodes {
        ids.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rId{}\"/>",
            255 + number,
            number + 2
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst></p:presentation>"
    )
}

fn presentation_rels(nodes: usize) -> String {
    let mut slides = String::new();
    for number in 1..=nodes {
        slides.push_str(&format!(
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>",
            number + 2,
            number
        ));
    }
    format!("{PRESENTATION_RELS_HEAD}{slides}</Relationships>")
}

/// One slide: its eyebrow, its headline, and its components as positioned
/// text boxes. The positions come from the layout the planner chose, so a
/// `split_two_column` node's evidence really does sit beside its
/// conclusion rather than wherever the writer put it.
fn slide(node: &Node) -> String {
    let mut shapes: Vec<String> = Vec::new();
    let mut id = 2u32;
    // The headline, top left, at the size the node's role asks for.
    shapes.push(text_box(
        &mut id,
        &TextBox {
            left: 685_800,
            top: 457_200,
            width: 10_795_000,
            height: 1_000_000,
            text: strip_markup(&node.headline),
            size: 2800,
            bold: true,
        },
    ));
    if !node.eyebrow.trim().is_empty() {
        shapes.push(text_box(
            &mut id,
            &TextBox {
                left: 685_800,
                top: 274_320,
                width: 10_795_000,
                height: 320_000,
                text: strip_markup(&node.eyebrow),
                size: 1100,
                bold: false,
            },
        ));
    }
    // The components, laid out by the node's own template.
    let columns = component_columns(node);
    for (index, component) in node.components.iter().enumerate() {
        let (column, of) = columns[index];
        let width = 10_795_000 / of as u32;
        let left = 685_800 + width * column as u32;
        shapes.push(text_box(
            &mut id,
            &TextBox {
                left,
                top: 1_600_000,
                width,
                height: 4_400_000,
                text: component_text(component),
                size: match component {
                    Component::Metric { .. } | Component::Metrics { .. } => 2000,
                    _ => 1300,
                },
                bold: false,
            },
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{}</p:spTree></p:cSld></p:sld>",
        shapes.join(""),
    )
}

/// How many columns each component sits in, and which one: a
/// `split_two_column` node puts its evidence beside its conclusion, a
/// `metric_row` spreads its figures across the width, and everything else
/// stacks.
fn component_columns(node: &Node) -> Vec<(usize, usize)> {
    let count = node.components.len();
    match node.layout {
        Layout::SplitTwoColumn | Layout::Comparison | Layout::MetricRow => (0..count)
            .map(|index| (index.min(count - 1), count.max(1)))
            .collect(),
        _ => (0..count).map(|_| (0, 1)).collect(),
    }
}

/// One component as the text a slide carries. A chart becomes its
/// caption and its series as lines, which is what a static slide can
/// hold without a chart engine.
fn component_text(component: &Component) -> String {
    match component {
        Component::BulletList { items } => items
            .iter()
            .map(|item| format!("\u{2022} {}", strip_markup(item)))
            .collect::<Vec<_>>()
            .join("\n"),
        Component::Metric { value, label } => {
            format!("{}\n{}", strip_markup(value), strip_markup(label))
        }
        Component::Metrics { items } => items
            .iter()
            .map(|(value, label)| format!("{}  {}", strip_markup(value), strip_markup(label)))
            .collect::<Vec<_>>()
            .join("\n"),
        Component::Steps { items } => items
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}. {}", index + 1, strip_markup(item)))
            .collect::<Vec<_>>()
            .join("\n"),
        Component::Contrast { left, right } => {
            format!("{}\n\n{}", strip_markup(left), strip_markup(right))
        }
        Component::Quote { text, attribution } => {
            format!(
                "\u{201c}{}\u{201d}\n\u{2014} {}",
                strip_markup(text),
                strip_markup(attribution)
            )
        }
        Component::Chart { caption, series } => {
            let mut out = strip_markup(caption);
            for (label, value) in series {
                out.push_str(&format!(
                    "\n{}: {}",
                    strip_markup(label),
                    format_number(*value)
                ));
            }
            out
        }
        Component::Prose { text } => strip_markup(text),
    }
}

/// One text box, as the geometry and the run it carries. The size is in
/// hundredths of a point and the offsets in EMU, which is the unit every
/// one of these attributes carries.
struct TextBox {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    text: String,
    size: u32,
    bold: bool,
}

/// One text box rendered as a shape.
fn text_box(id: &mut u32, box_at: &TextBox) -> String {
    *id += 1;
    let identifier = *id;
    let TextBox {
        left,
        top,
        width,
        height,
        size,
        ..
    } = *box_at;
    let boldness = if box_at.bold { " b=\"1\"" } else { "" };
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{identifier}\" name=\"TextBox {identifier}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{left}\" y=\"{top}\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr wrap=\"square\"><a:normAutofit/></a:bodyPr><a:p><a:r><a:rPr lang=\"en\" sz=\"{size}\"{boldness} dirty=\"0\"/><a:t>{}</a:t></a:r></a:p></p:txBody></p:sp>",
        xml_escape(&box_at.text)
    )
}

/// The markup a slide cannot carry: `**bold**` becomes the words, since a
/// run-level emphasis needs a second `<a:r>` and the bold flag already
/// says it for a headline.
fn strip_markup(text: &str) -> String {
    text.replace("**", "")
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A ZIP of `parts`, stored rather than deflated. Every reader accepts a
/// stored entry, and the crate contract is no new dependency — so the
/// container is written here rather than pulled in for one export.
fn zip(parts: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let mut offset: u32 = 0;
    for (name, bytes) in parts {
        let crc = crc32(bytes);
        let header = local_header(name, crc, bytes.len() as u32);
        out.extend_from_slice(&header);
        out.extend_from_slice(bytes);
        central.extend_from_slice(&central_header(name, crc, bytes.len() as u32, offset));
        offset += header.len() as u32 + bytes.len() as u32;
    }
    let central_start = out.len() as u32;
    let central_len = central.len() as u32;
    out.extend_from_slice(&central);
    // The end-of-central-directory record.
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&central_len.to_le_bytes());
    out.extend_from_slice(&central_start.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

fn local_header(name: &str, crc: u32, size: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&0u16.to_le_bytes()); // stored
    out.extend_from_slice(&0u16.to_le_bytes()); // time
    out.extend_from_slice(&0x0021u16.to_le_bytes()); // date
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra
    out.extend_from_slice(name.as_bytes());
    out
}

fn central_header(name: &str, crc: u32, size: u32, offset: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    out.extend_from_slice(&20u16.to_le_bytes()); // version made by
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&0u16.to_le_bytes()); // stored
    out.extend_from_slice(&0u16.to_le_bytes()); // time
    out.extend_from_slice(&0x0021u16.to_le_bytes()); // date
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra
    out.extend_from_slice(&0u16.to_le_bytes()); // comment
    out.extend_from_slice(&0u16.to_le_bytes()); // disk
    out.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
    out.extend_from_slice(&0u32.to_le_bytes()); // external attrs
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out
}

/// CRC-32, the checksum every ZIP entry carries. The table is computed
/// once on first use rather than spelled out.
fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 != 0 {
                    0xedb8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *slot = value;
        }
        table
    });
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc = table[((crc ^ u32::from(*byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

#[cfg(test)]
mod tests {

    /// A deterministic embedder for the matcher's test: a small keyword
    /// space, so a text's vector is its own keyword counts.
    struct KeywordStub;
    impl crate::memory::Embedder for KeywordStub {
        fn box_clone(&self) -> Box<dyn crate::memory::Embedder> {
            Box::new(KeywordStub)
        }
        fn embed(&self, text: &str) -> Option<Vec<f32>> {
            let axis = |word: &str| text.matches(word).count() as f32;
            Some(vec![
                axis("number"),
                axis("sequence") + axis("time"),
                axis("contrast") + axis("versus"),
                axis("quote") + axis("said"),
                axis("evidence") + axis("proof"),
                axis("scan") + axis("infographic"),
                axis("trend") + axis("drawn"),
            ])
        }
        fn name(&self) -> &'static str {
            "keyword-stub"
        }
        fn serves_vectors(&self) -> bool {
            true
        }
    }

    #[test]
    fn suggest_layout_matches_the_intent_across_the_index() {
        // A number-led intent lands on the metric callout; a sequence
        // lands on the timeline. The stub's keyword space is crude on
        // purpose: the matcher's job is ordering, not semantics.
        let number_intent = "one single number that carries the whole point of the argument";
        let (layout, score) =
            suggest_layout_with(&KeywordStub, number_intent).expect("the stub serves");
        assert_eq!(layout, Layout::MetricCallout, "score {score}");
        assert!(score > 0.9, "{score}");

        let sequence_intent = "a sequence in time, step after step, left to right";
        let (layout, score) =
            suggest_layout_with(&KeywordStub, sequence_intent).expect("the stub serves");
        assert_eq!(layout, Layout::Timeline, "score {score}");

        // No backend, no suggestion: the intent skill's table answers,
        // as it always has.
        struct NoneEmbedder;
        impl crate::memory::Embedder for NoneEmbedder {
            fn box_clone(&self) -> Box<dyn crate::memory::Embedder> {
                Box::new(NoneEmbedder)
            }
            fn embed(&self, _: &str) -> Option<Vec<f32>> {
                None
            }
            fn name(&self) -> &'static str {
                "none"
            }
            fn serves_vectors(&self) -> bool {
                false
            }
        }
        assert!(suggest_layout_with(&NoneEmbedder, "anything").is_none());
    }

    #[test]
    fn auto_fit_compresses_an_overfull_component_and_keeps_the_takeaway() {
        let long_tail = " across three regions by a wide margin, with commentary".repeat(13);
        let deck = deck(vec![node(
            "slide_01",
            "Q3 margins grew by fourteen percent on regional strength",
            vec![Component::BulletList {
                items: vec![format!("**Margins grew 14%**{long_tail}")],
            }],
        )]);
        assert!(!review(&deck).is_empty(), "the fixture must overflow");
        let fitted = auto_fit(&deck);
        assert!(
            review(&fitted.deck).is_empty(),
            "residual: {:?}",
            fitted.residual
        );
        let item = match &fitted.deck.nodes[0].components[0] {
            Component::BulletList { items } => items[0].clone(),
            other => panic!("unexpected component: {other:?}"),
        };
        assert!(item.contains("**"), "the bolding rule survives: {item}");
        assert!(fitted.actions.iter().any(|a| a.contains("compressed")));
        // the consultant verdict is the input's own: the fixture is a
        // one-node presentation, so its bounds refusal rides along —
        // what the repair may never do is add or change one
        assert_eq!(check(&fitted.deck), check(&deck));
    }

    #[test]
    fn auto_fit_upgrades_the_layout_when_compression_would_cut_the_content_away() {
        // Enough text that holding it in a split column would clip past
        // half of it: the roomier layout is the honest repair.
        let heavy = "one ".repeat(120);
        let deck = deck(vec![node(
            "slide_01",
            "The landscape report names four forces and their directions",
            vec![
                Component::Prose {
                    text: format!("{heavy}with detail"),
                },
                Component::BulletList {
                    items: vec!["**Force one** named".to_string()],
                },
            ],
        )]);
        assert!(!review(&deck).is_empty(), "the fixture must overflow");
        let fitted = auto_fit(&deck);
        assert!(
            review(&fitted.deck).is_empty(),
            "residual: {:?}",
            fitted.residual
        );
    }

    #[test]
    fn auto_fit_leaves_a_clean_deck_untouched() {
        let deck = deck(vec![node(
            "slide_01",
            "Q3 margins grew by fourteen percent on regional strength",
            vec![Component::BulletList {
                items: vec!["**Margins grew 14%** regionally".to_string()],
            }],
        )]);
        let fitted = auto_fit(&deck);
        assert_eq!(fitted.deck, deck, "a clean deck is not edited");
        assert!(fitted.actions.is_empty());
        assert!(fitted.residual.is_empty());
    }
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
    fn the_renderer_emits_the_faces_own_tokens_and_escapes_content() {
        // A deck rendered inside the browser face inherits the theme
        // switch, which is only true because the fragment references the
        // alias tokens rather than its own palette.
        let good = deck(vec![node(
            "slide_01",
            "Q3 growth came from regional expansion",
            vec![Component::BulletList {
                items: vec!["**54% increase** in APAC sales".to_string()],
            }],
        )]);
        let html = render(&good);
        assert!(html.contains("class=\"deck\""), "{html}");
        assert!(html.contains("data-type=\"presentation\""), "{html}");
        assert!(html.contains("data-layout=\"split_two_column\""), "{html}");
        assert!(html.contains("<h3 class=\"deck-headline\">"), "{html}");
        // The bolding rule the checker enforces is the bolding the
        // renderer marks.
        assert!(html.contains("<strong>54% increase</strong>"), "{html}");
        assert!(html.contains("<ul class=\"deck-bullets\">"), "{html}");

        // Content from a model is data, never markup: the page a deck is
        // rendered into also runs a conversation.
        let hostile = deck(vec![node(
            "slide_01",
            "A conclusion with <script>alert(1)</script> in it",
            vec![Component::Prose {
                text: "<img src=x onerror=alert(1)>".to_string(),
            }],
        )]);
        let hostile_html = render(&hostile);
        assert!(!hostile_html.contains("<script>"), "{hostile_html}");
        assert!(!hostile_html.contains("<img"), "{hostile_html}");
        assert!(hostile_html.contains("&lt;script&gt;"), "{hostile_html}");
        assert!(hostile_html.contains("&lt;img"), "{hostile_html}");
    }

    #[test]
    fn every_component_renders_its_own_markup() {
        let all = deck(vec![node(
            "slide_01",
            "One node carrying every component the schema allows",
            vec![
                Component::BulletList {
                    items: vec!["**one** point".to_string()],
                },
                Component::Metric {
                    value: "$4.2M".to_string(),
                    label: "Net new ARR".to_string(),
                },
                Component::Metrics {
                    items: vec![("54%".to_string(), "APAC".to_string())],
                },
                Component::Steps {
                    items: vec!["**first** step".to_string()],
                },
                Component::Contrast {
                    left: "**plan** providers".to_string(),
                    right: "pay-as-you-go".to_string(),
                },
                Component::Quote {
                    text: "**constraints** liberate".to_string(),
                    attribution: "the Rails doctrine".to_string(),
                },
                Component::Chart {
                    caption: "ARR by quarter".to_string(),
                    series: vec![("Q1".to_string(), 1.2), ("Q2".to_string(), 2.4)],
                },
                Component::Prose {
                    text: "One paragraph of **prose**.".to_string(),
                },
            ],
        )]);
        let html = render(&all);
        for mark in [
            "deck-bullets",
            "deck-metric",
            "deck-metrics",
            "deck-steps",
            "deck-contrast",
            "deck-quote",
            "deck-chart",
            "deck-prose",
        ] {
            assert!(html.contains(mark), "{mark} missing from {html}");
        }
        // A chart's bars are proportional to the peak, and the peak
        // fills its track.
        assert!(html.contains("height:100%"), "{html}");
        assert!(html.contains("height:50%"), "{html}");
        // An unclosed bold marker is literal, not a broken tag.
        assert!(inline("a ** b").contains("**"), "{}", inline("a ** b"));
        assert_eq!(inline("plain"), "plain");
    }

    #[test]
    fn a_chart_with_no_series_draws_no_full_bar() {
        // A zero peak must not read as every series being maximal.
        let flat = deck(vec![node(
            "slide_01",
            "A chart with nothing measured yet in it",
            vec![Component::Chart {
                caption: "nothing yet".to_string(),
                series: vec![("Q1".to_string(), 0.0)],
            }],
        )]);
        let html = render(&flat);
        assert!(html.contains("height:0%"), "{html}");
        assert!(!html.contains("height:100%"), "{html}");
    }

    #[test]
    fn the_export_is_a_zip_a_reader_can_open() {
        let deck = deck(vec![
            node(
                "slide_01",
                "Q3 growth came from regional expansion",
                vec![
                    Component::BulletList {
                        items: vec!["**54% increase** in APAC sales".to_string()],
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
                    items: vec!["**18 months** of runway".to_string()],
                }],
            ),
        ]);
        let bytes = to_pptx(&deck);
        // A ZIP starts with its local-file-header signature, and ends with
        // the end-of-central-directory record.
        assert_eq!(&bytes[..4], &[0x50, 0x4b, 0x03, 0x04], "the local header");
        let tail = &bytes[bytes.len() - 22..];
        assert_eq!(&tail[..4], &[0x50, 0x4b, 0x05, 0x06], "the EOCD");
        // Two slides, so two entries past the fixed parts.
        let entries = u16::from_le_bytes([tail[10], tail[11]]);
        assert_eq!(
            entries, 13,
            "the fixed parts plus one slide and its rels each"
        );

        // Every part a PowerPoint-compatible reader needs is present.
        let text = String::from_utf8_lossy(&bytes);
        for part in [
            "[Content_Types].xml",
            "ppt/presentation.xml",
            "ppt/slides/slide1.xml",
            "ppt/slides/slide2.xml",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/theme/theme1.xml",
        ] {
            assert!(text.contains(part), "{part} missing");
        }
        // The content rides as text, not a picture: the whole point of the
        // export is that the owner can open it and edit it.
        assert!(
            text.contains("Q3 growth came from regional expansion"),
            "{text}"
        );
        assert!(text.contains("54% increase"), "{text}");
        assert!(text.contains("$4.2M"), "{text}");
        // And the bold marker is stripped rather than shown.
        assert!(!text.contains("**54%"), "{text}");
    }

    #[test]
    fn a_slides_text_boxes_are_positioned_by_its_layout() {
        // A split_two_column node's evidence sits beside its conclusion
        // rather than wherever the writer put it, so the two components
        // carry different horizontal offsets.
        let split = node(
            "slide_01",
            "Growth came from two regions, not one",
            vec![
                Component::BulletList {
                    items: vec!["**54% increase** in APAC".to_string()],
                },
                Component::Metric {
                    value: "$4.2M".to_string(),
                    label: "ARR".to_string(),
                },
            ],
        );
        let xml = slide(&split);
        let offsets: Vec<u32> = xml
            .match_indices("<a:off x=\"")
            .filter_map(|(at, _)| {
                xml[at + 10..]
                    .split('"')
                    .next()
                    .and_then(|value| value.parse().ok())
            })
            .collect();
        // The headline, the eyebrow-less body, and the two components.
        assert_eq!(offsets.len(), 3, "{offsets:?}");
        assert_ne!(
            offsets[1], offsets[2],
            "the two columns differ: {offsets:?}"
        );

        // A stacked layout keeps everything in one column. The layout has
        // to be named explicitly: `node` defaults to split_two_column,
        // which is the whole difference under test.
        let mut stacked = node(
            "slide_01",
            "Everything on one frame, read at a glance",
            vec![
                Component::BulletList {
                    items: vec!["**one** point".to_string()],
                },
                Component::Prose {
                    text: "and a paragraph".to_string(),
                },
            ],
        );
        stacked.layout = Layout::MetricCallout;
        let xml = slide(&stacked);
        let stacked_offsets: Vec<u32> = xml
            .match_indices("<a:off x=\"")
            .filter_map(|(at, _)| {
                xml[at + 10..]
                    .split('"')
                    .next()
                    .and_then(|value| value.parse().ok())
            })
            .collect();
        assert_eq!(
            stacked_offsets[1], stacked_offsets[2],
            "{stacked_offsets:?}"
        );
    }

    #[test]
    fn the_grader_refuses_a_component_that_will_not_fit_its_box() {
        // A bullet list long enough to set past the box is refused with
        // its estimated lines against the lines the box holds.
        let long_items: Vec<String> = (0..12)
            .map(|index| format!("**point {index}** carries a full sentence of explanation here"))
            .collect();
        let overfull = deck(vec![node(
            "slide_01",
            "Twelve points will not fit one box on one slide",
            vec![Component::BulletList { items: long_items }],
        )]);
        let faults = review(&overfull);
        assert!(
            faults
                .iter()
                .any(|fault| matches!(fault, LayoutFault::Overfull { .. })),
            "{:?}",
            faults
        );
        // And a headline long enough to set past two lines is its own
        // fault, named separately.
        let long_headline = deck(vec![node(
            "slide_01",
            "This headline is long enough that it will set to three or four lines at the width the node has, which means it collides with every component rendered beneath it and the whole node becomes unreadable rather than merely dense",
            vec![Component::Prose { text: "short".to_string() }],
        )]);
        assert!(
            review(&long_headline)
                .iter()
                .any(|fault| matches!(fault, LayoutFault::HeadlineOverflow { .. })),
            "{:?}",
            review(&long_headline)
        );
    }

    #[test]
    fn the_grader_refuses_a_node_whose_columns_are_lopsided() {
        // One column carrying ten times the other reads as one column
        // with a stub beside it.
        let lopsided = deck(vec![node(
            "slide_01",
            "One column is carrying this node and the other is a stub",
            vec![
                Component::BulletList {
                    items: (0..10)
                        .map(|index| format!("**point {index}** with a full sentence of its own"))
                        .collect(),
                },
                Component::Metric {
                    value: "1".to_string(),
                    label: "x".to_string(),
                },
            ],
        )]);
        let faults = review(&lopsided);
        let unbalanced = faults
            .iter()
            .find(|fault| matches!(fault, LayoutFault::Unbalanced { .. }))
            .expect("the lopsided node is named");
        assert!(
            unbalanced.line().contains("one side is carrying"),
            "{unbalanced:?}"
        );

        // Two columns of comparable weight pass.
        let even = deck(vec![node(
            "slide_01",
            "Both columns carry their share of this node",
            vec![
                Component::BulletList {
                    items: vec![
                        "**first** point with a sentence".to_string(),
                        "**second** point with a sentence".to_string(),
                    ],
                },
                Component::BulletList {
                    items: vec![
                        "**third** point with a sentence".to_string(),
                        "**fourth** point with a sentence".to_string(),
                    ],
                },
            ],
        )]);
        assert!(
            !review(&even)
                .iter()
                .any(|fault| matches!(fault, LayoutFault::Unbalanced { .. })),
            "{:?}",
            review(&even)
        );
    }

    #[test]
    fn a_well_proportioned_deck_passes_the_review() {
        // The grader is the ceiling above the checker's floor: a deck
        // that passes both is one that renders.
        let good = deck(vec![
            node(
                "slide_01",
                "Q3 growth came from regional expansion",
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
        assert!(review(&good).is_empty(), "{:?}", review(&good));
        assert!(
            review_line(&good).contains("the layout holds"),
            "{}",
            review_line(&good)
        );
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
