# Changelog

All notable changes to fleet land here, one section per release, written at
cut time. The repo was born 2026-10-12; this changelog starts at v0.4.0 with
the first release — everything before it is `git log` (492 commits, each one
through the local gate).

**What beta means while the beta epoch runs:** this is the tool the owner
uses daily, not a promise of API stability. The tag is a real version
(`v0.4.0`); the version literal keeps its beta suffix (`0.4.0-beta.1`) until
the owner says otherwise, and the changelog header carries this paragraph
until then.

## v0.4.0 — 2026-11-06

The first GitHub release. The `0.3.0-beta` literal had been parked since the
repo's first weeks; the work below closed that out, so the release takes the
next stable minor number. Binaries: the native shell as a dmg and the CLI as
a tarball, both built from the tagged tree after the gate ran on it.

### Fixed — the open ledger, closed by class

The known-issues ledger went from **28 open rows to 5** at the closeout's
end (465 rows total); every row was re-verified against the code before it
was worked on, and one (KI-326) proved already fixed. The evening's own
scheduled corpus-refresh then found the KI-394 refusal shape a second time
and **KI-466** was filed, open — so this release's tree carries 6 open
rows, the new row naming its own fix-side choice rather than hiding the
regression.

- **KI-463** — a related-memories request built the whole embedding corpus
  synchronously on the request path (cold-corpus reads blocked the server);
  fixed so the request path reads the built caches and only the warm path
  builds, pinned by a source-shape test.
- **KI-464** — the telegram wait test waited on half its own assertion; the
  wait now watches both observables it asserts on.
- **KI-465** — a gate procedure that kept the summary and destroyed the
  failing names (`tail | grep`); the rule is now `> /tmp/<leg>.log 2>&1`
  with the exit read on the file — the release script's own gate runs this
  way.
- The nine classes the plan landed in one sweep: the nomenclature write
  guard, load-sensitive fixtures, the supersede/remember file routes,
  `reload` + delivery steps, child reports, account-state 400 failover, and
  plan hooks.

### Added

- **Releases.** `scripts/release.sh` gates HEAD on a detached worktree
  before it tags and cuts; `scripts/bump-version.sh` moves every version
  site in one step; this changelog is the release note, in one place.
- The first dmg deliverable, cut from the gated tree.

### Verification

The gate ran green on the committed release tree: **1,929 lib + 168
integration tests + 962 face-harness checks, 0 build warnings**, the ledger
and plan checker at exit 0.

### Install

See [the releases page](https://github.com/adelvillar1/fleet/releases/latest)
for the dmg and the CLI tarball, or
[build from the repo](https://github.com/adelvillar1/fleet#install).
