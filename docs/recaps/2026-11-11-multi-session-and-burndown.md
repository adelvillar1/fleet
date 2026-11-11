# Session Recap — 2026-11-11: multi-session, the KI burndown, and two cuts

**Summary.** A wedged-face triage became a feature: the face now runs
admitted runs concurrently (KI-525, the multi-session plan), shipped as
0.6.28. The owner then handed over the interrupted KI burndown — four rows
closed (KI-513/466/480/456), the fifth (KI-252) drafted into a plan — shipped
as 0.6.29. dev-decisions' plan-gate fix branch merged whole to main. Both
cuts verified on GitHub by anonymous feed read-back; both trees clean and
pushed. Two direction notes captured along the way: armada, and the
cross-harness memory pull.

## The arc

- **Triage (KI-525).** The owner's two-active-sessions test wedged the face:
  a stale `running=true` on an empty chat whose `driving` named a dead run.
  Root cause: the face is single-run (one serial agent thread) while its
  state is a per-session table; a queued second run never executed, leaving
  a phantom flag that blocked the composer. Filed KI-525, restarted the face
  clean, drafted the multi-session plan (gated PASS).
- **Multi-session (0.6.28).** Implemented as designed: the serial loop is a
  dispatcher, each admitted run drives its own session thread with isolated
  per-run events; `contract::admits_concurrent` grades the cumulative budget
  against headroom; over capacity paints a polite reconsider-or-queue card.
  C0–C5 verified by the session's own gate (2,389 tests) + face harness
  (1,108 checks / 140 cases). One load-sensitive flake found and fixed by the
  release gate (the C2 test pinned its route to skip the sys1 judgment).
- **The KI burndown (0.6.29).** KI-513 closed (already fixed, shipped 0.6.28);
  KI-466 fixed (corpus-refresh task strings name the dotless fact, the .md is
  the file); KI-480 fixed (task_text falls past house words, the digest reads
  owner messages + compaction summaries, the rail title falls to the run's
  summary); KI-456 fixed in dev-decisions (the plan-gate criteria parser
  scoped to the Acceptance criteria heading — the plan that parsed 19 now
  parses 14). KI-252's clustering remainder drafted as
  `docs/plans/2026-11-11-deck-semantic-clustering.md` (status draft), row
  stays open pointing at it. Open ledger: 5 → 1.
- **dev-decisions merge.** The `fix/plan-gate-truncation-and-criteria-index`
  branch (D-001 truncation + D-002 criteria-index) merged whole to main and
  pushed (`01f1377`); selftest 115 green.

## The cuts (both verified on GitHub)

- **0.6.28** — tag + release + six assets; anonymous feed read-back names
  0.6.28, cli sha256 810042e37e46.
- **0.6.29** — tag + release + six assets; anonymous feed read-back names
  0.6.29, cli sha256 2671dc0ced51.
- Both gates green on their HEAD; both settles recorded a runaway (the cold
  gate lane and the assets build outgrew their declared budgets) — contract
  learning rows, not cut failures. fleet master pushed to `c16bb6f`; both
  working trees clean.

## Direction notes captured (ideation, not plans)

- **Armada** (`docs/architecture/2026-11-11-armada-direction.md`): the
  owner's multi-install C2 + shared-corpus program, two growth lanes
  (single-owner armada → multi-user fleets org), the relay as the two-way
  control lane, deployment of the self-contained payload, and five security
  rulings (org SOUL as constitution; prompt ≠ security boundary; actor-blind
  governance boundary; least privilege at every layer; step-up 2FA on
  destructive grants the agent cannot satisfy). The owner sees it
  implemented sooner rather than later.
- **The cross-harness memory pull** (in conversation, not yet written): the
  inverse of the zcode→fleet transplant — fleet pulls memories from other
  installed harnesses into the projects it works on. The owner's portability
  ruling settled the shape: **import into the substrate, not federated
  recall** — a fresh armada instance has no local foreign stores, so the
  memories must live in the git-tracked corpus armada syncs. The pull is the
  cross-harness feeder into the same corpus armada distributes cross-install.
  Open: project-triggered vs continuous pull.

## Open

- **KI-252** — the deck's semantic clustering, drafted plan awaiting the
  owner's review; the row stays open until it lands and its measurement gate
  is met.
- **Armada + the memory pull** — direction notes, not committed builds; the
  owner's next program. The memory pull's project-triggered-vs-continuous
  question is unanswered.
- **The settle runaways** — both cuts outgrew their budgets; the contract's
  calibration rows carry the numbers (the gate lane builds cold).