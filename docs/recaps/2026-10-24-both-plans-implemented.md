# Both plans, implemented in the order that made sense

**2026-10-25** · plans `2026-10-24-embed-bounds-from-measured-tail.md`
(active, its window accumulating) and
`2026-10-24-deck-vector-moves-2-3.md` (completed, 5/5, gate PASS)
· full suite green (**1674 passed, 0 failed**), fmt/clippy/release clean.

## The order, and why

The measured-bounds plan's reachable half is its reader — increments 2–3
wait for a 20-row/24-hour window of `embed_timing` data that only
accumulates with real use. So: land the reader (it names the shortfall
while the window is under it), then build the deck's vector moves while
the window fills passively. KI-249's plan is now **active** with its
C0/C1 landed; its bounds-setting increments execute when the window is
met, by the plan's own gate.

## The deck's vector moves (KI-252's remainder)

**Move 2 — the Matryoshka pre-filter.** The slice (leading 128 dims,
the truncation property the model was trained for) ranks the vector
channel's fan-out; the top-24 re-rank full-width; the whole path rides
`memory.vectors.two_stage`, default **off** — because the committed
measurement harness ran the gate and it **refused the default**: mean
recall@10 agreement 0.830 against the ≥0.98 requirement over this
50-fact corpus (single-pass 427 ms vs two-stage 241 ms for ten
instructions). The plan's own floor — 500 facts before the pre-filter
is anything but overhead — explains the refusal; re-measure when the
corpus crosses it. The prefilter's ranked-out shape is the partial
embed's own (vector term 0.0), which is what made it testable.

**Move 3 — the asset library.** `FLEET_ASSETS_DIR` names the library;
`deck::find_assets` embeds each image's caption (the stem, plus a
`.caption.txt` sidecar when one exists) against the intent and ranks by
cosine; and the grader certifies the aspect against the project frame's
own ratio — 16:9 for a presentation, √2 portrait for infographic and
poster — refusing a breaking aspect by name. An unreadable aspect
(unknown format) is a named non-certification, never a silent pass; an
unset or empty directory is the named absence. The `deck` tool's
suggest mode carries the asset line — what matched, at what score,
certified or refused — with an optional `project_type` for the fit.

## Where the implementation honestly diverged from the plan text

- The measurement harness lives in `src/memory.rs`'s suite, not
  `deck.rs`'s — the plan's path was a drafting slip; the corpus recall
  is what it measures.
- The asset index is per-call embedding over the directory, not the
  sem1 store's `assets` namespace the plan sketched — asset libraries
  are small, and a store dependency for it would buy nothing the
  directory does not already hold. The grader certifies **aspect**
  only; the whitespace budget needs a pixel decoder stdlib Rust lacks,
  and the certification names what it checked.

## The state

KI-252 stays **open carrying its gate refusal** — both moves landed,
the default off until the numbers change. KI-249 stays **open** — the
reader landed, the bounds wait for the measured day. The ledger reads
311 rows: 294 closed, 2 open, 15 noted. Fleet's commits through
`fa06552` + this session's six are local awaiting your push.

## The commits

```
c1fa009 feat(calib): embed_tail — the tails and the window gate, named
c104db2 feat(memory): the two-stage vector path, default off
47e5472 feat(deck): the asset library, ranked and certified
57bf927 docs(ledger): the gate refusal carried, the reader half named
5f88bf6 docs(plan): the moves complete, the refusal recorded
```
