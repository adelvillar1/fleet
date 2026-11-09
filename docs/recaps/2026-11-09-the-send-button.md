# Session Recap — 2026-11-09: the send button with a message queued

**Summary.** The owner reported that with a message queued the composer's send
button did nothing. It was two faults under one control, both silent, both
proven red before the fix. v0.6.7 carries the repair; the served face is on it.

## The two shapes

**1. The idle press.** With nothing typed and a message waiting, the press ran
`send()`, which — a queue present but the composer empty — took its queue
branch, pushed the empty string nowhere, and returned. No run, no queue change,
no line anywhere. A queue can outlive the run that made it (a state read that
says idle while the queue still waits), and the press was the only way to move
it.

**2. The dead Stop.** A face that attached to a running chat — a reload, a
conversation opened mid-run — learns `running` from the state read but has no
process card until a process event lands. `stopRun` opened with a guard that
returned whenever that card was absent or inactive, so the Stop the button
showed was a no-op under a live run.

Both are the same failure at the owner's hands: the control the screen offers
does nothing, and nothing says why.

## The fix

- The press, idle with nothing typed and a message waiting, sends the queue's
  head (`flushQueue()`); a run working in another conversation still keeps the
  queue waiting, which is what the chip already says.
- `stopRun` fires whenever the face believes a run is in flight; the local
  process card now only decides what the reply paints (`paintProcess` on the
  card, the disabled state and the notice otherwise).

## The red-proof

With the app.js change stashed, the harness failed four checks in the two new
cases — `KI-495 the idle Send dispatches the queued message` (no `/api/run`
posted at all), `the dispatched run carries the queued words` (no run),
`the queue is empty after the press` (still `queued · 1`), and
`KI-495 the press stops the run the face attached to` (no `/api/stop`). With
the fix: 1036 checks across 125 cases.

## The gate and the release

fmt, clippy, `build --release` and `test` all exit 0 (2067 lib tests, 0
failures). v0.6.7 was cut through `scripts/release.sh` — tag gated on a
detached worktree, assets uploaded, the feed read back anonymously (`the
published feed names 0.6.7; the cli row serves the raw binary, sha256
ad507e74be00`) — and then installed by the updater itself: `fleet update`
moved `~/.fleet/bin/fleet` from 0.6.6 to 0.6.7 and kept the old binary as
`fleet.prev`; the LaunchAgent restart put 0.6.7 on 3080.

## Files changed

- `src/gui/assets/app.js` — the click handler's queue branch and `stopRun`'s
  guard.
- `tests/face/harness.mjs` — `aQueuedMessageSendsFromTheIdleButton` and
  `aStopWorksWithoutAProcessCard`.
- `docs/known-issues.md` — KI-495, closed with both red-proofs.
- `CHANGELOG.md` — the 0.6.7 section.

## Open questions / next steps

- The desktop bundle is still 0.6.6; one press on its About pane moves it to
  0.6.7 (the channel is proven, so it is a button now).
- Not changed by design: while a run is in flight the button is Stop, and
  messages queue from the composer's own Enter. If the owner wants the button
  to queue instead of stopping there, that is a separate product call.
