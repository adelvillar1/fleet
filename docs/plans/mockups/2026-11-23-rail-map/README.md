# R9 evidence — the map drawer below 900px

Captured 2026-11-23 from the **running implementation** (the release binary built
for R9, port 3995), not from a mock: WKWebView at the stated viewport, the rail
drawer forced open (`body[data-mobile-rail="open"]`, transitions disabled), each
theme set through the face's own `data-theme`.

| File | Viewport | Theme | What it shows |
|---|---|---|---|
| `png/rail-drawer-night-390.png` | 390 × 844 | night | the drawer full-width: Home tinted at the head, SOFTWARE / DESIGN / KNOWLEDGE regions with their lanes, SYSTEM strip with the plugin rows, quiet lanes dotless |
| `png/rail-drawer-night-820.png` | 820 × 1180 | night | the same map at the left edge, the work visible behind the scrim |
| `png/rail-drawer-day-390.png` | 390 × 844 | day | the same at 390 in the day theme |
| `png/rail-drawer-day-820.png` | 820 × 1180 | day | the same at 820 in the day theme |

The harness pin that keeps this composition is
`theMapIsTheRailDrawerBelowNineHundred` in `tests/face/harness.mjs` (390 boot,
`#rail-menu` press, the region headers in order, Home present, the scrim).
