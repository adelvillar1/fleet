---
name: session-parity-remainders-plan-drafted-2026-11-02
description: "project 2026-11-02 — the remainders plan approved, IMPLEMENTED and COMPLETED 6/6 (5c0c5ab + W4 d816704, origin/master d816704): the Responses protocol as a stateless wire dialect per model, {trace}, the W0 probe committed (unrun — waits for a /responses provider), and W4's account-card headers row landing the day U9 unblocked it"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner asked (2026-11-02, after [[session-header-parity-plan-drafted-2026-11-02]] landed) for "a plan for the out of scope items", then said **"implement plan as designed"**. Plan `docs/plans/2026-11-02-session-parity-remainders.md` — **LANDED at origin/master `5c0c5ab`, gate 1,941/0 (face 16/0 inside), plan status stays ACTIVE: 5/6 criteria ticked, C4 carries its designed wait.**

**What landed:**
- **W1 — the wire dialect rides the roster**: `Protocol::Chat | Responses` (`Option<Protocol>` per entry, `model_protocols: BTreeMap<String, Protocol>` per model for the mixed gateway — OpenCode Go: kimi/glm over `/chat/completions`, Luna over `/responses`); `Candidate::protocol_for(model)` resolves override > entry > chat; `one_attempt` matches on it. ~21 roster literals swept via the uniform-tail replace_all trick.
- **W2 — the adapter** (`api.rs`): the chat pump extracted into shared `pump_stream`; `chat_stream_responses` builds `{base}/responses`, maps system→`instructions`, messages→typed input items, tool results→`function_call_output` (joined by `call_id`), tools→flat `{type:"function",…,strict:false}`. **Statelessness is pinned, not promised: `store: false`, no `previous_response_id`, full history every turn** (hard rule 5). Streaming maps typed SSE onto the shared `StreamEvent` vocabulary — text/reasoning deltas live, **argument-delta fragments consumed without effect because `response.completed`'s own output is the authoritative whole call** (no fragment assembly needed, contra the plan's first draft; the LiteLLM #20711 class is thereby moot).
- **W3 — `{trace}`**: third placeholder, fresh UUID per preparation (`session_id_for(None)` even when the session is seeded — a trace is not a session), substituted beside `{session}`; `wire_headers` and `Router::build` both gained the parameter.
- **W0 — `scripts/responses-probe.sh`** committed, syntax-checked, on-demand only; its live run is the named state C0 carries until a `/responses` provider+key exist on this machine.
- **C4 — the face's headers row: UNBLOCKED 2026-11-02 (`c9c6139`).** It was open by design, waiting on settings-ux U9's account card; U9 landed (`d13ca29`, the card's walks + the job card's next fire), so the plan's C4 note now reads UNBLOCKED and names the cut: a row inside `providerSections`, its write riding the provider entry's `headers` object, its text never a resolved value. The criterion itself is still unticked — the row is not built — see [[settings-ux-plan-2026-11-01]].

**Build traps:** clippy `-D warnings` killed `chat_responses` as never-used (the walk only streams) — delete rather than keep-alive-by-a-pin; a python deletion that replaced tail-to-EOF ate the `impl Client` closing brace (unclosed-delimiter error at the file end is the tell); the parallel session's uncommitted rustfmt reflow of my own hunks arrived mid-edit (see [[a-parallel-session-co-edits-your-file]]) — it was complementary, kept, and the file re-read before editing. Dispositions: the borderline trio waived (p≈0.31–0.33), use case B **fixed** by rewording to name the concrete mechanism (`{trace}` in a header value) — use cases must name the deliverable, not the scenario.

**W4 LANDED same day (`d816704`, origin/master `d816704`) — the plan is COMPLETED 6/6.** U9's card arrived mid-session (`d13ca29`, the parallel session; the plan amended `c9c6139`), so the row was built the same evening:
- **The read carries the CONFIG's headers, never the resolved ones** — `RosterEntry.headers` is filled in `router_config` by zipping `resolved` with the RAW `roster.candidates`, because `resolve()` substitutes `{env:NAME}` in place and a substituted value is a secret no surface renders. `catalog_document` and the roster read both carry it.
- The account card (`providerSections`) gained **the headers its requests carry**: name+value boxes per header (placeholders visible — what the owner typed), add/remove presses, the save riding the provider write the card already makes (always sending the `headers` object; emptied rows = a deliberate clear).
- **The trap named inline, live**: a session-named header with a constant value renders the affinity-bucket warning beside the field as it is typed (JS mirror of the Rust `session_constant_warning`; the writer stays the authority at save).
- Eight face pins (pre-filled placeholder, trap show-and-silence, add/remove, the save's write); harness **633/0 across 56 cases**, suite **1,941/0**.
- **The server's credential sweep caught the first draft**: the empty-state text said "the request carries the key's **Authorization**" and `no_payload_carries_a_credential` red'd it — face copy is a served payload; see [[the-credential-sweep-reads-the-faces-copy]].
- Harness-count coupling: new card writes break older `length === 1` write-count pins (the connect check counted ALL `op:provider` calls) — scope the filter to the write's own distinctive substring, and remember the entry JSON's inner keys are ESCAPED in the body, so match on the raw key name (`x-opencode-session`), never `\"headers\":{…}`.

**The whole session's later arc (2026-11-02).** W4's landing (`d816704`)
completed this plan at 6/6 — and the same evening the owner's *"we had to
upgrade rust to 1.99.1 and that generated some conflicts, the session cleaned
all that up"* arrived: the cleanup commit `eabe031` carried the 41 lint fixes
AND the co-tenant's parked plugin-host work, and the follow-up session landed
the finish at **`c8dde93`** with the gate re-verified (1,996/0, harness
678/0). The re-verification earned its keep — the co-tenant's merges had
**reintroduced six lints already fixed** ([[a-parallel-session-can-reintroduce-your-fixed-lints]]).
See [[session-recap-2026-11-02-two-plans-landed]] for the session's full arc.
