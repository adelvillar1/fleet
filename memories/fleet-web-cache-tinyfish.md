---
name: fleet-web-cache-tinyfish
description: "KI-253 closed 2026-10-19 — the web lane's answers cache to <home>/web-cache.json (TTL FLEET_WEB_CACHE_TTL_SECS, default 1h, 0 disables; 64-entry cap; errors never cached; hits named with age), and the TinyFish leg was brought up live on this machine (key stored, tinyfish installed for the lane's interpreter)"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The `web` tool's path is: agent `Work::Web` → spawn the blend1 MCP lane
(`services/blend1/service/mcp_server.py`, launched as **system `python3`** with
`BLEND1_PYTHONPATH=<services>/blend1/src`) → the lane's `web` MCP tool → TinyFish
search + fetch. The TinyFish calls themselves live in blend1's Python (the
vendored tree `services/blend1/` in fleet's repo **and** `~/Projects/blend1/`),
not in fleet's Rust — fleet's surface is the tool spec and the lane spawn.

**KI-253 closed (`web_cache.rs`, wired into `Work::Web`):** every identical call
paid TinyFish again. The lane's answers now cache to `<home>/web-cache.json`,
keyed on the exact call arguments JSON, TTL `FLEET_WEB_CACHE_TTL_SECS` (default
one hour, `0` disables outright), capped at 64 entries with the oldest evicted,
written atomically. **Only answers cache — a lane that failed is exactly the one
worth retrying** — and a hit is named as what it is, with its age, in the
answer's first line. Five tests: round-trip, key separation, file reload, the
disabled shape writing nothing, cap eviction.

**The lane bring-up (two things the doctor's message does not spell out):**
1. The owner's TinyFish key went into **`~/.config/blend1/env`** (the lane's own
   env file) and **`~/.fleet/.env`** (fleet's credential chain), both 0600, the
   value never printed — the owner pasted it in chat, so rotate if the
   transcript is sensitive.
2. "tinyfish not installed" meant the **lane's interpreter** lacked it. The lane
   launches with system `python3` (homebrew 3.15) + `BLEND1_PYTHONPATH` —
   installing into `~/Projects/blend1/.venv` was not enough. The fix was
   `python3 -m pip install --user --break-system-packages "tinyfish>=0.9.0"`
   (homebrew python is PEP-668 externally managed; uv `--system` refuses).
   `fleet lanes` then reported the web leg keyed and importable.

**Verified live end-to-end:** a real `fleet run` web query answered by TinyFish
(`status: ok`, 2 results, 3.7s — with a bot-blocked LinkedIn URL **named in the
errors array**, the graceful per-source failure the blueprint asks for, already
in the lane), the cache written, and the identical repeat answered from
"(fleet's web cache, 26s old)" with zero fresh provider calls.

**Wiring bug worth remembering:** the first version resolved the cache home from
bare `HOME` — the cache silently wrote nowhere near the fleet home. The home
must resolve through `crate::home::fleet_home` (which appends `.fleet`), the
same way every other home consumer does.

**How to apply:** never cache lane errors; name hits with their age; key on the
exact arguments. The fan-out the blueprint draws is the agent layer's job (fleet
runs tool calls in parallel waves) — the remaining unbuilt piece is KI-252's
vector half (template index, Matryoshka 128-d pre-filter, cross-modal asset
library), open with a three-move build order. Related:
[[blend1-web-leg-and-corpus]], [[fleet-vault-credentials]],
[[fleet-deck-visual-pipeline]], [[fleet-run-control-and-timeout-gaps]].
