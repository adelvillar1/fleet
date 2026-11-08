//! Reciprocal Rank Fusion — the parameter-light way to combine ranked routes.
//!
//! Fleet's corpus search was vector-only and the memory ranker is a weighted
//! sum over six tuned channels. RRF is the standard alternative: it fuses
//! routes by rank alone, so no weights are tuned and a route's score scale
//! cannot dominate another's. zvec-grep fuses its lexical and vector routes
//! the same way at `K = 60`.

use std::collections::HashMap;

/// The RRF constant. The standard value; the same one zvec-grep uses.
pub const RRF_K: f64 = 60.0;

/// Fuse ranked routes into one ranking.
///
/// Each route is a ranked list of ids, best first. An id's fused score is the
/// sum over the routes that contain it of `1 / (RRF_K + rank)`, with rank
/// 1-based. Ties break by id, so the order is total and reproducible.
pub fn fuse(routes: &[Vec<String>]) -> Vec<(String, f64)> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    for route in routes {
        for (index, id) in route.iter().enumerate() {
            let rank = index + 1;
            *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (RRF_K + rank as f64);
        }
    }
    let mut fused: Vec<(String, f64)> = scores.into_iter().collect();
    fused.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    fused
}

/// The RRF score contribution of one rank in one route. Exposed for the
/// tests and for a caller that wants to weight a route before fusing.
pub fn contribution(rank: usize) -> f64 {
    1.0 / (RRF_K + rank.max(1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn a_document_in_both_routes_outranks_one_in_a_single_route() {
        let fused = fuse(&[
            route(&["shared", "left-only"]),
            route(&["shared", "right-only"]),
        ]);
        assert_eq!(fused[0].0, "shared", "the document both routes found wins");
        assert_eq!(
            fused[0].1,
            contribution(1) + contribution(1),
            "its score is the sum of both routes' top contributions"
        );
    }

    #[test]
    fn the_fused_order_follows_the_ranks() {
        let fused = fuse(&[route(&["a", "b"]), route(&["b", "a"])]);
        // a: 1/(60+1) + 1/(60+2); b: 1/(60+2) + 1/(60+1) — equal, so id breaks.
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].0, "a", "an exact tie breaks by id");
        assert_eq!(fused[1].0, "b");
    }

    #[test]
    fn a_route_order_is_reproduced_when_one_route_is_longer() {
        let fused = fuse(&[route(&["x", "y", "z"]), route(&["z"])]);
        assert_eq!(fused[0].0, "z", "the document both routes found wins");
        assert_eq!(fused[1].0, "x", "the longer route's top document follows");
        assert_eq!(fused[2].0, "y");
    }

    #[test]
    fn empty_routes_fuse_to_nothing() {
        assert!(fuse(&[]).is_empty());
        assert!(fuse(&[route(&[]), route(&[])]).is_empty());
    }

    #[test]
    fn the_constant_is_the_standard_one() {
        assert_eq!(RRF_K, 60.0);
    }
}
