---
name: harness-landscape-converged-spec-2026-10
description: "2026-10-16 deep research — the converged feature spec of modern agent harnesses (Claude Code, Codex, Cursor, Amp, Cline...), the distinctive mechanics worth stealing, and fleet's ranked gap list for the GUI"
type: reference
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

Deep research done 2026-10-16 across Claude Code (v2.1.301), OpenAI Codex CLI +
cloud, GitHub Copilot coding agent, Jules, Devin, Cursor 3, Windsurf→Devin
Desktop, Amp, Zed, Aider, Cline, OpenHands/Agent Canvas, Gemini CLI, Crush,
opencode, Qwen Code, Goose.

**The converged spec** (what every serious harness now ships): permission mode
ladder + persistent allowlist rules ("yes, and don't ask again"); plan mode
(read-only explore, editable plan, then execute); todo/task visualization;
per-turn checkpoints/rewind; diff review with per-hunk accept; context
management (compaction + visible context meter); subagents with
summary-only return; cost/token visibility; session resume/fork/permalink;
MCP as the tool standard + skills (SKILL.md) as procedural knowledge + hooks
as the deterministic lifecycle layer; scheduled tasks.

**Distinctive mechanics worth stealing**: context meter with per-category
breakdown (Claude Code /context, Cursor context ring); plan as an editable
virtual file then "Build" (Cursor); steer-without-interrupt — queue a message
or fire it at the next tool boundary (Cursor); shadow-git checkpoints with
3-way restore Files/Task/Both (Cline); threads as cross-surface URLs (Amp);
cost in minutes not tokens (Amp orbs); timer-based plan auto-approval for
async work (Jules); memory as a git repo with daily consolidation (Devin
"dreaming"); multi-model head-to-head on one task (Qwen Agent Arena);
token-efficient tool output — logs to disk, preview to the model (Cursor
browser tool); trusted-folder scan showing what config would load before
trusting it (Gemini CLI); tool-call budget per prompt (Cascade's 40).

**Fleet's strengths nobody else has**: in-process router with per-workload
preference table + route-decision cache + calibration stream (no peer —
Amp's Dial is closest); delegation with dependency/unlock conditions
(graph.rs — peers have flat subagents); markdown-fact memory substrate with
recall counts; 441-skill library; three telemetry boards; Rust single binary
(only Codex CLI and Goose are Rust peers).

**Fleet's ranked gaps** (all interaction-layer): 1) no checkpoints/rewind of
file state anywhere; 2) no context meter (fleet knows exactly what it loads:
memories, skills, MCP tools); 3) no formal plan mode (TUI modes are Chat,
Sessions, Approval, Lanes, Explain, Settings — no Plan, no Diff); 4) no
persistent approval allowlist (ask/yes/yes-guarded only); 5) no diff-review
surface; 6) spend not visible per session (calibration stream exists, not
surfaced); 7) no steer-without-interrupt; 8) no session permalinks/fork.

**Why:** these gaps are the GUI's to close — the loopback web app
([[gui-revamp-browser-face-decision]]) has room for a checkpoints panel,
context meter in the right rail, an editable plan surface, and a diff view
that a 42-column terminal pane cannot host.

**How to apply:** when implementing the GUI, adopt in this order — context
meter, diff review, checkpoints, plan mode, allowlist rules, spend
visibility. Skip for a solo owner: agent teams, mobile/remote surfaces
(loopback-only ruling), plugin marketplaces, ACP client support. Do not
rebuild what fleet already does better than the field.
