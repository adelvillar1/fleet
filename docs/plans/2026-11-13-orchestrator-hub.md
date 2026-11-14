---
title: the orchestrator is the hub — persona-to-orchestrator communication, the peer-to-peer dependency signal
date: 2026-11-13
status: completed (2026-11-13; the channel and the gate landed with their tests; the gate green)
---

# The orchestrator is the hub

The owner's ruling: personas with assigned workloads communicate freely
with their orchestrator; same-level personas do not communicate directly —
a need to is an unresolved dependency in the task breakdown, and that
signal is visible.

## Acceptance criteria

- [x] C1: the `message_orchestrator` tool — declared, admitted only for
  role-carrying dispatches (at the role narrowing, structural: beside the
  narrowing, never inside it), resolving the orchestrator from the run's
  own start record, delivered through the orchestrator's control file,
  landed attributed to the persona and its thread (the steer frame's
  discipline), bounded like a steer; the child's own call is logged by the
  ordinary tool-call path (hard rule 5).
- [x] C2: the room hierarchy — the lead is the orchestrator; member to
  member reroutes to the lead with an `unresolved dependency` Recommend
  signal and the speaker moves to the hub; lead/member/`@you` pass;
  room_line teaches the topology.
- [x] C3: the signal renders on the room's existing Recommend path; the
  delegation panel's GraphNode.reason stays the graph-side slot.
- [x] C4: tests — the parse shape, the hub gate (reroute + signal + the
  passes), the tool delivery (attributed payload in the parent's control
  file; the orphan refusal), the scope pin (structural admission).
- [x] C5: surfaces.md carries the hub rule and the signal's meaning; the
  gate; commits on master, push; local deploy; no release steps.

## Out of scope (named, not built)

True mid-run parent consumption; async children; auto re-decomposition
from the signal; peer channels between delegation siblings.
