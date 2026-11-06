---
name: verify-the-binary-is-newer-than-the-source
description: "feedback 2026-10-18/20/30 — when a fix \"doesn't take,\" check the artifact before theorizing (shared CARGO_TARGET_DIR, an asserting patch script, the ~/.local/bin symlink, a long-running gui serving its launch build, stale SPA tabs); 2026-10-30 added the sharpest form — the face ran the PREVIOUS build for 20+ minutes because every rebuild command was CANCELLED before the restart, so an interrupted build+restart ships nothing and only the served bytes / a visible build stamp prove otherwise"
type: feedback
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The remote-access work ran in a git worktree on its own branch while a
second session worked the KI ledger in the main tree. Both gates were
pointed at the same `CARGO_TARGET_DIR` to reuse the build cache — and
both produce a binary named `fleet`, so they overwrote each other's at
one path. Twice I "verified" a fix by running the binary and saw the old
behavior; twice I went on to add debug prints, restructure code, and
re-measure before noticing the binary predated the source I had just
edited.

**Second form:** a patch script with an assert in the middle exited
*before* its write, so the file on disk was untouched while the build
stayed green and the fix "applied". After any scripted or generated
edit, read the change back before drawing a conclusion from a green
build.

**Third form:** `~/.local/bin/fleet` is a symlink to the **main tree's**
`target/release/fleet`, so the owner got `unknown gui option` for a flag
that existed on a branch. For any "missing flag" report, check *which
binary the shell resolved* first.

**Forms four–eleven** (2026-10-20 → 10-29), each seen live: a running
`fleet gui` serves the assets compiled into the binary it launched
with; `ps` alone mis-maps the face when a test binary shares the name
(identify by `lsof -nP -iTCP:3080 -sTCP:LISTEN`); "rebuilt" ≠ "contains
HEAD" when a parallel session pushed after the build; a KeepAlive
LaunchAgent restarts it for you; `kill` races the supervisor; a tab
loaded before a restart runs OLD JS against the NEW API and looks
stuck; and an instrumented verification must rebuild before launching
the server it drives.

**The thirteenth form, 2026-11-02 (late), is the quietest one — a face that
was never stale from a cancel but from a whole day of GUI work landing
without a rebuild.** The LaunchAgent's binary was built at 18:16 from
`3ebf1fe`; every later commit that day carried GUI changes, and the face
served the pre-day build for **2.5 hours**. The tell was two measurements
against each other: the served `app.js` had **zero** occurrences of a symbol
the new source carried, and its byte count (950,592) was **less than the
source's** (961,837). The fix — merge the master worktree, build there,
`launchctl kickstart -k`, then re-grep the served bytes (36 hits, byte
count exactly the source's) — is the same recipe as
[[fleet-gui-is-a-launchagent]]'s closeout reload. **How to apply:** when the
owner asks about the app's delivery state, the served-bytes comparison is
the one check that cannot be faked by a green gate or a running process:
`curl -s 127.0.0.1:3080/app.js | grep -c <a symbol only the new source
carries>` and the byte count against `wc -c src/gui/assets/app.js`. A
symbol count of 0 with a smaller byte count is a stale face, full stop.

**The twelfth form, 2026-10-29/30, is the one that cost the evening —
an INTERRUPTED rebuild ships nothing, and I argued past it.** The owner
repeatedly reported the bots page had no pairing ("was the actual code
ever built and made available on port 3080", "I used localhost:3080 and
I'm not dense", "so the disk can say jack shit, if it doesn't load on
the browser then it's broken"). I proved the served `app.js` carried
the strings — twice — and each proof was TRUE AT THE MOMENT and then
falsified by the next edit: after changing the surface I started
`cargo build --release && launchctl kickstart`, the command was
cancelled mid-build twice in a row, and because the `kickstart` sat
**after** the build in the same `&&` chain the restart never ran — the
face kept serving the 20:05 build for 20+ minutes while I talked about
a placement fix that was nowhere near the screen (`ps -p <pid> -o
lstart` 20:07 vs the binary's mtime was the tell I should have re-read).
The answer that finally worked was the owner's own: **the browser is
the arbiter** — build and restart in a BACKGROUND task so it cannot be
cancelled, then render the exact origin (`localhost:3080`, the one he
installed from) at a short viewport and read the PNG.

**The flip side, same evening (KI-371, `950f52d`): once the artifact
question is settled, a session can hide a real defect behind it.** After
the interrupted-build evening every check was "is the binary new" — and
when the browser then showed a real failure (pairing succeeded, every
read 400'd, then the face went blank with NO failed request) the
instinct was still to suspect the artifact. The defect was in the
product: a bodyless GET marked sealed, and a sealed body read as raw
bytes instead of base64 TEXT. **How to apply:** use artifact checks to
bound the problem ("the code IS on screen"), then switch modes and
debug the product on its own terms — a mount-and-read repro in the real
browser with the failure bodies captured is the next instrument, not
another build stamp
([[ki-371-the-sealed-frame-boundary]]).

**Why:** a stale binary, an unwritten patch, or a lost restart produces
the exact symptom of an ineffective fix, so every debugging technique
that would have worked on a real bug aims at a phantom — and the
owner's confidence is spent on your proof of a build that is no longer
on screen.

**How to apply:** (1) never chain `build && restart` so a cancel loses
the restart — restart explicitly, then confirm; (2) after every restart
re-grep the SERVED bytes and re-render, because the previous proof does
not survive a new edit; (3) ship a **visible build stamp** (this
session's `<read_at> · build <hash>` line on the bots strip) so the
owner can answer "is this the new code" without you; (4) treat the
owner's "it's not there" as evidence about the screen, not as a claim
to refute — change the screen and show the render. Related:
[[fleet-gui-is-a-launchagent]], [[fleet-gate-exit-code-trap]],
[[the-screenshot-is-the-visual-review]].
