---
name: turn-feedback-shipped-402d947
description: "2026-10-29 — the owner's zcode-parity ask (copy/like/dislike/fork/hook at the end of every turn) shipped as 402d947: verdicts in the sessions-feedback sidecar keyed by exchange, fork = prefix-copy with the frame rule shared byte-for-byte, hook = a seeded cron job; two Rust tests, a ten-check harness case, the plan at docs/plans/2026-10-29-turn-feedback.md"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-29 (fleet): the owner asked "add turn feedback like zcode has with
copy, like, dislike, fork and hook at the end of every turn"; the plan was
approved (the two questions — hook's meaning, thumbs' UX — went unanswered,
so the recommended readings were taken and recorded in the plan) and landed
same session as `402d947` (seven files, gate green on the staged tree:
fmt/clippy/release, 1,596 lib tests + the face harness at 284 checks across
30 cases, ledger clean, `git rev-parse <sha>^{tree}` == the gate's
write-tree hash; foreign hunks in app.js/bridge.rs blob-staged again).

The five controls ride every reply's `.msg-tools` footer, keyed by the
**exchange** the reply answered (`exchanges()` at row build — one vocabulary
with the chat review's count):

- **like/dislike** — freshest-wins verdict in `sessions-feedback.json` (the
  titles/archived sidecar pattern, `up|down|clear`, atomic rename) — never
  the session log, because `conversation()` rebuilds the model's context
  from it. `POST /api/feedback`, closed vocabulary, both state branches
  carry `feedback` so pressed states rehydrate; only the button the verdict
  belongs to flips (label → "liked"/"disliked" + accent — the visible
  trace).
- **fork** — `Bridge::fork`/`fork_at` on the blocking pool: cut before the
  (exchange+1)-th **non-frame** owner message else the whole log;
  `Session::create` + `record`, source's start line skipped,
  `forked_from` as one Value key on the fork's own start line (no record
  grammar), folder/goal/title copied beside as the new id's entries,
  in-flight guard the rewind/resume pair uses, `POST /api/session/fork`,
  the face opens the returned id via `openWork`. The frame rule is
  byte-for-byte the face's `systemFrame` — `is_system_frame` in bridge.rs
  must stay identical to `systemFrame` in app.js or the cut drifts from the
  count (the fork test pins the cut both frames imply).
- **hook** — the honest mapping: a drawer that seeds a **cron job** (the
  settings surface's existing `{"op":"cron"}` write, zero new Rust), task
  prefilled with the ask the turn answered (a job sees its task and
  nothing else), name `hook-<session-suffix>-<n>`, daily 09:00 default,
  owner arms it; honors the batch-only no-in-loop-hook rule. If zcode's
  hook means something else, the whole behavior is this one dialog.

Pins: `a_reply_verdict_lands_beside_the_log_and_reads_back` (stamp → state
read-back → 400 on a bad word → clear → gone), the fork test (cut with the
steer frame riding along, lineage key, source whole at seven records,
whole-log copy past the end, both refusals by name), and harness case
`TURN the turn controls grade, fork, and hook` (ten checks). Docs:
gui.md's controls sentence + writes table; rulings in
`docs/plans/2026-10-29-turn-feedback.md`.

**Follow-up the same day — the owner asked "did you relaunch the app?"**
because none of it was visible: the face serves `include_str!`-embedded
assets, so a commit is invisible until `cargo build --release` +
`launchctl kickstart -k gui/$(id -u)/com.fleet.gui`. The verification that
settles it is the SERVED asset, not the source:
`curl -s 127.0.0.1:3080/app.js | grep -c assistantFooter` (and the five
label strings). Check `/api/state` for `running`/`seat` before kicking so
a live run is not cut; the binary also carries whatever foreign WIP is in
the tree, which is how every restart on a co-edited checkout behaves. See
[[fleet-gui-is-a-launchagent]] and
[[verify-the-binary-is-newer-than-the-source]].

**Why:** the owner's verbs are a UI contract, but each needed an honest
fleet substrate — a verdict that could reach the model's context, a fork
that mutated the source, or a "hook" that invented an event runner would
each violate a standing ruling; the exchange key keeps feedback, fork, and
the header's count on one vocabulary.

**How to apply:** a new per-row control keys off a count the face already
derives (never a second key space); sidecars for anything owner-authored
that is not model-visible; when a frame-filter predicate exists in two
languages, pin both sides with a test that includes the frames. Named
out-of-scope: verdicts → calibration disposition (wrong substrate), a
summary tally, event-based hooks, bot-face controls. Related:
[[chat-review-shipped-f191516]],
[[a-reload-moves-the-total-unless-system-frames-are-excluded]],
[[gate-the-staged-tree-not-the-working-tree]].
