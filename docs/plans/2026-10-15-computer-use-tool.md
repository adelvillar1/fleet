# The computer-use tool

Status: complete
Date: 2026-10-15

## Why

Every tool fleet has reaches a file, a process, or a lane. None of
them reaches the desktop — the browser window with a form half filled,
the native app with no CLI, the thing a human would do with a mouse.
The cua driver exists to drive that surface, and fleet should expose
it as a tool.

## The ruling that shapes the whole plan

**Computer use requests approval every time, regardless of the
execution mode.** Not `yes`, not a config that asked for `yes`, not an
unattended cron job. The owner's ruling (2026-10-15), recorded in
`docs/plans/2026-10-15-run-control-surfaces.md` and already encoded in
`src/approvals.rs` as `ALWAYS_ASK_TOOLS`.

The reasoning: a click, a keystroke, or a screenshot has the machine
itself as its blast radius. `bash` can destroy a file; computer use can
send an email, submit a form, or read whatever is on screen. The
destructive class under `yes-guarded` is about severity; this is about
whose machine is at risk. The ruling is encoded before the tool exists
so the tool inherits it rather than needing a second change to
remember.

## Acceptance criteria

- **C1** A `computer_use` tool declared on every run, admitted to a
  delegate's scope by default, and dispatched through the cua driver
  over the same lane-client path the judge and the web tool use.
- **C2** Every call asks, in every mode. The gate's `ALWAYS_ASK_TOOLS`
  entry is load-bearing and pinned by a test that runs the tool through
  `Gate::approves_without_asking` under `Mode::Yes` and asserts false.
- **C3** A forwarded `subagent → computer_use` call is caught by the
  same rule — the arrow prefix is stripped before the lookup.
- **C4** The prompt the model reads names what the tool can reach and
  what it cannot: it drives the desktop, it does not read the model's
  own credentials, and a screenshot is an image the model must ask to
  see rather than text it can grep.
- **C5** The action surface is small and named: screenshot, click,
  type, key, scroll, drag. Each takes bounded coordinates or text. No
  arbitrary shell, no arbitrary file path.
- **C6** Every action lands its row in the session log with the action,
  the bounded arguments, and the approval it carried — the
  model-visible⟺logged rule applies to the desktop exactly as it
  applies to a file.
- **C7** A screenshot's bytes never ride the log as text. The log
  records that one was taken and where it landed; the image is a file
  beside the session, named in the row.
- **C8** `fleet doctor` names whether the cua driver is reachable, the
  way it names the vector path and the web leg.
- **C9** Tests: the declaration rides every run; the always-ask rule
  fires under every mode; the forwarded call is caught; the log records
  the action without the image; an unreachable driver is a named
  refusal the model can read.

## Design

The cua driver is an external process, so the tool is a lane-shaped
client: resolve the driver's launch spec the way `judge()` resolves
sys1's and the web tool resolves blend1's, spawn, call, tear down. The
`Work::Web` variant in `src/agent.rs` is the pattern to copy — a spec
riding the work because it is resolved where the launch is known.

The tool's name is `computer_use`, which `ALWAYS_ASK_TOOLS` already
carries. Nothing in the approvals module changes to land this plan;
that is the point of encoding the ruling first.

## What this plan deliberately does not decide

- **Which cua driver.** The plan assumes one exists on this machine
  that fleet can spawn over stdio; if none does, the first task is
  naming it, and the plan's shape does not change.
- **Whether a screenshot is vision-analyzed in-process or handed to
  the model as an image.** The model-visible path decides it; C7 only
  requires the bytes never ride the log.
- **The delivery seam**, still open in `docs/architecture/delivery-seam.md`.

## Out of scope

- Automating a specific application. The tool drives a desktop; the
  task says which.
- A recording or replay surface for desktop sessions. The session log
  is the record.
- Any mode that waives the approval. The ruling is not a default to be
  tuned later.
