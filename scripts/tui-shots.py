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
# face in a mode the next key would not leave.
SCREENS = [
    ("tui-conversation", []),
    ("tui-palette", [("hex", "10")]),                   # C-p
    ("tui-sessions", [("hex", "1B"), ("hex", "0F")]),   # Esc, C-o
    ("tui-checkpoints", [("hex", "1B"), ("hex", "0B")]),  # Esc, C-k
    ("tui-lanes", [("hex", "1B"), ("hex", "0C")]),      # Esc, C-l
    ("tui-plan", [("hex", "1B"), ("hex", "14")]),       # Esc, C-t
]


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
    args = ap.parse_args()

    font_path = next((f for f in FONT_CANDIDATES if os.path.exists(f)), None)
    if font_path is None:
        sys.exit("no monospace font found")
    home = args.home or tempfile.mkdtemp(prefix="fleet-tui-shots-")
    os.makedirs(home, exist_ok=True)
    # The scratch home walks the compiled default chain otherwise; the
    # owner's routing.json makes the shots show the chain a real home walks.
    routing = os.path.expanduser("~/.fleet/routing.json")
    if os.path.exists(routing):
        shutil.copy(routing, os.path.join(home, "routing.json"))

    sh("tmux", "kill-session", "-t", SESSION)
    sh("tmux", "new-session", "-d", "-s", SESSION, "-x", str(COLS), "-y", str(ROWS))
    binpath = os.path.abspath(args.bin)
    env = f"FLEET_HOME={home} {binpath}"
    sh("tmux", "send-keys", "-t", SESSION, f"cd {os.getcwd()} && {env}", "Enter")
    time.sleep(5)

    made = []
    for name, keys in SCREENS:
        for key in keys:
            send(SESSION, key)
            time.sleep(0.3)
        time.sleep(SETTLE)
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
