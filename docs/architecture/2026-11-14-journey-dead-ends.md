# The user-journey dead-end audit

Audit, 2026-11-14. The question: which user journeys leave the owner
**stuck** — a press that silently does nothing, a refusal with no way
forward, words destroyed with no recovery, a decision surface one lens
away from the work that caused it. The core run journeys were walked
end to end through the actual handlers (composer, run, seat, queue,
edit/rewind, plan mode, stop/steer, connection loss, threads/archive);
the work surfaces and edge journeys were spot-checked.

The shape of the findings: fleet's refusals are usually **named** (the
notice discipline holds), but the **recovery** is where journeys die —
words destroyed instead of restored, presses that don't survive a
network gap, and a decision surface that doesn't travel with the
composer that caused it.

## The dead ends, ranked by cost

### D1 — the goal auto-start refusal is swallowed (primary control, silent broken promise)
Setting a goal on an idle conversation hands the session its task
(bridge.rs:5541-5548). If the run is refused (capacity, the
full-access floor), the bridge answers **200 with a `refused` field**
(bridge.rs:5580-5585 → server.rs:1610-1612) — and the face reads only
`reply.status === 200` (app.js:10284-10297). The chip repaints as set,
no run ever starts, no notice. The owner believes work began.
*Fix: read `reply.body.refused` — notice + chip reset. One line.*

### D2 — the queue flush destroys words on a 409 (user data lost)
`flushQueue` shifts the head off the queue before sending
(app.js:3635-3641); on a 409 refusal the words are neither restored to
the composer nor re-queued — a notice appears only if the log gained
no events (app.js:9100-9107). The message is gone. Same loss shape in
`planSend`'s network catch (app.js:26042 — clears everything, paints
nothing). *Fix: on refusal, restore the words to the composer (the
needs_setup discipline, app.js:3081-3093).*

### D3 — a capacity-refused run strands the owner's words in the transcript
Send while the machine has no headroom: a polite "queued · reconsider"
card lands (good), but unlike `needs_setup`, the `queued` path never
restores the task text to the composer (app.js:3038-3049 vs the
guarded twin 3085-3089). The words exist only as a transcript row; the
retry is copy/retype by hand. *Fix: the queued card gains a "retry"
press that restores the words — or the composer keeps them.*

### D4 — the seat's decision buttons don't travel with the composer
A run started from the **desk or learning** surface (the borrowed
composer, app.js:11752-11770) raises a seat whose section lives only
in the chat column (index.html:454-468); `placeComposer` moves the
composer but not the seat (app.js:11764-11767), and those lenses hide
the chat panel (app.js:26327-26334). The pill says "waiting for you",
the tab cue counts it, the inbox row routes home — but the decision
buttons are one lens away from the composer that caused the seat (the
KI-379 invisible-seat shape, surviving at lens granularity).
*Fix: `placeComposer` carries the seat with the composer — the same
borrow it already does.*

### D5 — relayed faces never declare connection loss
A paired/relayed page's `/events` **poll** failing has an empty catch
(app.js:9763-9778) — no banner, no pill, no retry notice. The KI-380
contract (never swallowed) holds only for direct EventSource faces;
the phone learns its connection died by noticing nothing updates.
*Fix: the relay tick's catch calls `connectionDown` like the direct
path does.*

### D6 — presses that don't survive a network gap
`stopRun` while the server is momentarily unreachable: the control
file is never written, a notice says so — and the owner must press
again after recovery (the 5s re-arm re-reads state but does not replay
the stop). Same shape: `openWork`/`openRoom`/`archiveSession`'s empty
catches (app.js:14549, 13330, 11206) — a press during a gap does
nothing at all. *Fix: queue the control write behind the re-arm (the
reads already re-arm; the writes should too).*

### D7 — a queued message flushes into a session archived while queued
Archiving refuses only a *running* session (bridge.rs:5615-5626); a
queued message for a session archived mid-wait dispatches into it at
the flush — the reply lands inside a tucked-away thread, the chip
vanishes as "delivered", nothing navigates. *Fix: the flush checks the
archived map and re-queues with a named notice.*

### Smaller, named
- Empty Send with a queue on screen and another conversation driving:
  a silent no-op — disclosed on the chip, but the press gives no
  flicker (app.js:9999-10004).
- Desk/learning send while a run is in flight: silent return
  (`surfaceSend`, app.js:11788) — the borrowed composer's words sit
  with no feedback (the normal composer queues them; these don't).
- Edit-rewind over a dead network: words restored silently, no notice
  (app.js:8978-8981, 26077).
- Plan-mode Enter while a run/seat is in flight: silent return — plan
  mode never queues (app.js:26020).
- No seat timeout exists: an unattended seat pauses the run
  indefinitely (escapable — answer or stop — but unbounded).

## Checked and guarded (coverage)

Dispatch without folder (preempted + named 400); run-refused 409s
(log re-read then notice); failed runs (end card + notify-if-hidden);
max-turns (named; the main loop uncapped by design); stops (KI-495
guard, double-press guard, the seat resolved by name, the `stopping`
card); steers (bounded one turn, refusal notices, every control lands
as a log card); the seat's roads home from any lens (pill, tab cue,
inbox group, rail counts, needs-row auto-open, desktop notification);
answered-elsewhere closes the seat **and says who took it**; remember
hidden on never-remembered classes; rules written before the mode
gate; KI-380's zombie recovery re-raises seats; queue session guards
(a queued message cannot drain into another conversation); edit while
running becomes a queued row; rewind refusals named; plan refusals
named; plan review cards replay-stable; the skills gate's refusals
notice + re-arm; the update apply's refusals ride KI-483's pane
discipline; plugin promotion gated on the suite (pinned Rust tests);
the desk's brief form turns empty-desk words into the brief.

## The shape of the fix

Three clusters: **(1) restore-the-words** (D2, D3, the desk/learning
silent send, planSend) — on any refusal, the composer gets the words
back; **(2) travels-with-the-composer** (D4, D5) — the seat and the
connection state follow the surface the owner is on; **(3) survive
the gap** (D6, D5) — control writes and presses queued behind the
re-arm. D1 is a one-line read. All are face-side; no bridge or agent
changes.
