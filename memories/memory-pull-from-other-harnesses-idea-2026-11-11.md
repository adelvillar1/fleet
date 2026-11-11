---
name: memory-pull-from-other-harnesses-idea-2026-11-11
description: "project 2026-11-11 — the cross-harness memory PULL (inverse of the zcode→fleet push transplant) is IMPLEMENTED, VERIFIED by the parent (own four-leg gate fmt/clippy/build/test=0, 2190 lib passed; independent live pull imports + dedup + cursor-unchanged + armada-stub all correct), plan COMPLETED, COMMITTED 2907d19 + pushed to origin/master, tree clean; docs/plans/2026-11-11-cross-harness-memory-pull.md; owner ruled BUILD NOW before armada; IMPORT not federated (portability pushback); bootstrap + commit-curso…"
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

The owner's idea (2026-11-11, while waiting on the v0.6.29 cut): "a few days
ago we did a memories 'transplant' from zcode to fleet as a push. Could we
invert the pattern where fleet picks (pull) up memories from other installed
harnesses into projects that are being worked on on fleet?" The push
transplant is [[memory-transplant-plan-drafted]] /
[[memory-transplant-zcode-to-fleet-proposed-2026-11-04]]; this is its inverse
— ongoing pull, not one-time migration. IDEATION, no plan drafted, no owner
ruling yet beyond the ask itself.

My framing (a proposal, NOT an owner ruling): fleet already has the exact
pattern — `session_recall.rs` reads its own sessions as SYNTHETIC FACTS
through the same six-channel ranker without writing them into the corpus,
and the dream/consolidation pass adopts the useful ones into durable facts.
The pull widens the source from "fleet's own sessions" to "other installed
harnesses' memories." My lean is FEDERATED RECALL, not bulk import: read
live and project-scoped (always fresh, no stale copy); provenance names the
source harness (`source: zcode`, `basis: Inferred` — half-strength like a
session fact); adopt-on-demand via the consolidation/dream pass with
recall-count feedback earning a foreign fact its place; the substrate's
supersede/dedup handles a rule living in two harnesses (freshness + owner
rulings decide the winner). Discovery precedent: the `mcp_registry` scan
already finds foreign harness configs. Two boundaries: KNOWLEDGE ONLY — the
own-home ruling holds ([[own-home-credentials-ruling]]), the pull reads
memories never another harness's keys/config (the transplant already
established knowledge may cross harness lines); and BOUNDED — foreign-store
reads ride the recall budget, project-scoped and capped, not every store
every recall.

THE CONNECTION I named: this is armada's shared corpus in miniature —
armada federates memory across fleet INSTALLS, this across HARNESSES; the
same provenance/dedup/authority seams, so building the pull is a down
payment on armada's corpus layer ([[armada-multi-install-c2-idea-2026-11-11]]).

THE OPEN FORK I posed (unanswered): recall-time ONLY (foreign facts surface
when relevant, adopted selectively — fresher, lighter) vs also a SYNC
(periodic import of a project's foreign facts into the substrate — durable,
offline-capable); adopt-on-demand gives most of both, but which emphasis
matches how the owner would use it is theirs to say. Do not treat any of
this as decided until the owner rules.

THE OWNER'S PUSHBACK — A RULING (2026-11-11, same ideation): "I will push
back on federated, because what happens when armada is built and a fleet is
working on an existing project on an instance that has none, the federated
recall wouldn't work there." Federated recall reads LOCAL foreign stores
live, so it silently depends on zcode/hermes being installed on that
machine; a fresh armada instance working an existing project has fleet and
nothing else — no local store to federate over, so recall comes back empty
exactly when the memories are needed. THE PORTABILITY TEST DEFEATS
FEDERATED AS THE FOUNDATION. My federated lean above is OVERTURNED.

My reframing (proposal around the owner's ruling): the foundation must be
IMPORT INTO THE SUBSTRATE — the durable, git-tracked corpus — because THAT
is the thing armada syncs; memories must live in the substrate to travel,
a live read cannot cross a machine. (1) The pull is the transplant made
ongoing and project-scoped: fleet ← any installed harness, into the
substrate as durable facts, provenance-tagged (source: zcode, basis:
Inferred), deduped against what fleet holds, conflicts settled by freshness
+ owner rulings; knowledge only, own-home holds. (2) Imported facts PERSIST
when the source is absent — once imported a fact is fleet's own, so a fresh
instance with no zcode still has it (in the corpus, not behind a live read).
(3) Armada's corpus sync carries it everywhere: a memory born in zcode on
machine A → (pull/import) → fleet substrate → (armada git-sync) → a fleet
instance on machine B working the same project with no zcode in sight; the
substrate is the portable unit, federation never was. (4) Re-sync handles
FRESHNESS not presence: re-pull when a foreign store is available to update
imported facts; when absent the last imported version stands — staleness is
a re-syncable freshness problem, not an unfixable absence problem.
Federated recall could ride ON TOP as a local freshness optimization (read
live when the store happens to be present) but it is a garnish, not the
foundation. THE TIGHTENED ARMADA CONNECTION: the pull is the CROSS-HARNESS
FEEDER into the same shared corpus armada syncs CROSS-INSTALL — one corpus,
two feeders (harnesses feed locally, armada distributes globally), shared
provenance/dedup seams ([[armada-multi-install-c2-idea-2026-11-11]]). NEW
OPEN FORK I posed (unanswered): PROJECT-TRIGGERED import (fleet imports a
project's foreign facts when it starts working that project — bounded,
relevant; my lean given the resource contract) vs CONTINUOUS (a background
pass keeps the substrate fed from every installed harness — fresher but
reads stores for projects never touched). Still ideation, no plan drafted.

THE OWNER'S TRIGGER RULING (2026-11-11, same ideation): "I think it should
be a couple of things a one-time pull of all the project facts on the local
instance, and the only when new commits happen on a project on a foreign
harness or foreign instance." This ANSWERS the project-triggered-vs-
continuous fork with a hybrid: (1) a ONE-TIME BOOTSTRAP — when fleet first
takes up a project, pull ALL that project's facts from the local foreign
stores into the substrate (the transplant, project-scoped, done once);
(2) then COMMIT-TRIGGERED DELTAS only — no continuous polling. THE COMMIT
IS THE CURSOR: each foreign source keeps a last-pulled commit per project;
a new commit pulls only the changed facts since the cursor and advances it.
This is git-sync semantics at project scope — native because the substrate
is git-tracked, and exactly what armada's corpus speaks. THE UNIFICATION
the owner's "foreign harness OR foreign instance" names: a local zcode
commit and a remote armada-instance commit are the SAME event (a new commit
on the project's corpus), so ONE mechanism serves both feeders — bootstrap
feeds locally, armada git-sync propagates globally, the cursor pulls the
delta wherever it originated.

My framing around the ruling (proposal, NOT yet ruled): two things to
settle — (1) commit DETECTION must be READ-ONLY (own-home: fleet must not
install hooks INTO a foreign harness's config): a read-only watch on the
foreign store's git HEAD, or a lazy rev-parse when fleet touches the project
(cheap, bounded, no daemon), and for remote instances the armada git fetch
itself reveals new commits; my lean is check-on-touch + the armada fetch
(bounded, no background watcher, fresh exactly when fleet works the
project). (2) The DELTA is add/update/supersede/DELETE — updates ride the
substrate's supersede semantics; a deletion in the source should ARCHIVE
fleet's imported copy (preserve provenance), not hard-delete, since an
imported fact is fleet's own once adopted. I offered to write the pull into
the armada direction note as the corpus's FEEDER section (bootstrap +
commit-cursor + the two sources, detection choice left an open fork); the
owner has not answered. Still ideation, no plan drafted.

THE OWNER'S BUILD-NOW RULING (2026-11-11): "I think this is something that we
can do now even without armada being scoped, because this allows me to test
the sync from the foreign harness point of view, and we just prepare the
structure for the foreign instance use case." This moves the pull from
ideation to BUILD: implement the cross-harness feeder NOW (local-harness
side), with the source abstraction preparing the armada remote-instance
feeder so it plugs in later without rework. No longer waiting on armada.

PLAN DRAFTED + GATED + ACTIVE (2026-11-11):
docs/plans/2026-11-11-cross-harness-memory-pull.md (status active). plan-gate
PASS via drex (6/6 covered, 1/1 in scope); arch-gate PASS with 8 UNDOCUMENTED
conventions gaps (the new feeder area, zero conflicts). Six criteria: C0
read-only discovery (names stores + project facts, absent named, nothing
foreign written); C1 bootstrap import (provenance-tagged source/basis
Inferred, type-mapped, deduped); C2 cursor delta (only changed facts, cursor
advances, unchanged imports nothing); C3 import semantics (update supersedes,
delete-at-source ARCHIVES not hard-deletes); C4 source abstraction
(LocalHarness works, RemoteInstance named stub, generic revision cursor); C5
the `fleet memories --pull [--project <name>] [--source <harness>]` verb +
four-leg gate. Implementation DELEGATED to a background general-purpose agent
(new src/memory_pull.rs feeder + Source trait + cursor sidecar + --pull verb +
tests over scripted foreign stores); the parent retains verification (re-run
gate + a LIVE pull against the real zcode store) before committing. NOT
assumed a release cut — owner says the word for 0.6.30, else lands on master.

Grounding captured for the build: fleet Fact (memory.rs:269, basis enum:191,
source:317), the Remember writer (~4216, reuse its validation, never bypass),
mcp_registry::scan (~631, the READ-ONLY "reads them and never writes them"
discovery precedent). zcode store layout:
~/.zcode/cli/memories/projects/<slug>/memory/*.md (slug=<name>-<hash>, e.g.
fleet-e2ba136ae3eba04b), frontmatter name/description/metadata.type
(user|feedback|project|reference), MEMORY.md is an index (skip), some filenames
carry a "[type] " prefix. The cursor is a GENERIC revision (git HEAD where
tracked, else a STABLE sorted/normalized content hash) because zcode's store
likely is NOT git-tracked — so a non-git local store still syncs and armada's
git commits plug in unchanged. Detection lean: check-on-touch (lazy rev-parse
when fleet works the project) + the armada fetch later — bounded, no daemon,
read-only (own-home: never install hooks INTO a foreign harness).

BUILD COMPLETE — delegated agent report (2026-11-11, parent verification in
flight): new src/memory_pull.rs (~1659 lines incl. 11 tests) — Source trait
(harness/project/read → Snapshot{revision,facts,skips}), LocalHarness zcode
reader (nested-metadata frontmatter mapper, folded-description join, slug
discovery), RemoteInstance armada stub (PullError::NotYetBuilt),
store_revision (git HEAD or stable content hash), cursor sidecar
memory-pull-cursors.json (per-project-per-source), pull through
Remember::parse/Memory::remember/Memory::supersede with name dedup +
archive-on-delete. Minimal pub(crate) seams in memory.rs only
(sanitize_description, archive_fact — no behavior change to existing paths);
lib.rs mod; cli.rs MemoriesOptions pull/pull_project/pull_source; main.rs
memories_pull_command + default_project (git-toplevel basename). Docs updated
(features/memory.md, TECHNICAL-DOCUMENTATION, FUNCTIONAL-SPECIFICATIONS, and
a two-feeders note + seams row in the armada direction note §3/§4 — the
agent left the parent's concurrent §7–§12 rewrite untouched). FOUR-LEG GATE
GREEN per the agent: fmt/clippy/build/test all 0, 2190 lib passed 0 failed.
LIVE SMOKE (scratch corpus, repo memories/ untouched): `fleet memories --pull
--project fleet --source zcode` imported 558 facts (1 named within-batch
collision), second pull correctly UNCHANGED, absent project named exit 1,
--source armada named stub exit 1, foreign store verified clean. THE REAL
ZCODE STORE IS NOT GIT-TRACKED (git rev-parse fails) → cursor basis is the
stable content hash (content:<sha256>, sorted filenames, CRLF folded,
trailing whitespace trimmed); slug rule confirmed
<lowercased-name>-<sha256(project-path)[..16]>. DESIGN DEVIATION: change
detection is a CONTENT-DIGEST against the substrate (writer-normalized both
sides), not a git diff between revisions — the revision is the trigger, each
fact compared to the held copy so only changed/new/deleted import; works
uniformly for git and non-git stores. Update provenance: supersede inherits
the old source pointer, the feeder re-stamps the replacement with the new
revision (a failed stamp named beside the landed supersession). Two
robustness fixes found via the smoke + pinned: name truncation no longer
overshoots the 64-char bound by a joining hyphen; within-batch foreign-name
collisions are named skips. The agent reverted the test side-effects
(memories recall-counts, events.jsonl) to HEAD so the tree carries only the
feature diff. STATE: UNCOMMITTED — the parent is re-running the four-leg gate
itself + an independent live pull before committing; NO release cut assumed
(owner says the word for 0.6.30, else lands on master). Tower report filed
t-20261111-175644-bdcd (done, no SHA).

PARENT VERIFICATION COMPLETE + COMMITTED (2026-11-11): the parent re-ran the
four-leg gate ITSELF (fmt/clippy/build/test all 0, 2190 lib passed 0 failed)
and an INDEPENDENT live pull — first pull imported the zcode fleet facts with
named within-batch + native collisions, second pull read "unchanged since the
last pull", absent project + --source armada named (the stub). Plan marked
COMPLETED (C0–C5 ticked with evidence). COMMITTED 2907d19 (the pull feature:
src/memory_pull.rs + memory.rs seams + lib/cli/main + features/memory.md +
TECHNICAL-DOCUMENTATION + FUNCTIONAL-SPECIFICATIONS + the plan) and PUSHED to
origin/master; the README + armada direction note committed separately at
9d9b6de. Tree clean. NO release cut (0.6.30 only on the owner's word).

THE VERIFICATION GOTCHA (a real trap, links [[fleet-home-does-not-move-the-memory-corpus]]):
the pull resolves the memory root via resolve_root(current_dir()) (main.rs),
so running `fleet memories --pull` FROM THE REPO imports into the REPO's
memories/ corpus regardless of FLEET_HOME (FLEET_HOME only redirects the cursor
sidecar + sessions, NOT the corpus). The parent's verification pull therefore
dirtied the real corpus — 124 untracked imported facts (source: zcode:fleet:
content:<sha>, basis inferred) + 16 modified recall-count bumps from the gate's
cargo test. REVERTED precisely: `git checkout -- memories/ events.jsonl` for
the tracked bumps, then removed ONLY untracked memories/ files carrying
`^source: zcode:` (the imports), restoring the corpus to HEAD. LESSON: to
verify a pull without touching the real corpus, run it from a SCRATCH cwd (so
resolve_root falls back to the scratch home), not from the repo; and the
real-corpus pull is the OWNER's deliberate call, never a verification side
effect.
