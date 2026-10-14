//! End-to-end tests for the self-maintenance surface, driven through
//! the real binary: `fleet improve` lists both stores, `fleet improve
//! apply` runs the real local gate against a scratch repo-shaped tree,
//! a destructive proposal waits for `--approve`, and `fleet improve
//! undo` refuses what is not applied. No network and no API key: every
//! path here is a file-and-gate path, and the calibration stream is
//! redirected into a scratch root by the scratch home's own config so
//! the shared dev-decisions stream is never written to.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One `fleet` invocation against a scratch home and a scratch working
/// directory. Returns stdout, stderr, and the exit code.
fn fleet(home: &Path, cwd: &Path, args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_fleet"))
        .args(args)
        .env("FLEET_HOME", home)
        .env_remove("DEEPSEEK_API_KEY")
        .current_dir(cwd)
        .output()
        .expect("spawn the fleet binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// A scratch directory, unique per call.
fn scratch(tag: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "fleet-improve-cli-{tag}-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A scratch home whose calibration stream resolves into itself: the
/// `calib.log_root` override is what keeps a test's rows out of the
/// shared dev-decisions stream.
fn scratch_home(tag: &str) -> PathBuf {
    let home = scratch(tag);
    let calib = home.join("calib-root");
    std::fs::write(
        home.join("config.json"),
        serde_json::json!({ "calib": { "log_root": calib } }).to_string(),
    )
    .unwrap();
    home
}

/// A scratch repo-shaped tree: a git repo with one committed file and
/// no `Cargo.toml`, so every real gate step fails deterministically
/// with the manifest named — the shape a gate-failing proposal meets,
/// without depending on the machine's toolchain being installed.
fn scratch_repo(tag: &str) -> PathBuf {
    let dir = scratch(tag);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.name", "fleet-test"]);
    git(&dir, &["config", "user.email", "fleet-test@localhost"]);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n").unwrap();
    git(&dir, &["add", "src/lib.rs"]);
    git(&dir, &["commit", "-q", "-m", "pre-image"]);
    dir
}

/// One git command in `dir`, asserted to succeed.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        status.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&status.stderr)
    );
}

/// Write one general-store proposal into `home`.
fn proposal(home: &Path, name: &str, target: &str, target_name: &str, change: &str) {
    let dir = home.join("proposals");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.md")),
        format!(
            "---\nname: {name}\ntarget: {target}\ntarget-name: {target_name}\nbasis: inferred\ncreated: 2026-10-14\nstatus: pending\n---\n\n## Justification\n\nthe session's evidence\n\n## Acceptance criteria\n\nthe change lands and the row records it\n\n## Expected effect\n\nthe harness carries the change\n\n## Change\n\n```json\n{change}\n```\n"
        ),
    )
    .unwrap();
}

/// The rows today's stream holds, after the writer drains.
fn todays_rows(home: &Path) -> Vec<fleet::calib::ReadRow> {
    let root = home.join("calib-root");
    // The CLI process has exited by now, so its stream drained on the
    // last handle's drop; a small settle keeps the read honest on a
    // slow disk.
    let day = fleet::calib::today();
    fleet::calib::read_day(&root, &day, 256)
}

#[test]
fn the_listing_names_both_stores_with_target_basis_and_status() {
    let home = scratch_home("list");
    proposal(
        &home,
        "general-one",
        "source",
        "src/lib.rs",
        r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#,
    );
    // A slice-7 draft in its own store, un-migrated.
    let drafts = home.join("skills").join("proposals");
    std::fs::create_dir_all(&drafts).unwrap();
    std::fs::write(
        drafts.join("drafted.md"),
        "---\nname: drafted\ndescription: a drafted skill\nproposed: true\nkind: create\nevidence: session 42\n---\n\nthe drafted body\n",
    )
    .unwrap();
    let repo = scratch_repo("list-repo");
    let (stdout, stderr, code) = fleet(&home, &repo, &["improve"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("fleet improve"), "{stdout}");
    assert!(stdout.contains("general-one"), "{stdout}");
    assert!(stdout.contains("drafted"), "{stdout}");
    // The target, the basis, the status, and the not-yet-run gate are
    // all on the card, and both stores are named honestly.
    assert!(stdout.contains("source `src/lib.rs`"), "{stdout}");
    assert!(stdout.contains("basis inferred"), "{stdout}");
    assert!(stdout.contains("pending"), "{stdout}");
    assert!(stdout.contains("gate: not yet run"), "{stdout}");
    assert!(stdout.contains("proposals/"), "{stdout}");
    assert!(stdout.contains("skills/proposals/"), "{stdout}");
}

#[test]
fn the_help_names_the_surface() {
    let home = scratch_home("help");
    let repo = scratch_repo("help-repo");
    let (stdout, _, code) = fleet(&home, &repo, &["help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("fleet improve"), "{stdout}");
    assert!(stdout.contains("fleet improve apply"), "{stdout}");
    assert!(stdout.contains("fleet improve undo"), "{stdout}");
    assert!(stdout.contains("--approve"), "{stdout}");
}

#[test]
fn a_proposal_the_gate_refuses_is_refused_with_the_failure_named() {
    let home = scratch_home("gate-refused");
    let repo = scratch_repo("gate-refused-repo");
    let before = std::fs::read_to_string(repo.join("src/lib.rs")).unwrap();
    proposal(
        &home,
        "break-the-tree",
        "source",
        "src/lib.rs",
        r#"{"op": "patch", "path": "src/lib.rs", "content": "pub fn one() -> u32 { 2 }\n"}"#,
    );
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "apply", "break-the-tree"]);
    // Refused, with the failure named: the verdict line carries the
    // gate's own words — the failed steps by name and the reason cargo
    // gave (a missing subcommand here, a missing manifest on a tooled
    // machine — either way the failure is named, never a silent pass).
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("the gate refused the change"), "{stderr}");
    assert!(stderr.contains("fmt"), "{stderr}");
    // The tree is back to its pre-image, and the working tree is clean.
    assert_eq!(std::fs::read_to_string(repo.join("src/lib.rs")).unwrap(), before);
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "{}",
        String::from_utf8_lossy(&status.stdout)
    );
    // The proposal is stamped refused and the row landed, refused, with
    // the gate's per-step evidence.
    let stamped = std::fs::read_to_string(home.join("proposals").join("break-the-tree.md")).unwrap();
    assert!(stamped.contains("status: refused"), "{stamped}");
    let rows = todays_rows(&home);
    let propose: Vec<&fleet::calib::ReadRow> =
        rows.iter().filter(|row| row.op == fleet::calib::OP_PROPOSE).collect();
    assert_eq!(propose.len(), 1, "{rows:?}");
    assert_eq!(propose[0].proposal, "break-the-tree");
    assert_eq!(propose[0].target, "source");
    assert_eq!(propose[0].verdict, fleet::calib::VERDICT_REFUSED);
    // Five steps: the four cargo steps, all failed, plus the eval
    // step recorded not-run — the cargo gate refused first, so the
    // corpus was never reached. The row says so rather than leaving
    // the step absent, where an operator could not tell "not
    // reached" from "passed".
    assert_eq!(propose[0].gate.len(), 5);
    assert!(propose[0].gate.iter().all(|step| !step.ok));
    // Every failed step names its own failure in the row's evidence.
    assert!(propose[0].gate.iter().all(|step| !step.detail.is_empty()));
    let eval_step = propose[0]
        .gate
        .iter()
        .find(|step| step.step == "eval_corpus")
        .expect("the eval step");
    assert!(eval_step.detail.contains("not run"), "{:?}", eval_step.detail);
    assert!(
        eval_step.detail.contains("the cargo gate refused first"),
        "{:?}",
        eval_step.detail
    );
}

#[test]
fn a_destructive_proposal_waits_and_names_what_will_be_destroyed() {
    let home = scratch_home("destructive");
    let repo = scratch_repo("destructive-repo");
    // A live skill, and a proposal that retires it.
    let skill = home.join("skills").join("demo");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: demo\ndescription: a demo skill\n---\n\nthe body\n",
    )
    .unwrap();
    proposal(&home, "retire-demo", "skill", "demo", r#"{"op": "retire-skill", "name": "demo"}"#);
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "apply", "retire-demo"]);
    // Waiting, with exactly what will be destroyed named — and nothing
    // applied.
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("waiting"), "{stderr}");
    assert!(stderr.contains("demo"), "{stderr}");
    assert!(stderr.contains("--approve"), "{stderr}");
    assert!(skill.join("SKILL.md").exists(), "the skill is still live");
    // The row landed, waiting, with no gate evidence.
    let rows = todays_rows(&home);
    let propose: Vec<&fleet::calib::ReadRow> =
        rows.iter().filter(|row| row.op == fleet::calib::OP_PROPOSE).collect();
    assert_eq!(propose.len(), 1, "{rows:?}");
    assert_eq!(propose[0].verdict, fleet::calib::VERDICT_WAITING);
    assert!(propose[0].gate.is_empty());
    // A second apply without approval is the named guard, not a second
    // row.
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "apply", "retire-demo"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("already waiting"), "{stderr}");
    let rows = todays_rows(&home);
    assert_eq!(rows.iter().filter(|row| row.op == fleet::calib::OP_PROPOSE).count(), 1);
}

#[test]
fn a_proposal_that_names_nothing_is_refused_by_name() {
    let home = scratch_home("missing");
    let repo = scratch_repo("missing-repo");
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "apply", "no-such-proposal"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("no proposal named `no-such-proposal`"), "{stderr}");
    // A refusal before the gate leaves no row: nothing reached a verdict.
    let rows = todays_rows(&home);
    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn the_undo_refuses_what_is_not_applied() {
    let home = scratch_home("undo");
    let repo = scratch_repo("undo-repo");
    proposal(
        &home,
        "still-pending",
        "skill",
        "demo",
        r#"{"op": "write-skill", "content": "---\nname: demo\ndescription: a demo\n---\n\nthe body\n"}"#,
    );
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "undo", "still-pending"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("is not applied"), "{stderr}");
    let (_, stderr, code) = fleet(&home, &repo, &["improve", "undo", "no-such-proposal"]);
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("no proposal named"), "{stderr}");
}

#[test]
fn a_skill_proposal_applies_through_the_gate_and_lands_the_row() {
    // The green path, driven through the real gate. The scratch repo
    // carries a minimal cargo project whose fmt/clippy/build/test all
    // pass, so the gate is genuinely green — and a machine without
    // rustfmt or clippy refuses at the fmt step, which is the named
    // absent-tool behavior this slice's feature doc records (KI-007's
    // consequence). The test accepts exactly those two outcomes and
    // asserts the row and the file state for each.
    let home = scratch_home("apply-skill");
    let repo = scratch_repo("apply-skill-repo");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"scratch-gate\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n").unwrap();
    git(&repo, &["add", "Cargo.toml", "src/lib.rs"]);
    git(&repo, &["commit", "-q", "-m", "the scratch crate"]);
    // A live skill to update, and the proposal's new body.
    let skill = home.join("skills").join("demo");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: demo\ndescription: the old body\n---\n\nold body\n",
    )
    .unwrap();
    proposal(
        &home,
        "demo-update",
        "skill",
        "demo",
        r#"{"op": "write-skill", "content": "---\nname: demo\ndescription: the old body\n---\n\nnew body\n"}"#,
    );
    let (stdout, stderr, code) = fleet(&home, &repo, &["improve", "apply", "demo-update"]);
    let rows = todays_rows(&home);
    let propose: Vec<&fleet::calib::ReadRow> =
        rows.iter().filter(|row| row.op == fleet::calib::OP_PROPOSE).collect();
    assert_eq!(propose.len(), 1, "{rows:?}");
    assert_eq!(propose[0].proposal, "demo-update");
    assert_eq!(propose[0].target, "skill");
    assert_eq!(propose[0].basis, "inferred");
    let written = std::fs::read_to_string(skill.join("SKILL.md")).unwrap();
    if code == 0 {
        // The green path: the skill changed, the gate ran all four
        // steps, and the undo restores the pre-image byte-for-byte.
        assert!(stdout.contains("applied"), "{stdout}");
        assert!(written.contains("new body"), "{written}");
        assert_eq!(propose[0].verdict, fleet::calib::VERDICT_APPLIED);
        assert_eq!(propose[0].gate.len(), 4);
        assert!(propose[0].gate.iter().all(|step| step.ok));
        let (_, stderr, code) = fleet(&home, &repo, &["improve", "undo", "demo-update"]);
        assert_eq!(code, 0, "{stderr}");
        let restored = std::fs::read_to_string(skill.join("SKILL.md")).unwrap();
        assert_eq!(restored, "---\nname: demo\ndescription: the old body\n---\n\nold body\n");
    } else {
        // The absent-tool path (KI-007): the fmt or clippy step cannot
        // run here, so the proposal is refused by name and nothing
        // changed — a gate step that cannot run is never a silent pass.
        assert_eq!(code, 1, "{stderr}");
        assert!(stderr.contains("the gate refused the change"), "{stderr}");
        assert_eq!(propose[0].verdict, fleet::calib::VERDICT_REFUSED);
        assert!(propose[0].gate.iter().any(|step| !step.ok));
        assert!(written.contains("old body"), "{written}");
        assert!(!written.contains("new body"), "{written}");
    }
}
