#!/usr/bin/env python3
"""The memory transplant's census (docs/plans/2026-11-04-memory-transplant.md).

Reads the two corpora — the harness session memory (the source) and
Fleet's own git-tracked fact substrate (the target) — and emits the
committed census: one line per source fact, a disposition, and the
reason. Deterministic by construction: every listing is sorted, nothing
timestamps, and a re-run on an unchanged corpus reproduces the document
byte-for-byte.

Dispositions:
  transplant   the fact's subject is Fleet's domain; it lands through
               `fleet remember` in a batch.
  skip-harness the fact's subject is this harness's own behavior (the
               named class below); the knowledge is the harness's, not
               Fleet's.
  superseded   the fact's own description names a superseding fact; the
               chain trims to its tip.
  collision    the slug exists in Fleet's corpus already; the census
               quotes both and the batch resolves by reading them.
  merge        the description overlaps a Fleet fact above the named
               threshold; both sides are quoted for the batch to judge.
  screen       the body carries something the public repo must be asked
               about first: a machine path, a key-shaped token, or
               sibling-repo internals deeper than the corpus carries.

Run:  python3 -I scripts/memory-transplant-census.py > docs/plans/2026-11-04-memory-transplant-census.md
"""

import os
import re
import sys

# The source: the harness session memory for the fleet project.
SOURCE_DIR = os.path.expanduser(
    "~/.zcode/cli/memories/projects/fleet-e2ba136ae3eba04b/memory"
)
# The target: Fleet's own fact substrate, at the repo root.
TARGET_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "memories")

# The named skip class (C2): the harness-subject facts, by slug. A fact
# lands here only when its subject is THIS harness's behavior — the
# chat surface's quirks, the harness's own model routing, the session
# machinery — never when the subject is Fleet or the ecosystem. The
# list is quotable in the census; a candidate the list misses but the
# keyword heuristic flags reads as `screen`, never a silent skip.
HARNESS_SUBJECT_SLUGS = {
    "long-replies-get-swallowed-by-the-owners-chat",
    "the-router-picks-the-model-for-usage",
    "a-per-turn-log-record-breaks-eval-count-assertions",
    "measuring-a-mockup-by-reading-the-png",
    "user-stated-inference-quota",
}

# Description markers that flag a possible harness subject for review
# rather than auto-skipping: the census names them, the batch decides.
HARNESS_MARKERS = [
    "the harness's",
    "this harness",
    "the owner's chat",
    "the composer",
    "context window",
    "token budget for the session",
]

# The screening patterns (C4): anything the public repo needs the
# owner's nod for before it lands.
SCREEN_PATTERNS = [
    (r"/Users/[a-z0-9_]+/", "machine path"),
    (r"~/\.", "home-relative path"),
    (r"\bsk-[A-Za-z0-9]{8,}", "key-shaped token"),
    (r"[A-Z_]{4,}_(KEY|TOKEN|SECRET)\s*=", "key assignment"),
    (r"jobhound/apps/worker", "jobhound worker internals"),
    (r"api\.typesafe\.ai|api\.fastino\.ai|drex\.nace\.ai", "provider endpoint"),
]

# The dedup threshold (C3): Jaccard overlap on description tokens above
# which a source fact reads as `merge` against a Fleet fact.
MERGE_THRESHOLD = 0.5

TOKEN_SPLIT = re.compile(r"[^a-z0-9]+")


def frontmatter(text):
    """The fact's frontmatter as a flat dict: the block between the
    first two --- fences, `key: value` lines, the metadata nesting
    flattened one level (metadata.type -> type)."""
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    block = text[3:end]
    fields = {}
    current = None
    for line in block.split("\n"):
        if not line.strip() or line.strip().startswith("#"):
            continue
        if line[:1] not in (" ", "\t"):
            key, _, value = line.partition(":")
            current = key.strip()
            fields[current] = value.strip().strip("\"'")
        elif current:
            sub, _, value = line.strip().partition(":")
            fields[f"{current}.{sub.strip()}"] = value.strip().strip("\"'")
    return fields


def tokens(text):
    return {t for t in TOKEN_SPLIT.split(text.lower()) if len(t) > 2}


def jaccard(a, b):
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


def load_corpus(directory):
    corpus = {}
    if not os.path.isdir(directory):
        return corpus
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".md") or name == "MEMORY.md":
            continue
        path = os.path.join(directory, name)
        if not os.path.isfile(path):
            continue
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        fields = frontmatter(text)
        slug = fields.get("name") or name[: -len(".md")]
        corpus[slug] = {
            "description": fields.get("description", ""),
            "type": fields.get("type", fields.get("metadata.type", "")),
            "body": text,
        }
    return corpus


def screen(body):
    """The screening matches, as `reason` fragments."""
    found = []
    for pattern, label in SCREEN_PATTERNS:
        if re.search(pattern, body):
            found.append(label)
    return found


def main():
    source = load_corpus(SOURCE_DIR)
    target = load_corpus(TARGET_DIR)
    target_tokens = {slug: tokens(fact["description"]) for slug, fact in target.items()}

    rows = []
    counts = {}
    for slug in sorted(source):
        fact = source[slug]
        description = fact["description"]
        body = fact["body"]
        reasons = []

        if slug in HARNESS_SUBJECT_SLUGS:
            counts["skip-harness"] = counts.get("skip-harness", 0) + 1
            rows.append((slug, fact["type"], "skip-harness", "the named harness-subject class"))
            continue

        if "SUPERSEDED" in description.upper() or "superseded" in description.lower():
            tip = re.search(r"\[\[([a-z0-9-]+)\]\]", description)
            reason = "the description names a superseding fact"
            if tip:
                reason += f": {tip.group(1)}"
            counts["superseded"] = counts.get("superseded", 0) + 1
            rows.append((slug, fact["type"], "superseded", reason))
            continue

        if slug in target:
            counts["collision"] = counts.get("collision", 0) + 1
            rows.append((
                slug,
                fact["type"],
                "collision",
                f"the slug exists in Fleet's corpus ({target[slug]['description'][:80]})",
            ))
            continue

        flagged = screen(body)
        harness_flag = any(marker in description.lower() for marker in HARNESS_MARKERS)

        merge_with = None
        merge_score = 0.0
        mine = tokens(description)
        for other_slug, other_tokens in target_tokens.items():
            score = jaccard(mine, other_tokens)
            if score > merge_score:
                merge_score = score
                merge_with = other_slug

        if flagged:
            counts["screen"] = counts.get("screen", 0) + 1
            rows.append((
                slug,
                fact["type"],
                "screen",
                " + ".join(flagged) + ("; possible harness subject" if harness_flag else ""),
            ))
        elif merge_score >= MERGE_THRESHOLD:
            counts["merge"] = counts.get("merge", 0) + 1
            rows.append((
                slug,
                fact["type"],
                "merge",
                f"{merge_score:.2f} overlap with `{merge_with}` ({target[merge_with]['description'][:60]})",
            ))
        elif harness_flag:
            counts["screen"] = counts.get("screen", 0) + 1
            rows.append((slug, fact["type"], "screen", "possible harness subject — the batch decides"))
        else:
            counts["transplant"] = counts.get("transplant", 0) + 1
            basis = "confirmed-by" if "ruled" in description.lower() or "owner" in description.lower() else "inferred"
            rows.append((slug, fact["type"], "transplant", f"basis {basis}"))

    out = sys.stdout
    out.write("# The memory transplant's census\n\n")
    out.write("Generated by `scripts/memory-transplant-census.py` (deterministic; re-run it\n")
    out.write("on an unchanged corpus and this document reproduces byte-for-byte).\n\n")
    out.write("## The tally\n\n")
    out.write("| disposition | facts |\n|---|---|\n")
    for disposition in ["transplant", "skip-harness", "superseded", "collision", "merge", "screen"]:
        out.write(f"| {disposition} | {counts.get(disposition, 0)} |\n")
    out.write(f"| **total** | **{len(source)}** |\n\n")
    out.write("## The listing\n\n")
    out.write("| slug | type | disposition | reason |\n|---|---|---|---|\n")
    order = {"skip-harness": 0, "superseded": 1, "collision": 2, "merge": 3, "screen": 4, "transplant": 5}
    for slug, fact_type, disposition, reason in sorted(rows, key=lambda r: (order[r[2]], r[0])):
        reason = reason.replace("|", "\\|")
        out.write(f"| {slug} | {fact_type or '—'} | {disposition} | {reason} |\n")
    out.write("\n## The rules this ran under\n\n")
    out.write(f"- The named harness-subject class: {len(HARNESS_SUBJECT_SLUGS)} slugs, listed in the script.\n")
    out.write(f"- The merge threshold: {MERGE_THRESHOLD} Jaccard on description tokens.\n")
    out.write(f"- The screening patterns: {len(SCREEN_PATTERNS)}, named in the script.\n")


if __name__ == "__main__":
    main()
