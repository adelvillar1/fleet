---
name: plan-gate-drafting-mechanics
description: "How dev-decisions' plan gates actually parse plans (checkboxes document-wide, single-line criteria, section caps, ### split) — cap now 4000 and meta skip-list extended 2026-11-10; plus the jev pin/billing state"
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

The plan gates (`dev-decisions plan-gate`/`plan-surface`, script at `~/.agents/skills/dev-decisions/scripts/dev_decisions.py`, outside this repo) parse plans in ways that silently shift or starve criteria — confirmed by running them 2026-11-10 on `2026-11-10-improvement-modes-inbox.md`:

**Why:** `_parse_plan_criteria` collects every `- [ ]` line **document-wide** in file order — a `## Use cases` section with checkbox boxes becomes C0..C4 and shifts every C-number (the recorded trap from 2026-11-07; this is its mechanism). Criteria are taken as **one physical line** (wrapped criteria truncate mid-sentence). Only four sections are gated for coverage/scope: **Approach, Use cases, Risks, Dependencies** (Verification/Files/Context/Notes are in `_PLAN_GATE_SKIP_SECTIONS`), each body capped at **2000 chars** (`_PLAN_GATE_SECTION_CHARS`) — and `_parse_plan_sections` splits at `###` too, specifically so later criteria aren't truncated.

**How to apply:** de-box use cases (`- **A:** …`, never `- [ ]`); write each acceptance criterion as one physical line; structure `## Approach` as short `###` subsections each naming its criteria in text plus a numbered step (include an explicit gate-green step — Verification is skipped, so C-gate-green needs Approach presence); expect `SCOPE?` flags on the Approach intro parent, Use cases, Risks, Dependencies → `dev-decisions disposition plan-gate <plan> --status waived --reason …` (the 2026-11-07 plan waived the identical Risks flag), `--status overridden` when the flag misreads a criterion-anchored subsection.

**Disposition quirks (the second, 13-criteria run):** `disposition plan-gate … --status waived` records fine for the recurring four template-section flags, but **`disposition plan-surface … --status overridden` reported "no matching gate event found — no feedback rows written"** — surface dispositions don't stick (surface runs don't emit a matchable gate event); the durable record for a kept order is the plan's own "order is identity" header, not a disposition. Appending criteria AFTER a gated run (C10–C12 after C0–C9) preserves identity — re-run the gates, don't reorder.

**The 12,000-char state cap (found 2026-11-10):** plan-gate builds ONE state string — criteria lines + gated section bodies in doc order — truncated at `scan.max_diff_chars = 12000` (`~/.config/dev-decisions/config.toml`). Sections past the cut read as `[MISSING] + [UNVERIFIABLE]` with low p values (~0.001–0.07), NOT as structure warnings — the ###-split fix alone does not help if the *total* overflows. Budget rule: ~16 criteria ≈ 5,000 chars, leaving ~7,000 for criterion-bearing sections; keep Use cases/Risks/Dependencies *after* the criterion sections in doc order (the cut eats them harmlessly — scope reads sections separately, uncapped); simulate before gating a grown plan (criteria + gated bodies + ~85 intro + ~20/line; cap 12,000). Trimming Approach prose is the lever; criteria are contract and stay. Related: [[ki-ledger-pipes-shift-naive-column-reads]].

**Cap raised to 4000 + skip list extended (owner ruling, 2026-11-10):** the owner called the per-section 2000-char cap *"artificial and non-sensical — a large plan is bound to be large"* and ordered it to 4000. Fixed in BOTH copies (`scripts/devdec/gates.py` = live package, `scripts/dev_decisions.py` = standalone fallback — edit both, the shim prefers the package): `_PLAN_GATE_SECTION_CHARS = 4000`, and `_PLAN_GATE_SKIP_SECTIONS` += `decision-model check`, `use cases`, `risks`, `dependencies` (the template-mandated meta sections the scope head kept false-flagging; the gate's own comment already said meta sections false-flag, it just missed these four). Before the fix, a 3,645-char Approach read as coverage 5/10 with C5–C9 silently MISSING; after, 10/10. This closes the cap half of dev-decisions' open issue **D-001** — the truncation-*reporting* half (name each cut section + chars, selftest case) is still open, offered to the owner, not yet done. A `--provider glide` override gets plan-gate answering while the default jev chain is 402-dead ([[jev-chain-failure-diagnosed-2026-11-10]]); a residual SCOPE flag on a framing-only intro section can be left standing as a presented trade-off rather than waived.

**20-criterion run (grounded-selection, 2026-11-10):** the skip-list extension worked as intended — 8 kept sections (Approach intro + six `###` phases + …), coverage 20/20, the only flag the Approach intro (p=0.08, left standing); the total-state cap (12,000) fit at 9,648 chars — SIMULATE state size before gating any plan past ~16 criteria (criteria lines ~20/char-count + gated bodies), trimming Approach prose is the lever, criteria are contract. The live gate command for this repo: `dev-decisions plan-gate <plan> --provider glide` (package copy `devdec/gates.py` is what runs; keep both copies in sync). The cap+skip-list fix is COMMITTED on the dev-decisions repo's branch `fix/plan-gate-truncation-and-criteria-index` at **bc5f370** (both copies, 4 lines) — the branch also carries the prior criteria-index fix; the two untracked docs/architecture/system.{drawio,svg} files there are not mine, left alone. **21-criteria activation run** (after appending C20 + a `## Cleanup ledger` section): 9 kept sections, coverage 21/21, same lone Approach-intro flag; the ledger section itself read CONFORMS on arch-gate. Appending criteria + appending a whole skipped/covered section both preserved identity — re-run gates after every append.
