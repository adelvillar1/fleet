---
name: obscura-live-probe-reaches-and-limits
description: "Live face probes go through obscura (subprocess `obscura mcp --allow-private-network`, not a piped stdin), which gives real layout and JS evaluation but no input-level drag synthesis — DataTransfer and DragEvent are undefined there."
type: project
date: 2026-11-18
basis: inferred
tier: durable
shape: [deps, resolved]
phrases: [obscura mcp --allow-private-network, browser_evaluate, DataTransfer is not defined, browser_snapshot, a piped stdin is not a driver]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

obscura is the house browser engine (services/browser/service is a stdio proxy to `obscura mcp`), so a live probe of the face goes through it. It is a real renderer: real layout, real CSS, real DOM. What it does NOT give you is input-level drag synthesis — `DataTransfer` and `DragEvent` are both `undefined` in its page context, and its MCP surface (38 tools) has `browser_click` (no modifiers), `browser_press_key`, `browser_fill` and `browser_evaluate`, but no mouse-move/drag tool. So it cannot exercise a pointer drag end to end.

What it CAN do, and what made it worth driving: evaluate arbitrary JS in a real page, read `getBoundingClientRect()` through the real cascade, and watch network calls by shadowing `window.fetch`. That produced the two facts this session needed — a typed query sent no `/api/sessions?q=` at all, and every queue row measured 0x0 because an ancestor (`.composer`) was `display: none`.

Two traps. Its stdio server silently skips stdin it cannot parse and exits when the file ends, so a driver that PIPES commands in (rather than subprocessing with its own pipes) looks like a green exit 0 with no output. And `obscura mcp` refuses loopback unless `--allow-private-network` is passed.

Reach for it before a second browser: the reflex to `npx playwright install` costs ~1.1 GB in ~/Library/Caches/ms-playwright and installs a browser the repo does not ship.
