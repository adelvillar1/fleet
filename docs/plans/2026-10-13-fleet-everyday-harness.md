---
status: active
created: 2026-10-13
updated: 2026-10-13
slug: fleet-everyday-harness
---

# Plan: Fleet, the everyday native harness

## Context

The owner wants a fully working, installable, launchable native application to use as the every-day harness — the agent loop, sessions, skills, memory, subagents, scheduling, routing, and the boards — in one self-contained unit, **and as self-improving as technically possible**. Architecture is the owner's non-concern; the outcome is the concern. Time and token budgets are explicitly not constraints. The owner is open to pivoting to a different base harness if that is the better path.

### The base decision: no pivot (and why)

The question was "full pivot and rewrite onto a different base, or finish this one." The answer is **finish this one** — build Fleet out on the base already chosen — for three reasons:

1. **The ecosystem is base-agnostic, so a pivot buys nothing where it matters.** The lanes integrate over MCP (a standard protocol) and the calibrations/boards integrate over file contracts and read-only HTTP. None of that work changes with the base. What a pivot changes is only the *shell* — and the shell is exactly what slice 1 already started natively.
2. **On this base, every remaining piece has a working, tested reference implementation in a codebase the owner owns.** Streaming, skills, subagents, compaction, scheduling, routing, calibration, and the boards all exist in the DeepSeek Harness fork's fleet profile (`packages/fleet/dsh-*`, `services/`, `docs/fleet/architecture.md`). Each slice below is a port with a spec, not an invention. A third-party base throws the spec away and makes every integration a fresh design.
3. **A pivot rents an architecture and its mismatches become permanent constraints.** The credible native-ish candidates (codex-cli: Rust but OpenAI-loop-shaped; opencode: excellent but TypeScript) each mismatch this stack — DeepSeek/z.ai roster routing, sys1 judgment under the loop, the dev-decisions calibration vocabulary — in places that would be fought forever. The owner explicitly does not want legacy constraints from earlier decisions; inheriting a foreign loop's seams is the largest such constraint available.

The fork is therefore the **spec and integration lab**, not a shipping competitor: it keeps the canonical dsh-* implementations and the vendored cores; Fleet becomes the shipping native core; the two never both ship the same feature.

### The self-improvement requirement

"As self-improving as technically possible" is defined here as a **ladder of closed loops**, each earning the next on evidence — not a single feature:

1. **Remember** — the semantic layer: git-tracked facts with provenance and lifecycle, hybrid shape-routed recall with a probed vector path, scheduled consolidation (slice 8).
2. **Learn procedures** — skills the harness itself authors and maintains from session outcomes (slice 7).
3. **Calibrate** — every decision and every outcome graded into the dev-decisions stream; autonomy earned on evidence (slice 6).
4. **Self-maintain** — the harness proposes and applies changes to its own skills, memory, and config under gates (slice 12), with source-level self-editing as its gated extension path.
5. **Self-diagnose** — failures classified, layers probed, a fixed eval corpus run on a schedule, recaps and postmortems drafted from evidence (slice 13).

The guardrail philosophy is the owner's own, already proven in dev-decisions and the fork: **nothing destructive without approval in the current turn; every self-modification is a plan with acceptance criteria; it applies only with the gate green; it is recorded in the calibration ledger; it is reversible.** Self-improvement is proposed-then-gated, never silent. An ungraded change is not an improvement, it is a mutation — which is why the calibration ledger (slice 6) and the eval corpus (slice 13) exist before the loops that consume them.

### What "everyday harness" means here (the target surface)

Verified against the fork's package surface and the owner's working style: streaming chat and headless runs; sessions with resume; a closed-but-extensible tool set plus MCP tools; multi-provider routing with judgment and failover; skills; durable memory with recall; subagents with isolated contexts; compaction; scheduled runs; the router/second-brain/tower boards; approvals; one-binary install with `doctor` as the gate.

### Current state (verified 2026-10-13)

Slice 1 is in-repo and green: 55 tests (44 unit + 11 integration), binary smoke-verified, methodology layer landed (`a7ad239`, `aea7c69`, `107610e`). The client is non-streaming; there is no MCP client, no routing, no skills/memory/subagents/compaction/cron, no TUI, no installer. The four lane cores are vendored in the fork at `services/<svc>/` (pins at upstream HEADs) and boot-verified this session. The router decision core, calibration stream, and boards exist as fork packages with the architecture doc as their record. Nine known issues found along the way are logged in `docs/known-issues.md` — none ignored.

## Approach

A Rust core that **composes** rather than reimplements: the agent loop, client, sessions, and approvals are native (slice 1, done); the lanes are Python services spawned as MCP stdio children (they can never be Rust — the ML ecosystem is Python); routing, calibration, skills, memory, subagents, compaction, and cron are native ports of the fork's implementations, each slice naming its spec; the boards are native read-only surfaces over the same sources the fork's boards read.

Self-improvement is a **property assembled from the slices, not a slice bolted on**: the calibration ledger is the audit spine every later loop writes to (from slice 6 onward), memory and skills are the substrates the loops modify (slices 7–8), and slices 12–13 are where the loops close. The design rule for every loop: **propose → gate → apply → log → measure**. A loop that cannot show its effect in the ledger or the eval corpus does not ship.

### The owner's pre-build directives (2026-10-13) — standing rules for every slice

1. **Documentation is English-only.** No Chinese (or other non-English) documentation anywhere in this repository — a deliberate departure from the fork's bilingual convention.
2. **Delegation runs on the parent's base model** (step-5-preview class): subagents inherit the model and never downgrade; build-time delegation follows the same rule.
3. **Every error, warning, and improvement opportunity is logged, not ignored.** Anything encountered — in this crate, the vendored cores, sibling repos, or the toolchain — goes to `docs/known-issues.md` with its evidence, and is worked or explicitly deferred by the owner. Defects, gaps, and "this could be better" all land the same way; an improvement is logged, never silently scope-creeped into the running slice. "Not my code" is a reason to log, not a reason to skip.
4. **UX is co-equal with technical maturity.** Every slice ships its user-facing surface with the feature, never as a later polish phase; each slice's AC includes a hand-exercised UX criterion. The bar is the owner's own, set in the dashboards plan: genuinely visual surfaces — cards, status dots, real layout — never a plain-text-table-only view.

Plus the engineering standing rules: model-visible ⟺ logged; no secrets in the repo; the local gate (`cargo fmt --check`, `clippy -D warnings`, `build --release`, `test`) runs before every push and its exit code is read — never through a status-losing pipe — because CI is disarmed by owner decision (no Actions funds; the workflow is manual-dispatch only); unit + integration tests at the current bar, with MCP slices tested against the real vendored cores on loopback; no duplication of anything the fork already ships as canonical.

**Foundation decision — async.** Slice 2 adopts tokio. Streaming reads, concurrent MCP children, subagents, and scheduled runs are concurrent by nature; std threads would be fought for the rest of the app's life. This is the one place a small upfront rewrite prevents permanent debt.

## The slices

Order is dependency-driven: each slice unblocks the next and nothing is built ahead of its consumer. **The TUI lands at slice 3, immediately after the async core**, so Fleet is daily-drivable from there and every later slice extends it — UX is a per-slice deliverable, not a finale. Slices 12–13 are the self-improvement loops; they exist last because every loop needs its substrate and its audit spine first.

### Slice 2 — the async core: tokio + streaming (foundation)

Adopt tokio; rework `api.rs` to stream chat-completions (SSE parse over ureq's reader); refactor the agent loop to consume tokens as they arrive while keeping every existing behavior and log shape. Spec: the fork's `llm/stream` seam. AC: a streamed two-turn tool loop against the mock server passes; token events surface progressively; the 55 existing tests still green (adapted); usage accounting unchanged.

### Slice 3 — the TUI (daily-drivable from here)

A ratatui interface over the async core, landed immediately after streaming so Fleet becomes the daily driver at the earliest possible moment and every later slice extends it. First cut: chat and run panes with streamed tokens, the approval prompt, session browse/resume, tool-call progress, and the board pane mount point. Spec: the fork web client's information architecture (not its code); the UX bar is directive 4 above. AC: `fleet` with no args launches the TUI; a full task runs interactively with streamed tokens; approvals work from the keyboard; headless `run`/`chat`/`doctor` unchanged; the surface is exercised by hand, not only by tests.

### Slice 4 — MCP stdio client + the vendored cores

New `src/mcp.rs`: spawn `python3 -u <home>/service/mcp_server.py` children, speak JSON-RPC over stdio (initialize, notifications/initialized, tools/list, tools/call), register their tools in the agent loop under `mcp__<svc>__*` namespaces. Home resolution ported from `dsh-components`: machine home override → bundled copy under `services/` (vendored into this repo with the same `VENDOR.json` discipline) → unavailable with one named log line, boot continues. Fixes the sys1/gen1 `notifications/initialized` deviation (known issue KI-002) at the vendoring boundary or upstream. AC: all four cores launch from a fresh clone with no machine config; their tools callable inside a run; a dead child degrades without killing the run; integration tests drive the real cores.

### Slice 5 — multi-provider routing

New `src/route.rs`: a roster of OpenAI-compatible gateways (DeepSeek direct, z.ai Coding Plan, Xiaomi MiMo, and the router kit at `127.0.0.1:8300` as escape hatch) in config; per-task route selection that asks `sys1_classify` (the slice-4 MCP tool) instead of a ported judge; failover walks the declared chain on failure classes; every decision appends a calibration row (slice 6's writer). Chain order starts declared and becomes *calibrated*: graded route outcomes (slice 6) reorder candidates over time — the fork's router is the spec for the semantics, and the kit's provider benching is the spec for learning from failure. AC: a judged `auto` turn routes inside the roster; a failed head fails over to the next candidate; the kit serves as last resort; decisions are logged; a graded failure moves the failed provider down its chain.

### Slice 6 — the calibration stream

New `src/calib.rs`: one `events.jsonl` per UTC day under the dev-decisions log root, in `devdec`'s row vocabulary, through a serialized non-blocking queue. **Graded outcomes, not just decisions**: route and failover rows, tool-call screening verdicts, turn outcomes (ok/failed, with the partial endings named), and — once slice 13 lands — eval-corpus scores. This ledger is the audit spine every self-improvement loop writes to and reads from. Spec: `@fleet/dsh-decisions` plus the dev-decisions repo's own "gates the operations, grades the outcomes, earns autonomy on evidence" contract. AC: rows land in the stream; the external `devdec` CLI reads them beside its own; a full disk never fails a decision.

### Slice 7 — skills (with self-authoring)

SKILL.md loading from a skills directory with frontmatter, progressive disclosure (description first, body on demand), and selection by the loop. Spec: the fork's `packages/skill` and the owner's existing skill conventions. **The self-improving half**: after a session, the harness drafts the SKILL.md creations and updates its outcomes justify — the owner's own protocol (difficult task → lesson → skill) executed by the harness — and files them as proposals through slice 12's gate, never applied silently. Curation uses recall data: unused skills are proposed for retirement, near-duplicates for merge. AC: skills load with malformed ones skipped by name; a relevant skill's body enters context when selected; `fleet skills` lists them; a session's lessons surface as drafted skill updates awaiting the gate.

### Slice 8 — the semantic layer: memory that ranks, routes, and protects

The memory design merges the two implementations the owner already runs, fixing each one's weakness. Six parts:

**Substrate — git-tracked fact files.** One markdown file per durable fact under `memories/`, frontmatter carrying `name`, `description`, `type`, `date`, `shape`, and the lifecycle fields `recall-count`, `superseded-by`, `valid-until`, and — load-bearing — `basis: confirmed-by | inferred`. Human-readable, diffable, and structurally reversible: a bad fact is a revert, which is what slice 12's gated self-modification requires. The fork's `memories/` tree (234 facts in exactly this shape) is the spec.

**Retrieval — hybrid ranking with the vector path probed, not assumed.** Embeddings (a small local model, bge-small-en-v1.5 class, CPU, no network, no key) + keyword/FTS + importance + temporal decay, weights tunable. The lesson from the live system — where the embedding stack is absent and recall silently degrades to keyword matching over 11k entries — is that **the vector path is a probed, named dependency**: `doctor` checks it, and its absence degrades recall to keyword with one named line, never silently. Recall refreshes before each new instruction (the fork's brain pattern). The every-turn path owns its index in-process rather than depending on the sem1 child; sem1 (slice 4) stays available for the bulk corpus — workspace files, code symbols — where latency does not matter.

**Routing — shape-routed recall.** The incoming instruction classifies into one structure shape from a curated keyword vocabulary (`rendering`, `state-sync`, `deploy`, `build`, `change`, `deps`, `docs` — the fork's Wave D taxonomy, keyword-backed rather than model-backed after the model path proved unreliable), and retrieval routes by it. An unclassifiable instruction answers from the surface alone — the documented fallback, not an error.

**Feedback — the measure rung.** A recall hit updates the fact's `recall-count` and its rank; importance is earned by use, not asserted at write. Never-recalled facts age toward consolidation. Recall hits and consolidation actions land in the calibration stream (slice 6) as rows, so the loop's effect is auditable.

**Writes and tiering — mechanism over prose, and the protected tier.** `valid-until` is required for session-scoped facts and defaulted for working memory; durable writes record `basis`; the write path is `brain_remember`-shaped (the fork's vocabulary). Three speeds: working (session-scoped, expiring), durable (basis-recorded), episodic (consolidated summaries). **The deviation from both current designs**: identity and standing preferences are a protected tier the injection budget can never evict — the fix for the live system's mis-tiered cap, where the most load-bearing facts contend with task facts in the smallest space. Turn context is *assembled* (protected tier + shape-routed recall + recent working memory, budget-aware) rather than dumped whole; the observed fixed-dump failure — a live session injecting terrain-rendering and lead-pipeline facts into a turn about the memory design — is the behavior to avoid.

**Consolidation — scheduled, not manual.** A timer runs the cycle: compress old working memory into episodic summaries, supersede stale facts, forget expired ones, log every action as a calibration row. The live system's consolidation is manual and its episodic store has been dormant for weeks — the timer is the fix.

**Provenance as the permission model (feeds slice 12).** Inferred facts are proposals the self-maintenance loop may supersede on its own justification; confirmed-by facts are load-bearing and require approval to modify or delete. Memory provenance becomes the guardrail's input, not just metadata.

Spec: the fork's `@fleet/dsh-brain` and the Wave D slices (durable-fact lane, shape-routed recall, out-of-process indexing), the fork's `memories/` tree as the substrate, and the live Mnemosyne's hybrid-ranking design — ported with the five fixes above (probed vector path, recall-hit feedback, scheduled consolidation, protected tier, mechanism-constrained writes).

AC: facts persist across sessions and a bad one reverts cleanly; recall returns ranked matches for a new instruction and re-ranks on hits; with the embedding model present the vector path serves, with it absent `doctor` names the degradation and keyword recall still answers; a session-scoped fact expires on its `valid-until`; the consolidation cycle runs on its timer and logs rows; the protected tier survives an injection-budget pressure test; a confirmed-by fact cannot be modified without approval while an inferred one is supersedable by a justified proposal.

### Slice 9 — subagents

Spawn child agents with isolated contexts and scoped tools; results return with verification (the child's claims are checked before the parent reports them). **Same base model**: a subagent inherits the parent's model (step-5-preview class) and never downgrades — directive 2 applies in-product, not only at build time. Spec: the fork's `packages/subagent`. AC: a delegated task runs in its own session log on the parent's model; the parent verifies external side effects itself; failures return partial results, not silence.

### Slice 10 — compaction

Context-window management: summarize-and-continue under a token budget with the summary logged as a session record so model-visible ⟺ logged survives. Spec: the fork's `packages/compaction`. AC: a session outgrowing the window compacts and resumes coherently; the log reconstructs every request.

### Slice 11 — scheduled runs

Cron-shaped scheduling with a persistent job store, catch-up on missed runs, and delivery to a gateway-connected surface. Spec: the fork's `packages/schedule` + `packages/jobs`. AC: a scheduled task fires on time and after a missed window; `fleet cron` lists/edits jobs; a job's output is retained.

### Slice 12 — self-maintenance under gates (the loop closes)

New `src/improve.rs`: the harness maintains **its own** skills, memory, and config. Every change is a proposal — a plan with acceptance criteria, a diff, and an expected effect — that applies only when: the local gate is green, the change is non-destructive or explicitly approved in the current turn, and the proposal is recorded in the calibration ledger. Every applied change is reversible (git-tracked substrates make this structural). Memory changes follow the provenance rule from slice 8: inferred facts are supersedable on the loop's own justification; confirmed-by facts require approval. **Stage 2 — self-editing**: the same gate structure extended to source patches — the harness proposes a change to its own Rust, the proposal carries the failing test or graded outcome that justifies it, and it applies only with fmt/clippy/build/test green plus approval. Spec: the fork's runtime self-modification surface (`packages/extensions` — runtime API inspection, process-local runners, creator-mode plugin installation through Plugin Manager) and the owner's plan-gate protocol. AC: a session's justified skill update applies through the gate and appears in the ledger; a proposal that fails the gate is refused with the failure named; a destructive proposal waits for approval; an applied change reverts cleanly; a source-level proposal runs the full gate before applying.

### Slice 13 — self-diagnosis, the eval corpus, and the known-issues ledger

Three mechanisms. **Diagnosis**: per-layer probes in `doctor`'s image (credentials, home, each MCP core, the roster, the ledger), failure classification (network vs auth vs quota vs decode vs tool), and evidence-first drafting — recaps and postmortems composed from the ledger and session logs rather than from narrative, with unimplemented action items surfaced until closed. **The eval corpus**: a fixed, graded task suite (headless runs with asserted outcomes, in the image of the fork's `benchmarks/` tree — session-corpus replay, agent-continuation, conversation-fold) run on a schedule and before any slice-12 stage-2 self-edit applies; scores land in the calibration stream, and a regression blocks the edit. **The known-issues ledger**: `docs/known-issues.md` stays the human-facing system of record (directive 3); this slice automates its evidence gathering — a diagnosed failure files or updates a ledger row with its evidence instead of relying on someone noticing. sdm1 telemetry (turn-duration prediction, outcome scoring) joins here as the harness's model of its own performance. Spec: `@fleet/dsh-doctor`, `@fleet/dsh-sdm1`'s telemetry loop, the fork's `benchmarks/`, and the owner's recap protocol. AC: a broken layer is named at boot with its reason; a failed run produces a classified diagnosis and a drafted recap citing ledger evidence; the eval corpus runs on a schedule and its scores are queryable; a self-edit proposal that regresses the corpus is refused; a diagnosed failure files its ledger row with evidence.

### Slice 14 — the boards

Native read-only surfaces: router board (the kit's operator API), second-brain board (the six derived state files), tower (git facts per project). TUI panes and/or a small embedded web server with hand-rolled SVG/CSS — no chart library, matching the fork's approach. The visual bar is directive 4: genuinely visual, never plain-text tables. Spec: `dsh-router-board`, `dsh-second-brain`, `dsh-tower`, `dsh-board-seam`. AC: each board renders real data from its source; a refused read keeps the last snapshot visible and names the error.

### Slice 15 — install and launch

`fleet install`: home layout, config scaffold, optional heavy-lane venvs (a port of `services/bootstrap.mjs` — per-lane, idempotent, degrade-without), `fleet doctor` as the gate; release binaries per OS; the README's quick start becomes literally true on a clean machine. AC: on a clean machine, install → doctor green → `fleet chat` completes a real task; no step requires the fork or Node.

## Acceptance criteria (plan-gate)

- C1: `fleet install` on a clean machine produces a working home; `fleet doctor` green.
- C2: `fleet chat` streams; routes across the roster with sys1 judgment and failover; completes a real multi-step task end to end.
- C3: all four lanes callable as tools inside a run; calibration rows land in the dev-decisions stream and `devdec` reads them.
- C4: skills load and are selected; durable memory persists across sessions, recalls by shape with hybrid ranking, re-ranks on recall hits, and protects confirmed-by facts from unapproved modification.
- C5: subagents run isolated tasks on the parent's base model; the parent verifies their results.
- C6: sessions survive compaction and resume; a scheduled task fires on time and after a missed window.
- C7: the three boards render real data read-only and visually; a refused read keeps the last snapshot.
- C8: the local gate green at every slice — exit code read, never piped through a status-losing pipe; a release binary installs and launches on macOS and Linux.
- C9: after a session, the harness has drafted the skill/memory updates its outcomes justify; a justified update applies through the gate, lands in the calibration ledger, and reverts cleanly; a gate-failing or destructive proposal is refused or waits for approval.
- C10: a broken layer is named at boot; a failed run yields a classified diagnosis and an evidence-cited recap; the eval corpus runs on a schedule; a self-edit that regresses the corpus is refused; every error or warning encountered during the build is logged in `docs/known-issues.md` with evidence, and none is closed without it.

## Files to be touched (first slices)

- `Cargo.toml` — tokio (slice 2), ratatui/crossterm (slice 3); nothing else without a plan line.
- `src/api.rs` — streaming rework (slice 2).
- `src/agent.rs` — async loop, MCP tool registration, routing hook, calibration hook.
- `src/tui/` — new (slice 3, extended by every later slice).
- `src/mcp.rs`, `src/route.rs`, `src/calib.rs` — new (slices 4–6).
- `src/skills.rs`, `src/subagent.rs`, `src/compact.rs`, `src/cron.rs` — new (slices 7, 9–11).
- `src/memory/` (substrate, index, routing, recall, consolidation) and the `memories/` fact tree — new (slice 8).
- `src/improve.rs`, `src/diagnose.rs`, `eval/` — new (slices 12–13).
- `src/boards/`, `src/install.rs` — new (slices 14–15).
- `services/` — the vendored cores with `VENDOR.json` (slice 4).
- `tests/` — each slice's integration coverage at the current bar.

## Out of scope

- Reimplementing the lanes natively (they are Python services by nature).
- Shipping the fork's dsh-* packages as products; the fork stays the integration lab.
- The fork's web/desktop client parity; Fleet's surfaces are the TUI plus embedded read-only boards.
- Multi-tenant, hosted, or team anything; this is a solo daily driver.
- Windows as a first-class target until the core is daily-driven on macOS/Linux.
- **Un-gated self-modification of any kind**: no silent skill/memory/config/source edits, no self-rewriting of the system prompt outside the slice-12 gate, no autonomy the calibration ledger has not earned. The loops propose; the gates and the owner dispose.
- Non-English documentation of any kind (directive 1).

## Verification

Per slice: the local gate plus that slice's ACs exercised for real — MCP slices against the real vendored cores, routing against the live roster once the account has balance, boards against the live kit and the second-brain tree, self-improvement slices against a scratch clone where a bad proposal must fail visibly, UX criteria exercised by hand. The live-API e2e stays `#[ignore]`d until the DeepSeek balance is topped up; every other claim is tool-verified in-session.

## Linked artifacts

- `docs/known-issues.md` — the ledger every error, warning, defect, gap, and improvement lands in (directive 3); seeded with the items found before the build, KI-001 through KI-015.
- `docs/architecture/overview.md` — slice-1 crate architecture (updated per slice).
- `docs/architecture/ecosystem.md` — the verified ecosystem map and the per-piece absorption shapes.
- `docs/plans/2026-10-12-fleet-native-app.md` — slice 1's record; this plan supersedes it as the forward plan (it stays `active` only for C3's balance blocker).
- The fork's `docs/fleet/architecture.md` and `docs/fleet/runtime.md` — the spec for the ports.
