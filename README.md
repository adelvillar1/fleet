# Fleet

**Fleet is a coding agent that runs on your machine, under your own keys,
inside the folder you point it at.** It remembers, delegates, schedules,
and routes each piece of work to the model your plans already pay for —
with five specialized services deciding, measuring, indexing and
rendering, and the language model speaking last. It began as a question:
could an agent be self-contained — one binary, no runtime, no harness
checkout, credentials from its own home and nothing else? The answer
that holds is a harness whose router it owns, whose memory it can read
and edit, whose self-changes are proposals that never apply silently,
and whose autonomy is graded on evidence. MIT-licensed; the repo is
private.

## The arc

**The idea.** An agent that belongs to its owner: one static Rust
binary, no Node runtime, no dependency on another harness's checkout,
credentials resolved from the process environment and
`<FLEET_HOME>/.env` and read from nowhere else. The agent loop, the
session log, and the rule that everything the model sees is
reconstructable from that log were inherited from the DeepSeek Harness's
architecture and reimplemented natively — a port, not a fork
(`docs/architecture/overview.md`).

**The experiment.** Absorb the whole ecosystem into that one binary:
the sys1 house's five lanes — judgment, tabular, embeddings, media, and
their grounded composition — driven over a hand-rolled MCP client; a
router the app owns, with the precursor's decision core as its spec
rather than its dependency; a calibration stream in dev-decisions'
vocabulary that grades what served; and a self-maintenance loop whose
changes to the harness's own skills, memory, and source are proposals
that apply only through the local gate. The methodology grew with it:
every non-trivial piece of work is a plan with acceptance criteria, and
the plan is the source of truth for whether it is done.

**The solution.** A language model is one component, not the harness.
Five specialized services do the work a model would only guess at, and
the language model speaks last:

| Lane | What it does | What it returns |
|---|---|---|
| **sys1** — judgment | classifies the work: which workload tier, which execution shape, is this diff risky, did the run land | a label from a closed set, with a confidence — it cannot invent a category |
| **sdm1** — tabular | computes a number from rows: classification or regression over a table | a score with an interval — arithmetic is never generated |
| **sem1** — embeddings | similarity and recall: which fact, which skill, which session means this | vectors in, vectors out — never verdicts |
| **gen1** — media | renders images to a brief, and judges images against one | pixels, and a verdict on pixels |
| **blend1** — composition | composes the three siblings under one grounded answer, with the language model speaking last | an answer whose reasoning stands on computed facts |

Hallucination is controlled by construction — closed
output spaces, confidence gates in the path rather than in the prompt, a
generative leg that is bounded and validated before it ships, and
outcomes that are graded before they may change behavior. That is what
lets flash-tier models do work they would fail at alone: the cognitive
load the model is worst at is carried by components that are cheap,
fast, and reliable beside it. The case, with the evidence, is
[docs/architecture/lanes.md](docs/architecture/lanes.md).

**Tested.** The verification is one logic at three scales. The gate at the change: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test` with exit codes read directly, beside the fixed eval corpus graded headless and the plan, evidence, and arch gates over the plans. The judge at the day: the calibration stream grades every route it recorded, per provider-and-workload-shape cell, from redacted evidence only, and the owner's agree-or-disagree rides the same append-only row as ground truth. The same pattern runs at project scale: one-line tower reports reconciled against git. Nothing ships green by assertion; each claim has a gate, and the gates grade themselves.

**Here.** Alpha, version 0.2.0, past the agent core: routing, memory
recall, delegation, scheduling, self-calibration, a browser face, a
terminal face, and the harness maintaining itself. What is next is
named, not implied — see Sequencing in
[TECHNICAL-DOCUMENTATION.md](TECHNICAL-DOCUMENTATION.md).

## What ships today

- **Three faces over one agent core** — the browser console
  (`fleet gui`), the terminal surface (`fleet`), and the headless CLI —
  each with the conversation first. What the surfaces are for, one by
  one, is [docs/architecture/surfaces.md](docs/architecture/surfaces.md).
- **A router that owns its decisions** — the owner's declared chain,
  sys1-judged per task, failover by failure class, and deepseek-flash as
  the tail every walk closes on. Work is judged into tiers — quick,
  standard code, **frontend** (interface surface routes to
  deepseek-flash first, GLM behind it across two more hosts), hard,
  prose, deep context — and nothing is hardcoded: the router decides
  unless you pin, and `--model` is a pin resolved by name. Routing is a decision, not an inference: a closed verdict set, a confidence floor with the fallback recorded, and a judge that grades each provider by the shape it served, never an average.
- **Sessions that end only by archive** — the work list shows what is
  alive (`handed back`, `capped`, `working`), not what merely stopped;
  archiving is the owner's verb, and archived sessions gather in one
  collapsed group, restorable on demand.
- **A working folder that is a default** — set it once; every chat
  without its own choice works in it, and the face shows the folder the
  run will use.
- **Memory that reads, edits, and remembers its failures** — one
  markdown fact per file, git-tracked, recalled by meaning, shape-first: the problem's shape partitions recall and the words corroborate within it. Facts carry the vocabulary of trajectories that diverged as anti-vectors, the three endings of a problem (diverged, incomplete, resolved), the owner's own corrections as a tier that is never evicted, and an hourly consolidation that supersedes and archives what aged out. The corpus and its recall counts are both inspectable.
- **Compaction that scales with the model** — the trigger is three fifths of the serving model's declared window, with a 100k floor, a 32k reserve at the edge, and the last four turns verbatim, so a 128k model and a 1M model run the same rule at different scales.
- **Skills as procedural memory** — the canonical cross-harness library read in place, disclosed description-first, fetched on demand, arranged on the rail by declared category, with the harness's own drafts waiting as proposals that never apply silently — and an approve that lands a home-side write on its own validations, no repo gate required.
- **Delegation with a record** — a child is a real session on a scoped
  tool set, routed by the router over its own brief, with the plan's
  dependency graph carried beside it.
- **Work from outside, by grant** — paired devices and channels, off by
  default, capability fixed at pairing.
- **Credentials that seal** — a vault that lists names and renders no
  value, with onboarding that migrates the plaintext `.env`.

The console, surface by surface — every destination a user lands on.
The long pages run one idiom: a rail of sections (what needs you first,
then the substrate's own categories), one section at a time, one-line
rows, and the right pane for the item — settings, bots, skills, memory,
workspace history, and calibration all share it.

| | |
|---|---|
| ![the conversation](docs/images/surface-chat.jpg) | ![the router](docs/images/surface-router.jpg) |
| ![sessions](docs/images/surface-sessions.jpg) | ![delegation](docs/images/surface-delegation.jpg) |
| ![memory](docs/images/surface-memory.jpg) | ![skills](docs/images/surface-skills.jpg) |
| ![second brain](docs/images/surface-second-brain.jpg) | ![bots](docs/images/surface-bots.jpg) |
| ![workspace history](docs/images/surface-workspace-history.jpg) | ![vault](docs/images/surface-vault.jpg) |
| ![settings](docs/images/surface-settings.jpg) | ![calibration](docs/images/surface-calibration.jpg) |

- **Collision-free delegation, on LayerFS** — each delegated child works
  in its own writable workspace over one shared committed history, so
  concurrent agents answer a named conflict instead of stepping on each
  other's code. Fleet drives
  [LayerFS](https://github.com/Ephemeral-AI-Lab/layerfs) (MIT) the way
  it drives the vendored lanes — an external tool reached through its
  `layerfs` command, with nothing of it vendored and no runtime entering
  the binary. `fleet install` provisions the substrate as a standard
  phase, and a folder's first delegation registers it with no manual
  step; the merged child's work is applied home through a named
  conflict path, never silently.
- **The workspace history instrument** — the LayerFS tree read as a
  surface: a band for every published layer, a lane for every branch at
  its actual fork, a leaf per commit, a dashed stem where a publish
  grafted work back, the substrate's live call feed, a file's
  text/blame/revisions/diff, and the Store's own accounting — all from
  one backend read that answers warm in under a quarter-second.

## What is next

Effort levels per workload (research done, plan not written); the
`composed` workload's reachability by judgment; the bot-first phone
shell; the cron corpus-refresh repair. Each is named with its reason in
the technical document's Sequencing section.

Shipped: the guided onboarding (`docs/features/onboarding.md`). A fresh
`fleet gui` opens on the door — one account, one decision, and the
answer it proves — and everything else (the lanes, the projects folder,
the evidence token, the cost ceiling) arrives as benefit cards on
demand, never as a checklist. `fleet onboard` walks the same sequence in
a terminal, resuming where the machine left off.

## Start here

```sh
cargo build --release                     # one binary at target/release/fleet
target/release/fleet gui                  # the guided first run: the door
target/release/fleet onboard              # or the same steps in a terminal
target/release/fleet chat                 # the everyday surface
```

`fleet gui` on a fresh home asks for one thing — an account to think
with — and proves it with a real answer before anything else is offered.
`fleet onboard` walks the sequence in a terminal and resumes at the first
incomplete step; `--json` prints the document the face reads.

`fleet install` is idempotent; its exit code is the doctor's. The lanes
are vendored in this repository under `services/`, and their heavy
dependencies are an optional second step (`fleet install --lanes`), each
skipped by name when it cannot be built. No step needs Node or another
harness's checkout.

## Foundations

Ten foundations, named rather than borrowed quietly. Two are external; eight are the
owner's own work that predates and feeds this harness. What grew from them is its own architecture.

- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** — the seed this repo grew from. Its architecture gave Fleet the agent loop, the session log, the reconstructability rule, the lanes' vendoring pattern, and the decision core the router keeps as its spec. The port to Rust is where the inheritance ends: memory, context, skills, workflows, and the verification loops are Fleet's own designs, and the harness they compose is its own thing — answerable to this repository's rulings, not its ancestor's shape.
- **[LayerFS](https://github.com/Ephemeral-AI-Lab/layerfs)** (MIT, v1.1.0) — the substrate under collision-free delegation and the workspace-history instrument. Fleet drives the released tool through its command; every delegated child's copy-on-write workspace stands on it.
- **[dev-decisions](https://github.com/adelvillar1/dev-decisions)** — the owner's verification and calibration layer, built before this harness and integrated by wire contract. Its plan, evidence, docs, and architecture gates run Fleet's development; its JSONL log vocabulary is what Fleet's calibration stream writes into; its judge-grading floors are the trust model Fleet reuses.
- **[agnostic-router-kit](https://github.com/adelvillar1/agnostic-router-kit)** — the owner's harness-neutral router and workflow plane, the precursor to Fleet's router. Fleet owns routing in-process; the kit's decision core (roster, tiering, sys1-judged workload, quota-aware failover, and the journaled workflow plane) is the spec it was ported from.
- **The sys1 house lanes**, each one its own repository under dev-decisions' umbrella and each vendored into Fleet under `services/`: [sys1](https://github.com/adelvillar1/sys1) (judgment), [sdm1](https://github.com/adelvillar1/sdm1) (tabular scores), [sem1](https://github.com/adelvillar1/sem1) (embeddings), [gen1](https://github.com/adelvillar1/gen1) (media), and [blend1](https://github.com/adelvillar1/blend1) (grounded composition). The five-lane structure itself is the owner's design; Fleet carries the bundled copies, not the architectural idea.

## Where things live

- [TECHNICAL-DOCUMENTATION.md](TECHNICAL-DOCUMENTATION.md) — the system
  of record for running the project: interface contracts, environments,
  runbooks, the verification and release process, the risk register,
  sequencing, and the module reference.
- [FUNCTIONAL-SPECIFICATIONS.md](FUNCTIONAL-SPECIFICATIONS.md) — what
  the product does, surface by surface, in the user's terms.
- [docs/architecture/surfaces.md](docs/architecture/surfaces.md) · the
  what and why of all twelve destinations.
  [docs/architecture/lanes.md](docs/architecture/lanes.md) — the five-lane
  case.
- `CLAUDE.md` — the house rules, the pointer index, and the session
  protocol (warmup → plan → build → recap → commit). `docs/plans/` and
  `docs/recaps/` — the contracts and the journals. `docs/features/` —
  the how, module by module.
- Commands, exit codes, and the CLI's flags: `fleet help` and
  [FUNCTIONAL-SPECIFICATIONS.md](FUNCTIONAL-SPECIFICATIONS.md). Platform
  support (macOS arm64/x86_64, Linux x86_64/arm64; Windows anticipated
  and unverified — KI-011) and the release path
  (`cargo build --release`, one binary at `target/release/fleet`) are in
  [docs/features/install.md](docs/features/install.md).
