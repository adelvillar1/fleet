//! The recall graph: one bounded hop over the edges the substrate
//! already declares and the retriever has never walked.
//!
//! The frontmatter carries relations between facts, and the ranker reads
//! none of them for retrieval:
//!
//! - **`missing`** — the companions an incomplete trajectory needed and
//!   did not have. The field names them, and it is deliberately excluded
//!   from `Fact::indexed_text` because an absent companion is not the
//!   fact's direction — but nothing ever fetches the named fact either,
//!   so the corpus declares an edge and the retriever renders it as
//!   prose. When the stalled fact matches, the companion is what the
//!   session needs to finish the trajectory.
//! - **`superseded-by`** — this fact was folded into another, so the
//!   other is the current truth about the same thing. The substrate
//!   *retires* the fact that carries it (`Fact::forgotten` reads a
//!   non-empty value as retired), so it never enters a pool and there is
//!   no forward source to expand from. What it can still do is vouch: a
//!   retired fact whose own vocabulary matches an instruction points at
//!   the fact that replaced it, and that replacement is what the turn
//!   actually needs. So a retired fact is a SOURCE here and never a hit.
//! - **a shared `shape` tag** — deliberately NOT an edge here. The
//!   partition already reads it, so a shape hop would re-admit facts the
//!   ranking has already ordered: noise, not expansion.
//!
//! Every neighbour that enters is MARKED with the edge that reached it —
//! the same discipline the spillover's `off-shape` marker runs on — so a
//! fact that entered on another fact's authority is never silently mixed
//! into the shape's own answer.
//!
//! Both constants can be zeroed, which turns the whole mechanism off
//! without removing it: the seam stays for the measurement that decides
//! whether it ships.

use std::collections::HashMap;

use crate::memory::Fact;

/// How many of the primary pool's strongest hits one recall expands
/// from. Three, so a wrong source's authority is bounded by the same
/// bound that bounds the pool's head.
pub const GRAPH_SOURCES: usize = 3;

/// How far the expansion reaches. One, and it stays one: a second hop
/// compounds a wrong source's authority, and the corpus's own edges are
/// shallow.
pub const GRAPH_HOPS: usize = 1;

/// The edge that reaches a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// `superseded-by`: the source was folded into the neighbour, so the
    /// neighbour is the current truth. The source is retired, so it
    /// vouches rather than ranks.
    SupersededBy,
    /// `missing`: the neighbour is a companion the source's trajectory
    /// needed and did not have.
    Missing,
}

impl Edge {
    /// The word the frontmatter carries, and the word the assembled
    /// line marks the neighbour with.
    pub fn label(self) -> &'static str {
        match self {
            Edge::SupersededBy => "superseded-by",
            Edge::Missing => "missing",
        }
    }
}

/// One neighbour and the edge that reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbour {
    pub name: String,
    pub edge: Edge,
}

/// The adjacency map, derived from the facts at load: one entry per fact
/// that points somewhere, in the frontmatter's own order.
#[derive(Debug, Default, Clone)]
pub struct Graph {
    edges: HashMap<String, Vec<Neighbour>>,
}

impl Graph {
    /// Build the adjacency map over `facts`. A name that does not
    /// resolve to a fact in the corpus is dropped silently — the corpus
    /// is hand-editable, and a dangling `missing` name is not an error
    /// the recall has to answer for.
    pub fn build(facts: &[Fact]) -> Graph {
        let known: std::collections::HashSet<&str> =
            facts.iter().map(|fact| fact.name.as_str()).collect();
        let mut edges: HashMap<String, Vec<Neighbour>> = HashMap::new();
        for fact in facts {
            let mut out: Vec<Neighbour> = Vec::new();
            // A bare scalar: the fork's writer emits it empty for
            // "unset", so an empty value is no edge.
            let target = fact.superseded_by.trim();
            if !target.is_empty() && known.contains(target) {
                out.push(Neighbour {
                    name: target.to_string(),
                    edge: Edge::SupersededBy,
                });
            }
            // A bracketed list of names, case-preserving.
            for companion in &fact.missing {
                if known.contains(companion.as_str()) {
                    out.push(Neighbour {
                        name: companion.clone(),
                        edge: Edge::Missing,
                    });
                }
            }
            if !out.is_empty() {
                edges.insert(fact.name.clone(), out);
            }
        }
        Graph { edges }
    }

    /// The neighbours `name` points at, in frontmatter order. Empty for
    /// a fact that points nowhere.
    pub fn neighbours(&self, name: &str) -> &[Neighbour] {
        self.edges.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Every fact that points somewhere — the candidate sources a
    /// retired fact can vouch from, which the pool loop never visits
    /// because a retired fact is out of recall by design.
    pub fn sources(&self) -> impl Iterator<Item = &String> {
        self.edges.keys()
    }
}
