# The curator: per-turn surfaces, then the judgment, then the panel

**2026-10-30** · the day's second arc. The owner's mem1/tool1 question —
would lanes for memories and tools improve decision making? — was
answered with a deep dive, a plan, and two landings: **the curator**
(the memory and tool surfaces re-decided per turn, in-process) and **the
curation judgment** (a three-judge sys1 panel, best of 3 wins, holding
the deciding seat by the owner's award with the one-constant rollback
standing beside it). Three defects from the deep dive are fixed and
closed (KI-375/376/377); the suite closed at **1,838 passed / 0 failed
across all targets**; the ledger checker green at **377 rows, 33 open**.

## The arc, in order

- **The deep dive** — the router (`src/route.rs` end to end),
  dev-decisions (three things sharing one name: the repo, the blend1
  corpus — 142 items/12 docs, verified current — and the cron job, which
  failed that night and cost nothing), and the six lanes (not five:
  `browser` sits between gen1 and blend1). Three defects found, none on
  the ledger: a dead lane's tools stayed declared for the run,
  `tool_search` activations never expired against their own documented
  lifetime, and `tool_route` was not a sys1 task — the class half of
  every search dead since it landed.
- **The plan** (`7b96232`) — the mem1/tool1 question answered as *the
  curator, not two more lane services*: both decisions already exist and
  are good (the six-channel memory ranker, the 32/9000 tool tier); the
  gap is when they fire. Four rules: trigger on drift, never a tick;
  deliver at the message tail, never the system prompt; hash-cached
  verdicts; availability is a logged transition.
- **Housekeeping** (`af7c360`) — the working tree's stray untracked
  state sorted by the repo's own rules: the architecture sketch landed,
  doc locks ignored on the KI-062 precedent, run debris to
  `tmp/stray-20261030/`, nothing deleted.
- **The curator landed** (`a7007a4`) — C0 the dead lane stops declaring
  (the `LaneFailed` withdrawal record, three replay readers taught the
  rule); C1 activations expire (`ACTIVATION_TURNS = 3`, a call
  re-stamps) and `tool_route` registers in the vendored sys1 (the doc
  that described it as gained for a week became true); C2 memory
  re-curates on drift (`DRIFT_FLOOR = 0.78`, measured live, the note at
  the message tail with the system prompt byte-identical, pinned); C3
  the ranker reaches the core (`CORE_KEEP` the floor, the
  `declarations` record names what was withheld — dormant on today's
  real surface at 28 declarations / 7,293 tokens / 0 withheld, quoted in
  the plan). KI-375/376/377 filed and closed.
- **K5 completed** (`babb109`) — the real-session replay the criterion
  named: session `997a54b4`'s four steers through the production
  embedder, zero mis-fires, the decision set stable across floors
  0.72–0.84.
- **The judgment** (`50e807f` drafted, `1731faf` landed) — the deferred
  C4 as a `curate` task in sys1, cached in the route cache's second
  namespace, composed over a four-row table whose fail-open row is
  byte-identical to the cosine-only behavior. The J6 replay (real steers
  through a real sys1 child) measured identical decisions with no
  domination, and the plan's contingency shipped advisory.
- **The award** (`b16597a`) — the owner read the verdict and awarded the
  seat anyway: *"award the seat and we roll it back if needed."*
  `CURATE_JUDGED_DECIDES = true`, the advisory behavior pinned beside
  the awarded path, the cosine safety net and fail-open unchanged.
- **The panel** (`caa5f2f`) — the owner's shape for the vote: three sys1
  judges, best of 3 wins. `CURATE_JUDGES` (glide, drex, jev — the routed
  chain's trio) fanned out in one classify call; the majority votes, the
  winning side's mean confidence meets the 0.6 gate, focus and staleness
  take their own strict majorities, a tie is no verdict and the cosine
  decides. The split (`judges`/`yes_votes`/`no_votes`) rides the calib
  row and the cached verdict — a 2-1 majority and a 3-0 sweep stay
  different facts.

## Commits

| Hash | What |
|---|---|
| `7b96232` | the surface-curation plan |
| `af7c360` | housekeeping: the sketch, the doc locks, the strays |
| `a7007a4` | the curator — C0–C3, KI-375/376/377 closed |
| `babb109` | K5 completed: the real-session replay |
| `50e807f` | the curator-judgment plan (C4) |
| `1731faf` | the judgment landed, advisory by the replay |
| `64894aa` | use case A unchecked under the advisory verdict |
| `b16597a` | the seat awarded |
| `caa5f2f` | the panel — three judges, best of 3 |

## Acceptance criteria

- `2026-10-30-surface-curation.md` — **completed**. K1–K8 all checked
  against named pins; the Landed section carries the real numbers
  (including the measurement the mechanism must stay dormant on: 28
  declarations, 7,293 tokens, 0 withheld).
- `2026-10-30-curator-judgment.md` — **completed**. J1–J7 checked; use
  case A is delivered by the owner's award rather than the replay (the
  plan says exactly that); the J6 section carries the measurement table
  and the contingency's verdict; the award section records the owner's
  decision verbatim and the rollback's shape.

## Doc updates applied

- `docs/features/tools.md` — the tier's real expiry rule (three
  requests, a call re-activates), the core withholding and its
  `withheld` record, and `tool_route` as a fleet addition.
- `docs/features/memory.md` — the new mid-run curation section (the
  drift trigger, the floor's measurement, tail delivery) and the
  curation-judgment section (the panel, the seat, the rollback).
- `TECHNICAL-DOCUMENTATION.md` — the `agent.rs` entry names the
  per-turn curation: activation expiry, the withdrawal, the judgment
  and its seat.
- `docs/known-issues.md` — KI-375/376/377 filed and closed; one row of
  this session's own writing (KI-377) repaired for the ledger checker
  just before this recap (raw pipes in the class list shifting the
  cells).

## CLAUDE.local.md changes

None this session.

## Open questions / next steps

- **The seat's behavior in production is the calib rows' to judge.** The
  J6 replay showed the panel under-confident on the glide provider
  (three of four votes under the 0.6 gate); if the `curate` rows show
  the gate never passing, the panel's confidence calibration — not the
  composition — is what to revisit. The rollback is
  `CURATE_JUDGED_DECIDES = false`, one line, pinned for.
- **Two defects from the deep dive remain open and diagnosed:** the
  vision workload always errors (`Client::routed` + the sync `chat`
  with no router branch), and `measure` is starved of `TABPFN_API_KEY`
  while blend1's tabular leg works (the two key universes). Neither is
  covered by a plan yet.
- **`corpus-refresh-dev-decisions` failed its 2026-10-30T00:50Z run**
  (runs 7, ok 6) — the only corpus job to fail that night; the store
  itself is current, so the cost was nothing, but the schedule is not
  currently reliable.
- The foreign draft `docs/plans/2026-10-30-landing-and-rail-details.md`
  appeared mid-session and is not this session's work — left untracked
  for its author.

## Notes

- The compaction tests and the memory-rail work share the turn loop
  with the curator now; the full suite ran green after every increment,
  and the two curation integration tests (the prefix pin and the
  first-steer rule) are the ones that would catch a regression in the
  delivery rule.
- The delegation tests' `home.txt` fixture keeps landing at the repo
  root (the path resolves against the process CWD); it is gitignored
  now rather than re-swept.
- The 8901 embedder stayed a single caller's problem by design: the
  curation reuses the recall embedder in-process, and the panel fans
  out over sys1's hosted chain, not over embeddings.
