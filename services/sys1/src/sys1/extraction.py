"""The extraction pass: entity spans out of a document's text.

sys1 serves the extraction leg from inside the lane, so no caller talks
to a System One vendor directly: Fleet asks sys1, sys1 picks the vendor.
This module is the entity half — the extraction-class GLiNER that is the
`decide` provider's classifier sibling (`gliner2.5-multi-v1`), called
through the chat-completions route the vendor serves it on.

Every span carries the text it was read from, its character offsets, and
the model's own confidence. A span whose offsets do not slice back to its
own text is DROPPED and recorded as a problem: a wrong offset is worse
than no span. A field with no candidate is DECLINED by name — the
absence is a named fact, never an invented value.
"""

from __future__ import annotations

import hashlib
import json
import os
from typing import Any

from .config import api_key_for, load_config
from .providers import http_post_json

#: The hosted default: the extraction-class GLiNER, per the vendor docs
#: (the Decide model is its classification sibling and stays the
#: `decide` provider's business).
DEFAULT_EXTRACT_URL = "https://api.fastino.ai/v1/chat/completions"
DEFAULT_EXTRACT_MODEL = "fastino/gliner2.5-multi-v1"

#: The vendor's own documented input ceiling for the synchronous route;
#: longer inputs are the async endpoint's job, which this pass does not
#: grow until a caller needs it.
MAX_INPUT_CHARS = 50_000

#: The entity threshold: below this a candidate is not a mention.
ENTITY_THRESHOLD = 0.5

#: One HTTP call's timeout — the bound a cold or loaded endpoint meets,
#: not a warm one.
EXTRACT_TIMEOUT = 30.0


def _settings(cfg: dict) -> tuple[str, str, str | None]:
    """The url, model, and key, from the config's own provider table.

    The table's `extract_*` entries are the authority — the extraction
    vendor's url, model and key env (`extract_api_key_env`, defaulting
    to Fastino's) — so a deployment that configures the extraction
    vendor once configures it everywhere, and a caller never resolves a
    key on its own.
    """
    providers = cfg.get("providers", {})
    url = os.environ.get("SYS1_EXTRACT_URL") or providers.get(
        "extract_api_url", DEFAULT_EXTRACT_URL
    )
    model = os.environ.get("SYS1_EXTRACT_MODEL") or providers.get(
        "extract_model", DEFAULT_EXTRACT_MODEL
    )
    key = os.environ.get("SYS1_EXTRACT_API_KEY") or api_key_for("extract", cfg)
    return url, model, key


def _located(entities: Any, text: str) -> tuple[list[dict], list[str]]:
    """The spans the reply carried, and the offsets it got wrong.

    One entry per candidate: `field`, `text`, `start`, `end`, `confidence`.
    A candidate whose `[start, end)` does not slice back to its own text is
    dropped into `problems` — the span is the evidence, and evidence with
    the wrong coordinates is not evidence.
    """
    spans: list[dict] = []
    problems: list[str] = []
    if not isinstance(entities, dict):
        return spans, problems
    for field, listed in entities.items():
        if not isinstance(listed, list):
            continue
        for span in listed:
            if not isinstance(span, dict):
                continue
            value = span.get("text")
            start = span.get("start")
            end = span.get("end")
            confidence = span.get("confidence", 0.0)
            if not isinstance(value, str) or not isinstance(start, int) or not isinstance(end, int):
                problems.append(f"{field}: a span without text or offsets")
                continue
            if start < 0 or end > len(text) or start >= end or text[start:end] != value:
                problems.append(
                    f"{field}: the offsets [{start},{end}) do not slice back to {value!r}"
                )
                continue
            spans.append(
                {
                    "field": field,
                    "text": value,
                    "start": start,
                    "end": end,
                    "confidence": float(confidence or 0.0),
                }
            )
    return spans, problems


def _log_extract(
    cfg: dict,
    *,
    model: str,
    text: str,
    ok: bool,
    code: str | None,
    spans: list[dict],
    declined: list[str],
    problems: list[str],
    telemetry: dict,
) -> None:
    """The lane's own line for the extraction call — counts, never text.

    Same JSONL shape `classify` writes (`<log_dir>/YYYY/MM/DD/events.jsonl`),
    with `op: "extract"` so a proof can point at the vendor call from
    inside the lane rather than from the caller's side. The document and
    the span text are never recorded; the fingerprint is.
    """
    if not cfg.get("classify"):
        return
    from .telemetry import log_record

    log_record(
        {
            "op": "extract",
            "ok": ok,
            "code": code,
            "model": model,
            "input_chars": len(text),
            "input_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest()[:16],
            "fields": sorted({span["field"] for span in spans}) if ok else [],
            "spans": len(spans),
            "declined": declined,
            "problems": len(problems),
            "latency_ms": telemetry.get("latency_ms"),
            "http_status": telemetry.get("http_status"),
            "retries": telemetry.get("retries"),
        },
        cfg=cfg,
    )


def extract(
    text: str, fields: list[dict], cfg: dict | None = None, *, log: bool = True
) -> dict:
    """One entity pass over `text` for `fields` (`[{name, description}]`).

    Answers `{"ok": True, "spans": [...], "declined": [...], "problems": [...]}`
    or `{"ok": False, "code": ..., "reason": ...}`. An empty field is
    declined by name; a failed call is a named failure, never a zero.
    """
    cfg = cfg or load_config()
    if not isinstance(text, str) or not text.strip():
        return {"ok": False, "code": "empty", "reason": "the text is empty"}
    if not fields:
        return {"ok": False, "code": "no_fields", "reason": "no fields were requested"}
    url, model, key = _settings(cfg)
    if not key:
        return {
            "ok": False,
            "code": "no_key",
            "reason": "the extraction vendor's key is not configured (FASTINO_API_KEY)",
        }
    bounded = text[:MAX_INPUT_CHARS]
    body = {
        "model": model,
        "messages": [{"role": "user", "content": bounded}],
        "schema": {
            "entities": [
                {"name": field.get("name", ""), "description": field.get("description", "")}
                for field in fields
            ]
        },
        "threshold": ENTITY_THRESHOLD,
        "include_confidence": True,
        "include_spans": True,
        "store": False,
    }
    telemetry: dict[str, Any] = {}
    log_it = log
    try:
        reply = http_post_json(
            url,
            body,
            api_key=key,
            timeout=EXTRACT_TIMEOUT,
            max_retries=2,
            backoff=1.0,
            telemetry=telemetry,
            label="extract",
        )
    except Exception as exc:  # pragma: no cover — transport boundary
        if log_it:
            _log_extract(
                cfg, model=model, text=bounded, ok=False, code="transport",
                spans=[], declined=[], problems=[], telemetry=telemetry,
            )
        return {"ok": False, "code": "transport", "reason": str(exc)}
    content = (
        ((reply.get("choices") or [{}])[0].get("message") or {}).get("content")
        or ""
    )
    try:
        parsed = json.loads(content)
    except (TypeError, ValueError):
        if log_it:
            _log_extract(
                cfg, model=model, text=bounded, ok=False, code="parse",
                spans=[], declined=[], problems=[], telemetry=telemetry,
            )
        return {
            "ok": False,
            "code": "parse",
            "reason": "the vendor's content was not the entity JSON it promises",
        }
    spans, problems = _located(parsed.get("entities"), bounded)
    found = {span["field"] for span in spans}
    declined = [field.get("name", "") for field in fields if field.get("name", "") not in found]
    if log_it:
        _log_extract(
            cfg, model=model, text=bounded, ok=True, code=None,
            spans=spans, declined=declined, problems=problems, telemetry=telemetry,
        )
    return {
        "ok": True,
        "spans": spans,
        "declined": declined,
        "problems": problems,
        "model": model,
        "telemetry": telemetry,
    }
