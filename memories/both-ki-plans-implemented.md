---
name: both-ki-plans-implemented
description: "2026-10-24/25 — both remaining open-row plans implemented: KI-249's embed_tail reader landed (bounds gated on a 20-row/24h window still accumulating), KI-252's moves 2-3 landed with the measurement gate REFUSING the two-stage default (0.830 < 0.98) and the asset library certifying aspect before placement"
type: project
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

2026-10-24/25, the owner's /goal "implement both plans as designed in the
order that makes most sense" — executed KI-249's reachable half first
(its reader is all that can land until the data window fills), then
KI-252's moves in dependency order. Fleet commits c1fa009 → 25d0966,
full suite 1674 green. Recaps: docs/recaps/2026-10-24-both-plans-implemented.md.

**KI-249 (plan active, not complete):** `calib::embed_tail` walks the
stream's day files and returns per-kind count/median/p95/max over
`embed_timing` rows, with the window gate (20+ rows over 24+ hours)
stating its shortfall BY NAME — that named shortfall is C0's deliverable.
The bounds-setting increments execute only when real use fills the
window; test `the_tail_reader_measures_and_names_the_window_shortfall`.

**KI-252 move 2:** the Matryoshka slice (`SLICE_DIMS` 128 — truncation is
the trained property; constant vectors are all parallel under cosine, so
test fixtures need variance) ranks the corpus recall's fan-out, top-24
(`PREFILTER_K`) re-rank full-width, behind `Memory.two_stage` (AtomicBool,
default OFF). The committed harness `the_two_stage_measurement`
(#[ignore]d — grades the live corpus and embedder) REFUSED the default:
mean recall@10 agreement 0.830 < 0.98 at K=24 over the 50-fact corpus
(single 427 ms vs two-stage 241 ms); 500 facts is the plan's floor for
the pre-filter being anything but overhead. The prefilter's ranked-out
shape is the partial embed's own (vector term 0.0), which is what made
it testable.

**KI-252 move 3:** `FLEET_ASSETS_DIR` names the library;
`deck::find_assets` embeds captions (file stem + optional
`.caption.txt` sidecar) against the intent and ranks by cosine;
`deck::certify_asset` grades the aspect against the project frame
(16:9 presentation, √2 portrait) refusing a breaking aspect by name;
an unreadable aspect is a named non-certification, an empty directory
the named absence. The deck tool's suggest mode carries the asset line
plus an optional `project_type`. Divergences from the plan text,
recorded: the harness lives in memory.rs's suite (deck.rs was a
drafting slip) and the asset index is per-call embedding over the
directory (no store namespace).

**The whitespace leg landed 2026-10-25 (0762777), where the format's
own text carries it:** `svg_whitespace` reads ink coverage from the
SVG's rect/circle/ellipse areas over the canvas — a FLOOR estimate
(paths have no text-extractable bbox, overlapping shapes double-count)
— and `certify_asset` requires it inside WHITESPACE_FLOOR 0.10 /
CEILING 0.90: a solid slab or a blank one is refused by name with the
numbers. PNG/JPG need a pixel decoder stdlib Rust lacks, so they carry
the aspect certification plus the literal sentence "whitespace
unchecked — needs a pixel decoder (KI-252's recorded remainder)". The
completion verifier flagged the original gap (ticked C4 claimed aspect
AND whitespace; the caveat lived only in the recap) and the fix was the
stronger option: implement SVG, record the remainder IN the plan's
ticked C4 — the plan file, not the recap, carries the true state.

KI-252 stays OPEN carrying its gate refusal; KI-249 stays OPEN until
the measured day. Ledger: 294 closed / 2 open / 15 noted over 311 rows.
Related: [[unlocked-ki-items-shipped]], [[ledger-rows-lag-their-fixes]].
