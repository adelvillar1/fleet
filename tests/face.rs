//! The face's own tests, run from the Rust gate: tests/face/harness.mjs
//! boots the real `src/gui/assets/app.js` against a stubbed DOM and the
//! bridge's own document shapes, and asserts what the page built — the
//! work list's groups and the server's counts, the one open verb, the
//! conversation's head and switcher, a send beside driving work, and a
//! reply that renders at its block boundary.
//!
//! The harness is zero-dependency JavaScript, so the only requirement is
//! a `node` on PATH. Where there is none the check says so by name and
//! skips rather than failing the gate for a missing toolchain — the same
//! shape as the machine-dependent checks elsewhere.

use std::process::Command;

#[test]
fn the_face_work_surface_holds_its_contract() {
    match Command::new("node").arg("--version").output() {
        Ok(version) if version.status.success() => {}
        _ => {
            eprintln!(
                "face harness: no `node` on PATH, so tests/face/harness.mjs was skipped; \
                 install node to run the face's own checks"
            );
            return;
        }
    }
    let run = Command::new("node")
        .arg("tests/face/harness.mjs")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the harness starts");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success(),
        "the face harness failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("checks passed"),
        "the harness printed no verdict:\n{stdout}\n{stderr}"
    );
}
