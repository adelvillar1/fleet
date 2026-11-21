"""Laya as an in-process sys1 provider: availability, questions, parse.

Run from `services/sys1`:  python3 -m unittest discover -s tests -t .

The unit pins are stdlib-only and hermetic: no venv, no model, no network.
The live leg at the bottom runs ONLY when the laya venv exists and the
package is in it — a missing venv SKIPS by name, because a skip is not a
pass (the availability rule this provider is built on).
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "src"))
sys.path.insert(0, os.path.join(ROOT, "service"))

from sys1 import providers  # noqa: E402
from sys1.parsers import parse_laya  # noqa: E402
from sys1.types import Head  # noqa: E402


def _cfg(**overrides) -> dict:
    cfg = providers.load_config()
    cfg["providers"].update(overrides)
    return cfg


def _choice() -> Head:
    return Head(
        id="workload",
        kind="choice",
        task="Which workload shape does this request need?",
        labels=["quick", "standard_code", "hard"],
    )


def _score() -> Head:
    return Head(
        id="risk",
        kind="score",
        task="How risky is this change?",
        labels=["0", "1", "2"],
        descriptions={"0": "safe", "1": "careful", "2": "dangerous"},
        score_min=1,
    )


def _noul() -> Head:
    return Head(id="breaking", kind="noul", task="Does this break an existing caller?")


class TheAvailabilityIsTheVenv(unittest.TestCase):
    """Availability means the package is IN the venv — never a server on a port."""

    def test_a_venv_with_the_package_in_it_is_available(self):
        with tempfile.TemporaryDirectory() as tmp:
            venv = Path(tmp) / "venvs" / "laya"
            (venv / "bin").mkdir(parents=True)
            (venv / "bin" / "python").write_text("#!/bin/sh\n")
            package = venv / "lib" / "python3.12" / "site-packages" / "laya"
            package.mkdir(parents=True)
            (package / "__init__.py").write_text("")
            cfg = _cfg(laya_venv=str(venv), laya_api_url="")
            self.assertTrue(providers._laya_installed(cfg))
            self.assertTrue(providers._laya_available(cfg))
            # Not enabled yet: installed and available, but out of the chain.
            self.assertFalse(providers.provider_report(cfg)["laya"]["available"])
            cfg["providers"]["enabled"] = "core,laya"
            self.assertTrue(providers.provider_report(cfg)["laya"]["available"])
            self.assertEqual(providers.availability_reason("laya", cfg), "available")

    def test_a_bare_venv_is_not_availability(self):
        """`uv venv` writes bin/python before any package lands: that is not a lane."""
        with tempfile.TemporaryDirectory() as tmp:
            venv = Path(tmp) / "venvs" / "laya"
            (venv / "bin").mkdir(parents=True)
            (venv / "bin" / "python").write_text("#!/bin/sh\n")
            cfg = _cfg(laya_venv=str(venv), laya_api_url="")
            self.assertFalse(providers._laya_installed(cfg))
            self.assertFalse(providers._laya_available(cfg))

    def test_a_missing_venv_names_the_press_that_builds_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            cfg = _cfg(laya_venv=str(Path(tmp) / "absent"), laya_api_url="", enabled="core,laya")
            self.assertFalse(providers._laya_available(cfg))
            reason = providers.availability_reason("laya", cfg)
            self.assertIn("not installed", reason)
            self.assertIn("fleet install --provider laya", reason)
            probe = providers.probe("laya", cfg)
            self.assertFalse(probe["ok"])
            self.assertIn("fleet install --provider laya", probe["detail"])

    def test_the_config_view_names_the_venv_and_keeps_the_served_url(self):
        cfg = _cfg(laya_venv="/tmp/laya-venv", laya_api_url="http://127.0.0.1:8454/v1")
        view = providers.provider_config_view("laya", cfg)
        self.assertEqual(view["kind"], "venv")
        self.assertEqual(view["venv"], "/tmp/laya-venv")
        self.assertEqual(view["served_url"], "http://127.0.0.1:8454/v1")

    def test_laya_stays_a_candidate_and_out_of_the_default_chain(self):
        """The standing rule: it competes by being measurable, never by being pre-enabled."""
        self.assertEqual(providers.REGISTRY["laya"].tier, "candidate")
        self.assertNotIn("laya", providers.enabled_set(_cfg(enabled="core")))
        chain = _cfg()["providers"]["route"]["default_chain"]
        self.assertEqual(chain, "glide,drex,jev")
        self.assertNotIn("laya", chain)

    def test_the_served_endpoint_is_still_the_fallback(self):
        """`laya_api_url` is not dead: a listener there makes it available."""
        cfg = _cfg(
            laya_venv="/nonexistent/laya",
            laya_api_url="http://127.0.0.1:1/v1",
            enabled="core,laya",
        )
        self.assertFalse(providers._laya_available(cfg))
        with mock.patch.object(providers, "_tcp_open", return_value=True):
            self.assertTrue(providers._laya_available(cfg))
            self.assertIn("fallback endpoint", providers.availability_reason("laya", cfg))


class TheQuestionsLayaIsAsked(unittest.TestCase):
    def test_a_choice_head_carries_its_labels_as_criteria(self):
        questions = providers.laya_questions([_choice()])
        self.assertEqual(questions["workload"]["type"], "choice")
        self.assertEqual(questions["workload"]["criteria"]["quick"], "quick")
        self.assertEqual(questions["workload"]["instructions"], _choice().task)

    def test_a_score_head_carries_its_ordered_levels(self):
        questions = providers.laya_questions([_score()])
        self.assertEqual(questions["risk"]["type"], "score")
        self.assertEqual(questions["risk"]["criteria"], ["safe", "careful", "dangerous"])

    def test_a_noul_head_carries_the_question_alone(self):
        questions = providers.laya_questions([_noul()])
        self.assertEqual(questions["breaking"], {"type": "noul", "instructions": _noul().task})

    def test_a_choice_head_with_no_labels_is_not_asked(self):
        bare = Head(id="empty", kind="choice", task="?", labels=[])
        self.assertEqual(providers.laya_questions([bare]), {})


class TheAnswersLayaGives(unittest.TestCase):
    """The three heads, and the three rules the wire must not break."""

    def test_a_choice_answer_carries_the_top1_probability_as_confidence(self):
        out = parse_laya(
            {
                "answers": {
                    "workload": {
                        "type": "choice",
                        "choice": "quick",
                        "probabilities": {"quick": 0.82, "standard_code": 0.12, "hard": 0.06},
                        "answer_confidence": 0.82,
                        "confidence": 0.70,
                        "prediction_set": ["quick"],
                        "abstain": False,
                        "unbound": False,
                        "route": "auto",
                        "route_reason": "conformal singleton, not abstaining",
                    }
                }
            },
            [_choice()],
            providers.laya_questions([_choice()]),
        )
        answer = out["workload"]
        self.assertEqual(answer["label"], "quick")
        self.assertEqual(answer["confidence"], 0.82)
        self.assertEqual(answer["laya_confidence"], 0.70)  # kept apart, not substituted
        self.assertEqual(answer["prediction_set"], ["quick"])
        self.assertEqual(answer["route"], "auto")
        self.assertNotIn("_declined", answer)

    def test_an_abstain_is_a_refusal_not_a_guess(self):
        """judge.rs's fail_open and route.rs's parse_workload read a missing label as no answer."""
        out = parse_laya(
            {
                "answers": {
                    "workload": {
                        "type": "choice",
                        "choice": "quick",
                        "probabilities": {"quick": 0.30, "standard_code": 0.19, "hard": 0.04},
                        "answer_confidence": 0.30,
                        "prediction_set": ["quick", "standard_code"],
                        "abstain": True,
                        "unbound": True,
                        "route": "review",
                        "route_reason": "the engine abstains at risk 0.1",
                    }
                }
            },
            [_choice()],
        )
        answer = out["workload"]
        self.assertIsNone(answer["label"])
        self.assertIsNone(answer["confidence"])
        self.assertIn("abstain", answer["_declined"])
        self.assertTrue(answer["ambiguous"])
        self.assertEqual(answer["unbound"], True)

    def test_a_multi_option_prediction_set_is_surfaced_not_hidden(self):
        out = parse_laya(
            {"answers": {"workload": {"choice": "hard", "answer_confidence": 0.51,
                                      "prediction_set": ["hard", "standard_code"], "abstain": False}}},
            [_choice()],
        )
        self.assertEqual(out["workload"]["prediction_set"], ["hard", "standard_code"])
        self.assertTrue(out["workload"]["ambiguous"])

    def test_a_score_maps_the_level_index_back_to_score_min(self):
        out = parse_laya(
            {
                "answers": {
                    "risk": {
                        "type": "score",
                        "score": 1,
                        "expected_score": 1.42,
                        "answer_confidence": 0.55,
                        "probabilities": {"0": 0.02, "1": 0.55, "2": 0.43},
                        "abstain": False,
                        "prediction_set": ["1"],
                    }
                }
            },
            [_score()],
            providers.laya_questions([_score()]),
        )
        answer = out["risk"]
        self.assertEqual(answer["score"], 2.0)  # score_min 1 + index 1
        self.assertEqual(answer["legend"], {"1": "safe", "2": "careful", "3": "dangerous"})
        self.assertEqual(answer["expected_score"], 1.42)

    def test_a_noul_carries_no_confidence_field(self):
        out = parse_laya(
            {"answers": {"breaking": {"type": "noul", "noul": 0.071,
                                      "probabilities": {"true": 0.071, "false": 0.929},
                                      "abstain": False}}},
            [_noul()],
        )
        answer = out["breaking"]
        self.assertEqual(answer["noul"], 0.071)
        self.assertNotIn("confidence", answer)
        self.assertNotIn("label", answer)

    def test_a_missing_head_declines(self):
        out = parse_laya({"answers": {}}, [_choice()])
        self.assertEqual(out["workload"]["_declined"], "missing")

    def test_an_unreadable_envelope_declines_every_head(self):
        out = parse_laya({"nothing": 1}, [_choice(), _noul()])
        self.assertEqual(set(out), {"workload", "breaking"})
        self.assertEqual(out["workload"]["_declined"], "missing")

    def test_no_confidence_is_invented(self):
        out = parse_laya(
            {"answers": {"workload": {"choice": "quick", "probabilities": {"quick": 1.0}}}},
            [_choice()],
        )
        self.assertIsNone(out["workload"]["confidence"])


class TheReadinessProbe(unittest.TestCase):
    """KI-030: the handshake says the lane answers, never that a provider does."""

    def test_a_missing_venv_is_the_probes_honest_failure(self):
        import mcp_server

        with tempfile.TemporaryDirectory() as tmp:
            cfg = _cfg(laya_venv=str(Path(tmp) / "absent"), laya_api_url="")
            report = mcp_server.probe_providers(cfg)
        entry = report["laya"]
        self.assertFalse(entry["available"])
        self.assertIn("not installed", entry["reason"])
        self.assertIn("fleet install --provider laya", entry["reason"])
        # The shape Fleet's parse_health_map reads: a name, a bool, a reason.
        self.assertIsInstance(entry["name"], str)
        self.assertIsInstance(entry["available"], bool)
        self.assertIsInstance(entry["reason"], str)

    def test_the_probe_reports_the_route_head_not_a_toy_question(self):
        """The round trip asks the workload head the router actually asks."""
        import mcp_server

        with tempfile.TemporaryDirectory() as tmp:
            cfg = _cfg(laya_venv=str(Path(tmp) / "absent"), laya_api_url="")
            # The probe refuses before it asks; the state it WOULD send is
            # the judge's own evidence record, not a toy string.
            self.assertIn("the workload shape the router judged", mcp_server.PROBE_STATE)
            self.assertIn("the run's outcome", mcp_server.PROBE_STATE)
            self.assertEqual(mcp_server.probe_providers(cfg)["laya"]["available"], False)


def _laya_venv_python() -> Path:
    cfg = providers.load_config()
    return providers._laya_venv_python(cfg)


@unittest.skipUnless(
    _laya_venv_python().exists(),
    "no laya venv at the configured path — `fleet install --provider laya` builds it",
)
class TheLiveRoundTrip(unittest.TestCase):
    """One real in-process call through the production wire and parser."""

    def test_the_route_workload_head_answers_through_the_parser(self):
        from sys1.tasks import get_task

        task = get_task("route")
        heads = [h for h in task.heads if h.id == "workload"]
        cfg = providers.load_config()
        cfg["providers"]["enabled"] = "core,laya"
        telemetry: dict = {}
        text = (
            "the workload shape the router judged: quick\n"
            "how: judged\nsys1 verdict: pass\nreason: single-file edit\n"
            "input chars: 120\nthe run's outcome: ok (completed)\n"
        )
        out = providers.REGISTRY["laya"].classify(text, heads, cfg, telemetry)
        answer = out["workload"]
        self.assertTrue(telemetry.get("structured_ok"), telemetry)
        self.assertEqual(telemetry.get("mode"), "in-process")
        if answer.get("label") is None:
            # An abstain is the legal refusal: no label, and it says so.
            self.assertIn("abstain", str(answer.get("_declined", "")))
        else:
            self.assertIn(answer["label"], heads[0].labels)
            self.assertGreaterEqual(answer["confidence"], 0.0)
            self.assertLessEqual(answer["confidence"], 1.0)
        # The engine's own statistics survive the parse when it gives them:
        # a classic encoder answer carries none of these, and the parse does
        # not invent them.
        for field in ("prediction_set", "route", "abstain", "unbound"):
            self.assertIn(field, answer, f"{field} missing from {answer}")
        self.assertIn("probabilities", answer)
        self.assertEqual(len(answer["probabilities"]), len(heads[0].labels))

    def test_the_probe_reports_a_typed_round_trip_and_both_memory_facts(self):
        import mcp_server

        cfg = providers.load_config()
        entry = mcp_server.probe_providers(cfg)["laya"]
        self.assertTrue(entry["available"], entry)
        self.assertIn("venv ok", entry["reason"])
        self.assertIn("model loaded", entry["reason"])
        # An abstain is a PASS: the engine declining to decide is the
        # semantics, not a failure of the probe.
        self.assertIn(entry["answer"], {"abstained"} | set(heads_labels()))
        # Two memory facts, never conflated: the load transient (a
        # high-water mark) and the resident size now.
        self.assertIn("peak_rss_mib", entry, entry)
        self.assertIn("rss_mib", entry, entry)
        self.assertLess(entry["rss_mib"], entry["peak_rss_mib"])

    def test_the_resident_child_is_reused_and_dies_on_close(self):
        from sys1.tasks import get_task

        cfg = providers.load_config()
        head = next(h for h in get_task("route").heads if h.id == "workload")
        text = "the workload shape the router judged: quick\nthe run's outcome: ok\n"
        first = providers.REGISTRY["laya"].classify(text, [head], cfg, {})
        child = providers._laya_child(cfg)
        pid = child._proc.pid
        second = providers.REGISTRY["laya"].classify(text, [head], cfg, {})
        self.assertEqual(providers._laya_child(cfg)._proc.pid, pid, "one child, reused")
        self.assertEqual(set(first), set(second))
        providers._close_laya_children()
        self.assertIsNone(child._proc, "closing forgets the child")
        self.assertNotIn((str(providers._laya_venv_python(cfg)), str(cfg["providers"].get("laya_model") or "")), providers._LAYA_CHILDREN)


def heads_labels():
    from sys1.tasks import get_task

    return [h.id for h in get_task("route").heads]


if __name__ == "__main__":
    unittest.main()
