#!/usr/bin/env python3
"""Capture the terminal face's screens as PNGs.

Runs the TUI in a tmux pane of a fixed size, drives each screen's key,
captures the pane with its SGR colours, and renders it to a PNG beside
the other surfaces in docs/images/. A docs tool: tmux and Pillow are
machine tools, not fleet dependencies, and the shots come from a scratch
home — the owner's own home is never the fixture.

    python3 scripts/tui-shots.py [--bin target/release/fleet] [--out docs/images]
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

SESSION = "fleet-tui-shots"
COLS, ROWS = 120, 40
FONT_SIZE = 30
SETTLE = 1.4

# (name, keys before the capture) — the keys are ("hex", "XX") for a raw
# control byte (tmux's own C-p names did not land in the TUI's raw mode)
# or ("key", "Escape"). Escape first when the previous screen left the
# face in a mode the next key would not leave. A panel opens through the
# palette: C-p, the panel's name typed as the filter, Enter.
SCREENS = [
    ("tui-conversation", []),
    ("tui-palette", [("hex", "10")]),                   # C-p
    ("tui-sessions", [("hex", "1B"), ("hex", "0F")]),   # Esc, C-o
    ("tui-checkpoints", [("hex", "1B"), ("hex", "0B")]),  # Esc, C-k
    ("tui-lanes", [("hex", "1B"), ("hex", "0C")]),      # Esc, C-l
    ("tui-plan", [("hex", "1B"), ("hex", "14")]),       # Esc, C-t
    # The three intelligence surfaces (the 2026-11-21 plan): the seeded
    # scratch home carries one declared graph, one approved plan, and the
    # second brain's state files, so each shot is the surface's own read.
    ("tui-delegation", [("hex", "1B"), ("hex", "10"), ("key", "delegation"), ("key", "Enter")]),
    ("tui-board", [("hex", "1B"), ("hex", "10"), ("key", "board"), ("key", "Enter")]),
    ("tui-second-brain", [("hex", "1B"), ("hex", "10"), ("key", "second-brain"), ("key", "Enter")]),
]


def seed_surfaces(home):
    """Seed the scratch home the three panel shots read.

    A declared graph with three nodes (the delegation panel joins the
    day's rows onto it — none here, so every node reads `never spawned`,
    which is the honest state of a fresh machine), one approved plan with
    three atomic tasks (the board's three columns), and the second
    brain's state files (the morning read; the tower file is deliberately
    absent, so its refusal is in the shot).
    """
    os.makedirs(os.path.join(home, "docs", "plans"), exist_ok=True)
    graph = {
        "nodes": [
            {"id": "read-the-brief", "cost": 2, "approach": "read the brief"},
            {"id": "draft-the-plan", "cost": 3, "prereqs": ["read-the-brief"],
             "approach": "draft the plan"},
            {"id": "verify-the-draft", "cost": 1, "prereqs": ["draft-the-plan"],
             "approach": "verify the draft"},
        ]
    }
    with open(os.path.join(home, "graph.json"), "w") as handle:
        json.dump(graph, handle)
    plan = """---
status: active
created: 2026-11-21
updated: 2026-11-21
slug: the-demo
execution: mixture
---

# The demo plan

```graph
{ "nodes": [
  { "id": "read-the-brief", "cost": 2, "approach": "read the brief" },
  { "id": "draft-the-plan", "cost": 3, "prereqs": ["read-the-brief"], "approach": "draft the plan" },
  { "id": "verify-the-draft", "cost": 1, "prereqs": ["draft-the-plan"], "approach": "verify the draft" }
] }
```
"""
    with open(os.path.join(home, "docs", "plans", "2026-11-21-demo.md"), "w") as handle:
        handle.write(plan)
    # The plan's runtime file: one verified, one running, one pending
    # node, so the board's three columns each carry a card. The bridge
    # reads the runtime state when it has landed and the declaration
    # otherwise, exactly as it does on a real home.
    runtime = {
        "slug": "the-demo",
        "declaration": "demo",
        "serves": {"read-the-brief": "C1", "draft-the-plan": "C2", "verify-the-draft": "C3"},
        "nodes": [
            {"id": "read-the-brief", "cost": 2, "approach": "read the brief",
             "status": "verified"},
            {"id": "draft-the-plan", "cost": 3, "prereqs": ["read-the-brief"],
             "approach": "draft the plan", "status": "running"},
            {"id": "verify-the-draft", "cost": 1, "prereqs": ["draft-the-plan"],
             "approach": "verify the draft", "status": "pending"},
        ],
        "revisions": [],
    }
    with open(
        os.path.join(home, "docs", "plans", "2026-11-21-demo.graph.json"), "w"
    ) as handle:
        json.dump(runtime, handle)
    brain = os.path.join(home, "second-brain")
    os.makedirs(brain, exist_ok=True)
    states = {
        "gates_state": {
            "generated": "2026-11-21T08:00:00Z",
            "board": [{"gate": "review", "verdicts": 12, "dispositions": 4,
                       "graded_progress": "2/20", "graded_rows": 2,
                       "fit_ready": True, "override_rate": 0.1}],
            "inbox": [],
            "totals": {"events": 40, "verdicts": 12, "dispositions": 4},
            "daily": [{"day": "2026-11-21", "count": 6}],
        },
        "harness_state": {
            "generated": "2026-11-21T08:00:00Z",
            "loops": [{"loop": "the-sweep", "runs": 12, "converged": 11,
                       "last_status": "ok"}],
            "recent": [{"ts": "2026-11-21T08:00:00Z", "kind": "loop",
                        "label": "the sweep", "outcome": "ok"}],
        },
        "loops_state": {
            "derived_at": "2026-11-21T08:00:00Z",
            "loops": [{"loop": "the-sweep", "family": "quality",
                       "verdict": "converging", "iterations": 12,
                       "pending_review": 2}],
            "graded": [],
            "findings": ["the sweep converged on 11 of 12 runs"],
        },
        "reports_state": {
            "generated_at": "2026-11-21T08:00:00Z",
            "doing": [{"id": "t1", "project": "fleet",
                       "title": "the terminal surfaces", "harness": "claude",
                       "opened": "2026-11-20", "last": "2026-11-21"}],
            "pending_verification": [],
            "conflicts": [],
            "trust": [],
        },
        "maturity_state": {
            "generated": "2026-11-21T08:00:00Z",
            "weeks": ["2026-W45", "2026-W46"],
            "series": {"continuity": [3, 4]},
            "milestones": [{"when": "2026-W46", "what": "the surfaces landed"}],
        },
    }
    for name, payload in states.items():
        with open(os.path.join(brain, f"{name}.json"), "w") as handle:
            json.dump(payload, handle)
    return brain


def send(session, key):
    kind, value = key
    if kind == "hex":
        sh("tmux", "send-keys", "-t", session, "-H", value)
    else:
        sh("tmux", "send-keys", "-t", session, value)

FONT_CANDIDATES = [
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Monaco.ttf",
    "/System/Library/Fonts/SFNSMono.ttf",
]


def sh(*args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def sgr_palette():
    """The xterm-256 palette as RGB triples."""
    base = [
        (0, 0, 0), (205, 49, 49), (13, 188, 121), (229, 229, 16),
        (36, 114, 200), (188, 63, 188), (17, 168, 205), (229, 229, 229),
        (102, 102, 102), (241, 76, 76), (35, 209, 139), (245, 245, 67),
        (59, 142, 234), (214, 112, 214), (41, 184, 219), (255, 255, 255),
    ]
    pal = list(base)
    # 6×6×6 cube at 0, 95, 135, 175, 215, 255.
    cube = [0] + [55 + 40 * i for i in range(5)]
    for r in cube:
        for g in cube:
            for b in cube:
                pal.append((r, g, b))
    for i in range(24):
        v = 8 + i * 10
        pal.append((v, v, v))
    return pal


PALETTE = sgr_palette()


def parse_line_extended(line):
    """One capture line → [(char, fg, bg, bold)], the SGR state carried."""
    cells = []
    fg = bg = None
    bold = False
    i = 0
    n = len(line)
    while i < n:
        ch = line[i]
        if ch == "\x1b" and i + 1 < n and line[i + 1] == "[":
            end = line.find("m", i)
            if end == -1:
                i += 2
                continue
            params = line[i + 2:end]
            i = end + 1
            parts = [p for p in params.split(";")]
            k = 0
            while k < len(parts):
                p = parts[k] or "0"
                try:
                    code = int(p)
                except ValueError:
                    k += 1
                    continue
                if code == 0:
                    fg = bg = None
                    bold = False
                elif code == 1:
                    bold = True
                elif code == 22:
                    bold = False
                elif 30 <= code <= 37:
                    fg = PALETTE[code - 30]
                elif code == 39:
                    fg = None
                elif 40 <= code <= 47:
                    bg = PALETTE[code - 40]
                elif code == 49:
                    bg = None
                elif code in (38, 48):
                    if k + 1 < len(parts) and parts[k + 1] == "5":
                        idx = int(parts[k + 2]) if k + 2 < len(parts) else 0
                        col = PALETTE[idx % 256]
                        if code == 38:
                            fg = col
                        else:
                            bg = col
                        k += 2
                    elif k + 1 < len(parts) and parts[k + 1] == "2":
                        r, g, b = (int(parts[k + 2]), int(parts[k + 3]),
                                   int(parts[k + 4])) if k + 4 < len(parts) else (0, 0, 0)
                        col = (r, g, b)
                        if code == 38:
                            fg = col
                        else:
                            bg = col
                        k += 4
                k += 1
        else:
            cells.append((ch, fg, bg, bold))
            i += 1
    return cells


def render(lines, out_path, font_path):
    from PIL import Image, ImageDraw, ImageFont
    probe = ImageFont.truetype(font_path, FONT_SIZE)
    box = probe.getbbox("M")
    cell_w = max(1, box[2] - box[0])
    ascent, descent = probe.getmetrics()
    cell_h = ascent + descent
    img = Image.new("RGB", (COLS * cell_w, ROWS * cell_h), (24, 24, 28))
    draw = ImageDraw.Draw(img)
    regular = ImageFont.truetype(font_path, FONT_SIZE)
    bold_font = ImageFont.truetype(font_path, FONT_SIZE)
    for y, line in enumerate(lines[:ROWS]):
        cells = parse_line_extended(line.rstrip("\n"))
        x = 0
        for ch, fg, bg, bold in cells:
            if x >= COLS:
                break
            if bg is not None:
                draw.rectangle([x * cell_w, y * cell_h, (x + 1) * cell_w - 1,
                                (y + 1) * cell_h - 1], fill=bg)
            if ch != " ":
                draw.text((x * cell_w, y * cell_h), ch,
                          font=bold_font if bold else regular,
                          fill=fg or (229, 229, 229))
            x += 1
    img.save(out_path, quality=92)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="target/release/fleet")
    ap.add_argument("--out", default="docs/images")
    ap.add_argument("--home", default=None, help="scratch FLEET_HOME to use")
    ap.add_argument(
        "--cwd",
        default=None,
        help="directory the face is launched in (the board and the declared "
        "workload read docs/plans under it); defaults to this checkout",
    )
    ap.add_argument(
        "--only",
        default=None,
        help="comma-separated screen names to capture, leaving the rest alone",
    )
    args = ap.parse_args()

    font_path = next((f for f in FONT_CANDIDATES if os.path.exists(f)), None)
    if font_path is None:
        sys.exit("no monospace font found")
    home = args.home or tempfile.mkdtemp(prefix="fleet-tui-shots-")
    os.makedirs(home, exist_ok=True)
    # The three intelligence panels read a declared graph, an approved
    # plan, and the second brain's state files; the scratch home carries
    # all three, and the brain's home rides the process environment.
    brain_home = seed_surfaces(home)
    # The scratch home walks the compiled default chain otherwise; the
    # owner's routing.json makes the shots show the chain a real home walks.
    routing = os.path.expanduser("~/.fleet/routing.json")
    if os.path.exists(routing):
        shutil.copy(routing, os.path.join(home, "routing.json"))

    sh("tmux", "kill-session", "-t", SESSION)
    sh("tmux", "new-session", "-d", "-s", SESSION, "-x", str(COLS), "-y", str(ROWS))
    binpath = os.path.abspath(args.bin)
    launch = os.path.abspath(args.cwd or os.getcwd())
    env = f"FLEET_HOME={home} SECOND_BRAIN_HOME={brain_home} {binpath}"
    sh("tmux", "send-keys", "-t", SESSION, f"cd {launch} && {env}", "Enter")
    time.sleep(5)

    wanted = set(args.only.split(",")) if args.only else None
    made = []
    for name, keys in SCREENS:
        if wanted is not None and name not in wanted:
            continue
        for key in keys:
            send(SESSION, key)
            time.sleep(0.3)
        time.sleep(SETTLE)
        # A one-column resize forces a full repaint: the live backend
        # diffs against its own buffer, and an overlay opened after the
        # base frame can leave the cells it did not write showing the
        # previous frame. The resize makes the terminal redraw every cell,
        # so the capture is the frame the app actually holds.
        sh("tmux", "resize-window", "-t", SESSION, "-x", str(COLS - 1), "-y", str(ROWS))
        time.sleep(0.3)
        sh("tmux", "resize-window", "-t", SESSION, "-x", str(COLS), "-y", str(ROWS))
        time.sleep(0.5)
        cap = sh("tmux", "capture-pane", "-t", SESSION, "-p", "-e")
        lines = cap.stdout.splitlines()
        out = os.path.join(args.out, f"{name}.png")
        os.makedirs(args.out, exist_ok=True)
        render(lines, out, font_path)
        made.append(out)
        head = next((ln.strip()[:72] for ln in lines if ln.strip()), "(empty)")
        print(f"tui-shots: {out} — {head}")
    sh("tmux", "kill-session", "-t", SESSION)
    print(f"tui-shots: {len(made)} images from scratch home {home}")


if __name__ == "__main__":
    main()
