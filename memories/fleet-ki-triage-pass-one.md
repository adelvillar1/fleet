---
name: fleet-ki-triage-pass-one
description: "the KI ledger's triage plan (docs/plans/2026-10-17-ki-triage.md) — pass one landed 2026-10-18 (9060d79) and all three batch sweeps completed the same day; of the 227 rows then, every one but KI-066 is now closed — the 2026-10-24 burn-down, external-repo session, owner-input audit and owner rulings resolved the rest, and the ledger reads ~282 closed / 11 open / 17 noted with all open rows mechanical"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The ledger (`docs/known-issues.md`, now 226 rows) is the system of record; the
triage plan batches the open rows by action class. **Pass one landed
2026-10-18 as `9060d79`; batches 1 and 2 completed the same day (`7779e6a`).**

**C0 holds:** every row carries an ID, a severity, a what, a status, and a
next action — the parser reports zero missing fields. Every pipe inside a
ledger cell is escaped as `\|`, so each row renders as one row.

**Pass one — five batch-1 rows closed, each with its regression pinned:**

- **KI-220** — the write-time convention guard refused a live roster model
  named in a doc's backticks (`GLM-5.3-Flash` is a case variant of the
  lane's `glm-5.3-flash`). Fixed: a token whose lowercase form matches a
  compiled-in roster model is a *wire identifier* — the string the API takes
  — not project drift. Pinned by `a_live_roster_model_is_a_name_the_guard_does_not_police`.
- **KI-222** — fleet's own corpus (638 entries over 85 sources) exceeds the
  embed's inherited 60-second default. Fixed: the rebuild's one-shot embed
  passes the longer timeout the kwarg already accepts
  (`EMBED_TIMEOUT_SECONDS = 600.0` in `services/blend1/src/blend1/corpus.py`,
  the kwarg at `services/sem1/src/sem1/providers/http_llama.py:28`). The
  ceiling was a default, not a wall.
- **KI-223** — the corpus refresh's change detection was poisoned by cwd
  (keys were `os.path.relpath`-relative, so the same sources under a
  different cwd derived a different key set with byte-identical text and
  rebuilt everything). Fixed: the chunk key is the source's ABSOLUTE path,
  the one cwd-independent name it has.
- **KI-225** — a job's declared sources included files that do not exist and
  the refresh derived silently from the remainder. Fixed: `_source_files`
  returns the missing ones and `refresh` carries them as `missing_sources`
  in its report.
- **KI-226** — the write guard blocked ordinary Python drivers (every
  capitalized token with a lowercase established form was refused). Fixed on
  both scopes the row named: the guard polices the tree its vocabulary was
  derived from (a path outside it — `/tmp` scratch — is not project code),
  and a Python file's capitalized builtins (`Thread`, `Queue`,
  `TimeoutError`, `SystemExit`, `Exception`, …) are the language's names.
  Pinned by `an_ordinary_python_driver_is_writable_as_written`.

**Batch 1 COMPLETE — all 26 tagged rows closed.** The memory group
(KI-059 protected-tier synonyms, KI-061 the loader's description bound,
KI-063 the consolidator's drain, KI-074 the adopted root named, KI-087 the
home's corpus preferred over a walked-up foreign one); skills (KI-053
evidence appends to a waiting draft, KI-054 merge drafts named by the pair);
compaction (KI-080 an over-window tail fails by name, KI-081 the estimate
counts the prompt); docs (KI-011 the Windows claim scoped, KI-051 the
README layout, KI-084 the ledger pipes escaped, KI-032 the bare
`cargo run --` verified gone); quick (KI-018 the mock survives an empty
connection, KI-027 the mock's stderr rides the panic, KI-082 the
"summarizing" line, KI-079 RunReport's compaction count, KI-149 closed via
KI-222's timeout); explain (KI-167 the spend read by op, KI-168 the verdict
as of the explanation's ts, KI-169 a row with no sha is refused); lanes
(KI-030 the provider health tools, KI-148 blend1's longer call bound); cron
(KI-089 the atomic per-minute claim); the last group (KI-039 the SYS1_
namespace passthrough, KI-067 the supersede prompt carries the target,
KI-069 the remember end-to-end test).

**Batch 2 COMPLETE — 22 closed, 9 moved to batch 3 with reasons.**
Closed: KI-064 (the probe's answer cached per process in a OnceLock),
KI-056 (`session::summarize` head-and-tail, so scan_usage pays one replay),
KI-044 (sys1's availability cached, the fallback free), KI-017 (tool
execution already on spawn_blocking; the row's other two legs accepted on
its own terms), KI-118 (the cadence reads only the active board),
KI-077 (the status bar's width budget), KI-078 (the modal wraps, the card's
goal bounded), KI-086 (the system marker), KI-012, KI-072, KI-073, KI-075,
KI-040, KI-042, KI-055, KI-100, KI-032, KI-101, KI-146. Moved: KI-013 (the
fork's doc — external), KI-023 (the runtime restructure — its own
increment), KI-025 (unicode-width — a new crate), KI-052 (a model-polishing
pass — a feature), KI-071 (measure first), KI-076 (a bounds decision),
KI-098 (unifying stores — a design decision), KI-107 (a feature), KI-117
(preserve_order — a new dependency).

**The triage lesson that paid for itself: a KI row's next-action can be
STALE.** KI-017's fix ("wrap tool execution in spawn_blocking") had already
landed in a previous session — `run_builtin` and the write guard both run on
the blocking pool — and KI-032's was fixed earlier too. Read the code before
building what a row suggests; close on the row's own terms when its other
legs are the ones it accepted. A batch tag is an assignment, not a closure.

**Batch 3 swept 2026-10-18 (`34d694b`): 216 of 227 rows are closed** — the
number the parser reports, not the 217 a hand count gave. Five rows
landed as code — KI-050 (the demotion's streak accumulates across a 3-day
window read as the newest days that exist under the stream root), KI-019 (the
streamed response's `finish_reason` is derived when the provider carries it on
the choice rather than in the usage object), KI-022 (the session log is created
on the first task, not at launch, on both faces), KI-076 (the `subagent`
declaration states that `max-turns` is the only bound on the delegation as a
whole), KI-013 (the fork doc names the lineage — the kit is the neutral core
extracted FROM `zcode-router-kit`, per the kit's own CLAUDE.md). The rest
closed by naming the decision rather than writing code: KI-008/028/070/058/113
as decided or already-fixed in-tree, and KI-046/049/052/065/083/098/107
deferred with their triggers named (KI-046's reweighting trigger has not fired
— no calibration stream exists on this machine). The sweep's gate also found
and fixed KI-227, a test-isolation defect with a nameable mechanism — see
[[fleet-memories-tests-need-their-own-home]].

**The ledger's own verdict closed the sweep (`3a7b471`), and it corrected the
count.** The parser said 12 open while the sweep's commit said 217 of 227
closed, and both causes were invisible to every check the parser has: KI-128's
fix had landed in `ddb9152` while its row still read `open` (code right,
ledger wrong), and fourteen rows carried an unescaped pipe — the
`— moved to batch 3 —` annotations appended as a new cell instead of inside
the next-action cell, which shifted the status read rather than reading as a
missing field. A third row, KI-207, sat open in the file the whole time and no
hand count had ever found it. See [[a-validator-that-pads-hides-the-shift]].

**Eleven rows remain open, every one outside this repo or waiting on the
owner:** KI-001 (the kit's CI, diagnosed — Node 20 against the probes' `.mts`
imports and an unlinked `workflow-plane` package), KI-003 and KI-029 (upstream
sys1's four uncommitted files), KI-004 (gen1 has no remote), KI-005 (sem1's
missing `CLAUDE.md`), KI-066 (the smoke pattern's fail-closed port check),
KI-117 (preserve_order pulls `indexmap`, a genuinely new crate — the
zero-new-crates contract makes it the owner's call), KI-135 (the remaining
lanes' live calls), KI-136 (the Linux quick start), KI-143 (filing blend1's MCP
adapter upstream), KI-207 (the cua driver binary against the running daemon's
contract — `cua-driver update --apply` or a pinned FLEET_CUA_DRIVER, either
closes it).

**The plan is marked `completed` and pushed (`2340733`, master → origin):** all
eight acceptance criteria met, each verified rather than assumed — C0 by the
parser's zero-problem verdict, C5 by ten consecutive clean runs, C7 by the
eleven open rows each naming their action. The session recap is
`docs/recaps/2026-10-18-the-ledger-closes-on-itself.md`; it proposes two
contract-doc updates it deliberately does not apply — `docs/features/
calibration.md` still says the demotion reads only today's file (KI-050 made
that false) and `docs/features/subagents.md` omits that `max-turns` is the
only whole-delegation bound (KI-076).

**Environment fact:** pytest is NOT installed on this machine, but python3 is
(homebrew python3.15) — the blend1 lane's seven tests (4 adapter + 3 corpus-key
tests in `services/blend1/tests/test_corpus_keys.py`) run directly under
python3 with a tiny runner that supplies `tmp_path`; all pass. Related:
[[fleet-memory-corpus-dirties-the-tree]] (the ledger and the memories corpus
both dirty the tree), [[gui-revamp-browser-face-decision]] (the revamp this
triage ran beside), [[name-the-missing-tool-not-the-runtime]] (say pytest, not
python).

**2026-10-24 disposition of the eleven:** the ledger had grown to 310 rows and
the burn-down plus the external-repo session closed all but the mechanical
ones. Of this file's eleven: KI-001, KI-003, KI-004, KI-005, KI-029, KI-143
and KI-207 closed in the external-repo session (see
[[external-repo-ki-closures-2026-10-24]] — router-kit CI green after four
layers, sys1 landed and re-vendored, gen1's remote was the owner's own work,
sem1's CLAUDE.md landed, blend1's adapter filed upstream, the driver updated
to 0.37.0 and live-verified); KI-117 and KI-135 closed in the same day's
owner-input audit (the ruling was in KI-117's own row; KI-135's gap closed
under the capability surface). The audit itself (`8f613c4`) also closed
KI-117's neighbors KI-135/309/024 and re-pointed KI-268 — see
[[a-ledger-row-can-name-the-wrong-cause]] for the verify-rows-against-history
lesson. **By that evening nothing on this file's list waited on the owner:**
KI-136 closed on a docker verification (the quick start's literal sequence in
`rust:1`, the only FAIL the unset DeepSeek key), KI-192 closed by ruling
("not needed" — append-only dispositions stand), and blend1's
`feat/mcp-stdio-adapter` plus a second filing (`feat/corpus-carries`: the
KI-297 dedupe, KI-222's embed budget, KI-223/225 contracts) merged to
upstream main and pushed. Of the original eleven only KI-066 (the fail-closed
smoke guard) remains open, mechanical.

**Evening session — the unlocked backlog worked (plan
`2026-10-24-unlocked-ki-items.md`, 9/9, gate 1671 green; see
[[unlocked-ki-items-shipped]]).** KI-066 closed (the guard is a standing
sentence in `docs/harness-orientation.md`'s environment section — assert the
mock's port file and fail closed; a pinned candidate alone already fell
through), KI-190 closed on evidence (its fix was `e1796da`, nine days stale —
see [[ledger-rows-lag-their-fixes]]), and KI-239/240/241/268/310 closed with
work: the deck's pptx+fragment doors, the intent skill's schema-authoritative
boundary, the partial-embed pin in `bridge::rank_with`, and blend1's refresh
naming `BLEND1_SEM1_PYTHONPATH` upstream and vendored. KI-249 stays open with
its measurement half landed (`embed_timing` calib rows at the corpus loads);
KI-252 stays open on its gated blueprint moves. **Final census: 294 closed /
2 open / 15 noted over 311 rows** — every open row open by design.
