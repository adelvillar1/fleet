#!/usr/bin/env python3
"""The memory transplant's batch procedure (C6 of the transplant plan).

Reads the committed census, takes its `transplant` rows (the screened
rows wait for the owner's nod — this script never touches them), orders
them by the date their description carries, and lands each through the
deployed `fleet remember` — the writer whose validation is the only
frontmatter authority. The writer's own collision refusal makes the run
idempotent: a fact already in the store refuses by name and the batch
continues.

Usage:
  python3 -I scripts/memory-transplant-batches.py --start 0 --size 40   # one batch
  python3 -I scripts/memory-transplant-batches.py --plan                # list the batches
"""

import argparse
import os
import re
import subprocess
import sys

CENSUS = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "docs", "plans", "2026-11-04-memory-transplant-census.md",
)
SOURCE_DIR = os.path.expanduser(
    "~/.zcode/cli/memories/projects/fleet-e2ba136ae3eba04b/memory"
)
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FLEET = os.path.expanduser("~/.fleet/bin/fleet")

DATE = re.compile(r"(20\d{2}-\d{2}-\d{2})")


def census_rows(dispositions):
    """The census's listing rows whose disposition is named, as
    (slug, type)."""
    rows = []
    with open(CENSUS, encoding="utf-8") as handle:
        for line in handle:
            if not line.startswith("| "):
                continue
            cells = [c.strip() for c in line.split("|")]
            # The listing's rows carry four cells (slug, type,
            # disposition, reason); the tally's carry two — the length
            # check keeps the tally out.
            if len(cells) < 5 or cells[3] not in dispositions:
                continue
            rows.append((cells[1], cells[2]))
    return rows


def census_transplants():
    return census_rows({"transplant"})


def slug_paths():
    """The source corpus's slug -> filename map: the frontmatter's name
    is the slug, the filename may carry a type prefix."""
    paths = {}
    for name in sorted(os.listdir(SOURCE_DIR)):
        if not name.endswith(".md") or name == "MEMORY.md":
            continue
        with open(os.path.join(SOURCE_DIR, name), encoding="utf-8") as handle:
            text = handle.read(4096)
        match = re.search(r"^name:\s*(.+)$", text, re.M)
        slug = match.group(1).strip().strip("'\"") if match else name[: -len(".md")]
        paths[slug] = os.path.join(SOURCE_DIR, name)
    return paths


SLUG_PATHS = slug_paths()


def read_fact(slug):
    """The source fact's folded description and body."""
    path = SLUG_PATHS.get(slug)
    if path is None:
        return None, None
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    end = text.find("\n---", 3)
    if end < 0 or not text.startswith("---"):
        return None, None
    block = text[3:end]
    body = text[end + 4 :].strip()
    desc_lines = []
    in_desc = False
    for line in block.split("\n"):
        if line.startswith("description:"):
            in_desc = True
            desc_lines.append(line[len("description:") :].strip())
        elif in_desc and line[:1] in (" ", "\t"):
            desc_lines.append(line.strip())
        elif in_desc:
            break
    description = " ".join(desc_lines).strip().strip('"')
    return description, body


def basis_for(description):
    lowered = description.lower()
    if "ruled" in lowered or "owner" in lowered or "the owner's" in lowered:
        return "confirmed-by"
    return "inferred"


def date_of(description, slug):
    match = DATE.search(description)
    return match.group(1) if match else f"9999-{slug}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--size", type=int, default=40)
    parser.add_argument("--plan", action="store_true")
    parser.add_argument(
        "--screened", action="store_true",
        help="land the census's screened rows (the owner's nod is recorded in the plan)",
    )
    args = parser.parse_args()

    rows = census_rows({"transplant", "screen"} if args.screened else {"transplant"})
    dated = [(date_of(read_fact(slug)[0] or "", slug), slug, ftype) for slug, ftype in rows]
    dated.sort()
    batches = [dated[i : i + args.size] for i in range(0, len(dated), args.size)]
    if args.plan:
        for number, batch in enumerate(batches):
            print(f"batch {number}: {len(batch)} facts, {batch[0][0]} → {batch[-1][0]}")
        print(f"{len(dated)} facts in {len(batches)} batches")
        return

    batch = batches[args.start]
    landed, refused, failed = 0, 0, 0
    for _, slug, ftype in batch:
        description, body = read_fact(slug)
        if description is None:
            failed += 1
            print(f"FAIL {slug}: unreadable source", file=sys.stderr)
            continue
        cmd = [
            FLEET, "remember",
            "--name", slug,
            "--description", description,
            "--type", ftype,
            "--basis", basis_for(description),
            "--body", body,
        ]
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO)
        output = result.stdout + result.stderr
        if result.returncode == 0:
            landed += 1
        elif "already exists" in output:
            refused += 1
        else:
            failed += 1
            print(f"FAIL {slug}: {output.strip()[:200]}", file=sys.stderr)
    print(f"batch {args.start}: {landed} landed, {refused} already in the store, {failed} failed")


if __name__ == "__main__":
    main()
