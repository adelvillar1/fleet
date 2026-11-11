---
name: a-preflight-that-needs-a-binary-lies-about-a-narrow-path
description: "feedback 2026-11-09 — the doctor's new disk check shelled out to `df` and the install tests (a deliberately narrow PATH) got \"disk FAIL\" through `fleet install`'s doctor gate; a preflight must read the OS directly (statvfs/libc), never depend on a binary a test or a dock-launched app may not have on PATH"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

2026-11-09: I added a `disk` check to `fleet doctor` to make the drive eating
observable. The first build's `free_kib` ran `Command::new("df")`. The full lib
suite went red — **41 failures**, all in `tests/install.rs`, all the same
sentence: `disk FAIL the volume's free space could not be read` — because
those tests run the install with a controlled PATH carrying only `/bin/sh`
(the deterministic missing-tool driver) and `fleet install`'s own gate is the
doctor's exit code.

**Why:** a check that depends on a foreign binary conflates "the machine is
broken" with "my PATH is narrow". The install tests exist precisely to prove a
narrow PATH still works; a preflight is the wrong place to demand more.

**How to apply:** a diagnostic or preflight reads the OS directly
(`libc::statvfs`, a syscall, a protocol) — never `df`/`du`/`python3` unless the
surface is a shell script whose PATH is its own contract (`release.sh`'s
preflight may use `df`, and does, because the script already requires cargo and
gh; the install path's gate may not). When a new check makes a suite red in a
target that deliberately starves PATH, the check is the defect, not the
fixture.

Related: [[build-lanes-in-tmp-eat-the-drive]].
