---
name: extraction-leg-through-sys1-2026-11-05
description: "The extraction leg is a sys1 lane client (4820c72, 9/9, merged to master at de5d643) — sys1_extract + task_spec, the floors still in Rust; the live walk found four defects, the audit found the last direct caller, the follow-up repaired three silent-pass paths; the face binary and Tauri dmg rebuilt and the face relaunched"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

**COMPLETED 2026-11-05, 9 of 9, pushed 4820c72** on `face-plugin-host`
(`docs/plans/2026-11-05-extraction-through-sys1.md`, recap
`docs/recaps/2026-11-05-extraction-through-sys1.md`). The owner's ruling
[[sys1-is-the-abstraction-a-vendor-key-gate-is-not]] has no violators
left: `src/extract.rs` resolves no vendor key and names no vendor host.

**What shipped.** sys1 serves `sys1_extract` (entity schema + threshold
+ offsets + confidence; a wrong offset is DROPPED and named, an empty
field DECLINED by name, failures are codes) configured by the provider
table's own `extract_*` entries; `sys1_classify` takes an inline
`task_spec` (the HTTP face's `task_from_spec`), with
`task_for_arguments` module-level so the seam tests without stdio; each
extract call writes its own `op: "extract"` JSONL row; `ExtractSession`
drives the leg (LOCATE via `sys1_extract`, VERIFY via
`sys1_classify`, the adjudication once across every provider). Floors
0.85/0.35, segmentation, budgets and absence semantics unchanged.
`LaneSource`/`CountingLog`/`lane_spec` live in `src/lanes.rs`.

**The live walk found four defects — all pinned:**
1. `metric_numbers` stepped BYTES and panicked slicing an em dash in a
   real draft (the desk's review stage).
2. A relative source resolved against the CWD, so a correct brief read
   as ZERO claims with the stage `done` — the silent skip, the exact
   shape of [[a-declaration-is-not-an-execution]]. Relative paths now
   resolve under the desk project; an unreadable source is a named
   refusal.
3. `mark_stage` no-opped on a store lacking the entry — a whole walk
   could land nothing. It inserts in walk order.
4. **This move's own regression:** `answers()` began returning the
   provider-keyed reply, serde_json sorts keys, so the desk's readers
   took "the first map" as "the provider that answered" and read 0.0
   for every head. `provider_map` reads the answering provider; an
   escalated answer's empty provider name no longer reaches `models`.

**Live proof:** `invoice_id INV-2026-114` p 0.9939, `total_due
$12,480.00` p 0.9929, `due_date 2026-11-11` p 0.9984,
`credit_note_total` absent by name; models gliner + glide/drex/jev (the
lane chose the chain); the lane's log line
`{"op":"extract","ok":true,"model":"fastino/gliner2.5-multi-v1","spans":4,"declined":["credit_note_total"],"latency_ms":276,"http_status":200}`.

**The same-day audit (e86cd4e): the probe was the last script.** Read
surface by surface, not inferred: every Fleet *decision* surface already
calls the lane — router judgment (`JUDGE_LANE = "sys1"`), `fleet judge`,
delegation grading, `curate_judgment`, tool_search's `tool_route`, the
desk's claims + `desk_coverage` (sys1) and outline + corpus (blend1),
perception (`sys1_perceive_document`), the GUI's ask control. The one
remaining direct caller was `scripts/extract-probe.sh`, the E0 probe
still POSTing to `api.fastino.ai` for both passes; it now drives the
lane over stdio and was re-run live (four spans; verify answered by SIX
providers: decide, jev, drex, glide, julia, liquid). Three near-misses
cleared, not violations: `src/api.rs`'s `/chat/completions` are the
language-model client (route.rs's `liquid` is the 24B *chat* endpoint,
not d1 System One); blend1's judgments call `sys1.classify(...)` as a
library, its raw HTTP is Liquid's generator + critic chain;
`memory.rs`'s embedder posts to `127.0.0.1:8901` (sem1's serving port).
`src/lanes.rs` names every vendor host and key env, but ONLY as the
settings picker's declaration table — labels, never a call.

**The follow-up repairs (4bf08f6, de5d643) — three silent-pass paths.**
Asked "anything that needs fixing?", the sweep found: a comment in
`layerfs_tools.rs` still citing the removed `sys1_extracts()`; the live
test PANICKING on a missing key instead of the house loud skip (so
`--ignored` failed the run on a fact it never measured); and the
retargeted probe printing nothing and exiting 0 on a missing or refused
reply. Also corrected two docs I had just written: the recap's "80 s lib
half is load" (the re-run measured 32.5 s with the same 1,865 tests) and
`desk.md`'s "the sentence it was read from" — the claim stores the
SEGMENT, sometimes two fragments merged under the 40-character floor.

**The close-out the owner had to ask for (2026-11-05, late).** Asked
"did you rebuild the face and tauri apps? relaunch? merge and push?",
the honest answer was: only the branch push had happened. Then done in
one pass — `cargo tauri build` (dmg 10.87 MiB, mounted and read:
`sys1_extract` in the bundled `mcp_server.py`, `/Applications/fleet.app`
byte-identical to the fresh build), the release binary copied over
`~/.fleet/bin/fleet`, `launchctl kickstart -k gui/$(id -u)/com.fleet.gui`
(new pid, `/` 200, `fleet lanes` shows `sys1_classify · sys1_extract ·
sys1_perceive_document`), and master fast-forwarded **without a merge
commit** by pushing the branch ref onto it (`origin/master` ef591be →
de5d643; 0 behind, 70 ahead). The Tauri window itself was not opened.
Asked next to "catch it up", `fleet-master-wt` was fast-forwarded
(`git -C Projects/fleet-master-wt merge --ff-only origin/master`) from 62
behind to `de5d643`, clean, in sync with `origin/master` — so all three
checkouts now agree. See [[rebuild-is-not-commit-push-or-ship]] — the
same ask, now the ninth time.

**How to apply.** A vendor call from Rust is a defect by definition —
add the verb to sys1 and log it there. Audit by reading call sites, not
by grepping a remembered host list: a dev script is a caller too. When
asked "anything to fix", look for the paths that SUCCEED on absence — a
panic where the convention is a skip, an empty print with exit 0 — those
are the ones a green gate hides. pytest is not installed on this
machine — sys1's tests are stdlib `unittest`. The mock tests were green
while the live walk read every probability as 0.0.
