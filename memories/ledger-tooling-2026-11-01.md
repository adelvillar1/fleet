---
name: ledger-tooling-2026-11-01
description: "project 2026-11-01 — the ledger's own tooling landed: the checker refuses a status outside open/closed/noted/none (KI-322), `fleet ki append` files a row under the ledger's lock (KI-331); severity drift filed as KI-391; both follow-on plans drafted (56438be), ledger 391 rows / 32 open"
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

The 2026-11-01 ledger session's tooling half, both halves landed the same day:

- **KI-322 closed (`d62c217`)** — `scripts/ki_ledger.py` gained `status_word()`
  (strip emphasis, first word before a dash/paren/colon/space, so the table's
  own `closed-fixed` and `open — reason` shapes both land) against
  `STATUS_WORDS = {open, closed, noted, none}`; a row leading with anything
  else is a named problem at its line and the run exits 1. Proven red on a
  scratch ledger and green on the real one.
- **KI-331 closed (`4e12b01`)** — `fleet ki append` (`doctor::append_ledger_row`)
  is the run-time append path: five row cells in, the id derived from the
  table **while `FileLock::beside` is held**, temp+rename out, the id printed.
  A cell carrying a pipe/newline is refused by name (KI-084's class) and a
  status outside the four words dies at the writer (KI-322's class). Pinned by
  `concurrent_hand_appends_never_share_an_id` (8 released → KI-002..009, six
  cells each) and a refusals test; the ledger's id rule and both contract docs
  now name the verb. Before it, scheduled catch-ups hand-appended (read table,
  max+1, edit — nothing serialized) and had shared ids live twice.
- **KI-391 filed (open)** — the severity vocabulary: the preamble names six
  words but the table carries `note` (14), `warning` (4), `flake`, `decision`,
  `environment`, `constraint`, `hang`, plus four rows with `defect **fixed**`
  in the severity cell. **Ruling framed as O3** in
  `docs/plans/2026-11-01-open-calls.md` (see
  [[corpus-contract-and-open-calls-plans-2026-11-01]]): my FINAL
  recommendation (after the 25723a9 re-audit flipped it) is the rows
  normalize to the six-word taxonomy — the drift words are all
  expressible in it — and the checker then polices severity beside
  status, completing the KI-322 pattern.
- The morning's four malformed rows (KI-385/388/389/390, evidence merged into
  the issue cell — 5 cells vs the header's 6) were repaired at `e7a0a21`.

Ledger state at session end: **391 rows, 32 open** (was 38) — the five
pin-sized defects closed at `be4488a` (see [[pin-sized-defects-closed-2026-11-01]]).
Remaining work is the two plans committed at `56438be`: the corpus-refresh
contract (buildable now, W1 first) and the seven open calls (awaiting the
owner's rulings — O1 KI-367 relay join, O2 KI-378 send bound, O3 KI-391,
O4 KI-328, O5 KI-386, O6 KI-332, O7 KI-369). KI-249/252 stay with the
active embed-bounds plan. Unpushed: `e7a0a21`, `d62c217`, `4e12b01`,
`be4488a`, `56438be` on local master.
