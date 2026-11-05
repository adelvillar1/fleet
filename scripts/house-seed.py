#!/usr/bin/env python3
"""The house seed (docs/plans/2026-11-05-house-methodology.md, C5).

Lands the house fact pack (`~/.agents/house/facts/`) into a project's
own corpus through `fleet remember` — the writer is the only
frontmatter authority, and the writer's collision refusal makes the
run idempotent: a fact already in the store refuses by name and the
batch continues.

The shadow hazard: fleet's home corpus SHADOWS every project corpus
when it holds facts (`resolve_root_from`'s home-first rule, KI-087's).
The pack therefore never targets `~/.fleet/memories`, and this script
refuses that target by name.

Usage:
  python3 -I scripts/house-seed.py --project /tmp/house-proof
  python3 -I scripts/house-seed.py --project /tmp/house-proof   # idempotent
"""

import argparse
import os
import subprocess
import sys

PACK = os.path.expanduser("~/.agents/house/facts")
FLEET = os.path.expanduser("~/.fleet/bin/fleet")
HOME_CORPUS = os.path.expanduser("~/.fleet/memories")


def remember(description, body, ftype, basis, shape, cwd):
    cmd = [
        FLEET, "remember",
        "--name", description["name"],
        "--description", description["text"],
        "--type", ftype,
        "--basis", basis,
        "--shape", shape,
        "--body", body,
    ]
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)


def parse_fact(path):
    """The pack fact's frontmatter (name, type, basis, shape) and body.

    The description is a folded YAML scalar: the value's lines plus
    every indented continuation join into one string, and the outer
    quote pair strips ONCE here — the writer quotes what it writes,
    and a quote carried in becomes a double-wrapped value on disk
    (the live proof's own find: the fact's topic text started with a
    literal quote mark)."""
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    if not text.startswith("---"):
        return None
    end = text.find("\n---", 3)
    if end < 0:
        return None
    block = text[3:end]
    body = text[end + 4 :].strip()
    fields = {}
    current = None
    for line in block.split("\n"):
        if not line.strip():
            continue
        if line[:1] not in (" ", "\t"):
            key, _, value = line.partition(":")
            current = key.strip()
            fields[current] = value.strip()
        elif current:
            fields[current] = (fields[current] + " " + line.strip()).strip()
    for key, value in fields.items():
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            fields[key] = value[1:-1]
    return fields, body


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", required=True, help="the project directory to seed")
    args = parser.parse_args()
    project = os.path.abspath(args.project)

    # The shadow hazard, checked on the RESOLVED corpus — fleet's own
    # resolution appends (a cwd that already ends in memories/ stays),
    # so the equality check must compare the resolved path against the
    # home's corpus, not a re-appended guess. This script's first run
    # misdirected six facts into ~/.fleet/memories/memories and then
    # into the home itself before the check was fixed — the hazard is
    # real, and the check is name-based on the resolved path.
    corpus = os.path.join(project, "memories")
    resolved = os.path.realpath(corpus)
    home_real = os.path.realpath(HOME_CORPUS)
    shadow = (
        resolved == home_real
        or resolved.startswith(home_real + os.sep)
        or home_real.startswith(resolved + os.sep)
    )
    if shadow:
        print(
            f"REFUSED: {resolved} is inside the fleet home's corpus ({home_real}) — "
            "the home-first rule (KI-087) makes the home corpus shadow every "
            "project corpus on the machine. The pack seeds a project's own "
            "memories/, never the home's.",
            file=sys.stderr,
        )
        sys.exit(2)

    os.makedirs(project, exist_ok=True)
    landed, refused, failed = 0, 0, 0
    for name in sorted(os.listdir(PACK)):
        if not name.endswith(".md"):
            continue
        parsed = parse_fact(os.path.join(PACK, name))
        if parsed is None:
            failed += 1
            print(f"FAIL {name}: unreadable frontmatter", file=sys.stderr)
            continue
        fields, body = parsed
        # The pack's `shape: [change, build]` is YAML-list shaped; the
        # writer wants the bare comma list (--shape splits on commas).
        shape = fields.get("shape", "").strip().strip("[]").strip()
        result = remember(
            {"name": fields["name"], "text": fields["description"]},
            body,
            fields.get("type", "feedback"),
            fields.get("basis", "inferred"),
            shape,
            project,
        )
        output = result.stdout + result.stderr
        if result.returncode == 0:
            landed += 1
        elif "already exists" in output:
            refused += 1
        else:
            failed += 1
            print(f"FAIL {fields['name']}: {output.strip()[:200]}", file=sys.stderr)
    print(
        f"house seed: {landed} landed, {refused} already in the store, "
        f"{failed} failed — corpus at {corpus}"
    )
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
