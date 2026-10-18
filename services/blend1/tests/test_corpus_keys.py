"""Tests for blend1's corpus derivation — the two keys the KI triage fixed.

The derivation is the lane's own, so its tests live beside the adapter's
in this vendored tree's tests/ dir. These pin:

- KI-223: the chunk key is the source's ABSOLUTE path, so the same
  source derives the same key set from any working directory. The key
  used to be relative to the process's cwd, which made the refresh's
  change detection report "rebuilt" for byte-identical text whenever the
  lane was launched from a different directory.
- KI-225: a declared source that resolves to nothing is named in the
  refresh's `missing_sources`, rather than the refresh deriving
  silently from whatever remained.

Run: python3 -m pytest services/blend1/tests/test_corpus_keys.py
"""

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from blend1 import corpus  # noqa: E402


def test_the_key_is_the_absolute_path_so_the_cwd_cannot_poison_it(tmp_path):
    """KI-223: the same source derives the same keys from any cwd."""
    source = tmp_path / "doc.md"
    source.write_text("# Title\n\n## One\n\nfirst\n\n## Two\n\nsecond\n", encoding="utf-8")
    absolute = str(source.resolve())

    here = os.getcwd()
    try:
        items_from_here, _, _ = corpus._derive([absolute])
        os.chdir(tmp_path)
        items_from_there, _, _ = corpus._derive([absolute])
    finally:
        os.chdir(here)

    keys_here = [item["key"] for item in items_from_here]
    keys_there = [item["key"] for item in items_from_there]
    assert keys_here == keys_there, (
        f"the same source derived different keys per cwd:\n{keys_here}\n{keys_there}"
    )
    assert all(key.startswith(str(tmp_path.resolve())) for key in keys_here), keys_here


def test_a_source_that_does_not_exist_is_named_not_skipped(tmp_path):
    """KI-225: a declared source resolving to nothing rides the report."""
    present = tmp_path / "here.md"
    present.write_text("## Only\n\ntext\n", encoding="utf-8")
    absent = str(tmp_path / "gone.md")

    files, missing = corpus._source_files([str(present), absent])
    assert [path.name for path in files] == ["here.md"], files
    assert missing == [absent], f"the absent source is named: {missing}"


def test_the_missing_sources_ride_the_refresh_report(tmp_path, monkeypatch=None):
    """The refresh's report carries them, so a stale job says so."""
    report = corpus.refresh(project="ki-225-probe", sources=[str(tmp_path / "gone.md")])
    assert report["action"] == "no-source", report
    assert report["missing_sources"] == [str(tmp_path / "gone.md")], report
