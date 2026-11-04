---
name: shared-file-commits-can-sweep-another-sessions-work
description: "a parallel session's uncommitted changes in a co-edited file ride along in your commit — `git add src/agent.rs` swept the dogfooding session's hunks into ba08055; verify the staged hunks are all yours before committing"
type: feedback
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

On 2026-10-16 a parallel dogfooding session held uncommitted work in
`src/agent.rs`, `src/nomenclature.rs` and `docs/known-issues.md`. Committing
the window-aware compaction change with `git add src/agent.rs` swept their
hunks into my commit — the diff stat showed it (375 insertions where my
change was ~50 lines), but the message named only my work, so their changes
landed in history under a stranger's message.

**Why:** explicit-path commits are the house rule, but an explicit path to a
file another session is editing is not enough — the path is right and the
*hunks* are not all yours. Misattribution is invisible in `git log` and
misleads the next reader about whose work a commit is.

**How to apply:** before committing a co-edited file, read
`git diff --cached --stat` and confirm the size matches your change; if any
hunk is not yours, back up the working-tree file, take the parent's version
with `git show HEAD~1:<file>`, re-apply only your own hunks (lift your test
hunks verbatim out of the backup by line range so they stay byte-identical),
amend, then restore the backup. The gate passes either way, so the only
casualty is attribution. Related: [[fleet-dogfooding-viability-verdict]],
[[fleet-memory-corpus-dirties-the-tree]].

**A co-edited tree can also refuse to compile at all** (2026-10-20). The same
working tree held another session's in-progress `src/roles.rs`, `settings.rs`,
`lanes.rs` and `app.js` hunks; at one point `cargo test` failed on *their* file
(an `\u2014` written where Rust needs `\u{2014}`), then on a shadowed `catalog`
binding in their test. Neither error was mine, and the tree's state moved under
me between commands. The move: do not edit, revert, or "fix" their file — and
do not report the red leg as yours. Verify your own module in a throwaway
detached worktree (`git worktree add /tmp/x HEAD --detach`, copy your files in,
build there), then retry the main tree when their edit settles. Commit nothing
across a moving tree: keep your files separable (new modules, one doc) and say
plainly in the recap that the work is uncommitted and why.

**Hunk-level staging is the working answer (2026-10-20).** A parallel session
held a second-brain plugin WIP in the *same* files I was shipping
(`app.js`, `bridge.rs`, `server.rs`, `app.css`, plus untracked `src/plugins/`
and a `lib.rs` line). Splitting with `git apply --cached` per hunk worked, but
two hunks were **interleaved** (my `access:` face fields next to their
`plugin:` fields in one diff hunk), so hunk selection alone was not enough.
The sequence that shipped cleanly: generate per-hunk patches with a small
python splitter (`re.split(r'(?m)^(?=@@ )', diff)`), pick the hunks, repair
any interleaved hunk by rewriting the staged blob directly
(`git show :path` → delete their lines → `git hash-object -w` →
`git update-index --cacheinfo mode,sha,path`) — the index accepts a blob the
working tree never had. Then prove it: `git diff --cached | grep -i
<their-words>` and compile the **staged tree alone** in a detached worktree
(`git worktree add --detach /tmp/x HEAD; git apply <staged patch>; cargo
check`) so a hidden dependency on their untracked module fails there, not in
the commit. Say in the commit body that the other WIP is excluded, and in the
recap that it is still uncommitted.

**Keep following own-hooks out of the diff (2026-10-21).** In the same
tandem arrangement, my commit's body had one line mangled — a backticked
`` `.env` `` at the start of a line became a shell command substitution, so
`zsh: no such file or directory` appeared and the sentence lost its phrase
("The  section marks each credential…"). A multi-line commit message
written through a shell here-doc/double-quoted `-m` is running text through
the shell. Use a single-quoted heredoc (`git commit -F - <<'EOF' … EOF`) or
a message file when the body contains backticks, `$(`, or names like
`.env`; and read `git log -1 --format=%B` after committing. The commit was
already pushed, so the repair is a follow-up note or a reword on a branch —
not a force-push on master.

**A moving tree can also cost a full gate run for nothing (2026-10-20).** While
another session edited `src/roles.rs`/`settings.rs`, `cargo test --lib`
compiled their broken file and my module's tests could not run at all — the
plugin's tests passed only in the detached worktree. The cheaper order: run the
few tests that cover your own new module (`cargo test --lib <module>::`) and
`cargo build --lib` first; full-suite and gate runs wait until the tree settles.

**The clean ending (2026-10-20, same day as the interleaved-hunk fight).** The
parallel session committing its own work as `adfd353` closed the problem without
any staging surgery: it staged only its hunks, wrote in the commit body "the
working tree also holds an unrelated second-brain plugin WIP; it is not in this
commit", and after that `git status` listed only my files. Check `git log` and
`git status --short` after the other session reports done — if their commit
names your WIP as excluded, the split held and a plain `git add <your paths>`
is now safe. Verify anyway (`git show <their sha> | grep -c <your feature>` came
back 0) before trusting it.

**The untracked case is worse, because it reddens the gate for everyone.** On
2026-10-17 another session's untracked `src/vault.rs` (an AES-GCM
implementation, registered through a modified `src/lib.rs`) broke the whole
gate on two legs — `cargo fmt` wanted `pub mod vault;` in alphabetical order,
and `vault::tests::gcm_matches_a_reference_implementation` failed against a
reference vector. Neither diagnostic was in any file I had touched; clippy,
build and 992 tests were green on my side. So before diagnosing a red leg as
yours, run `git status --short` and look for `??` entries: an untracked file is
someone's in-flight work, and fixing it is theirs. Leave it alone and say so.

**The race is real-time, not just concurrent (2026-10-20).** Executing
"commit and push all changes" on a tree a tandem session was actively writing:
the tracked work I was inspecting got committed AND pushed by the other
session between my two status reads (it landed the bot-interface plan in
`1d910f4` seconds before my `git add` reached it, so my commit shrank to
gitignore-only and its first message overclaimed — amended before push, which
is safe while unpushed); and three NEW files (`system.drawio`, `system.svg`,
a `bot-pwa.md` plan) appeared mid-command, mid-write. The order that held:
re-read `git status --short` immediately before every `add`, never sweep
another live session's just-appeared files (they may be half-written — leave
them for that session's own commit and say so), verify what the commit
actually carries with `git show --stat` before writing the message, and
`git push` only after reading the ahead/behind line.

**The same split, one commit later, left master unbuildable (2026-10-23).**
`70dc3bf` took `src/agent.rs`'s new `goal` field and `src/gui/bridge.rs`'s read
of `snapshot.retired`, but left the matching hunks in the worktree — five
`goal: None` construction sites plus `retired` on `SkillsSnapshot` — so the
committed tree never compiled while the worktree did. The green gate was
honest: it ran on the worktree, not on what was committed. The owner found it
as "I was dogfooding with fleet and it made a mess."

**The per-file staged blob, on a tree moving in real time (2026-10-25).**
The judgment-beats commit shared four files with a LayerFS session's live
WIP (`calib.rs`, `subagents.rs`, `delegate/tests.rs`, `tests/integration.rs`).
The sequence that shipped only my work: reverse THEIR hunks on a copy (`cp`
the file, build a theirs-only patch from the hunk headers, `patch -R -p1` the
copy), `git hash-object -w` the copy, `git update-index --cacheinfo` the blob,
then compile the staged tree alone (`git write-tree` → `git commit-tree` →
`git worktree add --detach`) — which immediately caught that my staged
`delegate/tests.rs` referenced their `DelegatedRunRow` fields the staged
`calib.rs` didn't have. One more gotcha from the same tree: `cargo test --lib`
does NOT build the `mock-mcp` bin the scripted lanes need, so a worktree run
manufactured thirty phantom failures until `cargo build --bin mock-mcp` ran.
And a file can be half-mine in a way `git diff` hides: their commit landed
between my status reads, so re-check every shared file's diff immediately
before staging.

**The reverse transform, when YOUR edits landed in a foreign context
(2026-10-28).** The chat-review commit held uncommitted foreign work in the
SAME two files, and my edits touched one of their regions: their uncommitted
`resumed` arm sat inside `showSessionObject`'s summary, which my exchange
count also rewrote. Reversing their hunks was not enough — the staged blob
had to be the worktree MINUS their two hunks (the `STATE_LABELS resumed`
line, dropped with a one-line replace) PLUS a reshape of the summary back to
HEAD's structure carrying MY line (their arm lifted out, the indentation
restored): blob = transforms(worktree), while every other co-edited file
(bridge.rs, known-issues.md) went the other way — HEAD + only my edits,
applied as python pairs with `count == 1` assertions. Both directions are
covered by one verification habit: after staging, `git diff --cached` must be
ALL yours and `git diff` (unstaged) must be ALL foreign — the two reviews
complement each other, and a straggler hunk shows in whichever one it
landed in. Then `git rev-parse <sha>^{tree}` == the gate's write-tree hash
proves the gated tree is the committed tree.

**How to apply:** the staged-tree compile is not optional, and checking out
HEAD is not a substitute for it: `git worktree add /tmp/x HEAD`, `cargo check
--all-targets` **there**, and require that it pass before writing the message.

**The sweep can land while your gate is still running — and their commit
fails its own tests (2026-10-27).** I had staged six C5 files
(`layerfs-live.sh`, four docs, the plan) and was building the detached
verification worktree when the parallel session's `git commit` swept
them into its own `feat(bot-pwa)` commit — same mechanism, faster
cycle: **anything sitting in the index when another session commits
gets committed, whatever the message says.** The repair that held:
verify the swept content is intact (`git diff HEAD -- <paths>` empty =
in), do NOT rewrite their published commit, name the split plainly in
your follow-up's body, and carry the rest yourself. The same session
also left `delegate::`/`judge::`/`explain::` tests RED **at their own
HEAD** — proven by running one in a worktree at their commit — so the
honest gate statement was my staged tree green *minus their named WIP*
(fmt 0, clippy 0, lib 1450/0 with `--skip delegate:: --skip judge::
--skip explain::`, face 252/0), with the exclusion named rather than
either claiming full green or calling their red mine. Rule of thumb for
an index on a shared tree: stage, build the verification worktree, and
commit in ONE window — the longer the index sits staged, the likelier
another session's `git commit` carries it.
A struct gains a field in one file and every construction site breaks, so a
field added by one hunk is exactly the shape that survives partial staging.
Tell-tale worth checking first: the mtimes. Here the repair hunks were on disk
at 20:36–20:49 and the "final" commit landed 21:26 — a file modified *before*
the commit that is still dirty afterwards was left out of it. And run
`cargo check` in whatever tree you inherit before promising a gate result:
`git log` says nothing about whether a commit builds.

**The same partial-paths mistake recurred inside the hour (2026-10-23).** a
`feat(work):` commit added three `SessionSummary` fields but named only the
source paths, leaving six test fixtures (three files) behind — the committed
tree did not compile while the worktree's gate was green. Caught before the
push by `git diff HEAD -- src tests` returning a non-empty diff. **That check —
not a longer list of paths — is the corrected habit:** explicit paths are a
guess about which files a change reaches, and a new struct field reaches every
literal. Recorded as KI-218's second sighting in the ledger (its first was 23
`AgentOptions` constructors in 2026-10-17). Keep test fixtures in the same
commit as the field they satisfy, and run the diff check before claiming green.

**The repair lands as its own increment (outcome, 2026-10-23).** The leftover
hunks were seven files and 43 lines — five `goal: None` sites, `retired` on
`SkillsSnapshot`, the test literals — committed as `fix(build): …` with the
evidence in the body (what a HEAD checkout reported, and that the gate figures
came from this tree, not the broken commit). Sorting the rest split cleanly by
*why* the file was dirty: build repair, ledger corrections, memory
bookkeeping, and cron-written corpus snapshots (ignored, not committed) — one
commit each, because folding bookkeeping into a feature commit is what made
the original knot hard to read. The committed tree then matched the gated tree
exactly (`git diff HEAD -- src tests` empty), which is the claim worth making
when you finally say "green".
