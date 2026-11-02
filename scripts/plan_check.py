#!/usr/bin/env python3
"""List every completed plan whose acceptance criteria still carry open boxes.

The plans README says the plan file is the source of truth for whether
something is done, and a plan moves to `completed` only when every
acceptance criterion is confirmed met. KI-398 found the other shape: work
that shipped with its boxes never ticked, so the plan answered neither
"what shipped" nor "what did not". This check makes the next sweep a
command instead of an audit.

Scope: only the `- [ ]` boxes under a `## Acceptance criteria` heading.
A plan's Goal or use-case checkboxes are not criteria, and counting them
inflates the list (the first sweep did, by 46).

Exit 0 with every completed plan clean; exit 1 naming the plans that are
not, with the count and the first open criterion's own words.
"""
import pathlib
import re
import sys

DONE = {"completed", "complete", "done", "landed-pushed", "landed-pending-review"}


def status_of(text):
    match = re.search(r"^status:\s*(\S+)", text, re.M)
    return match.group(1) if match else "?"


def open_criteria(text):
    """The `- [ ]` boxes under each Acceptance criteria heading."""
    found = []
    parts = re.split(r"^(## .+)$", text, flags=re.M)
    for index in range(1, len(parts) - 1, 2):
        heading, body = parts[index], parts[index + 1]
        if "acceptance criteria" not in heading.lower():
            continue
        for line in body.splitlines():
            if line.startswith("- [ ] "):
                found.append(line[len("- [ ] "):].strip())
    return found


def main(plans_dir):
    root = pathlib.Path(plans_dir)
    offenders = []
    for plan in sorted(root.glob("*.md")):
        text = plan.read_text(encoding="utf-8")
        if status_of(text) not in DONE:
            continue
        open_boxes = open_criteria(text)
        if open_boxes:
            offenders.append((plan.name, open_boxes))
    if not offenders:
        print("every completed plan's acceptance criteria are ticked")
        return 0
    total = sum(len(boxes) for _, boxes in offenders)
    print(f"{len(offenders)} completed plan(s) carry {total} unticked criteria:")
    for name, boxes in offenders:
        print(f"  {name}: {len(boxes)} open")
        print(f"    first: {boxes[0][:110]}")
    print("\nTick what shipped with its evidence, or record the residual and its owner (KI-398).")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "docs/plans"))
