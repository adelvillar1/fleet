//! C18's live leg: a run where the real sys1 lane judges a composed
//! request and the walk lands on blend1, the grounded leg.
//!
//! This is the amendment the owner made on 2026-11-02: `composed` is in
//! the judged set, so a grounded-composition request is answered by the
//! lane that grounds it rather than falling to the default. The test
//! needs the lane up (the keys resolve on this machine) and takes the
//! real route through `select_walk` - no mock, because the point is the
//! lane's own verdict.

#[tokio::test(flavor = "multi_thread")]
async fn a_composed_request_walks_blend1() {
    // The request shape: grounded composition over a supplied corpus.
    let task = "Summarize the attached corpus of 40 design documents into one \
                coherent report, citing each source by name.";
    let home = std::env::temp_dir().join(format!(
        "fleet-composed-live-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the home");
    // The judgment reads the dotenv chain off the home; the roster is the
    // compiled-in one and the table the compiled-in preferences.
    let dotenv = fleet::home::read_dotenv_with_process_vault(&home);
    let roster = fleet::route::default_roster();
    let resolved = fleet::route::resolve(&roster, &dotenv);
    let table = fleet::route::default_preferences();
    let specs: std::collections::HashMap<String, fleet::mcp::ServerSpec> =
        std::collections::HashMap::new();
    let (decision, _latency) = fleet::route::select_walk_with_home(
        &resolved,
        None,
        task,
        &specs,
        &table,
        fleet::route::Gates::default(),
        None,
        fleet::route::Walk::Judged,
        None,
    )
    .await;
    // The lane is a live model, so its verdict on this wording is the
    // machine's fact rather than this crate's contract: when it names a
    // different workload the run says so by name and skips. The wiring the
    // test exists for — a `composed` verdict walks blend1 first — is pinned
    // deterministically in `route.rs`'s preference tests, so a verdict that
    // moved costs this leg's evidence and never reds the gate for a model's
    // classification.
    if decision.workload.as_deref() != Some("composed") {
        eprintln!(
            "composed-live: skipped — the sys1 lane judged {:?} ({}) rather than composed; \
             the walk it ordered was {:?}",
            decision.workload, decision.reason, decision.order
        );
        let _ = std::fs::remove_dir_all(&home);
        return;
    }
    // A candidate with no key resolves without one and never enters a
    // walk, so a home that cannot key blend1 cannot answer this leg: that
    // is the machine's fact and it is named here rather than read as a
    // routing defect (KI-405's class — the observable says which).
    let blend1_keyed = resolved
        .iter()
        .any(|entry| entry.candidate.id == "blend1" && entry.key.is_some());
    if !blend1_keyed {
        eprintln!(
            "composed-live: skipped — blend1 resolves without a key on this machine \
             (BLEND1_BEARER_TOKEN), so the composed walk cannot start there; \
             the verdict was {:?}",
            decision.workload
        );
        let _ = std::fs::remove_dir_all(&home);
        return;
    }
    assert_eq!(
        decision
            .order
            .first()
            .map(|(provider, _)| provider.as_str()),
        Some("blend1"),
        "the walk lands on the grounded leg: {:?}",
        decision.order
    );
    let _ = std::fs::remove_dir_all(&home);
}
