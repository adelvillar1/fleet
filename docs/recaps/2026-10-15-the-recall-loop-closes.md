# The recall loop closes — 2026-10-15

Status: complete — two commits, pushed (`e574db5`, `dc16f07`).

## What the session found

The recall machinery was never the gap. The vector path had been live
the whole time — llama-server serving embeddinggemma-2 at 768
dimensions on `127.0.0.1:8901`, `doctor` naming it, the embedder-keyed
index caching every fact's vector. The first probe of this session
checked llama-server's default port 8080 and concluded no loopback
embedder was running; that was the probe's error, not the machine's,
and the correction is recorded here rather than quietly dropped.

What was actually dark was the channel the owner's ruling made the
largest single term. The trajectory channel read 0.00 on every
CLI-written fact, because `fleet remember` spoke a subset of the
writer's shape and exposed neither `--phrases` nor `--missing`.

## What landed

**The direction channel became writable** (`e574db5`). `--phrases` and
`--missing` joined the CLI's write path, split on commas the way
`--shape` is, with the writer's own validation refusing a malformed
list rather than the surface truncating silently. Six durable facts
written with their resolution vocabulary.

**KI-058 re-measured on the current model.** Eight probe pairs through
the live embedder: unrelated pairs 0.5836–0.6648, related pairs
0.7673–0.9327. The unrelated floor of ~0.63 reproduces KI-058's 0.6049
within 0.03, which confirms the slice-19 reweighting on
embeddinggemma-2 — a larger vector weight would let proximity outrank
direction, the owner's ruling inverted. No weight change warranted;
the ruling is now evidenced rather than assumed.

**The phrase-writing piece** (`dc16f07`). When a `remember` call
supplies no phrases, the harness derives the direction from the
current task's own successful tool calls off this run's conversation:
a `bash` call contributes its command, a file call its path. Two
scopes keep the channel's meaning intact — the window is the calls
after the last user message (a fact about the router must not carry an
earlier task's phrases; that is proximity pollution), and only
succeeded calls count (the join is by call id, so "succeeded" is read
off the log rather than guessed). The model's own phrases always win,
and the confirmation names the derivation.

## Verified

Gate at 925 passed / 2 ignored, release exit 0, warning-free. Live
proof of the piece: a real run verified the wiring and called
`remember` with no phrases of its own; the fact landed carrying
`[sed -n '2010,2075p' src/agent.rs, src/agent.rs, grep -n
"trajectory_phrases" src/agent.rs]`, and a recall for its subject
scores it at trajectory 0.29 — non-zero on a fact nobody hand-wrote
phrases for. The channel self-populates.

## Errors worth keeping

- **The port probe.** Checked 8080 (llama-server's default) instead of
  fleet's own 8901 default, and reported "no loopback embedder" from
  it. The endpoint under test is the one the code declares, not the
  one the server binary defaults to.
- **The fixture collision, second instance.** A new test in an
  unrelated module named the nonsense word zorbnicator in a string
  literal and killed
  the honest-miss proof in `nomenclature/tests.rs` — the vocabulary is
  derived from the whole tree, so any new test anywhere can retire a
  proof of absence. The preventive rule (grep a nonsense word to zero
  hits before writing it into a test) is now recorded in the
  `fleet-repo-conventions` skill.
- **The truncated status.** `git status --short | head -12` cut off the
  line naming a fact written that turn, and the commit that should
  have carried it claimed six facts while seven existed. Committed in
  the next turn and named in its message rather than folded in
  silently. The skill now records: read the status in full, filter
  noise with `grep -v`, never with `head`.

## Open

The corpus is 17 facts. The vector channel's discriminative power
stays untested at scale until recall has to choose among hundreds —
the re-measurement says the channel carries signal (a ~0.22
related/unrelated gap), so the corpus is the thing to grow, not the
weights. The trajectory channel measures literal token overlap with no
stemming, which is an argument for writing phrases in the vocabulary a
future session would actually use.
