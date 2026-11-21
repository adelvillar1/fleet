#!/usr/bin/env python3
"""
sys1 MCP server: exposes `sys1.classify` as the `sys1_classify` tool.

Transport: stdio JSON-RPC (MCP 2024-11-05).

Dependencies: sys1 package on PYTHONPATH (use `pip install -e` or set
SYS1_PYTHONPATH to the `src` directory).
"""

from __future__ import annotations

import json
import os
import sys
import time


def _add_sys1_src_to_path() -> None:
    """Ensure `import sys1` resolves from the repo src layout."""
    env_path = os.environ.get("SYS1_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


def task_for_arguments(arguments: dict, task_from_spec):
    """The task a `sys1_classify` call asks for: an inline spec, or a name.

    An inline `task_spec` (id + heads) is the HTTP face's own seam and IS
    the task — a caller's dynamic heads ride the same routing and cascade
    a registered name gets. `task` is ignored then. A dict without an id
    is not a spec (the caller's shape is wrong, not the task), so the
    registered name stands. Pulled out of `main()` so the seam is
    testable without a stdio session.
    """
    spec = arguments.get("task_spec")
    if isinstance(spec, dict) and spec.get("id"):
        return task_from_spec(spec)
    return arguments.get("task", "change")


# The probe's state: a short route-evidence record in the same shape the
# judge's ask carries, so the round-trip exercises the real question rather
# than a toy one. Its LABEL is never asserted — an engine may answer any
# workload or abstain, and both are legal (KI-030: the handshake is not
# provider health, and a probe must not pin the answer it hopes for).
PROBE_STATE = (
    "the workload shape the router judged: quick\n"
    "how: judged\n"
    "sys1 verdict: pass\n"
    "reason: a single-file edit, small diff\n"
    "model: glide\n"
    "provider that served: glide\n"
    "attempts, in order:\n- glide served ok\n"
    "input chars: 412\n"
    "input hash: 8f2c1a9b4d5e6f70\n"
    "the run's outcome: ok (completed)\n"
)


def probe_providers(cfg: dict) -> dict:
    """
    The per-provider readiness report: `{provider_id: {name, available,
    reason, …}}`, the shape Fleet's `parse_health_map` already renders
    beside sdm1's and sem1's rows.

    Laya is the one provider with a real round-trip here, because it is the
    one whose availability is a model in a venv rather than a key in the
    environment: `available` means a typed answer came back from
    `laya.Agent().predict(...)` — venv ok, model loaded, the parse survived
    — and `reason` carries the measured facts (the answer or the abstain,
    the latency, the child's peak RSS). An abstain is a PASS: the engine
    saying it should not decide is the semantics this provider was chosen
    for, not a failure of the probe.
    """
    from sys1 import providers
    from sys1.tasks import get_task

    provider = providers.REGISTRY.get("laya")
    if provider is None:
        return {}
    entry: dict = {"name": provider.name, "available": False, "reason": "not registered"}
    venv = providers._laya_venv(cfg)
    entry["venv"] = str(venv)
    if not providers._laya_installed(cfg):
        entry["reason"] = (
            f"venv not installed in {venv} — `fleet install --provider laya` builds it"
        )
        return {"laya": entry}

    task = get_task("route")
    head = next((h for h in task.heads if h.id == "workload"), None)
    if head is None:  # pragma: no cover — the task registry owns this head
        entry["reason"] = "the route task carries no workload head"
        return {"laya": entry}

    telemetry: dict = {}
    started = time.monotonic()
    try:
        answers = provider.classify(PROBE_STATE, [head], cfg, telemetry)
    except Exception as exc:
        entry["reason"] = f"venv ok, the round-trip failed: {str(exc)[:200]}"
        entry["latency_ms"] = int((time.monotonic() - started) * 1000)
        return {"laya": entry}

    elapsed_ms = int((time.monotonic() - started) * 1000)
    answer = answers.get("workload") or {}
    label = answer.get("label")
    confidence = answer.get("confidence")
    declined = answer.get("_declined")

    if label is None:
        # An abstain (or a decline) is legal and is a PASS: the engine
        # answered with a refusal rather than a confident wrong label.
        entry["available"] = bool(declined) and telemetry.get("structured_ok", False)
        entry["answer"] = "abstained"
        said = answer.get("route_reason") or declined or "no label"
        entry["reason"] = f"venv ok · model loaded · abstained ({said}) · {elapsed_ms} ms"
    elif label not in head.labels:
        entry["available"] = False
        entry["answer"] = label
        entry["reason"] = f"answered `{label}`, which is not in the head's declared set"
    else:
        entry["available"] = True
        entry["answer"] = label
        entry["confidence"] = confidence
        prediction_set = answer.get("prediction_set")
        extra = f" · set {len(prediction_set)}" if isinstance(prediction_set, list) else ""
        entry["reason"] = (
            f"venv ok · model loaded · answered {label} @ "
            f"{confidence if confidence is not None else 'no confidence'}"
            f"{extra} · {elapsed_ms} ms"
        )
    # Two memory facts, kept apart because they answer different questions:
    # the child's load transient (a high-water mark, over the 2 GiB ceiling
    # and paid once per lane lifetime) and its resident size now (what a
    # running lane actually holds). Reporting one as the other is how a
    # footprint claim goes wrong. Both come from the child itself: a parent's
    # RUSAGE_CHILDREN only counts children it has reaped, and a resident
    # child is never reaped while it matters.
    peak = telemetry.get("peak_rss_mib")
    steady = None
    try:
        steady = providers._laya_child(cfg).rss_mib()
    except Exception:
        steady = None
    if steady is not None:
        entry["rss_mib"] = steady
    if peak is not None:
        entry["peak_rss_mib"] = peak
    memory = []
    if steady is not None:
        memory.append(f"RSS {steady} MiB")
    if peak is not None:
        memory.append(f"load peak {peak} MiB")
    if memory:
        entry["reason"] += " · " + " · ".join(memory)
    entry["latency_ms"] = elapsed_ms
    entry["mode"] = telemetry.get("mode")
    return {"laya": entry}


def main() -> None:
    _add_sys1_src_to_path()
    from sys1.api import classify
    from sys1.config import load_config
    from sys1.extraction import extract
    from sys1.tasks import task_from_spec

    try:
        # Fleet's carry: the documents session's Perception tool rides the
        # sys1 lane, and sys1.perception is a carried module (see
        # VENDOR.json's fleetVendor note) — the adapter degrades to
        # classify-only when the module is absent upstream.
        from sys1.perception import parse_document  # noqa: E402
    except ImportError:
        parse_document = None

    cfg = load_config()

    def respond(req_id, result):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}) + "\n")
        sys.stdout.flush()

    def error(req_id, code, msg):
        respond(req_id, {"error": {"code": code, "message": msg}})

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue

        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params", {}) or {}

        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "sys1", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "sys1_classify",
                        "description": (
                            "Run sys1 decision-model classification on text. "
                            "Returns verdict, providers used, summary, and latency."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "The payload to classify (diff, PR body, command string, …).",
                                },
                                "task": {
                                    "type": "string",
                                    "description": "Registered task name or 'change' default.",
                                    "default": "change",
                                },
                                "provider": {
                                    "type": "string",
                                    "description": (
                                        "Provider id (decide, glide, jev, …), alias ('both'/'all'), or comma list. "
                                        "Omit or leave empty to route per policy (providers.route); an explicit value bypasses routing."
                                    ),
                                },
                                "task_spec": {
                                    "type": "object",
                                    "description": (
                                        "An inline task (id + heads), the HTTP face's own seam: when present it IS "
                                        "the task, so a caller's per-call heads ride the same routing and cascade as "
                                        "a registered task. `task` is ignored then."
                                    ),
                                },
                                "images": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": (
                                        "Optional image payloads (data URLs) for vision-capable providers "
                                        "(liquid); text-only wires ignore them. The vision_qc task's own seam."
                                    ),
                                },
                            },
                            "required": ["text"],
                        },
                    },
                    {
                        "name": "sys1_extract",
                        "description": (
                            "Read entity spans out of a text with the extraction-class GLiNER (the "
                            "Decide classifier's sibling), through the vendor sys1 already holds. "
                            "Every span carries its offsets and confidence; a span whose offsets do "
                            "not slice back is dropped and named, and a field with no candidate is "
                            "declined by name."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "The document text to read.",
                                },
                                "fields": {
                                    "type": "array",
                                    "description": "The entity fields to read for.",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"},
                                            "description": {"type": "string"},
                                        },
                                        "required": ["name"],
                                    },
                                },
                            },
                            "required": ["text", "fields"],
                        },
                    },
                    *(
                        []
                        if parse_document is None
                        else [
                            {
                                "name": "sys1_perceive_document",
                                "description": (
                                    "Parse one local document through Nace Perception, using the "
                                    "provider credential sys1 already holds. Returns markdown, pages, "
                                    "units, job id, and any content_truncated disclosure. A scan or a "
                                    "format local extraction cannot read is this tool's case."
                                ),
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "path": {
                                            "type": "string",
                                            "description": "Absolute path to the document on this machine.",
                                        },
                                    },
                                    "required": ["path"],
                                },
                            }
                        ]
                    ),
                    {
                        "name": "sys1_probe",
                        "description": (
                            "The lane's PROVIDER-level readiness report: for each provider whose "
                            "availability is a model rather than a key, one real typed round-trip "
                            "and the facts it measured (venv, model loaded, the answer or the "
                            "abstain, latency, the child's peak RSS). The MCP handshake says the "
                            "lane answers, not that its providers do — this is the difference."
                        ),
                        "inputSchema": {"type": "object", "properties": {}},
                    },
                ],
            })
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {}) or {}

            if tool_name == "sys1_perceive_document":
                if parse_document is None:
                    error(req_id, -32601, "Unknown tool: sys1_perceive_document")
                    continue
                path = arguments.get("path")
                if not isinstance(path, str) or not path.strip():
                    error(req_id, -32602, "`path` must be a non-empty string.")
                    continue
                try:
                    result = parse_document(path, cfg)
                except Exception as exc:  # pragma: no cover — transport boundary
                    result = {"ok": False, "code": "internal", "reason": str(exc)}
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(result)}],
                })
                continue

            if tool_name == "sys1_probe":
                # KI-030: the handshake proves the lane answers; this is
                # the provider-level truth beside it. Never raises: a
                # provider that cannot answer is an entry with a reason,
                # which is what the card renders.
                try:
                    report = probe_providers(cfg)
                except Exception as exc:  # pragma: no cover — transport boundary
                    report = {"laya": {"name": "Laya", "available": False, "reason": str(exc)}}
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(report)}],
                })
                continue

            if tool_name == "sys1_extract":
                # The entity pass through the lane: the vendor call is
                # sys1's, the offsets and the declines are named.
                text = arguments.get("text")
                fields = arguments.get("fields")
                if not isinstance(text, str) or not text.strip():
                    error(req_id, -32602, "`text` must be a non-empty string.")
                    continue
                if not isinstance(fields, list) or not fields:
                    error(req_id, -32602, "`fields` must be a non-empty array.")
                    continue
                try:
                    result = extract(text, fields, cfg)
                except Exception as exc:  # pragma: no cover — transport boundary
                    result = {"ok": False, "code": "internal", "reason": str(exc)}
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(result)}],
                })
                continue

            if tool_name != "sys1_classify":
                error(req_id, -32601, f"Unknown tool: {tool_name}")
                continue

            text = arguments.get("text", "")
            # An inline task spec — the HTTP face's own seam — IS the
            # task: a caller's dynamic heads ride the same routing and
            # cascade a registered name gets, with no vendored task
            # added per shape.
            try:
                task = task_for_arguments(arguments, task_from_spec)
            except Exception as exc:  # pragma: no cover — caller's shape
                error(req_id, -32602, f"`task_spec` was refused: {exc}")
                continue
            # An absent or empty `provider` passes through unset: classify then
            # routes per policy (providers.route), so the MCP surface never
            # coerces a caller into pinning one vendor. An explicit id, alias,
            # or comma list still bypasses routing entirely.
            provider = arguments.get("provider") or None
            if not isinstance(text, str) or not text.strip():
                error(req_id, -32602, "`text` must be a non-empty string.")
                continue

            images = arguments.get("images")
            if images is not None and (
                not isinstance(images, list)
                or not all(isinstance(item, str) and item for item in images)
            ):
                error(req_id, -32602, "`images` must be a list of non-empty strings (data URLs).")
                continue

            try:
                result = classify(
                    provider=provider,
                    task=task,
                    text=text,
                    cfg=cfg,
                    log=False,
                    images=images,
                )
            except Exception as exc:  # pragma: no cover — transport boundary
                error(req_id, -32603, str(exc))
                continue

            respond(req_id, {
                "content": [
                    {
                        "type": "text",
                        "text": json.dumps(
                            {
                                "task": result.task,
                                "verdict": result.verdict,
                                "escalated": result.escalated,
                                "provider": result.provider,
                                "providers_used": result.providers_used,
                                "answers": result.answers,
                                "summary": result.summary,
                                "latency_ms": result.latency_ms,
                                "reason": result.reason,
                            }
                        ),
                    }
                ],
            })
        elif method == "initialized":
            # Client notification; no response required.
            pass
        else:
            error(req_id, -32601, f"Method not found: {method}")


if __name__ == "__main__":
    main()
