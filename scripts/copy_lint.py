#!/usr/bin/env python3
"""The vocabulary lint (U45 / P6): the default, plain register of the face
must not lead with subsystem jargon. It reads the static shell's visible
text — comments, scripts, the expert-only spans (the expert register), and
the console's feature toggles are excluded — and refuses any forbidden term.

Run: python3 scripts/copy_lint.py [path-to-index.html]
Exit 0 when the plain copy is clean; 1 with the offending terms otherwise.
"""
import re
import sys

FORBIDDEN = [
    "sys1", "blend1", "corpus", "substrate", "harness", "lanes", "deck",
    "calibration", "principal", "mcp", "workload", "anti-vector",
    "provenance", "receipts", "tissue", "spine",
]

def visible_text(html: str) -> str:
    html = re.sub(r"<!--.*?-->", "", html, flags=re.S)          # comments
    html = re.sub(r"<(script|style).*?</\1>", "", html, flags=re.S)
    html = re.sub(r'<span class="expert-only">.*?</span>', "", html, flags=re.S)
    html = re.sub(r'<footer class="console-bar".*?</footer>', "", html, flags=re.S)
    text = re.sub(r"<[^>]+>", " ", html)                      # tags -> space
    return re.sub(r"\s+", " ", text)

def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "src/gui/assets/index.html"
    text = visible_text(open(path, encoding="utf-8").read()).lower()
    hits = [w for w in FORBIDDEN if w in text]
    if hits:
        print("copy_lint: forbidden terms in the plain register: " + ", ".join(hits))
        return 1
    print("copy_lint: the plain register is clean")
    return 0

if __name__ == "__main__":
    sys.exit(main())
