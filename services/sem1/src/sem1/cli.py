"""sem1 CLI: embed, probe, doctor. argparse, stdlib only."""

from __future__ import annotations

import argparse
import json
import sys

import sem1
from sem1 import registry, store
from sem1.telemetry import LOG_DIR


def cmd_embed(args: argparse.Namespace) -> int:
    items: list = []
    if args.stdin:
        items = [{"text": line} for line in sys.stdin.read().splitlines() if line.strip()]
    for t in args.text or []:
        items.append({"text": t})
    if args.image:
        items.append({"image": args.image})
    if not items:
        print("error: nothing to embed (pass TEXT, --stdin, or --image)", file=sys.stderr)
        return 3
    try:
        kw = {}
        if args.endpoint:
            kw["endpoint"] = args.endpoint
        r = sem1.embed(items, provider=args.provider, model=args.model or None, **kw)
    except sem1.Sem1Error as e:
        print(f"error: {e}", file=sys.stderr)
        return 3
    if args.json:
        print(json.dumps(r.to_dict(), indent=1))
    else:
        print(f"{r.provider} {r.model}: {r.count} vector(s), {r.dims} dims, "
              f"{r.telemetry.get('latency_ms')} ms")
        for i, v in enumerate(r.vectors):
            print(f"  [{i}] head={[round(x, 5) for x in v[:5]]}")
    return 0


def cmd_probe(args: argparse.Namespace) -> int:
    kw = {}
    if args.model:
        kw["model"] = args.model
    out = sem1.probe(args.provider, endpoint=args.endpoint, battery=args.battery, **kw)
    print(json.dumps(out, indent=1))
    if args.battery and out.get("battery", {}).get("pass") is False:
        return 1
    return 0 if out.get("reachable") else 3


def cmd_doctor(_args: argparse.Namespace) -> int:
    print(f"sem1 {sem1.__version__}")
    print(f"  telemetry : {LOG_DIR}")
    print(f"  vectors   : {sem1.store.DEFAULT_ROOT}")
    from sem1.providers import st_worker
    print(f"  st venv   : {st_worker.venv_python()} "
          f"({'present' if st_worker.venv_python().exists() else 'MISSING — see README'})")
    for name in registry.all_ids():
        caps = registry.capabilities(name)
        caps_s = ", ".join(f"{k}={'yes' if v else 'no'}" for k, v in caps.items())
        print(f"  provider  : {name} — {registry.get(name)['description']} [{caps_s}]")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="sem1", description="semantic embeddings lane (house pattern)")
    sub = ap.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("embed", help="Embed text/image items")
    sp.add_argument("text", nargs="*", help="text items")
    sp.add_argument("--stdin", action="store_true", help="read one text item per stdin line")
    sp.add_argument("--image", help="image file path (st-worker provider)")
    sp.add_argument("--provider", default="llama-server")
    sp.add_argument("--model", default=None)
    sp.add_argument("--endpoint", default=None, help="override endpoint (llama-server)")
    sp.add_argument("--json", action="store_true", help="print the full EmbedResult")
    sp.set_defaults(func=cmd_embed)

    sp = sub.add_parser("probe", help="Probe a provider (reachability, dims, capabilities)")
    sp.add_argument("--provider", default="llama-server")
    sp.add_argument("--endpoint", default=None)
    sp.add_argument("--model", default=None)
    sp.add_argument("--battery", action="store_true",
                    help="run the semantic mirror battery (image-capable providers only)")
    sp.set_defaults(func=cmd_probe)

    sp = sub.add_parser("doctor", help="Show sem1 configuration and providers")
    sp.set_defaults(func=cmd_doctor)

    args = ap.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
