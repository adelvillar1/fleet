#!/bin/sh
# Runs the design-loop eval scenario and prints its report verbatim through
# the corpus test that carries it (`the_corpus_runs_green_and_lands_a_queryable_row`).
#
# The scenario is `design-loop-closes` in src/eval.rs: it seeds a scratch home,
# renders a design version through the browser lane the product itself launches,
# names the defect from the render's own pixels, records the review, lands a
# revision, and asserts the revision moved the pixels (delta threshold 0.50,
# "the revision changed nothing: delta 0.000" as the named failure). Absent a
# browser engine it SKIPS with the named reason — a skip is not a pass.
#
#    sh scripts/design-eval.sh
#
# It never reads or writes ~/.fleet: everything lives in a scratch home the
# scenario removes itself.
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO:-$HOME/.cargo/bin/cargo}"
"$CARGO_BIN" test -p fleet --test eval the_corpus_runs_green -- --nocapture 2>&1
