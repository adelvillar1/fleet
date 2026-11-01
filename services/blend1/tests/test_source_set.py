"""W2's pins: one source-set contract (KI-339/343/345/348).

- KI-343: a wildcard matching nothing is SATISFIABLE and named apart
  from a literal path that matches nothing (dark, KI-225's class).
- KI-345: the default set is the job convention's own — every root .md
  beside docs/**/*.md — so a default-built and a job-built store agree
  and README is not the file that forces a full re-embed.
- KI-339: when the DEFAULT set resolves to nothing, the report names
  the cwd it resolved against instead of the quiet no-source.
- KI-348: the report carries the source-set delta since the stored
  manifest, so a definition change reads as itself.

Run: python3 services/blend1/tests/test_source_set.py
"""

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
sys.path.insert(
    0, str(Path(__file__).resolve().parent.parent.parent / "sem1" / "src")
)

from blend1 import corpus  # noqa: E402
import sem1  # noqa: E402
from sem1.types import EmbedResult  # noqa: E402


def _stub_embed(items, **_kwargs):
    return EmbedResult(
        vectors=[[0.0, 0.0, 1.0] for _ in items],
        dims=3,
        model="stub",
        provider="stub",
        telemetry={},
        extras={},
    )


def _chdir(directory):
    previous = os.getcwd()
    os.chdir(directory)
    return previous


def test_a_wildcard_matching_nothing_is_satisfiable_not_dark():
    """KI-343: the two classes split — a literal that does not exist is
    dark and lands in missing_sources; a wildcard that matched nothing
    is satisfiable and lands in empty_wildcards."""
    scratch = tempfile.mkdtemp(prefix="fleet-source-set-")
    Path(scratch, "docs").mkdir()
    Path(scratch, "docs", "guide.md").write_text("# Guide\n\n## One\n\ntext\n")
    previous = _chdir(scratch)
    try:
        files, missing, empty = corpus._source_files(
            ["docs/**/*.md", "root-ghost.md", "no-such-dir/*.md"]
        )
        assert [path.name for path in files] == ["guide.md"], files
        assert missing == ["root-ghost.md"], missing
        assert empty == ["no-such-dir/*.md"], empty
    finally:
        _chdir(previous)


def test_the_default_set_is_the_job_convention_whole():
    """KI-345: every root .md beside docs — README included — so the
    default and seed_task's glob say one thing."""
    scratch = tempfile.mkdtemp(prefix="fleet-default-set-")
    Path(scratch, "docs").mkdir()
    Path(scratch, "docs", "guide.md").write_text("# Guide\n\n## One\n\ntext\n")
    Path(scratch, "README.md").write_text("# Readme\n\n## Intro\n\nhello\n")
    Path(scratch, "CLAUDE.md").write_text("# Claude\n\n## Rule\n\nobey\n")
    previous = _chdir(scratch)
    try:
        files, missing, empty = corpus._source_files(None)
        names = sorted(path.name for path in files)
        assert names == ["CLAUDE.md", "README.md", "guide.md"], names
        assert missing == [] and empty == [], (missing, empty)
    finally:
        _chdir(previous)


def test_the_default_zero_names_its_cwd():
    """KI-339: a default set that resolves to nothing says WHERE it
    looked — the fact a scheduled job needs when the child's cwd is the
    lane home rather than the project."""
    scratch = tempfile.mkdtemp(prefix="fleet-default-zero-")
    previous = _chdir(scratch)
    vectors = tempfile.mkdtemp(prefix="fleet-default-zero-store-")
    previous_vectors = os.environ.get("BLEND1_VECTORS_DIR")
    os.environ["BLEND1_VECTORS_DIR"] = vectors
    try:
        report = corpus.refresh(project="ghost", sources=None)
        assert report["action"] == "no-source", report
        assert "reason" in report, report
        assert str(Path.cwd()) in report["reason"], report["reason"]
        assert "scheduled" in report["reason"], report["reason"]
    finally:
        _chdir(previous)
        if previous_vectors is None:
            os.environ.pop("BLEND1_VECTORS_DIR", None)
        else:
            os.environ["BLEND1_VECTORS_DIR"] = previous_vectors


def test_the_report_carries_the_source_set_delta():
    """KI-348: added and removed sources name themselves — a definition
    change is distinguishable from a content change from the report
    alone. The embedder is stubbed: the store writes, the machine's
    live server is never consulted."""
    real_embed = sem1.embed
    sem1.embed = _stub_embed
    scratch = tempfile.mkdtemp(prefix="fleet-source-delta-")
    vectors = tempfile.mkdtemp(prefix="fleet-source-delta-store-")
    previous_vectors = os.environ.get("BLEND1_VECTORS_DIR")
    os.environ["BLEND1_VECTORS_DIR"] = vectors
    Path(scratch, "one.md").write_text("# One\n\n## First\n\nalpha\n")
    Path(scratch, "two.md").write_text("# Two\n\n## Second\n\nbeta\n")
    previous = _chdir(scratch)
    try:
        first = corpus.refresh(
            project="delta", sources=[str(Path(scratch) / "one.md")]
        )
        assert first["action"] == "rebuilt", first
        assert first["sources_added"] and not first["sources_removed"], first

        # A definition change: the set gains a file and loses none.
        widened = corpus.refresh(
            project="delta",
            sources=[
                str(Path(scratch) / "one.md"),
                str(Path(scratch) / "two.md"),
            ],
        )
        assert widened["action"] == "rebuilt", widened
        assert widened["sources_added"] == [str((Path(scratch) / "two.md").resolve())], widened
        assert widened["sources_removed"] == [], widened

        # Narrowing back: the removed file names itself.
        narrowed = corpus.refresh(
            project="delta", sources=[str(Path(scratch) / "one.md")]
        )
        assert narrowed["action"] == "rebuilt", narrowed
        assert narrowed["sources_added"] == [], narrowed
        assert narrowed["sources_removed"] == [str((Path(scratch) / "two.md").resolve())], narrowed

        # Steady state: no delta, unchanged.
        steady = corpus.refresh(
            project="delta", sources=[str(Path(scratch) / "one.md")]
        )
        assert steady["action"] == "unchanged", steady
        assert steady["sources_added"] == [] and steady["sources_removed"] == [], steady
    finally:
        _chdir(previous)
        sem1.embed = real_embed
        if previous_vectors is None:
            os.environ.pop("BLEND1_VECTORS_DIR", None)
        else:
            os.environ["BLEND1_VECTORS_DIR"] = previous_vectors


if __name__ == "__main__":
    test_a_wildcard_matching_nothing_is_satisfiable_not_dark()
    print("ok: a wildcard matching nothing is satisfiable, not dark")
    test_the_default_set_is_the_job_convention_whole()
    print("ok: the default set is the job convention, whole")
    test_the_default_zero_names_its_cwd()
    print("ok: the default zero names its cwd")
    test_the_report_carries_the_source_set_delta()
    print("ok: the report carries the source-set delta")
    print("all passed")
