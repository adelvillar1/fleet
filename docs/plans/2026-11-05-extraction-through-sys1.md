---
status: draft
created: 2026-11-05
updated: 2026-11-05
slug: extraction-through-sys1
---

# Plan: the extraction leg goes through sys1

## Context

The owner's ruling (2026-11-05): sys1 was built to abstract the System
One models and providers, so no caller may talk to a vendor directly.
The route's judge was the first violator (a hand-copied vendor key list;
fixed, `95dc98c`). The **extraction leg** is the bigger one and it is
still open: `src/extract.rs` posts straight to three vendors —

- **LOCATE** → `api.fastino.ai/v1/chat/completions` with
  `fastino/gliner2.5-multi-v1` (`EXTRACT_BASE_DEFAULT`, `extract.rs:49`)
- **VERIFY** → glide (`api.fastino.ai/v1/systemone`), drex
  (`drex.nace.ai`), jev (`api.typesafe.ai`) — three System One
  endpoints, chosen by a chain the leg builds itself
  (`extract.rs:226-262`)

— and resolves their keys in Rust (`FASTINO_API_KEY`, `TYPESAfe_…`,
`DREX_…`). Meanwhile the vendored sys1 lane already receives exactly
those keys through its passthrough (`src/lanes.rs:86-92`), already has
the provider registry and the failover, and already exposes an MCP
surface Fleet calls for the route and the curate beats.

Two facts make the move clean:

- **The VERIFY half needs no new sys1 capability**, only a seam: the
  HTTP face already accepts an inline `task_spec` and builds a `Task`
  from it (`services/sys1/service/main.py:151-158`), and the library's
  `classify(task=Task)` takes it — but the **MCP tool does not expose
  `task_spec`** (text / task / provider only). One optional parameter
  closes it.
- **The LOCATE half is a capability sys1 does not yet have**: an
  entity-span extraction verb. sys1's `decide` provider is
  GLiNER-2.5-**Decide** (a classifier); the extraction-class GLiNER is a
  different model on the same vendor. The call the leg makes today
  (schema + threshold + spans) belongs behind sys1's provider registry,
  not in Fleet.

So the leg becomes a **lane client**: Fleet asks sys1; sys1 chooses the
vendor. The floors, the segmentation, the reconcile, the budgets, the
absence semantics — the leg's own judgment — stay in Rust, unchanged.

## The contracts this plan answers

- **Hard rule 12, self-containment**: the extraction stays keyless in
  Rust (it resolves no vendor key), and the lane carries the call. A
  machine with no lane degrades by name, the way every lane already does.
- **The 2026-10-25 re-point ruling** ("the tier's own vendor already
  serves it") — that ruling chose *which vendor*, not *who calls it*;
  sys1 is the tier's abstraction, so the vendor choice moves inside it.
- **The vendored service is Fleet's own** (`services/sys1`, VENDOR.json):
  a tool and a provider path can be added there without an upstream
  release, the `tool_route`/`curate`/`desk_coverage` precedents.
- **The extraction leg's measured behaviour** (the jobhound floors, the
  absence semantics, the capped segments) is the contract: none of it
  changes; only the transport moves.
- **The documented architecture this plan amends**:
  `docs/architecture/lanes-deep-dive-2026-11-04.md:21-26` records the
  leg as "its own client — LOCATE on Fastino's
  `fastino/gliner2.5-multi-v1`, VERIFY walking the glide → drex → jev
  chain with the keys resolved from the home env… No MCP in that path by
  design". That sentence was an observation of the code, not a ruling,
  and the owner's 2026-11-05 ruling supersedes it: the leg becomes a
  sys1 client, and that doc's "How Fleet calls the lanes today" section
  is amended with this plan's landing. The arch-gate's DRIFT on this
  approach is that contradiction, made deliberately.

## Approach

1. **sys1 gains an extraction verb.** `services/sys1/src/sys1/extraction.py`:
   the entity call the leg makes today (schema of named entities +
   descriptions, `threshold`, `include_confidence`, `include_spans`,
   `store: false`), posted to a config-driven vendor base/key/model
   (`SYS1_EXTRACT_BASE` → the provider's base; the Fastino key the
   provider registry already resolves; `SYS1_EXTRACT_MODEL` default
   `fastino/gliner2.5-multi-v1`). It parses the vendor's entities map
   into `{field, text, start, end, confidence}`, dropping a span whose
   offsets do not slice back (the leg's own rule) and **declining
   empty per field** — never inventing.
2. **The MCP surface gains `sys1_extract`** (text, fields) and
   **`sys1_classify` gains `task_spec`** (one optional object; when
   present it is the task, exactly the HTTP face's seam).
3. **`src/extract.rs` becomes a lane client.** It spawns the sys1 lane
   (the judge beat's own `LaneSource`/`McpServer` session), calls
   `sys1_extract` for LOCATE and `sys1_classify` with a `task_spec` for
   VERIFY (one noul head per field, the same questions the leg asks
   today). The floors, segmentation, reconcile, budgets and absence
   semantics are untouched. `ExtractConfig` loses its vendor bases/keys;
   `sys1_extracts()` becomes an attempt, not a key gate.
4. **The tests move to the lane.** The leg's stub-endpoint tests become
   `mock-mcp` scripted tests: the mock answers `sys1_extract` and
   `sys1_classify`, and the leg's parsing/floors/reconcile are asserted
   exactly as now.
5. **Prove it live**: a scratch home, the machine's keys in the env, a
   real document through the leg — the claims, their quotes, the
   floors, a contested span — with the vendor call observed inside the
   sys1 lane (its own telemetry line), not from Rust.
6. **Docs + the gate**: `docs/architecture/lanes.md` and
   `docs/features/tools.md` name the move; the gate runs whole.

## Acceptance criteria

- [ ] **C1 — the extraction verb.** sys1 serves `sys1_extract` (text +
      fields): the entity call against the configured vendor, spans
      parsed with offsets and confidence, a mismatched offset declined
      and named, an empty field declined. Its own tests cover the
      parse and the decline.
- [ ] **C2 — the classify seam.** `sys1_classify` accepts an optional
      `task_spec` and builds the task from it (the HTTP face's
      `task_from_spec`), with the same routing as a registered task.
      Pinned by a test over the MCP handler.
- [ ] **C3 — the leg is a lane client.** `src/extract.rs` makes no
      vendor HTTP call and resolves no vendor key: LOCATE rides
      `sys1_extract`, VERIFY rides `sys1_classify` with a per-field
      `task_spec`. `ExtractConfig` carries the lane, not a base/key.
      Proven by the absence of any `api.fastino.ai` / `drex.nace.ai` /
      `api.typesafe.ai` string in the leg and by the gate.
- [ ] **C4 — the behaviour is unchanged.** The floors (0.85/0.35), the
      segmentation constants, the budgets, the absence semantics and
      the quote rule are byte-identical; the existing pins pass against
      the lane.
- [ ] **C5 — the availability is an attempt.** `sys1_extracts()` no
      longer gates on a key list: the leg attempts the lane and reads
      its answer or its named error.
- [ ] **C6 — the tests.** The leg's stub tests are mock-mcp scripted;
      the suite is green.
- [ ] **C7 — the live proof.** On a scratch home: a real document
      extracted through the lane, the claims and quotes quoted in the
      recap, the floors demonstrably applied, and the extraction's own
      telemetry line read from the sys1 lane's log.
- [ ] **C8 — the docs.** `docs/architecture/lanes.md` (the extraction
      leg as a sys1 client), `docs/features/tools.md` (the
      `read_document`/extract path), and the service's own README note
      the verb.
- [ ] **C9 — the gate.** fmt, clippy `-D warnings`, release build,
      `cargo test` — exit codes read directly; the service's Python
      tests (its own) run.

## Files to be touched

- `services/sys1/src/sys1/extraction.py` — new: the entity pass.
- `services/sys1/service/mcp_server.py` — the two tool surfaces.
- `services/sys1/src/sys1/api.py` — the extract entry (if the seam
  belongs at the library).
- `services/sys1/tests/` — the parse/decline/task-spec tests.
- `src/extract.rs` — the lane client.
- `src/agent.rs` (the leg's caller) — the lane source it passes.
- `tests/integration.rs` — the leg's pins, scripted.
- `docs/architecture/lanes.md`, `docs/features/tools.md`.

## Out of scope

- The lane's provider chain or the vendor choice for extraction — the
  configured vendor serves; a second extraction vendor is its own plan.
- The route/curate gate (already through the lane, `95dc98c`).
- `documents.rs`'s local reading path — local extraction is not a
  System One call and stays keyless.
- Any change to the leg's measured floors or its segmentation.

## Verification

```sh
# the service's own tests
cd services/sys1 && python3 -m pytest -q

# the leg's pins and the gate
cargo test
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The live proof drives the release binary on a scratch home (the
machine's keys in the environment, nothing on disk), extracts a real
document, and reads the lane's own log for the vendor call — the proof
that the call is sys1's.

## Linked artifacts

- `docs/architecture/lanes.md` — the leg as a sys1 client.
- `docs/features/tools.md` — the extraction path.
- `services/sys1/README.md` — the new verb.
- `docs/recaps/` — the proof.

## Risks

- **The span wire is the vendor's** — the entity reply's shape (entities
  map, offsets, confidence) is parsed inside sys1; a vendor change is
  one lane's change, not Fleet's. The parse is pinned and declines
  loudly.
- **The lane spawns per extraction.** The leg is a once-per-document
  call; the lane's own launch span is the cost. The judge beat already
  pays it for a judgment.
- **The lane's provider registry may route VERIFY differently** than
  the leg's hard-coded glide→drex→jev chain: sys1's chain is the
  authority now, and the leg's floors are applied to whatever answers.
  The live proof observes which provider served.
- **The vendored service's tests need pytest** — the environment has
  it (the service shipped with its tests); a missing pytest is a named
  skip, not a silent one.
