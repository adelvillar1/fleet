---
name: gallery-shots-ride-a-scratch-home
description: "project 2026-11-02 — the design gallery and every doc screenshot are shot from a scratch FLEET_HOME on port 3099 (never the owner's canvas), one fresh page per state, committed as docs/images"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

For README / feature-doc screenshots, `fleet gui` runs against a
**copy** of the home, never the real one:

- home `/tmp/fleet-gallery-home` — `config.json` copied from `~/.fleet`,
  a throwaway `.env` with placeholder `OLLAMA_API_KEY` /
  `ZAI_CODING_API_KEY` (availability only, so the model chips render
  pressable), and seeded `canvas/<slug>/project.json` boards (a bound
  `harbor-site` with two frames in set `site` + a review, a bound
  `fleet-board`, an unbound `scratch-board`). Fixture versions need the
  `html` field or `projects()` skips the board (`.ok()?`).
- face: `FLEET_HOME=/tmp/fleet-gallery-home ./target/release/fleet gui
  --port 3099 --no-open` (kill the pid on 3099 first).
- shoot: `tmp/design-gallery-shoot.js`, playwright-core from
  `~/.hermes/hermes-agent/node_modules`, chromium-1234 headless shell,
  1440×900, jpeg q82, **one fresh page per state** (goto → click →
  shoot) — see [[a-visual-coherence-shoot-rides-a-scratch-home]]'s parent
  lesson in [[a-visual-coherence-check-is-reading-the-png]] for why.

Shipped 2026-11-02 in `572852b`: six images (`surface-design`,
`design-focus-page`, `design-review`, `design-export`, `design-fanout`,
`design-site`) in README's gallery table + `docs/features/design.md`'s
"In pictures" (paths there are `../images/…`).

The same scratch home proves **interactive** flows, not only pictures
(2026-11-02, the settings U9 probe): seed `routing.json` with a walk
(`{"workloads":[{"workload":"standard_code","candidates":[...]}]}`),
drive the card in Playwright, and read the **file** back — stop and add
both moved the real chain, which is the proof a screenshot cannot give.
`tmp/u9-probe.js` is that pattern. A visible-card check on a long
settings page must use `innerText` (or Playwright `hasText`): a
container's `textContent` concatenated every hidden section and read
false.

**How to apply:** the assets are `include_str!`'d, so any face change
needs `cargo build --release` before the scratch face serves it; a
`cargo` not on PATH here lives at
`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`. Kill the
scratch face when the shoot is done. The real console is the supervised
launchd one on :3080 rebuilt from `~/Projects/fleet-master-wt`.

**2026-11-02 — the PATH line still resolves, but the toolchain it names
moved.** `rustup update stable` took that directory from 1.89.0 to 1.99.1,
so any command run through it now compiles under a clippy that fails with
41 errors. A scratch-home shoot is unaffected (it serves a built binary),
but a `cargo build --release` before the shoot now needs the 41 fixed
first — see [[rust-toolchain-moved-to-199-and-clippy-red]].
