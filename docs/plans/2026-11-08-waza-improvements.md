---
status: active
created: 2026-11-08
updated: 2026-11-08
slug: waza-improvements
---

# Plan: the Waza-inspired improvements — the review contract, the statusline, and the habits

## Context

The 2026-11-08 deep dive (`docs/architecture/deep-dive-2026-11-08-waza.md`)
read tw93/Waza — eight habit-skills and twenty rules — against fleet's house.
The correspondence validated the hard rules; five areas came out worth
building, and this plan builds them. All five are skill edits, small face
changes, or docs — none touches the machine's core.

**Owner ruling, 2026-11-08: this plan is needed and is to be implemented.**

### What the recovery audit found already landed

This plan was recovered from `stash@{0}^3`, and its criteria were then checked
against the tree one by one. **Six of the seven are already on master** —
landed by other routes between the stash and now — so this document records
the state rather than re-proposing the work. Re-landing any of it would be the
KI-473 shape in reverse: a tick claiming work that was already there.

| Criterion | State on master | Evidence |
|---|---|---|
| C0 — the review contract | **landed** | `~/.agents/skills/review-changes/SKILL.md` carries the outcome-contract, classification, risk-tier, finding-quality, sibling-sweep, isolation and sign-off sections; `~/.zcode/skills/review-changes/SKILL.md` is byte-identical (129 lines each) |
| C1 — the face's statusline | **landed and pinned** | `src/gui/assets/app.js:3377` renders the context row as the used tokens of the budget tokens, naming the model when the state document sets one; `tests/face/harness.mjs:1528` pins it |
| C2 — the rounds cap | **landed** | the skill's rounds section: three review/fix rounds, and from round two only the new diff |
| C3 — the model-switch re-run | **landed and pinned** | `src/cron.rs` `eval_model_path`/`read_eval_model`/`write_eval_model`, the forced run off the cadence, pinned by `a_model_switch_forces_the_eval_corpus_off_the_cadence` (`src/cron.rs:3864`) |
| C4 — the references | **six of seven landed; the build loop is the gap** | verified by listing the folders: `house-declaration-is-not-execution`, `house-evidence-discipline`, `house-immediacy`, `house-load-sensitive-tests`, `house-mockups-first`, `house-right-fixes` each hold exactly one playbook file in `references/`, linked from their SKILL.md; `house-build-loop` holds `templates/` only and has no `references/` |
| C5 — the chaining doc | **landed** | `docs/architecture/skill-chains.md` exists; `CLAUDE.md` points at it |
| C6 — the gate | this plan's own close condition | runs when the remainder lands |

So this plan's real remainder is **C4's last leg**: the build loop's playbook.
That is the work below, and C0–C3 and C5 are recorded here as already-true so
`plan_check.py` and a later reader see the evidence rather than a stale to-do.

## Direction lock

- **The review contract formalizes what exists; it does not replace the hard
  rules.** The edit consolidated scattered discipline — severity labeling, the
  paired review, the gate reading — into the review skill's contract layer.
  **Landed.**
- **The face's statusline reads the state document that already exists.** The
  context-health work landed the data (context, budget, and the model's name);
  the face renders it with no new bridge surface. **Landed.**
- **The house skills' references are content, not a loader change.** The
  SKILL.md bodies link their reference folders; the loader is untouched.
- **The rounds cap is a rule, not a mechanism.** Three rounds, then only the
  new diff — a discipline the review skill states. **Landed.**

## Approach

The remainder is one edit, in the shape its six siblings already have:

1. **The build loop's playbook (C4).** `~/.agents/skills/house-build-loop/references/`
   gains `verification-economics.md` — the deep playbook behind the SKILL.md's
   compressed rules: the pins-before-edits sequence, the generic preflight
   before the first compile, scoped test runs while iterating, the
   load-sensitive bounds and what a bound measured in the wrong unit costs,
   the read-the-exit-code rule, and the verification ladder that matches the
   check to the change. The SKILL.md gains the reference section in the exact
   form its siblings carry: a two-line section whose single bullet links the
   playbook and calls it the deep playbook.
2. **The gate (C6).** fmt, clippy, release build, and the full test suite —
   each exit code read directly, never through a status-losing pipe.

## Use cases

- [x] A: A review runs on a diff touching deployment workflows and the desktop
  shell — the risk tier applies the heavy gate and the human review whatever
  the diff's size. *(the skill's risk-tier section)*
- [x] B: A third review round finds new problems — the loop stops and the
  verdict is stated, not a fourth round. *(the skill's rounds section)*
- [x] C: The roster's served model changes — the eval corpus re-runs without
  waiting for the daily cadence. *(pinned by the cron test)*
- [x] D: A long conversation's face shows the context line, so the owner sees
  the window's use against the budget at a glance. *(the C1 harness case)*

## Acceptance criteria

- [x] **C0 — the review contract.** `~/.agents/skills/review-changes/SKILL.md`
  and `~/.zcode/skills/review-changes/SKILL.md` carry the outcome contract,
  the classification, the risk tiers by path, the finding-quality gate, the
  sibling sweep, and the isolation rule, with the status-first sign-off block.
  Verified: both files present, 129 lines each, byte-identical, every named
  section read.
- [x] **C1 — the face's statusline.** The face's run status area renders the
  context line from the bridge's state document in `src/gui/assets/app.js`,
  fed by `src/gui/bridge.rs`'s existing state read, pinned by a
  `tests/face/harness.mjs` case. Verified: `app.js:3377` and
  `harness.mjs:1528`.
- [x] **C2 — the rounds cap.** The review skill states the three-round cap and
  the round-two new-diff rule. Verified: the skill's rounds section.
- [x] **C3 — the model-switch re-run.** The cron's eval leg in `src/cron.rs`
  records the served model with each eval run and forces a run when the model
  changed since the last, pinned by a `src/cron.rs` test. Verified: the three
  model-path helpers, the changed-model branch, and
  `a_model_switch_forces_the_eval_corpus_off_the_cadence`.
- [ ] **C4 — the references.** Each `~/.agents/skills/house-*/` carries a
  `references/` folder with its playbook, linked from its SKILL.md. Remainder:
  `house-build-loop` — the other six are landed.
- [x] **C5 — the chaining doc.** `docs/architecture/skill-chains.md` names the
  chains, with a pointer from CLAUDE.md's session protocol. Verified: the
  doc's presence and the pointer.
- [ ] **C6 — the gate.** fmt, clippy, release build, and test — exit codes
  read directly, the whole suite.

## Files to be touched

**Skills (content) — the remainder:**
- `~/.agents/skills/house-build-loop/references/verification-economics.md` — new.
- `~/.agents/skills/house-build-loop/SKILL.md` — the reference section and its link.

**The six landed playbooks and the two review-skill trees are already present
and are not touched.**

**Docs:**
- `docs/plans/2026-11-08-waza-improvements.md` — this file, recording the audit.

## Out of scope

- **The small-set tension.** Waza's eight-skill bar against fleet's larger
  library is the owner's ruling; this plan adds no new skill.
- **The review contract's enforcement machinery.** The skill states the
  discipline; no hook enforces it, and the hard rules' hooks stay as they are.
- **The statusline's quota half.** The context line rides the state document;
  the five-hour and seven-day quota display is a later candidate.
- **Re-landing C0–C3 and C5.** They are on master; this plan verifies them and
  moves on.

## Verification

```sh
# The skill edit: the file exists and the link resolves
ls ~/.agents/skills/house-build-loop/references/verification-economics.md
grep -n "verification-economics" ~/.agents/skills/house-build-loop/SKILL.md
# The statusline (already green)
node tests/face/harness.mjs
# The cron change (already green)
cargo test --lib cron
# The gate
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```

## Linked artifacts

- `docs/architecture/skill-chains.md` — the chains.
- `CLAUDE.md` — the session protocol's pointer.
- `docs/architecture/deep-dive-2026-11-08-waza.md` — the design record this
  plan implements.

## Risks

- **The two skill trees.** `review-changes` lives in both `~/.agents` and
  `~/.zcode`; the two copies are byte-identical today (129 lines each) and a
  future edit to one alone reintroduces the drift, so that skill is edited in
  both. The `house-*` family is **not** mirrored — it exists only under
  `~/.agents/skills/` — so this plan's C4 edit has one home.
- **The statusline's data freshness.** The state document's context is the
  last request's tokens, so the line is stale between requests by design and
  refreshes on the next event.
- **The eval re-run's cost.** A model switch forces one corpus run against the
  mock-driven suite, with no live model — bounded.

## Notes

The plan's own premise — that five areas needed building — was accurate when
drafted and stale by the time it was recovered: most landed in the intervening
week. The audit table is the correction, and it is why this file is worth
keeping rather than re-drafting: it names which claims are true and how each
was checked.
