# The session archive becomes workable — recall, view, resume

Status: complete — C1–C6 and C8 delivered at df95b06: `fleet sessions --recall` ranks the archive through the six channels (live proof: a session's own vocabulary ranks it first at 0.64, trajectory 0.86), the second embedder-keyed index at `sessions/.recall-index.json`, `fleet sessions` and `fleet sessions <id>` both render cards, and provenance reads the outcome grade. The one leg deferred, by the module's own record: C7's feedback loop — bumping a recalled session's recall-count is a later piece, because a synthetic fact's path is the session log and the corpus's own bump must never be pointed at it (`src/session_recall.rs`'s "What this module is not").
Date: 2026-10-15

## Why

`fleet sessions` takes no flags (`src/cli.rs:534`, `no_flags`) and
renders a plain-text table — `id created model state` — which is both
the only surface over the archive and a violation of the house UX rule
that no fleet surface is ever a plain-text table. The archive itself is
the densest evidence fleet produces: every task, every tool call, every
error and its fix, append-only and complete by the
model-visible⟺logged rule. Today none of it is reachable except by
`cat`, and resume requires already knowing the session id.

Two things make this the right moment. The six-channel ranker is built
and proven live (unrelated cosines 0.58–0.66, so vector 0.15 and
direction 0.30). And the session archive is a *better* fit for that
ranker than the facts corpus: a session IS a trajectory — the task, the
calls that succeeded, the ones that failed and were corrected, in
order. On facts the direction channel needs phrases composed by hand;
on sessions the derivation is the record itself, with failures already
marked by their `error:` results.

## Acceptance criteria

- **C1** `fleet sessions --recall "<instruction>"` ranks the archive
  through the same six channels and renders cards in the house visual
  language — never a table.
- **C2** The trajectory channel reads the session's own record: the
  successful calls are the phrases, the failed ones are anti-vectors.
  The phrase-writing piece's derivation, applied to a finished session
  instead of a live one.
- **C3** The outcome maps to the sign vocabulary the ranker already
  carries: a session that stopped `completed` reads `resolved:`,
  `max-turns` reads `incomplete:`, `failed`/`interrupted` reads
  `diverged:`. No new sign vocabulary is invented.
- **C4** A second index, `sessions/.recall-index.json`, on the
  recall-index's exact discipline: embedder-keyed (a model swap
  invalidates it whole), stamp-keyed per session file (only new
  sessions embed), atomic write, never errors. Separate from
  `memories/.recall-index.json` so the two never invalidate together
  and neither pollutes the other.
- **C5** `fleet sessions <id>` views one session as cards: the task,
  the turns, the calls, the outcome, the route it walked. The audit
  view shows the record, not a summary of it.
- **C6** The plain list becomes cards too — the same treatment every
  other fleet surface already gets.
- **C7** A recalled session bumps its recall-count and lands a
  `recall_hit` row naming the session id, closing the same feedback
  loop the facts corpus runs.
- **C8** Provenance reads the session's outcome grade: a completed
  session is stronger evidence than one that failed. Documented as the
  session-archive reading of a channel the facts corpus reads from
  `basis`.

## Design

One entry per session, not per turn: the session is the unit of recall
("what did I do about X"), and a per-turn index would answer a question
nobody asks while costing an embed per turn.

The indexed text is bounded — the task, then the tool-call names and
compacted arguments, capped — because a long session's log is large
and the vector channel reads a sample, not a transcript. The cap is a
named constant with the same honesty as every other bound in the
crate: the channel degrades by design rather than flooding the fact's
frontmatter.

`--recall` shares the ranker's entry point with `fleet memories
--recall`; the two differ only in what they index. That is the whole
point — one ranking, two sources, and the credit loop gets to read
what actually happened rather than what someone wrote down afterward.

## Out of scope

- Full-text search over session *content* beyond the indexed text.
  The ranker is the retrieval; `grep` remains the audit tool.
- Cross-session threading (session A continued into session B). The
  parent/root chain already rides the delegation row; reading it back
  into recall is a later question.
- Deleting or archiving sessions. `memories` has `--archive`; the
  session archive is append-only by the same rule the log runs on.
