---
name: uncommitted-state-classify-then-commit
description: "feedback 2026-10-30 — owner instruction mid-turn, \"whatever is uncommitted should be committed\": classify before committing — real artifacts land in their named homes, harness debris moves to tmp/ (never deleted), .lock files are runtime state per the KI-062 precedent — and the tree ends clean without committing \"hello"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 55
superseded-by:
valid-until:
---

Owner instruction 2026-10-30, sent mid-turn while I hesitated over a
dirty tree with unknown-author files: *"there are no sessions stepping
into your work, whatever is uncommitted should be committed."* The
instruction's intent is a clean tree with nothing left hanging — not
literal wholesale `git add -A`.

**The classification that worked** (landed as af7c360 + 7b96232):

- **Real artifacts commit to their named homes** — a plan draft into
  `docs/plans/`, an architecture sketch into `docs/architecture/`.
- **Harness debris relocates, never deletes**: the dated
  blend1-refresh retarget, screenshot outputs, `.tmp-corpus-refresh/`,
  a `home.txt` ("hello"), a scraped-HTML page misnamed `.json`, and a
  `.bak-resume` moved to `tmp/stray-YYYYMMDD/` — the gitignore's own
  comment names tmp/ as "harness-side noise, never repo content".
  Everything stays recoverable on disk.
- **`.lock` files are runtime state, ignored not cleaned**: the
  KI-062 precedent ("unlinking a flocked file another process holds is
  a race") extends to docs — `/docs/**/*.lock` joined
  `/memories/*.lock` in the gitignore.
- **Debris recurs**: the delegation tests' fixture path
  (`"change home.txt"`) resolves against the process CWD, so every
  full-suite run drops a fresh `home.txt` at the repo root — it got
  its own gitignore rule (`/home.txt`, `/home.txt.lock`) instead of a
  third relocation.
- Report the judgment call ("if you'd rather they were committed
  as-is, say so") — the owner can override, but the repo's own hygiene
  law decides the default.
