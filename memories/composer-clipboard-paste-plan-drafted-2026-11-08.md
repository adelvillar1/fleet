---
name: composer-clipboard-paste-plan-drafted-2026-11-08
description: "project 2026-11-08 — the composer's clipboard-paste plan is DRAFTED and gated (docs/plans/2026-11-08-composer-clipboard-paste.md, status draft); a paste affordance reads the clipboard in Rust and returns the image's BYTES, the 64 KiB request cap is REMOVED (the owner's two steers), the run builds the vision message, and the router forces the vision workload; awaiting the owner's approval"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner approved drafting the plan for the composer's clipboard paste after
the live measurement ([[composer-paste-measured-live-2026-11-08]]) showed the
unmet case is **image paste** — a silent no-op whose promised refusal never
fires. The plan is `docs/plans/2026-11-08-composer-clipboard-paste.md`,
**status draft**, gated (arch PASS with 25 conventions gaps; plan-gate GAPS —
C0/C8/C9 flagged MISSING, the house's systematic process-criterion pattern, same
waiver as the last five plans). Not yet active; the owner's approval is the
next step, then the worktree.

**The design's three decisions, each earned by reading the code:**

1. **The clipboard read lives in Rust, not JavaScript.** The webview will not
   hand an image to the page — that is exactly why the image paste is a silent
   no-op. A new `GET /api/clipboard` reads the system clipboard (`arboard`)
   and returns `{text}` or `{image: {name, mime, bytes}}` (base64); one
   endpoint serves both faces (the shell and the browser face run the same
   server), and the call rides the user's click as its gesture. The reader is
   behind a trait so the tests do not touch the system pasteboard.
2. **The image travels as bytes, and the artificial body cap goes.** The first
   draft wrote the image to `<fleet-home>/clipboard/` and carried a path,
   because the face's request body is capped at 65,536 bytes (KI-378) and a
   screenshot is far larger. The owner steered twice: "we should increase the
   cap to allow screenshots", then "models these days have large context
   windows, putting an artificial cap makes no sense". The plan now **removes
   `MAX_BODY_BYTES`**, keeps a 25 MiB sanity limit as a process guard (not a
   content cap), and drops the composer's per-image and file-count bounds. The
   real bound is the model's context window: a payload past it fails at the
   provider with that provider's error, surfaced as a named refusal rather than
   a silent truncation. The chip shows the image's size so the owner sees the
   spend before sending.
3. **The downstream path already exists, unused.** `Message::text_with_images`
   (`src/api.rs:82`) and `ImageRef` are present but no run uses them, and the
   router already declares a `vision` workload with multimodal candidates
   (`GLM-5.3-Flash`, `mimo-v2.6-flash`, `step-5-preview`, `gemma4:31b`). The
   plan wires both: the run builds the vision message when an image is
   attached, and the route decision forces `vision` for an image-bearing run
   whatever the task's words judge — a task carrying an image must not be
   judged by a text-only chain.

**The criteria (C0–C9):** the paste affordance drawn and measured (C0 — the
house mockups rule applies to the new affordance); the clipboard endpoint
returning text and the image's bytes (C1); the button inserting text (C2) and
attaching an image (C3); the payload carrying the bytes with the cap gone and
the sanity limit pinned (C4); the run's vision message (C5); the forced vision
workload (C6); the bot face's same affordance (C7); the gate (C8); the docs
(C9).

**Risks the plan names, not yet resolved:** `arboard` is a system dependency
that must name its failure on a locked session; a home with no multimodal
model fails by name (the router's existing rule) — an image paste with no
vision model is a named refusal, never a silent text-only send; removing the
cap shifts the bound to the provider, so the run must surface the provider's
error as a named refusal rather than a silent truncation; and the paste
event's image branch stays unreachable in the webview (it remains as the
real-browser path, and the new button is the app's path).

**Why:** the owner's ask was ambiguous across four cases; measuring first
([[composer-paste-measured-live-2026-11-08]]) exposed that the obvious reading
(Cmd+V) already worked and the image path is the real gap. The plan's shape
follows the house's pattern: mockup first, then the mechanism, then the
wiring, then the gate.

**How to apply:** on approval, flip the plan to `active`, record the
disposition, and set up the plan worktree (the house's collision rule). The
first increment is the mockup (C0) — drawn, measured in every theme, and
owner-read before any code. Related: [[composer-paste-measured-live-2026-11-08]]
(the measurement), [[computer-use-measured-the-installed-app-2026-11-08]] (the
probe method), [[a-plan-gate-will-not-credit-a-process-criterion]] (the C8/C9
waiver pattern), [[hybrid-retrieval-delivered-2026-11-08]] (the app this
targets).
