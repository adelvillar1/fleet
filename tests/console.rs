//! The mini console's own tests, run from the Rust gate
//! (`docs/plans/2026-11-21-mini-console.md`): `tests/console/harness.mjs`
//! loads the real `src/gui/assets/console.js` both ways — as a core with no
//! page, and booted against a stub DOM and a stub bridge — and asserts what
//! it printed: each panel's four sentences by their exact words, the two
//! divergence words by name, an empty state as a sentence rather than a
//! zero, the wrapping law, and the shell's own size budget (60 KB
//! uncompressed, 20 KB gzip over the three files).
//!
//! Zero-dependency JavaScript, so the only requirement is a `node` on PATH.
//! Where there is none the check says so by name and skips rather than
//! failing the gate for a missing toolchain — the same shape as
//! `tests/face.rs` and the machine-dependent checks elsewhere.

use std::process::Command;

#[test]
fn the_mini_console_holds_its_contract() {
    match Command::new("node").arg("--version").output() {
        Ok(version) if version.status.success() => {}
        _ => {
            eprintln!(
                "console harness: no `node` on PATH, so tests/console/harness.mjs was skipped; \
                 install node to run the console's own checks"
            );
            return;
        }
    }
    let run = Command::new("node")
        .arg("tests/console/harness.mjs")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the console harness starts");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success(),
        "the console harness failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("checks passed"),
        "the harness printed no verdict:\n{stdout}\n{stderr}"
    );
    // The four panels are the contract, not a sample: a harness that ran
    // one case and said "checks passed" would pass this gate too.
    assert!(
        stdout.contains("every panel owes the four sentences"),
        "the harness did not check the four-sentence law:\n{stdout}"
    );
    assert!(
        stdout.contains("the shell fits its budget"),
        "the harness did not check the size budget:\n{stdout}"
    );
}
