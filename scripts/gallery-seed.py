#!/usr/bin/env python3
"""gallery-seed.py — write the scratch FLEET_HOME the doc gallery shoots from.

The 2026-11-04 gallery was seeded by a script that lived in /tmp and is
gone; this is the committed replacement. It writes a SELF-CONTAINED home
that the gallery shoots from — the only thing read from the owner's
machine is the public skills library (`~/.agents/skills`), six canonical
skills copied in. Nothing under `~/.fleet` is read or written.

Usage:
  python3 scripts/gallery-seed.py                       # /tmp/fleet-gallery-home
  python3 scripts/gallery-seed.py --home /tmp/other     # a different scratch dir

Idempotent: the target directory is wiped and rewritten in full on every
run. The script refuses any target that resolves to the owner's
`~/.fleet` — the home it writes is always a scratch copy.

Then serve it from the scratch PROJECT (the board reads
`<cwd>/docs/plans`; never :3080 — that is the owner's live console):

  python3 scripts/gallery-seed.py                 # writes home + project
  cd /tmp/fleet-gallery-project && \
    FLEET_HOME=/tmp/fleet-gallery-home ./target/debug/fleet gui --port 3099 --no-open

The seed writes `gallery-manifest.json` into the home (session ids the
shoot script resolves `#room` placeholders against) and prints both paths.
The substrate probe's fixture record points at the serve port, so the
health card does not paint a Repair banner over every shot — reseed with
`--port <other>` if you serve elsewhere.

Fixture contents, mapped to the surfaces the gallery needs:
- sessions as `Record` jsonl: a conversation (tasks -> the board), an
  active room (mention/deliverable/recommend -> huddles), an ended+archived
  room with a breakout child (huddles' ended column), and a capped run
  (`max-turns` end). Prose is one paragraph per LINE — hard-wrapped text
  renders as broken paragraphs in the transcript.
- memories with frontmatter, six canonical skills copied, `roles/*.md`,
  `canvas/<slug>/project.json` boards, `desk/<slug>/project.json` briefs,
  - calibration rows under `logs/<Y>/<M>/<D>/events.jsonl` — route, turn,
    tool-screen and recall rows, the route-judged verdicts, and the
    delegation rows (op `delegation`, each carrying its `node`) the P4
    overlay joins the declared graph against — and
    `graph.json` in the GraphFile shape (status pending|running|verified|
    failed; nodes carry tools/skills/workflows/loops and peak).
  - canvas versions on the landing frame, each naming its own captured
    raster under `canvas/<slug>/versions/` (the P5 version-diff fixture).
  - a vault with three of the five placeholder keys sealed through the
    CLI's own writer (`fleet vault set`, under a fixture passphrase) and
    two left as plaintext `.env` lines, so the P6 unlock map has rows and
    the migration card still names a line to delete.
  - one installed untrusted bundle (`plugins/galley`) with a manifest, a
    registry row and an ask ledger carrying answered, refused and
    undeclared asks — the three honest shapes the P6 manifest diff draws.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone

DEFAULT_HOME = "/tmp/fleet-gallery-home"
DEFAULT_PROJECT = "/tmp/fleet-gallery-project"
DEFAULT_PORT = 3099
OWNER_HOME = os.path.realpath(os.path.expanduser("~/.fleet"))
SKILLS_LIBRARY = os.path.expanduser("~/.agents/skills")
CANONICAL_SKILLS = [
    "browser-automation",
    "browser-behavior-verification",
    "api-performance-baseline",
    "codebase-inspection",
    "review-changes",
    "severity-labeling",
]
WORKSPACE_PROJECT = "/Users/alejandrodelvillar/Projects/harbor-site"

REPO_MARKERS = ("ports.md",)


def utc_at(seconds_ago: float) -> str:
    stamp = datetime.now(timezone.utc) - timedelta(seconds=seconds_ago)
    return stamp.strftime("%Y-%m-%dT%H:%M:%SZ")


def compact_at(seconds_ago: float, serial: int) -> str:
    """A session id in the log's own shape: compact stamp + short serial."""
    stamp = datetime.now(timezone.utc) - timedelta(seconds=seconds_ago)
    return f"{stamp.strftime('%Y%m%dT%H%M%SZ')}-{serial:06x}"


def day_dir(home: str, seconds_ago: float) -> str:
    stamp = datetime.now(timezone.utc) - timedelta(seconds=seconds_ago)
    return os.path.join(
        home, "logs", f"{stamp.year:04d}", f"{stamp.month:02d}", f"{stamp.day:02d}"
    )


def write(path: str, text: str) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def write_bytes(path: str, payload: bytes) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as handle:
        handle.write(payload)


def png_bytes(width: int, height: int, rgb: tuple) -> bytes:
    """A minimal solid-color PNG, stdlib only — the fixture versions'
    rasters. Real pixels, small ones: the versions card's compare paints
    them side by side, so they only have to read as two distinct frames."""
    import struct
    import zlib

    row = b"\x00" + bytes(rgb) * width
    raw = row * height

    def chunk(kind: bytes, data: bytes) -> bytes:
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF))

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def write_json(path: str, payload) -> None:
    write(path, json.dumps(payload, indent=2) + "\n")


def jsonl(path: str, records: list) -> None:
    write(path, "".join(json.dumps(record, ensure_ascii=False) + "\n" for record in records))


# --- sessions ---------------------------------------------------------------

def conversation_records(created: str, ended: str):
    """The owner's conversation: a task list (the board), a plan, an end."""
    return [
        {"type": "start", "v": 2, "model": "stepfun/step-3.7-flash",
         "created": created, "workspace": WORKSPACE_PROJECT},
        {"type": "user", "content":
            "Pull the launch board together for harbor-site: inventory what shipped, "
            "the staging checklist, and the copy pass. Friday is the review."},
        {"type": "assistant", "content":
            "The launch work splits into four legs that can run in order: the shipped "
            "inventory first, the staging checklist second, the copy pass once the "
            "checklist holds, and the release notes written from what the first two "
            "landed. I will keep the board current as each leg moves."},
        {"type": "tasks", "items": [
            {"step": "inventory what shipped against the launch brief", "status": "completed"},
            {"step": "the staging checklist, box by box", "status": "in_progress"},
            {"step": "the copy pass over the landing and the docs", "status": "in_progress"},
            {"step": "release notes from the shipped inventory", "status": "pending"},
            {"step": "the launch review card for Friday", "status": "pending"},
        ]},
        {"type": "assistant", "content":
            "The inventory leg is done — eight shipped items, two cut. The staging "
            "checklist has the relay and the pairing legs left; the copy pass is "
            "running against the landing page and will hand you a marked-up draft "
            "when it lands."},
        {"type": "usage", "turn": 2, "prompt_tokens": 18244, "completion_tokens": 1203},
        {"type": "end", "stop_reason": "completed", "turns": 2,
         "served_model": "step-3.7-flash"},
    ]


def room_records(created: str, room_id: str):
    """The active room: a mention, a deliverable, a capability gap."""
    return [
        {"type": "start", "v": 2, "model": "GLM-5.3-Flash", "created": created,
         "personas": ["scout", "keeper"]},
        {"type": "join", "persona": "scout",
         "role": "reads the work and names what it lacks", "at": created},
        {"type": "join", "persona": "keeper",
         "role": "keeps the brief honest against its sources", "at": created},
        {"type": "user", "content":
            "@scout review the staging checklist before Friday; flag anything that "
            "would block the copy pass."},
        {"type": "mention", "member": "scout", "kind": "review", "at": created},
        {"type": "assistant", "persona": "scout", "content":
            "The checklist holds on seven of nine legs. The relay leg is thin: it "
            "asserts the port is open but never probes it after the reload, which is "
            "the one step that has bitten us twice."},
        {"type": "assistant", "persona": "scout", "content":
            "@you should the relay probe land before the copy pass starts? The pass "
            "will quote the checklist as it stands, and a fixed probe changes two of "
            "its lines."},
        {"type": "mention", "member": "you", "kind": "ask", "from": "scout", "at": utc_at(1500)},
        {"type": "assistant", "persona": "keeper", "content":
            "Either way the brief stands: the claim is that the staging checklist is "
            "complete and the copy pass is done, and the probe only widens the "
            "evidence under the first half."},
        {"type": "recommend", "name": "a load probe for the staging box",
         "kind": "capability",
         "reason": "the relay leg needs a probe after reload and no member holds one",
         "from": "keeper", "at": utc_at(1400)},
        {"type": "assistant", "persona": "keeper", "content":
            "The marked-up copy is ready — handed to the shelf so you can read it "
            "beside the checklist."},
        {"type": "tool_result", "call_id": "c1", "name": "send_file",
         "output": "handed `launch-brief.md` to the owner (4821 bytes); the face shows it as a card",
         "truncated": False, "approved": True},
    ]


def archived_room_records(created: str, ended: str):
    """The ended room the owner put away."""
    return [
        {"type": "start", "v": 2, "model": "GLM-5.3-Flash", "created": created,
         "personas": ["archivist"]},
        {"type": "join", "persona": "archivist",
         "role": "distils what a walkthrough taught", "at": created},
        {"type": "user", "content":
            "@archivist distil what the staging walkthrough taught us, one page."},
        {"type": "mention", "member": "archivist", "kind": "directive", "at": created},
        {"type": "assistant", "persona": "archivist", "content":
            "The walkthrough taught one thing worth keeping: every leg that ends in a "
            "reload needs a probe after the reload, not before it. The page is on the "
            "desk under staging-walkthrough."},
        {"type": "end", "stop_reason": "completed", "turns": 1,
         "served_model": "GLM-5.3-Flash"},
    ]


def breakout_records(created: str, parent_id: str, ended: str):
    """The room that split away from the archived one."""
    return [
        {"type": "start", "v": 2, "model": "GLM-5.3-Flash", "created": created,
         "parent": parent_id, "personas": ["scout"]},
        {"type": "join", "persona": "scout",
         "role": "reads the work and names what it lacks", "at": created},
        {"type": "assistant", "persona": "scout", "content":
            "The probe work split here so the distillation page could end on time: "
            "the probe spec is three lines and it belongs with the relay leg, not "
            "with the archive."},
        {"type": "end", "stop_reason": "completed", "turns": 1,
         "served_model": "GLM-5.3-Flash"},
    ]


def capped_records(created: str, ended: str):
    """A run the turn bound stopped mid-work."""
    return [
        {"type": "start", "v": 2, "model": "stepfun/step-3.5-flash",
         "created": created, "workspace": "/Users/alejandrodelvillar/Projects/fleet"},
        {"type": "user", "content":
            "Sweep the repo for stray fixture paths under scripts/ and name the "
            "owners of each."},
        {"type": "assistant", "content":
            "The sweep walked the scripts directory and named the owners; the count "
            "pass is the leg the bound stopped. Nine of fourteen files read, the "
            "tasks list carries the rest."},
        {"type": "tasks", "items": [
            {"step": "read scripts/ releases and guards", "status": "completed"},
            {"step": "read scripts/ measures and probes", "status": "pending"},
            {"step": "name the owners of each stray path", "status": "pending"},
        ]},
        {"type": "end", "stop_reason": "max-turns", "turns": 8},
    ]


def seed_sessions(home: str, manifest: dict) -> None:
    sessions_dir = os.path.join(home, "sessions")
    capped_id = compact_at(52 * 3600, 0x11)
    archived_id = compact_at(48 * 3600, 0x12)
    breakout_id = compact_at(47 * 3600, 0x13)
    room_id = compact_at(1800, 0x14)
    conversation_id = compact_at(600, 0x15)

    jsonl(os.path.join(sessions_dir, f"{capped_id}.jsonl"),
          capped_records(utc_at(52 * 3600), utc_at(51.9 * 3600)))
    jsonl(os.path.join(sessions_dir, f"{archived_id}.jsonl"),
          archived_room_records(utc_at(48 * 3600), utc_at(47.8 * 3600)))
    jsonl(os.path.join(sessions_dir, f"{breakout_id}.jsonl"),
          breakout_records(utc_at(47 * 3600), archived_id, utc_at(46.8 * 3600)))
    jsonl(os.path.join(sessions_dir, f"{room_id}.jsonl"),
          room_records(utc_at(1800), room_id))
    jsonl(os.path.join(sessions_dir, f"{conversation_id}.jsonl"),
          conversation_records(utc_at(600), utc_at(60)))

    # The owner's put-away rooms, and the standing goal of the conversation.
    write_json(os.path.join(home, "sessions-archived.json"),
               {archived_id: utc_at(46 * 3600)})
    write_json(os.path.join(home, "sessions-goals.json"),
               {conversation_id: "the harbor-site launch board"})
    write_json(os.path.join(home, "sessions-titles.json"),
               {conversation_id: "the harbor-site launch board"})

    manifest["sessions"] = {
        "conversation": conversation_id,
        "active_room": room_id,
        "archived_room": archived_id,
        "breakout": breakout_id,
        "capped": capped_id,
    }


# --- memories, roles, skills -------------------------------------------------

def fact(name: str, description: str, ftype: str, basis: str, tier: str,
         shape: list, body: str) -> str:
    return (
        "---\n"
        f"name: {name}\n"
        f"description: \"{description}\"\n"
        f"type: {ftype}\n"
        f"basis: {basis}\n"
        f"tier: {tier}\n"
        f"shape: {json.dumps(shape)}\n"
        "phrases: []\n"
        "missing: []\n"
        "recall-count: 0\n"
        "---\n\n"
        f"{body}\n"
    )


MEMORIES = [
    fact("gallery-relay-leg-needs-a-post-reload-probe",
         "the staging relay leg asserts an open port but never probes after the reload",
         "project", "confirmed-by", "durable", ["build", "verify"],
         "The walkthrough found the relay leg's probe sits before the reload, so a "
         "port that closes on the reload reads as open. The probe belongs after the "
         "reload, beside the health check that already runs there."),
    fact("gallery-capability-gap-is-a-hiring-decision",
         "a room's recommend names what to hire, not what to blame",
         "pattern", "confirmed-by", "durable", ["process"],
         "When a room addresses a capability no member holds, the address is a "
         "hiring decision: convene the role, author the persona, or leave the gap "
         "named. Blaming the members answers nothing."),
    fact("gallery-judged-routes-beat-one-lane",
         "the workload judgment picks the chain; the walk names who served",
         "project", "inferred", "durable", ["routing"],
         "A route row carries the judged workload, the chain it walked, and the "
         "candidate that served. Reading the served model without the judgment "
         "confuses the default with the decision."),
    fact("gallery-desk-claims-read-their-sentences",
         "a desk claim stands only on the sentence it was read from",
         "project", "confirmed-by", "working", ["docs"],
         "The extraction floors hand a claim its source sentence; the review "
         "renders the sentence beside the claim, and a refusal names the nearest "
         "sentence it fell short of."),
    fact("gallery-boards-are-project-scoped",
         "a board scopes by project so many projects stay browsable",
         "project", "confirmed-by", "working", ["build"],
         "The rail carries an all-tasks row and one row per project the work "
         "belongs to; a board with many projects is browsable without every task "
         "being one undifferentiated column set."),
    fact("gallery-rooms-end-by-the-owner",
         "ending a room is the owner's act, not the run's",
         "pattern", "inferred", "working", ["process"],
         "A room that ran and stopped sits parked until the owner archives it; the "
         "archived set is a map the owner's act writes, and the ended column reads "
         "it, never the stop reason alone."),
]

ROLES = {
    "scout": (
        "reads the work and names what it lacks",
        "read_file, search_files, web_extract",
        "codebase-inspection, severity-labeling",
        "build, verify",
        "standard_code",
        "You read the artefacts in front of the room and say plainly what they "
        "lack. You never fix what you read; you name the gap, point at the line, "
        "and hand the next step to whoever holds it.\n",
    ),
    "keeper": (
        "keeps the brief honest against its sources",
        "read_file, web_extract",
        "api-performance-baseline",
        "docs",
        "prose",
        "You hold the deliverable's claim against its evidence. When a sentence "
        "outruns its sources you refuse it by name and say which sentence came "
        "close.\n",
    ),
    "archivist": (
        "distils what a walkthrough taught",
        "read_file, search_files",
        "codebase-inspection",
        "process",
        "deep_context",
        "You turn a finished walkthrough into the one page worth keeping: the "
        "lesson, where it bit, and the rule that prevents the next bite.\n",
    ),
}

CANVAS_FRAMES = [
    {
        "id": "frame-landing",
        "name": "the landing",
        "description": "the launch landing's first frame",
        "html": ("<!doctype html><html><head><meta charset=\"utf-8\">"
                 "<style>body{font-family:Georgia,serif;margin:0;padding:48px;"
                 "background:#101418;color:#e8e4da}h1{font-size:34px;margin:0 0 12px}"
                 "p{max-width:52ch;line-height:1.55;color:#b9b2a4}</style></head>"
                 "<body><h1>harbor-site</h1><p>The launch landing: one question over "
                 "the composer, the rail's nine lenses beside it, and the copy pass "
                 "walking the words underneath.</p></body></html>"),
        "x": 0, "y": 0, "w": 1280, "h": 900, "tweak_spec": [],
        "tweak_values": {}, "model": "import",
        "set": "site",
        # Two versions on the landing, each with its own captured raster
        # (the P5 version-diff fixture): the versions card's compare
        # paints the two newest side by side, and a version carries
        # pixels only when the record names them (`image`).
        "versions": [
            {"id": "ver-landing-1", "name": "the first pass",
             "html": "", "tweak_values": {}, "source": "generate",
             "created": utc_at(8 * 3600), "image": "ver-landing-1.png"},
            {"id": "ver-landing-2", "name": "the copy-pass mark",
             "html": "", "tweak_values": {}, "source": "refine",
             "created": utc_at(3 * 3600), "image": "ver-landing-2.png"},
        ],
        "pins": [
            {"id": "pin-1", "x": 0.5, "y": 0.62,
             "text": "the composer's placeholder carries the one question",
             "author": "owner", "source": "owner", "created": utc_at(8 * 3600),
             "replies": []},
        ],
        "created": utc_at(9 * 3600), "updated": utc_at(6 * 3600),
    },
    {
        "id": "frame-docs",
        "name": "the docs page",
        "description": "the docs page of the same site",
        "html": ("<!doctype html><html><head><meta charset=\"utf-8\">"
                 "<style>body{font-family:Georgia,serif;margin:0;padding:48px;"
                 "background:#f4f1ea;color:#2a2724}h1{font-size:30px;margin:0 0 10px}"
                 "p{max-width:56ch;line-height:1.6}</style></head>"
                 "<body><h1>the docs</h1><p>The docs page carries the walkthrough "
                 "the archivist distilled; the copy pass marks the two lines the "
                 "relay probe changes.</p></body></html>"),
        "x": 1340, "y": 0, "w": 1280, "h": 900, "tweak_spec": [],
        "tweak_values": {}, "model": "import",
        "set": "site",
        "versions": [], "pins": [],
        "created": utc_at(9 * 3600), "updated": utc_at(7 * 3600),
    },
    {
        "id": "frame-scratch",
        "name": "the scratch frame",
        "description": "an unbound scratch frame",
        "html": ("<!doctype html><html><head><meta charset=\"utf-8\">"
                 "<style>body{margin:0;padding:40px;background:#ffffff;color:#222}"
                 "h1{font-size:26px}</style></head><body><h1>scratch</h1></body></html>"),
        "x": 0, "y": 0, "w": 900, "h": 640, "tweak_spec": [],
        "tweak_values": {}, "model": "import",
        "versions": [], "pins": [],
        "created": utc_at(30 * 3600), "updated": utc_at(30 * 3600),
    },
]


def seed_knowledge(home: str, manifest: dict) -> None:
    for text in MEMORIES:
        name = text.split("name: ", 1)[1].split("\n", 1)[0]
        write(os.path.join(home, "memories", f"{name}.md"), text)

    for name, (description, tools, skills, shapes, workload, overlay) in ROLES.items():
        write(os.path.join(home, "roles", f"{name}.md"), (
            "---\n"
            f"description: \"{description}\"\n"
            f"tools: {tools}\n"
            f"skills: {skills}\n"
            f"shapes: {shapes}\n"
            f"workload: {workload}\n"
            "---\n"
            f"{overlay}\n"
        ))

    # Six canonical skills, copied from the public library — the only thing
    # the seed reads from the owner's machine.
    for skill in CANONICAL_SKILLS:
        source = os.path.join(SKILLS_LIBRARY, skill)
        if not os.path.isfile(os.path.join(source, "SKILL.md")):
            sys.exit(f"REFUSED: the public library has no `{skill}` under {SKILLS_LIBRARY}")
        target = os.path.join(home, "skills", skill)
        shutil.copytree(source, target, dirs_exist_ok=True)

    conversation_id = manifest["sessions"]["conversation"]
    now_iso = utc_at(0)
    bound_frames = [dict(frame) for frame in CANVAS_FRAMES if frame.get("set") == "site"]
    unbound_frames = [dict(frame) for frame in CANVAS_FRAMES if not frame.get("set")]
    write_json(os.path.join(home, "canvas", "harbor-site-launch", "project.json"), {
        "version": 1,
        "name": "harbor-site launch",
        "created": utc_at(9 * 3600),
        "updated": utc_at(6 * 3600),
        "frames": bound_frames,
        "chat": [
            {"role": "owner", "text": "the landing first, docs second; keep one "
                                      "token system across the set",
             "frames": [], "at": utc_at(9 * 3600)},
            {"role": "designer", "text": "both frames landed in the site set; the "
                                         "copy pass marks the relay lines",
             "frames": ["frame-landing", "frame-docs"], "at": utc_at(7 * 3600)},
        ],
        "brand": None,
        "project": WORKSPACE_PROJECT,
    })
    write_json(os.path.join(home, "canvas", "scratch-board", "project.json"), {
        "version": 1,
        "name": "scratch board",
        "created": utc_at(30 * 3600),
        "updated": utc_at(30 * 3600),
        "frames": unbound_frames,
        "chat": [],
        "brand": None,
    })

    # The versions' own rasters (the P5 version-diff fixture): the PNGs
    # the browser lane would have captured, one per version the record
    # names, at `canvas/<slug>/versions/<version-id>.png` — the path
    # `canvas::version_image` serves the compare panes from.
    versions_root = os.path.join(home, "canvas", "harbor-site-launch", "versions")
    write_bytes(os.path.join(versions_root, "ver-landing-1.png"),
                png_bytes(64, 40, (16, 20, 24)))
    write_bytes(os.path.join(versions_root, "ver-landing-2.png"),
                png_bytes(64, 40, (233, 224, 210)))

    # The desk: two briefs, one walked to review, one just claims.
    write_json(os.path.join(home, "desk", "launch-brief", "project.json"), {
        "slug": "launch-brief",
        "name": "the harbor-site launch brief",
        "audience": "the owner, before Friday's review",
        "claim": "the staging checklist is complete and the copy pass is done",
        "sources": [{"type": "file",
                     "path": "/Users/alejandrodelvillar/Projects/harbor-site/docs/staging.md"}],
        "shape": "docs",
        "session": conversation_id,
        "project": WORKSPACE_PROJECT,
        "stages": [["claims", {"state": "done"}], ["outline", {"state": "done"}],
                   ["render", {"state": "done"}], ["review", {"state": "running"}],
                   ["knowledge", {"state": "pending"}]],
        "claims": [
            {"text": "the staging checklist covers nine legs",
             "source": "/Users/alejandrodelvillar/Projects/harbor-site/docs/staging.md",
             "sentence": "The checklist covers nine legs from relay to copy.",
             "confidence": 0.93, "kind": "claim", "verdict": "asserted"},
            {"text": "the relay leg needs a probe after the reload",
             "source": "/Users/alejandrodelvillar/Projects/harbor-site/docs/staging.md",
             "sentence": "The relay probe runs after the reload, never before.",
             "confidence": 0.91, "kind": "claim", "verdict": "asserted"},
            {"text": "the copy pass quotes the checklist as it stands",
             "source": "/Users/alejandrodelvillar/Projects/harbor-site/docs/staging.md",
             "sentence": "The copy pass reads the checklist, not the code.",
             "confidence": 0.54, "kind": "claim", "verdict": "contested"},
        ],
        "refusals": [],
        "figures": [],
        "draft": "# The launch brief\n\nThe staging checklist covers nine legs and "
                 "the copy pass walks the two the relay probe changes.\n",
        "number_refusals": [],
        "created": utc_at(10 * 3600),
        "updated": utc_at(2 * 3600),
    })
    write_json(os.path.join(home, "desk", "staging-walkthrough", "project.json"), {
        "slug": "staging-walkthrough",
        "name": "the staging walkthrough page",
        "audience": "the fleet project's own corpus",
        "claim": "every leg that ends in a reload probes after the reload",
        "sources": [{"type": "file",
                     "path": "/Users/alejandrodelvillar/Projects/fleet/docs/staging-walkthrough.md"}],
        "shape": "process",
        "session": None,
        "project": None,
        "stages": [["claims", {"state": "done"}], ["outline", {"state": "done"}],
                   ["render", {"state": "done"}], ["review", {"state": "done"}],
                   ["knowledge", {"state": "done"}]],
        "claims": [
            {"text": "the probe belongs after the reload",
             "source": "/Users/alejandrodelvillar/Projects/fleet/docs/staging-walkthrough.md",
             "sentence": "The walkthrough taught: probe after the reload, not before.",
             "confidence": 0.9, "kind": "claim", "verdict": "asserted"},
        ],
        "refusals": [],
        "figures": [],
        "created": utc_at(47 * 3600),
        "updated": utc_at(46 * 3600),
    })


# --- calibration rows and the graph -----------------------------------------

def route_row(ts: str, provider: str, workload: str, model: str,
              how: str = "judged", failed: bool = False) -> dict:
    providers_used = [provider] if not failed else ["stepfun", provider]
    attempts = [{"provider": provider, "outcome": "served", "class": None}] \
        if not failed else \
        [{"provider": "stepfun", "outcome": "failed", "class": "server-5xx"},
         {"provider": provider, "outcome": "served", "class": None}]
    return {
        "op": "route", "ts": ts, "provider": provider, "how": how,
        "workload": workload, "verdict": workload,
        "reason": f"the {workload} judgment named {provider} first",
        "model": model, "providers_used": providers_used, "attempts": attempts,
        "input_chars": 512, "input_hash": "b3f1a9c4d2e7",
        "latency_ms": 843 if not failed else 2120, "judged_from": "fresh",
    }


def delegation_row(ts: str, node: str, session: str, brief_shape: str,
                   provider: str, outcome: str) -> dict:
    """The row the P4 delegation overlay joins on: the run's own record
    that this graph node's work was delegated to this child session. The
    overlay's join key is the row's `node` (`DelegatedRunRow.node`); a
    row without it joins nothing, which is why every row here carries it."""
    return {
        "op": "delegation", "ts": ts, "node": node, "session": session,
        "brief_shape": brief_shape, "provider": provider, "outcome": outcome,
    }


def turn_row(ts: str, outcome: str, stop: str, turns: int) -> dict:
    return {
        "op": "turn", "ts": ts, "outcome": outcome, "stop_reason": stop,
        "turns": turns,
        "usage": {"prompt_tokens": 18244, "completion_tokens": 1203},
        "input_chars": 512, "input_hash": "b3f1a9c4d2e7",
    }


def seed_calibration(home: str) -> None:
    # Today's rows (the surfaces read the day) and yesterday's, so the
    # calibration views hold more than one day of traffic.
    today = day_dir(home, 0)
    yesterday = day_dir(home, 26 * 3600)
    today_rows = [
        route_row(utc_at(5 * 3600), "stepfun", "standard_code", "step-3.7-flash"),
        route_row(utc_at(4.5 * 3600), "zai-coding-plan", "quick", "GLM-5.3-Flash"),
        route_row(utc_at(4 * 3600), "stepfun", "hard", "step-5-preview"),
        route_row(utc_at(3.5 * 3600), "deepseek", "frontend", "deepseek-flash"),
        route_row(utc_at(3 * 3600), "stepfun", "prose", "step-3.7-flash"),
        route_row(utc_at(2.5 * 3600), "zai-coding-plan", "standard_code",
                  "GLM-5.3-Flash", how="fallback", failed=True),
        turn_row(utc_at(5 * 3600), "ok", "completed", 2),
        turn_row(utc_at(2.5 * 3600), "ok", "completed", 3),
        {"op": "tool_screen", "ts": utc_at(5 * 3600), "tool": "read_file",
         "verdict": "approved", "input_chars": 96, "input_hash": "1a2b3c",
         "tier": "read"},
        {"op": "tool_screen", "ts": utc_at(4.9 * 3600), "tool": "edit_file",
         "verdict": "approved", "input_chars": 640, "input_hash": "2b3c4d",
         "tier": "standard"},
        {"op": "tool_screen", "ts": utc_at(3.4 * 3600), "tool": "bash",
         "verdict": "denied", "input_chars": 210, "input_hash": "3c4d5e",
         "tier": "destructive"},
        {"op": "recall_hit", "ts": utc_at(5 * 3600),
         "fact": "gallery-judged-routes-beat-one-lane", "shape": "routing",
         "score": 2.41, "vector": 1.12, "keyword": 0.61, "trajectory": 0.22,
         "history": 0.18, "freshness": 0.16, "provenance": 0.12,
         "off_shape": False},
        {"op": "recall_hit", "ts": utc_at(4.9 * 3600),
         "fact": "gallery-relay-leg-needs-a-post-reload-probe", "shape": "build",
         "score": 2.18, "vector": 0.98, "keyword": 0.55, "trajectory": 0.19,
         "history": 0.21, "freshness": 0.15, "provenance": 0.10,
         "off_shape": False},
        {"op": "recall_hit", "ts": utc_at(2.5 * 3600),
         "fact": "gallery-boards-are-project-scoped", "shape": "build",
         "score": 1.86, "vector": 0.81, "keyword": 0.44, "trajectory": 0.16,
         "history": 0.14, "freshness": 0.19, "provenance": 0.12,
         "off_shape": True},
        {"op": "route_judged", "ts": utc_at(5 * 3600),
         "route_key": "b3f1a9c4d2e7", "provider": "stepfun",
         "workload": "standard_code", "verdict": "right", "confidence": 0.88,
         "reason": "the standard_code ask served first-try and the diff landed"},
        {"op": "route_judged", "ts": utc_at(4.5 * 3600),
         "route_key": "c4d5e6f7a8b9", "provider": "zai-coding-plan",
         "workload": "quick", "verdict": "right", "confidence": 0.82,
         "reason": "the quick ask answered under a second"},
        {"op": "route_judged", "ts": utc_at(3.5 * 3600),
         "route_key": "d5e6f7a8b9c0", "provider": "deepseek",
         "workload": "frontend", "verdict": "wrong", "confidence": 0.71,
         "reason": "the frontend ask stalled for want of the companion pass"},
        {"op": "route_judged", "ts": utc_at(2.5 * 3600),
         "route_key": "e6f7a8b9c0d1", "provider": "zai-coding-plan",
         "workload": "standard_code", "verdict": "insufficient",
         "confidence": 0.66, "reason": "the fallback walk served but the run capped"},
        # The delegation overlay's rows (the P4 join, `node` carried): one
        # child on `shipped-inventory` (the declared shape — no caption),
        # two on `staging-checklist` (`ran twice`; the LATEST row's judged
        # shape is the one the caption names), none on the rest (`never
        # spawned`). Session ids in the log's own compact shape.
        delegation_row(utc_at(6.8 * 3600), "shipped-inventory",
                       compact_at(6.8 * 3600, 0x21), "standard_code",
                       "stepfun", "completed"),
        delegation_row(utc_at(5.2 * 3600), "staging-checklist",
                       compact_at(5.2 * 3600, 0x22), "standard_code",
                       "stepfun", "completed"),
        delegation_row(utc_at(3.2 * 3600), "staging-checklist",
                       compact_at(3.2 * 3600, 0x23), "prose",
                       "zai-coding-plan", "max-turns"),
    ]
    yesterday_rows = [
        route_row(utc_at(30 * 3600), "stepfun", "standard_code", "step-3.7-flash"),
        route_row(utc_at(29 * 3600), "xiaomi-mimo", "quick", "mimo-v2.6-flash"),
        turn_row(utc_at(30 * 3600), "ok", "completed", 1),
        turn_row(utc_at(29 * 3600), "failed", "max-turns", 8),
    ]
    jsonl(os.path.join(today, "events.jsonl"), today_rows)
    jsonl(os.path.join(yesterday, "events.jsonl"), yesterday_rows)


def seed_graph(home: str) -> None:
    def node(node_id, cost, status, peak, prereqs=(), paths=(), skills=()):
        return {
            "id": node_id, "cost": cost, "prereqs": list(prereqs),
            "paths": list(paths),
            "capabilities": {
                "tools": ["read_file", "edit_file", "bash"],
                "skills": list(skills),
                "workflows": ["verify-diff"] if status != "pending" else [],
                "loops": ["the judge batch"] if node_id == "staging-checklist" else [],
            },
            "approach": f"walk {node_id} against the launch brief's own words",
            "assignee": "scout" if status == "running" else "",
            "status": status, "dispatches": 1 if status == "failed" else 0,
            "peak": peak,
        }

    write_json(os.path.join(home, "graph.json"), {
        "nodes": [
            node("shipped-inventory", 3, "verified", 3, paths=["docs/inventory.md"],
                 skills=("codebase-inspection",)),
            node("staging-checklist", 5, "running", 5, prereqs=["shipped-inventory"],
                 paths=["docs/staging.md"], skills=("severity-labeling",)),
            node("copy-pass", 4, "pending", 0, prereqs=["staging-checklist"],
                 paths=["README.md", "docs/landing.md"], skills=("review-changes",)),
            node("release-notes", 2, "failed", 2, prereqs=["shipped-inventory"],
                 paths=["docs/release-notes.md"], skills=("severity-labeling",)),
            node("launch-review", 2, "pending", 0,
                 prereqs=["copy-pass", "release-notes"], skills=("review-changes",)),
        ],
        "revisions": [],
    })


# --- the scratch project (the board's plan) ----------------------------------

# The board reads `<cwd>/docs/plans/*.md` off the face's working directory:
# a plan card's existence IS its provenance, so the gallery serves the face
# from a scratch project carrying one approved plan, never from the fleet
# repo (whose docs/plans are the other writers' ground).
PLAN = """---
status: active
created: {created}
updated: {updated}
slug: harbor-launch
execution: mixture
---

# The harbor-site launch

The launch work as the board executes it: an approved plan whose graph the
face derives into the board's three columns.

```graph
{graph}
```
"""

PLAN_GRAPH = {
    "nodes": [
        {"id": "shipped-inventory", "cost": 3, "prereqs": [],
         "paths": ["docs/inventory.md"],
         "capabilities": {"tools": ["read_file", "search_files"], "skills": ["codebase-inspection"],
                          "workflows": [], "loops": []},
         "approach": "walk the shipped list against the launch brief",
         "serves": "C1", "assignee": ""},
        {"id": "staging-checklist", "cost": 5, "prereqs": ["shipped-inventory"],
         "paths": ["docs/staging.md"],
         "capabilities": {"tools": ["read_file", "edit_file"], "skills": ["severity-labeling"],
                          "workflows": [], "loops": []},
         "approach": "one leg per box, the reload probes after the reload",
         "serves": "C2", "assignee": "scout"},
        {"id": "copy-pass", "cost": 4, "prereqs": ["staging-checklist"],
         "paths": ["README.md", "docs/landing.md"],
         "capabilities": {"tools": ["edit_file"], "skills": ["review-changes"],
                          "workflows": [], "loops": []},
         "approach": "mark the two lines the relay probe changes",
         "serves": "C3", "assignee": "keeper"},
        {"id": "release-notes", "cost": 2, "prereqs": ["shipped-inventory"],
         "paths": ["docs/release-notes.md"],
         "capabilities": {"tools": ["edit_file"], "skills": ["severity-labeling"],
                          "workflows": [], "loops": []},
         "approach": "write from the shipped inventory, never from memory",
         "serves": "C4", "assignee": ""},
        {"id": "launch-review", "cost": 2, "prereqs": ["copy-pass", "release-notes"],
         "paths": [],
         "capabilities": {"tools": [], "skills": ["review-changes"], "workflows": [], "loops": []},
         "approach": "the Friday card reads the brief against the four legs",
         "serves": "C5", "assignee": ""},
    ],
}


def seed_project(project: str, manifest: dict) -> None:
    """One approved plan + its runtime graph, so the board's three columns
    (done / in-process / queued) all have cards."""
    plans = os.path.join(project, "docs", "plans")
    os.makedirs(plans, exist_ok=True)
    created = utc_at(9 * 3600)
    updated = utc_at(2 * 3600)
    graph = json.dumps(PLAN_GRAPH, indent=2)
    write(os.path.join(plans, "harbor-launch.md"),
          PLAN.format(created=created, updated=updated, graph=graph))
    # The runtime file beside the plan, in the PlanGraphFile shape: the
    # statuses are the board's columns (done = verified, in-process =
    # running, queued = pending|failed), and the digest joins the plan.
    plan_path = os.path.join(plans, "harbor-launch.md")
    block = declaration_block(plan_path)
    digest = hashlib.sha256(block.encode("utf-8")).hexdigest()[:16]

    def node(node_id, cost, status, peak, prereqs, paths, tools, skills, approach,
             serves, assignee):
        return {
            "id": node_id, "cost": cost, "prereqs": prereqs, "paths": paths,
            "capabilities": {"tools": tools, "skills": skills, "workflows": [], "loops": []},
            "approach": approach, "assignee": assignee,
            "status": status, "dispatches": 1 if status == "failed" else 0,
            "peak": peak,
        }

    write_json(os.path.join(plans, "harbor-launch.graph.json"), {
        "slug": "harbor-launch",
        "declaration": digest,
        "serves": {},
        # PlanGraphFile carries the GraphFile FLATTENED: nodes and revisions
        # sit beside slug/declaration/serves, never under a "graph" key.
        "nodes": [
                node("shipped-inventory", 3, "verified", 3, [], ["docs/inventory.md"],
                     ["read_file", "search_files"], ["codebase-inspection"],
                     "walk the shipped list against the launch brief", "C1", ""),
                node("staging-checklist", 5, "running", 5, ["shipped-inventory"],
                     ["docs/staging.md"], ["read_file", "edit_file"], ["severity-labeling"],
                     "one leg per box, the reload probes after the reload", "C2", "scout"),
                node("copy-pass", 4, "pending", 0, ["staging-checklist"],
                     ["README.md", "docs/landing.md"], ["edit_file"], ["review-changes"],
                     "mark the two lines the relay probe changes", "C3", "keeper"),
                node("release-notes", 2, "failed", 2, ["shipped-inventory"],
                     ["docs/release-notes.md"], ["edit_file"], ["severity-labeling"],
                     "write from the shipped inventory, never from memory", "C4", ""),
                node("launch-review", 2, "pending", 0, ["copy-pass", "release-notes"],
                     [], [], ["review-changes"],
                     "the Friday card reads the brief against the four legs", "C5", ""),
        ],
        "revisions": [],
    })
    manifest["project"] = os.path.abspath(project)


def declaration_block(plan_path: str) -> str:
    """The ```graph fence's body, the way plan_graph::declaration_block reads
    it — the seed hashes the file's own bytes, never its intentions."""
    lines = open(plan_path, encoding="utf-8").read().splitlines()
    inside = False
    body: list = []
    for line in lines:
        trimmed = line.strip()
        if not inside:
            if trimmed == "```graph":
                inside = True
        elif trimmed == "```":
            break
        else:
            body.append(line)
    if not inside:
        sys.exit(f"REFUSED: {plan_path} carries no ```graph block")
    return "\n".join(body)


# --- the rest of the home ----------------------------------------------------

def seed_env_and_config(home: str) -> None:
    # Placeholder credentials, availability only — a screenshot run makes no
    # provider calls, and the value is never sent anywhere.
    write(os.path.join(home, ".env"), "\n".join([
        "# gallery fixture: placeholder keys, availability only",
        "ZAI_CODING_API_KEY=gallery-placeholder-zai",
        "STEPFUN_API_KEY=gallery-placeholder-stepfun",
        "XIAOMI_MIMO_API_KEY=gallery-placeholder-mimo",
        "TOKEN_PLAN_API_KEY=gallery-placeholder-tokenplan",
        "DEEPSEEK_API_KEY=gallery-placeholder-deepseek",
        "",
    ]))
    # The empty config keeps the compiled-in defaults (the routing table,
    # the roster); the gallery shots read the archive, the stream and the
    # stores, not the owner's own config.
    write_json(os.path.join(home, "config.json"), {})


def seed_vault(home: str) -> None:
    """Seal three of the five placeholder keys, so the vault's unlock map
    has rows to draw and the .env migration card still has plaintext left
    to name. Sealing is the CLI's own writer (`fleet vault set`), never a
    hand-written vault file: the file is AES-GCM under the passphrase, and
    a fixture that fabricated one would not be the format the app reads.

    No binary on disk means no vault — named, never silent: the map then
    paints its own empty sentence and the shot still passes readiness
    (the migration card is the populated half).
    """
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    binary = next((one for one in (
        os.path.join(repo, "target", "debug", "fleet"),
        os.path.join(repo, "target", "release", "fleet"),
    ) if os.path.exists(one)), None)
    if not binary:
        print("gallery-seed: no fleet binary under target/ — the vault map "
              "will paint its empty sentence (build first to shoot a sealed map)")
        return
    sealed = [
        ("ZAI_CODING_API_KEY", "gallery-placeholder-zai"),
        ("XIAOMI_MIMO_API_KEY", "gallery-placeholder-mimo"),
        ("STEPFUN_API_KEY", "gallery-placeholder-stepfun"),
    ]
    env = dict(os.environ)
    env["FLEET_HOME"] = home
    env["FLEET_VAULT_PASSPHRASE"] = "gallery-fixture-passphrase"
    for name, value in sealed:
        done = subprocess.run(
            [binary, "vault", "set", name, value],
            env=env, capture_output=True, text=True)
        if done.returncode != 0:
            print(f"gallery-seed: sealing {name} failed: {done.stderr.strip()}")
            return
    print(f"gallery-seed: sealed {len(sealed)} keys into the vault")


# --- a bundle for the plugins surface ---------------------------------------

PLUGIN_NAME = "galley"
PLUGIN_MANIFEST = {
    "name": PLUGIN_NAME,
    "tier": "untrusted",
    "version": "0.3.1",
    "entry": {"group": "system", "label": "the galley board"},
    "queries": ["sessions", "memory", "usage"],
    "provenance": "creator:gallery-fixture",
}
PLUGIN_ASKS = [
    # answered — the ordinary case
    {"at": "2026-11-21T09:12:00Z", "query": "sessions", "answered": True},
    # refused by the bridge, named
    {"at": "2026-11-21T09:14:00Z", "query": "memory", "answered": False,
     "reason": "the memory index is locked for this run"},
    # asked for something the manifest never declared: the sandbox refused it
    {"at": "2026-11-21T09:15:00Z", "query": "preview", "answered": False,
     "reason": "the manifest declares no `preview` query"},
]


def seed_plugins(home: str) -> None:
    """One installed untrusted bundle, its registry row and its ask ledger —
    the three things the manifest diff reads. The ledger carries all three
    of its honest shapes: answered, refused with a reason, and asked for a
    query the manifest never declared (so the diff's undeclared row is a
    fixture, not a hope)."""
    bundle = os.path.join(home, "plugins", PLUGIN_NAME)
    os.makedirs(bundle, exist_ok=True)
    write_json(os.path.join(bundle, "manifest.json"), PLUGIN_MANIFEST)
    jsonl(os.path.join(bundle, "asks.jsonl"), PLUGIN_ASKS)
    write_json(os.path.join(home, "plugins.json"), {
        "plugins": {PLUGIN_NAME: {"enabled": True, "promotions": []}},
    })


def seed_substrate(home: str, port: int) -> None:
    """The health loop's layerfs probe: with no machine record it reads the
    conventional `<home>/layerfs-home/worker/endpoint.json`. The fixture
    records the FACE's OWN port, so the connect answers and the services
    card stays quiet instead of painting a Repair banner over every shot.
    Serve the face on the port this seed was run with (--port), or the
    banner comes back."""
    write_json(os.path.join(home, "layerfs-home", "worker", "endpoint.json"), {
        "port": port,
        "note": "gallery fixture — the face's own port, so the substrate probe answers",
    })


def guard(home: str) -> None:
    target = os.path.realpath(home)
    if target == OWNER_HOME or target.startswith(OWNER_HOME + os.sep):
        sys.exit(f"REFUSED: {home} resolves inside the owner's fleet home ({OWNER_HOME})")
    if target in ("/", "/tmp", os.path.expanduser("~")) or target == os.getcwd():
        sys.exit(f"REFUSED: {home} is not a scratch gallery home")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="write the scratch FLEET_HOME the doc gallery shoots from")
    parser.add_argument("--home", default=DEFAULT_HOME,
                        help=f"the scratch home to write (default {DEFAULT_HOME})")
    parser.add_argument("--project", default=DEFAULT_PROJECT,
                        help="the scratch project the face serves from — the "
                             "board reads <cwd>/docs/plans (default "
                             f"{DEFAULT_PROJECT})")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT,
                        help="the port the face will be served on; the "
                             "substrate probe's fixture record points here "
                             f"(default {DEFAULT_PORT})")
    args = parser.parse_args()
    home = os.path.abspath(args.home)
    project = os.path.abspath(args.project)
    guard(home)
    guard(project)

    if os.path.isdir(home):
        shutil.rmtree(home)
    os.makedirs(home, exist_ok=True)

    manifest = {"home": home, "project": project, "port": args.port,
                "seeded_at": utc_at(0), "sessions": {}}
    seed_env_and_config(home)
    seed_substrate(home, args.port)
    seed_vault(home)
    seed_plugins(home)
    seed_sessions(home, manifest)
    seed_knowledge(home, manifest)
    seed_calibration(home)
    seed_graph(home)
    seed_project(project, manifest)

    # A name and a stamp the shell chrome can name without reading the owner.
    write_json(os.path.join(home, "identity.json"),
               {"name": "gallery", "created": utc_at(0)})
    write_json(os.path.join(home, "gallery-manifest.json"), manifest)

    print(f"gallery home written:     {home}")
    print(f"gallery project written:  {project}")
    print(f"serve it with: cd {project} && FLEET_HOME={home} "
          f"./target/debug/fleet gui --port {args.port} --no-open")
    print(f"active room:              {manifest['sessions']['active_room']}")


if __name__ == "__main__":
    main()
