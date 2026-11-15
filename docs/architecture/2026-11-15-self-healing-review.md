# The self-healing review — every failure mode, and what could heal itself

Review, 2026-11-15. The substrate heals itself now (the 2026-11-15
extension: auto-start the worker, wake a sleeping runtime, offer to
install kiri). This review walks every failure mode across fleet,
classifies the healing that exists today, and names the candidates for
**autonomous** vs **owner-approved** self-healing — so the next healing
slices have a ranked map instead of a fresh audit each time.

## The four classes

- **AUTONOMOUS** — the app detects and fixes without asking (background
  loop, built-in retry, silent fallback). Safe only for idempotent,
  reversible, non-destructive repairs.
- **APPROVED** — ask-then-act: a card/route exposes a press, or a run
  pauses for the owner's word. For consequential or irreversible actions.
- **MANUAL** — a command is named but nothing in-process performs it
  (`fleet install`, `fleet recover`, `fleet reload`, a script).
- **NONE** — detection and/or healing absent (a silent failure).

## Today's healing, by area

| Area | Heals today | Class |
|---|---|---|
| Substrate worker dead | health loop auto-runs `layerfs_step` (guard + 60s cooldown) | autonomous |
| Stale endpoint record | `reconcile_stale_record` rewrites the port | autonomous |
| Runtime asleep | `wake_runtime` opens Kiri/Docker | autonomous |
| No runtime installed | card offers "Install kiri"; `/api/repair/install-runtime` downloads + opens the dmg | approved |
| Provider failure / quota / 5xx / timeout | failover walk advances; 402 unlocks the walk; 3-day demotion; deepseek tail | autonomous |
| Route cache stale/malformed | keyed check, discard + recompute | autonomous |
| Embedder absent/flapping | keyword-only degradation; index rebuild; 4 paused retries | autonomous |
| Torn session-log tail | drop the torn final line, resume | autonomous |
| Unreadable fact/log | skip + continue | autonomous |
| Checkpoint bloat | prune every 15 min, keep newest | autonomous |
| Cron catch-up / claim / panic / record retry | one catch-up run; atomic claim; loop survives | autonomous |
| Relay down / poll fail | rejoin every 2s; one retry | autonomous |
| Pairing expiry / wrong code | re-mint; self-burn at zero attempts | autonomous |
| Corpus merge conflict / empty read-back | abort + reset to local HEAD | autonomous |
| Update artifact wrong sha / won't run | refuse before staging | autonomous |
| Disk near reserve | refuse the run before the first turn | autonomous |
| Reserve encroached mid-run | **pause and ask** to permit past the reserve | approved |
| Worker won't auto-start | the manual **Repair** press | approved |
| Lanes build / venv | `fleet install` names it; no GUI repair | manual |
| Release cut | `scripts/release.sh` | manual |
| Cache bloat | `scripts/cache-sweep.sh` / `daily-maintenance.sh` (not invoked from `src/`) | manual |

## The healing candidates — ranked

### Autonomous (idempotent, reversible, non-destructive — the app acts)

1. **API key present-but-invalid.** `api_probe` only checks non-empty
   (`health.rs`), so the card says `api up` while a 401 waits for the first
   run. Probe with one tiny real request on the slow tier; a 401/402 sets
   the card down with the named cause. The run already fails over — this
   just surfaces it before the owner waits.
2. **MCP server health in the loop.** A configured-but-broken MCP server is
   invisible until a run tries it (`mcp_registry::probe_server` runs only on
   demand). Probe configured servers on the slow tier; a failed server is
   *disabled for the run* (reversible) and named.
3. **Lane health beyond presence.** The 30s card only stats
   `mcp_server.py`; a lane whose venv can't serve still reads healthy. A
   light `initialize` handshake on the slow tier flips it to the real state
   (`/api/lanes` already knows how).
4. **Checkpoint manifest corruption.** `checkpoints::list` silently drops an
   unreadable manifest. Surface a named refusal for the turn; a re-capture
   is safe (the manifest is derived).
5. **ENOSPC in-process.** No `ENOSPC` detector in `src/`; writes fail per
   operation and reclamation depends on external scripts. On an `ENOSPC`
   from a writer, run `scripts/cache-sweep.sh` once and name it.
6. **Cron job failure retry.** A failed job's window is spent, no retry, no
   health line (only an inbox row). A bounded retry-once on the next tick.
7. **Scheduler thread death.** The cron tick thread dying is only a stderr
   line. A supervisor that restarts the loop.
8. **Vault wrong-passphrase at launch.** `read_dotenv_with_vault` silently
   falls back to the plaintext map with no log. Name the absence (the
   sealed keys were not opened) — a detection, not a fix.
9. **Device store corruption.** An unparsable `devices.json` 401s every
   device with only a stderr line. It cannot be *rebuilt* (the tokens are
   hashed), but the face should say "re-pair this device" rather than a
   bare 401.
10. **Update auto-rollback.** A bad applied release keeps the previous
    binary (`update.rs`) but nothing restores it; detection is manual
    (`doctor::evaluate_binary`). When the served binary is stale/broken,
    restore `previous` (bounded, reversible) and name it.

### Owner-approved (the app asks; the action is consequential)

1. **Substrate repair** for a worker that will not auto-start — the press
   (already shipped).
2. **Install kiri** when no runtime is present — the press (already shipped).
3. **Rebuild a lane** — a card that offers `fleet install`'s venv leg for a
   specific lane (spends minutes + downloads).
4. **Corpus sync** when the remote moved — a "sync now" offer (a merge can
   conflict; the abort path is safe but the owner should know).
5. **Vault reset / init** — destructive; owner only.
6. **Update apply** — already the press.

### Surface, do not heal (detection gaps → name the failure)

1. **API connectivity** — no periodic probe; a "can't reach the API" line
   from a request-time signal on the card.
2. **Corpus drift** — no probe; a staleness check on read.
3. **Release-delivery on machines without a checkout** — by design absent;
   named as such.

## The line

Autonomous healing is for repairs that are **idempotent, reversible, and
non-destructive**: restart a container, wake a runtime, rewrite a derived
record, re-probe, retry once, restore a previous binary. Owner-approved is
for anything that **changes configuration, spends money, deletes state, or
installs software**. The destructive class — vault reset, corpus force, a
run permitted past its reserve — stays owner-only. Runtime-agnostic stays
the rule: fleet speaks the Engine API, never a vendor.

## The silent failures (no detection and/or no healing)

Ranked by cost: (1) API key present-but-invalid reads healthy; (2) no API
connectivity line; (3) MCP server health off-loop; (4) lane presence-only
health; (5) vault wrong-passphrase silent; (6) corrupt checkpoints vanish
from listings; (7) no ENOSPC detector; (8) corpus drift unprobed; (9) cron
failures with no retry; (10) device-store corruption locks out remotes;
(11) no update rollback; (12) scheduler-thread death; (13) release faults
invisible without a checkout (by design).

## The shape of the fix

Every candidate is the substrate's now-shipped shape applied once: a
detection line in the health loop (or a request-time signal), then one
idempotent action — or an ask, for the consequential class. The loop, the
guard, the cooldown, and the `starting`/`needs_runtime` card states already
prove the pattern; the next slices reuse them.
