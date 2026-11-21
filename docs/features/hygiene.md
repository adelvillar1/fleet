# `fleet hygiene`

The one command that puts the machine back in shape. It is a report first and a
cleaner second: every section prints what it found, including when it found
nothing, so a reader can tell "nothing to do" from "the check did not run".

## The three sections

1. **Checkpoints.** Per-turn rewind snapshots pruned to their retention — each
   session keeps its last `KEEP_TURNS` (20) turns under a `STORE_CAP_BYTES`
   (5 GiB) store cap — and the bytes freed are printed. The manual verb uses
   the same live-safe retention the face runs on its timer, because the plain
   cap would take the newest checkpoint of every session still open: the turn
   its rewind button points at (KI-478).

2. **Memory corpus.** One consolidation pass: expired working facts compressed
   into episodic summaries, stale inferred facts retired. Deterministic, no
   model call. Silence means the corpus needed nothing.

3. **Orphaned processes.** Two families, and they are governed by *different*
   rules on purpose:
   - **The stale-process sweep** stops only what it can prove fleet spawned: a
     command running from the home's own venvs, or the browser lane's Chrome on
     its fixed debugging port, and only once its parent is gone. Everything else
     is named, never killed — the owner's Chrome and the owner's servers are not
     fleet's to stop.
   - **The keep-alive sweep** is broader, because the object is different. See
     below.

## The keep-alive sweep

`tail -f /dev/null` is the keep-alive idiom: a process whose only job is to
exist. When its owner goes away it reparents to launchd (`ppid=1`) and then
holds a process slot, an address space and a file descriptor forever.

Measured on the owner's machine 2026-11-21: **1,232** of them had accumulated
over ~55 days, holding 580 MB and **46 %** of the per-uid process cap
(`maxprocperuid` = 2,666), with ~95 MB of RAM free. The spawner was never
identified — no literal in the Hermes tree, and omh's attach module spawns no
processes at all — which is exactly why the sweep does not depend on knowing
it.

**This half is not restricted to fleet's own processes.** The stale-process
rule does not transfer: a `tail -f /dev/null` carries nothing in its command
line to attribute it, and — more to the point — an orphan has **no waiter**, so
stopping it cannot disrupt anything. The goal is an environment in optimal
shape, not a tidy fleet. So the default is to stop them all, whoever made them.

Two rails keep that safe rather than eager:

- **The command must match exactly** — `tail -f /dev/null`, not
  `tail -f /dev/null.bak`, not `/usr/bin/mytail`, not a trailing `--retry`. A
  substring match is how a sweep kills the wrong thing.
- **The parent must be gone.** A keep-alive that still has a parent is
  somebody's sleep — possibly a supervisor deliberately holding a session
  open — and is left alone.

Attribution is still read, from the process environment (`ps -E`): an orphan
that inherited a `FLEET_`/`HERMES_` namespace is reported as such. It is
**information, not a gate** — the owner wants to know whose litter it was, not
to be blocked by it.

`FLEET_KEEPALIVE_DRY_RUN=1` turns the sweep into a census: it counts and names,
stops nothing.

### What the report looks like

```
hygiene: 3 orphaned keep-alive(s) stopped (1 carrying a harness namespace)
hygiene: 2 orphaned keep-alive(s) found (0 carrying a harness namespace) — census only; unset FLEET_KEEPALIVE_DRY_RUN to stop them
hygiene: no orphaned keep-alive processes
```

## Why it is a command and not only a timer

The 2026-11-06 incident's standing fix was that nothing was pruning per-turn
snapshots while a run kept writing. The same shape applies here: a leak with no
reaper is a slow outage. A timer makes it invisible-but-handled; the command
makes it *checkable* — the owner can ask the machine what state it is in, which
is the difference between a fix and a hope.
