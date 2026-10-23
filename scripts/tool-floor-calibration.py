#!/usr/bin/env python3
"""Calibrate tool_search's FLOOR and MARGIN against the live embedding
service.

Run against the local llama-server that backs sem1:

    python3 scripts/tool-floor-calibration.py
    FLEET_EMBED_URL=http://127.0.0.1:8901/v1/embeddings python3 scripts/tool-floor-calibration.py

The script prints the corpus's top-3 cosines, the true-match and no-tool
score bounds, and a floor sweep. The numbers recorded in
docs/features/tools.md came from this script.
"""
import json
import math
import os
import urllib.request

URL = os.environ.get("FLEET_EMBED_URL", "http://127.0.0.1:8901/v1/embeddings")


def embed(texts):
    body = json.dumps({"input": texts}).encode()
    request = urllib.request.Request(
        URL, data=body, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        rows = json.load(response)["data"]
    rows.sort(key=lambda row: row["index"])
    return [row["embedding"] for row in rows]


def cosine(left, right):
    dot = sum(a * b for a, b in zip(left, right))
    left_norm = math.sqrt(sum(a * a for a in left))
    right_norm = math.sqrt(sum(b * b for b in right))
    return dot / (left_norm * right_norm) if left_norm and right_norm else 0.0


def measure(name, cards, queries):
    texts = [query for query, _, _ in queries] + cards
    vectors = embed(texts)
    query_vectors = vectors[: len(queries)]
    card_vectors = vectors[len(queries) :]
    print(f"\n== {name}: {len(queries)} queries, {len(cards)} cards ==")
    true_tops, no_tool_tops, margins = [], [], []
    for (query, expected, _), vector in zip(queries, query_vectors):
        scored = sorted(
            ((cosine(vector, card_vector), card) for card, card_vector in zip(cards, card_vectors)),
            reverse=True,
        )
        top = scored[:3]
        margin = top[0][0] - top[1][0]
        if expected is None:
            no_tool_tops.append(top[0][0])
            print(f"  none top={top[0][0]:.3f} margin={margin:.3f} | {query} | {top[0][1].split(' — ')[0]}")
            continue
        rank = next((index + 1 for index, (_, card) in enumerate(scored) if card == expected), None)
        expected_score = next(score for score, card in scored if card == expected)
        true_tops.append(top[0][0])
        margins.append(margin)
        print(
            f"  rank={rank} top={top[0][0]:.3f} expected={expected_score:.3f} "
            f"margin={margin:.3f} | {query} | {top[0][1].split(' — ')[0]}"
        )
    if true_tops:
        print(
            f"  true top min={min(true_tops):.3f} median={sorted(true_tops)[len(true_tops)//2]:.3f}"
        )
        print(f"  margin min={min(margins):.3f} median={sorted(margins)[len(margins)//2]:.3f}")
    if no_tool_tops:
        print(
            f"  no-tool top min={min(no_tool_tops):.3f} "
            f"median={sorted(no_tool_tops)[len(no_tool_tops)//2]:.3f} max={max(no_tool_tops):.3f}"
        )
    return true_tops, no_tool_tops


# The plan's corpus: 40 tools, 24 queries, each naming one tool's token.
plan_cards = [f"tool c{i} — capability token{i} work" for i in range(40)]
plan_queries = [(f"do token{i} now", f"tool c{i} — capability token{i} work", None) for i in range(24)]
measure("plan corpus", plan_cards, plan_queries)

CARDS = [
    "bash — Run a shell command and return its output. Use timeout_ms for a long command; set run_in_background for a server, watch, or build that outlives the turn.",
    "read_file — Read a text file with line numbers. Returns a window; use offset and limit to continue; read a file before you edit it.",
    "write_file — Create or replace a whole file, atomically. Use edit for a change to part of an existing file.",
    "list_dir — List a directory's entries, directories marked with a trailing slash; depth lists nested levels.",
    "edit — Replace one exact passage in a file you have already read. old_string must match byte for byte and appear exactly once.",
    "grep — Search file contents. Results are capped, counted, and say when they were cut; output_mode files, content, or count.",
    "glob — Find files by path pattern. Supports * and **; newest first.",
    "todo — Replace the run's task list; status pending, in_progress, or completed.",
    "use_skill — Load a skill's body into context by name.",
    "remember — Write one durable fact about the owner or the project into the memory corpus.",
    "supersede — Modify or forget an existing durable fact.",
    "subagent — Hand one self-contained task to a child agent with its own context.",
    "convention — Ask what this project already calls a concept before you write it.",
    "plan — Draft a plan document's declaration; never approve it.",
    "deck — Build a presentation, infographic, or poster from a declaration.",
    "vision — Ask a question about one or more images on disk.",
    "web — Search the live web and fetch pages, with excerpts and URLs.",
    "computer_use — Drive the desktop: screenshot, click, type, press keys, scroll, drag.",
    "mcp__filesystem__read_text_file — Read a text file from an allowed directory.",
    "mcp__github__create_pull_request — Create a pull request in a repository.",
]
BY_NAME = {card.split(" — ")[0]: card for card in CARDS}
REAL = [
    ("read the file src/main.rs", BY_NAME["read_file"]),
    ("show me the first 100 lines of Cargo.toml", BY_NAME["read_file"]),
    ("change one line in src/lib.rs", BY_NAME["edit"]),
    ("find every place TODO appears in the codebase", BY_NAME["grep"]),
    ("which files match src/**/*.rs", BY_NAME["glob"]),
    ("list what is in the src directory", BY_NAME["list_dir"]),
    ("run the test suite", BY_NAME["bash"]),
    ("start a dev server and leave it running", BY_NAME["bash"]),
    ("search the web for the latest Rust release", BY_NAME["web"]),
    ("remember that the owner prefers tabs to spaces", BY_NAME["remember"]),
    ("what does this project call a route decision", BY_NAME["convention"]),
    ("look at this screenshot and tell me what is wrong", BY_NAME["vision"]),
    ("break this into a task list", BY_NAME["todo"]),
    ("hand this refactor to another agent", BY_NAME["subagent"]),
    ("make a slide deck about the architecture", BY_NAME["deck"]),
    ("write a file at notes/today.md", BY_NAME["write_file"]),
]
NO_TOOL = [
    "book a flight to Lisbon next Friday",
    "what is the weather tomorrow",
    "tell me a joke",
    "who won the 2018 World Cup",
    "translate good morning into French",
    "what is the capital of Australia",
]
true_tops, no_tool_tops = measure(
    "realistic tools",
    CARDS,
    [(query, expected, None) for query, expected in REAL] + [(query, None, None) for query in NO_TOOL],
)

print("\n== floor sweep ==")
for floor in (0.35, 0.55, 0.62, 0.64, 0.66, 0.68, 0.70):
    passed = sum(1 for score in true_tops if score >= floor)
    abstained = sum(1 for score in no_tool_tops if score < floor)
    print(f"  floor={floor:.2f}: true pass {passed}/{len(true_tops)}, no-tool abstain {abstained}/{len(no_tool_tops)}")
