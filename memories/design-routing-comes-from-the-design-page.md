---
name: design-routing-comes-from-the-design-page
description: "Owner ruling 2026-10-31 ("both") — design is a special workload with 3 designated models, and the router must NOT walk them: the design page names the model per run and sends a provider/model pair pin"
type: feedback
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

**Owner ruling 2026-10-31, verbatim:** *"design-canvas specifically uses 3 models or gives the choice to select models to the users, there router should not use the models that are designated for design work, not route to a selection of them"* — then *"this case in particular routing comes from the design page, by the fact that it is a design task coming from that page"*.

**Why:** page design is a special category because models differ in how well they *design* (`deepseek-flash`, zai `GLM-5.3-Flash`, and the same GLM on `ollama-cloud` as the fallback when zai is unavailable) — but a designation is not a routing chain. The owner wants to *choose*, and the fact that the request started on the design page is what makes it design work. A router workload would route a selection of them behind the owner's back.

**What landed instead of the router workload (`b46383d`):**
- The attempted router `design` workload (preference entry + sys1 label + leader-judged exception + surface workload hint) was **reverted whole** on the owner's steer. `route.rs`/`agent.rs`/`tasks.py`/`VENDOR.json` went back to HEAD — the fastest correct response to "the router should not use these" was removal, not narrowing.
- `canvas::DESIGN_MODELS` — the three, in owner order. `GET /api/design` resolves each against the roster (`Resolved::usable`), so the page refuses an unavailable model *before* a run starts.
- The composer's `#design-model` picker opens on the first that can serve, keeps the choice, and the run goes out `pin: "<provider>/<model>"` — a pair pin, which **skips the router's judgment entirely** (the one existing lever for "the surface decides the route").
- Design asked for anywhere else (conversation, cron, CLI) still lands on the board; the face raises one **steer card** naming the board and its press opens `#/design/<slug>`. Harness case PM-E.
- `frame`'s result now names the preview route `/api/design/frame/<slug>/<id>`, because a live pinned run spent 8 of 12 turns browsing for a canvas route (`/canvas/…`, `/design`, `/design/slug` all 404 — the canvas is a HASH route) before giving up on seeing its own page.

**How to apply:** a surface that knows what its run is for names the route itself (pin), and the router's chains stay clear of designated models. Verify with the `end` record: the live run started as the composer starts one (`pin: deepseek/deepseek-flash`) landed a frame and read `served_model: "deepseek-flash"`. Supersedes the owed items in [[a-second-label-for-one-capability-splits-confidence]] (the label was earned by the misrouting, the chain was not what the owner wanted) and corrects [[fleet-frontend-workload-2026-10-27]]'s caveat only in that design no longer needs a judgment exception. Part of [[design-canvas-complete-plan-2026-10-31]]. Related: [[the-screenshot-is-the-visual-review]] (the 319px panel overflow), [[a-test-double-must-read-the-declared-body]].
