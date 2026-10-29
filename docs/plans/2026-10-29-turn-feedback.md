# Turn feedback — copy · like · dislike · fork · hook

Owner ask (2026-10-29): "add turn feedback like zcode has with copy,
like, dislike, fork and hook at the end of every turn." The plan was
approved the same day; every increment below landed in the same
session.

## Rulings

1. **The key is the exchange.** Every reply's footer stamps the
   exchange number it answers — `exchanges()`, the count that excludes
   steer and scheduled frames — so live and reloaded views agree, and
   feedback, the fork's cut, and the header's count share one
   vocabulary.
2. **Verdicts land in `sessions-feedback.json`**, the titles/archived
   sidecar pattern (`up`/`down`, freshest wins, `clear` removes,
   atomic temp-and-rename). Never the session log: `conversation()`
   rebuilds the model's context from the log, and grading is
   bookkeeping about a reply, not a message in it. The state document
   reads the map back so a pressed state rehydrates on load.
3. **A fork copies a prefix.** Cut before the (exchange + 1)-th
   non-frame owner message, else the whole log; `Session::create` plus
   `record`, the source's own start line skipped, `forked_from`
   stamped on the fork's start line as one Value key (no record
   grammar change), folder/goal/title copied beside as the new id's
   entries. The frame rule is byte-for-byte the face's `systemFrame`
   — `is_system_frame` in `bridge.rs`, `systemFrame` in `app.js` —
   and the fork test pins the cut the exchange count promises.
4. **A hook seeds a cron job.** Fleet's only owner-armed automation
   seam: the dialog posts the settings surface's existing
   `{"op":"cron"}` write, the task is the ask the turn answered (a
   job sees its task and nothing else), and nothing runs in-loop —
   the batch-only rule the judge/delegate/explain modules state stands
   untouched. If zcode's hook means something else, the behavior
   lives in this one dialog and swaps there.
5. **Five text buttons in the reply's footer**, always visible (the
   no-hover ruling), wrapping on narrow faces; the owner's bubble
   keeps edit + copy — verdicts grade replies, not asks.

## Increments — all landed

- **I1 feedback substrate.** `FEEDBACK_FILE` + load/save beside the
  titles fns, `Bridge::set_feedback` (closed vocabulary
  `up|down|clear`), `feedback_for` in both state branches, and
  `POST /api/feedback`. Pinned by
  `a_reply_verdict_lands_beside_the_log_and_reads_back` (stamp → read
  back → out-of-vocabulary 400 → clear → gone).
- **I2 client controls.** `assistantFooter(getText, ex)` builds copy ·
  like · dislike · fork · hook for both the live close and the replay
  path; `stampFeedback` posts and repaints from the reply;
  `paintFeedback` flips only the button the verdict belongs to (one
  verdict per exchange, so like and dislike never both read pressed).
- **I3 fork.** `Bridge::fork` → `fork_at` on the blocking pool, the
  in-flight guard the rewind/resume pair uses, `POST
  /api/session/fork {exchange}`, and the face opens the returned id
  through `openWork`. Pinned by
  `a_fork_cuts_after_the_exchange_and_leaves_the_source_whole`
  (cut after the reply, the steer frame riding with its run, the
  lineage key, the source whole at seven records, the whole-log copy
  past the end, and both refusals by name).
- **I4 hook dialog.** Name (`hook-<session-suffix>-<exchange>`),
  schedule (daily 09:00 default), and the task prefilled from
  `ownerMessageFor(ex)`; `arm the job` posts the cron op and the
  notice names what was armed.
- **I5 pins and docs.** Harness case `TURN the turn controls grade,
  fork, and hook` (ten checks: five controls, the owner's bubble
  without verdicts, rehydration, the post and its repaint, the hook's
  seeded form and cron op, the CSS, the fork's post and navigation);
  `docs/features/gui.md`'s controls sentence and writes table.

## Out of scope (named, not forgotten)

- Verdicts feeding the calibration stream's disposition seam — wrong
  substrate today: that seam keys route decisions, not replies.
- A tally line in the session summary or the inspector.
- Event-based hooks (a real hook runner) — the owner rules if
  zcode's hook turns out to mean that.
- Bot face transcript controls; this cut is the console.
