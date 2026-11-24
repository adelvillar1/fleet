#!/usr/bin/env python3
"""Repair session logs whose `start` record names the wrong workspace.

The defect this repairs (the owner's report, 2026-11-24): the face created a
conversation by writing the **process's** working directory into the log's
`start` record, before it assigned the chat the folder the owner had chosen.
Run from inside a project the two agreed by luck; the desktop app runs with
cwd `/`, so a chat with `/Users/.../fleet` on file still grouped under
`no project` in the conversations list.

The writer is fixed (`Session::create_in`, and the run thread reads the
run's own folder at birth). This repairs the logs already written.

## What it will and will not do

- It rewrites **only** the `workspace` value of the first line's JSON, and
  only when that value is null, `/`, or a path that no longer exists.
- Everything else on the line keeps its exact bytes: the JSON is decoded,
  one key replaced, and re-encoded with the original separators. The rest of
  the file is untouched — the log is append-only and stays that way.
- The authority is `<home>/sessions-workdirs.json`, the face's own record of
  the folder the owner chose for that chat. A session absent from it is left
  alone: this never invents a project.
- Every rewrite keeps a `.pre-repair` copy of the original line 1, writes
  atomically (temp + rename), and is reported by id and path.

Usage:
    python3 scripts/repair_session_workspace.py            # dry run
    python3 scripts/repair_session_workspace.py --apply    # rewrite
    python3 scripts/repair_session_workspace.py --home DIR
"""
import argparse
import json
import os
import pathlib
import sys
import tempfile


def read_start(path):
    """The first line's parsed JSON, or None when it is not a start record."""
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        first = handle.readline()
    if not first.strip():
        return None
    try:
        record = json.loads(first)
    except json.JSONDecodeError:
        return None
    return record if record.get("type") == "start" else None


def needs_repair(record, wanted):
    """Whether this start record's workspace is wrong, given the folder on file."""
    if not wanted:
        return False
    recorded = record.get("workspace")
    if recorded is None or recorded == "/":
        return True
    # A workspace that points at a directory which no longer exists is the
    # same stale fact; the folder the owner chose is the better record.
    return not os.path.isdir(str(recorded)) and os.path.isdir(wanted)


def retire_args(original):
    """The separators/key order the original line used, so re-encoding is stable."""
    has_space_after_colon = '": ' in original
    has_space_after_comma = '", ' in original or '",' in original
    return has_space_after_colon, has_space_after_comma


def rewrite(path, wanted, apply):
    """Replace one key on line 1; return the (old, new) workspace or None."""
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        original = handle.read()
    newline_at = original.find("\n")
    first = original if newline_at < 0 else original[: newline_at + 1]
    rest = "" if newline_at < 0 else original[newline_at + 1 :]

    record = json.loads(first)
    old = record.get("workspace")
    record["workspace"] = wanted
    separators = (", ", ": ") if retire_args(first)[0] else (",", ":")
    encoded = json.dumps(record, separators=separators) + ("\n" if first.endswith("\n") else "")

    if apply:
        backup = path + ".pre-repair"
        with open(backup, "w", encoding="utf-8") as handle:
            handle.write(first)
        directory = os.path.dirname(path)
        with tempfile.NamedTemporaryFile(
            "w", dir=directory, delete=False, encoding="utf-8"
        ) as handle:
            handle.write(encoded)
            handle.write(rest)
            temp = handle.name
        os.replace(temp, path)
    return old, wanted


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="rewrite the logs")
    parser.add_argument(
        "--home",
        default=os.path.expanduser("~/.fleet"),
        help="the fleet home to repair (default ~/.fleet)",
    )
    args = parser.parse_args(argv)

    home = pathlib.Path(args.home)
    sessions = home / "sessions"
    workdirs_path = home / "sessions-workdirs.json"
    if not sessions.is_dir():
        print(f"repair: {sessions} is not a directory", file=sys.stderr)
        return 2
    if not workdirs_path.exists():
        print(f"repair: {workdirs_path} is absent — nothing names a chosen folder")
        return 0
    workdirs = json.loads(workdirs_path.read_text(encoding="utf-8"))

    findings = []
    for path in sorted(sessions.glob("*.jsonl")):
        session_id = path.stem
        wanted = workdirs.get(session_id)
        record = read_start(path)
        if record is None or not needs_repair(record, wanted):
            continue
        findings.append((session_id, path, wanted))

    if not findings:
        print("repair: every start record agrees with the folder on file")
        return 0

    verb = "repaired" if args.apply else "would repair"
    for session_id, path, wanted in findings:
        old, new = rewrite(str(path), wanted, args.apply)
        print(f"{verb} {session_id}: workspace {old!r} -> {new!r}")

    tail = "repairing" if args.apply else "run with --apply to write"
    print(f"\n{len(findings)} session(s) {verb} — {tail}.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
