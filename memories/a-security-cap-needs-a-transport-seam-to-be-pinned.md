---
name: a-security-cap-needs-a-transport-seam-to-be-pinned
description: "Feedback 2026-10-31 — the reference fetch's redirect cap and byte cap could not be exercised against the real network (no public host produces a loop), so the exchange sits behind a SiteTransport trait and the tests script one; a cap you cannot provoke is a cap you have not pinned"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Feedback 2026-10-31 (T3, the reference-site fetch).** `fetch_site` refuses
loopback/private hosts, a redirect past three, and a body past 2 MiB. The
private-host refusals are trivially testable (the check runs before any socket);
the **caps are not** — no public host redirects forever and no fixture server is
reachable because the private-host check runs first.

The fix that made W9 honest: extract one `SiteTransport` trait
(`get(url, pinned_addrs) -> SiteResponse {status, location, content_type,
body}`), have the real ureq agent implement it with the resolved addresses
pinned via `.resolver(closure)`, and let the tests implement it with scripts —
a `Loop` transport that always answers 302 to itself, a `Huge` transport that
answers a body of `MAX_SITE_BYTES + 1`, a `Pdf` one for the content type. Now
`the_reference_fetch_caps_redirects_and_size` pins all three by name, and
`the_reference_fetch_refuses_the_private_and_the_malformed` pins the scheme,
credentials, ten private host shapes and a loopback fetch that never opens a
socket.

**How to apply:** a bound that exists for security must be pinnable without the
network — put the exchange behind one trait, keep the policy (validation, caps,
pinning) in the pure layer above it, and script the fake. The same shape served
the codegen (`to_tsx` is pure) and the packet (`frames_packet` takes frames,
not a home).

Related: [[a-fixture-can-pin-the-bytes-and-encode-the-wrong-shape]],
[[a-page-set-report-needs-the-union-not-the-intersection]].
