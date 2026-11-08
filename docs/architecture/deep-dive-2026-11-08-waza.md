# Deep dive: tw93/Waza read against Fleet — 2026-11-08

Source: [tw93/Waza](https://github.com/tw93/Waza) (MIT), read from a
full clone at fidelity — the eight skill files, the twenty rules, the
resolver, the scope bar, and the check skill's reference set. Every
fleet-side claim rides this session's verified reads of fleet's own
tree; the Waza-side claims are the repo's own files. Where this doc
infers beyond either, it says so.

Waza in one line: **tw93's engineering habits, turned into eight
small skills** — the technique (技) practiced until instinct. It is a
habits layer for general coding agents (Claude Code, Codex, Cursor,
and anything reading `~/.agents/skills`), not a machine: no memory
store, no face, no calibration stream. Fleet is the machine; the
interesting question is which of Waza's habit-structures fleet's
machine should adopt.

## Waza's shape

- **Eight skills, one job each**: `/think` (pressure-test and plan),
  `/ui` (interface work), `/check` (review and ship), `/hunt`
  (root-cause debugging), `/write` (prose), `/learn` (research),
  `/read` (URL/PDF), `/health` (agent-setup audit). Each carries an
  outcome contract, hard rules, and a gotchas table; the trigger
  words ride the frontmatter; the resolver is a human-maintained
  index.
- **The twenty rules** (`docs/ai-coding-rules.md`), grouped in the
  order a task moves — Need, Rules, Code, Debug, Review, Ship, Retro
  — written to paste into an agent's instructions.
- **The chaining model**: the skills compose (`/think` → implement →
  `/check`; `/hunt` → fix → `/check`; `/read` → `/learn` → `/write`),
  and an authorized workflow continues without re-approval.
- **The gotchas trace to real failures** ("every gotcha traces to a
  real failure", refined through 300+ sessions) — the same discipline
  as fleet's KI ledger and failure shapes.
- **The scope bar**: a new skill needs an observed failure and a fit;
  new behavior folds into an existing skill; the set stays small.
- **The install surface**: one `npx skills add` into the shared
  `~/.agents/skills`; optional persistent rules; a statusline.

## Where fleet already matches or exceeds

The correspondence is closer than expected — fleet's house rules and
machinery embody most of Waza's twenty:

| Waza | Fleet |
|---|---|
| `/think`: outcome contract, no placeholders, phase independence, the simplicity gate, implementation authorization | The plan-as-contract + `draft-feature-plan`: the acceptance criteria, the plan-gate's structure traps, the mockups-first rule, the owner-approval boundary |
| `/hunt`: root cause before any fix, the one-sentence hypothesis, the runtime evidence ladder, the regression guard (red-green run) | `root-cause-first-debugging`, `verifying-failures-before-acting`, `post-edit-error-attribution`, the verify-fix-actually-ran skill; the "say only what you have read" ruling is Waza's "no unverified claims" hard rule |
| `/check`: the outcome contract, the completion ledger, the worktree-safety preflight, the risk tiers, the sibling sweep, the three-round cap | `review-changes`, the hard rules (the destructive approval, the local gate, the explicit paths), `paired-agent-pr-review` (Waza's rule 14: writer apart from reviewer), the plan-worktree machinery, `severity-labeling` |
| Rule 8: enforce rules with hooks; count work only once it ran | The `.githooks/plan_check`, the gate's exit-code reading, the "a declaration is not an execution" ruling |
| Rule 11: worktrees for parallel agents; commit only your changes | The plan-worktree machinery (landed this week), hard rule 3 |
| Rule 16: deletion, billing, publishing are one-way doors | Hard rule 7; the approvals gate's always-ask class |
| Rule 17: every commit says what, how verified, how to roll back | The commit discipline; the recap's evidence columns |
| Rule 19: every production bug gets a test before each release | The eval corpus (the fixed graded suite) |
| Rule 20: fix the class, not the instance | The KI ledger's class fixes; the failure-shape taxonomy; this week's clustering beat and shape proposals |
| Rule 13: after three failed attempts, clear the context | The compaction fold (context health) |
| `/health`: the budget-aware summary before deep inspection | The context-health machinery (the budget/cache reading, the fold) |

Two Waza decisions fleet should note rather than copy: **no memory
skill** (a deliberate `.out-of-scope/` decision — habits live in
skills, not a store) and **the set stays small** (new behavior folds
into an existing skill). Fleet's memory substrate is the machine
Waza deliberately doesn't have; fleet's skill count (60+) is the
opposite of Waza's eight — the small-set bar is a genuine tension
worth the owner's eye, not this dive's call.

## The improvement areas

### 1. The review skill's contract (the `/check` shape)

Fleet's review-changes skill has the substance (the hard rules, the
severity discipline, the paired review) but not the **contract
layer** Waza formalizes: the outcome contract up front (done when
every verification claim comes from this session), the depth
classification (quick/standard/deep by size, stated before
proceeding), the risk tiers by path (CI, release, signing, auth,
billing, installers, deletion — the heavy gate and the human review
whatever the size), the finding quality gate (file:line, the trigger,
the upstream/downstream read, the severity a senior reviewer would
raise; "a clean review is a valid review"), the status-first sign-off
block, the completion ledger, the sibling sweep as a review step,
and the dirty-checkout isolation rule (verify in a separate worktree
when the tree is contaminated). Fleet's pieces exist scattered; the
improvement is consolidating them into the review skill's contract —
a skill edit, not code.

### 2. The face's statusline

Waza's statusline shows the context window and the quotas at a
glance. Fleet's context-health machinery already computes the
budget/cache reading; the face's status bar shows the memory line.
The improvement: the status bar gains the context/quota line — a
small face change over data that exists.

### 3. The review-rounds cap and the model-switch re-run

Waza's rule 15 caps review at three rounds (from round two, only the
new diff); rule 19's second half re-runs the test set on a model
switch. Fleet's plan-gate revision loop has no stated cap, and the
eval corpus runs on demand — a model-switch trigger (the roster's
model changes → the corpus re-runs) is a candidate. Both are
process-rule candidates for the house methodology.

### 4. The house skills' reference folders

Waza's skills carry `references/` and `scripts/` per skill, loaded on
demand — the `/check` skill's ten reference files are its mode
playbooks. Fleet's skills are single SKILL.md files; the house-*
skills' deeper playbooks would live in per-skill reference folders.
Fleet's loader reads the SKILL.md body; the bodies can inline or the
loader can resolve relative paths — the content structure is the
transferable idea (each skill owns its deep references beside it).

### 5. The chaining documentation

Waza's README documents the workflows (think→check, hunt→check,
read→learn→write). Fleet's session protocol (warmup → plan → build →
recap) is the chain, and the skill-chaining (a skill handing to
another) is implicit. The improvement: name the chains where the
owner looks — the session protocol's doc could list the common
skill chains.

## What this dive does not settle

Which of the five is worth a plan and in what order; whether the
review skill's consolidation or the face's statusline comes first;
and the small-set tension (Waza's eight vs fleet's sixty) — the
owner's ruling, not the dive's. The candidates are skill edits and
small face changes; none touches the machine's core.
