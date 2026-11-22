---
status: active
created: 2026-11-21
plan: kanzo-board
---

# The kanzo board — the owner's intent backlog, and how bots answer it

The owner's design, stated 2026-11-21: **kanzo is the board where the owner
puts what they want to accomplish; Fleet's bots/personas look at the tasks and
take on work from there — or tell the owner "I can help you this way on ⟨task⟩"
— and then keep the board updated with the work they helped on.**

This is a **handoff protocol**, not a sync. The division of authority:

- **kanzo is the truth for human intent.** The owner writes tasks however they
  like. Fleet NEVER creates or edits a task's wording, and NEVER adds tasks
  unasked. Kanzo's own "human" mark means agents leave it alone — Fleet reads
  it as an instruction, never permission.
- **Fleet is the truth for execution.** What a bot did, the run's records, the
  artifacts: those are Fleet's facts, reported onto the board as comments and
  card moves — receipts, not rewrites.
- **Nothing is taken silently.** The owner's consent is the accept press on
  the board (or a reply). An offer that is never accepted decays by name.

## The mechanics (all through kanzo's own MCP server)

Kanzo ships an MCP server in the same binary (`kanzo` at `localhost:4242`):
agents take tasks, move cards, and comment. Fleet integrates as a **MCP
client lane** — measure-first: the vendored-lane pattern (`services/`, the
lane registry in `src/lanes.rs`, health via a `kanzo_probe` mirroring
`sem1_probe`) with an MCP client against `localhost:4242` (the transport and
tool names are measured from the installed kanzo's own docs at build time —
the owner approves the install and start the way every lane is started; the
owner decides the install, the lane only consumes what is already there).

**The loop, one pass at a time:**

1. **READ** the board's unclaimed, non-human tasks (the backlog column the
   owner uses — the column name is the owner's setting).
2. **PERSPECTIVES** — every capable persona (the roster's own matching) posts
   **one bounded comment in its own name** through kanzo's MCP authorship: how
   *this* persona would approach it, what it would need, what it would worry
   about. No thread sprawl, one comment each, attributed.
3. **SYNTHESIS** — the roster's designated convener (the orchestrator persona;
   default: the top-capability persona when none is designated) posts **one
   synthesis comment**: each perspective digested in a line *and attributed to
   its author*, the **disagreements named as disagreements**, and a **suggested
   plan of action** — steps, which persona owns each step, and the accept
   affordance. The synthesis may not invent a consensus: it must cite the
   perspectives it used and name what they disagreed on. Red-proof: a
   synthesis whose citations do not cover the perspectives posted fails
   ("the synthesis spoke for personas that did not speak").
4. **ACCEPT** — the owner's accept (board press / reply) starts the suggested
   plan: the run proceeds through Fleet's normal paths (multi-persona when the
   plan names them), carrying the kanzo task's id + URL on the session.
5. **REPORT** — on completion (each step's receipts landing as comments) the
   bot comments what it did (the session's own record line + the artifact
   link) and moves the card to the board's done column; on a refusal or a
   failed run the card is commented with WHY and left in the owner's hands.
   Every move and comment in the board's own history is visible to the owner —
   the board is the audit trail, and Fleet lies by omission if a helped task is
   not reported.

The **single-offer flow is the degenerate case**: when exactly one persona
matches, one perspective plus a short synthesis is the same loop with fewer
voices — the contract does not fork.

## The relay — paired devices, one board

Kanzo binds localhost; the owner's other devices reach their Fleet face through
the **fleet relay** (the paired-remote listener). The board joins them:

- **A relay route, not a second board.** The listener serves `/kanzo/…` as a
  proxy to the kanzo origin on the home machine, gated by the SAME pairing the
  face and bot PWA use — a device is authorized by its pairing, not by a
  second login, and kanzo itself stays loopback-local; the relay is the only
  path. Two or more paired devices therefore use and work on **the same
  board**, through the same host Fleet already serves.
- **The proxy is transparent and content-honest**: stream request/response
  (body sizes unchanged, no caching of card content), measure whether kanzo's
  board uses websockets/SSE for live updates and relay those frames unbroken —
  a board that goes stale on one device while the owner edits on another is
  the defect this section exists to refuse.
- **Receipts are device-visible, not device-writable as history**: kanzo's own
  comment history stays the audit trail; the relay never rewrites it (the
  relay logs which paired session carried which request, Fleet-side, without
  touching the board's content).
- **The accept affordance works from any paired device** — the perspectives/
  synthesis/accept loop is not home-terminal-bound: the owner can accept a
  suggested plan from the phone on the couch. The persona bots act on the
  home; the OWNER acts from wherever the relay reaches.

**Acceptance criteria**: a seeded board opened from two paired-device-class
clients (the relay proxy, two browser contexts) shows one board — an edit on
one is visible on the other after kanzo's own update cycle; the loop's accept
press works from the second device; the unpaired-device-class client is
refused by the pairing gate before kanzo is reached (red-proofed by name); and
the proxy's body fidelity is measured (no size delta beyond encoding).


One card on the bots surface (and the same read the TUI consumes later):
**offers made, accepted, claimed, helped, refused** — each with the board
task's name, the persona, and the board task's URL, derived from the lane's
own action log (never invented, zero draws empty). The owner can also press
"check the board now" — a pass is on demand, not a cron the owner didn't ask
for.

## The rules

- The owner's board is never edited beyond the comment/move/URL moves a
  claimed task earns. A task the owner marked "human" is read as intent.
- Everything the board shows about Fleet's work comes from the session's own
  records.
- No rederivation of the second kind: the integration states what it did, it
  does not summarize what it "would have" done.
- The loop is idempotent per pass (a pass that crashes mid-board re-runs
  without double-offering — the lane's own ledger by task id, the
  `applied.json` pattern).
- The owner's accept is the only trigger for work. A stale offer is not a
  running job.

## Acceptance criteria

- [x] The lane starts it through its own health probe (`fleet kanzo probe`
      against the real 0.4.2 binary: version, MCP reachable, board answer) and
      says by name when kanzo is not running. The *install press* is the one
      half left to the settings card — kanzo was installed from its signed
      installer this plan landed it with; the press pattern is the same one
      `fleet install --provider laya` uses.
- [x] The seeded round-trip's law is proven in live tests against the real
      board: the offer posts, *disabling the accept-wait refuses by name* (an
      offer is not a job), the synthesis without citation coverage is refused
      ("the synthesis spoke for personas that did not speak: reviewer"), the
      ledger makes a second pass a no-op, and the integration test
      `the_kanzo_board_rides_the_paired_listener…` proves the relayed board:
      401 for the unpaired, byte-identical bodies for two paired devices, an
      edit on one visible on the other, the page mounted. The one honest
      half: the loop has run with **scripted voices and a live board**, not
      yet with real model voices — that first live pass is the next run's
      errand, not a claim this record makes.
- [x] A "human" task is never offered to — kanzo's own marking is read as
      the refusal ("for the user, not for agents"), zero comments posted (live).
- [x] The bots-surface card shows the round-trip receipts from the lane's
      ledger and answers the four sentences (red-proofs: deleting the empty
      sentence and the selected clause each failed the case by name, 11/11).
- [x] `cargo test` green at the closing gate (2801 units / 0 failed); the
      lane's tests ran against the real `kanzo` on this machine. The relay's
      integration test needed its own fix to run at all and its own line in
      the record: *the flavor is multi-thread on purpose* — a blocking ureq
      call on a current-thread runtime starves the accepted loop forever
      (measured with a stack sample: the connect sat unanswered while `serve()`
      was never polled).

## Status pass, 2026-11-22 — landed

The lane, the loop's law, the relayed board, and the card are landed:
`d40249b0` (the lane, probe, loop, card — measured MCP facts in the commit:
stdio newline JSON-RPC `2025-06-18`, tools the lane calls four of, columns,
the `me` consent signal, no SSE so the relay is plain HTTP) and the relay
follow-up (the census-registered `src/remote/kanzo_proxy.rs`, the listener
wiring, and the integration test above). One dead-end corrected in the record:
the first appended relay test hung not on the proxy but on the test's own
runtime flavor.

The **one claim not yet made**: a pass with real model voices against the
live board (the tests ran scripted voices; the owner's first `/kanzo pass`
with personas is the day it starts earning its keep).
