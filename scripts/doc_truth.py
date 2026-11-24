#!/usr/bin/env python3
"""Report every documentation claim that can be checked against the code.

The 2026-11-23 audit found the front door, the functional specification and
two architecture docs stating counts the code no longer held: nine lenses
against fifteen rail rows, five lanes against six, eight built-ins against
eleven, fourteen destinations against fifteen. Nothing failed, because a
number in prose had no producer and no test.

`src/drift.rs` owns the numbers the crate can produce. This checker is the
command form of the same audit: it reads each named doc statement and
compares it to the value the repository actually holds — the constant, the
collection's length, the declaration in the shipped asset, or `git`.

Every entry below names the doc, the line pattern, and the producer. A
pattern that stops matching is a **failure**, not a skip: a guard that
silently stops seeing its sentence is how the last round of drift survived.

Usage:
    python3 scripts/doc_truth.py            # report; exit 1 on any drift
    python3 scripts/doc_truth.py --json     # machine-readable
"""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def read(path):
    return (ROOT / path).read_text(encoding="utf-8")


# ---- the live values, read from their own declarations ------------------

def builtin_tools():
    """The built-in tool count, from `ALL_TOOLS`' declaration."""
    text = read("src/tools.rs")
    match = re.search(r"pub const ALL_TOOLS: \[Tool; (\d+)\]", text)
    if not match:
        raise SystemExit("doc_truth: ALL_TOOLS' declaration moved; update this producer")
    return int(match.group(1))


def lanes():
    """The vendored lane count, from `LANES`' own table."""
    text = read("src/lanes.rs")
    start = text.index("pub const LANES: &[Lane] = &[")
    body = text[start:]
    end = body.index("\n];")
    return len(re.findall(r'^\s{8}name: "', body[:end], re.M))


def rail_rows():
    """The rail rows the face paints, from `RAIL_MAP`'s declaration."""
    text = read("src/gui/assets/app.js")
    start = text.index("var RAIL_MAP = {")
    body = text[start:]
    body = body[:body.index("\n  };")]
    return sum(len(re.findall(r"'([^']+)'", rows))
               for rows in re.findall(r"rows: \[([^\]]*)\]", body))


def face_routes():
    """Every address the face resolves, from the `ROUTES` table."""
    text = read("src/gui/assets/app.js")
    start = text.index("var ROUTES = [")
    body = text[start:]
    body = body[:body.index("\n  ];")]
    return len(re.findall(r"^\s+\['", body, re.M))


def eval_scenarios():
    """The graded corpus's scenario count, from its `Scenario::new` calls.

    The corpus body is delimited by its closing `\n}` at column 0, not by
    brace depth: the closures passed to `Scenario::new` carry their own
    braces, so a depth scan closes on the first closure instead of the
    function (measured: it reported 0 against 22).
    """
    text = read("src/eval.rs")
    start = text.index("pub fn corpus() -> Vec<Scenario> {")
    end = text.index("\n}", start)
    return len(re.findall(r'Scenario::new\(\s*"', text[start:end]))


def diagram_pin(path):
    """The revision an Archify artifact is pinned to, from its own HTML."""
    text = read(path)
    match = re.search(r'"revision"\s*:\s*"([0-9a-f]{40})"', text)
    return match.group(1) if match else None


def commits_since(revision):
    """How many commits HEAD sits past `revision`, or None if unknown."""
    out = subprocess.run(
        ["git", "rev-list", "--count", f"{revision}..HEAD"],
        cwd=ROOT, capture_output=True, text=True,
    )
    return int(out.stdout.strip()) if out.returncode == 0 else None


# The diagrams may not lag HEAD by more than this many commits. The
# 2026-11-23 audit found them 345 behind, still in-bounds, silently
# mis-pointing every SRC badge. 25 is roughly a session's work: past it,
# the badges have stopped meaning what they claim.
DIAGRAM_LAG_LIMIT = 25

DIAGRAMS = [
    "docs/architecture/fleet-architecture.html",
    "docs/architecture/fleet-turn.html",
]


# ---- the statements -----------------------------------------------------

WORDS = {
    "one": 1, "two": 2, "three": 3, "four": 4, "five": 5, "six": 6,
    "seven": 7, "eight": 8, "nine": 9, "ten": 10, "eleven": 11,
    "twelve": 12, "thirteen": 13, "fourteen": 14, "fifteen": 15,
    "sixteen": 16, "seventeen": 17, "eighteen": 18, "nineteen": 19,
    "twenty": 20,
}


def as_number(word):
    word = word.strip("*`(.,").lower()
    if word in WORDS:
        return WORDS[word]
    return int(word) if word.isdigit() else None


def statements():
    """(doc, pattern, producer, what it says) for every checkable claim.

    The pattern's first group is the number the doc publishes.
    """
    return [
        # The built-in tool count.
        ("CLAUDE.md", r"the (\w+) built-ins, schemas and validation",
         builtin_tools, "built-in tools"),
        ("docs/features/tools.md", r"`fleet tools` lists the (\w+) built-ins",
         builtin_tools, "built-in tools"),
        ("docs/features/tools.md", r"^## The (\w+) built-ins",
         builtin_tools, "built-in tool section heading"),

        # The lane count.
        ("docs/architecture/lanes.md", r"built around (\w+) specialized",
         lanes, "vendored lanes"),
        ("docs/architecture/lanes.md", r"^## The (\w+) lanes",
         lanes, "lanes section heading"),
        ("services/README.md", r"^These (\w+) lanes",
         lanes, "vendored lanes"),

        # The rail's rows.
        ("README.md", r"(\w+) rail rows in\s+four regions",
         rail_rows, "rail rows"),
        ("FUNCTIONAL-SPECIFICATIONS.md", r"^(\w+) destinations in four regions",
         rail_rows, "rail rows"),

        # The eval corpus.
        ("CLAUDE.md", r"the eval corpus: (\d+) fixed scenarios",
         eval_scenarios, "eval scenarios"),
    ]


def check_diagrams():
    """(ok, detail) per diagram: it must be pinned and not lag HEAD."""
    results = []
    for path in DIAGRAMS:
        pin = diagram_pin(path)
        if pin is None:
            results.append((False,
                            f"{path}: carries no pinned revision — its SRC badges "
                            f"point at nothing checkable"))
            continue
        lag = commits_since(pin)
        if lag is None:
            results.append((False,
                            f"{path}: pinned to {pin[:8]}, which is not in this history"))
            continue
        if lag > DIAGRAM_LAG_LIMIT:
            results.append((False,
                            f"{path}: pinned to {pin[:8]}, {lag} commits behind HEAD "
                            f"(limit {DIAGRAM_LAG_LIMIT}) — every SRC badge has drifted"))
        else:
            results.append((True, f"{path}: pinned to {pin[:8]}, {lag} commit(s) behind"))
    return results


def check(entry):
    """(ok, detail) for one statement — a missing pattern is a failure."""
    doc, pattern, producer, says = entry
    try:
        text = read(doc)
    except (FileNotFoundError, UnicodeDecodeError) as error:
        return False, f"{doc} is unreadable: {error}", says
    match = re.search(pattern, text, re.M)
    if not match:
        return False, (
            f"{doc}: no line matches the `{says}` statement any more — the sentence "
            f"moved, so it is no longer guarded"
        ), says
    published = as_number(match.group(1))
    if published is None:
        return False, f"{doc}: `{match.group(1)}` is not a number ({says})", says
    live = producer()
    if published != live:
        line = " ".join(match.group(0).split())[:110]
        return False, (
            f"{doc}: states {published} {says} but the code holds {live} — {line}"
        ), says
    return True, f"{doc}: {says} = {live}", says


def main(argv):
    entries = statements()
    results = [(entry, *check(entry)) for entry in entries]
    diagrams = check_diagrams()

    if "--json" in argv:
        payload = [
            {"doc": e[0], "says": says, "ok": ok, "detail": detail}
            for e, ok, detail, says in results
        ]
        payload += [
            {"doc": "diagram", "says": "pinned revision", "ok": ok, "detail": detail}
            for ok, detail in diagrams
        ]
        print(json.dumps(payload, indent=2))
    else:
        failures = 0
        for entry, ok, detail, says in results:
            if ok:
                continue
            failures += 1
            print(f"drift: {detail}")
        for ok, detail in diagrams:
            if not ok:
                failures += 1
                print(f"stale: {detail}")
        total = len(results) + len(diagrams)
        if failures == 0:
            print(f"doc truth: {len(results)} statement(s) and {len(diagrams)} diagram(s) "
                  f"checked, all agree with the code")
        else:
            print(f"\n{failures} of {total} check(s) drifted.")
    return 1 if (any(not ok for _, ok, _, _ in results)
                 or any(not ok for ok, _ in diagrams)) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
