"""
Minimal CLI companion to the sdm1 library.

Only what the library itself owns: score a CSV, print a health snapshot,
list backends, show the version. Anything app-aware (dashboards, pipelines)
stays in the consuming project — keeping sdm1 a clean, app-agnostic
dependency.
"""

from __future__ import annotations

import argparse
import json
import sys as _sys

from . import __version__, classify_csv, health_report
from .registry import all_ids, get
from .types import Sdm1Error


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="sdm1", description="Structured-data model abstraction.")
    sub = p.add_subparsers(dest="cmd", required=True)

    c = sub.add_parser("classify", help="Score a CSV's unlabeled rows using its labeled rows as context.")
    c.add_argument("--input", "-i", required=True, help="path to a CSV file")
    c.add_argument("--target", "-t", required=True, help="column to predict (null on query rows)")
    c.add_argument("--task-type", choices=["classification", "regression", "forecast", "anomaly"], default="classification")
    c.add_argument("--model", "-m", default="nvidia-sdm", help="backend id from the registry")
    c.add_argument("--num-estimators", type=int, default=8)
    c.add_argument("--floor", type=float, default=None, help="request-level confidence floor override")
    c.add_argument("--table-name", default=None, help="identifier for the result/log record")
    c.add_argument("--no-log", action="store_true", help="skip the JSONL calibration log")
    c.add_argument("--json", "-j", action="store_true", help="emit raw JSON instead of a human summary")
    c.set_defaults(func=_cmd_classify)

    d = sub.add_parser("doctor", help="Backend availability snapshot (subprocess probe, never imports torch).")
    d.add_argument("--json", "-j", action="store_true", help="emit JSON")
    d.set_defaults(func=_cmd_doctor)

    m = sub.add_parser("models", help="List registered backends.")
    m.add_argument("--json", "-j", action="store_true", help="emit JSON")
    m.set_defaults(func=_cmd_models)

    v = sub.add_parser("version", help="Print version and exit.")
    v.set_defaults(func=lambda _a: print(__version__) or 0)

    return p


def _cmd_classify(args: argparse.Namespace) -> int:
    try:
        result = classify_csv(
            args.input,
            target=args.target,
            task_type=args.task_type,
            model=args.model,
            num_estimators=args.num_estimators,
            confidence_floor=args.floor,
            table_name=args.table_name or args.input,
            log=not args.no_log,
        )
    except (Sdm1Error, ValueError, OSError) as e:
        print(f"error: {e}", file=_sys.stderr)
        return 2
    if args.json:
        print(json.dumps(result.as_dict(), indent=2, default=str))
    else:
        print(
            f"table: {result.table}  task: {result.task}  model: {','.join(result.models_used) or '(none)'}"
        )
        print(
            f"verdict: {result.verdict}  escalated: {result.escalated}  "
            f"rows: {result.n_query_rows} (context {result.n_context_rows})  latency: {result.latency_ms}ms"
        )
        print("\n".join(result.summary))
    return 0 if result.verdict == "pass" else (2 if result.verdict == "error" else 1)


def _cmd_doctor(args: argparse.Namespace) -> int:
    report = health_report()
    if args.json:
        print(json.dumps(report, indent=2))
        return 0
    for pid, info in sorted(report.items()):
        tick = "✓" if info["available"] else "✗"
        print(f"  {tick} {pid}: {info['name']} — {info['reason']}")
    return 0


def _cmd_models(args: argparse.Namespace) -> int:
    if args.json:
        entries = {}
        for pid in all_ids():
            provider = get(pid)
            entries[pid] = {"name": provider.name, "description": provider.description, "meta": provider.meta}
        print(json.dumps(entries, indent=2))
        return 0
    for pid in all_ids():
        provider = get(pid)
        print(f"  {pid}: {provider.name} — {provider.description}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    _sys.exit(main())
