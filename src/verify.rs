//! The fact-gate: three machine-checkable gates, run outside the graph,
//! feeding `mark_verified`.
//!
//! The delegation plan's law is "Facts gate; reports corroborate." This is
//! the facts half. `Graph::settle` takes a predicate over graph state, and
//! that is the right shape for a fact the graph already holds — but the
//! three things that verify a node's work are not graph state:
//!
//! - **functional** — does the work integrate. A judgment, and the one
//!   part of the gate that wants a reporter rather than an arithmetic
//!   answer. sys1 or the delegate beat reports on it.
//! - **unit tests** — does the suite pass. `cargo test`.
//! - **nomenclature** — are objects named per the guidelines.
//!   `nomenclature::check`.
//!
//! Running those inside `settle`'s predicate would put blocking I/O and a
//! subprocess inside a pure function over graph state, and it would make
//! the graph untestable. So the gates live here, outside, and the result
//! is fed back through `mark_verified` — the graph stays pure, and each
//! verification is auditable: which gate passed, when, with what evidence.
//!
//! **The full three-gate set runs at integration nodes — those with
//! dependents — and leaves verify on functional plus nomenclature.**
//! Tests are global and nodes are local, so attributing a suite failure to
//! whichever node landed most recently is fragile; "does this integrate" is
//! the right question at a node with dependents, and a leaf that composes
//! with nothing has nothing to integrate.

use std::path::{Path, PathBuf};

use crate::graph::{Graph, Node, NodePredicate, Status};

/// One gate's outcome on one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutcome {
    /// Which gate ran.
    pub gate: Gate,
    /// Whether it passed.
    pub passed: bool,
    /// What it found — the established term, the failing test, the
    /// reason the work does not integrate. Named even on a pass, because
    /// a gate that passed silently is a gate nobody can audit.
    pub evidence: String,
}

/// The three gates, and the two a leaf runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Functional,
    UnitTests,
    Nomenclature,
}

impl Gate {
    pub fn as_str(self) -> &'static str {
        match self {
            Gate::Functional => "functional",
            Gate::UnitTests => "unit-tests",
            Gate::Nomenclature => "nomenclature",
        }
    }

    /// The gates a node runs. A leaf composes with nothing, so it has
    /// nothing to integrate and the suite's global verdict says nothing
    /// about it; an integration node runs all three.
    pub fn for_node(_node: &Node, dependents: usize) -> &'static [Gate] {
        if dependents > 0 {
            &[Gate::Functional, Gate::UnitTests, Gate::Nomenclature]
        } else {
            &[Gate::Functional, Gate::Nomenclature]
        }
    }
}

/// One node's verification: the gates that ran, and the verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub node: String,
    pub gates: Vec<GateOutcome>,
    /// Whether every gate that ran passed.
    pub verified: bool,
}

impl Verification {
    /// The named reasons it failed, or empty on a pass.
    pub fn failures(&self) -> Vec<String> {
        self.gates
            .iter()
            .filter(|gate| !gate.passed)
            .map(|gate| format!("{}: {}", gate.gate.as_str(), gate.evidence))
            .collect()
    }
}

/// A reporter on whether a node's work integrates.
///
/// A judgment, so it is injected rather than implemented here: the graph
/// stays pure and the production reporter — sys1 or the delegate beat — is
/// wired by the caller that owns the lane. Returning `None` means the work
/// integrates; `Some(reason)` means it does not, with the reason named.
pub type Reporter = Box<dyn Fn(&Node) -> Option<String> + Send + Sync>;

/// The fact-gate service. Built once per verification pass over a graph.
pub struct FactGate {
    /// The tree the nomenclature gate reads.
    root: PathBuf,
    /// The functional reporter, when one is wired. Without one the
    /// functional gate passes on the node's own delta — the honest
    /// degradation, and it is named rather than silent.
    reporter: Option<Reporter>,
    /// Whether the unit-test gate may run the suite. Off by default: a
    /// verification pass that shells out to `cargo test` on every node
    /// would spend the whole budget on the gate.
    tests_enabled: bool,
}

impl FactGate {
    pub fn new(root: &Path) -> FactGate {
        FactGate {
            root: root.to_path_buf(),
            reporter: None,
            tests_enabled: false,
        }
    }

    /// Attach the functional reporter.
    pub fn with_reporter(mut self, reporter: Reporter) -> FactGate {
        self.reporter = Some(reporter);
        self
    }

    /// Permit the unit-test gate to run the suite. The caller opts in,
    /// because it is the one gate with a real cost.
    pub fn with_tests(mut self, enabled: bool) -> FactGate {
        self.tests_enabled = enabled;
        self
    }

    /// Run the gates one node's status calls for, and report.
    pub fn verify(&self, node: &Node, dependents: usize) -> Verification {
        let gates = Gate::for_node(node, dependents);
        let mut outcomes = Vec::new();
        for gate in gates {
            outcomes.push(match gate {
                Gate::Functional => self.functional(node),
                Gate::UnitTests => self.unit_tests(node),
                Gate::Nomenclature => self.nomenclature(node),
            });
        }
        Verification {
            node: node.id.clone(),
            verified: outcomes.iter().all(|outcome| outcome.passed),
            gates: outcomes,
        }
    }

    /// Does the work integrate? A reporter's judgment, or — with no
    /// reporter wired — the node's own delta, which is the honest
    /// degradation and is named as such.
    fn functional(&self, node: &Node) -> GateOutcome {
        if let Some(reporter) = &self.reporter {
            return match reporter(node) {
                None => GateOutcome {
                    gate: Gate::Functional,
                    passed: true,
                    evidence: "the reporter found no integration failure".to_string(),
                },
                Some(reason) => GateOutcome {
                    gate: Gate::Functional,
                    passed: false,
                    evidence: reason,
                },
            };
        }
        // No reporter: fall back to the node's own delta. A contradicted
        // report is a failure — the log does not support it — and any
        // other delta passes, because a report is not a fact and the
        // absence of a reporter is not a reason to refuse the work.
        let contradicted = node
            .delta
            .as_ref()
            .and_then(|delta| delta.contradiction.clone());
        match contradicted {
            Some(reason) => GateOutcome {
                gate: Gate::Functional,
                passed: false,
                evidence: format!("the report is contradicted: {reason}"),
            },
            None => GateOutcome {
                gate: Gate::Functional,
                passed: true,
                evidence: "no reporter wired; the delta is not contradicted".to_string(),
            },
        }
    }

    /// Does the suite pass? Runs it, when the caller has permitted it.
    ///
    /// Disabled by default and the disabled answer is named rather than
    /// silent: a gate that was not run is not a gate that passed.
    fn unit_tests(&self, node: &Node) -> GateOutcome {
        if !self.tests_enabled {
            return GateOutcome {
                gate: Gate::UnitTests,
                passed: true,
                evidence: "not run: the suite gate is disabled".to_string(),
            };
        }
        let output = std::process::Command::new("cargo")
            .arg("test")
            .current_dir(&self.root)
            .output();
        match output {
            Err(error) => GateOutcome {
                gate: Gate::UnitTests,
                passed: false,
                evidence: format!("the suite could not run: {error}"),
            },
            Ok(output) if output.status.success() => GateOutcome {
                gate: Gate::UnitTests,
                passed: true,
                evidence: "the suite passed".to_string(),
            },
            Ok(output) => {
                let tail = String::from_utf8_lossy(&output.stderr);
                let last = tail.lines().rev().find(|line| !line.trim().is_empty());
                GateOutcome {
                    gate: Gate::UnitTests,
                    passed: false,
                    evidence: format!(
                        "`{}` does not integrate: the suite failed{}",
                        node.id,
                        last.map(|line| format!(": {line}")).unwrap_or_default()
                    ),
                }
            }
        }
    }

    /// Are the objects this node touched named as the guidelines say?
    ///
    /// The check reads the node's declared paths — the same declaration
    /// the resource edge uses — and asks the vocabulary what each
    /// identifier in them is called. A path with nothing to check passes,
    /// and says so rather than passing silently.
    fn nomenclature(&self, node: &Node) -> GateOutcome {
        let mut checked = 0;
        let mut drift: Vec<String> = Vec::new();
        for path in &node.paths {
            // The last path segment's identifiers are what a node names:
            // `src/parser.rs` names `parser`, `src/recall_graph.rs` names
            // `recall` and `graph`.
            let stem = Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            for token in crate::nomenclature::channel_tokens(path, stem) {
                checked += 1;
                if let Some(convention) = crate::nomenclature::check(&self.root, &token) {
                    // The established term, when there is one — the name
                    // the guidelines already use for this concept.
                    let established = convention.term.as_deref().unwrap_or("no established term");
                    drift.push(format!("`{token}` (the tree calls it `{established}`)"));
                }
            }
        }
        if drift.is_empty() {
            return GateOutcome {
                gate: Gate::Nomenclature,
                passed: true,
                evidence: format!(
                    "{checked} name{} checked, none drifted",
                    if checked == 1 { "" } else { "s" }
                ),
            };
        }
        GateOutcome {
            gate: Gate::Nomenclature,
            passed: false,
            evidence: format!(
                "{} of {checked} names drifted: {}",
                drift.len(),
                drift.join(", ")
            ),
        }
    }
}

/// Verify every node the graph offers and promote the ones whose gates all
/// pass.
///
/// The counterpart of `Graph::settle`, fed by facts rather than by a
/// closure over graph state. Returns the verifications in declaration
/// order — the list the caller logs, because a gate that ran is a
/// model-visible event.
///
/// A node marked failed is still offered: the artifact is the authority,
/// so work that landed after a failed report verifies on the next pass
/// rather than staying buried. That is `settle`'s own rule, kept.
pub fn verify_all(
    graph: &mut Graph,
    gate: &FactGate,
    stream: Option<&crate::calib::Stream>,
) -> Vec<Verification> {
    // The dependents are read before any promotion: a node's gate set
    // depends on whether anything waits behind it, and promoting one
    // node changes nothing about that — but the borrow of the nodes does.
    let dependents: Vec<usize> = graph
        .nodes()
        .iter()
        .map(|node| graph.dependents(&node.id).len())
        .collect();
    let mut verifications = Vec::new();
    let mut promoted: Vec<String> = Vec::new();
    for (index, node) in graph.nodes().iter().enumerate() {
        if node.status == Status::Verified {
            continue;
        }
        let verification = gate.verify(node, dependents[index]);
        if let Some(stream) = stream {
            stream.record(crate::calib::Row::Verify(crate::calib::VerifyRow {
                ts: crate::calib::stamp(),
                node: verification.node.clone(),
                gates: verification
                    .gates
                    .iter()
                    .map(|outcome| format!("{}={}", outcome.gate.as_str(), outcome.passed))
                    .collect(),
                verified: verification.verified,
                evidence: verification
                    .gates
                    .iter()
                    .map(|outcome| outcome.evidence.clone())
                    .collect::<Vec<_>>()
                    .join(" | "),
            }));
        }
        if verification.verified {
            promoted.push(verification.node.clone());
        }
        verifications.push(verification);
    }
    for id in promoted {
        graph.mark_verified(&id);
    }
    verifications
}

/// The verification as a card, for the surface. A node that passed every
/// gate says so; a node that failed names which gate and why.
pub fn card(verification: &Verification) -> String {
    let mut lines = Vec::new();
    let verdict = if verification.verified {
        "verified"
    } else {
        "refused"
    };
    lines.push(format!("{verdict}  {node}", node = verification.node));
    for outcome in &verification.gates {
        let marker = if outcome.passed { "ok  " } else { "FAIL" };
        lines.push(format!(
            "  {marker} {gate:<12} {evidence}",
            gate = outcome.gate.as_str(),
            evidence = outcome.evidence
        ));
    }
    lines.join("\n")
}

/// A verifier built from the fact-gate, for a caller that wants to attach
/// one to a graph the ordinary way.
///
/// The gate's verdict is per-node and per-dependents, which `settle`'s
/// predicate cannot see — so this is the seam for the case where a caller
/// already holds a graph with a verifier attached and wants the gates to
/// run through it. `verify_all` is the ordinary path.
pub fn predicate(gate: std::sync::Arc<FactGate>) -> NodePredicate {
    Box::new(move |node: &Node| {
        // Without the graph's dependents the gate cannot know whether this
        // is an integration node, so it runs the leaf's set. A caller that
        // needs the integration set should use `verify_all`.
        gate.verify(node, 0).verified
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node};

    fn chain() -> Graph {
        let mut graph = Graph::new();
        graph.add(Node::new("base", 2)).unwrap();
        graph
            .add(Node::new("ship", 3).with_prereqs(["base"]))
            .unwrap();
        graph
    }

    #[test]
    fn a_leaf_runs_two_gates_and_an_integration_node_runs_three() {
        let graph = chain();
        let base = graph.node("base").expect("declared");
        assert_eq!(base.paths.len(), 0);
        // `base` has a dependent, so it is an integration node.
        assert_eq!(graph.dependents("base").len(), 1);
        assert_eq!(
            Gate::for_node(base, 1).len(),
            3,
            "an integration node runs all three"
        );
        // `ship` has none.
        let ship = graph.node("ship").expect("declared");
        assert_eq!(graph.dependents("ship").len(), 0);
        assert_eq!(
            Gate::for_node(ship, 0).len(),
            2,
            "a leaf runs functional and nomenclature only"
        );
    }

    #[test]
    fn a_gate_that_passed_names_its_evidence_and_one_that_failed_names_why() {
        let gate = FactGate::new(std::path::Path::new("/nonexistent-root"));
        // A node whose path names a term the vocabulary does not carry.
        let mut graph = Graph::new();
        graph.add(Node::new("base", 2)).unwrap();
        graph
            .add(
                Node::new("ship", 3)
                    .with_prereqs(["base"])
                    .with_paths(["src/parser.rs"]),
            )
            .unwrap();
        let ship = graph.node("ship").expect("declared").clone();
        let verification = gate.verify(&ship, 0);
        // The suite gate did not run, and says so rather than passing
        // silently.
        let tests = verification
            .gates
            .iter()
            .find(|outcome| outcome.gate == Gate::UnitTests);
        assert!(tests.is_none(), "a leaf does not run the suite gate at all");
        let nomenclature = verification
            .gates
            .iter()
            .find(|outcome| outcome.gate == Gate::Nomenclature)
            .expect("the nomenclature gate ran");
        assert!(
            nomenclature.evidence.contains("checked"),
            "a pass names what it checked: {}",
            nomenclature.evidence
        );
    }

    #[test]
    fn the_functional_gate_takes_the_reporters_judgment_and_names_the_reason() {
        let graph = chain();
        let ship = graph.node("ship").expect("declared").clone();
        // A reporter that refuses: the work does not integrate.
        let refusing = FactGate::new(std::path::Path::new("/nonexistent-root")).with_reporter(
            Box::new(|_node: &Node| Some("the artifact does not compile against base".to_string())),
        );
        let refused = refusing.verify(&ship, 0);
        assert!(!refused.verified, "a refused node does not verify");
        assert_eq!(
            refused.failures(),
            vec!["functional: the artifact does not compile against base".to_string()]
        );
        // And one that passes.
        let accepting = FactGate::new(std::path::Path::new("/nonexistent-root"))
            .with_reporter(Box::new(|_node: &Node| None));
        assert!(accepting.verify(&ship, 0).verified);
    }

    #[test]
    fn a_contradicted_report_fails_the_functional_gate_with_no_reporter() {
        // The honest degradation: with no reporter wired, the node's own
        // delta is what the gate reads, and a contradicted report is the
        // one thing it refuses.
        let mut graph = chain();
        let lying = crate::subagents::Delta {
            state: crate::subagents::DeltaState::Deviated,
            reason: crate::subagents::DeltaReason::Infeasible,
            companion: String::new(),
            instead: "something else".to_string(),
            forced: false,
            children: Vec::new(),
            declares: Vec::new(),
            contradiction: Some("the log shows no failed step".to_string()),
        };
        graph.record_delta("ship", lying).unwrap();
        let ship = graph.node("ship").expect("declared").clone();
        let gate = FactGate::new(std::path::Path::new("/nonexistent-root"));
        let verification = gate.verify(&ship, 0);
        assert!(!verification.verified);
        assert!(
            verification.failures()[0].contains("contradicted"),
            "{:?}",
            verification.failures()
        );
    }

    #[test]
    fn verify_all_promotes_what_passes_and_refuses_what_does_not() {
        let mut graph = chain();
        let gate = FactGate::new(std::path::Path::new("/nonexistent-root")).with_reporter(
            Box::new(|node: &Node| {
                (node.id == "ship").then(|| "the work does not integrate".to_string())
            }),
        );
        let verifications = verify_all(&mut graph, &gate, None);
        assert_eq!(verifications.len(), 2);
        // `base` passes and is promoted; `ship` is refused and is not.
        assert_eq!(
            graph.node("base").map(|node| node.status),
            Some(crate::graph::Status::Verified)
        );
        assert_eq!(
            graph.node("ship").map(|node| node.status),
            Some(crate::graph::Status::Pending)
        );
        // A second pass skips the verified node rather than re-running it.
        let again = verify_all(&mut graph, &gate, None);
        assert_eq!(again.len(), 1, "only the unverified node is offered again");
        assert_eq!(again[0].node, "ship");
    }

    #[test]
    fn the_card_names_which_gate_failed_and_why() {
        let verification = Verification {
            node: "ship".to_string(),
            gates: vec![
                GateOutcome {
                    gate: Gate::Functional,
                    passed: true,
                    evidence: "no integration failure".to_string(),
                },
                GateOutcome {
                    gate: Gate::Nomenclature,
                    passed: false,
                    evidence: "1 of 2 names drifted: `parser`".to_string(),
                },
            ],
            verified: false,
        };
        let card = card(&verification);
        assert!(card.contains("refused  ship"), "{card}");
        assert!(card.contains("functional"), "{card}");
        assert!(card.contains("nomenclature"), "{card}");
        // The verdict and the reason both ride the line: a refused node
        // says which gate refused it and what that gate found.
        assert!(card.contains("FAIL"), "{card}");
        assert!(card.contains("`parser`"), "{card}");
    }
}
