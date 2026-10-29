//! The onboarding's own tests: the derivation reads the registries, the
//! refusals name what is wrong, and a skip survives a restart without
//! any new storage.

use super::*;
use std::collections::HashMap;

fn scratch(name: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!(
        "fleet-onboard-{}-{}-{}",
        name,
        std::process::id(),
        crate::boards::now_stamp().replace([':', '-'], "")
    ));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("a temp home");
    home
}

#[test]
fn the_sequence_is_the_declared_order() {
    assert_eq!(
        STEPS,
        &[
            STEP_HOME,
            STEP_LLM,
            STEP_LANES,
            STEP_PROJECTS,
            STEP_GITHUB,
            STEP_ROSTER
        ]
    );
    // The first step is the home, the second the account: the critical
    // path is two steps long and nothing else may block it.
    assert_eq!(STEPS[0], STEP_HOME);
    assert_eq!(STEPS[1], STEP_LLM);
}

#[test]
fn the_door_is_up_exactly_when_no_account_resolves() {
    let home = scratch("door");
    let dotenv = HashMap::new();
    let state = state(&home, &dotenv);
    let facts = Facts::read(&home, &dotenv);
    // The gate is the derivation's own: a machine reads ready exactly
    // when a candidate resolves a key — whatever this test process's
    // own environment happens to hold.
    assert_eq!(
        state.ready(),
        !facts.keyed.is_empty(),
        "ready must mean a candidate resolved"
    );
    assert_eq!(state.steps.len(), STEPS.len());
    assert_eq!(state.steps[0].id, STEP_HOME);
    assert_eq!(state.steps[1].id, STEP_LLM);
    let door = facts.door();
    // The door's copy carries no mechanism word anywhere.
    let words = format!(
        "{} {} {}",
        door["title"].as_str().unwrap_or_default(),
        door["lede"].as_str().unwrap_or_default(),
        door["action"].as_str().unwrap_or_default()
    );
    for banned in ["_API_KEY", "endpoint", "roster", "vault", "tier", "port"] {
        assert!(
            !words.contains(banned),
            "the door's default path names `{banned}`: {words}"
        );
    }
    // And the discovery press is offered exactly when a key is already
    // somewhere the roster resolves it from.
    assert_eq!(
        door["discovered"].as_bool().unwrap_or(false),
        !facts.present_keys.is_empty()
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_narration_is_derived_from_the_lane_registry() {
    let home = scratch("derived");
    let dotenv = HashMap::new();
    let facts = Facts::read(&home, &dotenv);
    // Every lane the registry declares appears in the lanes narration,
    // in the registry's own order — no copied list can drift from it.
    let copy = narration(STEP_LANES, &facts);
    let mut at = 0;
    for lane in lanes::LANES {
        let found = copy.mechanism.find(lane.name).unwrap_or_else(|| {
            panic!(
                "{} missing from the narration: {}",
                lane.name, copy.mechanism
            )
        });
        assert!(found >= at, "the lanes lost the registry's order");
        at = found;
        assert!(
            copy.mechanism.contains(lane.role),
            "{} role missing: {}",
            lane.name,
            copy.mechanism
        );
    }
    // The projects narration names the root the tower actually scans.
    let projects = narration(STEP_PROJECTS, &facts);
    assert!(
        projects
            .mechanism
            .contains(&facts.projects_root.display().to_string()),
        "{} must name {}",
        projects.mechanism,
        facts.projects_root.display()
    );
    // And the roster narration carries the floor the compaction reads.
    let roster = narration(STEP_ROSTER, &facts);
    assert!(
        roster.mechanism.contains(&facts.window_floor.to_string()),
        "{} must name the floor {}",
        roster.mechanism,
        facts.window_floor
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn refusals_are_named_in_plain_words() {
    let home = scratch("refusals");
    // A non-credential-shaped name is refused by its own rule.
    let refused = set_key(&home, "SOMETHING_ELSE", "value").unwrap_err();
    assert!(refused.contains("_KEY"), "{refused}");
    // An empty value refuses.
    let empty = set_key(&home, "STEPTEST_API_KEY", "   ").unwrap_err();
    assert!(empty.contains("empty"), "{empty}");
    // The one step that may not be declined says why.
    let cannot = skip(&home, STEP_LLM, "no consequence").unwrap_err();
    assert!(cannot.contains("cannot be skipped"), "{cannot}");
    // An unknown step is refused by name.
    let unknown = skip(&home, "not-a-step", "x").unwrap_err();
    assert!(unknown.contains("not an onboarding step"), "{unknown}");
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn a_skip_survives_a_restart_and_the_first_incomplete_moves_past_it() {
    let home = scratch("skip");
    let dotenv = HashMap::new();
    // Decline the lanes with the consequence the narration names.
    let facts = Facts::read(&home, &dotenv);
    let consequence = narration(STEP_LANES, &facts).skip;
    assert!(!consequence.is_empty(), "the lanes step must name its cost");
    skip(&home, STEP_LANES, &consequence).expect("the skip lands");

    // A second read — the restart — sees the decision, with no state of
    // the onboarding's own anywhere on disk.
    let decided = decisions(&home);
    assert_eq!(decided.skipped, vec![STEP_LANES]);
    let state = state(&home, &dotenv);
    let lanes_step = state.step(STEP_LANES).expect("the lanes step");
    assert!(lanes_step.skipped, "the skip must be read back");
    assert!(
        !state
            .steps
            .iter()
            .any(|step| step.id == STEP_LANES && !step.skipped && !step.done),
        "a declined step must not read as outstanding"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_home_step_lays_out_what_it_checks() {
    let home = scratch("layout");
    let dotenv = HashMap::new();
    let before = state(&home, &dotenv);
    assert!(!before.step(STEP_HOME).expect("the home step").done);
    for name in HOME_LAYOUT {
        crate::install::layout_step(&home, name).expect("a layout step");
    }
    crate::install::config_step(&home).expect("the config scaffold");
    let after = state(&home, &dotenv);
    assert!(
        after.step(STEP_HOME).expect("the home step").done,
        "the home step's own proof must hold after its own action"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_key_shapes_name_their_accounts() {
    assert_eq!(
        provider_for_key_shape("sk-ant-api03-abc"),
        Some("anthropic")
    );
    assert_eq!(provider_for_key_shape("sk-proj-abc"), Some("openai"));
    assert_eq!(provider_for_key_shape("gsk_abc"), Some("groq"));
    // An unknown shape names nothing rather than guessing — the door
    // asks which account it is.
    assert_eq!(provider_for_key_shape("plain-old-key"), None);
}

#[test]
fn the_onboarding_fact_is_the_one_the_probe_reads_back() {
    let home = scratch("fact");
    let dotenv = HashMap::new();
    let facts = Facts::read(&home, &dotenv);
    let name = remember_onboarding(&home, &facts).expect("the fact lands");
    let completion = completion(&home, &dotenv);
    assert!(
        completion.recalled_fact,
        "the probe must read back the fact the onboarding wrote ({name})"
    );
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn the_completion_is_green_only_when_all_four_reads_hold() {
    let home = scratch("completion");
    let dotenv = HashMap::new();
    let completion = completion(&home, &dotenv);
    assert!(!completion.green(), "an empty home is not ready");
    // Each read is independent: the document names all four.
    let document = completion.to_json();
    for key in ["doctor", "routed_turn", "recalled_fact", "tower_project"] {
        assert!(
            document.get(key).is_some(),
            "the completion names its {key} read"
        );
    }
    std::fs::remove_dir_all(&home).ok();
}
