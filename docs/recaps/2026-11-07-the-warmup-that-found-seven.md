# Session Recap — 2026-11-07: the warmup that found seven

**Summary.** The warmup asked one question — is this tree healthy? — and
the answer was no: the suite reported 2019 passed / 5 failed in the lib
and 166 / 2 in integration, and all seven failures were the same defect
in two shapes, a test that read the machine it ran on instead of the code
it graded. All seven are fixed at the mechanism rather than by unsetting
a variable. Two further defects came out of the same read: the browser
lane could not find Chrome on macOS at all, and `docs/known-issues.md`
was committed carrying merge-conflict debris that its own checker passed.
The full four-leg gate is green, the ledger carries three new rows, and
six plans whose boxes were already ticked were moved to `completed`.

## The finding, and why it was the finding

The suite's failures were not flaky and not environmental noise. They
were a gate that could not be trusted: five of them failed on any machine
that exported `FLEET_SERVICES_ROOT`, `GEN1_KOKORO_PYTHON`,
`SDM1_MODELS_VENV` or `SEM1_ST_VENV`, and only unsetting all four at once
made the run green. The house's own name for this shape is a fixture
whose budget is counted in different units than the mechanism — here, a
test whose premise ("an empty home means every setting is unset") is
about a world the process was never given.

`cargo test` at the point of discovery, with the four ambient variables
exported:

```
lib:         2019 passed; 5 failed; 3 ignored
integration: 166 passed; 2 failed; 4 ignored
```

## The seven, and the seam each one got

| Case | What it read | The fix |
|---|---|---|
| `settings::render_tests::an_empty_home_renders_every_setting_as_unset` | `std::env::var` per setting | `render_with(home, process)`; the test passes `no_environment` |
| `settings::render_tests::the_surface_is_cards_and_never_a_table` | the same, and died of the poisoned lock above it | the same seam; `ENV_LOCK` deleted |
| `lanes::tests::lanes_json_values_reach_the_spec_and_only_declared_names_cross` | `std::env::var` inside the precedence | `configured_spec_holding(…, held)`; the test passes a closure that answers false for every name |
| `lanes::tests::the_dotenv_wins_over_the_lane_config` | the same | the same seam, plus a second leg asserting a name the test reports as present never enters the spec |
| `mcp::tests::an_absent_allowlist_entry_is_absent_not_empty` | `std::env::vars_os()` as its fixture's base | `parent_env` builds only from its own entries |
| `a_lane_home_override_shadows_the_bundled_copy` | the spawned `fleet lanes`' inherited `FLEET_SERVICES_ROOT` | `repo_services_root()` names this checkout's `services/` |
| `the_lanes_command_reports_every_real_lane` | the same | the same |

The two integration cases are the sharper half of the lesson. Neither was
about the environment being dirty — `FLEET_SERVICES_ROOT` is *set on this
machine on purpose*, pointing at `/Applications/fleet.app/Contents/
Resources/services`, the installed app's frozen copy of the services tree.
So those two tests graded a *different tree* from the one under test, and
the difference was visible: `grep -c "Applications/Google Chrome.app"` on
the installed copy's browser lane returned 0 against 2 in the repo copy —
which is how the second defect surfaced.

Both integration cases were confirmed pre-existing at HEAD before any fix,
by stashing the three source files and re-running: the same two failed.

## The two defects the read turned up

**KI-471 — the browser lane could not find Chrome on macOS.**
`find_chrome()` probed four PATH names and then `~/.cache/puppeteer`. A
Mac installs Chrome as an app bundle with none of those names on PATH, so
on the owner's own laptop the lane refused with "no Chrome found" while
Chrome sat in `/Applications`. The module's docstring already stated the
rule the code broke: a capability the machine must supply is named, not
assumed. The bundles are now probed between the PATH names and the cache,
and the refusal names them.

**KI-470 — the ledger gate passed a file with committed debris.**
`docs/known-issues.md` ended in a bare `master` line and a
`>>>>>>> origin/master` closer, left by the 2026-11-06 merge `d5e9770`
(blame: line 547 from d5e9770, line 548 from eb57d8f). `ki_ledger.py`
reported "all cell counts agree with the header", exit 0, with the debris
in place — because `parse` skips every line that does not start with a
table pipe, which is right for the file's prose preamble and wrong for a
marker, which is also not a row. The printed claim was true and useless:
the failure shape of a botched resolution is rows *deleted* and markers
*left*, and a checker that counts only what survived cannot price what did
not. `conflict_markers()` now names all four marker shapes anywhere in the
file, outside the row parse. Row counts were reconciled across the merge
regardless — `d5e9770^1` 462 rows, `d5e9770^2` 466, HEAD 469 — so nothing
was actually lost that time.

## The gate

Run twice: once on the seven-test fix alone, and again after the
owner-inbox recovery landed.

```
cargo fmt --all --check                    exit 0
cargo clippy --all-targets -- -D warnings  exit 0
cargo build --release                      exit 0   (2m 18s)
cargo test                                 exit 0
  lib          2025 passed; 0 failed; 3 ignored
  integration   168 passed; 0 failed; 4 ignored
  face            1 passed; 0 failed
python3 scripts/ki_ledger.py               exit 0   (473 rows, 6 open)
python3 scripts/plan_check.py              exit 0
```

The suite was run with all four ambient variables exported *and* unset and
gave the same numbers both ways — which is the property the fix was for:
the gate now measures the code, not the machine.

`plan_check.py`'s green is a weaker statement than it looks, and this
session is the proof: it checks that a completed plan's boxes are
ticked, not that the ticked boxes are true. A box ticked on faith passes
it exactly as a box ticked on evidence does.

## Plans closed

Six plans moved to `completed`, each after its acceptance boxes were
checked against the tree rather than taken at their word:

| Plan | What was verified |
|---|---|
| `2026-10-25-layerfs-seamless` | `src/layerfs_view.rs` present; the `workspace-history` rail entry in `app.js`; C6's close-out recorded |
| `2026-11-03-provider-add-is-independent-of-its-catalog` | KI-407 closed; all four pins present in `src/settings.rs` |
| `2026-11-06-readme-install-and-tui-images` | the dmg install path in the README; all 24 `docs/images/` links resolve |
| `2026-11-06-reload-reattach` | the `fleet-open-session` / `fleet-pane` writers; the RE harness cases |
| `2026-11-07-owner-inbox` | **prematurely closed — see below.** The feature and its harness cases are real and on master (`GET /api/owner-inbox`, `src/gui/server.rs:1640`), but C0's mockups and C10's gui.md section were absent from the tree, and C1's ask row read two fields the bridge never emitted |
| `2026-11-07-plan-worktree-machinery` | `scripts/plan-worktree.sh`; the CLAUDE.md convention at line 146 |

`plan_check.py` exits 0 over the result — but the check is only as
strong as the boxes it reads, and one of the six did not survive a second
pass.

## The close that did not hold (KI-473)

The owner-inbox flip was made on one grep: `GET /api/inbox` "serves",
which matched `src/gui/server.rs:1639` — the **bot** face's own
session-list read, a different surface for a different face. The owner's
queue is at `:1640`, `/api/owner-inbox`. Three of that plan's criteria
named artifacts that were not in the tree at all:

| Criterion | Claimed | On master, before this pass |
|---|---|---|
| C0 | 16 measured mockups in `docs/plans/mockups/2026-11-07-owner-inbox/` | the directory did not exist |
| C1 | the ask row carries its kind and its age | `owner_inbox` built `session`/`from`/`words`; `app.js` read `ask.kind` and `ask.at` — an empty age slot, every ask titled "ask" |
| C10 | gui.md gains the inbox surface | `grep -c -i inbox docs/features/gui.md` → 0 |

None of the three was lost work; all three were still in `stash@{0}`. The
mockups lived in the stash's untracked tree (`stash@{0}^3`, commit
`f915f5d`) — 17 files, 16 distinct PNGs (8 at 2144 × 1800 for the 1024
width, 8 at 2976 × 1800 for the 1440 — the two widths C0 claims) plus
`inbox.html`, whose five rows carry the `seat`/`ask`/`review`/`failed`
classes the plan names.
gui.md's section applied clean to HEAD, and it is the one that names
`GET /api/owner-inbox`. Both are now on master.

C1 was a real gap, not a missing file: the `Mention` record carries
`kind` and `at`, the huddles fold discarded both, and the surface read
both. The fold now keeps them, `owner_inbox` emits them, the age slot
reads `roomAge` (the words the rail's own rows use) instead of
stringifying a timestamp, and the bridge test asserts all four of the
ask's facts. Removing the two emissions re-reds it — `left: Null`,
`right: "ask"` — so the assertion measures the row, not the plan.

The plan's C0, C1 and C10 now carry that history in-line rather than
reading as though the artifacts had always been there.

## Delivery

Six commits on `master`, explicit paths only:

| Commit | What |
|---|---|
| `5b218f8` | `fix(tests)` — the seven seams, and KI-291's `ENV_LOCK` guard retired with its cause |
| `7ba8dd9` | `fix(browser)` — the macOS bundles in `find_chrome()` |
| `0ea650e` | `fix(ledger)` — `conflict_markers()` in `ki_ledger.py` |
| `d93be84` | `docs` — the debris gone, KI-470/471/472 filed, six plans closed, CLAUDE.md's version |
| `6e54580` | `fix(inbox)` — KI-473: the ask's `kind` and `at` carried through the fold, the 17 mockups and gui.md's section recovered from `stash@{0}` |
| `378a848` | `docs(recap)` — this file, with the mockups' two widths stated right |

Uncommitted and left alone: the substrate's own churn — 15 `memories/`
recall-count bumps and the day's `events.jsonl` — which `daily-maintenance`
commits under `chore(memories)`.

The four-commit count this section first carried was itself a claim
written before the work was done: the warmup's own follow-up found a plan
closed around absent artifacts, and closing that honestly needed two
more.

## What remains

- **`stash@{0}` — now partly recovered, and what it still holds is
  named.** The first note called it "26 files, `docs/plans/` only"; both
  halves were wrong. It is 29 modified tracked files — 26 under
  `memories/` (recall-count bumps) and 3 docs — plus an untracked tree of
  23. This session restored from it the owner-inbox mockups (17 files)
  and gui.md's inbox section. Still only in the stash and nowhere on
  master: `docs/plans/2026-11-08-composer-clipboard-paste.md`,
  `docs/plans/2026-11-08-waza-improvements.md`, `ib-proof.txt`, and the
  26 `memories/` files' prior states. **Whether those two plans are
  wanted is the owner's decision** — two of the four plan docs in that
  tree (`hybrid-retrieval`, the zvec deep-dive) already landed on master
  by other routes, so the pair that did not may be superseded rather
  than pending. Nothing else was taken from the stash, so it stands as
  it was.
- **`docs/plans/2026-10-15-router-enforcement.md`** stays `in-progress` on
  one leg: a live run where the real sys1 judges a task `composed` (its
  route task answers low-confidence labels, so the gate routes to the
  default workload). The composed path itself is proven.
- **The installed app's bundle is stale.** `/Applications/fleet.app`'s
  services tree predates the `find_chrome` fix, which is exactly what made
  those two integration cases fail while they pointed at it. A rebuild and
  dmg install closes it; nothing in the repo does.
