---
name: shell-stub-traps-echo-and-controlled-path
description: "feedback 2026-10-26 — stub scripts on a controlled PATH fail three quiet ways: /bin/sh echo interprets `\n` and splits a JSON answer across lines (use printf '%s\n'), bare mkdir/dirname don't resolve on the controlled PATH (absolute /bin/mkdir, /usr/bin/dirname), and `git clone --quiet <url> <dir>` puts the target in $4 not $3"
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

Three quiet stub failures from the LayerFS C1 install-build test
(`the_install_builds_the_sealed_bundle_when_none_is_discovered`), each
producing a confusing downstream symptom instead of a stub error:

**`/bin/sh` echo interprets `\n`.** A stub answering JSON with an embedded
escape — `echo '{"kind":"ok","text":"base\n"}'` — prints a REAL newline,
splitting the answer across lines; the driver's first-line JSON parse then
fails (or worse, silently mis-reads). Use `printf '%s\n' '<json>'`, which
keeps the two-character escape intact.

**A controlled PATH has no mkdir, no dirname.** The stub world replaces
PATH entirely, so any bare helper the stub calls is ENOENT — and the stub
exits 0 anyway, so the fixture is silently missing (a clone that created
no directory makes the next command's `current_dir` fail with an ENOENT
that reads as a product bug). Absolute helpers: `/bin/mkdir -p`,
`/usr/bin/dirname`. The house's `fake_tool_dir` comment already warned
this ("/bin/mkdir is absolute because a bare mkdir would not resolve on
the controlled PATH") — the pattern exists, it just wasn't applied.

**`git clone --quiet <url> <dir>` puts the target in `$4`.** A clone stub
keying on `$3` mkdirs the URL. Count the flags.

**A fourth, same day: a `.mjs` stub is ESM, not CommonJS.** The
history-script stub was written `.mjs` and used `require("node:fs")` —
`ReferenceError: require is not defined in ES module scope`. Use an
`import` line at the top. And a JSON answer printed as a bare object
literal (`console.log({...})`) comes out in Node's *inspect* format
(unquoted keys), not JSON — wrap it: `console.log(JSON.stringify(<obj>))`.
Both failed as "the answer was not JSON", i.e. the downstream parse
error named the wrong layer (the file also recurred: `echo` eating `\n`
re-hit in the C3 instrument stub until it switched to `printf '%s\n'`).

**How to apply:** when a stubbed-chain test fails, FIRST read the stub's
own log and list what the stub actually created — all three of these
leave the product code holding an empty bag while the stub looks like it
ran. And when writing a new stub, copy `fake_tool_dir`'s conventions
(absolute helpers, argv log) before inventing.

Related: [[rust-unicode-escapes-break-in-python-heredocs]] (the same
class: an escape-eating layer between you and the file),
[[a-fixture-can-pin-the-bytes-and-encode-the-wrong-shape]] (the stub's
ANSWER can also lie), [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]].
