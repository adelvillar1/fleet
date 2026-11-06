#!/usr/bin/env python3
"""List every completed plan whose acceptance criteria still carry open boxes.

The plans README says the plan file is the source of truth for whether
something is done, and a plan moves to `completed` only when every
acceptance criterion is confirmed met. KI-398 found the other shape: work
that shipped with its boxes never ticked, so the plan answered neither
"what shipped" nor "what did not". This check makes the next sweep a
command instead of an audit.

Scope: only the boxes under a `## Acceptance criteria` heading, at column 0
or nested one level under a sub-bullet. A plan's Goal or use-case checkboxes
are not criteria, and counting them inflates the list (the first sweep did,
by 46). A plan with no `status` line reads as not-completed and is skipped
rather than guessed at — the plans README makes the frontmatter mandatory, and
the seventeen inherited fork records that predate it all carry zero open
criteria, so treating them as unknown costs nothing.

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


# A criterion box at column 0 or nested under a sub-bullet (`  - [ ]`), with any
# of the three list markers. Four or more spaces would be a code block, but a
# real one in these plans is always fenced, and the fenced spans are skipped
# below — so the indent bound is what nested criteria actually use, not a syntax
# guess. An indented box is still a criterion: the docstring's promise is the
# boxes UNDER the heading, and the first version silently read only column 0.
BOX = re.compile(r"^\s{0,6}[-*+]\s+\[ \]\s+(.*)$")


def open_criteria(text):
    """The `- [ ]` boxes under each Acceptance criteria heading, fences skipped."""
    found = []
    parts = re.split(r"^(## .+)$", text, flags=re.M)
    for index in range(1, len(parts) - 1, 2):
        heading, body = parts[index], parts[index + 1]
        if "acceptance criteria" not in heading.lower():
            continue
        fenced = False
        for line in body.splitlines():
            if line.lstrip().startswith("```"):
                fenced = not fenced
                continue
            if fenced:
                continue
            match = BOX.match(line)
            if match:
                found.append(match.group(1).strip())
    return found


def main(plans_dir):
    root = pathlib.Path(plans_dir)
    offenders = []
    unstated = []
    for plan in sorted(root.glob("*.md")):
        text = plan.read_text(encoding="utf-8")
        status = status_of(text)
        open_boxes = open_criteria(text)
        if status in DONE:
            if open_boxes:
                offenders.append((plan.name, open_boxes))
        elif status == "?" and open_boxes:
            # No declared state and work still open: the plan answers neither
            # "what shipped" nor "what did not", which is KI-398's own defect
            # with the status cell empty instead of wrong. The legacy records
            # that predate the frontmatter rule carry zero open boxes, so this
            # costs nothing on the existing corpus.
            unstated.append((plan.name, open_boxes))
    if not offenders and not unstated:
        print("every completed plan's acceptance criteria are ticked")
        return 0
    for name, boxes in unstated:
        print(f"  {name}: {len(boxes)} open and no status line")
        print(f"    first: {boxes[0][:110]}")
    if offenders:
        total = sum(len(boxes) for _, boxes in offenders)
        print(f"{len(offenders)} completed plan(s) carry {total} unticked criteria:")
        for name, boxes in offenders:
            print(f"  {name}: {len(boxes)} open")
            print(f"    first: {boxes[0][:110]}")
    if unstated:
        print(f"{len(unstated)} plan(s) carry unticked criteria and no status line.")
    print("\nTick what shipped with its evidence, or record the residual and its owner (KI-398).")
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "docs/plans"))
