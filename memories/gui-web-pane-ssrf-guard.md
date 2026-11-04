---
name: gui-web-pane-ssrf-guard
description: "2026-10-17 — the GUI's right-hand pane reads the web (third tab, 452dd60): a server-side fetch reduced to prose, because an iframe cannot frame anything and a fetch is an SSRF vector — so the guard checks the scheme and the RESOLVED addresses on every hop"
type: project
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

Owner directive 2026-10-17: "on the right side collapsible pane we also need to
allow browsing of web pages." Shipped as **`452dd60`** (`src/web.rs`, plus
`/api/web` and the pane's third tab).

**Server-side fetch, NOT an iframe — two independent reasons, and both hold.**
An iframe simply does not work: nearly every site worth reading sends
`X-Frame-Options: DENY` or a `frame-ancestors` CSP directive, so the pane would
show a blank rectangle for exactly the pages the owner wants. And a
server-side fetch is an **SSRF vector**, which is the reason the guard is the
module rather than a footnote to it.

**The guard is the load-bearing part.** A browser that can name any URL is a
browser that can name `http://127.0.0.1:8901` (the embedding model),
`http://127.0.0.1:8420` (the blend1 lane), and `http://169.254.169.254` — the
cloud metadata service, which hands out credentials to anything that asks. So
**every hop — the first and every redirect — is checked** against the scheme
and against the addresses the host **resolves to**. Refused by name: loopback,
private (10/8, 172.16/12, 192.168/16), link-local, unspecified, broadcast,
`169.254.169.254`, and the v6 equivalents (unique-local fc00::/7, link-local
fe80::/10). A **v4-mapped** v6 address is checked as v4, and a **bracketed
IPv6 literal** is split on the bracket rather than the colon — both were live
holes the test caught (`http://[::1]:8420/` sailed through the first version
with its port attached). The check is on the resolved addresses, not the name,
so a host pointing at 127.0.0.1 is refused the same as the literal. Redirects
are followed by the module itself (ureq's are disabled) precisely so each hop
is re-validated, capped at 5.

Verified live: all five probe vectors refuse with the reason named, and a real
public page reads.

**What renders is text only** — title, headings, paragraphs, list items, code,
quotes — plus the links listed beside the prose. No script from the page runs,
no external resource loads, nothing the page says about itself is trusted. A
link's **label stays in its sentence** rather than being torn out of it: a
reader mode that leaves "the first paragraph, with an in it" is worse than one
that keeps the prose whole and lists the links after it. The extractor is a
tag-and-text scanner, not a parser, and it says so — a page whose markup
defeats it renders as its visible text rather than failing, which is the
honest degradation.

**Two bugs the tests caught, both the same class** — a state machine that
leaks because a close path does not actually close the state:
1. The entity decoder stopped *on* the semicolon, so `&amp;` became `&;`.
2. `mem::take` on the code block left it `Some("")`, so the block never
   closed and everything after it — footer, aside, the rest of the page —
   landed inside the code.

**Why:** the pane's other two tabs already let the browser ask for a file and
for a deliverable; the web is the third thing an owner wants beside a run, and
the only way to give it without either a blank frame or a proxy into fleet's
own loopback services.

**How to apply:** when a surface gains the ability to name a remote resource,
the address guard is the first thing built, not the last. Check the *resolved*
addresses, re-check on every redirect, and refuse by name so the pane shows a
reason rather than a blank. Distinct from [[fleet-browser-lane-architecture]],
which is fleet *driving* a browser (navigate/screenshot/read over CDP) — this
is fleet *reading* a page's prose with no browser at all. Related:
[[gui-revamp-browser-face-decision]] (the pane and its file tab),
[[gui-client-verified-by-a-vm-harness]] (how the third tab was verified).
