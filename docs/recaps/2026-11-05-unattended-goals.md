# Session recap — 2026-11-05: the cap leaves the main session, the goal drives the work

**Summary.** The plan completed whole — 14 of 14 criteria — and the
live proof earned its keep: it caught three product defects the unit
tests could not, each fixed and re-proven on a clean scratch home. The
main session's loop is uncapped by turns; only a delegated child carries
a bound, and that bound is now config-authoritative and
re-dispatch-ceilinged. A goal starts the work, the `goal` tool finishes
it as a fact, and the face reads the completion back.

## What was built

- **The uncapped main loop.** `AgentOptions.capped` (false on the main
  path, true for the subagent's run) replaces the turn ceiling as the
  loop's condition; the main run ends on its own report, the owner's
  stop, a refusal, or an approval wait. The cap receipt and the face's
  "continue the work" control are gone.
- **The child's bound.** `delegation.max_turns` is the setting; config
  is authoritative (the clamp that protected a global ceiling has
  nothing left to protect), under the depth-stepped `turn_ceiling`, with
  `parent_remaining = None` on an uncapped parent — the existing path.
- **The goal starts the work.** `set_goal` on a non-room conversation
  with no run in flight starts the run with the goal as the task. The
  welcome's goal rides the pending key.
- **The `goal` tool.** show / complete / clear, declared on every homed
  run; `complete` writes the objective into `sessions-goals-done.json`,
  the state document exposes `goal_done`, and the chip wears
  `goal · done`.
- **Supervision.** A child bound to a plan node dispatches it on spawn
  and marks it failed on a non-delivering end, with the attempt count
  and the three options named (absorb, respawn, surface). The graph's
  card carries the demotion's reason; `MAX_NODE_REDISPATCHES = 2` bounds
  the respawn loop and persists across restarts.

## What the live proof found (and the fixes)

1. **A deadlock that wedged the whole face.** The start path read the
   state fields through a guard held across `run_with`, which locks the
   same mutex — every endpoint stopped answering. Fixed by reading into
   locals and releasing before the call: `a_goal_on_an_ordinary_session_attempts_its_run`.
2. **The goal tool keyed on the wrong id.** It used `options.session_id`
   — the wire-header UUID derived from the conversation — instead of the
   raw session id, so a real completion answered "no goal is set". Fixed
   with `goal_session`, set by the bridge.
3. **The start was gated on the launch session**, so a goal on any other
   conversation silently did nothing. Fixed: any non-room session with
   no run in flight starts (rooms drive their own turns).

A fourth finding is the plan's own amendment from earlier in the day: the
route's availability gate asked vendor keys rather than sys1, and the
judge now goes through the lane (`95dc98c`).

## The live proof

On a scratch home (`FLEET_HOME` elsewhere, the machine's credentials in
the environment, nothing on disk):

- `/goal` on an idle conversation: **`{'started': True}`** — the run
  began on the goal's own text.
- The run drove **112 turns** (past the old 100-turn cap), 111 tool
  calls, ending naturally: `{"type":"end","stop_reason":"handed back","turns":112}`.
- The `goal` tool completed it: `{"completed":true,"goal":"Run the
  scripted steps to the end and complete the goal.","note":"the goal is
  marked complete…"}`, and `/api/state` then read
  `goal: None, goal_done: Run the scripted steps…`.
- A delegated child, bounded at 2 turns, ended on its own cap from its
  own log: `{"stop_reason":"max-turns","turns":2,"served_model":"glm-4.5"}`,
  kind `subagent`, parent named.

The 110-turn drive used the repo's scripted mock as the model (the
house instrument for loop behaviour); the goal→run→complete chain ran
through the real bridge, the real state document and the real store.

## The verification

Gate in a clean worktree (the tree pushed): fmt 0, clippy 0, release 0,
lib **1,883/0**, integration **166/0**, harness **914 checks / 99 cases**.

## Honest residues

- `memory::tests::the_probe_retries_after_a_pause_so_a_flap_is_not_a_missing_backend`
  flakes under the full parallel lib suite and passes alone — KI-405's
  load-sensitive class, pre-existing and outside this plan.
- A machine with the sys1 lane installed but no provider keys pays one
  lane spawn (~1.2 s) at its first route, the price of asking the
  abstraction instead of guessing at its keys (KI-044's trade, now
  explicit).
- The `capability` gap the plan named at drafting (the extract leg still
  calling System One vendors directly) remains the bigger application of
  the sys1-abstraction ruling; it is a plan-sized change and stays a
  separate decision.
