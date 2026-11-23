//! The face's own tests, run from the Rust gate: tests/face/harness.mjs
//! boots the real `src/gui/assets/app.js` against a stubbed DOM and the
//! bridge's own document shapes, and asserts what the page built — the
//! work list's groups and the server's counts, the one open verb, the
//! conversation's head and switcher, a send beside driving work, and a
//! reply that renders at its block boundary.
//!
//! The harness is zero-dependency JavaScript, but it requires **Node 26 or
//! newer** (KI-593): its ~290 fresh vm contexts provoked a V8 mark-compact
//! segfault on older builds (~1/10 full runs on v25.2.1; v26 measured clean).
//! The harness refuses an older runtime by name, and tests/common resolves a
//! 26+ binary rather than trusting PATH order. Where there is no `node` at all the
//! check still says so by name and skips — the same shape as the
//! machine-dependent checks elsewhere; a present-but-old node fails, because a
//! gate must not report green about a harness it never ran.

use std::process::Command;

mod common;

#[test]
fn the_face_work_surface_holds_its_contract() {
    let node = match common::node() {
        Ok(Some(node)) => node,
        Ok(None) => {
            eprintln!(
                "face harness: no `node` on PATH, so tests/face/harness.mjs was skipped; \
                 install node to run the face's own checks"
            );
            return;
        }
        Err(why) => panic!("{why}"),
    };
    // KI-593: the runtime is pinned to 26+ (face_node), which is the fix — the
    // crash belonged to older V8 builds. The old `--no-concurrent-marking`
    // workaround is gone on purpose: the gate must exercise the default GC the
    // owner's manual runs use, or a residual race hides here and shows there.
    let run = Command::new(&node)
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
