# Session Recap — 2026-11-15: the draining, the project-centered workspace, and self-healing

**Summary.** The session opened with a wedged **Fleet Dogfooding** session
stuck "compressing context" — diagnosed as a provider window overflow (1.36M
tokens) whose reactive compaction looped forever because its own chunks
exceeded the window; stopped it and resumed its work here. From there, four
releases: **v0.8.4** (draining the sinkholes and dead ends), **v0.8.5** (the
project-centered workspace), **v0.8.6** (the substrate heals itself), and
**v0.8.7** (the self-healing review built out). Two audits and one plan
landed between them, and the harness's own reliability got two fixes — the
cut pipeline and the learned budget. Every cut verified by anonymous feed
read-back; both trees clean and pushed.

## The arc

- **The stuck session.** The dogfooding session hit ~1.36M tokens; every
  request 400'd `model_context_exceeded`, and reactive compaction started but
  cancelled after 22s (attempt 1). Sending a message re-triggered it — this
  time it ran, but each chunk was itself over the window, so it slid a fixed
  64-message window backward, failing every time. The unstick: read the stuck
  state from the ZCode DB + rollout log, `InterruptSession` to stop the futile
  loop, resume in a fresh session (the plan + diff were on disk).
- **The sinkholes + dead ends (v0.8.4).** The resumed plan's remaining work:
  the calibration lens surfaces all seven dark ops (footprint, shape, curate,
  validate, demand, learn probes, learn checks) through `ReadRow`/`row_json`,
  `curate` admitted to the read gate, the dead sidecar `spend` retired, four
  rail icons; the recall session channel opens itself; the Routed event
  carries one order key; the last C3 reads (a checkpoint's held-at/cost, a
  cron fire's session click-through). Committed, gated, deployed, released.
- **The surface audit → the project-centered workspace (v0.8.5).** An audit of
  the 0.7.0 twenty-plus surfaces against the 0.8 spine + seven lenses found
  nothing core fully orphaned but nine surfaces buried in Settings→Advanced
  and several views deleted. The owner's vision: a **project-centered
  workspace**. Built in P1–P5: design boards become deliverables (the desk
  reaches them, `design:<slug>` in the spine + `/api/projects`); a visible
  **Health** lens (second-brain + lanes + doctor, the `brainEvidence`
  collision fixed); routing → a Reviews section and a `delegates_to` spine
  edge; Advanced surfaces answer to plain names (secrets/assistants/versions).
  U44 amended. Released as v0.8.5.
- **The expert switch + project-centric desk.** Two dogfood bugs: the expert
  button flipped a flag nothing read (fixed: the rail wears subsystem names
  the switch reveals, `EXPERT_NAMES`); the desk landing showed runs instead of
  projects (fixed: deliverables group by project, briefs and design boards
  together, runs the last-resort fallback). The orphaned shelf helpers
  (`dkRail`/`dkShelf`/`dkOwed`/`dkDesignSection`) removed.
- **The substrate heals itself (v0.8.6).** Tracing "the app complains about
  Docker Desktop and layerfs" showed the probes were healthy but the wording
  was stale. Built app-driven self-healing: `detect_runtime` names
  kiri/Docker/none, the health loop auto-starts a stopped worker (a "starting
  container…" card, not a press), wakes a sleeping runtime, and offers
  **install kiri** (`/api/repair/install-runtime`, downloads + opens the
  dmg); the manual Repair press stays for a worker that will not start;
  runtime-neutral messages retire "start Docker Desktop". Released as v0.8.6.
- **The self-healing review → built out (v0.8.7).** A deep review mapped every
  failure surface, classified the healing (autonomous / approved / manual /
  none), and ranked candidates. Three slices shipped: (1) a real no-token API
  probe (a refused key or unreachable API names itself), vault/checkpoint
  naming, ENOSPC → fleet's own cache sweep, a cron retry-once; (2) lane
  handshake + MCP health in the loop, owner-approved cards (restore the
  previous build, rebuild lanes, sync the corpus), device-store messaging;
  (3) the leftovers — a scheduler supervisor (`catch_unwind` restart), the API
  connectivity line, corpus drift. Released as v0.8.7.

## The cuts (all verified on GitHub)

- **v0.8.4** — `144c9b9`, cli sha `d0d33166859c`. The sinkholes + dead ends.
- **v0.8.5** — `0845cb1`, cli sha `887177e67db9`. The project-centered
  workspace. (First **clean** assets settle after the prior runaways.)
- **v0.8.6** — `77ae06c`, cli sha `39486b0ba521`. The substrate heals
  itself.
- **v0.8.7** — `5df68e7`, cli sha `f510631f6f21`. The self-healing slices.
  First assets run failed on `hdiutil: couldn't unmount "disk6" - Resource
  busy` (three stale mounted images); ejected them and re-ran clean.

Each tag gated on a detached worktree, pushed to **both** remotes (origin =
gitea, github = release), and read back anonymously.

## Reliability fixes after the cuts

- **The cut pipeline** (`b37bf2a`): the assets mode now ejects fleet's own
  stale mounted disk images before `cargo tauri build`, so a prior cut's dmg
  can never fail a cut with "Resource busy" again.
- **The learned budget** (`9f291a1`): the release-assets lane settled RUNAWAY
  on every cut because the learned estimate was the **median** — and a
  variable class's median sits below half its passages (the lane's cuts vary
  1.0–3.6 GiB). A budget is a *bound*, not a typical value: the estimate now
  reads the **90th percentile** (`SETTLE_PERCENTILE`). Live, release-assets
  moved 2.17 → 2.90 GiB, matching its real cuts.

## The docs correction

Two untracked architecture docs (the idle-cycle vision and the
router/self-healing deep dive) belong to a **separate session**; my release
cut's clean-tree step folded them in by mistake. Untracked again (`f7cb299`,
files kept on disk) so that session owns their commits. (The v0.8.7 tag's
source snapshot still contains them — cosmetic.)

## The line held

Autonomous healing only for idempotent, reversible, non-destructive repairs
(restart, re-probe, rewrite a derived record, retry once, restore a previous
build); owner-approved for anything that changes config, spends, deletes, or
installs; the destructive class stays owner-only. Fleet stays
runtime-agnostic — the Engine API, never a vendor.

## What's next

- The **separate session** owns the idle-cycle/housekeeper architecture
  (grading heals, supervising the four loops, a principal for background work,
  one typed record) — the next architectural layer after the self-healing
  review.
- The **dogfooding UX glitches** (streaming flicker, the drawer) remain an
  unexplored reliability surface.
- The **assets-lane budget declaration** now settles clean on the typical cut;
  a genuinely larger cut still flags (correctly).
