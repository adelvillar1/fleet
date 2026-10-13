#!/usr/bin/env python3
"""
sem1 MCP server: exposes `sem1.embed` as the `sem1_embed` tool and
`sem1.probe` as `sem1_probe`.

Transport: stdio JSON-RPC (MCP 2024-11-05).

Default provider is `llama-server` (local HTTP endpoint, text-only, default
http://127.0.0.1:8901/v1/embeddings). Images require the `st-worker`
provider (sentence-transformers in a subprocess venv). House rule: vectors
in, vectors out — embeddings propose, the decision layers dispose.

Dependencies: sem1 package on PYTHONPATH (use `pip install -e` or set
SEM1_PYTHONPATH to the `src` directory).
"""

from __future__ import annotations

import json
import os
import re
import sys


def _add_sem1_src_to_path() -> None:
    """Ensure `import sem1` resolves from the repo src layout."""
    env_path = os.environ.get("SEM1_PYTHONPATH")
    if env_path:
        sys.path.insert(0, env_path)


def _store_root(store: str):
    """One VectorStore root per named corpus, namespaced under the default."""
    from sem1.store import DEFAULT_ROOT, VectorStore

    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", store).strip("-")
    if not slug:
        raise ValueError("`store` must be a non-empty name.")
    return VectorStore(root=DEFAULT_ROOT / slug)


def _normalize_items_argument(value):
    """Accept a bare string or a list of strings / {text}/{image} objects."""
    if isinstance(value, str):
        return [value]
    if isinstance(value, list) and value and all(
        isinstance(item, (str, dict)) for item in value
    ):
        return value
    return None


def main() -> None:
    _add_sem1_src_to_path()
    from sem1 import embed, probe  # noqa: E402  (import after path patch)

    def respond(req_id, result):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "result": result}) + "\n")
        sys.stdout.flush()

    def error(req_id, code, msg):
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req_id, "error": {"code": code, "message": msg}}) + "\n")
        sys.stdout.flush()

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

        if req_id is None:
            # JSON-RPC notification: never answered. Replying emits a frame
            # with no pending requester (id null), which strict clients read
            # as protocol garbage on the shared stdin stream.
            continue
        if method == "ping":
            # MCP requires servers to answer pings with an empty result.
            respond(req_id, {})
            continue

        if method == "initialize":
            respond(req_id, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "sem1", "version": "0.1.0"},
            })
        elif method == "tools/list":
            respond(req_id, {
                "tools": [
                    {
                        "name": "sem1_embed",
                        "description": (
                            "Embed text (or images via the st-worker provider) "
                            "into vectors. Returns the vectors, dims, model, "
                            "provider, and telemetry. Raises on unsupported "
                            "modality instead of answering with a wrong vector."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "items": {
                                    "type": ["string", "array"],
                                    "items": {
                                        "type": ["string", "object"],
                                        "properties": {
                                            "text": {"type": "string"},
                                            "image": {
                                                "type": "string",
                                                "description": "Path to an image file (st-worker provider only).",
                                            },
                                        },
                                    },
                                    "description": (
                                        "A single string or a list of strings / "
                                        "{\"text\": …} / {\"image\": path} items."
                                    ),
                                },
                                "provider": {
                                    "type": "string",
                                    "description": "llama-server (text, default) or st-worker (text + images).",
                                    "default": "llama-server",
                                },
                                "model": {
                                    "type": "string",
                                    "description": "Optional model override.",
                                },
                                "endpoint": {
                                    "type": "string",
                                    "description": "llama-server base URL override (default http://127.0.0.1:8901).",
                                },
                            },
                            "required": ["items"],
                        },
                    },
                    {
                        "name": "sem1_probe",
                        "description": (
                            "Capability and reachability probe for a provider: "
                            "declared modalities, whether the endpoint answers, "
                            "dims, model, latency."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "provider": {
                                    "type": "string",
                                    "description": "Default: llama-server.",
                                    "default": "llama-server",
                                },
                                "endpoint": {
                                    "type": "string",
                                    "description": "llama-server base URL override.",
                                },
                            },
                        },
                    },
                    {
                        "name": "sem1_index",
                        "description": (
                            "(Re)build a named vector store from texts: embeds "
                            "the batch and rebuilds the store wholesale "
                            "(idempotent by key dedup). Manifest entries carry "
                            "the optional parallel `metas` objects (e.g. text "
                            "snippets, source ids) so search can return them."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "store": {
                                    "type": "string",
                                    "description": "Corpus name, e.g. fleet-sessions.",
                                },
                                "texts": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "The texts to embed and index.",
                                },
                                "metas": {
                                    "type": "array",
                                    "description": "Optional objects parallel to texts; stored in manifest entries.",
                                },
                                "keys": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "Optional stable keys parallel to texts (default item-<i>).",
                                },
                                "provider": {"type": "string", "description": "Default: llama-server."},
                                "model": {"type": "string", "description": "Optional model override."},
                            },
                            "required": ["store", "texts"],
                        },
                    },
                    {
                        "name": "sem1_search",
                        "description": (
                            "Top-k semantic search over a named store: embeds "
                            "the query and returns scored manifest entries "
                            "(key, score, and whatever meta was indexed). Vectors "
                            "never leave the store."
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "store": {
                                    "type": "string",
                                    "description": "Corpus name passed to sem1_index.",
                                },
                                "query": {
                                    "type": "string",
                                    "description": "The query text.",
                                },
                                "k": {
                                    "type": "integer",
                                    "description": "Neighbor count. Default: 5.",
                                    "default": 5,
                                },
                                "provider": {"type": "string", "description": "Must match the indexing provider's model. Default: llama-server."},
                                "model": {"type": "string", "description": "Must match the indexing model."},
                            },
                            "required": ["store", "query"],
                        },
                    },
                ],
            })
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {}) or {}

            provider = arguments.get("provider") or "llama-server"
            model = arguments.get("model")
            endpoint = arguments.get("endpoint")

            if tool_name == "sem1_embed":
                items = _normalize_items_argument(arguments.get("items"))
                if items is None:
                    error(req_id, -32602, "`items` must be a string or a non-empty list of strings / {text|image} objects.")
                    continue
                kwargs = {}
                if model is not None:
                    kwargs["model"] = str(model)
                if endpoint is not None:
                    kwargs["endpoint"] = str(endpoint)
                try:
                    result = embed(items, provider=str(provider), **kwargs)
                except TypeError as exc:
                    error(req_id, -32602, f"Provider {provider} does not accept these arguments: {exc}")
                    continue
                except Exception as exc:  # transport boundary: capability refusals, endpoint down
                    error(req_id, -32603, str(exc))
                    continue
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(result.to_dict())}],
                })
                continue

            if tool_name == "sem1_probe":
                kwargs = {"provider": str(provider)}
                if endpoint is not None:
                    kwargs["endpoint"] = str(endpoint)
                try:
                    report = probe(**kwargs)
                except Exception as exc:  # pragma: no cover — transport boundary
                    error(req_id, -32603, str(exc))
                    continue
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps(report)}],
                })
                continue

            if tool_name == "sem1_index":
                from sem1.store import VectorStore  # noqa: E402
                from sem1.telemetry import sha256  # noqa: E402

                texts = arguments.get("texts")
                if not isinstance(texts, list) or not texts or not all(isinstance(t, str) and t.strip() for t in texts):
                    error(req_id, -32602, "`texts` must be a non-empty list of non-empty strings.")
                    continue
                metas = arguments.get("metas")
                if metas is not None and (not isinstance(metas, list) or len(metas) != len(texts)):
                    error(req_id, -32602, "`metas` must be a list parallel to `texts`.")
                    continue
                keys = arguments.get("keys")
                if keys is not None and (not isinstance(keys, list) or len(keys) != len(texts)):
                    error(req_id, -32602, "`keys` must be a list parallel to `texts`.")
                    continue
                try:
                    store = _store_root(str(arguments["store"]))
                    result = embed(texts, provider=str(provider), **({"model": str(model)} if model is not None else {}))
                    manifest = store.rebuild(
                        result.model,
                        result.dims,
                        result.provider,
                        [(str(keys[i]) if keys else f"item-{i}", sha256(texts[i]), vec)
                         for i, vec in enumerate(result.vectors)],
                        meta=[dict(m) if isinstance(m, dict) else {} for m in (metas or [None] * len(texts))] or None,
                    )
                except ValueError as exc:
                    error(req_id, -32602, str(exc))
                    continue
                except Exception as exc:  # pragma: no cover — transport boundary
                    error(req_id, -32603, str(exc))
                    continue
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps({
                        "store": str(arguments["store"]),
                        "count": manifest["count"],
                        "dims": manifest["dims"],
                        "model": manifest["model"],
                        "provider": manifest["provider"],
                    })}],
                })
                continue

            if tool_name == "sem1_search":
                from sem1.similarity import top_k  # noqa: E402

                query = arguments.get("query")
                if not isinstance(query, str) or not query.strip():
                    error(req_id, -32602, "`query` must be a non-empty string.")
                    continue
                try:
                    store = _store_root(str(arguments["store"]))
                    q = embed([query], provider=str(provider), **({"model": str(model)} if model is not None else {}))
                    manifest, vectors = store.load(q.model)
                    if manifest is None:
                        respond(req_id, {
                            "content": [{"type": "text", "text": json.dumps({
                                "store": str(arguments["store"]),
                                "matches": [],
                                "count": 0,
                                "note": "store is empty — run sem1_index first",
                            })}],
                        })
                        continue
                    hits = top_k(q.vectors[0], vectors, k=max(1, int(arguments.get("k", 5))))
                    entries = manifest["entries"]
                    matches = [
                        {"key": entries[i].get("key"), "score": round(score, 4), **{
                            kk: vv for kk, vv in entries[i].items() if kk not in ("key", "sha256")
                        }}
                        for i, score in hits
                    ]
                except ValueError as exc:
                    error(req_id, -32602, str(exc))
                    continue
                except Exception as exc:  # pragma: no cover — transport boundary
                    error(req_id, -32603, str(exc))
                    continue
                respond(req_id, {
                    "content": [{"type": "text", "text": json.dumps({
                        "store": str(arguments["store"]),
                        "count": manifest["count"],
                        "matches": matches,
                    })}],
                })
                continue

            error(req_id, -32601, f"Unknown tool: {tool_name}")
        else:
            error(req_id, -32601, f"Method not found: {method}")


if __name__ == "__main__":
    main()
