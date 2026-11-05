---
name: embeddings-cannot-label-a-session-project
description: "measured 2026-10-20 and 2026-10-23 — embeddinggemma cosines on fleet are SOFT (unrelated text scores ~0.60, real matches only 0.69–0.80 and near-duplicate tools 0.01 apart), so session-to-project grouping got 1/81 top-1 and a tool-search floor of 0.35 could never abstain; classify by paths, and calibrate any cosine threshold against the live embedder"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The owner's ask, 2026-10-20:** "can you use the embeddings?" — raised while
building the sessions archive's project grouping ([[fleet-sessions-archive-groups-by-project]]).
The intuition is reasonable: fleet already embeds sessions.

**What exists.** `~/.fleet/sessions/.recall-index.json` holds one 768-dimensional
vector per session (EmbeddingGemma at `127.0.0.1:8901`, the session-recall
index). Project profiles are available from `tower_data.json`'s own
`tech_summary`/`spec_summary` per project.

**The measurement (do not re-run blind).** Classify each session's vector
against 32 project profiles, with the 81 sessions whose project is known from
their own log paths as ground truth: **1 correct out of 81 (0.012)**, top
similarity ~0.73–0.76 for essentially every pair, **median margin 0.00**
(top and second candidate statistically tied). The 66 unlabelled sessions
look identical. These vectors encode the harness's shared vocabulary, not a
project signature — the same finding as
[[embeddinggemma-cosines-confirm-ki-058]] (cosines alone do not separate
relevance in this corpus).

**The same softness, measured again 2026-10-23** while calibrating tool
retrieval ([[fleet-tool-surface-analysis-2026-10-23]]): a realistic
twenty-tool catalog put true intents at **0.691–0.802** and requests that
should match nothing at **0.602–0.668** — a separation of about 0.03, not
the 0.4–0.9 spread one expects from a cosine. Two tools with near-identical
meaning sat **0.009–0.010** apart. The first tool-search floor (0.35) was
therefore below the noise band and could never have abstained; the honest
floor is 0.68. Neat synthetic corpora (one distinctive token per card) do
score cleanly — 0.83–0.85 and 24/24 top-1 — which is exactly why a fixture
corpus flatters a retriever and a realistic one does not.

**What shipped instead, `f6593d6`.** Grouping uses evidence that exists:
`Session::create` records the workspace in the start record (new runs), and
`summarize` infers it for older logs from the first `/Projects/<name>` its own
tool calls or commands name, exposing `workspace_inferred` so the group header
says **inferred**. 15 groups, 81 runs attributed, 66 honestly in "no project".
A log with no paths and no recorded workspace stays unattributed.

**The third ask, 2026-10-23, and where the vectors do work:** while planning
the tool lifecycle ([[fleet-tool-availability-gaps-2026-10-23]]) the owner
said "we have embeddings at the conversation level and we can probably derive
what we have used or needed to use" — correct, with one boundary. The
session index holds **one vector per session**, not per turn, and each
session's synthetic fact already carries its successful tool calls as
`phrases`; so the conversation-level vectors are a usage handle, never
turn-level coverage. Turn-level "the owner needed a tool that was never
called" must embed the owner's turns itself (cached by message hash). And it
is a **review** list, not evidence: the near-duplicate tools that sit 0.01
apart would otherwise read as misses. Usage stays exact from the logs;
embeddings supply the ranking, never the verdict.

**How to apply:** when the owner suggests embeddings, treat it as a hypothesis
worth measuring, not an instruction to wire — build the ground-truth set first
(path-labelled sessions are free) and report accuracy and margin before the
UI depends on it. And never ship a cosine threshold from intuition: this
embedder's baseline similarity for unrelated text is ~0.60, so measure a
realistic corpus (real descriptions, real requests, and explicit no-match
queries), then place the floor in the gap. Where the grain matters, name it:
a session vector answers "which session", not "which turn" or "which tool
call". Say plainly when the measurement says no, and record it so the idea
is not retried every few months.

Related: [[fleet-recall-bump-invalidates-the-embedding-cache]] (why the index
is keyed the way it is), [[verify-the-arithmetic-behind-a-measured-narrative]].
