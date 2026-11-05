---
name: bot-pwa-implemented-2976098
description: "2026-10-28 — the bot-PWA plan IMPLEMENTED end to end and committed as 2976098 (P1 role files, P2 session personas+goal+join records, P3 write_file in the destructive class, P4 archived arm, B1–B5 the bot shell on the remote face); gate green by exit code (fmt/clippy/build/1,769 tests), plan status completed"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28, owner: **"implement plan as designed."** The whole plan
([[bot-pwa-plan-rewritten-2026-10-28]]) landed in one session and was
committed as **`2976098`** (38 files, explicit paths). This supersedes
the "not built" state in [[bot-pwa-b2-b3-landed-2026-10-26]] and closes
the open dependencies in [[bot-pwa-is-assistant-centric-not-work-centric]]
(a session now carries personas) and
[[bot-pwa-persona-roster-six]] (the role files exist).

**What landed, by increment:**

- **P1** — six role files in the repo at `roles/*.md` AND installed to
  `~/.fleet/roles/` (so `fleet roles` renders them). A test loads them
  through fleet's own grammar and asserts every `skills:` name resolves
  in `~/.agents/skills` — a typo cannot ship. auditor admits no write
  tool; the read-only contract lives in the file. tower's first draft
  said `edit_file`, which is only an ALIAS — role files must carry
  canonical tool names (`edit`, `grep`, `glob`).
- **P2** — `Session::create_with_persona` writes the persona set on the
  start record; `Record::Join { persona, role, at }` / `Leave` are
  bookkeeping records (replay to nothing; the bridge's replay surfaces a
  join as a card, per the ruling that a join is a row the owner reads).
  `summarize` folds start + joins − leaves into the summary's
  `personas: Vec<String>`. `Bridge::new_session_with(persona, goal)`
  refuses a persona that does not resolve and writes the goal to the
  goals sidecar; `Bridge::join`/`leave` refuse already-here and
  lead-leaves-with-others. `sessions_read` rows carry `personas` + `goal`.
- **P3** — `write_file` JOINED the destructive class
  (`Gate::is_destructive`): its ordinary work is an overwrite that
  discards what is there, held every time with no always-allow. The
  older pin ("only bash has reach") was superseded by the owner's dated
  R5 ruling — see
  [[a-dated-ruling-supersedes-a-pinned-tests-reason]]. The seat document
  carries `destructive` as data on BOTH delivery paths (`describe_seat`
  for the event, the state document's `approval`), and the bot client
  posts `mode: guarded` so non-destructive work never asks.
- **P4** — `inbox_document` gives `archived` its own word (the 2026-10-27
  drift closed) and every row carries personas + goal.
- **B1–B5** — the remote root serves the bot shell (`bot.html/css/js`,
  its own `bot.webmanifest`, `bot-sw.js`, generated serif-f icon pair on
  the accent gradient); `build.rs` stamps the bot assets; the remote
  allowlist carries the bot set and DROPPED `/app.css`, `/app.js`,
  `/manifest.webmanifest`, `/sw.js`, `/icon-*` — the phone gets the bot,
  the desk keeps the console. The worker caches exactly `/bot.css?v=`
  and `/bot.js?v=` and serves its own retry page for a failed
  navigation. Remote verbs: `/api/session/{new,join,leave}`.
- **B6** — `docs/features/bot-app.md` (new), gui.md's endpoints table
  (join/leave + the two-shells note), remote-access.md, surfaces.md;
  KI-340 logged; plan status → `completed`.

**Verification:** fmt 0, clippy 0, release build 0, 1,769 tests / 0
failed — each leg's exit code read directly. KI-340 logs the one
loaded-suite flake (green alone and on re-run).

**The named questions were addressed the same day in `9b4ef7b`** — see
[[bot-pwa-followups-9b4ef7b]]: per-turn authorship landed (the agent
stamps every Assistant record with the lead persona; the replay card and
the bot's byline read the record), the goal change is the owner's
explicit `/api/goal` verb (admitted on the remote face; the bot's goal
line is tappable), and the backfill + shared-filing questions resolved
as the implemented behavior (unassigned; filed under the lead). The D11
approval line was superseded by the owner's direct
"implement plan as designed".

**Then the owner ordered the dogfood** — "we have not used persona (bot)
mode because you just built it" — and using it found four defects the
gate could not (the mode word, the remote run policy, the catch-up
restart storm KI-350, the persona's voice). All fixed and verified live;
see [[bot-pwa-dogfooded-live-01a9ab3]].

Related: [[one-goal-many-personas-destructive-is-the-only-gate]],
[[bot-pwa-b2-b3-landed-2026-10-26]].
