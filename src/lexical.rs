//! BM25 lexical ranking — the ranked peer to the vector routes.
//!
//! Fleet's memory ranker matches keywords by set coverage, and the corpus and
//! code searches had no lexical route at all. This module is the standard
//! BM25: term frequency, inverse document frequency, and length normalization,
//! over an in-memory document set. It is deterministic, dependency-free, and
//! local — the same contract as the vector routes, with a ranking that set
//! coverage cannot give (a rare term outranks a common one).
//!
//! The terms come from [`crate::memory::stem`], fleet's own light stemmer, so
//! the lexical route and the memory ranker speak the same word forms.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The BM25 term-frequency saturation. The standard value.
pub const BM25_K1: f64 = 1.2;
/// The BM25 length-normalization strength. The standard value.
pub const BM25_B: f64 = 0.75;

/// One indexed document: an id and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexDoc {
    pub id: String,
    pub text: String,
}

/// A BM25 index over a set of documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LexIndex {
    docs: Vec<LexDoc>,
    /// term -> number of documents containing it.
    df: HashMap<String, u32>,
    /// per-document term frequencies, parallel to `docs`.
    tf: Vec<HashMap<String, u32>>,
    total_len: usize,
}

impl LexIndex {
    /// Build the index. The document frequencies and term frequencies are
    /// computed once; a query then scores every document against them.
    pub fn new(docs: Vec<LexDoc>) -> Self {
        let mut df: HashMap<String, u32> = HashMap::new();
        let mut tf: Vec<HashMap<String, u32>> = Vec::with_capacity(docs.len());
        let mut total_len = 0usize;
        for doc in &docs {
            let counts = term_counts(&doc.text);
            for term in counts.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
            total_len += counts.values().sum::<u32>() as usize;
            tf.push(counts);
        }
        Self {
            docs,
            df,
            tf,
            total_len,
        }
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    fn avg_len(&self) -> f64 {
        if self.docs.is_empty() {
            0.0
        } else {
            self.total_len as f64 / self.docs.len() as f64
        }
    }

    /// The inverse document frequency of a term, in the BM25+ form that keeps
    /// it positive for terms present in every document.
    fn idf(&self, term: &str) -> f64 {
        let n = self.docs.len() as f64;
        let df = *self.df.get(term).unwrap_or(&0) as f64;
        (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
    }

    /// The BM25 score of one document against a query's stemmed terms.
    pub fn score(&self, doc: usize, query_terms: &[String]) -> f64 {
        let avg = self.avg_len();
        let len = self.tf[doc].values().sum::<u32>() as f64;
        let mut score = 0.0;
        for term in query_terms {
            let f = *self.tf[doc].get(term).unwrap_or(&0) as f64;
            if f == 0.0 {
                continue;
            }
            let denom = f + BM25_K1 * (1.0 - BM25_B + BM25_B * len / avg.max(1.0));
            score += self.idf(term) * (f * (BM25_K1 + 1.0)) / denom;
        }
        score
    }

    /// Rank the documents against a query, best first. Ties break by id, so
    /// the order is total and reproducible.
    pub fn rank(&self, query: &str) -> Vec<(String, f64)> {
        let terms = crate::memory::stems(query);
        let mut scored: Vec<(String, f64)> = (0..self.docs.len())
            .map(|index| (self.docs[index].id.clone(), self.score(index, &terms)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        scored.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        scored
    }
}

/// The stemmed term counts of a text, duplicates kept — BM25's term frequency
/// needs the repeats, unlike the memory ranker's set-coverage channels.
fn term_counts(text: &str) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for raw in text
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        *counts.entry(crate::memory::stem(raw)).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(id: &str, text: &str) -> LexDoc {
        LexDoc {
            id: id.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn a_rare_term_outranks_a_common_one() {
        // "photosynth" appears in one document; "system" in every one of them.
        let index = LexIndex::new(vec![
            doc("a", "the system indexes the system store"),
            doc("b", "the system ranks the system results"),
            doc("c", "the photosynth pathway fixes carbon"),
        ]);
        let ranked = index.rank("photosynth");
        assert_eq!(
            ranked.len(),
            1,
            "only the document carrying the term matches"
        );
        assert_eq!(ranked[0].0, "c", "the rare term's document is the hit");
        assert!(ranked[0].1 > 0.0, "the score is positive");
    }

    #[test]
    fn the_common_term_ranks_the_document_that_repeats_it() {
        let index = LexIndex::new(vec![
            doc("once", "the router selects a model"),
            doc(
                "twice",
                "the router routes the router request through the router",
            ),
        ]);
        let ranked = index.rank("router");
        assert_eq!(ranked.len(), 2, "both documents carry the term");
        assert_eq!(
            ranked[0].0, "twice",
            "the document repeating the term outranks the one mentioning it once"
        );
    }

    #[test]
    fn length_normalization_keeps_a_long_document_from_winning_on_bulk() {
        let index = LexIndex::new(vec![
            doc("short", "the cache is warm"),
            doc(
                "long",
                "the cache is warm, and the rest of this document is filler about \
                 unrelated machinery that the stemmer keeps and the ranker must not \
                 reward merely for its length, repeated at length across the corpus \
                 so the document grows well past the average",
            ),
        ]);
        let ranked = index.rank("cache warm");
        assert_eq!(ranked[0].0, "short", "the short document wins on density");
    }

    #[test]
    fn the_parameters_are_the_standard_ones() {
        assert_eq!(BM25_K1, 1.2);
        assert_eq!(BM25_B, 0.75);
    }

    #[test]
    fn an_empty_index_and_a_missing_term_rank_nothing() {
        let index = LexIndex::new(vec![]);
        assert!(index.rank("anything").is_empty());
        let index = LexIndex::new(vec![doc("a", "the system store")]);
        assert!(
            index.rank("photosynth").is_empty(),
            "no term overlap, no hit"
        );
    }

    #[test]
    fn the_stemmer_is_shared_with_the_memory_ranker() {
        let index = LexIndex::new(vec![doc("a", "the derived facts resolve cleanly")]);
        let ranked = index.rank("deriving fact resolved");
        assert_eq!(
            ranked.len(),
            1,
            "the stemmed forms match across the two routes"
        );
    }
}
