# The unlocked items: the mechanical backlog, worked

**2026-10-24** · plan `docs/plans/2026-10-24-unlocked-ki-items.md`
· status completed, 9 of 9 criteria · gate green by exit code
(**1671 passed, 0 failed**, fmt/clippy/release clean).

The owner-input audit left nine open rows, all mechanical; the owner
called to proceed. One of them turned out to have been finished for
nine days: **KI-190's designed fix landed 2026-10-15** (`e1796da` —
content-keyed lane entries, the walked-confidence pin, the beat-owned
third-vector cell) and the row never closed; this morning's audit note
("the blocker is lifted") was itself stale. It closes on evidence, and
the lesson is the ledger's own: rows lag their fixes, and "verify
against commit history" applies to my own audit notes too.

**What shipped:**

- **The deck's doors (KI-239, KI-241).** `deck::to_pptx` existed and
  was tested; the `deck` tool now carries `format: "pptx"` and writes
  the real file beside the render. And every deck run writes the
  rendered fragment beside the session (`deck-<stamp>.html`, the
  house stamp) — headless as much as in the face — with both paths
  named in the summary the model reads. The agent hands the tool the
  session's own directory.
- **The partial embed, pinned (KI-268).** The rank route always
  permitted a candidate's embed failing while others score; the
  scoring extracted to `bridge::rank_with` and a double that answers
  some candidates and not others pins the shape: scored leads, failed
  rides at exactly 0.0, descending throughout.
- **The smoke guard, where the smoke lives (KI-066).** The
  orientation's environment section carries the rule both recurrences
  proved: assert the mock's port file and fail closed; a pinned
  candidate alone has already fallen through.
- **The named-env refusal (KI-310).** blend1's `refresh` names
  `BLEND1_SEM1_PYTHONPATH` and its description says it needs the
  wiring — landed upstream on merged main, fleet's vendored copy
  synced byte-identical, pinned over real stdio. The test enforces its
  own precondition (a venv with sem1 installed makes the refusal
  unreachable; it skips naming why) — the pushed `1a53246` ran red on
  exactly such a machine before `45c6933` fixed that.
- **The measurement before the bound (KI-249).** Embed operations now
  record `embed_timing` rows on the calibration stream — kind, wall
  clock, fact count, redacted by construction — wired at the run path
  and the scheduled load. The bounds move only when a day of measured
  tail exists; the row stays open saying exactly that.
- **The intent skill's boundary (KI-240).** `visual-deliverable-brief`
  now says research is for facts, never for the tool — the schema is
  authoritative, `deck.rs` need not be read, and a run that spends its
  budget on research never reaches the build.

**Still open, on purpose:** KI-252's gated blueprint moves (the
Matryoshka pre-filter, the asset library — each its own plan) and
KI-249's second half (the bound changes, after the measured day). The
ledger reads 311 rows: **294 closed, 2 open, 15 noted** — the two
open rows are KI-249 (its measurement landed; the bounds wait for the
measured day) and KI-252 (the gated blueprint moves).

## The commits

```
6968032 docs(plan) · 3b6115c docs(ledger) · KI-190's evidence close
65dc03f feat(deck) · 9c07be6 feat(gui) · 8c05b28 feat(calib)
dd80377 docs: the smoke guard, the skill boundary, six ledger rows
11a235f docs(plan): complete, gate PASS
```
