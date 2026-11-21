#!/usr/bin/env python3
"""Apply Fleet's confirmed judgments to Laya's Learner — batch-only, in-process.

Fleet exports a day's `route_judged` rows that carry an owner disposition, each
joined to the route row it grades and re-rendered as the evidence record the
judge was asked about. This script feeds the `agreed` ones to Laya's `Learner`
and reports what it did.

stdin:  {"records": [{"id": "<route_key>", "state": "<evidence record>",
                      "gold": {"workload": "<label>"}}, ...]}
stdout: {"applied": n, "skipped": n, "refused": n, "learned": bool, ...}

Three rules, and why:

- **Batch-only, never in a loop.** Nothing on a synchronous path calls this.
  The model load is the expensive part (measured 4.3 GiB peak, ~4 s), so a
  per-decision call would be absurd; a day's confirmed rows applied at once is
  the shape the vendor's own guide describes.
- **Idempotent by record id.** Laya's Learner has no record ids of its own —
  it stores features and labels — so the ledger beside the learner's state
  (`applied.json`) is what makes a second run a no-op. Without it, re-running
  the batch would teach the same answers twice.
- **`agreed` only.** An `agreed` row is the owner confirming the lane's own
  label, which is a gold answer. A `disagreed` row says the label was wrong
  without saying what was right, and a Learner cannot be taught "not this";
  a `waived` row says nothing at all. Fleet counts both and never sends them.

Exit code: 0 when the batch was read and the ledger written, whatever it
applied — a batch of all-skips is a successful no-op. 2 when the input is not
the contract (so a caller can tell "nothing to do" from "wrong shape").
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from typing import NoReturn

LEDGER = "applied.json"


def _fail(message: str) -> NoReturn:
    sys.stderr.write(f"laya_learn: {message}\n")
    raise SystemExit(2)


def _questions_for_workload():
    """The workload head's laya question spec — the same builder the wire uses."""
    from sys1 import providers
    from sys1.tasks import get_task

    task = get_task("route")
    head = next((h for h in task.heads if h.id == "workload"), None)
    if head is None:
        _fail("the route task carries no workload head")
    questions = providers.laya_questions([head])
    if "workload" not in questions:
        _fail("the workload head has no labels, so it cannot be asked")
    return head, questions


def _read_ledger(workdir: str) -> set[str]:
    path = os.path.join(workdir, LEDGER)
    try:
        with open(path, encoding="utf-8") as handle:
            raw = json.load(handle)
    except FileNotFoundError:
        return set()
    except (OSError, json.JSONDecodeError):
        # A ledger that cannot be read is not an empty ledger: it is the
        # record of what was already learned, and pretending it is empty
        # would double-count the whole history. Refuse by name.
        _fail(f"the ledger at {path} could not be read — refusing to re-teach")
    ids = raw.get("ids") if isinstance(raw, dict) else None
    if not isinstance(ids, list):
        _fail(f"the ledger at {path} is not the contract (no `ids` list)")
    return {str(item) for item in ids}


def _write_ledger(workdir: str, ids: set[str]) -> None:
    path = os.path.join(workdir, LEDGER)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump({"ids": sorted(ids)}, handle)
    os.replace(tmp, path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--workdir",
        required=True,
        help="where the learner's state and the applied-ids ledger live",
    )
    parser.add_argument(
        "--every",
        type=int,
        default=8,
        help="learn after this many confirmed rows (the batch flushes at the end)",
    )
    args = parser.parse_args()

    sys.path.insert(0, os.environ.get("SYS1_PYTHONPATH", ""))

    payload = json.load(sys.stdin)
    records = payload.get("records") if isinstance(payload, dict) else None
    if not isinstance(records, list):
        _fail("stdin must be {\"records\": [...]}")

    workdir = os.path.abspath(os.path.expanduser(args.workdir))
    os.makedirs(workdir, exist_ok=True)

    head, questions = _questions_for_workload()
    applied_ids = _read_ledger(workdir)

    report = {
        "applied": 0,
        "skipped": 0,
        "refused": 0,
        "learned": False,
        "workdir": workdir,
        "question": head.id,
        "labels": list(head.labels),
    }
    fresh: list[tuple[str, str, str]] = []
    seen: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            report["refused"] += 1
            continue
        rid = record.get("id")
        state = record.get("state")
        gold = record.get("gold") or {}
        label = gold.get("workload") if isinstance(gold, dict) else None
        if not rid or not state or not isinstance(label, str) or not label:
            report["refused"] += 1
            continue
        if label not in head.labels:
            # A label outside the head's declared set cannot be a gold answer
            # for this question: it would teach the learner an option the
            # wire never offers.
            report["refused"] += 1
            continue
        if rid in applied_ids or rid in seen:
            report["skipped"] += 1
            continue
        seen.add(rid)
        fresh.append((str(rid), str(state), label))

    if not fresh:
        _write_ledger(workdir, applied_ids)
        print(json.dumps(report))
        return

    import laya  # the venv this script runs in is the laya venv

    learner = laya.Learner(
        questions=questions,
        workdir=workdir,
        every=max(1, args.every),
        # No background thread: this is a batch, and a process that exits
        # mid-learn is a batch that did not land. `flush()` learns now.
        auto=False,
    )
    for rid, state, label in fresh:
        try:
            learner.feedback(state, {"workload": label})
        except Exception as exc:  # a row the learner refuses is named, not fatal
            sys.stderr.write(f"laya_learn: refused {rid}: {exc}\n")
            report["refused"] += 1
            continue
        report["applied"] += 1
        applied_ids.add(rid)

    if report["applied"]:
        learner.flush()
        report["learned"] = True
        report["pending"] = learner.pending()
        # `Store.counts` is per question `{source: n}`; the total is what a
        # card can print.
        counts = learner.store.counts(next(iter(questions)))
        report["rows"] = sum(counts.values()) if isinstance(counts, dict) else int(counts)
    # The ledger is written AFTER the learning lands, so a crash between the
    # two replays the batch rather than losing it. The window is named rather
    # than closed: Laya's Learner keeps no record ids, so a crash inside
    # `flush()` can only be resolved by the operator, not by this script.
    _write_ledger(workdir, applied_ids)
    print(json.dumps(report))


if __name__ == "__main__":
    main()
