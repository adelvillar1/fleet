---
name: fleet-frontend-workload-2026-10-27
description: "Owner ruling 2026-10-27: a `frontend` workload class routes user-facing interface work to deepseek-flash (the API reserve, first choice for THIS workload only) with GLM-5.3-Flash as backup — the payg reserve's ruled exception; 2026-10-31 DESIGN did NOT get a second label (it is pinned by its page), so frontend stays the only reserve-first workload."
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner ruling 2026-10-27: fleet gained a **`frontend` workload class** — "frontend and UX/UI workloads need to be routed to deepseek-flash with glm-5.3-flash as a backup", refined to: deepseek-flash from the **api provider** (the payg reserve, `TAIL_PROVIDER`), first choice **for this workload only**; backup `zai-coding-plan/GLM-5.3-Flash`, final fallback `ollama-cloud/glm-5.3-flash` (the same GLM family on the Ollama-cloud account — already in the roster, 1M window; the walk is exactly these three, with_tail dedupes the deepseek tail).

**Why:** frontend/UX/UI work is its own shape (interface surface, judged by how it reads and behaves for a user), and the owner wants the deepseek model heading it — the reserve's one ruled exception to "cannot serve unpinned".

**CAVEAT, verified 2026-10-31:** this chain only serves when `frontend` is the *walked* workload. An unpinned **leading** run walks the orchestrator workload (`hard`) by standing owner ruling, so the judged label is recorded and then ignored — a design run on this machine served `GLM-5.3-Flash` (from `hard`'s chain), not deepseek-flash. Read a label's real effect off a run's `end` record, not off this table.

**2026-10-31 correction (design):** I first answered that design SHOULD get its own label + a leader exception, and landed it; the owner overruled — *"design-canvas uses 3 models or gives the choice… the router should not use the models designated for design work"*. The whole `design` workload (preference, sys1 label, leader exception, hint seam) was reverted to HEAD and design runs are pinned by the design page instead ([[design-routing-comes-from-the-design-page]]). So: **the table has eight workloads again** (the `the_preference_table_is_the_owners_tuned_table` pin counts 8 and asserts frontend is the only deepseek-first chain; `the_router_board_reads_fleets_own_routing_state_into_its_snapshot` lists the tiers without a design entry; `a_missing_config_file_is_the_defaults` expects `roster.preferences.len() == 8`). A design-shaped ask now judged `frontend` simply walks frontend's chain like any interface work.

**How to apply:** the label lives in sys1's `ROUTE_WORKLOAD_LABELS` + the workload head's descriptions (the description IS what the decision model scores); the candidates live in BOTH `default_preferences()` (route.rs) and `~/.fleet/routing.json` (merge by name, new workloads push); `profile_rank` treats frontend like standard_code; `with_tail` dedupes so the walk stays exactly three. The **payg guard reads `allowed=true`** because a workload names the reserve. Adding a workload class touches: sys1 labels → vendor sync → fleet table → routing.json → profile_rank → the four pin tests (table length, reserve-guard, payg guard, board tier list). Related: [[fleet-folder-is-a-default-not-a-ritual]].
