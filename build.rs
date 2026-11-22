//! One build stamp, and the compressed twin of every text asset the faces
//! serve.
//!
//! ## The stamp
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
//!
//! The stamp covers the **compressed** bytes too, not only the source
//! files: a gzip stream is what the wire carries, and a build that shipped
//! a different compressed body under an unchanged coordinate would serve a
//! returning client yesterday's bytes at an `immutable` address. The four
//! assets that embed the stamp themselves (`index.html`, `sw.js`,
//! `bot.html`, `bot-sw.js`) are hashed by their source bytes alone — their
//! compressed form contains the stamp, so hashing it would be circular —
//! but those source bytes determine both, so the coordinate still moves
//! with them.
//!
//! ## The compression
//!
//! The face answered 1.89 MB of identity-encoded text on a cold load
//! (`docs/plans/2026-11-20-asset-compression.md`). Every text asset is
//! compressed here, once, into `OUT_DIR`, and `src/gui/server.rs` embeds
//! the result beside the raw copy: `flate2` is a **build** dependency, so
//! the served path costs one comparison and no encoder. The raw copy stays
//! because the identity path is the one the harness and the no-credential
//! test read, and a client that cannot decode gzip must get the bytes it
//! has always got.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// How an asset's served bytes differ from its bytes on disk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stamp {
    /// Text, served exactly as written.
    None,
    /// `__FLEET_ASSET_HASH__` stands in for the build's address.
    Marker,
    /// The page asks for its two script assets at the build's address.
    Page,
    /// Binary (an icon): served as written, and never compressed — gzip
    /// would grow a PNG.
    Binary,
}

/// One asset the faces serve, and what the server does to it on the way
/// out.
struct Asset {
    path: &'static str,
    stamp: Stamp,
}

/// Every asset the faces serve — the console's, the bot's, the vendored
/// renderers', and the install surface.
const ASSETS: &[Asset] = &[
    Asset {
        path: "src/gui/assets/index.html",
        stamp: Stamp::Page,
    },
    Asset {
        path: "src/gui/assets/app.css",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/app.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/core/store.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/core/router.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/core/dom.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/purify.min.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/pdf.min.mjs",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/pdf.worker.min.mjs",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/docx-preview.min.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/jszip.min.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/vendor/VENDOR.json",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/manifest.webmanifest",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/sw.js",
        stamp: Stamp::Marker,
    },
    Asset {
        path: "src/gui/assets/icon-192.png",
        stamp: Stamp::Binary,
    },
    Asset {
        path: "src/gui/assets/icon-512.png",
        stamp: Stamp::Binary,
    },
    // The bot shell (the 2026-10-20 plan's B1): a second app over the
    // same bridge, stamped by the same hash so both faces' coordinates
    // move together.
    Asset {
        path: "src/gui/assets/bot.html",
        stamp: Stamp::Marker,
    },
    Asset {
        path: "src/gui/assets/bot.css",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/bot.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/bot.webmanifest",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/bot-sw.js",
        stamp: Stamp::Marker,
    },
    Asset {
        path: "src/gui/assets/bot-icon-192.png",
        stamp: Stamp::Binary,
    },
    Asset {
        path: "src/gui/assets/bot-icon-512.png",
        stamp: Stamp::Binary,
    },
    // The mini console (the 2026-11-21 plan, phase 2): a third face over
    // the same bridge, its page and its worker carrying the same stamp, so
    // its `?v=` coordinate and its cache name move with every build.
    Asset {
        path: "src/gui/assets/console.html",
        stamp: Stamp::Marker,
    },
    Asset {
        path: "src/gui/assets/console.css",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/console.js",
        stamp: Stamp::None,
    },
    Asset {
        path: "src/gui/assets/console-sw.js",
        stamp: Stamp::Marker,
    },
];

/// The stamp's own arithmetic: FNV-1a over every byte fed to it.
fn feed(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The name an asset's derived files take in `OUT_DIR`: the tree's own
/// path, flattened, so `core/store.js` and `app.js` cannot collide.
fn flat(path: &str) -> String {
    path.strip_prefix("src/gui/assets/")
        .unwrap_or(path)
        .replace('/', "-")
}

/// One gzip stream, at the best ratio this build machine will pay for at
/// build time. `flate2` writes a fixed header (no mtime), so the same
/// input yields the same bytes on every build — which is what makes the
/// stamp's coverage of them meaningful.
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(bytes)
        .expect("the build's compressor accepts every asset");
    encoder.finish().expect("the build's compressor finishes")
}

/// The bytes the server will serve for an asset that carries the stamp.
fn stamp_into(text: &str, stamp: &str, how: Stamp) -> String {
    match how {
        Stamp::Marker => text.replace("__FLEET_ASSET_HASH__", stamp),
        Stamp::Page => text
            .replace("href=\"/app.css\"", &format!("href=\"/app.css?v={stamp}\""))
            .replace("src=\"/app.js\"", &format!("src=\"/app.js?v={stamp}\"")),
        Stamp::None | Stamp::Binary => text.to_string(),
    }
}

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let mut read: Vec<(&'static str, Vec<u8>, Stamp)> = Vec::with_capacity(ASSETS.len());
    for asset in ASSETS {
        println!("cargo:rerun-if-changed={}", asset.path);
        let bytes = fs::read(asset.path).unwrap_or_else(|error| {
            panic!("the build stamp could not read {}: {error}", asset.path);
        });
        read.push((asset.path, bytes, asset.stamp));
    }
    // The sources first, then the compressed twins: two passes so the
    // raw-only value is a number the tests can hold the build to
    // (`FLEET_ASSET_STAMP_RAW`). If a build ever stopped feeding the gzip
    // streams in, the two values would be equal and
    // `the_stamp_covers_the_compressed_bytes` reds.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, bytes, _) in &read {
        // The name is hashed too, so renaming an asset without changing
        // its bytes still moves the stamp.
        hash = feed(hash, path.as_bytes());
        hash = feed(hash, bytes);
    }
    let raw_stamp = format!("{hash:016x}");
    for (path, bytes, how) in &read {
        if *how != Stamp::None {
            continue;
        }
        // The compressed form of an asset whose served bytes are its
        // file's bytes: written here, and part of the coordinate.
        let gz = gzip(bytes);
        hash = feed(hash, &gz);
        let name = format!("{}.gz", flat(path));
        fs::write(out.join(&name), &gz)
            .unwrap_or_else(|error| panic!("the build could not write {name}: {error}"));
    }
    let stamp = format!("{hash:016x}");
    // The assets whose served bytes carry the stamp. Their compressed
    // form is compressed *after* the stamp is in it, so the wire carries
    // exactly the bytes the page is told to ask for.
    for (path, bytes, how) in &read {
        if !matches!(how, Stamp::Marker | Stamp::Page) {
            continue;
        }
        let text = String::from_utf8(bytes.clone()).unwrap_or_else(|error| {
            panic!("{path} is not text but the server stamps it: {error}");
        });
        let served = stamp_into(&text, &stamp, *how);
        let name = flat(path);
        let stamped = format!("{name}.stamped");
        fs::write(out.join(&stamped), served.as_bytes())
            .unwrap_or_else(|error| panic!("the build could not write {stamped}: {error}"));
        let gz = gzip(served.as_bytes());
        let compressed = format!("{name}.stamped.gz");
        fs::write(out.join(&compressed), &gz)
            .unwrap_or_else(|error| panic!("the build could not write {compressed}: {error}"));
    }
    println!("cargo:rustc-env=FLEET_ASSET_HASH={stamp}");
    println!("cargo:rustc-env=FLEET_ASSET_STAMP_RAW={raw_stamp}");
}
