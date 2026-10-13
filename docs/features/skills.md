# Skills (slice 7)

Fleet's procedural memory: SKILL.md files loaded from
`<FLEET_HOME>/skills/`, disclosed to the model description-first, fetched
body-on-demand through the `use_skill` tool — and drafted by the harness
itself from session evidence, filed as proposals that never apply
silently.

```
src/skills.rs     loading, the frontmatter subset, use_skill, the drafter, curation, the surface
src/agent.rs      the skills list in the system prompt, the use_skill call path, the skills record
src/tools.rs      the use_skill declaration (the tool vocabulary's home)
src/session.rs    the `skills` record: bookkeeping, like lane_tools
src/route.rs      the skills config section (curation knobs)
src/main.rs       `fleet skills`, and the drafter's wiring after a session
```

## The layout

One directory per skill under `<FLEET_HOME>/skills/`, each holding a
`SKILL.md` whose frontmatter carries at least `name` and `description`,
with the body after the fence — the owner's own convention, the same
shape the fork's `packages/skill` speaks:

```
~/.fleet/skills/
  bash-recovery/
    SKILL.md
  embeddings-probe/
    SKILL.md
  proposals/          drafts awaiting the slice-12 gate; never a live skill
    bash-recovery.md
```

The frontmatter is hand-parsed, not YAML-parsed. The subset is small and
its rejections are named:

- **Accepted:** an opening `---` fence, `key: value` lines, a closing
  `---` fence, then the body. Keys and values may carry colons and may
  be wrapped in matching single or double quotes — `"name": x` reads as
  `name: x`, the way a writer that quotes every key emits it. Blank
  lines inside the fence are fine. Unknown keys are kept, not rejected —
  a future field must not invalidate today's file. A leading UTF-8 BOM
  is stripped before anything else: editors write them, and a BOM'd
  file is a valid file.
- **Rejected, each with the reason named:** no opening fence; no closing
  fence; a fence line that is not a `key: value` pair (an indented
  nested block, a `- item` list entry, a bare scalar); a key whose
  quotes do not balance; a key that appears twice; an empty key; an
  empty value. A BOM'd file that still fails names the BOM in its
  reason, so the complaint is not misdirected at the fence.

A skill whose frontmatter is missing or malformed is **skipped by name**
with one logged line naming it — a broken skill never blocks the others,
and never enters the model's view at all. A live skill whose frontmatter
carries `proposed: true` is refused the same way: that is a proposal
that reached the live tree, and the loader does not promote it silently.
The frontmatter's `name` must equal the directory's: one skill, one
name.

## Progressive disclosure

Every run's system prompt carries the available skills as a
name+description list — the same seam the tool declarations use — and a
body enters context only when the loop selects it, through the
`use_skill(name)` tool. The body rides the ordinary `tool_result` path:
one approval-gated call, one bounded result, one log record.
Model-visible ⟺ logged falls out of the existing seam with no new
record type, and a skill's use in the TUI is the ordinary tool card it
has always been.

Because the list is dynamic — the files change between runs — each run
records the skills it declared as one session-log `skills` record at run
start, exactly the way a lane joining records `lane_tools`. The record
carries name+description per skill and replays to no message; a
selected skill's body is already in the log as that call's `tool_result`.
A run with no skills records nothing and its prompt is the compiled-in
constant, so the skill-less run is byte-for-byte the run it has always
been.

The list has a **soft cap**, `agent::SKILLS_PROMPT_SOFT_CAP` (50): past
it, one line per run is logged — `N skills declared; the list is
growing the prompt — curation retires unused ones`. A warning only:
nothing is hidden, every skill stays declared and callable, and the
prompt still carries all of them. The list is re-sent on every request
of every run, so an unbounded list is an unbounded per-turn cost, and
curation retiring unused skills is the lever that brings it down.

`use_skill` goes through the same Approver as every other tool: a
skill's body is model-visible content, and approving a body into
context is the same decision as approving any other text the model will
read. An unknown or unnamed skill is a typed error result
(`error: no skill named \`x\``), never a panic and never a silent empty
answer. The body is bounded by the same `MAX_TOOL_OUTPUT` cap and the
same `[truncated]` marker every built-in's output carries.

## The self-authoring half

After a session ends — `fleet run` after its task, `fleet chat` after
its loop, the TUI when its agent thread ends — the drafter reads that
session's log plus the day's calibration rows and identifies a **lesson**:

1. **A failure that was corrected.** One tool call that came back with a
   typed `error:` result, followed later in the same session by an
   approved, successful call to the same tool. The correction is the
   evidence the failure was understood, not just survived. A failure
   with no correction is not the lesson and does not end the search —
   the next failure in the log may be the one that was understood — and
   a denied retry corrects nothing, because its result is the denial
   rather than the tool's. Drafts `<tool>-recovery`.
2. **A repeated multi-step tool sequence.** One assistant turn that
   requested two or more tools whose exact ordered sequence appeared in
   another turn of the same session, every call in both turns approved
   — a procedure the session kept re-deriving. Two turns the user
   denied are not a procedure; nothing ran. Drafts
   `procedure-<a>-<b>`.
3. **A turn graded failed with a named class.** A calibration `turn` row
   with outcome `failed` and a non-empty `reason`, joined to this
   session by the task's `input_hash` — the row is the audit spine's own
   grade of the run, and the join is what makes the evidence this
   session's rather than the day's. Drafts `turn-failure-<class>`.

At most three lessons per session, in that order. The drafter is
deterministic: the same log and the same rows draft the same lessons,
which is what makes it testable without a model. A lesson whose name
matches a live skill is drafted as an **update** to that skill — the
live body plus a `## Proposed addition` section — so the draft diffs
against the file it would replace.

Every draft lands in `<FLEET_HOME>/skills/proposals/<name>.md`, never in
the live tree:

```yaml
---
name: bash-recovery
description: Recover a failed `bash` call: read the typed error, correct one thing, and rerun the same tool.
proposed: true
kind: create
evidence: session 20261013T214210Z-88706500: `bash` (command: ls) failed at record 4 with `error: exit 127`; the approved rerun `bash` (command: ls -la) succeeded at record 6
---

# bash-recovery
…
```

The evidence line is **self-contained**: the tool name, a compacted
argument summary on both sides of the pair, both record numbers, and
the session id, so the owner can judge the draft at the gate without
replaying the log. It is one line by construction — a long command is
cut with an ellipsis and a multi-line output keeps only its first line,
because the evidence rides the proposal's frontmatter.

**No code path in this slice moves a proposal into the live tree.**
Applying one is slice 12's gate: the local gate green, the change
non-destructive or explicitly approved, the proposal recorded in the
calibration ledger. `fleet skills --show <name>` prints a draft verbatim
so a reviewer can read exactly what would change. A lesson already
drafted is not re-drafted — the gate disposes of what is waiting, and a
later session's evidence for the same lesson does not rewrite the draft.

## Curation (pre-slice-8 recall)

Usage data comes from scanning past session logs: the last N session
logs, the `use_skill` calls whose results were approved, counted per
skill. This is usage-scan curation, not semantic recall — slice 8's
memory upgrades it.

- **Retirement.** A skill unused across the last N sessions (N
  configurable, default 20, `skills.retire_after_sessions` in
  `<FLEET_HOME>/config.json`) is proposed for retirement, with the
  window size in its evidence. The rule refuses to judge on a short
  window: with fewer sessions of history than N, nothing is proposed —
  "unused across the last 20" is not knowable from three sessions.
- **Merge.** Two skills whose descriptions are near-identical are
  proposed for merge. The similarity is **token Jaccard over the
  normalized descriptions**: lowercased, split on every non-alphanumeric
  run, deduplicated, shared tokens over the union. The threshold is
  **0.8**, documented here and in `MERGE_SIMILARITY_THRESHOLD`, and
  configurable as `skills.merge_similarity`. The survivor is the
  alphabetically first name; the absorbed one is named in the evidence
  and gets no separate retire draft — two drafts disposing of one skill
  is noise a reviewer should not have to reconcile.

## `fleet skills`

The loaded skills as cards — name, description, usage count from the
scan, the file's mtime — in the card style of `fleet lanes`,
`fleet providers`, and `fleet calib`. Never a plain-text table.

- `fleet skills` — the live skills.
- `fleet skills --proposed` — the drafts awaiting the gate: name, kind
  (create/update/retire/merge), the evidence line, and the draft's path.
- `fleet skills --curate` — run the usage-scan curation and file what it
  finds.
- `fleet skills --show <name>` — print one draft verbatim. The name is a
  bare proposal name: a name carrying a path separator or a parent
  component addresses a file rather than naming a draft, and is refused
  with a typed error rather than joined into a path.

The exit code is always success (a read surface over Fleet's own files,
like `fleet calib`); the failures are a `--show` naming a draft that
does not exist and a `--show` name that is not a bare name.
`--curate --proposed` composes: curate, then list.

**No new TUI surface this slice.** A skill's use shows as the ordinary
tool card in the conversation — the same card a `bash` call gets — and a
resumed log's `skills` record seeds no block. The skills' state is a
filesystem fact the headless surface owns, and the TUI already carries
the live state a user watches.

## What the drafter is not

The drafter is deterministic and evidence-shaped, not model-written: a
proposal's body is a structured draft carrying the records that justify
it, which is exactly what a gate needs to review and what makes the
drafter testable without a call. A model-polishing pass — the harness
writing the prose around the same evidence — is the natural upgrade and
is logged as KI-052.
