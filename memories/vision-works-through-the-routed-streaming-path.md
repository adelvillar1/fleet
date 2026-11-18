---
name: vision-works-through-the-routed-streaming-path
description: "Vision answers now — the blocking chat path that never consulted the router is replaced by the routed streaming call, so a vision ask routes, fails over, and returns an answer with no approval prompt."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [deps, drops-the-row, resolved]
phrases: [is vision working now, RelativeUrlWithoutBase, Client::routed empty base_url, chat_stream with a no-op event sink, block_on inside a running runtime panics]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Vision works. "is vision working now?" — yes, verified with two live headless runs on 2026-11-18, no approval prompt, real descriptions.

THE FIX (owner chose option (a) — a blocking Router chat — but (a) turned out to be impossible where vision lives, and the fallback was the routing path vision already had):

`Work::Vision` is handled inside `async fn run_work`, so the call is already inside a tokio runtime. A `Router::chat_blocking` would have to drive `lane_client::ask` (async) to completion, and `block_on` inside a running runtime panics. The blocking walk was written and then REVERTED for that reason.

The actual fix: the vision arm now `await`s `client.chat_stream(&request, &mut ignore)` with a no-op event sink, and reads the assembled `ChatResponse`. That routes through the router, which is what the blocking `chat` never did.

WHY VISION WAS THE ONLY CASUALTY: `Client::routed(router)` sets `base_url: String::new()` and relies on the router; the blocking `Client::chat` dialed `format!("{}/chat/completions", self.base_url)` directly and never consulted the router, so the URL was host-less (`RelativeUrlWithoutBase`). Only `chat_stream` routes. Vision was the crate's ONLY blocking `Client::routed` caller — every other one (delegate, cron, tui, subagents, gui) uses the streaming path.

A BONUS the streaming path brings: vision now gets the walk's FAILOVER across the `vision` workload's candidates, which the blocking call never had.

LIVE EVIDENCE, and the one thing to be careful about when judging a vision answer: on `tmp/health-mark.png` the model read the sheet's title, its six candidates, its tab labels and its colour scheme, and reported dimensions 1500x900 which match `file` exactly. On `tmp/candidates.png` it reported 500x16 where the file is 400x40 — a model misjudgement, NOT a resize: `Message::serialize` sends the whole PNG as a `data:` URI with no resampling. So dimension claims are the least reliable part of a vision answer; quoted text is the reliable part.

Also recorded: the model read "candidates at the rail's real 17px" while `tmp/health-pick.html` says "candidates at 17px" — the fixture PNG was rendered from a different build than the source on disk, so a vision answer can disagree with today's source and still be faithful to the pixels.

TEST NOTE: `agent::tests::a_lane_that_cannot_repair_*` and `a_lane_fold_serves_*` flake under parallel load (mock lane children race); the full suite is green serially — 2376 passed, 0 failed, `--test-threads=1`.
