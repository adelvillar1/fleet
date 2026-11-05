"""The extraction verb and the classify seam (plan 2026-11-05, C1/C2).

Run from `services/sys1`:  python3 -m unittest discover -s tests -t .

The lane's own tests are stdlib-only: no vendor is called (the HTTP
boundary is patched), so the parse, the declines, the offset rule and
the `task_spec` seam are pinned without a network or a pytest install.
"""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import unittest
from contextlib import redirect_stdout
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "src"))
sys.path.insert(0, os.path.join(ROOT, "service"))

import sys1.extraction as extraction  # noqa: E402
from sys1.tasks import task_from_spec  # noqa: E402

import mcp_server  # noqa: E402

TEXT = "Invoice INV-1 is due on 2026-11-30 for $4,180.00."


def _reply(content) -> dict:
    return {"choices": [{"message": {"content": json.dumps(content)}}]}


def _span(text, start, end, confidence=0.9):
    return {"text": text, "start": start, "end": end, "confidence": confidence}


class TheLocatedSpans(unittest.TestCase):
    def test_a_span_whose_offsets_slice_back_is_kept(self):
        at = TEXT.find("INV-1")
        spans, problems = extraction._located(
            {"invoice_id": [_span("INV-1", at, at + 5, 0.93)]}, TEXT
        )
        self.assertEqual(problems, [])
        self.assertEqual(len(spans), 1)
        self.assertEqual(spans[0]["field"], "invoice_id")
        self.assertEqual(spans[0]["text"], "INV-1")
        self.assertEqual(spans[0]["confidence"], 0.93)
        self.assertEqual(TEXT[spans[0]["start"] : spans[0]["end"]], "INV-1")

    def test_a_span_with_wrong_offsets_is_dropped_and_named(self):
        # The span is the evidence; evidence with the wrong coordinates
        # is not evidence. It is dropped, not clamped.
        spans, problems = extraction._located(
            {"invoice_id": [_span("INV-1", 0, 5, 0.99)]}, TEXT
        )
        self.assertEqual(spans, [])
        self.assertEqual(len(problems), 1)
        self.assertIn("do not slice back", problems[0])

    def test_a_span_without_offsets_is_named(self):
        spans, problems = extraction._located({"due": [{"text": "2026-11-30"}]}, TEXT)
        self.assertEqual(spans, [])
        self.assertIn("without text or offsets", problems[0])


class TheExtractVerb(unittest.TestCase):
    def _run(self, reply=None, key="test-key", **over):
        cfg = {"providers": {}}
        settings = ("https://example.invalid/v1/chat/completions", "model-x", key)
        kwargs = dict(cfg=cfg)
        kwargs.update(over)
        with mock.patch.object(extraction, "_settings", return_value=settings), mock.patch.object(
            extraction, "http_post_json", return_value=reply or _reply({"entities": {}})
        ) as post:
            # log=False: these pins are about the parse and the decline,
            # not about writing into the operator's JSONL log.
            return extraction.extract(kwargs.pop("text", TEXT), kwargs.pop("fields", [
                {"name": "invoice_id", "description": "the invoice number"}
            ]), log=False, **kwargs), post

    def test_the_call_is_the_entity_shape_the_legs_document_asks(self):
        result, post = self._run(
            reply=_reply({"entities": {}}),
        )
        self.assertTrue(result["ok"])
        body = post.call_args.args[1]
        self.assertEqual(body["model"], "model-x")
        self.assertEqual(body["schema"]["entities"][0]["name"], "invoice_id")
        self.assertEqual(body["threshold"], extraction.ENTITY_THRESHOLD)
        self.assertIs(body["include_spans"], True)
        self.assertIs(body["include_confidence"], True)
        self.assertIs(body["store"], False)

    def test_an_empty_field_is_declined_by_name(self):
        at = TEXT.find("INV-1")
        result, _ = self._run(
            fields=[
                {"name": "invoice_id", "description": "the invoice number"},
                {"name": "due_date", "description": "the due date"},
            ],
            reply=_reply({"entities": {"invoice_id": [_span("INV-1", at, at + 5)]}}),
        )
        self.assertTrue(result["ok"])
        self.assertEqual([span["field"] for span in result["spans"]], ["invoice_id"])
        self.assertEqual(result["declined"], ["due_date"])

    def test_a_mismatched_offset_is_a_named_problem_not_a_span(self):
        result, _ = self._run(reply=_reply({"entities": {"invoice_id": [_span("INV-1", 0, 5)]}}))
        self.assertTrue(result["ok"])
        self.assertEqual(result["spans"], [])
        self.assertEqual(result["declined"], ["invoice_id"])
        self.assertEqual(len(result["problems"]), 1)

    def test_no_key_is_a_named_failure_not_an_empty_read(self):
        result, post = self._run(key=None)
        self.assertFalse(result["ok"])
        self.assertEqual(result["code"], "no_key")
        post.assert_not_called()

    def test_a_non_json_reply_is_a_named_parse_failure(self):
        result, _ = self._run(reply={"choices": [{"message": {"content": "not json"}}]})
        self.assertFalse(result["ok"])
        self.assertEqual(result["code"], "parse")

    def test_an_empty_text_is_refused(self):
        result, post = self._run(text="   ")
        self.assertFalse(result["ok"])
        self.assertEqual(result["code"], "empty")
        post.assert_not_called()


class TheExtractSettings(unittest.TestCase):
    """The config's `extract_*` entries are the authority."""

    def test_the_key_env_is_the_extraction_vendors_own(self):
        cfg = {"providers": {"extract_api_key_env": "FLEET_TEST_EXTRACT_KEY"}}
        with mock.patch.dict(os.environ, {"FLEET_TEST_EXTRACT_KEY": "k-1"}, clear=False):
            url, model, key = extraction._settings(cfg)
        self.assertEqual(key, "k-1")
        self.assertEqual(model, extraction.DEFAULT_EXTRACT_MODEL)
        self.assertEqual(url, extraction.DEFAULT_EXTRACT_URL)

    def test_an_env_override_wins_over_the_table(self):
        cfg = {"providers": {"extract_api_url": "https://table.invalid/x"}}
        env = {"SYS1_EXTRACT_URL": "https://env.invalid/x", "SYS1_EXTRACT_MODEL": "m-2"}
        with mock.patch.dict(os.environ, env, clear=False):
            url, model, _ = extraction._settings(cfg)
        self.assertEqual(url, "https://env.invalid/x")
        self.assertEqual(model, "m-2")


class TheLanesOwnLine(unittest.TestCase):
    """`sys1_extract` writes its own telemetry line — counts, never text."""

    def test_the_extract_call_leaves_one_line_with_no_text(self):
        import tempfile

        at = TEXT.find("INV-1")
        with tempfile.TemporaryDirectory() as log_dir:
            cfg = {"classify": {"log_dir": log_dir}, "providers": {}}
            settings = ("https://example.invalid/v1/chat/completions", "model-x", "k")
            reply = _reply({"entities": {"invoice_id": [_span("INV-1", at, at + 5, 0.91)]}})
            with mock.patch.object(extraction, "_settings", return_value=settings), mock.patch.object(
                extraction, "http_post_json", return_value=reply
            ):
                result = extraction.extract(
                    TEXT, [{"name": "invoice_id", "description": "the invoice number"}], cfg
                )
            self.assertTrue(result["ok"])
            lines = []
            for root, _, files in os.walk(log_dir):
                for name in files:
                    with open(os.path.join(root, name), encoding="utf-8") as handle:
                        lines.extend(json.loads(line) for line in handle if line.strip())
            self.assertEqual(len(lines), 1)
            line = lines[0]
            self.assertEqual(line["op"], "extract")
            self.assertTrue(line["ok"])
            self.assertEqual(line["spans"], 1)
            self.assertEqual(line["fields"], ["invoice_id"])
            self.assertIn("latency_ms", line)
            # The document and the span text never reach the log.
            self.assertNotIn("INV-1", json.dumps(line))
            self.assertNotIn(TEXT, json.dumps(line))


class TheClassifySeam(unittest.TestCase):
    def test_a_task_spec_wins_over_the_registered_name(self):
        task = mcp_server.task_for_arguments(
            {
                "task": "change",
                "task_spec": {
                    "id": "extract-verify",
                    "heads": [
                        {
                            "id": "total_due",
                            "kind": "noul",
                            "task": "Does this text state the total due?",
                            "descriptions": {"true": "Yes.", "false": "No."},
                        }
                    ],
                },
            },
            task_from_spec,
        )
        self.assertNotEqual(task, "change")
        self.assertEqual(task.id, "extract-verify")
        self.assertEqual([head.id for head in task.heads], ["total_due"])

    def test_no_spec_falls_back_to_the_registered_name(self):
        self.assertEqual(mcp_server.task_for_arguments({"task": "curate"}, task_from_spec), "curate")
        self.assertEqual(mcp_server.task_for_arguments({}, task_from_spec), "change")

    def test_a_spec_without_an_id_is_not_a_spec(self):
        task = mcp_server.task_for_arguments(
            {"task": "change", "task_spec": {"heads": []}}, task_from_spec
        )
        self.assertEqual(task, "change")


class TheStdioSurface(unittest.TestCase):
    """The handler's own refusals, driven over the real stdio transport.

    No vendor call: both requests are refused before the classify/extract
    call is reached.
    """

    def _drive(self, requests):
        env = dict(os.environ)
        env["SYS1_PYTHONPATH"] = os.path.join(ROOT, "src")
        payload = "".join(json.dumps(request) + "\n" for request in requests)
        proc = subprocess.run(
            [sys.executable, os.path.join(ROOT, "service", "mcp_server.py")],
            input=payload,
            capture_output=True,
            text=True,
            env=env,
            timeout=60,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        lines = [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]
        return {line["id"]: line for line in lines}

    def test_the_new_tool_is_listed_and_empty_inputs_are_refused(self):
        replies = self._drive(
            [
                {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
                {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
                {
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {"name": "sys1_extract", "arguments": {"text": "", "fields": []}},
                },
                {
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "tools/call",
                    "params": {
                        "name": "sys1_classify",
                        "arguments": {"text": "x", "task_spec": {"id": "t", "heads": [{"kind": "noul"}]}},
                    },
                },
            ]
        )
        names = [tool["name"] for tool in replies[2]["result"]["tools"]]
        self.assertIn("sys1_extract", names)
        classify = next(tool for tool in replies[2]["result"]["tools"] if tool["name"] == "sys1_classify")
        self.assertIn("task_spec", classify["inputSchema"]["properties"])
        self.assertEqual(replies[3]["result"]["error"]["code"], -32602)
        self.assertIn("task_spec", replies[4]["result"]["error"]["message"])
        self.assertIn("refused", replies[4]["result"]["error"]["message"])


class TheStdoutIsJsonRpcOnly(unittest.TestCase):
    def test_the_module_imports_without_running_the_loop(self):
        # Importing must not start a stdio session: the handler's `main()`
        # is the only entry point.
        buffer = io.StringIO()
        with redirect_stdout(buffer):
            import importlib

            importlib.reload(mcp_server)
        self.assertEqual(buffer.getvalue(), "")


if __name__ == "__main__":
    unittest.main()
