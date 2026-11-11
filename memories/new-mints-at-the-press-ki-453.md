---
name: new-mints-at-the-press-ki-453
description: "owner ruling 2026-11-05 — New mints a real session at the press, never a deferred intent; a message can only land in the conversation it was typed for; KI-453's three doors fixed at fff6dc3 (detachToWelcome, {text,session} queue, no bare /api/run), gate 2,077/0, harness 941/105, delivered to 3080, dmg rebuilt"
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

Owner ruling 2026-11-05, after reporting the regression twice in one
shape ("when I click on creating a new session it just appends to the
last one active" → "clicking on the + icon on sessions should straight
away create a new session, never go back to an older active session"):
**the + creates the conversation immediately** — N2's create-then-send
intent is dead; an abandoned compose leaves the shell, and the intent
fallback survives only for a refused creation.

**Why:** the regression was three doors open at once — (1) the detach
left `face.running`/`face.driving`/`face.seat` and the old transcript
standing, so the next send queued behind the abandoned run's state and
could strand there forever (the CDP repro: the message sat in the queue
chip with no `/api/run` ever firing); (2) the queue was **sessionless** —
its own drain comment named the design ("the send names this session"),
so the message landed in whatever conversation was showing when the old
run ended; (3) a null-session send fell through to a bare `/api/run`,
which the bridge resolves to the thread's remembered session — the old
active one.

**How to apply (landed fff6dc3, pushed to face-plugin-host AND master):**
- `newConversation()` mints via `POST /api/session/new` on the press;
  `detachToWelcome()` clears the abandoned conversation's run state and
  transcript first.
- Every queue item carries `{text, session}` and `flushQueue` dispatches
  with `send(item.session)` — a message can only land where it was
  written. (The bug's sharpest form: `send()` took `queuedSession` but
  its final call dropped the argument, so the flush fell back to
  `face.session` — pass it through.)
- Every run names a conversation: a null target mints rather than going
  bare; a foreign `session` event is adopted only by a face with none;
  the create moves the launch folder and the pending goal (C6's attach
  now lands at the press).
- The three N2-era harness tests were re-pointed to the ruling, three
  REG/NEW pins added: harness **941 checks / 105 cases**, gate
  **2,077 / 0**; KI-453 carries the ledger record; spec + gui.md sentences
  name the timing.

Debugging trap worth its own line: the harness's `flush(n)` drains only
~n **microtasks, not timers** — a chain that stalls past the tick budget
reads as "no request fired" while the probes print; and app-side
`window.FACE_DEBUG` probes survive only if the flag is set on THAT
test's page, so a probe visible in one case tells you nothing about the
next.

**The co-tenant's stale buffer can silently eat one edit of a batch**
(hit while landing this fix): a parallel session shares the checkout, and
a `paintQueue` rewrite was wiped mid-batch while its siblings survived —
the harness failures (`[object Object]` rows) were the tell, and the fix
was to re-apply the lost hunk. **Re-read the file after any multi-edit
batch in a shared tree**, and treat a suspicious harness failure as a
possible lost edit before theorizing about the code. Related:
[[a-parallel-session-can-reintroduce-your-fixed-lints]],
[[a-parallel-session-co-edits-your-file]].

See also [[a-parallel-session-can-reintroduce-your-fixed-lints]] (the
co-tenant's stale-buffer write wiped ONE mid-batch edit and the
harness failures were the tell), [[rebuild-is-not-commit-push-or-ship]]
(the delivery legs all closed this time), [[a-deferred-action-is-an-error-to-the-owner]].
