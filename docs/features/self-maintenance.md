# Self-maintenance under gates (slice 12)

The harness maintains **its own** skills, memory, and config — and, as
stage 2, its own source. Every change is a proposal: a plan with
acceptance criteria, a diff, and an expected effect, that applies only
when the local gate is green, the change is non-destructive or
explicitly approved in the current turn, and the verdict is recorded
in the calibration ledger. Every applied change is reversible, and —
since the adopt-on-contrast increment (2026-10-19) — every source
change is **measured** before it is kept, and every applied change
leaves a **lesson** behind.

```
src/improve.rs     the proposal, the gate, the contrast, the lesson
                   store, apply/undo, the surface, the rows
src/eval.rs        the corpus, the score line, the three-outcome probe
src/tools.rs       the improve declaration (the tool vocabulary's home)
src/agent.rs       the improve_home option, the declaration, the call path
src/calib.rs       the propose op: the verdict row with the gate's evidence
src/route.rs       validate_provider_entry — the roster's own schema, reused
src/cli.rs         fleet improve: list, apply, undo, lessons
src/main.rs        the command, and the tool's wiring into run/chat/TUI
```

## The fork's vocabulary, ported natively

The spec is the fork's runtime self-modification surface
(`packages/extensions`): read-only runtime API inspection, process-local
runners, and creator-mode plugin installation through Plugin Manager —
where **no model tool creates a dynamic definition**. Fleet's port of
that rule is structural: the `improve` tool *files* a proposal, and only
the owner's `fleet improve apply` runs one, through the same gate. A
session can propose a change to the harness; it can never apply one.
Nothing is imported, vendored, or shelled out to — the fork is the spec.

## The proposal

One markdown file per proposal under `<FLEET_HOME>/proposals/`, with a
frontmatter the skills parser already speaks and a body carrying the
four things a reviewer needs:

```markdown
---
name: demo-update
target: skill
target-name: demo
basis: inferred
created: 2026-10-14
status: pending
---

## Justification

A session corrected a bash failure and the correction belongs in this
skill.

## Acceptance criteria

- the skill loads and its body carries the correction
- `fleet skills` lists it with the new body

## Expected effect

the next session recovers from the failure without rediscovering the
fix.

## Change

```json
{"op": "write-skill", "content": "---\nname: demo\ndescription: the old body\n---\n\nnew body\n"}
```
```

- **`target`** is `skill`, `memory`, `source`, or `config`;
  **`target-name`** is the skill's name, the fact's name, the
  repo-relative source path, or the provider id.
- **`basis`** is required on every proposal, exactly as it is on every
  memory write: `inferred` when the session produced the change,
  `confirmed-by` when the user stated or verified it.
- **`status`** is `pending`, `applied`, `refused`, or `waiting`; the
  gate stamps it (with the apply's `commit` and the last `gate` line)
  when it disposes of the proposal.
- The **change** is a fenced JSON object whose `op` is one of
  `write-skill`, `retire-skill`, `supersede`, `forget`, `config`,
  `config-remove`, or `patch`. A machine-checkable change, not prose a
  reviewer has to interpret — and an unknown `op` is refused by name.

**Two stores, named honestly.** Slice 7's drafter files its own skill
drafts under `<FLEET_HOME>/skills/proposals/`; those files are the
skill-proposal feedstock, and `fleet improve` lists them beside the
general store. Neither store is migrated into the other: a slice-7
draft adapts into the gate's view at read time (its `kind` becomes the
change — create/update write the SKILL.md its body carries, retire
retires the skill, merge retires the absorbed half the body's first
line names), and applying one stamps its verdict into the draft's own
frontmatter rather than moving it. A drafted lesson reads as
`inferred` — the harness's own inference until someone confirms it —
and the drafter's bodies carry evidence, not acceptance criteria, so a
draft's `acceptance` and `effect` are empty.

## The gate

`Gate::local` runs, in the repository root, in order:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo build --release`
4. `cargo test`

Exit codes are read directly from each step's own status — never
through a pipe (KI-014's rule, held at the runner) — and every step
runs, past a failure, so the row carries the whole gate's evidence. A
step whose tool is **absent is a named gate failure, never a silent
pass**: on this machine rustfmt and clippy are not installed (KI-007),
so `cargo fmt` answers `error: no such command: 'fmt'` and every
proposal is refused with that named — the honest behavior, and the
reason a source proposal cannot apply here until the toolchain gains
them. The failure a step names is its last `error:` line (cargo's
`help:` advice after it is not the reason), bounded to 240 characters
in the row.

## The probe: three outcomes, never a zero

For a source change the gate runs the eval corpus as a probe over the
freshly-built binary. The probe is not a yes/no: `fleet eval` always
prints one `eval-score: <n>` footer line, and one
`regressions: <a>, <b>` line when it is red, and the probe grades the
subprocess into exactly three shapes —

| shape | what happened | the gate |
|---|---|---|
| `Green { score }` | every scenario passed | the change may land |
| `Regressed { score, scenarios }` | the corpus ran and named failures | refused, the scenarios named |
| `Err(reason)` | the corpus **could not be measured** — the executable did not resolve, the child timed out, a spawn failed, or a nonzero exit printed no verdict (a crash) | refused, named "could not be measured" |

The last row is the rule the contrast rests on, borrowed from
Harness²'s design and stated the same way: **a judge failure is
`None`, never a zero**. A crashed probe is not a regression, and it is
never read as one — the old behavior reported "the eval corpus
regressed" for any nonzero exit, including the driver-panic exit 101
the eval module's own docstring names. The gate keeps failing closed
on an unmeasurable probe (a source self-edit is never waved through
unmeasured); what changed is that the failure is named for what it is.

## The contrast: adopt on a measurement

Before the apply lands a change, `improve::apply` reads the most
recent recorded `eval_corpus` row (`eval::latest_row` — today first,
then the days the stream holds) as the **baseline**. After the probe
runs, the verdict carries the contrast:

- **after > baseline** — applied, the summary naming both scores:
  `corpus 0.95 → 1.00 (measured gain)`;
- **after == baseline** — applied and labeled plainly:
  `corpus unchanged at 1.00 — no measured gain`. A green-but-inert
  change is a legitimate outcome, and the summary never implies an
  improvement it cannot show;
- **no baseline recorded** — applied with that named, rather than
  inventing a comparison;
- **regressed or unmeasured** — refused, per the probe table above.

The scope is stated, never implied: the corpus measures the harness's
*machinery* (session replay, the tool loop, delegation, recall), so it
instruments a **source** change and cannot instrument a skill, memory,
or config change — a superseded memory fact does not move a scripted
scenario. Those targets keep adopting on the green gate, and the
verdict says so: `no corpus step — a memory change does not move the
scripted corpus`. Implying a measurement that cannot exist would be
worse than having none.

## The lesson store

Every **applied** change writes one lesson under `<FLEET_HOME>/lessons/`
— the loop's own history, the improvement-harness half of the
increment:

```markdown
---
name: fold-carrying-summary
target: source
target-name: src/lib.rs
created: 2026-10-19
contrast: corpus 0.90 → 1.00 (measured gain)
confidence: measured
---

## Cue

a source proposal for `src/lib.rs`

## Lesson

the fold carries its summary

## Evidence

the corpus named conversation-fold before the fix
```

- **The evidence rule.** A lesson needs non-empty cue, lesson, and
  evidence — the cue is the target kind and name, the lesson is the
  proposal's expected effect (or its change op when the effect is
  empty), and the evidence is the proposal's recorded justification.
  A proposal whose justification is empty lands but records **no
  lesson**, and the summary says why (`no lesson: ... carries no
  justification`). The store refuses a bare assertion at the write
  boundary too, exactly as an inferred memory fact needs its recorded
  justification before it supersedes.
- **Confidence** is `measured` when the contrast is a corpus
  comparison, `unmeasured` when the corpus does not instrument the
  target kind. It records the instrument, not a guess.
- **Read before the next proposal.** The `improve` declaration is built
  per run with the home, and its description carries the five most
  recent lessons — so the model sees what earlier changes learned
  *before* it forms the arguments. The filing confirmation also names
  the lessons recorded for the same target kind, which is the narrower
  per-attempt view.
- **`fleet improve lessons`** renders the store as house cards, newest
  first.

The store is a sibling of the proposal store, not a second memory
corpus: a lesson records a *change the harness made to itself* and the
contrast that justified it, where a memory fact records what the world
is. It starts inside `improve.rs` and moves to its own module only if
it earns one.

## Apply, revert, and the provenance rule

The honest reading of "applies only with the gate green" for a source
change is: **apply to the working tree, run the full gate, and revert
on any failure**. The net effect is that a source change survives only
if the gate is green — and the revert is structural, because the
pre-image is committed state. Per target kind:

| Target | Apply | Revert (failed gate) | Undo (after apply) |
|---|---|---|---|
| `source` | write the file | `git checkout HEAD -- <path>` (a created file is deleted) | `git revert` the apply's commit |
| `memory` | slice 8's `Memory::supersede` | same structural git revert of `memories/` | same |
| `skill` | write/move the SKILL.md, pre-image stashed | restore the stashed pre-image | restore the pre-image byte-for-byte |
| `config` | merge into `<FLEET_HOME>/config.json`, pre-image stashed | restore the stashed pre-image | restore the pre-image byte-for-byte |

The FLEET_HOME skills tree and `config.json` are not git-tracked, so a
home-side apply keeps the target's pre-image in
`<FLEET_HOME>/proposals/preimages/` (a replaced `SKILL.md`, a
moved-aside skill directory, or a replaced `config.json`) and
`fleet improve undo` restores it. A retirement *moves* the skill
directory aside rather than deleting it, which is what makes the
destruction reversible. Git-tracked applies commit with explicit paths
naming the proposal; the undo is a revert of that commit.

**The provenance rule is wired, not reimplemented.** Memory proposals
apply through slice 8's existing `Memory::supersede`, whose rule is the
permission model: an inferred fact supersedes on the recorded
justification, and a confirmed-by fact is refused without approval —
the gate reads the basis from the corpus to decide waiting-versus-apply,
and `supersede` enforces the same rule on the write itself. The stream
is deliberately not handed to `supersede` on this path: a gate revert
after a `consolidate` row would claim an action the files no longer
show, so a slice-12 memory change records its single `propose` row with
the justification in the reason instead.

## Destructive changes wait

A proposal that retires a skill, forgets a fact, or removes a config
entry is destructive: it waits for explicit approval in the current
turn — `fleet improve apply <name> --approve` — its status is
`waiting`, nothing is applied, the gate never runs, and the message
names exactly what will be destroyed:

```
fleet: improve: retire-demo waiting — `retire-demo` is destructive and would destroy the skill `demo` (its directory moves aside, restorable by `fleet improve undo`); re-run `fleet improve apply retire-demo --approve` to confirm
```

A non-destructive update — a skill's body corrected, an inferred fact
superseded, a config value changed, a source patch — applies on a green
gate alone. A proposal left waiting is unblocked by the approval and by
nothing else; a second apply without approval is the named guard, not a
second row.

## What the apply refuses

Two refusals are structural, not policy:

- **A patch under `memories/`.** The `patch` op writes any repo-relative
  path, and a path under `memories/` would rewrite a fact while
  consulting no basis — the provenance rule bypassed in one line of
  JSON. A fact changes through the `supersede` op, which asks the fact's
  basis before it writes; that is the only door, and a patch aimed at
  the corpus is refused naming it.
- **A dirty target.** The apply overwrites the target before the gate
  runs and a red gate's revert restores HEAD, so pre-existing
  uncommitted work in the target path would be silently destroyed. A
  target carrying uncommitted work is refused, naming it, on both the
  patch path and the memory path — commit or stash first. (KI-103.)

An applied change commits with explicit paths and a message prefixed
`fleet improve:`, so a machine-made commit is self-describing in the
history it lands in.

## The surface

- **`fleet improve`** — every proposal in both stores as house-style
  cards: the name and status in the header, the target, the basis, the
  last gate result, and the path. Always exit 0, like `fleet calib`.
- **`fleet improve apply <name> [--approve]`** — validate, ask for
  approval when the change is destructive (or the memory target is
  confirmed-by), apply, run the full gate, record. Exit 0 only when the
  change landed; a refusal, a wait, and a failed revert each exit 1
  with the reason named.
- **`fleet improve undo <name>`** — revert an applied change:
  structurally for a repo-side apply, byte-for-byte from the pre-image
  for a home-side one. The status returns to `pending` — the change is
  not in effect, and the ledger keeps the record that it once was.
- **`fleet improve lessons`** — the lesson store as house cards: one
  record per applied change, with the contrast it was adopted on, its
  confidence, and the lesson it carries.
- **`fleet skills --duplicates`** — the collapse report: pairwise
  line-Jaccard over the skill bodies at the named threshold
  (`skills::DUPLICATE_SIMILARITY`, 0.60), most similar first.
  Report-only — nothing is merged or deleted; the curation proposals
  stay the only writer. The detector is Harness²'s diversity measure
  ported as a visibility instrument: a library drifting toward
  sameness names its own pairs before it becomes a loader problem.
- **the `improve` tool** — declared on every run that carries a home
  (`run`, `chat`, the TUI; not on an unattended scheduled run, and not
  on a subagent child, which sees nothing of the parent). It files one
  proposal and returns the confirmation naming where it landed, that
  applying is `fleet improve apply`, and the recent lessons recorded
  for the same target kind. The call crosses the ordinary approval
  gate like every other tool, and the confirmation rides the same
  bounded `tool_result` path, so model-visible ⟺ logged holds.

The gate runs in the **working directory as the repository root**, the
way the memory corpus resolves from the cwd (KI-074's walk): run
`fleet improve apply` from the repository you mean, because nothing in
the command names the adopted root. A memory proposal targets the
repo's own `memories/` — the same tree `git checkout` reverts.

## The ledger

Every proposal that is applied, refused, or left waiting lands exactly
one `propose` row — one per verdict, so a proposal that waits and is
later approved lands two:

```json
{"op":"propose","ts":"2026-10-14T12:56:09Z","proposal":"demo-update","target":"skill","target_name":"demo","basis":"inferred","verdict":"refused","reason":"the gate refused the change: fmt, clippy, build, test failed — error: no such command: `fmt`","gate":[{"step":"fmt","ok":false,"detail":"error: no such command: `fmt`"},{"step":"clippy","ok":false,"detail":"error: no such command: `clippy`"},{"step":"build","ok":false,"detail":"error: could not find `Cargo.toml` …"},{"step":"test","ok":false,"detail":"error: could not find `Cargo.toml` …"}]}
```

`fleet calib` renders the op's card with the verdict tally, the last
proposal's reason, and its gate line. A `waiting` row carries an empty
`gate` — the gate never ran — which is what distinguishes "waiting on
the owner" from "refused by the gate".

An applied row's `reason` is the verdict's summary, and the summary
carries the contrast — `... · corpus 0.90 → 1.00 (measured gain) ·
lesson recorded` — so the row is where the adoption's evidence lives.
A refusal also rides back into the proposal's own frontmatter as
`last-refusal: <the named failure>` (one line), cleared by any later
applied stamp; `fleet improve` shows it on the card, so the next
attempt starts from the failure the gate named instead of
rediscovering it.
