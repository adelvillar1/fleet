# The control plane — the hand (armada L8)

The layer the whole stack below exists to serve. A flagship — a fleet install
in a flagship **role** (role + config, one binary, never a fork) — commands
instances over the relay. Authority rides *in the frame*, signed by the
flagship's Ed25519 key and verified at the instance's guard before it obeys;
the relay stays a dumb broker (L6). An instance earns dispatch autonomy by
measured reliability, never by registration. `src/armada/` composes every
lower contract and re-implements none.

## Registration + roster + heartbeat (D44)

`roster.rs`: an instance registers on join with its install-id (L1), sid,
version, and capacity (the contract's reading). The flagship roster lives at
`<home>/roster.json`. **Liveness is earned by heartbeats, not assumed** — a
sid with no heartbeat within the window is reported not-live. A heartbeat
cannot register an unknown sid (that is the join's gated act). An install with
no roster is an ordinary single-instance fleet, unchanged.

## The authority transport (D45)

`authority.rs`: every control frame carries the principal and, for
`destructive`/`governance` verbs, a fresh assertion (L3) **signed by the
flagship's Ed25519 key**. The instance verifies the signature *before* it
obeys — a forged, unsigned, stale, or principal-mismatched assertion is
refused by name. The relay cannot fabricate (it holds no key), an instance
cannot escalate (it verifies against the flagship's public key), and an agent
cannot borrow (the private key lives only at the flagship).

**Key custody (D47):** the flagship mints the Ed25519 key once, persists the
PKCS#8 `0600` at `<home>/armada/flagship-key.pkcs8` and the raw public key at
`flagship-key.pub`. An instance holds only the public key (distributed
out-of-band) — it can verify, never sign. Rotation/revocation is an open fork
flagged for the owner.

## The control vocabulary (D46)

`control.rs`: each verb reuses a bridge verb and runs through the instance's
own guard (D15) and admission, after authority is verified:

| Verb | Reuses | Class |
|---|---|---|
| `dispatch` | `bridge.run` (gated dispatch, L5) | Dispatch |
| `state` | `bridge.state` | Read |
| `corpus-sync` | L4 sync | Write |
| `version-roll` / `retire` | deployment (L9) | Destructive |
| `health` | the health probe | Read |
| `register` / `heartbeat` | the roster | Governance |

An ungranted verb is refused by name and settles a `guard` journal event; a
privileged verb also clears step-up (D18) — the human second factor issues the
fresh per-action assertion the agent cannot forge (D48). The flagship cannot
overwhelm a small VPS: the instance admits for itself.

## Autonomy earned by measurement (D49)

A fresh instance is **un-calibrated until it settles runs**. A scheduled
reliability beat grades dispatch outcomes over the journal (never a
synchronous sys1 ask — the socket guards); a dispatch is graded against its
outcome before the pattern changes.

## Relay-down is a named degradation (D50)

Instances run autonomously, the corpus still syncs over git (L4 is independent
of the relay), and live control resumes when the relay returns — never a
silent stop.

## Surface

`fleet armada {roster|status|sync|deploy|dispatch}` drives the control plane
headlessly: the roster with earned liveness, an instance's measured
reliability, the corpus sync, provisioning a fresh install, and dispatching a
task through the guard.

## Deployment — fleet deploying fleet (L9, D51/D52/D57)

`deploy.rs` provisions a target with the self-contained binary (hard rule 12),
no foreign control-plane service. A **local spawn** lays out a new home, mints
a fresh install-id (L1), and bootstraps the corpus by cloning the remote —
inheriting the mind, not the identity (D29). A **VPS** is reached over SSH and
dropped the same binary from the release raw-binary channel; the command
sequence is pure (credentials resolve out-of-band via the SSH agent/env, never
the corpus). The fresh install registers with the flagship roster (L8) and is
**trusted only after it passes doctor and settles runs** — graded by
measurement, never by arrival (D57). Provision is a governance action
(step-up, D18/D19).

## The surface progression (L9, D53–D56)

One `src/armada` library, four surfaces — fleet's proven progression reused,
not a parallel app:

- **CLI** — `fleet armada …` (terminal-first).
- **TUI armada board** — a ratatui pane: instances with a colored liveness dot,
  version, capacity (GiB) bars, and a reliability gauge (calibrated → percent,
  un-calibrated → named). Genuinely visual, not a plain-text table.
- **Face armada section** — the roster board, a dispatch composer, and a corpus
  view in the centre-conversation + context-pane shape; the chat composer is
  borrowed, never rebuilt. `GET /api/armada` + `POST /api/armada/dispatch`.
- **fleet-desktop** — wraps the same face over the same server/bridge: no
  second frontend, no second transport. The section appears automatically.

## Verification

`cargo test -p fleet --lib armada` pins registration/liveness, signature
verify + forge/unsigned/stale refusal, key custody (flagship only, 0600), verb
classes, admission gating + audit, the second-factor step-up, corpus-sync
reaching L4, the reliability grade (un-calibrated until settled), relay-down
named, the no-roster install unchanged, and deployment (corpus inheritance,
fresh-install registration, the doctor+settles trust grade, the VPS command
sequence). The TUI render test pins the armada board; the face harness
(`tests/face/harness.mjs`) pins the armada section. The full gate confirms a
single-instance fleet behaves as today.

## As-built 2026-11-19 — the bridge console, containerized instances, and the served endpoint

The 0.7.0 section above describes what shipped then: a read-only roster lens
and a dispatch composer. This section records what the bridge-console plan
changed, and the two are not the same product.

**Instances are containers (D60).** `Dockerfile` builds a linux image —
multi-stage, so the binary compiles inside a linux stage (`ring` cross-compiles
badly) — 122 MB, unprivileged, `FLEET_HOME=/data` over one named volume, the
service lanes beside the binary so `lanes::services_root()` resolves with no
environment. Two facts learned by building rather than reading: the workspace's
`default-members` includes `src-tauri`, whose GTK dependency an instance can
never satisfy, so the build is `-p fleet`; and `cpal` links `libasound`, so the
`-sys` crate needs headers at build time AND `libasound.so.2` at runtime — the
first image booted to "error while loading shared libraries".

**The join (C3).** `fleet armada join` is the container's entry point:
environment only, no prompts, every refusal named before any write. It clones
the shared corpus, mints-or-reads the install-id, registers on the roster
un-calibrated, then heartbeats. A restart is idempotent by `bootstrap_corpus`'s
own fresh guard — verified in a real container: `corpus_cloned=false`, the
install-id byte-identical, the shared fact present on the volume.

**Deploy through providers (C4).** `vps_commands` is retired: it returned ssh
command strings nobody executed. In its place, `ServiceSpec` (image, one named
volume at `/data`, the corpus remote) is pure data, so a dry run and a real run
cannot disagree, and `preview()` prints env KEYS only — a deploy log must not be
able to leak join material. `Provider::{Docker, OpenShip}` execute, each step
settling a real `provision` journal event; a failed step names itself and stops.

**The served control endpoint (D59, C8).** `fleet armada mcp` serves D46's ten
verbs as MCP tools over stdio. Every call passes the same guard as a relayed
frame: `control::admit_local` shares `admit_principal` with `admit`, so the
endpoint cannot become a second permission system. Destructive and governance
verbs refuse by name for lacking fresh human proof, which an agent can never
supply — verified over a real pipe.

**Roles as grant templates (C5).** `security::RoleTemplate` — `fleet-owner`,
`org-admin`, `operator`, `auditor` — expands into rows over the same grant
vocabulary, so the guard needs no new code path. `refuse_owner_tampering`
refuses the owner principal and a second `fleet-owner` BEFORE reading the store,
which pins R3's actor-blind boundary in code rather than prose.

**The console (C6).** `#/armada` now carries the roster with heartbeat AGE (not
just a live boolean), the corpus's sync state, the guard audit stream (the
journal's own `guard`/`provision`/`governance`/`dispatch` events), the deploy
card, and the roles panel reading `/api/armada/roles`. The lens is a row on the
existing rail, annotated in the expert census — a console, not a second app.

**The shared mind's transport (D61, L4.5).** All three classes the direction
note names now travel, and a fresh instance is as capable as its siblings:

| class | transport | verified |
|---|---|---|
| `memories` | corpus sync (L4) | the fact tree merges and pushes |
| `skills` | its own repo, cloned at boot | `skills_cloned=true (N loaded)` |
| `mcp.json` | its own repo, merged at boot | `tools: N inherited, M new` |

The tool registry's merge is deliberately asymmetric, and the asymmetry is the
security boundary: a server DEFINITION this instance lacks arrives **disabled**
(a shared file must not decide what a machine runs), while a server it already
knows keeps its **enablement and probe history** (those describe this machine,
not the fleet) even as the definition refreshes from upstream. The schema
stores env NAMES with no field a credential could occupy, which is why the file
is safe on a volume at all.

`corpus::CLASSES` reports each class with its real transport, and
`corpus::PRIVATE` + `refuse_private` keep identity, grants, the vault, the
roster, sessions, cron and devices out of the shared set by name. GitRiver is
the documented host — self-hosted Rust git with an OCI registry beside it,
which is what a container deploy needs — with any git remote still working.

## As-built 2026-11-22 — tuios in the deployment map

The owner runs **tuios 0.13.3** as their desktop shell, and Fleet's TUI runs
inside it as a daemon-kept pane (`tuios attach fleet-tui`). The deployment map
now carries that as one more thing an instance holds: `src/tuios.rs` reads
tuios's own JSON control protocol (`tuios call <verb> [params] [--server NAME]
--timeout 4s --json`, control protocol v1) and the map's "what it carries" cell
renders it.

**Four read verbs, and no others.** `list-sessions`, `list-windows` (once per
session — with no session it answers for the most recent one, which would
attribute one session's windows to another), `list-agents` (`all`), and
`list-attention` (its `total`). `add-task`, `send-keys` and every verb that
changes the multiplexer are never called: Fleet watches the owner's shell, it
does not drive it.

**The AGENT words pass through verbatim** — `working`, `needs_input`, `idle`,
`done`, `errored`, `unknown`, `limit_reached`, `none` — counted per session and
printed as the daemon said them, never mapped onto Fleet's vocabulary. The map
shows, it does not judge: `agent-activity` is deliberately not read, because
tuios marks it untrusted itself.

**A fleet pane is matched by the live command line.** A pane runs Fleet when
`foreground_cmd`, or the first word of `running_cmdline` (which `list-windows`
documents as present *while a command runs*), is the fleet binary by basename —
`fleet`, `./target/debug/fleet`, a path ending in `/fleet`. History
(`last_cmdline`), the directory (`cwd`) and the pane's text are not evidence:
Fleet's own frame says "fleet" on every row, so a capture would match any pane
showing the docs, and a shell that ran `fleet tui` an hour ago is a shell.

**The refusal is named, never an empty grid.** Exit 3 is `tuios is not running
on <sid>` with the daemon's own words after the dash; a missing binary is
`not installed`; a verb error is the daemon's code and message; an unreadable
answer says so. The daemon's multi-line transport message is cut to its first
line, because a row is one line. An empty session list is a measurement; an
unreachable daemon is not.

**Per-instance opt-in, same-document.** `<home>/tuios.json` names the sids whose
machine runs the daemon (`{"instances":{"<sid>":{"enabled":true,"server":""}}}`);
an instance the owner has not enabled costs no call and carries no section, and
the card names that absence once in the section's own sentence. Every word in a
section is the daemon's answer for *this* read — nothing is cached or
recomputed.

Verified against the real daemon on this machine (`tests/tuios.rs`, which skips
by name where tuios is absent): the four verbs' shapes, the dead-socket refusal
(a named server with no socket, exit 3), the live specimen (session `fleet-tui`,
pane `Terminal cb1fe908`, command `fleet`), and the armada document carrying the
section only after the opt-in. The face harness pins the card: the sessions, the
AGENT words verbatim, the marked fleet pane, the refusal, and the absence
sentence — deleting that sentence fails its case by name. Measured while
building: a `--server` name is capped at 14 characters by the CLI itself.
