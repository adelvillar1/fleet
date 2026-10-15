//! The dependency graph's own tests: the arithmetic pinned, the two
//! gates reported separately, the resource edge mandatory, the critical
//! path named, and the render bounded.
//!
//! The facts a test controls stand in for the artifacts on disk: a node
//! verifies when its id is in the shared set, which is a fact the test
//! sets rather than a report a node makes — the same seam the parent's
//! D2 judge fills.

use super::*;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// The fact-set a test controls: the stand-in for the artifact on disk.
type Facts = Arc<Mutex<HashSet<String>>>;

/// An empty fact-set: nothing has landed.
fn facts() -> Facts {
    Arc::new(Mutex::new(HashSet::new()))
}

/// Land a node's artifact — the fact the verifier reads.
fn land(facts: &Facts, id: &str) {
    facts.lock().unwrap().insert(id.to_string());
}

/// A graph gated by the shared fact-set.
fn gated(facts: &Facts) -> Graph {
    let facts = Arc::clone(facts);
    Graph::with_verifier(move |node| facts.lock().unwrap().contains(&node.id))
}

/// The ids a test expects, as the owned strings the API returns.
fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// The card without its ANSI escapes, so a test reads the text an
/// operator reads rather than the bytes the terminal draws.
fn plain(card: &str) -> String {
    let mut out = String::with_capacity(card.len());
    let mut escape = false;
    for character in card.chars() {
        match character {
            '\u{1b}' => escape = true,
            'm' if escape => escape = false,
            _ if escape => {}
            c => out.push(c),
        }
    }
    out
}

/// The chain: fetch → build → deploy, costs 2, 3, 4. A node's
/// prerequisite is the one that must verify before it can start.
fn chain() -> Graph {
    let mut graph = Graph::new();
    graph.add(Node::new("fetch", 2)).unwrap();
    graph.add(Node::new("build", 3).with_prereqs(["fetch"])).unwrap();
    graph.add(Node::new("deploy", 4).with_prereqs(["build"]))
        .unwrap();
    graph
}

/// The diamond: ship needs left and right, and both need base. Costs:
/// base 2, left 3, right 4, ship 6.
fn diamond(facts: &Facts) -> Graph {
    let mut graph = gated(facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph.add(Node::new("left", 3).with_prereqs(["base"])).unwrap();
    graph.add(Node::new("right", 4).with_prereqs(["base"])).unwrap();
    graph.add(Node::new("ship", 6).with_prereqs(["left", "right"]))
        .unwrap();
    graph
}

#[test]
fn a_node_with_no_prerequisites_is_ready_immediately() {
    let mut graph = Graph::new();
    graph.add(Node::new("solo", 3)).unwrap();

    assert_eq!(graph.score("solo"), Some(0));
    assert_eq!(graph.node("solo").unwrap().peak(), 0);
    assert!(graph.critical_path().is_empty());
    let unlock = graph.unlock("solo").unwrap();
    assert!(unlock.score_zero);
    assert!(unlock.deps_verified);
    assert!(unlock.resource_free);
    assert!(unlock.is_unlocked());
    assert!(unlock.reason().is_none());
    assert_eq!(graph.ready(), ids(&["solo"]));
    assert_eq!(graph.dispatchable(3, &[]), ids(&["solo"]));
}

#[test]
fn the_score_is_the_sum_over_unfinished_dependencies_of_cost_plus_their_score() {
    // The formula, pinned on a chain:
    //
    //     score(n) = Σ over n's unfinished dependencies d of (cost(d) + score(d))
    //
    // score(fetch) = 0            — nothing stands in front of it
    // score(build) = 2 + 0        — cost(fetch) + score(fetch)
    // score(deploy) = 3 + 2       — cost(build) + score(build)
    let graph = chain();

    assert_eq!(graph.score("fetch"), Some(0));
    assert_eq!(graph.score("build"), Some(2));
    assert_eq!(graph.score("deploy"), Some(5));
    // The equivalent reading on a chain: the sum of every unfinished
    // transitive dependency's cost, which is 2 + 3.
    assert_eq!(graph.score("deploy"), Some(2 + 3));
    assert_eq!(
        graph.scores(),
        vec![
            ("fetch".to_string(), 0),
            ("build".to_string(), 2),
            ("deploy".to_string(), 5),
        ]
    );
}

#[test]
fn a_diamonds_score_carries_a_shared_dependency_once_per_path() {
    // The diamond is where the formula's own arithmetic shows: the base
    // is carried through both branches, so its cost enters the head's
    // score once per path that reaches it.
    //
    // score(left)  = 2 + 0
    // score(right) = 2 + 0
    // score(ship)  = (3 + 2) + (4 + 2) = 11
    let facts = facts();
    let mut graph = diamond(&facts);

    assert_eq!(graph.score("base"), Some(0));
    assert_eq!(graph.score("left"), Some(2));
    assert_eq!(graph.score("right"), Some(2));
    assert_eq!(graph.score("ship"), Some(11));

    // The trajectory: the base's completion drops the head by its cost
    // times the two paths, then each branch by its own cost, then zero.
    land(&facts, "base");
    assert_eq!(graph.settle(), ids(&["base"]));
    assert_eq!(graph.score("left"), Some(0));
    assert_eq!(graph.score("right"), Some(0));
    assert_eq!(graph.score("ship"), Some(7));

    land(&facts, "right");
    graph.settle();
    assert_eq!(graph.score("ship"), Some(3));

    land(&facts, "left");
    graph.settle();
    assert_eq!(graph.score("ship"), Some(0));
    assert!(graph.is_unlocked("ship"));
}

#[test]
fn a_dependency_completing_out_of_order_subtracts_its_own_cost_and_unlocks_on_the_last() {
    // A tree, so every completion subtracts exactly the completed node's
    // cost from every dependent's score — the common case the plan
    // names. The completions arrive out of declaration order: the
    // shallow sibling lands before the deep branch's own base.
    //
    // score(head) = (5 + 2) + (1 + 0) = 8
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("deepbase", 2)).unwrap();
    graph.add(Node::new("deep", 5).with_prereqs(["deepbase"]))
        .unwrap();
    graph.add(Node::new("shallow", 1)).unwrap();
    graph.add(
        Node::new("head", 9).with_prereqs(["deep", "shallow"]),
    )
    .unwrap();

    assert_eq!(graph.score("head"), Some(8));
    assert!(!graph.is_unlocked("head"));

    land(&facts, "shallow");
    graph.settle();
    assert_eq!(graph.score("head"), Some(7));

    land(&facts, "deepbase");
    graph.settle();
    assert_eq!(graph.score("head"), Some(5));

    land(&facts, "deep");
    graph.settle();
    assert_eq!(graph.score("head"), Some(0));
    assert!(graph.is_unlocked("head"));
}

#[test]
fn the_score_depends_only_on_which_nodes_are_unfinished_never_on_the_order() {
    // Two graphs, the same declarations, the same three completions in
    // two different orders. The intermediate scores differ because the
    // completed sets differ; the final scores are the order's blind
    // spot — identical, because the arithmetic reads the set.
    let mut finals: Vec<Vec<(String, u64)>> = Vec::new();
    for order in [["base", "left", "right"], ["base", "right", "left"]] {
        let facts = facts();
        let mut graph = diamond(&facts);
        let mut before = graph.scores();
        for id in order {
            land(&facts, id);
            graph.settle();
            let after = graph.scores();
            for (name, score) in &after {
                let was = before
                    .iter()
                    .find(|(seen, _)| seen == name)
                    .map(|(_, standing)| *standing)
                    .unwrap_or(0);
                assert!(*score <= was, "{name} rose from {was} to {score}");
            }
            before = after;
        }
        finals.push(graph.scores());
    }
    assert_eq!(finals[0], finals[1]);
}

#[test]
fn the_score_is_monotone_it_only_falls_as_completions_land() {
    // Monotone by construction: the score is derived on every read, so a
    // verified completion removes a term from every dependent's sum and
    // nothing ever adds one. (A cost revision is the orchestrator
    // correcting its own estimate, not a completion — see
    // `a_revised_cost_reshapes_the_ordering_and_never_the_gate`.)
    let facts = facts();
    let mut graph = diamond(&facts);
    graph.add(Node::new("tail", 1).with_prereqs(["ship"]))
        .unwrap();
    let mut before = graph.scores();
    for id in ["base", "right", "left", "ship", "tail"] {
        land(&facts, id);
        graph.settle();
        let after = graph.scores();
        for (name, score) in &after {
            let was = before
                .iter()
                .find(|(seen, _)| seen == name)
                .map(|(_, standing)| *standing)
                .unwrap_or(0);
            assert!(*score <= was, "{name} rose from {was} to {score}");
        }
        before = after;
    }
    assert_eq!(graph.score("tail"), Some(0));
    assert!(graph.is_unlocked("tail"));
}

#[test]
fn a_wildly_wrong_estimate_misorders_the_graph_and_the_revision_corrects_it() {
    // Two chains. The estimates say chain B is the long pole; the truth
    // is chain A, whose units each cost fifty turns. The estimate shapes
    // the ordering and the display, and D2's calibration corrects it —
    // while the gate never moves, because nothing verified.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("a1", 1)).unwrap();
    graph.add(Node::new("a2", 1).with_prereqs(["a1"])).unwrap();
    graph.add(Node::new("b1", 40)).unwrap();
    graph.add(Node::new("b2", 40).with_prereqs(["b1"])).unwrap();

    assert_eq!(graph.critical_path(), ids(&["b2", "b1"]));
    assert_eq!(graph.blocks_most_work(), Some("b1".to_string()));

    graph.revise_cost("a1", 50).unwrap();
    graph.revise_cost("a2", 50).unwrap();
    assert_eq!(graph.critical_path(), ids(&["a2", "a1"]));
    assert_eq!(graph.blocks_most_work(), Some("a1".to_string()));

    // The gate is untouched by either estimate: the chain heads dispatch
    // and the dependents never do. The order follows the corrected
    // estimate, so a1 — revised to fifty — goes ahead of b1.
    assert!(!graph.is_unlocked("a2"));
    assert!(!graph.is_unlocked("b2"));
    assert_eq!(graph.dispatchable(3, &[]), ids(&["a1", "b1"]));
}

#[test]
fn a_zero_estimate_never_unlocks_a_node_whose_dependency_never_landed() {
    // The sharpest form of the property: the estimate is zero, so the
    // score is zero, and the node is still held — the zero is computed
    // from verified completions, and this one never landed.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("never", 0)).unwrap();
    graph.add(Node::new("ghost", 7).with_prereqs(["never"]))
        .unwrap();

    assert_eq!(graph.score("ghost"), Some(0));
    let unlock = graph.unlock("ghost").unwrap();
    assert!(unlock.score_zero, "the estimate said the work was free");
    assert!(!unlock.deps_verified, "and the artifact never landed");
    assert!(!unlock.is_unlocked());
    assert_eq!(unlock.reason(), Some("waiting on never".to_string()));

    // Settling changes nothing: the fact-gate refuses the artifact.
    assert!(graph.settle().is_empty());
    assert!(!graph.is_unlocked("ghost"));
    // The root dispatches; the node whose dependency never landed does
    // not, whatever the estimate said its work was worth.
    assert_eq!(graph.dispatchable(3, &[]), ids(&["never"]));
    assert_eq!(graph.ready(), ids(&["never"]));
}

#[test]
fn the_default_graph_is_conservative_nothing_verifies_without_a_fact() {
    // No predicate is the conservative default: nothing verifies,
    // nothing unlocks, and every score sits where it was declared.
    let mut graph = chain();
    assert!(graph.settle().is_empty());
    assert_eq!(graph.score("deploy"), Some(5));
    assert!(!graph.is_unlocked("deploy"));
    assert_eq!(graph.dispatchable(3, &[]), ids(&["fetch"]));
}

#[test]
fn settle_returns_the_newly_verified_ids_in_declaration_order() {
    // The list the caller logs: a score that moved is a model-visible
    // event (hard rule 5), and the order is the declaration's so the log
    // is reconstructable.
    let facts = facts();
    let mut graph = diamond(&facts);
    land(&facts, "left");
    land(&facts, "base");
    assert_eq!(graph.settle(), ids(&["base", "left"]));
    // Idempotent: a second settle spends nothing and reports nothing.
    assert!(graph.settle().is_empty());
}

#[test]
fn a_node_fully_unblocked_by_its_dependencies_can_still_be_held_by_a_sibling_on_its_path() {
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph
        .add(
            Node::new("writer", 3)
                .with_prereqs(["base"])
                .with_paths(["src/graph.rs"]),
        )
        .unwrap();
    graph
        .add(
            Node::new("editor", 4)
                .with_prereqs(["base"])
                .with_paths(["src/graph.rs"]),
        )
        .unwrap();
    land(&facts, "base");
    graph.settle();

    // Both are score-zero: the dependencies are all verified.
    assert!(graph.unlock("writer").unwrap().score_zero);
    assert!(graph.unlock("editor").unwrap().score_zero);

    graph.dispatch("writer").unwrap();
    let unlock = graph.unlock("editor").unwrap();
    assert!(unlock.score_zero, "the dependencies are all verified");
    assert!(!unlock.resource_free, "the sibling holds the path");
    assert!(!unlock.is_unlocked());
    assert_eq!(
        unlock.held,
        vec![ResourceHold {
            holder: "writer".to_string(),
            path: "src/graph.rs".to_string(),
        }]
    );
    assert_eq!(
        unlock.reason(),
        Some("writer holds src/graph.rs".to_string())
    );
}

#[test]
fn a_node_waiting_on_work_and_a_node_waiting_on_a_lock_report_different_states() {
    // The two states are distinct in the data and in the render: one
    // names its dependencies, the other names its holder.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph.add(Node::new("waiter", 3).with_prereqs(["base"])).unwrap();
    graph.add(Node::new("holder", 4).with_paths(["src/graph.rs"]))
        .unwrap();
    graph
        .add(Node::new("held", 5).with_paths(["src/graph.rs"]))
        .unwrap();
    graph.dispatch("holder").unwrap();

    let waiting = graph.unlock("waiter").unwrap();
    let held = graph.unlock("held").unwrap();
    assert!(!waiting.score_zero, "work stands in front of it");
    assert!(waiting.resource_free, "and nothing holds a path");
    assert!(held.score_zero, "the dependencies are all verified");
    assert!(!held.resource_free, "and a sibling holds the path");
    assert_eq!(waiting.reason(), Some("waiting on base".to_string()));
    assert_eq!(
        held.reason(),
        Some("holder holds src/graph.rs".to_string())
    );

    let text = plain(&render(&graph));
    assert!(text.contains("waiting"), "{text}");
    assert!(
        text.contains("held by holder on src/graph.rs"),
        "{text}"
    );
}

#[test]
fn two_nodes_on_one_path_are_never_dispatched_together_and_the_conflict_is_named() {
    // The safety net: two nodes naming one admitted path never run
    // concurrently. The batch serializes them by name, the dispatch
    // refuses the second by name, and the path frees when the holder's
    // artifact lands.
    let facts = facts();
    let mut graph = gated(&facts);
    graph
        .add(Node::new("writer", 3).with_paths(["src/graph.rs"]))
        .unwrap();
    graph
        .add(Node::new("editor", 5).with_paths(["src/graph.rs"]))
        .unwrap();
    graph
        .add(Node::new("reader", 1).with_paths(["src/boards.rs"]))
        .unwrap();

    // The bound has room for all three; the resource edge takes one of
    // the two same-path nodes.
    assert_eq!(
        graph.dispatchable(3, &[]),
        ids(&["editor", "reader"])
    );
    // The caller's own record is consulted too: a node the caller says
    // is in flight holds its path even if the graph was never told.
    assert_eq!(
        graph.dispatchable(3, &ids(&["writer"])),
        ids(&["reader"])
    );

    graph.dispatch("editor").unwrap();
    assert_eq!(
        graph.dispatch("writer"),
        Err(GraphError::PathHeld {
            path: "src/graph.rs".to_string(),
            holder: "editor".to_string(),
        })
    );
    assert!(!graph.dispatchable(3, &ids(&["editor"])).contains(&"writer".to_string()));

    land(&facts, "editor");
    assert_eq!(graph.settle(), ids(&["editor"]));
    assert_eq!(
        graph.dispatchable(3, &[]),
        ids(&["writer", "reader"])
    );
}

#[test]
fn dispatch_returns_the_nodes_the_bound_has_room_for_in_blocks_most_work_order() {
    // The bound is respected, and the node that blocks the most work
    // goes first — the scheduling decision the score answers directly.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("shared", 4)).unwrap();
    graph.add(Node::new("p", 1).with_prereqs(["shared"])).unwrap();
    graph.add(Node::new("q", 1).with_prereqs(["shared"])).unwrap();
    graph.add(Node::new("lonely", 9)).unwrap();

    // shared appears in two scores, lonely in none, p and q are still
    // waiting on shared.
    assert_eq!(graph.dispatchable(2, &[]), ids(&["shared", "lonely"]));
    assert_eq!(graph.dispatchable(1, &[]), ids(&["shared"]));
    assert_eq!(graph.dispatchable(2, &ids(&["shared"])), ids(&["lonely"]));
    // A bound already full dispatches nothing.
    assert!(graph.dispatchable(2, &ids(&["shared", "lonely"])).is_empty());
}

#[test]
fn a_node_that_unlocks_mid_flight_is_dispatchable_without_waiting_for_the_wave() {
    // The wave pattern's cost: `late` waits for `early`'s artifact, not
    // for the batch's slowest sibling. When the fact lands mid-flight,
    // `late` is dispatchable on the next call.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("early", 2)).unwrap();
    graph.add(Node::new("late", 3).with_prereqs(["early"])).unwrap();
    graph.add(Node::new("filler", 9)).unwrap();
    graph.dispatch("filler").unwrap();

    assert_eq!(graph.dispatchable(2, &ids(&["filler"])), ids(&["early"]));
    assert!(
        !graph
            .dispatchable(2, &ids(&["filler"]))
            .contains(&"late".to_string())
    );

    land(&facts, "early");
    graph.settle();
    assert_eq!(graph.dispatchable(2, &ids(&["filler"])), ids(&["late"]));
}

#[test]
fn a_failed_prerequisite_does_not_unlock_its_dependent_and_the_reason_names_it() {
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph.add(Node::new("ship", 6).with_prereqs(["base"])).unwrap();
    graph.fail("base").unwrap();

    // The failed work never lands, so it still counts.
    assert_eq!(graph.score("ship"), Some(2));
    let unlock = graph.unlock("ship").unwrap();
    assert!(!unlock.score_zero);
    assert!(!unlock.deps_verified);
    assert_eq!(unlock.failed, ids(&["base"]));
    assert_eq!(unlock.reason(), Some("base failed".to_string()));
    assert!(!graph.is_unlocked("ship"));

    // A fact still outranks the report: the artifact landing promotes it.
    land(&facts, "base");
    assert_eq!(graph.settle(), ids(&["base"]));
    assert_eq!(graph.score("ship"), Some(0));
    assert!(graph.is_unlocked("ship"));
    // And a failure report never demotes a verified node.
    assert_eq!(
        graph.fail("base"),
        Err(GraphError::AlreadyVerified("base".to_string()))
    );
}

#[test]
fn the_critical_path_is_the_longest_chain_of_remaining_work() {
    // The highest score is the longest chain, and the path names itself
    // from it: head first, then the unfinished dependency whose own
    // closure carries the most work.
    let early = facts();
    let mut graph = diamond(&early);
    assert_eq!(graph.critical_path(), ids(&["ship", "right", "base"]));

    land(&early, "base");
    graph.settle();
    assert_eq!(graph.critical_path(), ids(&["ship", "right"]));

    land(&early, "right");
    graph.settle();
    assert_eq!(graph.critical_path(), ids(&["ship", "left"]));

    land(&early, "left");
    graph.settle();
    assert!(graph.critical_path().is_empty());

    // The path moves to the other chain when the first one lands.
    let later = facts();
    let mut graph = gated(&later);
    graph.add(Node::new("a1", 1)).unwrap();
    graph.add(Node::new("a2", 1).with_prereqs(["a1"])).unwrap();
    graph.add(Node::new("b1", 40)).unwrap();
    graph.add(Node::new("b2", 40).with_prereqs(["b1"])).unwrap();
    assert_eq!(graph.critical_path(), ids(&["b2", "b1"]));
    land(&later, "b1");
    land(&later, "b2");
    graph.settle();
    assert_eq!(graph.critical_path(), ids(&["a2", "a1"]));
}

#[test]
fn the_blocks_most_work_node_appears_in_the_most_scores() {
    // The diamond's base appears in three scores — left, right, and ship
    // — so finishing it moves the most work.
    let shared = facts();
    let graph = diamond(&shared);
    assert_eq!(graph.appears_in_scores("base"), 3);
    assert_eq!(graph.appears_in_scores("left"), 1);
    assert_eq!(graph.blocks_most_work(), Some("base".to_string()));

    // A tie breaks to the larger cost, then to the lowest id.
    let fresh = facts();
    let mut tie = gated(&fresh);
    tie.add(Node::new("t1", 5)).unwrap();
    tie.add(Node::new("t2", 9)).unwrap();
    tie.add(Node::new("p", 1).with_prereqs(["t1"])).unwrap();
    tie.add(Node::new("q", 1).with_prereqs(["t1"])).unwrap();
    tie.add(Node::new("r", 1).with_prereqs(["t2"])).unwrap();
    tie.add(Node::new("s", 1).with_prereqs(["t2"])).unwrap();
    assert_eq!(tie.appears_in_scores("t1"), 2);
    assert_eq!(tie.appears_in_scores("t2"), 2);
    assert_eq!(tie.blocks_most_work(), Some("t2".to_string()));
}

#[test]
fn a_declared_cycle_is_refused_by_name() {
    let mut graph = Graph::new();
    graph
        .add(Node::new("alpha", 1).with_prereqs(["beta"]))
        .unwrap();
    assert_eq!(
        graph.add(Node::new("beta", 1).with_prereqs(["alpha"])),
        Err(GraphError::Cycle {
            node: "beta".to_string(),
            prereq: "alpha".to_string(),
        })
    );
    assert_eq!(
        graph.add(Node::new("gamma", 1).with_prereqs(["gamma"])),
        Err(GraphError::Cycle {
            node: "gamma".to_string(),
            prereq: "gamma".to_string(),
        })
    );
    // The same refusal through the resequencing edge.
    graph.add(Node::new("one", 1).with_prereqs(["two"])).unwrap();
    graph.add(Node::new("two", 1)).unwrap();
    assert_eq!(
        graph.add_prerequisite("two", "one"),
        Err(GraphError::Cycle {
            node: "two".to_string(),
            prereq: "one".to_string(),
        })
    );
    // The graph is unchanged by the refusals.
    assert_eq!(graph.nodes().len(), 3);
}

#[test]
fn a_duplicate_node_id_is_refused_by_name() {
    let mut graph = chain();
    assert_eq!(
        graph.add(Node::new("build", 1)),
        Err(GraphError::DuplicateNode("build".to_string()))
    );
    assert_eq!(graph.len(), 3);
}

#[test]
fn an_unresolved_prerequisite_is_named_never_a_silent_zero() {
    // A forward declaration is legal; an unresolved one at dispatch time
    // is a named defect. It carries no declared cost, so the score reads
    // zero — and the verification condition is what holds the node,
    // naming the missing prerequisite.
    let mut graph = Graph::new();
    graph
        .add(Node::new("ship", 6).with_prereqs(["left", "ghost"]))
        .unwrap();

    assert_eq!(
        graph.unresolved(),
        vec![
            ("ship".to_string(), "left".to_string()),
            ("ship".to_string(), "ghost".to_string()),
        ]
    );
    assert_eq!(graph.score("ship"), Some(0));
    let unlock = graph.unlock("ship").unwrap();
    assert!(unlock.score_zero);
    assert!(!unlock.deps_verified);
    assert_eq!(unlock.unresolved, ids(&["left", "ghost"]));
    assert_eq!(
        unlock.reason(),
        Some("left, ghost not in the graph".to_string())
    );
    assert!(!graph.is_unlocked("ship"));
    assert!(graph.dispatchable(3, &[]).is_empty());
}

#[test]
fn a_forward_declared_prerequisite_scores_once_it_lands() {
    let mut graph = Graph::new();
    graph
        .add(Node::new("ship", 6).with_prereqs(["base"]))
        .unwrap();
    // The forward reference carries no declared cost yet.
    assert_eq!(graph.score("ship"), Some(0));
    assert_eq!(graph.node("ship").unwrap().peak(), 0);

    graph.add(Node::new("base", 2)).unwrap();
    assert_eq!(graph.score("ship"), Some(2));
    // The peak rises with it, so the readiness bar's denominator is the
    // score at the graph's widest rather than the zero it read first.
    assert_eq!(graph.node("ship").unwrap().peak(), 2);
}

#[test]
fn a_revised_cost_reshapes_the_ordering_and_never_the_gate() {
    // The estimate shapes the path to zero; only a verified completion
    // moves the score during a run. A revision can raise a score —
    // which is exactly why the estimates never gate anything.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph.add(Node::new("ship", 6).with_prereqs(["base"])).unwrap();
    assert_eq!(graph.score("ship"), Some(2));
    assert_eq!(graph.node("ship").unwrap().peak(), 2);

    graph.revise_cost("base", 9).unwrap();
    assert_eq!(graph.score("ship"), Some(9));
    assert_eq!(graph.node("ship").unwrap().peak(), 9);
    assert!(!graph.is_unlocked("ship"));

    land(&facts, "base");
    graph.settle();
    assert_eq!(graph.score("ship"), Some(0));
    assert!(graph.is_unlocked("ship"));
    // An unknown node is refused by name rather than silently ignored.
    assert_eq!(
        graph.revise_cost("ghost", 1),
        Err(GraphError::UnknownNode("ghost".to_string()))
    );
    assert_eq!(graph.dispatch("ghost"), Err(GraphError::UnknownNode("ghost".to_string())));
    assert_eq!(graph.fail("ghost"), Err(GraphError::UnknownNode("ghost".to_string())));
    assert!(graph.node("ghost").is_none());
    assert!(graph.score("ghost").is_none());
}

#[test]
fn a_verified_dependency_drops_its_whole_subtree_from_the_score() {
    // The formula's own arithmetic: a verified dependency contributes
    // nothing, so its whole closure is done work. This is why the
    // predicate is the authority (design decision 14 — facts gate) and
    // why an honest one never verifies a node before its own
    // prerequisites' artifacts have landed: the graph takes the
    // fact-gate's word for the artifact.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("fetch", 2)).unwrap();
    graph.add(Node::new("build", 3).with_prereqs(["fetch"])).unwrap();
    graph.add(Node::new("deploy", 4).with_prereqs(["build"])).unwrap();

    land(&facts, "build");
    assert_eq!(graph.settle(), ids(&["build"]));
    assert_eq!(graph.score("deploy"), Some(0));
    assert_eq!(graph.score("fetch"), Some(0));
    assert!(graph.is_unlocked("deploy"));
}

#[test]
fn the_render_is_cards_with_dots_bars_and_named_edges() {
    let facts = facts();
    let mut graph = gated(&facts);
    graph
        .add(
            Node::new("base", 2).with_capabilities(Capabilities {
                tools: ids(&["bash", "read_file"]),
                skills: ids(&["vendoring-lanes"]),
                workflows: Vec::new(),
                loops: Vec::new(),
            }),
        )
        .unwrap();
    graph
        .add(
            Node::new("ship", 6)
                .with_prereqs(["base"])
                .with_paths(["src/graph.rs"])
                .with_approach("land the graph in two cards and pin the widths"),
        )
        .unwrap();

    let text = plain(&render(&graph));
    // The summary card names the critical path and what blocks the most.
    assert!(text.contains("critical path"), "{text}");
    assert!(text.contains("ship \u{2192} base"), "{text}");
    assert!(text.contains("blocks the most"), "{text}");
    assert!(text.contains("base \u{b7} appears in 1 score"), "{text}");
    // Each node's card carries its dot, its bar, and its raw number.
    assert!(text.contains("\u{2713}") || text.contains("\u{25cb}") || text.contains("\u{25cf}"), "{text}");
    assert!(text.contains("\u{2588}"), "a full bar is drawn: {text}");
    assert!(text.contains("\u{2591}"), "an empty track is drawn: {text}");
    assert!(text.contains("2 left"), "{text}");
    // The edges are named, with the work standing behind them.
    assert!(text.contains("needs base (0 left)"), "{text}");
    assert!(text.contains("needs nothing"), "{text}");
    // The declaration and the approach ride the card.
    assert!(
        text.contains("declares tools: bash, read_file \u{b7} skills: vendoring-lanes"),
        "{text}"
    );
    assert!(
        text.contains("approach: land the graph in two cards and pin the widths"),
        "{text}"
    );
    assert!(text.contains("paths: src/graph.rs"), "{text}");
    // And it is a card, not a table.
    assert!(text.contains('\u{250c}') && text.contains('\u{2514}'), "{text}");
}

#[test]
fn every_render_line_fits_the_house_width() {
    // The KI-078/179 class: a line built by concatenation past the house
    // width. Every id, path, edge, declaration, and approach here is
    // unbounded, and every drawn line still fits the house bar — so the
    // card is whole at 80 columns and whole at 120.
    let facts = facts();
    let mut graph = gated(&facts);
    graph
        .add(
            Node::new(
                "a-node-with-an-identity-nobody-bounded-and-it-keeps-right-on-going",
                2,
            )
            .with_prereqs(["another-unbounded-prerequisite-identity-that-runs-and-runs"])
            .with_paths(["src/a/very/deep/path/that/nobody/bounded/either.rs"])
            .with_approach(
                "a brief's approach written as one long unbroken sentence with no regard for \
                 the card it will be drawn on, running well past the house width and then some \
                 more, and then some more after that",
            )
            .with_capabilities(Capabilities {
                tools: ids(&["bash", "read_file", "write_file", "list_dir"]),
                skills: ids(&["vendoring-lanes", "graph-vs-pg-decision-matrix"]),
                workflows: ids(&["the-judge-batch"]),
                loops: ids(&["consolidation"]),
            }),
        )
        .unwrap();
    graph
        .add(Node::new(
            "another-unbounded-prerequisite-identity-that-runs-and-runs",
            3,
        ))
        .unwrap();

    for line in render(&graph).lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
}

#[test]
fn the_render_names_what_is_blocking_what() {
    // A failed prerequisite is named rather than reading as ordinary
    // waiting, and the node it holds says so on its own card.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph.add(Node::new("ship", 6).with_prereqs(["base"])).unwrap();
    graph.add(Node::new("blocked", 1).with_prereqs(["base"])).unwrap();
    graph.fail("base").unwrap();

    let text = plain(&render(&graph));
    assert!(text.contains("failed"), "{text}");
    assert!(text.contains("needs base (failed)"), "{text}");
    assert!(
        text.contains("2 nodes named it as a prerequisite"),
        "{text}"
    );
    // The unresolved-edge case names the missing node too.
    let mut open = Graph::new();
    open.add(Node::new("ship", 6).with_prereqs(["ghost"]))
        .unwrap();
    let open_text = plain(&render(&open));
    assert!(open_text.contains("unresolved"), "{open_text}");
    assert!(open_text.contains("ship needs ghost"), "{open_text}");
    assert!(
        open_text.contains("needs ghost (not in the graph)"),
        "{open_text}"
    );
}

#[test]
fn the_render_is_bounded_at_the_house_width_and_prints_for_the_capture() {
    // The card is drawn at the house width, so it is whole at 80 columns
    // and whole at 120. The pty capture runs this test at both sizes and
    // reads the card back; the assertions here are the static half of
    // that proof.
    let facts = facts();
    let mut graph = gated(&facts);
    graph.add(Node::new("base", 2)).unwrap();
    graph
        .add(
            Node::new("left", 3)
                .with_prereqs(["base"])
                .with_paths(["src/graph.rs"])
                .with_approach("split the render into a summary card and one card per node"),
        )
        .unwrap();
    graph
        .add(Node::new("right", 4).with_prereqs(["base"]))
        .unwrap();
    graph
        .add(
            Node::new("ship", 6)
                .with_prereqs(["left", "right"])
                .with_paths(["src/graph.rs"]),
        )
        .unwrap();
    graph.dispatch("left").unwrap();
    land(&facts, "base");
    graph.settle();

    let card = render(&graph);
    for line in card.lines() {
        let width = plain(line).chars().count();
        assert!(
            width <= CARD_WIDTH,
            "a card line is {width} wide: {}",
            plain(line)
        );
    }
    if std::env::var_os("FLEET_GRAPH_CAPTURE").is_some() {
        print!("{card}");
    }
}
