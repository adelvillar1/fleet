---
status: completed
created: 2026-11-09
updated: 2026-11-10
slug: failsafe-restore
---

# Plan: the failsafe — the chain restores fleet to working order

## Context

The 2026-11-09 incident (KI-493, KI-492, KI-494) ended in refusals: two
delegated children died at LayerFS workspace open (Docker's store empty,
the pinned worker container gone), the failed chat carried `lane_failed`
noise for two dead MCP lanes, and nothing surfaced the substrate's death
until a delegation needed it. The owner's ruling, consolidated across
five statements: **the main session and its delegation chain are the
failsafe** — in the presence of a failure their goal is restoring fleet
to working order, never refusing work. Concretely: fleet needs a
diagnostic loop that knows a service is down and says so; a critical
service down (LayerFS-class) means the child skips the routing machinery
and runs on the main session's default model; any refusal, failed call,
or spent quota defaults back to the main session's chain; and the main
session's responsibility on a sys1, sem1, layerfs, or any service failure
is to keep working AND attempt the repair. The ultimate goal is **self-
healing**: there is always a mode where the main session can bring fleet
back to working order — reverting a change that broke it, riding out a
model failure — and the only true unrecoverable failure is no
connectivity with no local model set up: without a configured local
model, no network means no intelligence to heal with.

Ground truth read this session: the child's two refusal sites
(`src/subagents.rs:2048` route-prepare, `:2216` workspace-open — the
incident's death), the existing degraded seam (`own_route = None` passes
the parent's client/model through — the unrouted-parent arm), the walk's
failover (exhausted list fails the run by name, `src/route.rs:1874`), the
doctor's check set (`src/doctor.rs`: credentials/home/api/disk/shell/
layerfs), the lane health probes (`src/lanes.rs` health_tools), and the
server's existing background-task and SSE-heartbeat seams
(`src/gui/server.rs:3381`, spawns at `:212`, `:3988`, `:4019`).

## Decision-model check

The probe verdicts are deterministic (a TCP connect, a file's presence,
a port's answer) — no per-item judgment, so no sys1 job; the judgment
beat already lives where it belongs, in routing. One line, as the
template asks, instead of an unnamed absence.

## Approach

One recommended path, in three phases that each stand alone:

### Phase A — the diagnostic loop (know and say)

**A — the diagnostic loop (know and say).** A new `src/health.rs`: a
probe set over the critical services — layerfs (a connect to the
endpoint's persisted port; refused = down), each configured lane's cheap
presence (its venv/launch spec exists), the routed API's reachability —
tiered so the fast tier costs milliseconds (no MCP spawns; a lane's full
health probe stays on-demand), plus the **repair map**: one table naming,
per service, its repair verb and command (`fleet install` for the
layerfs substrate and for a missing lane venv; owner-action wording for
what cannot be self-repaired). The gui server runs the loop on a tokio
interval beside its existing spawns, the bridge carries the report in
`/api/state`, a transition event announces down→up and up→down, and the
face paints a service card in the `conn-down` idiom: what is down, since
when, and the repair.

### Phase B — degradation over refusal (keep working)

**B — degradation over refusal (keep working).** The child's two
refusal sites become degradations: a workspace that cannot open clears
the opt-in for that run and the child runs on the main session's model
and route with the plain launch folder (the pre-LayerFS delegation
shape; isolation traded for availability, named in the row); a
`prepare_child` failure sets `own_route = None` and the child rides the
parent's route. The main session's own route preparation degrades the
same way: a broken routing config falls back to the compiled-in declared
order with the degradation named in the decision row and on stderr —
never a silent swallow of a config typo. Every degradation is model-
visible and logged (hard rule 5): the row, the result, and the transcript
name what degraded and why.

### Phase C — the repair attempt (restore)

**C — the repair attempt (restore).** When the loop names a service
down or a use-time failure lands (`TurnFailed`, an exhausted chain), the
session carries a repair note naming the service, the cause, and the
repair verb from the map — logged as a record and visible to the model.
The repair itself is the main session's own bash through the standing
approval gate (ask mode asks, as every command does today); an exhausted
routing chain fails by name carrying each candidate's cause and the
map's word for repairable causes. The loop is bounded: one repair
attempt per down-transition, never a retry storm.

### Phase D — the recovery mode (revert what broke)

**D — revert what broke.** Self-healing needs an undo at every layer a
change can break, and one verb that drives them. The layers, each with
its known-good seam: the settings writer (`src/settings.rs`) stamps a
last-known-good copy of config.json / routing.json / fleet.env beside
each write; an applied improvement keeps its undo
(`fleet improve undo` exists — the journal is the seam); the MCP
registry's last-enable is reversible (disable); the updater's `.prev`
rollback covers a bad binary. `fleet recover` names the broken layer
(the health probes' own verdicts), restores that layer's last known
good, and re-probes — the main session's one command for "bring fleet
back". And the roster's tail grows a final rung behind the deepseek
tail: a **configured local model**, so the chain's last resort survives
no connectivity. Without one configured, that floor is the named true
failure, and every failure text says exactly that.

High-level steps:

1. `src/health.rs` — the probe set, tiers, repair map, `HealthReport`, tests.
2. The server loop + bridge state + transition event; `/api/state` carries it.
3. The face's service card (app.js/app.css), harness case with a dead port.
4. `src/subagents.rs` — the two degradation sites, notes in row and result.
5. `src/route.rs` — the config-refusal degradation to the compiled-in order.
6. The repair note seam (agent.rs) + the exhausted-chain cause list.
7. `src/settings.rs` known-good stamps + `fleet recover` + the local-model tail rung in `src/route.rs`.
8. Docs and ledger rows (KI-493's code side closes here).

## Acceptance criteria

- [x] C0 — `GET /api/state` carries a `services` block refreshed by a server-side probe loop, and the face paints a card naming every down service beside its repair verb; the harness proves it with a dead port.
- [x] C1 — a delegation whose LayerFS workspace cannot open runs to completion on the main session's model and route with the plain launch folder, its row and result naming the degradation.
- [x] C2 — a child whose `prepare_child` fails runs on the main session's model and route with the degradation named in its result, so no child is refused for a route-preparation error.
- [x] C3 — a main-session route whose preparation fails degrades to the compiled-in declared order with the degradation named in the decision row, and the run proceeds.
- [x] C4 — when the loop or a use-time failure names a service down, the session carries a logged note naming the service, the cause, and the repair verb, and the model sees it in its conversation.
- [x] C5 — an exhausted routing chain fails by name carrying each candidate's cause and, for a repairable cause, its repair verb.
- [x] C6 — with a dead substrate under a scratch home and a yes-mode approval, a run's repair note leads the main session to attempt the mapped repair through its own bash tool, and the log carries the attempt and its outcome.
- [x] C7 — a change that breaks a subsystem (a malformed routing.json, a failing applied improvement) is reverted by `fleet recover` to that layer's last known good, and the health probes re-probe green in a scratch home.
- [x] C8 — a configured local model sits as the roster's final candidate behind the deepseek tail and serves when connectivity is gone; with no local model configured and no connectivity, the failure text names that state as the one unrecoverable failure.

## Files to be touched

**Backend:**
- `src/health.rs` — new: probes, tiers, repair map, report.
- `src/gui/server.rs`, `src/gui/bridge.rs` — the loop, state, event.
- `src/subagents.rs` — the two degradation sites.
- `src/route.rs` — the config-refusal degradation; the local-model tail rung.
- `src/agent.rs` — the repair note seam and exhausted-chain causes.
- `src/settings.rs` — the last-known-good stamps.
- `src/cli.rs`, `src/main.rs` — `fleet recover`.
- `src/lib.rs` — the module line.

**Frontend:**
- `src/gui/assets/app.js`, `src/gui/assets/app.css` — the service card.

**Tests:**
- unit tests beside each module; `tests/face/harness.mjs` — the card case.

**Documentation:**
- `docs/features/delegation.md`, `docs/features/` health surface note,
  `TECHNICAL-DOCUMENTATION.md` (the repair runbook),
  `FUNCTIONAL-SPECIFICATIONS.md`, `docs/known-issues.md` (KI-493 code
  side), `CHANGELOG.md` at the cut.

## Out of scope

- Third-party MCP repair (fli's Python env) and registry hygiene — KI-494's own row.
- Full MCP lane health probes in the fast tier (spawn cost); cheap presence only, deep probes on demand.
- Auto-approved repairs — the standing approval gate governs every repair command.
- The owner's actual `docker pull` (KI-493's provisioning) — this plan makes the refusal name the repair; the pull stays the owner's act.
- Provider quota top-ups — owner action, named not attempted.
- Setting a local model up (installing ollama or any local runtime) — the owner's act; the plan makes the roster honor one once configured.
- KI-492's folder-event silence — its own row, its own three-line fix.

## Verification

**C0** — the harness case (a dead port in a scratch home) and a live read of the running face, whose Docker store is empty today and is therefore the standing fixture.
**C1/C2** — unit tests: a gate that refuses (dead port) and a route prepare that errors, each followed by a child that runs and a result whose degradation names the cause.
**C3** — a unit test pointing the config read at a malformed routing.json; the run proceeds on the compiled-in order and the decision row names the degradation.
**C4** — the log carries the note record; the conversation assertions see it.
**C5** — a route-walk test over an all-failing order; the failure text lists each cause and the map's verb.
**C6** — scratch home + mock API: the model's bash call runs the mapped repair verb, the log carries the attempt; the standing gate (fmt, clippy, build, test) green end to end.
**C7** — a scratch home seeded with a malformed routing.json and a failing improvement apply: `fleet recover` restores the stamped last known good for each and the probes read green; the stamps are asserted on every settings write.
**C8** — a roster test with a local candidate configured: it serves after every hosted candidate fails; without one, the exhausted-chain failure text names the unrecoverable state.

## Linked artifacts

- `docs/features/delegation.md` — the degraded child: no-workspace runs, parent-route fallback.
- `docs/features/surfaces.md` or a new health-surface note — the service card and the loop.
- `TECHNICAL-DOCUMENTATION.md` — the repair runbook: the map, the verbs, the tiers.
- `FUNCTIONAL-SPECIFICATIONS.md` — the surfaces' degradation behavior.
- `docs/known-issues.md` — KI-493's code-side nicety closes here.

## Risks

- Degraded children edit the host tree directly (no branch isolation) — the trade the ruling names; every row and result says so.
- C3 must never swallow a config typo silently — the degradation is loud (stderr + decision row), and a named note rides the session.
- The repair attempt must be bounded (one per down-transition) so a flapping service cannot loop the chain.
