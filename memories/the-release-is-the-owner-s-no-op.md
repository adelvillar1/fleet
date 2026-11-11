---
name: the-release-is-the-owner-s-no-op
description: "feedback 2026-11-09 — the release is one chain (tag mode gates a worktree and refuses a dirty tree; assets mode needs HEAD == the tag, and a co-tenant commit after the tag burns the cut; the cut reads its own feed back); each cut is ~10 min; never pipe the bump, prerelease an empty release immediately; and a pre-build disk preflight now refuses to start under 20 GiB free"
type: feedback
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

Feedback 2026-11-09 — **eleven releases in one session** (0.6.5 → 0.6.19)
through `scripts/release.sh`:

- **Tag mode** refuses a dirty tree and gates `HEAD` on a detached worktree.
  Commit or stash the `memories/` recall-count churn and untracked recaps first.
- **Assets mode** demands `HEAD == vX.Y.Z` — and in a shared repo a **co-tenant
  commit landing after the tag burns the cut**: the assets step refuses, the tag
  has already created an EMPTY release, GitHub makes it `latest`, and the feed
  404s until `gh release edit <tag> --prerelease` (KI-503, 0.6.18). Tag and
  assets in one breath; the moment an empty release exists, prerelease it.
- **The cut is its own proof** — it reads `latest.json` back anonymously and
  hashes the served artifact. A cut ending on that line is published.
- **Each cut is ~10 minutes**; run both modes in the background and poll the
  log, never in the foreground.
- **Install with the updater itself** (`fleet update`), restart the face by
  killing its pid and relaunching (`~/.fleet/bin/fleet gui --no-open --remote
  --relay … --bot-relay …` — 3080 is that process, not a LaunchAgent you can
  kickstart), verify `/api/version`; replace a running binary by `rm` + `cp`
  (KI-491: exit 137 on an in-place `cp`).

- **A disk preflight runs before either mode builds** (`preflight_disk`, 0.6.20):
  it refuses a volume under `FLEET_DISK_FLOOR_GIB` (20) with the numbers, and
  `cargo clean`s the gate lane past `FLEET_GATE_TARGET_MAX_GIB` (20). The line
  `== disk: N GiB free (floor 20); …` heads every cut's log.
- **Dirty-tree refusals come from the substrate's own churn**: `memories/`
  recall counts and `2026/<day>/events.jsonl` rewrite themselves between
  commands, so a tree that was clean two minutes ago may refuse the tag.
  Commit them as `chore: the substrate's recall counts ride the X cut`, then tag.
- **A red gate leg leaves `.failed-build`** (leg, time, log path) in its lane so
  the debris survives the diagnosis and the sweep takes it after the grace
  ([[build-lanes-in-tmp-eat-the-drive]]).

**Two traps, each paid for:** never pipe `bump-version.sh`'s output (SIGPIPE
leaves version sites disagreeing — KI-500: the empty release held `latest`
and the feed 404'd); and deleting a burned release takes the tag
(`gh release delete <tag> --cleanup-tag --yes`), then poll the feed.

**Why it matters:** a defect found after shipping costs another full cut —
verify the LIVE surface before cutting, not only the harness.

**The 2026-11-10 correction (the owner's word: "0.6.22 was never pushed to
the github releases, make sure that this behavior is corrected"):** a version
bump is NOT delivered until `scripts/release.sh` has cut it — tag mode, then
assets mode, the feed read back. `fleet reload --from target/release/fleet`
plus a local dmg is a dev-time install, never the delivery: it bypasses the
GitHub channel the owner ruled the only one ([[updates-come-from-github-never-locally]]).
The order is bump (`scripts/bump-version.sh`, never hand-`sed` the three
sites) → commit → `release.sh tag` → `release.sh assets` → `fleet update` →
relaunch → verify `/api/version`. 0.6.22 and 0.6.23 were both "delivered"
by reload alone and had to be cut retroactively (0.6.22 from a worktree at
its own commit, held `--prerelease` through its build so `latest` stayed
true). **How to apply:** when the owner says "deliver", "ship", or "cut" —
or any time a version site changes — run the release script in the same
session, and say plainly if a cut is still pending. Never report a version
as delivered while `gh release view v<version>` is a 404.

[[updater-fix-landed-2026-11-09]] has the pipeline's shape;
[[build-lanes-in-tmp-eat-the-drive]] has the disk side of the same day.

**Field lessons from the 2026-11-10 cuts (0.6.22/0.6.23 retroactive,
0.6.24 through the corrected order):**

- **The changelog's head section must be the tag being cut** — tag mode
  refuses otherwise ("the changelog's head section is not $TAG"). Parallel
  sessions insert their sections into the same file, so a section anchored
  below theirs lands out of order and the head names THEIR cut. Fix:
  sort the `## v` sections descending by version (one python pass), commit,
  then cut. 0.6.22's first tag attempt burned exactly here.
- **Retroactive cut of an old version:** detached worktree at that version's
  own commit (`git worktree add --detach /tmp/fleet-cut-XXXX <sha>`), run
  tag+assets from INSIDE the worktree (its `REPO_ROOT` is the worktree; its
  changelog head is that version's), hold the release `--prerelease` through
  the build so `latest` stays true, flip `--prerelease=false` after.
- **GitHub's `latest` redirect LAGS minutes after upload** — the script's
  30s anonymous readback can fail even with everything uploaded ("the
  published feed does not name X after 30s"). Pin `gh release edit vX
  --latest`, wait, and re-poll `/releases/latest/download/latest.json`
  before believing the cut failed. Once it still named the previous
  release after flipping, a longer poll showed 0.6.24 within a minute.
- **A co-tenant can RE-CREATE a parked untracked file mid-cut** — stashing
  their draft before the tag is not durable; they rewrite it. Stash and
  launch the cut in the same breath; if the file returns before the assets
  check, retry the assets leg alone (the tag survives).
- **A retroactive release's own feed verifies by its own path** —
  `/releases/download/vX/latest.json`, not `/releases/latest/download/…`
  (which names whatever is Latest at that minute).
- The gate's detached-worktree run catches races a green working tree hides:
  0.6.24's first cut red on a test race (parallel health tests sharing one
  temp home keyed by pid) that two local greens had missed. Expect the gate
  to find things; budget a retry.
- **The cut now cleans its own lane** (KI-514, 2026-11-10): the settle step
  `rm -rf`s `$RELEASE_TARGET` on a green gate and after the assets upload —
  three cuts had accumulated 12 GiB there in one morning. The standing
  recovery is `scripts/cache-sweep.sh --gate-lane`.

**The tag pushes the TAG, not the branch (2026-11-11, owner: "make sure we
merge and leave the tree clean").** After the 0.6.28 + 0.6.29 cuts, local
master was TEN commits ahead of origin/master — every release had pushed
only its tag ref, so the work sat unmerged on the remote until an explicit
`git push origin master` (a fast-forward; verify with `git ls-remote origin
master` and an empty `git log origin/master..master`). A published release
is NOT a merged branch: after any cut, push master and confirm the tree is
clean (the running face keeps bumping `memories/` counters mid-cut — commit
the final churn so it finishes clean). Same for a sibling repo: merging its
fix branch to main locally is not done until `main == origin/main`.
