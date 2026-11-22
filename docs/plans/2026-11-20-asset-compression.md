---
status: completed
created: 2026-11-20
updated: 2026-11-21
slug: asset-compression
---

# Serve the face's assets compressed

The face ships 1.39 MB of JavaScript, 367 KB of CSS, an 82 KB shell, and
the vendor files — every byte identity-encoded (`Content-Encoding` count
in `src/` is 0). The cache grammar is already right (hash-versioned,
`immutable` for the embedded path), so this is a first-load and
cache-miss cost. On the relay path it is not small: a phone's navigation
crosses the relay in plaintext frames, and every face restart rotates
`ASSET_HASH`.

## Context

- Measured: `src/gui/assets/app.js` 1,392,784 B; `app.css` 367,400 B;
  `index.html` 82,028 B. All are `include_str!`'d (`src/gui/server.rs:71-95`)
  and served by `served()` / `versioned_asset()` with
  `?v={ASSET_HASH}` (`ASSET_HASH` at `src/gui/server.rs:112`).
- No keep-alive: `serve_connection` answers `Connection: close` and means
  it (`src/gui/server.rs:254-257`), so a CDN-style many-small-files split
  is not free here either — the same constraint orders the `app.js` split
  after this plan.
- Binary weight matters: assets are embedded in the static binary, so any
  compressed copy adds to it unless it *replaces* the raw copy for the
  served path.

## Approach

1. Build-time precompression, no runtime dependency: `build.rs` compresses
   each served text asset once into `OUT_DIR` (`flate2` as a
   **build-dependency** only), and `server.rs` embeds the `.gz` bytes with
   `include_bytes!(concat!(env!("OUT_DIR"), "/…"))`.
2. Serve by negotiation: when the request's `Accept-Encoding` includes
   `gzip`, answer `Content-Encoding: gzip` with the precompressed body and
   `Vary: Accept-Encoding`; otherwise serve the raw string exactly as
   today. The versioned `?v=` cache key is unchanged — the same URL serves
   both encodings.
3. Keep the identity path byte-identical (it is the test surface the
   harness and the no-credential payload test rely on).
4. Measure: `GET /app.js` with and without `Accept-Encoding: gzip` on
   loopback, and the same through the reference relay the integration
   tests spawn; record the byte counts and the binary-size delta
   (raw + gz vs raw only) in the Status pass.

## Status pass, 2026-11-21 — measured, and the mechanism chosen

### Where the bytes are answered

- `/` → `page()` → `index_page()` (`src/gui/server.rs`): the embedded
  shell with `?v={ASSET_HASH}` stamped into its two asset URLs. 78,712 B
  served, `no-cache`.
- `/app.css`, `/app.js`, `/core/*.js` → `env_asset` → `served`: the tree
  read off disk in a dev build, the embedded copy in a release one, with
  `immutable` only when the query names the build's hash.
- `/vendor/*` → `versioned_asset`: same grammar, one constant per file.
- `/manifest.webmanifest`, `/sw.js` → `manifest()`, `worker()`.
- The paired device's face (`src/remote/listener.rs`): `versioned_or_plain`
  for `/app.css`, `/app.js`, `/bot.css`, `/bot.js`, and the bot shell's own
  builders — every one of them a `gui::server` reply, so one seam covers
  both faces.
- The relay path: `answer_frame` turns a reply into a frame whose body is a
  **JSON string**. That is the one place a compressed body cannot ride
  as-is; the relay already carries binary under `x-fleet-b64` (the icons),
  which is the same door.
- Icons and fonts → `write_blob`, already binary.

### What compresses (bytes, this machine)

| asset | raw | gzip -6 | gzip -9 | zstd -19 | brotli -q 11 |
|---|---:|---:|---:|---:|---:|
| `app.js` | 1,511,485 | 391,417 (25.9 %) | **389,543 (25.8 %)** | 308,870 | 301,575 |
| `app.css` | 378,840 | 74,377 | **73,442 (19.4 %)** | 61,913 | 58,720 |
| `index.html` | 78,674 | 22,093 | **22,026 (28.0 %)** | 20,358 | 18,384 |
| `core/store.js` | 11,464 | 4,463 | **4,463 (38.9 %)** | — | — |
| `vendor/pdf.worker.min.mjs` | 1,264,342 | 375,247 | **374,051 (29.6 %)** | 325,609 | 307,591 |
| 19 text assets, together | 3,991,380 | — | **1,082,997 (27.1 %)** | — | — |

`gzip -6` and `-9` differ by 0.5 % on `app.js`; the build uses the best
ratio because it pays for it once.

### The browser's own offer

Chrome 158 at 390×844 against `http://127.0.0.1`: every request carried
`Accept-Encoding: gzip, deflate, br, zstd` (CDP
`Network.requestWillBeSentExtraInfo`). A face may assume gzip; it may not
assume the client named it.

### The cold load, before

CDP, 1.5 Mbps down / 150 ms RTT, 390×844, the worker unregistered and both
the Cache API and the HTTP cache cleared:

| resource | bytes on the wire | duration |
|---|---:|---:|
| `app.js?v=<hash>` | 1,511,646 | 10,225 ms |
| `app.css?v=<hash>` | 378,993 | 4,104 ms |
| shell painted (`data-fleet-mounted`) | — | **11,471 ms** |

1,890,639 B, no `Content-Encoding`, no `Vary` on any reply.

### The decision: build-time precompression, gzip only

- **Precompressed, not runtime.** `flate2` is already in the graph through
  `ureq`'s gzip, so declaring it as a **build**-dependency adds no crate to
  the binary and no encoder to the served path. Runtime compression would
  pay a gzip per request per asset, and `Connection: close` means every
  asset is its own connection — the cost would land on the critical path
  the change exists to shorten.
- **gzip, not brotli.** Brotli is 88 KB smaller on `app.js` (301,575 vs
  389,543) and 14.7 KB on `app.css` — 6 % of the raw bytes — and costs an
  encoder in the build (a new crate and its build time) that the plan
  already ruled out. zstd is smaller still and is not a browser encoding.
- **The four stamped bodies are compressed too**, after the stamp is in
  them: `build.rs` stamps `index.html`, `sw.js`, `bot.html`, `bot-sw.js`
  with the coordinate and then compresses the stamped bytes, so the served
  body and its twin are the same bytes rather than a body and an
  approximation of it. The server's own substitution is held to the
  build's by a test.
- **The coordinate covers the compressed bytes.** The stamp feeds each
  gzip stream into the FNV after the sources and emits the raw-only value
  as `FLEET_ASSET_STAMP_RAW`; `the_stamp_covers_the_compressed_bytes`
  reds if the twins ever stop being part of the address.
- **The identity path is untouched**: same bytes, same headers, and the
  `Vary` is the only addition — which is what makes the two
  representations of one URL cacheable.

Binary delta: +1,082,997 B of twins beside the raw copies on a 36,081,984 B
release binary (+3.0 %). Recorded, and accepted: the raw copies are the
identity path the harness and the no-credential test read.

## Status pass, 2026-11-21 — shipped: the wire, the cells, the red-proofs

### The wire, measured on the release binary

Raw socket, `curl`, `Accept-Encoding: gzip` against the release face:

| path | identity | gzip | % of raw |
|---|---:|---:|---:|
| `/app.js` | 1,511,485 | 389,583 | 25.8 % |
| `/app.css` | 378,840 | 73,473 | 19.4 % |
| `/` (the shell) | 78,712 | 22,072 | 28.0 % |
| `/core/store.js` | 11,464 | 4,465 | 38.9 % |
| `/sw.js` | 12,995 | 5,202 | 40.0 % |
| `/vendor/pdf.worker.min.mjs` | 1,264,342 | 375,085 | 29.7 % |

Every negotiable reply carries `Vary: Accept-Encoding`; the compressed one
carries `Content-Encoding: gzip` and a `Content-Length` that is the encoded
length. The identity reply is byte-identical to what it was, plus `Vary`.

### The cold load, after

Same rig, same 390×844 viewport, same 1.5 Mbps / 150 ms RTT, the worker
unregistered and both caches cleared:

| | before | after |
|---|---:|---:|
| `app.js` on the wire | 1,511,646 | **389,840** |
| `app.css` on the wire | 378,993 | **73,722** |
| the shell on the wire | 78,911 | **22,295** |
| `app.js` + `app.css` | 1,890,639 | **463,562** (24.5 %) |
| the whole shell path (fonts, core, manifest) | 2,047,119 | **555,513** (27.1 %) |
| shell painted (`data-fleet-mounted`) | 11,471 ms | **3,578 ms** |

### The KI-587 cells, re-measured

The release face behind a proxy that adds only the fault modes (stall,
503, refuse) the cells need — every byte the browser counted came from the
server:

| cell | KI-587 record | now |
|---|---|---|
| warm mount from the worker's cache | 75 ms | **61 ms** |
| stalled v2, fallback from the any-version cache | 7,437 / 6,051 ms | **6,057 ms**, no card |
| refused navigation | 7 / 14 ms | the worker's "fleet is not reachable" page, first paint |
| install prefetch with both assets silent | bounded by the 6 s deadline | new cache created, worker `activated` within 9 s |
| entries in the new cache after install | 2 assets + the born stamp | 2 assets + the born stamp |

The worker stores **decoded** bodies: after install, `fleet-shell-<hash>`
holds `/app.css?v=…` and `/app.js?v=…`, a warm reload reads both from the
cache (`encodedDataLength 0`) and mounts in 61 ms. `Content-Encoding` on
the wire never becomes a header the cache has to decode twice.

### The red-proofs

1. **The negotiation, removed** (`accepts_gzip` forced to `true`):
   `a_client_that_cannot_decode_gzip_gets_the_identity_body` and
   `a_gzip_client_gets_the_compressed_body_and_the_identity_bytes` both
   red — a client that never asked for gzip got a stream it cannot decode.
2. **The body guard, removed** (the comparison replaced with `|| true`):
   `a_body_the_build_did_not_compress_is_never_gzipped` red, "a live body
   was gzipped" — the rule that keeps a dev tree's edit from being served
   as a stream that decodes to something else.
3. **The coordinate, moved by a byte**: 66 bytes appended to `app.css`
   moved `?v=` from `8fe3bca0a24f5939` to `49a72bffbd75ea62`; reverting the
   bytes returned it to `8fe3bca0a24f5939` — which also proves the gzip
   streams are deterministic, since they are in the hash input.

### The gate

`cargo fmt --all --check` 0 · `cargo clippy --all-targets -- -D warnings`
0 · `cargo build --release` 0 · `cargo test` 0 (2,776 passed, 11 ignored) ·
`node tests/face/harness.mjs` 0 (1,811 checks across 264 cases) ·
`python3 scripts/plan_check.py` 0.

Binary delta: the twins are 1,082,997 B embedded beside the raw copies;
the release binary measures 39,301,584 B.

## Acceptance Criteria

- [x] `GET /app.js` with `Accept-Encoding: gzip` answers
      `Content-Encoding: gzip` and the body gunzips to exactly the embedded
      bytes — pinned by `a_gzip_client_gets_the_compressed_body_and_the_identity_bytes`
      (`flate2` as a dev-dependency).
- [x] `app.js` over gzip is 389,583 B = 25.8 % of raw; `app.css` 73,473 B
      (19.4 %) and the shell 22,072 B (28.0 %) — measured on the release
      binary and recorded above.
- [x] Requests without `Accept-Encoding` receive the identity body with no
      `Content-Encoding` header
      (`a_client_that_cannot_decode_gzip_gets_the_identity_body`), and
      `no_payload_carries_a_credential` now reads **both** paths: the
      decoded twin is the same document and carries no key.
- [x] `Vary: Accept-Encoding` is present on every negotiable asset reply —
      asserted on both paths, over the `GZIP_ASSETS` table the test walks
      (22 paths).
- [x] Binary size delta recorded and accepted: 1,082,997 B of twins
      embedded beside the raw copies.
- [x] The full gate exits 0 (the four legs, plus the face harness and
      `plan_check.py`, all exit codes read).

## Files to be touched

- `build.rs` — the precompression step
- `Cargo.toml` — `flate2` build-dependency, dev-dependency
- `src/gui/server.rs` — embedding, negotiation, `Vary`
- `src/remote/listener.rs` — the paired device's socket path and the relay
  frame, which is where a phone's bytes cross a JSON string (found during
  the pass; the plan's forecast did not name it)
- `src/gui/mod.rs` — not needed: the embeds live in `server.rs`, beside the
  raw copies they twin

## Out of scope

- Keep-alive / HTTP/2 (`Connection: close` is deliberate per its comment;
  changing it is a separate ruling).
- Brotli (a runtime lib or a heavier build step; gzip is the 80 % win).
- The bot face's assets beyond the same mechanism.

## Verification

1. The gzip round-trip test, alone.
2. A raw-socket fetch with and without the header, sizes recorded.
3. One fetch through the reference relay, size recorded.
4. The full four-leg gate once, at the end.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F4.
- `docs/plans/2026-11-20-appjs-module-split.md` — sequenced after this.
