//! One build stamp: an FNV-1a hash over every asset the faces serve.
//!
//! The assets are embedded with `include_str!`/`include_bytes!` and served
//! at fixed paths, so there is no bundler to content-address them. This
//! stamp is that address: the page asks for `app.css?v=<hash>` and
//! `app.js?v=<hash>`, those URLs are served immutable, and the shell
//! worker's cache name carries the same string. Change any served asset and
//! every one of those coordinates moves together.
//!
//! FNV-1a is used rather than a cryptographic digest because this is a
//! cache coordinate, not a signature: collisions would name two builds
//! alike, which no adversary can arrange without already writing to the
//! tree.

use std::fs;

const ASSETS: &[&str] = &[
    "src/gui/assets/index.html",
    "src/gui/assets/app.css",
    "src/gui/assets/app.js",
    "src/gui/assets/manifest.webmanifest",
    "src/gui/assets/sw.js",
    "src/gui/assets/icon-192.png",
    "src/gui/assets/icon-512.png",
    // The bot shell (the 2026-10-20 plan's B1): a second app over the
    // same bridge, stamped by the same hash so both faces' coordinates
    // move together.
    "src/gui/assets/bot.html",
    "src/gui/assets/bot.css",
    "src/gui/assets/bot.js",
    "src/gui/assets/bot.webmanifest",
    "src/gui/assets/bot-sw.js",
    "src/gui/assets/bot-icon-192.png",
    "src/gui/assets/bot-icon-512.png",
];

fn main() {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for path in ASSETS {
        println!("cargo:rerun-if-changed={path}");
        let bytes = fs::read(path).unwrap_or_else(|error| {
            panic!("the build stamp could not read {path}: {error}");
        });
        // The name is hashed too, so renaming an asset without changing
        // its bytes still moves the stamp.
        for byte in path.as_bytes().iter().chain(bytes.iter()) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    println!("cargo:rustc-env=FLEET_ASSET_HASH={hash:016x}");
}
