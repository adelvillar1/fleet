---
status: draft
created: 2026-11-20
updated: 2026-11-20
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

## Acceptance Criteria

- [ ] `GET /app.js` with `Accept-Encoding: gzip` answers
      `Content-Encoding: gzip` and the body gunzips to exactly the embedded
      bytes — pinned by a test that decodes and compares (`flate2` as a
      dev-dependency).
- [ ] `app.js` over gzip is ≤ 30 % of raw (measured, recorded); `app.css`
      and the shell likewise measured.
- [ ] Requests without `Accept-Encoding` receive the identity body with
      no `Content-Encoding` header, and the existing
      `no_payload_carries_a_credential` test still passes over both paths.
- [ ] `Vary: Accept-Encoding` is present on every negotiable asset reply.
- [ ] Binary size delta recorded and accepted (gz copies add weight; the
      plan states the number rather than assuming it is fine).
- [ ] The full gate exits 0.

## Files to be touched

- `build.rs` — the precompression step
- `Cargo.toml` — `flate2` build-dependency, dev-dependency
- `src/gui/server.rs` — embedding, negotiation, `Vary`
- `src/gui/mod.rs` — asset table if the embeds live there

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
