---
name: deterministic-breach-is-a-pause
description: "A deterministic reserve breach pauses for a yes rather than hard-stopping — the owner's 2026-11-10 ruling — and the pause defaults to NO wherever nobody is present to answer, with `--yes` deliberately not counting as consent."
type: feedback
date: 2026-11-10
basis: confirmed-by
tier: durable
shape: [change, resolved]
phrases: [deterministic breach is a pause, permit_past_reserve, granted_past_reserve, pause for a yes, reserve breached not permitted, owner-permitted]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The resource contract's deterministic arm was specified as a hard stop — a reading below the reserve ended the run outright. Asked on 2026-11-10 whether it should hard-stop or pause for a yes, the owner ruled: **"deterministic breach is a pause."**

What that means in the code (`9e1f11b`):

- `Approver::permit_past_reserve(clause)` is a **new trait method, separate from `approve`**, because the question is not about a tool call — it is "may this run keep going after the machine reached its floor?" Its default is **NO**, and that default is the safety property: an unattended job, a headless batch, or a test has nobody to answer, and a machine at its floor must not hang waiting for a reply that will never come.
- `--yes` deliberately does NOT override the default. Approving tool calls is not standing consent for a run to keep writing past the reserve. Pinned by `an_approver_with_nobody_present_denies_the_breach_pause`.
- Interactive seats ask for real: `ConsoleApprover` prints the clause and takes y/N with end-of-input as no; `TuiApprover` and the face's `GuiApprover` carry the clause as the approval prompt's `summary` through the existing channel, so both seats render the numbers with no UI change. The face's wait polls, so a stop written mid-pause ends it the way it ends a tool prompt.
- A yes buys **one more of the run's own budgets** below the reserve (`Oversight::granted_past_reserve`: floor = reserve − budget). A second yes moves the floor by one more budget and no further, so a run cannot talk its way to the edge of disk one pause at a time. The grant lowers the floor only — it never excuses the budget, so the overage/judge arm still grades the work.
- The yes lands a calibration row with verdict `expected` and `decided_by: owner-permitted`, carrying the clause, so "somebody permitted this" is recorded rather than remembered.

The no path is unchanged from the old hard stop: the run ends with `Record::ContractStop` and the same clause, `why: "reserve breached, not permitted"`.

Documents updated to match: `docs/features/resource-contract.md` (the four beats and the judgment section), and the plan's C3 criterion (which said "stops") plus its Risks bullet, which now states the ruling and the one-budget non-compounding grant.
