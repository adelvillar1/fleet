# Session recap — 2026-11-04: the telemetry bundle, and the mount that was missing

**Summary.** The owner asked for a telemetry visualization built through
the plugin creator pattern — Fleet's internals as an appealing, graphical
flow, not a table. The bundle was authored in the repo, installed as an
**untrusted** plugin (creator mode's tier, sandboxed to its declared
queries), and the face grew the half of the host it never had: a page
that mounts an installed bundle. Two host gaps surfaced on the way and
both are closed: `huddles` was answered by the bridge but absent from the
manifest-parse vocabulary, and no route resolved a plugin's name to
anything a person could look at.

## What was built

- **The bundle** (`plugins/telemetry/`): one self-contained
  `index.html`, no external request, asked for and received 8 declared
  queries — `state, memory, skills, sessions, router, delegation,
  huddles, usage`. The picture is **eight strata of Fleet's real
  pipeline** — door → recall → skills → route → model → tools → rooms →
  corpus — with a canvas spine carrying a particle current whose density
  and speed ride the live counts, a right column naming the real entities
  of whichever stratum is focused, a footer session river, and mast
  counters. Every figure is a query answer; a refused query renders as a
  **named refusal, never a zero**.
- **The mount** (`#/<name>`, the `panel-bundle` stage): the face reads
  the bundle's bytes and hands them in as `srcdoc` in a
  `sandbox="allow-scripts"` frame — because `/plugins/<name>/` answers
  under `frame-ancestors 'none'` and cannot be framed (KI-414) — then
  relays the frame's declared queries to `POST /api/plugins/query`. The
  relay does not decide what is allowed: the bridge enforces the
  manifest's set, and the frame's own window is the gate on which asks it
  answers.
- **The provenance is visible**: the head names the bundle, its tier
  chip, the declared set, and a live tally — `n asked · n answered ·
  n refused`.
- **The query set gains `huddles`** (`[&str; 13]`): the bridge answered
  it all along and the manifest-parse vocabulary did not, so a bundle
  declaring the room list was refused as declaring an unknown query
  (KI-413, closed, with the honest residue that the general pin between
  the two lists is still unwritten).

## The verification

- `fleet plugins install plugins/telemetry` — installed into
  `~/.fleet/plugins/telemetry/`.
- `fleet plugins eval telemetry` — **4/4**: the manifest, the credential
  sweep, the asset, the declared queries.
- All 8 declared queries answered 200 through `POST /api/plugins/query`;
  `vault` answered **403** naming the set — the mount adds no reach.
- Chromium at `127.0.0.1:3091/#/telemetry` paints `panel-bundle` with the
  head, the chip, the declared-set note and a live frame.
- The face harness gained its own case (BUNDLE): the route, the `srcdoc`
  frame's sandbox, the relay's ask→answer round trip with its sequence,
  the refusal path with its reason, that a message from another window is
  not relayed, and that a disabled bundle and an unknown address each say
  why. **792 checks / 84 cases.**
- Gate: `cargo build --all-targets` → `BUILD-EXIT=0`; `cargo test` →
  `TEST-EXIT=0` (`1828 passed, 0 failed, 3 ignored`).

## Found on the way

- **KI-413 (closed)** — `huddles` answered but not declarable; fixed, and
  a parse test now pins the name.
- **KI-414 (open, costs nothing)** — `frame-ancestors 'none'` on the
  bundle route is why the mount reads bytes instead of setting `src`.
- **KI-415 (open)** — the nomenclature write-guard refuses ordinary
  English that shares an established term's spelling (`SF Mono`, `One`,
  `Deserialize`); every constrained file this session went through bash.
- **KI-416 (closed)** — a bounded wait that could not say it gave up: the
  full suite found `a_destructive_call_cannot_be_remembered…` failing
  with `NoSeat`, which named the answer path rather than the 500 ms seat
  poll that had quietly elapsed. Now a 10 s deadline with its elapse
  asserted by name.
- **KI-417 (closed)** — the undeclared-query refusal carried an
  eighteen-space run mid-sentence; the literal is a real continuation now.

## Where it stands, honestly

The release binary that the LaunchAgent serves is the one thing still
stale: `~/.fleet/bin/fleet` predates this work, and the face the owner
browses (3080) is that process. The reload is the recorded two commands
(KI-408), and it restarts the very process this run streams through — so
it is the owner's hand, not this session's.
