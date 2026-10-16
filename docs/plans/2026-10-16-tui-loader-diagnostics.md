---
status: active
created: 2026-10-16
updated: 2026-10-16
slug: tui-loader-diagnostics
---

# Plan: loader diagnostics off the TUI pane

## Context

KI-216, hit at the owner's first launch after the canonical-skills
round: the skills loader's skip diagnostics are bare `eprintln` lines,
and against the 444-file canonical library five of them fired at TUI
start — painting raw over the welcome pane, interleaved and clipped
(the screenshot is the evidence). The soft-cap growth line
(`agent.rs`'s `skills_prompt_growth_line`) is the same class: bare
stderr, per run, over the pane. The loader was written when fleet's own
skills home held a handful of files on a headless path; the library
made session-start diagnostics a TUI-surface problem.

## Approach

The loader **reports** instead of printing, and each surface decides
where its diagnostics go:

1. `skills::load_reported(canonical)` returns `(Vec<Skill>,
   Vec<SkillSkip>)` — the skip's name and reason, collected. `load`
   wraps it and keeps today's stderr behavior: the CLI surfaces
   (`fleet skills`, headless run/chat, cron) are unchanged.
2. The TUI's load site uses `load_reported` and, when skips exist,
   sends one `UiEvent::SkillsLoaded { loaded, skipped }` — rendered as
   a card in the conversation (the LaneJoined family: a fact about the
   session, its own block, wrapped at the pane width). A clean load
   sends nothing; the welcome banner stays alone.
3. The soft-cap growth line goes quiet when a live surface is listening
   (`options.ui_tx.is_some()`): the startup card already carries the
   count once, and a per-run stderr line over the pane is noise. The
   headless path keeps the line.

## Acceptance criteria

- [ ] D1: `load_reported` returns every skip with its reason; `load`
      still prints each (the CLI surfaces unchanged).
- [ ] D2: the TUI renders one wrapped card naming the loaded count and
      the skipped skills — never raw stderr over the pane.
- [ ] D3: the growth line is stderr-only on the headless path.
- [ ] D4: the gate runs whole, exit codes read directly.
- [ ] D5: KI-216 is in the ledger with the launch evidence.

## Files

`src/skills.rs`, `src/agent.rs`, `src/tui/mod.rs`, `src/tui/app.rs`,
`src/tui/ui.rs`, `docs/known-issues.md`, this plan.

## Verification

Unit tests: `load_reported`'s skips; the app's card render; the growth
line's gate on `ui_tx`. Gate per D4.
