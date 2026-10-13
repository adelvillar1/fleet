"""sdm1 worker — runs INSIDE the [models] venv (Python >=3.11, CUDA).

Reads one JSON request from stdin:
  {"model": "nvidia/Kumo-Tabular", "target": ..., "task_type": "classification"|"regression",
   "stype_overrides": {...}, "num_estimators": 8, "rows_context": [...], "rows_query": [...]}

Writes one JSON envelope to stdout:
  {"ok": true, "model_name": ..., "predictions": [{row_index, label, probabilities, confidence}], "warnings": [...]}

The core (stdlib-only) spawns this script; it may import torch/sdm/pandas
freely. Everything the model stack prints to stdout is diverted to stderr so
stdout carries exactly one JSON document. Errors come back as
{"ok": false, "error": ...} — never a traceback on stdout, never dataset
contents in either stream.
"""

from __future__ import annotations

import contextlib
import io
import json
import sys


def main() -> int:
    real_stdout = sys.stdout
    diverted = io.StringIO()
    request = json.loads(sys.stdin.read())
    warnings: list[str] = []
    try:
        with contextlib.redirect_stdout(diverted):  # keep model-stack chatter off the wire
            payload = _run(request, warnings)
        payload["warnings"] = warnings
        payload["ok"] = True
    except Exception as e:
        payload = {"ok": False, "error": f"{type(e).__name__}: {e}"[:500], "warnings": warnings}
    print(json.dumps(payload, default=str), file=real_stdout)
    return 0


def _run(request: dict, warnings: list[str]) -> dict:
    import pandas as pd
    import sdm
    import torch

    target = request["target"]
    task_type = request["task_type"]
    num_estimators = int(request.get("num_estimators") or 8)
    overrides = dict(request.get("stype_overrides") or {})
    rows_context = request["rows_context"]
    rows_query = request["rows_query"]

    if not torch.cuda.is_available():
        raise RuntimeError("CUDA is not available — sdm backends require a CUDA GPU")
    device = "cuda"

    df_context = pd.DataFrame(rows_context)
    df_query = pd.DataFrame(rows_query)
    overrides[target] = "categorical" if task_type == "classification" else "numerical"
    stypes = sdm.infer_stypes(df_context, overrides=overrides)

    table_ctx = sdm.TableTensor.from_pandas(df=df_context, stypes=stypes, device=device)
    x_context = table_ctx.drop_columns(target)
    y_context = table_ctx[:, target]

    stypes_query = {k: v for k, v in stypes.items() if k != target}
    table_query = sdm.TableTensor.from_pandas(df=df_query, stypes=stypes_query, device=device)

    model = sdm.models.KumoTabular(task=task_type, device=device)

    if task_type == "classification":
        probs = model(
            x_context=x_context,
            y_context=y_context,
            x_query=table_query,
            num_estimators=num_estimators,
        )
        classes = _class_names(stypes, target, y_context)
        rows = _matrix_rows(probs)
        n_classes = _n_classes(rows)
        if classes is None or len(classes) != n_classes:
            classes = [f"class_{i}" for i in range(n_classes)]
            warnings.append(
                f"class names unavailable from stypes; synthesized {n_classes} placeholder names"
            )
        predictions = []
        for i, row in enumerate(rows):
            prob_by_class = {str(c): float(p) for c, p in zip(classes, row, strict=False)}
            label = max(prob_by_class, key=prob_by_class.__getitem__)
            predictions.append(
                {
                    "row_index": i,
                    "label": label,
                    "probabilities": prob_by_class,
                    "confidence": float(prob_by_class[label]),
                }
            )
    else:  # regression
        values = model(
            x_context=x_context,
            y_context=y_context,
            x_query=table_query,
            num_estimators=num_estimators,
        )
        flat = _matrix_rows(values)
        predictions = []
        for i, row in enumerate(flat):
            value = row[0] if isinstance(row, list) else row
            predictions.append({"row_index": i, "label": float(value), "probabilities": None, "confidence": None})

    return {
        "model_name": str(request.get("model") or "nvidia/Kumo-Tabular"),
        "device": device,
        "task_type": task_type,
        "predictions": predictions,
    }


def _class_names(stypes: dict, target: str, y_context) -> list[str] | None:
    """Best-effort class vocabulary from the categorical stype or context tensor.

    The exact home of the vocabulary varies across sdm versions; try the known
    paths and let the caller synthesize placeholder names if none resolves.
    """
    candidates = []
    st = stypes.get(target)
    if st is not None:
        candidates += [getattr(st, "categories", None), getattr(st, "classes", None), getattr(st, "classes_", None)]
    for attr in ("classes", "categories", "classes_", "vocab"):
        candidates.append(getattr(y_context, attr, None))
        candidates.append(getattr(getattr(y_context, "stype", None), attr, None))
    for cand in candidates:
        try:
            names = [str(c) for c in cand]
        except TypeError:
            continue
        if names:
            return names
    return None


def _matrix_rows(tensor) -> list:
    """Coerce a model output tensor to nested python lists (or 1-D list)."""
    if isinstance(tensor, (list, tuple)):
        return list(tensor)
    if hasattr(tensor, "detach"):
        tensor = tensor.detach()
    if hasattr(tensor, "cpu"):
        tensor = tensor.cpu()
    return tensor.tolist()


def _n_classes(rows: list) -> int:
    first = rows[0] if rows else None
    return len(first) if isinstance(first, list) else 2  # 1-D output treated as binary P(positive)


if __name__ == "__main__":
    sys.exit(main())
