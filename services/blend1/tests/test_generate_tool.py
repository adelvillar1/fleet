"""Tests for blend1's `generate` MCP tool — the plain text leg.

The grounded-selection plan's Phase 4 seam (C10/C11): one chat completion
on Liquid's hosted text models, no router, no facts template, no
validation gate. Same conventions as test_mcp_server.py — the REAL
adapter over stdio, a fake lane for the pass-through proof, a scratch
HOME so no developer key leaks into a test.

Run: python3 -m pytest services/blend1/tests/test_generate_tool.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

SERVICE_DIR = Path(__file__).resolve().parent.parent / "service"
SRC_DIR = Path(__file__).resolve().parent.parent / "src"
SERVER = SERVICE_DIR / "mcp_server.py"

FAKE_GENERATE_RESULT = {
    "status": "ok",
    "model": "lfm3-1.7b",
    "text": "The Streaming Parser Fix",
    "usage": {"prompt_tokens": 30, "completion_tokens": 8},
}

# Boots the real adapter with blend1.lanes.generate_text replaced by a
# fake that echoes the kwargs the handler passed — proving the handler
# forwards prompt/system/model/max_tokens and rides the leg's own return
# value back untouched.
FAKE_GENERATE_WRAPPER = """
import sys
sys.path.insert(0, {src!r})
sys.path.insert(0, {service!r})
import blend1.lanes as lanes

_calls = []

def _fake_generate_text(prompt, *, system=None, model="lfm3-1.7b", max_tokens=None):
    _calls.append({{"prompt": prompt, "system": system, "model": model, "max_tokens": max_tokens}})
    return dict({fake!r}, seen=_calls[-1])

lanes.generate_text = _fake_generate_text
import mcp_server
mcp_server.main()
"""


def _child_env(home: Path) -> dict:
    env = dict(os.environ)
    env.pop("LIQUID_API_KEY", None)
    env["HOME"] = str(home)
    return env


def _drive(frames, tmp_path, wrapper=None):
    if wrapper is None:
        argv = [sys.executable, "-u", str(SERVER)]
    else:
        path = tmp_path / "wrapped_server.py"
        path.write_text(wrapper, encoding="utf-8")
        argv = [sys.executable, "-u", str(path)]
    home = tmp_path / "home"
    home.mkdir(exist_ok=True)
    stdin = "".join(json.dumps(f) + "\n" for f in frames)
    proc = subprocess.run(
        argv,
        input=stdin,
        capture_output=True,
        text=True,
        env=_child_env(home),
        cwd=str(SERVICE_DIR),
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    return [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]


def _call(name, arguments, req_id=2):
    return {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments},
    }


def _result_payload(reply):
    assert "result" in reply, reply
    content = reply["result"]["content"]
    assert content and content[0]["type"] == "text"
    return json.loads(content[0]["text"])


def _error_message(reply):
    assert "error" in reply, reply
    return reply["error"]["message"]


def test_tools_list_declares_generate_with_the_probed_models(tmp_path):
    replies = _drive([{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}], tmp_path)
    tools = replies[0]["result"]["tools"]
    names = [t["name"] for t in tools]
    assert "generate" in names
    schema = next(t for t in tools if t["name"] == "generate")["inputSchema"]
    assert schema["required"] == ["prompt"]
    assert set(schema["properties"]) == {"prompt", "system", "model", "max_tokens"}
    assert schema["properties"]["model"]["enum"] == ["lfm2.5-24b-a2b", "lfm3-1.7b"]
    assert schema["properties"]["model"]["default"] == "lfm3-1.7b"


def test_generate_call_rides_the_leg_result_verbatim(tmp_path):
    replies = _drive(
        [
            _call(
                "generate",
                {
                    "prompt": "Name this session: the parser fix.",
                    "system": "Answer with the name only.",
                    "model": "lfm3-1.7b",
                    "max_tokens": 1024,
                },
            )
        ],
        tmp_path,
        wrapper=FAKE_GENERATE_WRAPPER.format(
            src=str(SRC_DIR), service=str(SERVICE_DIR), fake=FAKE_GENERATE_RESULT
        ),
    )
    payload = _result_payload(replies[0])
    assert payload["status"] == "ok"
    assert payload["text"] == FAKE_GENERATE_RESULT["text"]
    # The handler's forwarding, echoed by the fake — every kwarg the
    # caller sent reaches the leg, and nothing else is invented.
    assert payload["seen"] == {
        "prompt": "Name this session: the parser fix.",
        "system": "Answer with the name only.",
        "model": "lfm3-1.7b",
        "max_tokens": 1024,
    }


def test_generate_defaults_model_and_leaves_system_none(tmp_path):
    replies = _drive(
        [_call("generate", {"prompt": "one line summary"})],
        tmp_path,
        wrapper=FAKE_GENERATE_WRAPPER.format(
            src=str(SRC_DIR), service=str(SERVICE_DIR), fake=FAKE_GENERATE_RESULT
        ),
    )
    payload = _result_payload(replies[0])
    assert payload["seen"]["model"] == "lfm3-1.7b"
    assert payload["seen"]["system"] is None
    assert payload["seen"]["max_tokens"] is None


def test_generate_rejects_malformed_arguments(tmp_path):
    replies = _drive(
        [
            _call("generate", {}, req_id=2),
            _call("generate", {"prompt": "x", "model": "lfm3-vl-2b"}, req_id=3),
            _call("generate", {"prompt": "x", "max_tokens": 0}, req_id=4),
            _call("generate", {"prompt": "x", "max_tokens": 99999}, req_id=5),
            _call("generate", {"prompt": "x", "system": ""}, req_id=6),
            _call("generate", {"prompt": "x", "max_tokens": True}, req_id=7),
        ],
        tmp_path,
    )
    assert len(replies) == 6
    for reply in replies:
        assert "error" in reply, reply
        assert reply["error"]["code"] == -32602


def test_generate_unkeyed_returns_the_legs_named_error(tmp_path):
    """No LIQUID_API_KEY in the child env: the leg's own named failure is
    the result — never a traceback, never an empty answer."""
    replies = _drive([_call("generate", {"prompt": "summarize"})], tmp_path)
    payload = _result_payload(replies[0])
    assert payload["status"] == "error"
    assert "LIQUID_API_KEY" in payload["error"]


def test_generate_text_allowlist_and_floor_unit():
    """The leg's own contract, without a subprocess: unprobed models are
    refused before the wire; the reasoning floor clamps the budget."""
    sys.path.insert(0, str(SRC_DIR))
    from blend1 import lanes

    # The allowlist fires before any network access.
    refused = lanes.generate_text("x", model="lfm3-omni-2b")
    assert refused["status"] == "error"
    assert "not a probed text leg" in refused["error"]

    # The floor: patch the wire, capture the budget.
    seen = {}

    def fake_wire(base, messages, model, api_key, timeout, max_tokens):
        seen["max_tokens"] = max_tokens
        seen["messages"] = messages
        return {
            "choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}],
            "model": model,
            "usage": {},
        }

    original = lanes._openai_chat
    lanes._openai_chat = fake_wire
    try:
        result = lanes.generate_text("title this", model="lfm3-1.7b", max_tokens=64)
    finally:
        lanes._openai_chat = original
    assert result["status"] == "ok"
    assert seen["max_tokens"] == lanes.GENERATE_TEXT_FLOOR_TOKENS
    assert seen["messages"] == [{"role": "user", "content": "title this"}]

    # A system prompt rides as the system message, before the user's.
    lanes._openai_chat = fake_wire
    try:
        lanes.generate_text("x", system="be brief", model="lfm3-1.7b")
    finally:
        lanes._openai_chat = original
    assert seen["messages"][0] == {"role": "system", "content": "be brief"}
