---
status: active
created: 2026-11-19
updated: 2026-11-19
slug: armada-bridge-console
---

# Plan: the armada bridge console — containerized fleet instances on the 0.8.x stack

## Context

The armada plan (`docs/architecture/2026-11-11-armada-direction.md`, the three
`ARMADA-*.md` ledgers) was written against the 0.6.30 tree and delivered its
body with v0.7.0. The owner's rulings (2026-11-19), quoted and resolved:

1. *"the armada button is an oversimplification of the armada plan to have a
   muxin like application to manage fleet deployments across multiple devices
   and vps"* — what shipped is a read-only roster lens; the plan's essence was
   a control surface for deployments.
2. *"plan it around the current 0.8.x stack … role scoped to org admins and
   fleet owners … ride the current interface but with the right
   functionality"* — and *"maintain the essence of the original plan and not
   be just a reskin of fleet and watered down functionality."*
3. *"we package everything that fleet needs into a docker container, that
   could be deployed to a service like openship or digital ocean"* and *"fleet
   instances, armada deploys them as containerized instances ready to be
   used"* — **the instance is a container**, and deployment is armada making
   containerized fleet instances live on a platform.

The survey this plan rests on (measured in the tree at 0.8.20):

**Built and intact — the essence's substrate.** L1 identity
(`src/identity.rs`: InstallId, PrincipalId, device→principal registry);
L2 journal (`src/journal.rs`: append-only bus, subscribe, cursors); L3
security (`src/security.rs`: `Grant{principal, dimension, scope, class}`,
default-deny, owner wildcard, governance layer immutable to the agent);
`src/armada/`: `control.rs` (eight verbs with class+dimension, `admit` =
authority→guard→step-up), `authority.rs` (Ed25519 flagship key, signed
frames, key custody at the flagship only), `roster.rs` (register,
heartbeat, earned liveness), `deploy.rs` (`provision_local` executes,
`bootstrap_corpus` clones the git remote); `src/workspace.rs` (D37 landed);
dial-out relay transport (instances/phones behind NAT reach the flagship —
no inbound port needed, which is exactly what a container wants); the relay's
own Dockerfile (`relay/Dockerfile`: slim python, unprivileged, port from env,
no secret in the image) as the packaging precedent; the OpenShip CLI
(`openship` 0.9.12 on PATH) and its volume semantics
(named volumes valid on Cloud, host paths rejected, per-service volumes in
stacks).

**The watering this plan fixes (each measured):**

- **The roles the owner named do not exist.** `src/roles.rs` types are
  persona overlays for delegation (tools/skills/shapes/workload/voice) and
  carry **no authority**. There is no org-admin, no fleet-owner. What exists
  is the substrate they were always meant to be built from: the grant store
  is principal-keyed (D14/D15) and lane-2 atom **O2 "roles as grant
  templates"** is drafted but unbuilt. Honest mapping: org roles are grant
  templates over principals; persona roles remain delegation overlays.
- **Deployment is half-blind:** `deploy::vps_commands` *returns a
  `Vec<String>` of ssh command strings* — printed, never run. The
  container-native contract replaces it.
- **No linux packaging:** `scripts/release.sh` builds and publishes
  `darwin-aarch64` only; the feed asserts it. An instance image needs a linux
  build target.
- **Fleet serves no MCP endpoint** (it is an MCP client only), so no external
  agent can command it — the agent-first pattern the direction doc §10.1
  names is absent.
- **The face lens is a roster card plus a dispatch composer:** no deploy
  flow, no roles panel, no audit stream, no heartbeat age or corpus-sync state.
- **The seam ledger is stale:** `ARMADA-CONTRACTS.md` verifies fleet seams at
  **0.6.30 file:line**.

The decided shape, recorded as ledger choices:

- **D58 — no second binary.** An instance and a flagship differ by *role and
  configuration*, not by a fork. The "standalone application" ambition is the
  flagship's control plane surfaced through fleet's existing interface, CLI,
  a new fleet-served MCP endpoint, and the relay.
- **D60 — the instance is a container.** A fleet instance ships as an OCI
  image (binary + service lanes + runtime), deployed by armada onto a
  platform (OpenShip, Docker Engine — later: any container host). "Ready to
  be used" means: corpus cloned, registered on the flagship's roster,
  heartbeating, dispatchable, and reported **un-calibrated** until it settles
  (D49). The ssh command-drop is retired: the container contract replaces it.

## Decision-model check

No sys1 job fits the atoms below, and the plan declines to invent one: verb
admission is already a deterministic guard (`control::admit`), grant checking
is set algebra, deploy sequencing is ordered process execution, and provider
calls are fixed protocol adapters. Dispatch *targeting* stays explicit per
D46 (a named sid); a placement scorer (capacity × reliability) is named as a
**later** candidate and deliberately not built — autonomy is earned (D4),
and nothing in the rulings asks for autonomous scheduling.

## Approach

**The flagship commands; instances are containers.** Six pieces, ordered by
what unblocks what:

1. **The seam refresh (the ledger first).** Re-verify every
   `ARMADA-CONTRACTS.md` EXPECTS row against the 0.8.20 tree with fresh
   file:line evidence, record the new seams the plan consumes (workspace
   graph, roles store, plugin host, the OpenShip provider), and re-stamp the
   verified-at line. The ledgers are the contract this plan amends, so they
   move first.
2. **The instance image (the packaging).** One `Dockerfile` builds a linux
   fleet image: the `fleet` binary (linux x86_64, from a new release target —
   `scripts/release.sh` gains the linux leg and, after tagging, `docker
   build` + push from the release host: the repo's CI is deliberately
   unarmed — `workflow_dispatch` only, owner decision 2026-10-13 — so the
   release script is the publisher, matching how the darwin binaries and the
   updater feed already ship), `services/` beside the binary (the
   lanes resolve through `FLEET_SERVICES_ROOT` or beside the exe, both
   working in-image), `python3 + git + uv` for the venv lanes and the corpus,
   `obscura` linux-x86_64 as the browser engine, an unprivileged runtime
   user, `FLEET_HOME=/data` over a named volume. Headless by design: an
   instance runs runs, corpus sync, cron and channels — not the desktop.
   The image carries no secrets: env and the volume are the carriers.
3. **The instance entry (`fleet armada join`).** One env-driven boot mode:
   read `FLEET_FLAGSHIP_PUBKEY`, `FLEET_RELAY_URL`, `FLEET_CORPUS_REMOTE`
   and the join secret from env, bootstrap the corpus (guarded to a fresh
   install — `bootstrap_corpus` already refuses when `.git` exists, which is
   what makes a restarted container idempotent), mint-or-read the install-id,
   `register` on the roster, then heartbeat on a timer. Missing env is
   refused by name, never defaulted silently (D3). An unknown sid's
   heartbeat stays refused (the join is the gated act). Interactive prompts
   are absent by construction — a container cannot answer one.
4. **The deploy leg (armada provisions via the platform).** The flagship's
   deploy act creates a **container service spec** — image + named volume +
   env (relay, corpus remote, join secret, capacity) — and hands it to a
   provider adapter: `openship` (its project/service/deployment API; the
   flagship holds an OpenShip token in the vault, never in config), and
   `docker` (a local or remote engine via `docker engine` context) as the
   self-hosted leg. The whole sequence is `destructive`-class (step-up per
   C7), each step settles a `provision` journal event with its real outcome,
   a failed step names itself and stops, and a dry-run preview is the default
   listing. The fresh instance appears on the roster un-calibrated and turns
   live at its first heartbeat — measured readiness, not assumed.
5. **Roles as grant templates + the bridge console (the owner's org admins,
   riding the current interface).** `src/security.rs` gains the template
   store over the existing grant vocabulary — `fleet-owner` (the bootstrap
   owner principal, holds all, unchanged), `org-admin` (`governance` +
   `dispatch` on `org`/`instance` scope, **cannot alter the owner's grants
   or the `fleet-owner` principal's own grants — refused by name**),
   `operator` (`dispatch` + `read` on granted instances), `auditor` (`read`
   of state and the audit stream). `src/identity.rs` gains the `principals`
   array (O1) beyond the implicit owner; a run's principal resolves from
   registry/device context, defaulting to owner exactly as today. Granting
   a template is a journaled governance act. The console rides the existing
   face: the armada lens becomes per-instance cards (heartbeat age,
   capacity, settled reliability, corpus-sync state), the deploy flow
   (target → provider → step-up → live journal tail), the roles panel
   (principals, templates, grant/revoke through the seat), and the guard
   audit stream. Route additions are additive (`/api/armada` extended,
   `POST /api/armada/deploy|grant`, `GET /api/armada/journal`); the lens
   stays a row in the existing rail, annotated in the expert census
   (landed 2026-11-19).
6. **The fleet-served MCP endpoint (agent-first, D59).** `fleet armada mcp`
   serves the control vocabulary as MCP tools over stdio — `roster_read`,
   `instance_state`, `instance_dispatch`, `corpus_sync`, `health_read`,
   `deploy`, `version_roll`, `retire`, `register`, `heartbeat` — D46's verb
   table is exactly the tool list. Framing reuses `src/mcp.rs`. Every call
   passes the **same** `control::admit` (authority → guard → step-up) with
   the caller's principal from the endpoint's context; default-deny;
   `destructive`/`governance` tools refuse by name without the grant and the
   fresh proof. `fleet mcp` loops the endpoint in as `mcp__fleet-armada__*`,
   so agents command the fleet — the muxin pattern without a fork.

7. **The shared mind's transport, widened (C11, the owner's gitriver
   ruling).** Measured before designing: the repo ALREADY carries
   `memories/` (681 files), `roles/` (7) and `plugins/` (4), and a home
   holds only its own private state — so the shared mind has a real
   transport. What is missing is three specific classes, and this piece
   closes exactly those: the **canonical skill library** (467 skills at
   the user home, and it is not a git repo — the largest gap), **`mcp.json`**
   (the tool registrations, home-only), and **workflows** (no store to
   sync; the chains are derived). The design is a declared split —
   `corpus::TRACKED` and `corpus::PRIVATE` with a `refuse_private` that
   names the class it refuses — plus a per-class sync report so a class
   that could not move is named instead of disappearing inside a
   whole-repo success. In-flight work's own index travels; its raw
   transcripts never do (they are a session's private record). GitRiver is
   the documented host: self-hosted, Rust, one binary plus Postgres, and it
   carries an OCI registry beside the git host, which is what makes it the
   natural home for both the shared mind and the instance images. The
   contract stays plain git, so a GitHub remote keeps working.

Throughout: **D6 holds** — a home with no roster, one principal, and only the
owner wildcard behaves byte-for-byte as 0.8.20, and the full gate proves it.

## Use cases

- **Grow the fleet:** the owner presses deploy, picks OpenShip and names the
  project/service, armada writes the service spec (image, volume, env), the
  platform runs it, the container joins, and the roster shows a new
  un-calibrated instance that goes live at its first heartbeat.
- **Hand a team member the wheel:** the owner grants `operator` to a paired
  principal; that principal dispatches to granted instances and reads their
  state, and cannot deploy, roll versions, or touch grants they do not hold.
- **Delegate administration:** an org-admin grants templates within their
  scope and reads the audit stream; the owner's grants are immutable to them
  (R3's actor-blind boundary).
- **Let an agent command the fleet:** an MCP client lists the fleet's tools
  and dispatches to an instance; a refused verb is refused by name.
- **Restart a container:** a redeploy re-runs `join` against an existing
  corpus — refused as already-bootstrapped, install-id preserved, liveness
  resumes; nothing re-clones, nothing re-mints (idempotence by the fresh
  guard already in `bootstrap_corpus`).
- **Read the fleet's honesty:** every refusal, grant check and destructive
  act lands in the audit stream per principal and per verb.

## UX routes

- `#/armada` — the bridge console (census row: `fleet` / subsystem
  `armada`); sections: roster, deploy, roles, audit.
- `fleet armada roster|status|sync|dispatch` — unchanged; `deploy` becomes
  the container-provisioning act (`--provider openship|docker`,
  `--dry-run` default preview); `grant|revoke` and `join` join the verb set;
  `fleet armada mcp` serves the endpoint.
- `POST /api/armada/deploy|grant`, `GET /api/armada/journal`, `/api/armada`
  extended — the console's reads and writes.
- The image: `Dockerfile` at the repo root; the linux release target rides
  `scripts/release.sh`, which also builds and pushes the image after a tag —
  the release host is the publisher (CI is unarmed by owner decision
  2026-10-13); the registry (ghcr or the OpenShip one) is named at build
  time.

## Acceptance criteria

- [x] C1: every `ARMADA-CONTRACTS.md` EXPECTS row names a live 0.8.20 offer with fresh file:line evidence, the new seams (workspace graph, roles store, plugin host, provider) are recorded, and the ledger's verified-at line says 0.8.20.
- [x] C2: the repo builds a linux fleet image whose container boots headless to `fleet --version`, resolves its service lanes in-image, and holds all state under one volume path; the image carries no secrets; the linux release target exists in `scripts/release.sh` and the publishing path is named in the plan's files list.
- [x] C3: `fleet armada join` bootstraps the corpus, registers on the roster and heartbeats from env alone; a restarted container with an existing corpus is refused re-bootstrap and keeps its install-id; missing env is refused by name; neither the corpus nor the install-id is re-minted.
- [x] C4: `fleet armada deploy` provisions a live instance through a provider adapter (openship or docker): the service spec carries image, named volume and env from the vault/config; every step settles a `provision` journal event; a failed step names itself and stops; dry-run is the default preview; the new instance lands on the roster un-calibrated and goes live at its first heartbeat.
- [x] C5: the identity registry holds a `principals` array and the four role templates (`fleet-owner`, `org-admin`, `operator`, `auditor`) exist as named grant sets; grant/revoke is a journaled governance act; an org-admin altering the owner's grants or the `fleet-owner` principal's own grants is refused by name; a run's principal resolves from registry/device context with owner as the unchanged default, and a delegated child inherits its parent's principal.
- [ ] C6: the armada lens is the bridge console: instance cards show heartbeat age, capacity, settled reliability and corpus-sync state; the deploy flow (provider → target → step-up → journal tail), roles panel and audit stream work; a refused action names the guard's reason on the surface.
- [x] C7: every `destructive`/`governance` act (deploy, version-roll, retire, grant, revoke) demands fresh human proof the agent cannot satisfy — the seat on the face, TOTP from the vault's crypto on CLI/MCP — and each check settles an audit event naming principal, verb and outcome.
- [x] C8: `fleet armada mcp` serves the ten control verbs as MCP tools over stdio through the same `control::admit`; an ungranted call is refused by name; a `fleet mcp` client invokes a read verb end to end from config alone.
- [ ] C9: a single-owner home with no roster, one principal and no non-owner grants behaves identically to 0.8.20: the full four-leg gate passes with the armada features dormant by default, and the retired `vps_commands` print path is gone from the CLI.
- [ ] C11: the shared mind's transport closes the three classes that have none (owner ruling 2026-11-19: *"gitriver for keeping multiple instances of fleet in sync in terms of the corpus of memories, skills, tools, workflows, in-flight work"*). Measured: `memories/`, `roles/` and `plugins/` are ALREADY git-tracked in the repo (681 / 7 / 4 files) and a home holds only its own private state — so the transport is real, not aspirational. What has none: the canonical skill library (467 skills at the user home, not a git repo), `mcp.json` (the tool registrations, home-only), and workflows (no store). The work is therefore: a declared `Tracked`/`Private` split in `corpus` with `refuse_private` naming the class it refuses; the canonical skills library becomes a tracked class (submodule or a synced subtree, chosen by whether the library is the owner's cross-harness one or fleet's own); `mcp.json` joins the tracked set; a sync reports per class, so a class that could not move is named rather than hidden inside a whole-repo success; the exclusion (identity, grants, vault, roster, sessions, cron, devices) is pinned by a test that asserts a private path is refused by name; and GitRiver is the documented host — a self-hosted Rust git platform that also carries an OCI registry, which is what a container instance needs — with plain GitHub or any git remote working unchanged because the contract is git, not a vendor.
- [ ] C10: `docs/features/armada.md` describes the console, the templates and the container contract as built; the armada ledgers carry D58/D60/D59 and the refreshed atoms; `TECHNICAL-DOCUMENTATION.md`'s identity/security/armada sections match the shipped seams; the harness pins the console and the lib tests pin authority, admission, deployment and joins.

## Files to be touched

**Packaging:**
- `Dockerfile` — new: the instance image (binary, services, runtime, unprivileged user, `FLEET_HOME=/data`).
- `scripts/release.sh` — the linux build target and the image build/publish leg (after tagging: build the linux binary, `docker build`, push).
- No new workflow file — the repo's CI is deliberately unarmed (`workflow_dispatch` only, owner decision 2026-10-13); the release script is the publisher, as it already is for darwin.

**Backend:**
- `src/armada/join.rs` — new: the env-driven instance entry (bootstrap, register, heartbeat loop).
- `src/armada/deploy.rs` — the provider adapters (`openship`, `docker`), the service-spec contract, per-step journal events, dry-run; retire the `vps_commands` print path.
- `src/armada/mcp.rs` — new: the served stdio endpoint; `src/mcp.rs` framing reuse.
- `src/security.rs` — the role-template store, grant-from-template, the org-admin boundary.
- `src/identity.rs` — the `principals` array, context-resolved `principal_for`.
- `src/cli.rs` — `armada join|deploy|grant|revoke|mcp`; the step-up TOTP leg.
- `src/gui/bridge.rs` — `armada()` extended (heartbeat age, sync state, audit), `armada_deploy`, `armada_grant`, principal threading.

**Face:**
- `src/gui/server.rs` — the new routes.
- `src/gui/assets/app.js` / `index.html` / `app.css` — the console sections (roster cards, deploy flow, roles panel, audit stream).

**Tests:**
- lib tests in `src/armada/*`, `src/security.rs`, `src/identity.rs` (authority, admission, providers mocked, joins, templates, boundaries);
  `tests/face/harness.mjs` (console cases);
  `tests/integration.rs` (the MCP endpoint loop, CLI verbs).

**Docs:**
- `docs/plans/ARMADA-CONTRACTS.md`, `ARMADA-DECOMPOSITION.md`, `ARMADA-DESIGN-DECISIONS.md` (D58/D60/D59),
  `docs/features/armada.md`, `docs/features/identity.md`, `docs/features/security.md`,
  `TECHNICAL-DOCUMENTATION.md`.

## Out of scope

- A second binary or GPUI shell — resolved by D58; packaging changes (a
  plugin-host carrier or a second tauri target later) touch no body code.
- The ssh command-drop — retired by D60 (the container contract replaces it);
  an ssh provider adapter is recorded as a future provider, not built.
- The instance running the desktop/face — instances are headless; a
  per-instance browser door rides the existing pairing/relay later.
- Team collaboration surfaces for humans (lane-2 O5: shared docs/chat/mail) —
  the org here is authority, not collaboration.
- Autonomous dispatch scheduling or a placement scorer — explicit targeting
  per D46; autonomy is earned, later.
- Multi-tenant corpus sharding beyond section-scoped grants; billing or any
  SaaS-shape; QUIC/Tailscale transport upgrades (recorded in §10.1, not built).

## Verification

- **C1 (`== C1 ==`):** the ledger read-back —
  `grep -n "verified at" docs/plans/ARMADA-CONTRACTS.md` prints 0.8.20;
  three spot-checks pin cited seams live:
  `rg -n "pub fn resolve_root_from" src/memory.rs`,
  `rg -n "pub struct Grant" src/security.rs`,
  `rg -n "pub fn register" src/armada/roster.rs` — each print matches its row's citation.
- **C2 (`== C2 ==`):** the container proof —
  `docker build -t fleet-instance .` exits 0;
  `docker run --rm fleet-instance fleet --version` prints the build's version;
  `docker run --rm fleet-instance fleet health` names the lanes as resolved in-image;
  `docker run --rm fleet-instance sh -c 'echo $FLEET_HOME'` prints `/data`;
  `docker history fleet-instance` shows no layer or env carrying a secret value;
  `grep -n "linux" scripts/release.sh` names the new build target and the publish leg it feeds.
- **C3 (`== C3 ==`):** the join proof —
  run the image with `FLEET_CORPUS_REMOTE`/`FLEET_FLAGSHIP_PUBKEY`/the join secret set:
  `fleet armada roster` at the flagship lists the new sid marked un-calibrated;
  stop the container and run it again: the log names the already-bootstrapped refusal
  (`grep -c "already exists" <container-log>` ≥ 1) and `fleet identity` prints the same install-id;
  run it with one env var unset: the boot refuses by name and exits non-zero before any write.
- **C4 (`== C4 ==`):** the deploy proof —
  `fleet armada deploy --provider openship --dry-run --target <project/service>` prints the service spec
  (image, named volume, env keys) and writes nothing;
  the real run settles `provision` events:
  `curl -s localhost:$PORT/api/armada/journal | jq '.events[] | select(.kind=="provision")'`
  lists the steps with their real outcomes;
  kill the run mid-sequence: `fleet armada status` names the step that failed;
  `fleet armada roster` shows the instance `live:false` before its first heartbeat and `live:true` after.
- **C5 (`== C5 ==`):** the identity and template proof —
  `cargo test -p fleet --lib identity` (the principals array, context resolution,
  child inheritance, the owner default) passes;
  `cargo test -p fleet --lib security` (template store, journaled grant/revoke,
  the org-admin boundary refusal) passes; the refusal text pins R3 by name.
- **C6 (`== C6 ==`):** the console proof —
  the harness cases: roster cards with heartbeat age from a stalled instance, the deploy
  flow reaching the seat, grant/revoke through the roles panel, the audit stream rendering
  refusals; live in obscura at `#/armada`: a refused dispatch shows the guard's named reason.
- **C7 (`== C7 ==`):** the step-up proof —
  face: deploy/grant/retire presses raise the seat (harness-pinned);
  CLI: `fleet armada retire <sid>` without fresh TOTP exits non-zero with a named refusal;
  MCP: a `retire` tool call without the assertion is refused by name;
  each attempted check lands in the audit stream with principal + verb + outcome
  (pinned by the journal assertions in `cargo test -p fleet --lib armada`).
- **C8 (`== C8 ==`):** the served endpoint proof —
  `fleet mcp add fleet-armada -- fleet armada mcp` then
  `fleet mcp call fleet-armada roster_read '{}'` returns the roster document end to end;
  `fleet mcp call fleet-armada retire '{"sid":"x"}'` for an ungranted principal is refused with
  the guard's named reason; admission through `control::admit` is pinned by the armada lib tests.
- **C9 (`== C9 ==`):** the not-breaking proof —
  on a pristine home the full gate passes: `cargo build --all-targets`,
  `cargo test -- --test-threads=1`, `python3 scripts/ki_ledger.py`, `python3 scripts/plan_check.py`;
  `grep -rn "vps_commands" src/` returns nothing;
  `fleet armada deploy --help` names providers, not ssh hosts;
  `fleet help` lists `join|deploy|grant|revoke|mcp` and no ssh-drop verb.
- **C10 (`== C10 ==`):** the docs proof —
  `docs/features/armada.md` names the console, the containers and the endpoint as built;
  `grep -n "D58\|D60\|D59" docs/plans/ARMADA-DESIGN-DECISIONS.md` finds all three appended;
  `TECHNICAL-DOCUMENTATION.md`'s identity/security/armada sections cite the shipped seams;
  the harness cases and `cargo test -p fleet --lib armada` cover the surfaces named.

## Linked artifacts

- `docs/features/armada.md` — rewritten: the bridge console, the templates, the container contract, the served MCP endpoint (C4/C6/C8/C10).
- `ARMADA-CONTRACTS.md` — seam refresh to 0.8.20 + the provider seam (C1).
- `ARMADA-DECOMPOSITION.md` — lane-2 O1/O2/O4 done; L9's deploy row rewritten to container provisioning; the join/mcp atoms added (C2–C8).
- `ARMADA-DESIGN-DECISIONS.md` — **D58** (one binary), **D60** (the instance is a container; ssh-drop retired), **D59** (the control vocabulary served over MCP) appended (C2–C8).
- `docs/features/identity.md`, `docs/features/security.md` — principals array, role templates (C5).
- `TECHNICAL-DOCUMENTATION.md` — identity/security/armada/packaging sections (C2, C10).

## Risks

- **The linux build is new territory** (the feed asserts darwin-only today). Mitigation: CI builds the image on tag so the desktop pipeline is untouched; the linux binary is validated by the container's own smoke test (C2).
- **Provider token custody.** The OpenShip token lives in the vault, resolved at deploy time, never logged (the vault's own discipline); a missing token is a named refusal.
- **Join-secret custody.** The per-deployment secret rides env into the container and is never baked into the image; rotation is the flagship re-deploying with a fresh secret (a destructive-class act).
- **`principal_for` is a constant today.** Threading real resolution is the widest surface; the default path (no registry entry) *is* the owner, so existing callers are unchanged — pinned by C9.
- **Roles conflation.** Org templates (authority) and persona overlays (delegation) stay two concepts in two stores; C5 pins the boundary so neither grows the other's powers.
- **Ledger rot recurrence.** The seam stamp names its verification version, so the next drift is detectable rather than silent.

## Dependencies

- L1–L8 are built and composed, not re-implemented; the workspace graph (D37), the roles store, and the corpus sync are consumed as-is.
- The OpenShip CLI must be on the flagship's PATH for the openship provider (docker leg needs an engine only); the vault holds its token.
- The relay + pairing supply the transport and the join-secret pattern the container boot reuses.
