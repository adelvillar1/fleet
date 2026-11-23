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

mod common;

#[test]
fn the_mini_console_holds_its_contract() {
    let node = match common::node() {
        Ok(Some(node)) => node,
        Ok(None) => {
            eprintln!(
                "console harness: no `node` on PATH, so tests/console/harness.mjs was skipped; \
                 install node to run the console's own checks"
            );
            return;
        }
        Err(why) => panic!("{why}"),
    };
    // KI-593: the console harness boots vm contexts too, so it runs on the same
    // pinned runtime as the face harness (tests/common::node).
    let run = Command::new(&node)
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
    // The clocks are the phase-2 contract (KI-587): a worker whose deadline,
    // fallback or sweep was removed must red a case by name, and the page's
    // watchdog and read deadline are the same gate.
    assert!(
        stdout.contains("the worker carries the KI-587 clock set"),
        "the harness did not check the worker's clocks:\n{stdout}"
    );
    assert!(
        stdout.contains("a read that goes quiet is dropped by name"),
        "the harness did not check the page's read deadline:\n{stdout}"
    );
    assert!(
        stdout.contains("the boot watchdog is armed at script evaluation"),
        "the harness did not check the boot watchdog:\n{stdout}"
    );
}
