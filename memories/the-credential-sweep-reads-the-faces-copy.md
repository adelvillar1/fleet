---
name: the-credential-sweep-reads-the-faces-copy
description: "feedback 2026-11-02 — the GUI server's no_payload_carries_a_credential test scans every served payload (app.js included) for DEEPSEEK_API_KEY/Authorization/Bearer, so user-facing copy must never name a header or credential word; the sweep red'd the headers row's own empty-state text"
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

The remainders plan's W4 (the account card's headers row) shipped with an
empty-state line reading "the request carries the key's **Authorization**
and nothing else" — and the full gate red'd
`gui::server::tests::no_payload_carries_a_credential` (src/gui/server.rs
~4557), the masking defense that scans every served payload for
`DEEPSEEK_API_KEY`, `Authorization`, and `Bearer`.

**Why:** the sweep is the load-bearing defense from the provider-session-
headers plan's Risks — a resolved secret must never ride a payload, and the
test cannot tell user-facing prose from a leaked value, so it refuses the
WORD wherever it appears. The face's own app.js is a served payload.

**How to apply:** when writing face copy (board-empty lines, derivations,
tooltips, aria-labels), describe credentials functionally — "the key's own
credential", "the key" — never by the header's name or the word
Authorization/Bearer. If a genuinely-needed claim cannot be phrased around
the word, the copy is the thing to change, not the sweep. Related:
[[session-parity-remainders-plan-drafted-2026-11-02]],
[[fleet-vs-opencode-missing-session-header]].
