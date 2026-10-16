# Fleet routes through the router fleet owns

Status: complete
Date: 2026-10-16

Branch: `master`, solo. The session corrected a design error the owner
had already ruled on and I had not applied: Fleet was still routing
through the `agnostic-router-kit`, a separate app Fleet superseded.
Port 8300 — which I had read as the kit's — belongs to the zcode
harness's own router, and the kit is the precursor to Fleet, not a
dependency of it. The owner's words: "the precursor is not the
solution."

## The diagnosis

Fleet's error message had read "fleet cannot find the router," which
named the wrong thing. Fleet was never supposed to find a router. Fleet
*is* the app with the router in it — the in-process `route::select`,
where sys1 asks for a workload verdict, the workload preference table
picks the provider, and failover walks the declared chain. No process,
no port, no service, and nothing in Fleet that needs a router to be up:
not to select a route, not to show the router board.

I had spent several turns confirming the owner's stated fact about port
8300 against the machine, which was the wrong move — the owner had
already told me it as fact, and the confirmation was never the missing
piece. The missing piece was the design, not the deployment.

## The precursor leaves the roster

The `agnostic-router-kit` entry is gone from the compiled-in roster in
`src/route.rs`, along with every surface that reached for it. What
replaced each reach, in code:

- The router board's three reads are named in its own doc section and
  none of them dial anything. `boards::read_router` filters the
  calibration day's own route rows through the existing
  `calib::read_day` reader. The tier chains are now derived from
  `route::default_preferences()` at draw time, so a fixture can never
  disagree with the table about what a workload's own chain is — which
  is exactly what my hand-written fixture had been doing, claiming
  `standard_code`'s primary was `step-5-preview` when the table says
  `step-3.7-flash`.
- `BoardData` boxes all three variants. A snapshot is a message, not a
  value the caller holds: built once per read, carried over the update
  channel, matched by reference, dropped. Without the boxes the enum
  would be the size of the brain snapshot's 616 bytes whichever board
  was being carried — and shrinking `RouterSnapshot` to 360 had widened
  the gap past clippy's `large_enum_variant` threshold, so the boxes
  were already owed.
- blend1's fixture points at its own lane port (8420), not the kit's
  8300, matching the compiled-in roster entry.
- The integration tests' mock stand-in is renamed `stand-in` and fully
  specified with `base_url`, `key_env`, `models` and `key_required`,
  because it can no longer merge onto a built-in entry that no longer
  exists.

## DeepSeek's honest claim

The payg guard had been reporting backwards, and the roster removal is
what exposed it. DeepSeek is the only `Billing::Payg` candidate and is
declared in no workload's chain, so it can serve only when the owner
pins it in `<FLEET_HOME>/routing.json`. That is the guard's claim now,
and KI-214 records the inverted defect it replaces. The router board's
token line is gone too, for the same reason: route rows carry no token
counts, and the board says so rather than inventing a number.

## The docs say the ruling

`docs/architecture/ecosystem.md`'s routing rows carry a supersession
banner that names the fork's 2026-10-13 history as history, and the
2026-10-15 deep dive carries one alongside corrections to the chain, the
boards line, and the live-state evidence line. The new plan
`docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` records
C1–C8 with the superseded design kept rather than deleted, so the next
reader can see what was replaced and why. KI-116 and KI-126 close-fixed
on the evidence that `boards::router_config_path` and every
`AGNOSTIC_ROUTER_*` name are genuinely gone from `src/`.

Two artifacts were left as dated history rather than rewritten: the
`docs/recaps/2026-10-13-*` through `docs/recaps/2026-10-15-*` records,
which describe what was true when they were written.

## The gate

Whole, exit codes read directly at every leg: `cargo fmt --all --check`
exit 0, `cargo clippy --all-targets -- -D warnings` clean, `cargo build
--release` clean, `cargo test` 1013 passing / 1 ignored — 890 lib, 6, 2,
7, 11, and 97 integration.

Two failures during the work were mine, not the design's. The 80-column
board test asserted `+8 more` from the old fixture's count and
`drawn.contains("ledger")` from a line that is genuinely below the fold
at that width; both were replaced with the invariant — a line naming
`+N more`, which is the board telling you it is holding rows back rather
than clipping silently. The `calibration-loop-closes` eval scenario
asserted the wrong rows were served by `deepseek`, a leftover from an
earlier rename; the quick rows are served by `HEAD_ID`.

## Commits

| Commit | What |
|---|---|
| `33b043f` | the recall bookkeeping lands as its own increment |
| `40e5277` | fleet routes through the router fleet owns — the precursor is out of the roster |

## Live verification

`fleet providers` shows seven candidates with the kit absent and
`head zai-coding-plan`; blend1 reads `http://127.0.0.1:8420`. No live
code reference to port 8300 survives anywhere in `src/` or `tests/` —
the only mentions left are doc comments recording the ruling.

## What is left

Nothing in the plan's scope; C1–C8 graded met. The kit's two extra local
converters (`codebuddy` at :8787, `opencodex` at :10100) stay out of the
roster deliberately, with `fleet settings provider.add:` as the seam if
the owner ever wants them. The kit deployment on the machine stays
running — the owner ruled it stays, and it is not Fleet's business to
touch. The second dive's remaining findings stay parked for a later
plan, and the attended dogfooding experiment now runs on a router that
answers to nothing but Fleet's own home.
