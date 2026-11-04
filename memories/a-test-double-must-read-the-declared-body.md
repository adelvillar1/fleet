---
name: a-test-double-must-read-the-declared-body
description: "feedback 2026-10-18/20 — a one-request mock server that replies after its first read closes the socket under a POST client still writing the body, so the client reports a header error that has nothing to do with the code under test; read the request to its Content-Length"
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

Third occurrence of one mechanism across two sessions. The second-brain plugin's
unit tests ([[fleet-second-brain-is-a-tool-plugin]]) failed on the two POST paths
— `a_refresh_posts_the_job_and_the_tools_guard_header` and
`a_busy_tool_names_the_job_it_is_already_running` — with
`Network Error: Error encountered in a header: Invalid argument (os error 22)`.
The code under test was correct. The mock did one `read()` into a 4096-byte
buffer, took whatever arrived, and answered immediately; the client was still
writing headers/body, so the reply landed on a half-sent request.

**Why:** the same mechanism already appeared twice the same week (see
[[a-test-that-passes-on-an-idle-machine-has-not-been-tested]]): doubles that
framed a request by where the buffer ended instead of its declared length. It
is intermittent-looking (it depends on write/read interleaving), it surfaces as
a client-side networking error rather than a test error, and the natural instinct
— blame ureq, retry, add a sleep — is wrong every time.

**How to apply:** a hand-rolled HTTP double must (1) loop reads until the
`\r\n\r\n` head terminator, (2) parse `Content-Length`, (3) keep reading until
the body reaches that length, then answer — never answer on a fixed buffer or a
single read. Same rule inside `fetch` stubs in Node (set `Content-Length`
yourself; see [[gui-client-verified-by-a-vm-harness]]). When a test against a
local double fails with a header/transport error while the logic looks right,
read the double first.
